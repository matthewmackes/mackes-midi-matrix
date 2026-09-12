use mackes_domain::{MidiEvent, MidiMessage};
use mackes_midi_engine::RoutedEvent;

pub fn midi_activity_json(
    event: &MidiEvent,
    routed: &[RoutedEvent],
    stable_endpoint: Option<&str>,
) -> serde_json::Value {
    let (kind, number, value) = match &event.message {
        MidiMessage::ControlChange { controller, value, .. } => {
            ("control_change", Some(controller.as_u8()), Some(u16::from(value.as_u8())))
        }
        MidiMessage::NoteOn { note, velocity, .. } => {
            ("note_on", Some(note.as_u8()), Some(u16::from(velocity.as_u8())))
        }
        MidiMessage::NoteOff { note, velocity, .. } => {
            ("note_off", Some(note.as_u8()), Some(u16::from(velocity.as_u8())))
        }
        MidiMessage::ProgramChange { program, .. } => {
            ("program_change", Some(program.as_u8()), None)
        }
        MidiMessage::PitchBend { value, .. } => ("pitch_bend", None, Some(value.get())),
        MidiMessage::PolyPressure { note, pressure, .. } => {
            ("poly_pressure", Some(note.as_u8()), Some(u16::from(pressure.as_u8())))
        }
        MidiMessage::ChannelPressure { pressure, .. } => {
            ("channel_pressure", None, Some(u16::from(pressure.as_u8())))
        }
        MidiMessage::SysEx(_) => ("sysex", None, None),
        MidiMessage::SystemCommon(_) => ("system_common", None, None),
        MidiMessage::Realtime(_) => ("realtime", None, None),
    };
    let channel = match &event.message {
        MidiMessage::ControlChange { channel, .. }
        | MidiMessage::NoteOn { channel, .. }
        | MidiMessage::NoteOff { channel, .. }
        | MidiMessage::ProgramChange { channel, .. }
        | MidiMessage::PitchBend { channel, .. }
        | MidiMessage::PolyPressure { channel, .. }
        | MidiMessage::ChannelPressure { channel, .. } => Some(channel.wire()),
        _ => None,
    };
    let endpoint_key =
        stable_endpoint.map_or_else(|| event.endpoint.get().to_string(), str::to_owned);
    let control_id = number.map_or_else(
        || format!("endpoint:{endpoint_key}:{kind}"),
        |n| format!("endpoint:{endpoint_key}:{kind}:{n}"),
    );
    serde_json::json!({"source_endpoint": event.endpoint.get(), "source_endpoint_id": stable_endpoint, "control_id": control_id, "timestamp_nanos": event.timestamp.get(), "kind": kind, "channel": channel, "number": number, "value": value, "destination_endpoints": routed.iter().map(|item| item.event.endpoint.get()).collect::<Vec<_>>(), "sequence": event.sequence})
}
