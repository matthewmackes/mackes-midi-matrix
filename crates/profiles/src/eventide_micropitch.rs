//! Eventide `MicroPitch` built-in profile.

use super::{CapabilityDefinition, ControlDefinition, ControlTransport, DeviceProfile, EffectType};

/// One deterministic Eventide controller-layout assignment.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EventideControllerAssignment {
    /// Stable Launch Control physical identity.
    pub physical_control_id: String,
    /// Eventide profile-owned control identity, when supported.
    pub parameter_id: Option<String>,
    /// Human-readable assignment label.
    pub label: String,
    /// Explicit reason when the physical control is intentionally unused.
    pub unsupported_reason: Option<String>,
}

/// Returns the approved row-2/row-3 Eventide layout without inventing MIDI.
#[must_use]
pub fn eventide_controller_assignments() -> Vec<EventideControllerAssignment> {
    let controls = profile()
        .controls
        .iter()
        .enumerate()
        .filter(|(_, control)| control.cc.is_some())
        .map(|(index, control)| (index, control.label.clone()))
        .collect::<Vec<_>>();
    let mut assignments = controls
        .iter()
        .enumerate()
        .map(|(index, (control_index, label))| EventideControllerAssignment {
            physical_control_id: if index < 8 {
                format!("knob-r2-c{}", index + 1)
            } else {
                format!("knob-r3-c{}", index - 7)
            },
            parameter_id: Some(format!("control-{control_index}")),
            label: label.clone(),
            unsupported_reason: None,
        })
        .collect::<Vec<_>>();
    if let Some(mix) = assignments.iter().find(|assignment| assignment.label == "Mix").cloned() {
        assignments.push(EventideControllerAssignment {
            physical_control_id: "fader-1".into(),
            parameter_id: mix.parameter_id,
            label: "Mix (master)".into(),
            unsupported_reason: None,
        });
    }
    assignments.push(EventideControllerAssignment {
        physical_control_id: "button-r1-c1".into(),
        parameter_id: Some("control-2".into()),
        label: "ACTIVE/BYPASS".into(),
        unsupported_reason: None,
    });
    assignments.push(EventideControllerAssignment {
        physical_control_id: "button-r2-c1".into(),
        parameter_id: None,
        label: "Delay bypass".into(),
        unsupported_reason: Some(
            "Eventide documents no independent delay-bypass MIDI control".into(),
        ),
    });
    assignments
}

fn cc_control(label: &str, cc: u8) -> ControlDefinition {
    ControlDefinition {
        label: label.into(),
        cc: Some(cc),
        program: None,
        range: (0, 127),
        operation: None,
    }
}

/// Builds the Eventide `MicroPitch` profile from its firmware 1.0+ MIDI table.
#[must_use]
pub fn profile() -> DeviceProfile {
    DeviceProfile {
        id: "eventide.micropitch".into(),
        version: 1,
        name: "Eventide MicroPitch".into(),
        effect_type: EffectType::Modulation,
        identity_probes: Vec::new(),
        provided_capabilities: vec![
            "pitch_shift".into(),
            "detune".into(),
            "delay".into(),
            "chorus".into(),
            "modulation".into(),
        ],
        capabilities: vec![CapabilityDefinition {
            id: "midi-cc-pc".into(),
            transport: ControlTransport::Midi,
            unsafe_on_connect: false,
        }],
        controls: vec![
            cc_control("Expression Pedal", 4),
            cc_control("TAP TEMPO", 9),
            cc_control("ACTIVE/BYPASS", 14),
            cc_control("FLEX", 15),
            cc_control("Mix", 20),
            cc_control("Pitch A", 21),
            cc_control("Pitch B", 22),
            cc_control("Depth", 23),
            cc_control("Rate/Sens", 24),
            cc_control("Pitch Mix", 25),
            cc_control("Tone", 26),
            cc_control("Delay A", 27),
            cc_control("Delay B", 28),
            cc_control("Mod", 29),
            cc_control("Feedback", 30),
            cc_control("Out Lvl", 31),
            ControlDefinition {
                label: "Preset 1".into(),
                cc: None,
                program: Some(1),
                range: (0, 0),
                operation: None,
            },
        ],
        queries: Vec::new(),
        replies: Vec::new(),
        templates: Vec::new(),
        max_message_size: 1024,
        documented_features: vec![
            "MIDI Program Change loads presets 1–127".into(),
            "MIDI Program Change in Save Mode stores presets 1–127".into(),
            "MIDI over USB or EXP pedal jack".into(),
            "Catch Up knob function".into(),
        ],
    }
}
