use serde_json::Value;
use std::path::Path;

pub fn apply_payload(
    value: &Value,
    path: &Path,
    document: &mut mackes_config::ConfigDocument,
) -> Result<(), mackes_config::ConfigError> {
    let parse = |message: &str| mackes_config::ConfigError::Parse {
        path: path.to_path_buf(),
        message: message.to_owned(),
    };
    if let Some(candidate) = value.get("configuration") {
        *document = serde_json::from_value(candidate.clone())
            .map_err(|_| parse("invalid configuration document"))?;
    }
    if let Some(raw) = value.get("configuration_json5").and_then(Value::as_str) {
        *document = mackes_config::parse(raw, path)?;
    }
    if let Some(setlists) = value.get("setlists") {
        document.setlists =
            serde_json::from_value(setlists.clone()).map_err(|_| parse("invalid setlists"))?;
    }
    if let Some(mappings) = value.get("learned_mappings") {
        let mappings = serde_json::from_value(mappings.clone())
            .map_err(|_| parse("invalid learned mappings"))?;
        *document =
            mackes_config::replace_learned_mappings(document, mappings).map_err(|error| {
                mackes_config::ConfigError::Semantic { path: path.to_path_buf(), message: error }
            })?;
    }
    mackes_config::validate(document).map_err(|message| mackes_config::ConfigError::Semantic {
        path: path.to_path_buf(),
        message,
    })
}
use std::io::{self, Write};

pub fn revision(value: &Value) -> Option<String> {
    value.get("configuration_revision").and_then(Value::as_str).map(str::to_owned)
}

pub fn write<W: Write>(stream: &mut W, catalog: &Value, generation: u64) -> io::Result<()> {
    stream.write_all(encode(catalog, generation).as_bytes())
}

pub fn encode(catalog: &Value, generation: u64) -> String {
    let mut response = catalog.clone();
    if let Some(object) = response.as_object_mut() {
        object.insert("ok".into(), Value::Bool(true));
        object.insert("generation".into(), Value::from(generation));
        object.insert("configuration_available".into(), Value::Bool(true));
    }
    format!("{response}\n")
}

/// Adds the persisted/applied revision comparison to a configuration read.
/// The applied revision is the daemon's last successfully committed catalog;
/// a changed file is therefore reported instead of being silently presented
/// as live runtime state.
pub fn with_runtime_status(mut catalog: Value, persisted_revision: Option<&str>) -> Value {
    let applied = catalog.get("configuration_revision").and_then(Value::as_str).map(str::to_owned);
    let state = match (persisted_revision, applied.as_deref()) {
        (Some(persisted), Some(applied)) if persisted == applied => "in_sync",
        (Some(_), Some(_)) => "divergent",
        (None, _) => "persisted_unavailable",
        (_, None) => "runtime_revision_unavailable",
    };
    if let Some(object) = catalog.as_object_mut() {
        object.insert(
            "configuration_runtime_state".into(),
            serde_json::json!({
                "state": state,
                "persisted_revision": persisted_revision,
                "applied_revision": applied,
                "requires_reapply": state == "divergent",
            }),
        );
    }
    catalog
}

pub fn error(error: &impl std::fmt::Display) -> String {
    format!("{}\n", serde_json::json!({"ok": false, "error": error.to_string()}))
}

#[cfg(test)]
mod tests {
    use super::{apply_payload, encode, with_runtime_status};
    use mackes_config::ConfigDocument;
    use serde_json::json;

    #[test]
    fn apply_payload_replaces_complete_document() {
        let path = std::path::Path::new("config.json5");
        let mut document = ConfigDocument::default();
        let mut candidate =
            serde_json::to_value(ConfigDocument::default()).expect("serialize default");
        candidate["schema_version"] = json!(1);
        apply_payload(&json!({"configuration": candidate}), path, &mut document)
            .expect("complete document is valid");
    }

    #[test]
    fn apply_payload_accepts_json5_document_draft() {
        let path = std::path::Path::new("config.json5");
        let mut document = ConfigDocument::default();
        apply_payload(
            &json!({"configuration_json5": "{schema_version: 1, // draft\n setlists: []}"}),
            path,
            &mut document,
        )
        .expect("JSON5 document draft is accepted");
        assert_eq!(document.schema_version, 1);
    }

    #[test]
    fn apply_payload_rejects_invalid_legacy_setlists() {
        let path = std::path::Path::new("config.json5");
        let mut document = ConfigDocument::default();
        let error = apply_payload(&json!({"setlists": {"not": "an array"}}), path, &mut document)
            .expect_err("invalid setlists must fail");
        assert!(error.to_string().contains("invalid setlists"));
    }

    #[test]
    fn encode_preserves_configuration_projection() {
        let body = encode(&json!({"configuration": {"schema_version": 1}}), 4);
        let value: serde_json::Value = serde_json::from_str(&body).expect("response JSON");
        assert_eq!(value["configuration"]["schema_version"], 1);
        assert_eq!(value["configuration_available"], true);
    }

    #[test]
    fn runtime_status_reports_external_persisted_change() {
        let catalog =
            with_runtime_status(json!({"configuration_revision": "applied"}), Some("persisted"));
        assert_eq!(catalog["configuration_runtime_state"]["state"], "divergent");
        assert_eq!(catalog["configuration_runtime_state"]["requires_reapply"], true);
    }

    #[test]
    fn runtime_failure_reports_persisted_but_not_applied_state() {
        let catalog = with_runtime_status(
            json!({"configuration_revision": "persisted-revision"}),
            Some("last-applied-revision"),
        );
        assert_eq!(catalog["configuration_runtime_state"]["state"], "divergent");
        assert_eq!(catalog["configuration_runtime_state"]["requires_reapply"], true);
        assert_eq!(
            catalog["configuration_runtime_state"]["persisted_revision"],
            "last-applied-revision"
        );
        assert_eq!(
            catalog["configuration_runtime_state"]["applied_revision"],
            "persisted-revision"
        );
    }

    #[test]
    fn complete_document_survives_apply_save_restart_and_read() {
        let path = std::env::temp_dir().join(format!(
            "mackes-w158-{}-{}.json5",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let mut document = ConfigDocument::default();
        let mut candidate = serde_json::to_value(&document).expect("serialize default");
        candidate["schema_version"] = json!(1);
        candidate["setlists"] = json!([]);
        apply_payload(&json!({"configuration": candidate}), &path, &mut document)
            .expect("apply complete document");
        mackes_config::save(&path, &document, 1).expect("persist applied document");
        let restarted = mackes_config::load(&path).expect("reload persisted document");
        assert_eq!(restarted, document);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejected_json5_draft_leaves_previous_document_recoverable() {
        let path = std::path::Path::new("draft.json5");
        let mut document = ConfigDocument::default();
        let before = document.clone();
        let rejected = serde_json::json!({"schema_version": 1, "setlists": {"broken": true}});
        assert!(mackes_config::parse("{schema_version: 1, // operator note\n setlists: []}", path)
            .is_ok());
        assert!(apply_payload(&json!({"configuration": rejected}), path, &mut document).is_err());
        assert_eq!(document, before);
    }
}
