//! Configuration persistence health projection.

use std::{
    io,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

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
