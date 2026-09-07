use super::Daemon;

const fn legacy_numeric_binding_allowed(stable_id: Option<&str>) -> bool {
    stable_id.is_none()
}

pub fn legacy_source_endpoint(
    source: &str,
    stable_id: Option<&str>,
    event: mackes_domain::EndpointId,
) -> Option<mackes_domain::EndpointId> {
    if !legacy_numeric_binding_allowed(stable_id) {
        return None;
    }
    mackes_midi_engine::numeric_endpoint_id(source).filter(|candidate| *candidate == event)
}

impl Daemon {
    pub(super) fn apply_native_transitions(
        &mut self,
        transitions: Vec<mackes_midi_engine::NativeIdentityTransition>,
    ) {
        for transition in transitions {
            if transition.identity.direction == mackes_midi_engine::EndpointDirection::Input
                && matches!(
                    transition.state,
                    mackes_midi_engine::PhysicalDeviceState::Offline
                        | mackes_midi_engine::PhysicalDeviceState::Ambiguous
                )
            {
                super::mapping_runtime::reset_button_states(&mut self.button_toggle_state);
            }
            if let Some(failure) = transition.failure {
                self.last_native_failure = Some(failure);
                self.health = super::Health::Degraded;
            }
            if transition.led_resync {
                self.native_led_resync = true;
                self.replay_controller_leds();
            }
        }
    }

    pub(super) fn migration_response(&self, payload: &[u8]) -> String {
        let value = serde_json::from_slice::<serde_json::Value>(payload).unwrap_or_default();
        let dry_run = value.get("dry_run").and_then(serde_json::Value::as_bool).unwrap_or(true);
        let Some(path) = value
            .get("path")
            .and_then(serde_json::Value::as_str)
            .map(std::path::PathBuf::from)
            .or_else(|| self.config_path.clone())
        else {
            return "{\"ok\":false,\"error\":\"configuration path is unavailable\"}\n".into();
        };
        match mackes_config::migrate_file(&path, dry_run, 10) {
            Ok(migrated) => format!(
                "{}\n",
                serde_json::json!({"ok": true, "dry_run": dry_run, "migrated": migrated, "generation": self.generation})
            ),
            Err(error) => format!(
                "{}\n",
                serde_json::json!({"ok": false, "error": error, "generation": self.generation})
            ),
        }
    }

    /// Sends an output event only for the current endpoint binding generation.
    ///
    /// # Errors
    /// Returns an error when the binding generation is stale or the endpoint is unavailable.
    pub fn send_event_to_endpoint_at_generation(
        &mut self,
        endpoint: mackes_domain::EndpointId,
        event: mackes_domain::MidiEvent,
        generation: u64,
    ) -> Result<(), String> {
        if generation != self.binding_generation {
            return Err("stale endpoint binding generation".to_owned());
        }
        self.outputs.send_to_endpoint(endpoint, event)
    }
}
