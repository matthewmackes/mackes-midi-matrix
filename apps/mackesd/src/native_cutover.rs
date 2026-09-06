//! Hermetic native ALSA cutover coverage for daemon assignment dispatch.

use super::{Daemon, Health};
use mackes_domain::{MidiChannel, MidiEvent, MidiMessage, SevenBit, TimestampNanos};
use mackes_midi_engine::{
    AlsaSequencerAddress, AlsaSequencerLifecycle, EndpointDirection, NativeHardwareIdentity,
    NativePortAnnouncement,
};
use std::fs;

fn event(message: MidiMessage) -> MidiEvent {
    MidiEvent {
        timestamp: TimestampNanos::new(1),
        sequence: 1,
        endpoint: mackes_midi_engine::numeric_endpoint_id("1").expect("endpoint"),
        message,
    }
}

const fn channel() -> MidiChannel {
    MidiChannel::new(9).expect("channel")
}

fn daemon() -> (Daemon, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "mackes-native-cutover-{}-{}.sock",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    (Daemon::bind(&path).expect("daemon"), path)
}

#[test]
fn native_device_press_starts_once_and_release_does_not_restart() {
    let (mut daemon, path) = daemon();
    daemon.deferred_inputs.push_back(event(MidiMessage::NoteOn {
        channel: channel(),
        note: SevenBit::new(105).expect("note"),
        velocity: SevenBit::new(127).expect("velocity"),
    }));
    assert_eq!(daemon.poll_and_dispatch_inputs(8).0, 1);
    assert_eq!(daemon.assignment_session.phase, mackes_ipc::AssignmentPhase::AwaitControl);
    let generation = daemon.assignment_generation;
    daemon.deferred_inputs.push_back(event(MidiMessage::NoteOn {
        channel: channel(),
        note: SevenBit::new(105).expect("note"),
        velocity: SevenBit::new(0).expect("release"),
    }));
    assert_eq!(daemon.poll_and_dispatch_inputs(8).0, 1);
    assert_eq!(daemon.assignment_session.phase, mackes_ipc::AssignmentPhase::AwaitControl);
    assert_eq!(daemon.assignment_generation, generation);
    let _ = fs::remove_file(path);
}

#[test]
fn native_dispatch_requeues_events_beyond_one_cycle_budget() {
    let (mut daemon, path) = daemon();
    for sequence in 0..130_u64 {
        daemon.deferred_inputs.push_back(MidiEvent {
            timestamp: TimestampNanos::new(sequence),
            sequence,
            endpoint: mackes_domain::EndpointId::new(1).expect("endpoint"),
            message: MidiMessage::ChannelPressure {
                channel: MidiChannel::new(1).expect("channel"),
                pressure: SevenBit::new(64).expect("pressure"),
            },
        });
    }
    let (processed, sent, unmatched) = daemon.poll_and_dispatch_inputs(1);
    assert_eq!((processed, sent, unmatched), (1, 0, 0));
    assert_eq!(daemon.deferred_inputs.len(), 129);
    let _ = fs::remove_file(path);
}

#[test]
fn native_knob_advances_and_arrows_navigate() {
    let (mut daemon, path) = daemon();
    daemon
        .register_input(Box::new(mackes_midi_engine::VirtualEndpoint::new(
            "1",
            "Launch Control XL MIDI",
            EndpointDirection::Input,
        )))
        .expect("native input");
    daemon.deferred_inputs.push_back(event(MidiMessage::NoteOn {
        channel: channel(),
        note: SevenBit::new(105).expect("note"),
        velocity: SevenBit::new(127).expect("velocity"),
    }));
    daemon.poll_and_dispatch_inputs(8);
    daemon.deferred_inputs.push_back(event(MidiMessage::ControlChange {
        channel: channel(),
        controller: SevenBit::new(13).expect("cc"),
        value: SevenBit::new(64).expect("value"),
    }));
    daemon.poll_and_dispatch_inputs(8);
    assert_eq!(daemon.assignment_session.phase, mackes_ipc::AssignmentPhase::ChooseDevice);
    daemon.deferred_inputs.push_back(event(MidiMessage::ControlChange {
        channel: channel(),
        controller: SevenBit::new(107).expect("right"),
        value: SevenBit::new(127).expect("press"),
    }));
    daemon.poll_and_dispatch_inputs(8);
    assert_eq!(daemon.assignment_session.phase, mackes_ipc::AssignmentPhase::ChoosePreset);
    daemon.deferred_inputs.push_back(event(MidiMessage::ControlChange {
        channel: channel(),
        controller: SevenBit::new(104).expect("up"),
        value: SevenBit::new(127).expect("press"),
    }));
    daemon.poll_and_dispatch_inputs(8);
    let snapshot: serde_json::Value =
        serde_json::from_str(daemon.snapshot_response().trim()).expect("snapshot");
    assert_eq!(snapshot["native_backend"], "alsa-seq");
    let _ = fs::remove_file(path);
}

#[test]
fn reconnect_preserves_assignment_and_output_requests_led_replay() {
    let (mut daemon, path) = daemon();
    daemon.deferred_inputs.push_back(event(MidiMessage::NoteOn {
        channel: channel(),
        note: SevenBit::new(105).expect("note"),
        velocity: SevenBit::new(127).expect("velocity"),
    }));
    daemon.poll_and_dispatch_inputs(8);
    let identity_in =
        NativeHardwareIdentity::new("Launch Control XL MK2", "MIDI", 0, EndpointDirection::Input);
    let identity_out =
        NativeHardwareIdentity::new("Launch Control XL MK2", "MIDI", 1, EndpointDirection::Output);
    daemon.alsa_supervisor.remember("lcxl-in", identity_in.clone());
    daemon.alsa_supervisor.remember("lcxl-out", identity_out.clone());
    daemon.alsa_supervisor.ingest(NativePortAnnouncement {
        lifecycle: AlsaSequencerLifecycle::Started,
        address: AlsaSequencerAddress::new(24, 1),
        identity: Some(identity_out.clone()),
        permission_denied: false,
    });
    daemon.alsa_supervisor.ingest(NativePortAnnouncement {
        lifecycle: AlsaSequencerLifecycle::Exited,
        address: AlsaSequencerAddress::new(24, 1),
        identity: Some(identity_out.clone()),
        permission_denied: false,
    });
    daemon.alsa_supervisor.ingest(NativePortAnnouncement {
        lifecycle: AlsaSequencerLifecycle::Started,
        address: AlsaSequencerAddress::new(31, 1),
        identity: Some(identity_out),
        permission_denied: false,
    });
    let transitions = daemon.alsa_supervisor.reconcile();
    daemon.apply_native_transitions(transitions);
    assert_eq!(daemon.assignment_session.phase, mackes_ipc::AssignmentPhase::AwaitControl);
    assert!(daemon.native_led_resync);
    assert!(daemon.last_native_failure.is_none());
    daemon.alsa_supervisor.ingest(NativePortAnnouncement {
        lifecycle: AlsaSequencerLifecycle::Started,
        address: AlsaSequencerAddress::new(24, 0),
        identity: Some(identity_in.clone()),
        permission_denied: false,
    });
    daemon.alsa_supervisor.ingest(NativePortAnnouncement {
        lifecycle: AlsaSequencerLifecycle::Started,
        address: AlsaSequencerAddress::new(25, 0),
        identity: Some(identity_in),
        permission_denied: false,
    });
    let duplicates = daemon.alsa_supervisor.reconcile();
    daemon.apply_native_transitions(duplicates);
    assert_eq!(daemon.health, Health::Degraded);
    assert_eq!(daemon.last_native_failure, Some("duplicate native ALSA identity"));
    let _ = fs::remove_file(path);
}

#[test]
fn native_routes_continue_and_unrelated_input_fails_closed() {
    let (mut daemon, path) = daemon();
    let source = mackes_midi_engine::numeric_endpoint_id("1").expect("source");
    let destination = mackes_domain::EndpointId::new(2).expect("dest");
    daemon
        .replace_routes(
            vec![mackes_midi_engine::Route {
                source,
                destination,
                destination_parameter: None,
                channel: None,
                class: None,
                enabled: true,
                priority: 0,
                curve: mackes_midi_engine::Curve::Linear,
                predicates: Vec::new(),
                allow_cycle: false,
            }],
            1,
            8,
        )
        .expect("routes");
    daemon
        .register_output(Box::new(mackes_midi_engine::VirtualEndpoint::new(
            "2",
            "Route Out",
            EndpointDirection::Output,
        )))
        .expect("output");
    daemon.deferred_inputs.push_back(MidiEvent {
        endpoint: source,
        ..event(MidiMessage::ControlChange {
            channel: MidiChannel::new(2).expect("ch"),
            controller: SevenBit::new(7).expect("cc"),
            value: SevenBit::new(10).expect("value"),
        })
    });
    let (processed, sent, unmatched) = daemon.poll_and_dispatch_inputs(8);
    assert_eq!(processed, 1);
    assert_eq!(sent, 1);
    assert_eq!(unmatched, 0);
    assert_eq!(daemon.assignment_session.phase, mackes_ipc::AssignmentPhase::Idle);
    daemon.deferred_inputs.push_back(event(MidiMessage::NoteOn {
        channel: MidiChannel::new(2).expect("ch"),
        note: SevenBit::new(105).expect("note"),
        velocity: SevenBit::new(127).expect("velocity"),
    }));
    daemon.poll_and_dispatch_inputs(8);
    assert_eq!(daemon.assignment_session.phase, mackes_ipc::AssignmentPhase::Idle);
    daemon.deferred_inputs.push_back(event(MidiMessage::ChannelPressure {
        channel: channel(),
        pressure: SevenBit::new(90).expect("pressure"),
    }));
    daemon.poll_and_dispatch_inputs(8);
    assert_eq!(daemon.assignment_session.phase, mackes_ipc::AssignmentPhase::Idle);
    let first = daemon.state_sequence;
    let snapshot = daemon.snapshot_response();
    assert!(snapshot.contains("\"native_backend\":\"alsa-seq\""));
    assert!(snapshot.contains("\"assignment_session\""));
    assert_eq!(daemon.state_sequence, first);
    let _ = fs::remove_file(path);
}

#[test]
fn native_permission_pressure_degrades_health_without_dropping_session() {
    let (mut daemon, path) = daemon();
    daemon.deferred_inputs.push_back(event(MidiMessage::NoteOn {
        channel: channel(),
        note: SevenBit::new(105).expect("note"),
        velocity: SevenBit::new(127).expect("velocity"),
    }));
    daemon.poll_and_dispatch_inputs(8);
    let identity =
        NativeHardwareIdentity::new("Launch Control XL MK2", "MIDI", 0, EndpointDirection::Input);
    daemon.alsa_supervisor.remember("lcxl-in", identity.clone());
    daemon.alsa_supervisor.ingest(NativePortAnnouncement {
        lifecycle: AlsaSequencerLifecycle::Started,
        address: AlsaSequencerAddress::new(24, 0),
        identity: Some(identity),
        permission_denied: true,
    });
    let transitions = daemon.alsa_supervisor.reconcile();
    daemon.apply_native_transitions(transitions);
    assert_eq!(daemon.health, Health::Degraded);
    assert_eq!(daemon.last_native_failure, Some("native ALSA permission denied"));
    assert_eq!(daemon.assignment_session.phase, mackes_ipc::AssignmentPhase::AwaitControl);
    let _ = fs::remove_file(path);
}
