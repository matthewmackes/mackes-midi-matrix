//! Configuration persistence health projection.

use std::{
    io,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_PAIR_JOURNAL_BYTES: u64 = 256 * 1024;

/// Persists one JSON document with cleanup on every failed commit boundary.
pub fn persist_json_atomic(path: &Path, value: &serde_json::Value, suffix: &str) -> io::Result<()> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let temporary = path.with_extension(format!("{suffix}.{}.{}.tmp", std::process::id(), stamp));
    let result = (|| -> io::Result<()> {
        let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
        std::fs::write(&temporary, bytes)?;
        std::fs::File::open(&temporary)?.sync_all()?;
        std::fs::rename(&temporary, path)?;
        std::fs::File::open(path.parent().unwrap_or_else(|| Path::new(".")))?.sync_all()
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// Recovers a previously prepared two-file JSON commit, if present.
pub fn recover_json_pair(journal: &Path) -> io::Result<bool> {
    let metadata = match std::fs::metadata(journal) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if !metadata.is_file() || metadata.len() > MAX_PAIR_JOURNAL_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid JSON pair journal"));
    }
    let bytes = std::fs::read(journal)?;
    let record: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let first_name = record
        .get("first_name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing first journal path"))?;
    let second_name = record
        .get("second_name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing second journal path"))?;
    let first_value = record
        .get("first")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing first journal value"))?;
    let second_value = record.get("second").ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing second journal value")
    })?;
    let parent = journal.parent().unwrap_or_else(|| Path::new("."));
    let first = parent.join(first_name);
    let second = parent.join(second_name);
    if first.file_name().and_then(|name| name.to_str()) != Some(first_name)
        || second.file_name().and_then(|name| name.to_str()) != Some(second_name)
        || first == second
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "JSON pair journal path is invalid",
        ));
    }
    persist_json_atomic(&first, first_value, "pair-recovery")?;
    persist_json_atomic(&second, second_value, "pair-recovery")?;
    std::fs::remove_file(journal)?;
    std::fs::File::open(parent)?.sync_all()?;
    Ok(true)
}

/// Commits two JSON documents through a durable prepare journal.
pub fn persist_json_pair_atomic(
    first: &Path,
    first_value: &serde_json::Value,
    second: &Path,
    second_value: &serde_json::Value,
    journal: &Path,
) -> io::Result<()> {
    let parent = journal.parent().unwrap_or_else(|| Path::new("."));
    if first.parent().unwrap_or_else(|| Path::new(".")) != parent
        || second.parent().unwrap_or_else(|| Path::new(".")) != parent
        || first == second
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "JSON pair paths must share a directory",
        ));
    }
    let first_name = first.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    let second_name = second.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    if first_name.is_empty() || second_name.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "JSON pair paths require file names",
        ));
    }
    let bytes = serde_json::to_vec(&serde_json::json!({
        "first_name": first_name,
        "first": first_value,
        "second_name": second_name,
        "second": second_value,
    }))
    .map_err(io::Error::other)?;
    if bytes.len() as u64 > MAX_PAIR_JOURNAL_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "JSON pair journal is oversized"));
    }
    let temporary = journal.with_extension(format!(
        "{}.{}.tmp",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos()
    ));
    let result = (|| -> io::Result<()> {
        std::fs::write(&temporary, bytes)?;
        std::fs::File::open(&temporary)?.sync_all()?;
        std::fs::rename(&temporary, journal)?;
        std::fs::File::open(parent)?.sync_all()?;
        persist_json_atomic(first, first_value, "pair-commit")?;
        persist_json_atomic(second, second_value, "pair-commit")?;
        std::fs::remove_file(journal)?;
        std::fs::File::open(parent)?.sync_all()
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// Projects the configured file's basic persistence availability for IPC snapshots.
pub fn config_persistence(path: Option<&Path>) -> serde_json::Value {
    let Some(path) = path else {
        return serde_json::json!({"state": "unconfigured", "action": "set a writable configuration path"});
    };
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => {
            match std::fs::OpenOptions::new().write(true).open(path) {
                Ok(_) => match mackes_config::load(path) {
                    Ok(_) => serde_json::json!({"state": "ready", "action": "none"}),
                    Err(_) => serde_json::json!({
                        "state": "corrupt",
                        "action": "restore a verified configuration backup",
                    }),
                },
                Err(_) => serde_json::json!({
                    "state": "read_only",
                    "action": "check configuration ownership and permissions",
                }),
            }
        }
        Ok(_) => {
            serde_json::json!({"state": "unreadable", "action": "choose a regular configuration file"})
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            serde_json::json!({"state": "missing", "action": "restore or create the configuration file"})
        }
        Err(_) => {
            serde_json::json!({"state": "unreadable", "action": "check configuration permissions"})
        }
    }
}

/// Lists daemon-managed backup artifacts without exposing host paths.
pub fn backup_inventory(path: Option<&Path>) -> serde_json::Value {
    let Some(config) = path else {
        return serde_json::json!({"state": "unconfigured", "backups": [], "manifests": []});
    };
    let Some(directory) = config.parent() else {
        return serde_json::json!({"state": "unreadable", "backups": [], "manifests": []});
    };
    let mut backups = std::fs::read_dir(directory)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            (name.ends_with(".backup") || name.ends_with(".manifest.json")).then_some(name)
        })
        .collect::<Vec<_>>();
    backups.sort();
    let manifests = backups
        .iter()
        .filter(|name| name.ends_with(".manifest.json"))
        .map(|name| {
            let path = directory.join(name);
            let metadata = std::fs::metadata(&path).ok();
            let value = metadata
                .filter(|metadata| metadata.is_file() && metadata.len() <= 64 * 1024)
                .and_then(|_| std::fs::read(&path).ok())
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
                .unwrap_or_else(|| serde_json::json!({"state": "invalid_or_unreadable"}));
            serde_json::json!({"name": name, "metadata": value})
        })
        .collect::<Vec<_>>();
    serde_json::json!({"state": "ready", "backups": backups, "manifests": manifests})
}

/// Encodes the daemon-owned backup inventory response.
#[allow(clippy::too_many_lines)]
pub fn backup_response(generation: u64, path: Option<&Path>, request: Option<&[u8]>) -> String {
    let request_value = request
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let action = request_value.get("action").and_then(serde_json::Value::as_str);
    if action == Some("create") {
        let result = path
            .ok_or_else(|| "configuration path is not configured".to_owned())
            .and_then(|config| {
                let payload = std::fs::read(config).map_err(|error| error.to_string())?;
                let backup = config.with_file_name(format!(
                    "{}.backup",
                    config.file_name().and_then(|name| name.to_str()).unwrap_or("mackes.json5")
                ));
                let manifest = mackes_config::BackupManifest {
                    profile: "mackes-configuration".into(),
                    device_identity: "host-configuration".into(),
                    source_alias: config
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("configuration")
                        .into(),
                    captured_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    sha256: mackes_config::BackupManifest::digest(&payload),
                    status: mackes_config::BackupStatus::Verified,
                };
                mackes_config::save_backup(&backup, &payload, &manifest)
                    .map(|()| backup.display().to_string())
            });
        return match result {
            Ok(_) => {
                serde_json::json!({"ok": true, "generation": generation, "created": true})
                    .to_string()
                    + "\n"
            }
            Err(error) => {
                serde_json::json!({"ok": false, "generation": generation, "error": error})
                    .to_string()
                    + "\n"
            }
        };
    }
    if action == Some("restore") {
        let result = path
            .ok_or_else(|| "configuration path is not configured".to_owned())
            .and_then(|config| {
                if request_value.get("confirm").and_then(serde_json::Value::as_bool) != Some(true) {
                    return Err("backup restore requires confirmation".into());
                }
                let name = request_value
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .ok_or("backup name is required")?;
                if name.contains('/') || name.contains('\\') || !name.ends_with(".backup") {
                    return Err("backup name is not a managed basename".into());
                }
                let backup = config.parent().unwrap_or_else(|| Path::new(".")).join(name);
                mackes_config::restore_backup(
                    &backup,
                    config,
                    "mackes-configuration",
                    "host-configuration",
                    mackes_config::RestoreMode::Apply,
                )
                .map(|result| serde_json::json!({"result": format!("{result:?}")}))
            });
        return match result {
            Ok(result) => {
                serde_json::json!({"ok": true, "generation": generation, "restored": result})
                    .to_string()
                    + "\n"
            }
            Err(error) => {
                serde_json::json!({"ok": false, "generation": generation, "error": error})
                    .to_string()
                    + "\n"
            }
        };
    }
    if action == Some("export") {
        let result = path
            .ok_or_else(|| "configuration path is not configured".to_owned())
            .and_then(|config| {
                let bytes = std::fs::read(config).map_err(|error| error.to_string())?;
                if bytes.len() > 1024 * 1024 {
                    return Err("configuration export exceeds the 1 MiB bound".into());
                }
                String::from_utf8(bytes).map_err(|_| "configuration export is not UTF-8".to_owned())
            });
        return match result {
            Ok(content) => serde_json::json!({"ok": true, "generation": generation, "exported": true, "content": content}).to_string() + "\n",
            Err(error) => serde_json::json!({"ok": false, "generation": generation, "error": error}).to_string() + "\n",
        };
    }
    if action == Some("portable_export") {
        let result = path
            .ok_or_else(|| "configuration path is not configured".to_owned())
            .and_then(|config| mackes_config::load(config).map_err(|error| error.to_string()))
            .and_then(|document| {
                serde_json::to_string_pretty(&document).map_err(|error| error.to_string())
            });
        return match result {
            Ok(content) => serde_json::json!({
                "ok": true, "generation": generation, "portable_exported": true, "content": content
            })
            .to_string()
                + "\n",
            Err(error) => {
                serde_json::json!({
                    "ok": false, "generation": generation, "error": error
                })
                .to_string()
                    + "\n"
            }
        };
    }
    if action == Some("portable_import") {
        let result = path
            .ok_or_else(|| "configuration path is not configured".to_owned())
            .and_then(|config| {
                let content = request_value
                    .get("content")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| "portable configuration content is required".to_owned())?;
                if content.len() > 1024 * 1024 {
                    return Err("portable configuration exceeds the 1 MiB bound".to_owned());
                }
                let document = mackes_config::parse(content, Path::new("portable-import.json5"))
                    .map_err(|error| error.to_string())?;
                mackes_config::save(config, &document, 3).map_err(|error| error.to_string())
            });
        return match result {
            Ok(()) => {
                serde_json::json!({
                    "ok": true, "generation": generation, "portable_imported": true
                })
                .to_string()
                    + "\n"
            }
            Err(error) => {
                serde_json::json!({
                    "ok": false, "generation": generation, "error": error
                })
                .to_string()
                    + "\n"
            }
        };
    }
    let inventory = backup_inventory(path);
    serde_json::json!({
        "ok": true,
        "generation": generation,
        "backup_state": inventory.get("state"),
        "backups": inventory.get("backups"),
        "manifests": inventory.get("manifests"),
    })
    .to_string()
        + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_inventory_is_truthful_when_configuration_is_unconfigured() {
        let inventory = backup_inventory(None);
        assert_eq!(inventory["state"], "unconfigured");
        assert_eq!(inventory["backups"], serde_json::json!([]));
        assert_eq!(inventory["manifests"], serde_json::json!([]));
        let response = backup_response(7, None, None);
        assert!(response.contains("\"generation\":7"));
        assert!(response.contains("\"backup_state\":\"unconfigured\""));
        assert!(response.contains("\"manifests\":[]"));
    }

    #[test]
    fn backup_inventory_lists_only_adjacent_managed_artifacts() {
        let root =
            std::env::temp_dir().join(format!("mackes-backup-inventory-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temporary backup directory");
        let config = root.join("mackes.json5");
        std::fs::write(&config, b"{}\n").expect("configuration");
        std::fs::write(root.join("mackes.json5.backup"), b"backup").expect("backup");
        std::fs::write(root.join("mackes.json5.manifest.json"), b"manifest").expect("manifest");
        std::fs::write(root.join("unrelated.txt"), b"ignore").expect("unrelated");
        let inventory = backup_inventory(Some(&config));
        assert_eq!(inventory["state"], "ready");
        assert_eq!(inventory["backups"].as_array().expect("backup list").len(), 2);
        assert_eq!(inventory["manifests"].as_array().expect("manifest list").len(), 1);
        let response = backup_response(9, Some(&config), None);
        assert!(response.contains("\"manifests\":["));
        assert!(response.contains("invalid_or_unreadable"));
        assert!(!inventory.to_string().contains("unrelated.txt"));
        std::fs::remove_dir_all(root).expect("remove temporary backup directory");
    }

    #[test]
    fn backup_create_and_restore_are_daemon_owned_and_path_bounded() {
        let root =
            std::env::temp_dir().join(format!("mackes-backup-roundtrip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temporary backup directory");
        let config = root.join("mackes.json5");
        std::fs::write(&config, b"original\n").expect("configuration");
        let create =
            backup_response(3, Some(&config), Some(br#"{"action":"create","confirm":true}"#));
        assert!(create.contains("\"created\":true"));
        let duplicate =
            backup_response(3, Some(&config), Some(br#"{"action":"create","confirm":true}"#));
        assert!(duplicate.contains("backup artifact already exists and is immutable"));
        std::fs::write(&config, b"changed\n").expect("changed configuration");
        let restore = backup_response(
            4,
            Some(&config),
            Some(br#"{"action":"restore","name":"mackes.json5.backup","confirm":true}"#),
        );
        assert!(restore.contains("\"restored\":{\"result\""), "{restore}");
        assert_eq!(std::fs::read(&config).expect("restored bytes"), b"original\n");
        let rejected = backup_response(
            5,
            Some(&config),
            Some(br#"{"action":"restore","name":"../mackes.json5","confirm":true}"#),
        );
        assert!(rejected.contains("backup name is not a managed basename"));
        let export = backup_response(6, Some(&config), Some(br#"{"action":"export"}"#));
        assert!(export.contains("\"exported\":true") && export.contains("original\\n"));
        let portable_config = root.join("portable.json5");
        std::fs::write(&portable_config, include_str!("../../../packaging/default-config.json5"))
            .expect("portable source configuration");
        let portable =
            backup_response(7, Some(&portable_config), Some(br#"{"action":"portable_export"}"#));
        assert!(portable.contains("\"portable_exported\":true"));
        let portable_value: serde_json::Value =
            serde_json::from_str(&portable).expect("portable response");
        let imported = serde_json::json!({
            "action": "portable_import",
            "confirm": true,
            "content": portable_value["content"].as_str().expect("portable content")
        });
        assert!(backup_response(
            8,
            Some(&config),
            Some(&serde_json::to_vec(&imported).expect("import request"))
        )
        .contains("\"portable_imported\":true"));
        std::fs::remove_dir_all(root).expect("remove temporary backup directory");
    }

    #[test]
    fn json_pair_commit_and_recovery_complete_both_files() {
        let root = std::env::temp_dir().join(format!("mackes-json-pair-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temporary pair directory");
        let first = root.join("routes.json");
        let second = root.join("routes.undo.json");
        let journal = root.join("routes.commit.json");
        persist_json_pair_atomic(
            &first,
            &serde_json::json!({"routes":[{"source":1}]}),
            &second,
            &serde_json::json!({"routes":[{"source":0}]}),
            &journal,
        )
        .expect("pair commit");
        assert!(!journal.exists());
        std::fs::write(
            &journal,
            serde_json::to_vec(&serde_json::json!({
                "first_name":"routes.json",
                "first":{"routes":[{"source":2}]},
                "second_name":"routes.undo.json",
                "second":{"routes":[{"source":1}]}
            }))
            .expect("journal bytes"),
        )
        .expect("prepared journal");
        assert!(recover_json_pair(&journal).expect("pair recovery"));
        assert!(!journal.exists());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(first).expect("first"))
                .expect("first JSON")["routes"][0]["source"],
            2
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(second).expect("second"))
                .expect("second JSON")["routes"][0]["source"],
            1
        );
        std::fs::remove_dir_all(root).expect("remove temporary pair directory");
    }

    #[test]
    fn json_pair_recovery_rejects_malformed_and_path_escaping_journals() {
        let root =
            std::env::temp_dir().join(format!("mackes-json-pair-invalid-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temporary pair directory");
        let journal = root.join("routes.commit.json");
        std::fs::write(&journal, b"not-json").expect("malformed journal");
        assert!(recover_json_pair(&journal).is_err());
        std::fs::write(
            &journal,
            serde_json::to_vec(&serde_json::json!({
                "first_name":"../outside.json",
                "first":{},
                "second_name":"routes.undo.json",
                "second":{}
            }))
            .expect("escaping journal bytes"),
        )
        .expect("escaping journal");
        assert!(recover_json_pair(&journal).is_err());
        assert!(journal.exists());
        std::fs::remove_dir_all(root).expect("remove temporary pair directory");
    }
}
