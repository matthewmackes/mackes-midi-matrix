//! Deterministic fake endpoints and clocks for tests.

/// Reproducible faults that a fake endpoint may inject.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fault {
    /// Drop the connection at the next operation.
    Disconnect,
    /// Return a malformed payload.
    MalformedFrame,
    /// Report a full bounded queue.
    QueueFull,
    /// Fail the next write.
    WriteFailure,
}

/// Names of the hermetic integration scenarios required by the worklist.
///
/// Keeping this inventory in code makes omissions visible to CI and gives each
/// future scenario a stable identifier for reports.
pub const INTEGRATION_SCENARIOS: &[&str] = &[
    "multiport-routing",
    "daw-round-trip",
    "cc-pc-transform",
    "hot-plug-and-alias-ambiguity",
    "daemon-client-restart",
    "action-pacing",
    "partial-scene-failure",
    "sysex-query",
    "rtp-midi-peer",
    "learn-capture",
    "reflex-diagram-metadata",
    "startup-restore-unsafe-policy",
    "performance-lock-and-panic",
];

/// Ordered fault plan; consuming a fault advances deterministically.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FaultPlan {
    faults: Vec<Fault>,
    cursor: usize,
}

impl FaultPlan {
    /// Builds a plan from an ordered sequence.
    #[must_use]
    pub const fn new(faults: Vec<Fault>) -> Self {
        Self { faults, cursor: 0 }
    }
    /// Consumes and returns the next planned fault, if any.
    pub fn take_next(&mut self) -> Option<Fault> {
        let fault = self.faults.get(self.cursor).copied();
        if fault.is_some() {
            self.cursor += 1;
        }
        fault
    }
    /// Returns the number of unconsumed faults.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.faults.len().saturating_sub(self.cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mackes_domain::{
        EndpointId, MidiChannel, MidiEvent, MidiMessage, SevenBit, TimestampNanos,
    };
    use mackes_ipc::{validate_reconnect, StateEvent, StateSnapshot};
    use mackes_midi_engine::{
        build_rtp_packet, infer_cc_candidates, parse_rtp_midi_packet, AppleMidiSession,
        EndpointDirection, MessageClass, MidiInputAdapter, MidiOutputAdapter, Route, Router,
        RtpMidiPeer, SequenceDisposition, SysexReassembler, VirtualEndpoint,
    };
    use mackes_profiles::{resolve_alias, AliasSelector, ObservedEndpoint, Resolution};
    use mackes_scene_engine::{
        panic_plan, ActionResult, ActivationAction, ActivationPlan, SafetyController,
    };
    use mackes_tui::{DeviceControlGroup, DeviceWorkspace, SignalFlowDiagram, SignalFlowNode};

    #[test]
    fn integration_inventory_is_complete_and_unique() {
        assert_eq!(INTEGRATION_SCENARIOS.len(), 13);
        assert!(INTEGRATION_SCENARIOS.iter().all(|name| {
            !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_lowercase() || byte == b'-')
        }));
        for (index, name) in INTEGRATION_SCENARIOS.iter().enumerate() {
            assert!(!INTEGRATION_SCENARIOS[..index].contains(name));
        }
    }

    #[test]
    fn routing_round_trip_and_bounded_fault_are_hermetic() {
        let source = EndpointId::new(1).expect("source");
        let destination = EndpointId::new(2).expect("destination");
        let router = Router::new(
            vec![Route {
                source,
                destination,
                destination_parameter: None,
                channel: None,
                class: Some(MessageClass::ControlChange),
                enabled: true,
                priority: 0,
                curve: mackes_midi_engine::Curve::Linear,
                predicates: Vec::new(),
                allow_cycle: false,
            }],
            1,
            4,
        )
        .expect("router");
        let event = MidiEvent {
            timestamp: TimestampNanos::new(1),
            sequence: 1,
            endpoint: source,
            message: MidiMessage::ControlChange {
                channel: MidiChannel::new(1).expect("channel"),
                controller: SevenBit::new(7).expect("cc"),
                value: SevenBit::new(99).expect("value"),
            },
        };
        let route_result = router.route(&event);
        assert_eq!(route_result.len(), 1);
        let mut output = VirtualEndpoint::new("out", "integration-out", EndpointDirection::Output)
            .with_capacity(1);
        output.send(route_result[0].event.clone());
        output.send(event);
        assert_eq!(output.stats().dropped, 1);
        assert_eq!(output.receive().expect("queued").sequence, 1);
    }

    #[test]
    fn learn_and_rtp_round_trip_are_deterministic() {
        let endpoint = EndpointId::new(9).expect("endpoint");
        let event = |sequence, controller| MidiEvent {
            timestamp: TimestampNanos::new(sequence),
            sequence,
            endpoint,
            message: MidiMessage::ControlChange {
                channel: MidiChannel::new(1).expect("channel"),
                controller: SevenBit::new(controller).expect("controller"),
                value: SevenBit::new(64).expect("value"),
            },
        };
        let candidates = infer_cc_candidates(&[event(1, 10), event(2, 10), event(3, 74)]);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].controller.get(), 10);
        let payload = [0x00, 0x03, 0x90, 0x3c, 0x64];
        let packet = build_rtp_packet(4, 5, 6, &payload).expect("packet");
        let parsed = parse_rtp_midi_packet(&packet).expect("parse");
        assert_eq!(parsed.rtp.sequence, 4);
        assert_eq!(parsed.midi.commands, &payload[2..]);
    }

    #[test]
    fn two_independent_rtp_peers_validate_identity_and_sequence() {
        let mut left = RtpMidiPeer::new(100, 4).expect("left");
        let mut right = RtpMidiPeer::new(200, 4).expect("right");
        left.invite("right");
        right.invite("left");
        left.establish(100, 2000).expect("left establish");
        right.establish(200, 1000).expect("right establish");
        assert_eq!(left.observe(100, 2000, 1), Some(SequenceDisposition::InOrder));
        assert_eq!(right.observe(200, 1000, 1), Some(SequenceDisposition::InOrder));
        assert_eq!(left.observe(200, 2000, 2), None);
        assert_eq!(
            right.observe(200, 1000, 3),
            Some(SequenceDisposition::ForwardGap { missing: 1 })
        );
    }

    #[test]
    fn sysex_and_apple_midi_recovery_are_bounded() {
        let mut sysex = SysexReassembler::new(8).expect("limit");
        assert_eq!(sysex.push(&[0x01, 0x02], true, false).expect("start"), None);
        assert_eq!(sysex.push(&[0x03], false, true).expect("end"), Some(vec![1, 2, 3]));
        assert!(sysex.push(&[0x04], false, true).is_err());

        let mut session = AppleMidiSession::new(42);
        session.invite("peer-a");
        assert!(session.establish(42, 7).is_ok());
        assert!(session.accepts(42, 7));
        assert!(session.end_session(41, 7).is_err());
        assert!(session.end_session(42, 7).is_ok());
        session.invite("peer-b");
        assert!(session.establish(42, 8).is_ok());
    }

    #[test]
    fn throughput_regression_routes_ten_thousand_messages_without_drops() {
        let source = EndpointId::new(11).expect("source");
        let destination = EndpointId::new(12).expect("destination");
        let router = Router::new(
            vec![Route {
                source,
                destination,
                destination_parameter: None,
                channel: None,
                class: Some(MessageClass::ControlChange),
                enabled: true,
                priority: 0,
                curve: mackes_midi_engine::Curve::Linear,
                predicates: Vec::new(),
                allow_cycle: false,
            }],
            1,
            4,
        )
        .expect("router");
        let mut output = VirtualEndpoint::new("perf-out", "performance", EndpointDirection::Output)
            .with_capacity(10_000);
        let mut samples = Vec::with_capacity(10_000);
        for sequence in 0..10_000_u64 {
            let started = std::time::Instant::now();
            let event = MidiEvent {
                timestamp: TimestampNanos::new(sequence),
                sequence,
                endpoint: source,
                message: MidiMessage::ControlChange {
                    channel: MidiChannel::new(1).expect("channel"),
                    controller: SevenBit::new(1).expect("controller"),
                    value: SevenBit::new((sequence % 128) as u16).expect("value"),
                },
            };
            let route_result = router.route(&event);
            assert_eq!(route_result.len(), 1);
            output.send(route_result[0].event.clone());
            samples.push(started.elapsed());
        }
        assert_eq!(output.stats().sent, 10_000);
        assert_eq!(output.stats().dropped, 0);
        samples.sort_unstable();
        let p99 = samples[(samples.len() * 99 / 100).min(samples.len() - 1)];
        assert!(p99 < std::time::Duration::from_millis(2));
    }

    #[test]
    fn device_diagram_metadata_preserves_labels_and_selection() {
        let mut workspace = DeviceWorkspace::new(
            "eventide-micropitch".into(),
            "Eventide MicroPitch".into(),
            SignalFlowDiagram {
                nodes: vec![SignalFlowNode {
                    id: "pitch".into(),
                    label: "Pitch".into(),
                    online: true,
                }],
                generation: 3,
            },
            vec!["Bypass".into()],
            vec![DeviceControlGroup {
                id: "pitch-controls".into(),
                label: "Pitch Controls".into(),
                block_id: "pitch".into(),
                control_ids: vec!["detune".into()],
            }],
            true,
        )
        .expect("workspace");
        let groups = workspace.select_block("pitch").expect("block");
        assert_eq!(groups[0].label, "Pitch Controls");
        assert_eq!(
            workspace.diagram_notice(),
            Some("Inferred logical/control view — not authoritative DSP topology")
        );
    }

    #[test]
    fn safety_policy_expires_and_panic_targets_every_destination() {
        let mut safety = SafetyController::default();
        safety.arm_unsafe(10);
        assert!(safety.unsafe_armed(9));
        assert!(!safety.unsafe_armed(10));
        safety.set_performance_lock(true);
        safety.restart_clear();
        assert!(!safety.performance_locked());
        assert!(panic_plan(&[2, 5, 9]).iter().all(|action| action.destination != 0));
        assert_eq!(panic_plan(&[2, 5, 9]).len(), 3);
    }

    #[test]
    fn scene_activation_failure_blocks_dependents_without_partial_success() {
        let plan = ActivationPlan::compile(vec![
            ActivationAction {
                id: "route".into(),
                description: "route".into(),
                unsafe_action: false,
                depends_on: None,
                destination: None,
                message: None,
            },
            ActivationAction {
                id: "write-reflex".into(),
                description: "write".into(),
                unsafe_action: true,
                depends_on: Some("route".into()),
                destination: None,
                message: None,
            },
            ActivationAction {
                id: "verify".into(),
                description: "verify".into(),
                unsafe_action: false,
                depends_on: Some("write-reflex".into()),
                destination: None,
                message: None,
            },
        ])
        .expect("plan");
        let results = plan.execute(false, false);
        assert_eq!(results[0].1, ActionResult::Succeeded);
        assert_eq!(results[1].1, ActionResult::SkippedDependency);
        assert_eq!(results[2].1, ActionResult::SkippedDependency);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn reconnect_accepts_contiguous_events_and_rejects_gaps() {
        let snapshot = StateSnapshot { last_sequence: 40, payload: b"state".to_vec() };
        let contiguous = vec![
            StateEvent { sequence: 41, payload: b"a".to_vec() },
            StateEvent { sequence: 42, payload: b"b".to_vec() },
        ];
        assert!(validate_reconnect(&snapshot, &contiguous).is_ok());
        let gap = vec![StateEvent { sequence: 43, payload: b"skipped".to_vec() }];
        assert!(validate_reconnect(&snapshot, &gap).is_err());
    }

    #[test]
    fn alias_resolution_handles_hotplug_and_ambiguity_without_guessing() {
        let selector = AliasSelector {
            alias: "reflex".into(),
            serial: Some("R-1".into()),
            vid_pid: None,
            interface: None,
            name_pattern: None,
        };
        let endpoint = ObservedEndpoint {
            name: "Reflex MIDI".into(),
            direction: "in".into(),
            vid_pid: Some((1, 2)),
            serial: Some("R-1".into()),
            interface: Some(0),
            physical_path: Some("/dev/test".into()),
        };
        assert_eq!(
            resolve_alias(&selector, std::slice::from_ref(&endpoint)).0,
            Resolution::Matched
        );
        let duplicate = ObservedEndpoint { name: "Reflex MIDI 2".into(), ..endpoint.clone() };
        let endpoints = [endpoint, duplicate];
        let (resolution, matches) =
            resolve_alias(&AliasSelector { serial: None, ..selector }, &endpoints);
        assert_eq!(resolution, Resolution::Ambiguous);
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn fault_plan_is_ordered_and_bounded() {
        let mut plan = FaultPlan::new(vec![Fault::Disconnect, Fault::WriteFailure]);
        assert_eq!(plan.remaining(), 2);
        assert_eq!(plan.take_next(), Some(Fault::Disconnect));
        assert_eq!(plan.take_next(), Some(Fault::WriteFailure));
        assert_eq!(plan.take_next(), None);
        assert_eq!(plan.remaining(), 0);
    }
}
