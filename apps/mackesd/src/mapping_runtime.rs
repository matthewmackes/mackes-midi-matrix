//! Runtime policy for profile-owned control mappings.

use std::collections::HashMap;

pub const fn health_after_authorized_command(
    current: super::Health,
    command: Option<mackes_ipc::Command>,
) -> super::Health {
    if matches!(command, Some(mackes_ipc::Command::Health)) {
        current
    } else {
        super::Health::Ready
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
