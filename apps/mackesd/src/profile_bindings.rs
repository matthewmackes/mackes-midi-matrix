//! Explicit profile-to-output binding and catalog projection.

/// Resolves only an explicit durable profile-to-output binding.
pub fn output_endpoint(
    outputs: &mackes_midi_engine::OutputRegistry,
    bindings: &[(String, String)],
    profile_id: &str,
) -> Option<mackes_domain::EndpointId> {
    stable_destination(outputs, bindings, profile_id)
        .as_deref()
        .and_then(mackes_midi_engine::numeric_endpoint_id)
}

/// Resolves a profile to one exact stable output ID.
pub fn stable_destination(
    outputs: &mackes_midi_engine::OutputRegistry,
    bindings: &[(String, String)],
    profile_id: &str,
) -> Option<String> {
    bindings.iter().find_map(|(profile, endpoint_id)| {
        (profile == profile_id
            && outputs.output_infos().iter().any(|output| output.id == *endpoint_id))
        .then(|| endpoint_id.clone())
    })
}

/// Returns unique profile IDs visible through bindings or identified hardware.
pub fn catalog_ids(
    physical_devices: &serde_json::Value,
    bindings: &[(String, String)],
) -> Vec<String> {
    let mut ids = bindings.iter().map(|(profile, _)| profile.clone()).collect::<Vec<_>>();
    if let Some(devices) = physical_devices.as_array() {
        for device in devices {
            let name = device
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_ascii_lowercase();
            if name.contains("reflex") {
                ids.push("lexicon.reflex".into());
            }
            if name.contains("eventide") || name.contains("micropitch") {
                ids.push("eventide.micropitch".into());
            }
        }
    }
    if ids.is_empty() {
        ids.extend(["lexicon.reflex".into(), "eventide.micropitch".into()]);
    }
    let mut unique = Vec::new();
    for id in ids {
        if !unique.contains(&id) {
            unique.push(id);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_generic_port_binding_is_exact_and_catalog_visible() {
        let mut outputs = mackes_midi_engine::OutputRegistry::new(4);
        outputs
            .insert(Box::new(mackes_midi_engine::VirtualEndpoint::new(
                "port-d",
                "MidiSport 4x4 MIDI 4",
                mackes_midi_engine::EndpointDirection::Output,
            )))
            .expect("output");
        let bindings = vec![("lexicon.reflex".into(), "port-d".into())];
        assert_eq!(
            output_endpoint(&outputs, &bindings, "lexicon.reflex"),
            mackes_midi_engine::numeric_endpoint_id("port-d")
        );
        assert_eq!(catalog_ids(&serde_json::json!([]), &bindings), vec!["lexicon.reflex"]);
        assert_eq!(
            stable_destination(&outputs, &bindings, "lexicon.reflex").as_deref(),
            Some("port-d")
        );
    }
}
