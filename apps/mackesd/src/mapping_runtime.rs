//! Runtime policy for profile-owned control mappings.

use std::collections::HashMap;

pub const fn health_after_authorized_command(
    current: super::Health,
    command: Option<mackes_ipc::Command>,
) -> super::Health {
    match (current, command) {
        (super::Health::Starting, Some(mackes_ipc::Command::Health)) => super::Health::Starting,
        (super::Health::Starting, _) => super::Health::Ready,
        (current, _) => current,
    }
}

pub fn is_experimental(mapping: &mackes_config::ControlMapping) -> bool {
    mackes_profiles::builtin_profile(&mapping.destination_profile).is_some_and(|profile| {
        mackes_profiles::destination_parameters(&profile)
            .into_iter()
            .find(|parameter| parameter.id == mapping.destination_parameter)
            .and_then(|parameter| parameter.evidence)
            == Some(mackes_profiles::EvidenceLevel::Experimental)
    })
}

pub fn needs_confirmation(mapping: &mackes_config::ControlMapping) -> bool {
    mapping.source_kind == "note"
        && mackes_profiles::builtin_profile(&mapping.destination_profile)
            .and_then(|profile| {
                mackes_profiles::destination_parameters(&profile)
                    .into_iter()
                    .find(|parameter| parameter.id == mapping.destination_parameter)
            })
            .is_some_and(|parameter| {
                parameter.source_role == Some(mackes_profiles::SourceRole::ButtonToggle)
            })
}

pub fn destination_value(
    states: &mut HashMap<String, (bool, bool)>,
    mapping: &mackes_config::ControlMapping,
    input: u16,
) -> Option<u16> {
    let press_only = mapping.destination_profile == "lexicon.reflex"
        && (mapping.destination_parameter.starts_with("reflex.algorithm-")
            || mapping.destination_parameter.starts_with("pcm70_reflex:"));
    if press_only && mapping.source_kind == "note" {
        let state = states.entry(mapping.id.clone()).or_insert((false, false));
        let pressed = input != 0;
        let rising = pressed && !state.0;
        state.0 = pressed;
        return rising.then_some(input);
    }
    let parameter =
        mackes_profiles::builtin_profile(&mapping.destination_profile).and_then(|profile| {
            mackes_profiles::destination_parameters(&profile)
                .into_iter()
                .find(|parameter| parameter.id == mapping.destination_parameter)
        });
    let toggles = parameter.as_ref().is_some_and(|parameter| {
        parameter.source_role == Some(mackes_profiles::SourceRole::ButtonToggle)
    });
    if !toggles || mapping.source_kind != "note" {
        return Some(input);
    }
    // ACTIVE/BYPASS is presented as Enable Bypass, so its first press must
    // request bypass. Other toggles retain the conventional first-press-on state.
    let initially_on = parameter.is_some_and(|parameter| parameter.label == "ACTIVE/BYPASS");
    let state = states.entry(mapping.id.clone()).or_insert((false, initially_on));
    let pressed = input != 0;
    let rising = pressed && !state.0;
    state.0 = pressed;
    if !rising {
        return None;
    }
    state.1 = !state.1;
    Some(if state.1 { 127 } else { 0 })
}

#[cfg(target_os = "linux")]
impl super::Daemon {
    /// Returns the authoritative assignment LED state at a fake-clock instant.
    #[must_use]
    pub const fn assignment_led_state_at(&self, elapsed_ms: u64) -> mackes_profiles::LedState {
        self.led.scheduler.state_at(elapsed_ms)
    }

    /// Returns the validated Mk1 `SysEx` frame for the current assignment LED state.
    #[must_use]
    pub fn assignment_led_frame_at(
        &self,
        template: u8,
        index: u8,
        elapsed_ms: u64,
    ) -> Option<Vec<u8>> {
        mackes_profiles::encode_launch_control_feedback(
            template,
            index,
            self.assignment_led_state_at(elapsed_ms),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn algorithm_mapping() -> mackes_config::ControlMapping {
        mackes_config::ControlMapping {
            id: "algorithm-1".into(),
            controller_profile: "launch-control-xl-mk2".into(),
            physical_control_id: "button-r1-c5".into(),
            source_endpoint: "controller".into(),
            source_kind: "note".into(),
            source_channel: 8,
            destination_channel: Some(0),
            source_number: 57,
            destination_endpoint: "lexicon".into(),
            destination_profile: "lexicon.reflex".into(),
            destination_effect: "algorithm-1".into(),
            destination_parameter: "reflex.algorithm-1".into(),
            behavior: mackes_config::MappingBehavior {
                source_range: (0, 127),
                destination_range: (0, 127),
                invert: false,
                curve: "linear".into(),
            },
            enabled: true,
            profile_version: 1,
        }
    }

    #[test]
    fn lexicon_algorithm_selection_is_press_only() {
        let mapping = algorithm_mapping();
        let mut states = HashMap::new();
        assert_eq!(destination_value(&mut states, &mapping, 127), Some(127));
        assert_eq!(destination_value(&mut states, &mapping, 127), None);
        assert_eq!(destination_value(&mut states, &mapping, 0), None);
        assert_eq!(destination_value(&mut states, &mapping, 127), Some(127));
    }
}
