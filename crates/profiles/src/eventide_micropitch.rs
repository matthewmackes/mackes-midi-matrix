//! Eventide `MicroPitch` built-in profile.

use super::{CapabilityDefinition, ControlDefinition, ControlTransport, DeviceProfile, EffectType};

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
