//! Characterization tests for public crate behavior.

use super::*;
use mackes_domain::{
    FourteenBit, MidiChannel, MidiMessage, SystemCommonMessage, TimestampNanos,
};

#[test]
fn virtual_endpoint_preserves_output_order() {
    let mut endpoint =
        VirtualEndpoint::new("virtual-out", "MACKES DAW Out", EndpointDirection::Output);
    for note in [60, 61] {
        endpoint.send(MidiEvent {
            timestamp: TimestampNanos::new(u64::from(note)),
            sequence: u64::from(note),
            endpoint: mackes_domain::EndpointId::new(1).expect("endpoint"),
            message: MidiMessage::NoteOn {
                channel: MidiChannel::new(1).expect("channel"),
                note: mackes_domain::SevenBit::new(note).expect("note"),
                velocity: mackes_domain::SevenBit::new(100).expect("velocity"),
            },
        });
    }
    assert_eq!(endpoint.receive().expect("first").sequence, 60);
    assert_eq!(endpoint.receive().expect("second").sequence, 61);
    assert!(endpoint.receive().is_none());
}

#[test]
fn router_filters_compound_route_and_enforces_hop_limit() {
    let source = mackes_domain::EndpointId::new(1).expect("source");
    let destination = mackes_domain::EndpointId::new(2).expect("destination");
    let router = Router::new(
        vec![Route {
            source,
            destination,
            destination_parameter: None,
            channel: Some(MidiChannel::new(2).expect("channel")),
            class: Some(MessageClass::ControlChange),
            enabled: true,
            priority: 0,
            curve: Curve::Linear,
            predicates: Vec::new(),
            allow_cycle: false,
        }],
        7,
        2,
    )
    .expect("router");
    let event = MidiEvent {
        timestamp: TimestampNanos::new(1),
        sequence: 1,
        endpoint: source,
        message: MidiMessage::ControlChange {
            channel: MidiChannel::new(2).expect("channel"),
            controller: mackes_domain::SevenBit::new(10).expect("cc"),
            value: mackes_domain::SevenBit::new(64).expect("value"),
        },
    };
    assert_eq!(router.route(&event).len(), 1);
    assert_eq!(router.route_with_hops(&event, 2), Vec::new());
    let wrong_channel = MidiEvent {
        timestamp: event.timestamp,
        sequence: event.sequence,
        endpoint: event.endpoint,
        message: MidiMessage::ControlChange {
            channel: MidiChannel::new(1).expect("channel"),
            controller: mackes_domain::SevenBit::new(10).expect("cc"),
            value: mackes_domain::SevenBit::new(64).expect("value"),
        },
    };
    assert!(router.route(&wrong_channel).is_empty());
    let store = RouterStore::new(Vec::new(), 7, 2).expect("store");
    assert_eq!(store.generation(), Some(7));
    assert!(store.routes().is_empty());
    store.swap(Vec::new(), 8, 2).expect("swap");
    assert_eq!(store.generation(), Some(8));
}

#[test]
fn router_enforces_enabled_priority_and_cc_curve() {
    let source = EndpointId::new(1).expect("source");
    let first = EndpointId::new(2).expect("first");
    let disabled = EndpointId::new(3).expect("disabled");
    let router = Router::new(
        vec![
            Route {
                source,
                destination: disabled,
                destination_parameter: None,
                channel: None,
                class: Some(MessageClass::ControlChange),
                enabled: false,
                priority: 0,
                curve: Curve::Linear,
                predicates: Vec::new(),
                allow_cycle: false,
            },
            Route {
                source,
                destination: first,
                destination_parameter: None,
                channel: None,
                class: Some(MessageClass::ControlChange),
                enabled: true,
                priority: 7,
                curve: Curve::Square,
                predicates: Vec::new(),
                allow_cycle: false,
            },
        ],
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
            controller: SevenBit::new(10).expect("controller"),
            value: SevenBit::new(64).expect("value"),
        },
    };
    let outputs = router.route(&event);
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].event.endpoint, first);
    assert_eq!(
        outputs[0].event.message,
        MidiMessage::ControlChange {
            channel: MidiChannel::new(1).expect("channel"),
            controller: SevenBit::new(10).expect("controller"),
            value: SevenBit::new(apply_curve(64, Curve::Square).into()).expect("curve"),
        }
    );
}

#[test]
fn router_applies_number_value_realtime_and_masked_sysex_predicates() {
    let source = EndpointId::new(1).expect("source");
    let destination = EndpointId::new(2).expect("destination");
    let event = |message| MidiEvent {
        timestamp: TimestampNanos::new(1),
        sequence: 1,
        endpoint: source,
        message,
    };
    let route = |predicates| Route {
        source,
        destination,
        destination_parameter: None,
        channel: None,
        class: None,
        enabled: true,
        priority: 0,
        curve: Curve::Linear,
        predicates,
        allow_cycle: false,
    };
    let cc = event(MidiMessage::ControlChange {
        channel: MidiChannel::new(1).expect("channel"),
        controller: SevenBit::new(10).expect("controller"),
        value: SevenBit::new(64).expect("value"),
    });
    let router = Router::new(
        vec![route(vec![
            RoutePredicate::NumberRange { minimum: 10, maximum: 12 },
            RoutePredicate::ValueRange { minimum: 60, maximum: 70 },
        ])],
        1,
        4,
    )
    .expect("CC router");
    assert_eq!(router.route(&cc).len(), 1);

    let realtime =
        Router::new(vec![route(vec![RoutePredicate::Realtime(RealtimeMessage::Clock)])], 1, 4)
            .expect("real-time router");
    assert_eq!(realtime.route(&event(MidiMessage::Realtime(RealtimeMessage::Clock))).len(), 1);
    assert!(realtime.route(&event(MidiMessage::Realtime(RealtimeMessage::Stop))).is_empty());

    let sysex = Router::new(
        vec![route(vec![RoutePredicate::SysExMask {
            pattern: vec![0x06, 0x00, 0x10],
            mask: vec![0x7f, 0x7f, 0x70],
        }])],
        1,
        4,
    )
    .expect("SysEx router");
    assert_eq!(
        sysex.route(&event(MidiMessage::sysex([0x06, 0x00, 0x1f]).expect("SysEx"))).len(),
        1
    );
    assert!(sysex.route(&event(MidiMessage::sysex([0x06, 0x01, 0x1f]).expect("SysEx"))).is_empty());
    assert!(Router::new(
        vec![route(vec![RoutePredicate::NumberRange { minimum: 12, maximum: 10 }])],
        1,
        4,
    )
    .is_err());
}

#[test]
fn router_rejects_accidental_cycles_and_bounds_explicit_cycles() {
    let one = EndpointId::new(1).expect("one");
    let two = EndpointId::new(2).expect("two");
    let edge = |source, destination, allow_cycle| Route {
        source,
        destination,
        destination_parameter: None,
        channel: None,
        class: None,
        enabled: true,
        priority: 0,
        curve: Curve::Linear,
        predicates: Vec::new(),
        allow_cycle,
    };
    assert!(Router::new(vec![edge(one, two, false), edge(two, one, false)], 1, 4).is_err());
    let router = Router::new(vec![edge(one, two, true), edge(two, one, true)], 1, 2)
        .expect("explicit bounded cycle");
    let event = MidiEvent {
        timestamp: TimestampNanos::new(1),
        sequence: 1,
        endpoint: one,
        message: MidiMessage::Realtime(RealtimeMessage::Clock),
    };
    let first = router.route(&event);
    assert_eq!(first.len(), 1);
    assert_eq!(router.route_with_hops(&first[0].event, first[0].hops).len(), 1);
    assert!(router.route_with_hops(&event, 2).is_empty());
}

#[test]
fn dispatch_routes_to_matching_output_and_counts_unmatched() {
    let source = mackes_domain::EndpointId::new(1).expect("source");
    let destination = mackes_domain::EndpointId::new(2).expect("destination");
    let router = RouterStore::new(
        vec![Route {
            source,
            destination,
            destination_parameter: None,
            channel: None,
            class: None,
            enabled: true,
            priority: 0,
            curve: Curve::Linear,
            predicates: Vec::new(),
            allow_cycle: false,
        }],
        1,
        2,
    )
    .expect("router");
    let event = MidiEvent {
        timestamp: TimestampNanos::new(1),
        sequence: 9,
        endpoint: source,
        message: MidiMessage::NoteOn {
            channel: MidiChannel::new(1).expect("channel"),
            note: mackes_domain::SevenBit::new(60).expect("note"),
            velocity: mackes_domain::SevenBit::new(100).expect("velocity"),
        },
    };
    let mut output = VirtualEndpoint::new("2", "destination", EndpointDirection::Output);
    let mut outputs: Vec<&mut dyn MidiOutputAdapter> = vec![&mut output];
    assert_eq!(dispatch_routed_event(&router, &event, &mut outputs), (1, 0));
    assert_eq!(output.stats().sent, 1);
    let mut none: Vec<&mut dyn MidiOutputAdapter> = Vec::new();
    assert_eq!(dispatch_routed_event(&router, &event, &mut none), (0, 1));
}

#[test]
fn output_registry_rejects_duplicates_and_capacity_overflow() {
    let mut registry = OutputRegistry::new(1);
    registry
        .insert(Box::new(VirtualEndpoint::new("1", "one", EndpointDirection::Output)))
        .expect("first output");
    assert_eq!(registry.len(), 1);
    assert!(registry
        .insert(Box::new(VirtualEndpoint::new("1", "duplicate", EndpointDirection::Output)))
        .is_err());
    assert!(registry
        .insert(Box::new(VirtualEndpoint::new("2", "full", EndpointDirection::Output)))
        .is_err());
    assert!(registry.remove("1"));
    assert!(!registry.remove("missing"));
    assert!(registry.is_empty());
}

#[test]
fn direct_output_send_is_named_bounded_and_validated() {
    let mut registry = OutputRegistry::new(1);
    registry
        .insert(Box::new(VirtualEndpoint::new("out", "output", EndpointDirection::Output)))
        .expect("output");
    registry.send_direct("out", &[0xF0, 0x7D, 0x01, 0xF7]).expect("SysEx send");
    assert_eq!(
        registry.send_direct("missing", &[0xF0, 0x7D, 0xF7]),
        Err("destination output is not registered".into())
    );
    assert!(registry.send_direct("out", &[0xF0, 0x80, 0xF7]).is_err());
}

#[test]
fn cc_mapping_remaps_controller_and_channel() {
    let event = MidiEvent {
        timestamp: TimestampNanos::new(1),
        sequence: 1,
        endpoint: mackes_domain::EndpointId::new(1).expect("endpoint"),
        message: MidiMessage::ControlChange {
            channel: MidiChannel::new(1).expect("channel"),
            controller: mackes_domain::SevenBit::new(10).expect("cc"),
            value: mackes_domain::SevenBit::new(64).expect("value"),
        },
    };
    let mapping = CcMapping {
        source_controller: mackes_domain::SevenBit::new(10).expect("cc"),
        destination_controller: mackes_domain::SevenBit::new(20).expect("cc"),
        destination_channel: Some(MidiChannel::new(2).expect("channel")),
    };
    assert!(
        matches!(mapping.apply(&event).expect("mapped").message, MidiMessage::ControlChange { channel, controller, .. } if channel == MidiChannel::new(2).expect("channel") && controller == mackes_domain::SevenBit::new(20).expect("cc"))
    );
    let candidates = infer_cc_candidates(&[event.clone(), event]);
    assert_eq!(
        candidates,
        vec![LearnCandidate {
            controller: mackes_domain::SevenBit::new(10).expect("cc"),
            observations: 2
        }]
    );
    assert_eq!(candidates[0].confidence_milli(4), 500);
    assert_eq!(candidates[0].confidence_milli(0), 0);
    assert_eq!(best_cc_candidate(&candidates), Some(candidates[0]));
}

#[test]
fn typed_number_mapping_preserves_family_and_rejects_conversion() {
    let event = MidiEvent {
        timestamp: TimestampNanos::new(1),
        sequence: 1,
        endpoint: EndpointId::new(1).expect("endpoint"),
        message: MidiMessage::NoteOn {
            channel: MidiChannel::new(1).expect("channel"),
            note: SevenBit::new(60).expect("note"),
            velocity: SevenBit::new(100).expect("velocity"),
        },
    };
    let mapping = TypedNumberMapping {
        source_kind: TypedMappingKind::Note,
        source_number: SevenBit::new(60).expect("source"),
        destination_kind: TypedMappingKind::Note,
        destination_number: SevenBit::new(64).expect("destination"),
        destination_channel: Some(MidiChannel::new(2).expect("channel")),
    };
    assert!(matches!(mapping.apply(&event).expect("mapped").message,
        MidiMessage::NoteOn { channel, note, velocity }
            if channel == MidiChannel::new(2).expect("channel")
                && note == SevenBit::new(64).expect("note")
                && velocity == SevenBit::new(100).expect("velocity")));
    let invalid =
        TypedNumberMapping { destination_kind: TypedMappingKind::ControlChange, ..mapping };
    assert!(invalid.apply(&event).is_none());
}

#[test]
fn conditional_typed_mapping_is_bounded_and_deterministic() {
    let event = MidiEvent {
        timestamp: TimestampNanos::new(1),
        sequence: 1,
        endpoint: EndpointId::new(1).expect("endpoint"),
        message: MidiMessage::ControlChange {
            channel: MidiChannel::new(1).expect("channel"),
            controller: SevenBit::new(10).expect("controller"),
            value: SevenBit::new(64).expect("value"),
        },
    };
    let mapping = ConditionalTypedMapping {
        mapping: TypedNumberMapping {
            source_kind: TypedMappingKind::ControlChange,
            source_number: SevenBit::new(10).expect("source"),
            destination_kind: TypedMappingKind::ControlChange,
            destination_number: SevenBit::new(20).expect("destination"),
            destination_channel: None,
        },
        minimum: 64,
        maximum: 127,
    };
    assert!(mapping.apply(&event).is_some());
    let below = MidiEvent {
        message: MidiMessage::ControlChange {
            channel: MidiChannel::new(1).expect("channel"),
            controller: SevenBit::new(10).expect("controller"),
            value: SevenBit::new(63).expect("value"),
        },
        ..event
    };
    assert!(mapping.apply(&below).is_none());
}

#[test]
fn generalized_learn_groups_every_midi_family_with_raw_evidence() {
    let endpoint = EndpointId::new(1).expect("endpoint");
    let channel = MidiChannel::new(2).expect("channel");
    let event = |sequence, message| MidiEvent {
        timestamp: TimestampNanos::new(sequence),
        sequence,
        endpoint,
        message,
    };
    let messages = vec![
        event(
            1,
            MidiMessage::NoteOn {
                channel,
                note: SevenBit::new(60).expect("note"),
                velocity: SevenBit::new(10).expect("velocity"),
            },
        ),
        event(
            2,
            MidiMessage::NoteOff {
                channel,
                note: SevenBit::new(60).expect("note"),
                velocity: SevenBit::new(0).expect("velocity"),
            },
        ),
        event(
            3,
            MidiMessage::ControlChange {
                channel,
                controller: SevenBit::new(7).expect("CC"),
                value: SevenBit::new(10).expect("value"),
            },
        ),
        event(
            4,
            MidiMessage::ControlChange {
                channel,
                controller: SevenBit::new(7).expect("CC"),
                value: SevenBit::new(100).expect("value"),
            },
        ),
        event(
            5,
            MidiMessage::ProgramChange { channel, program: SevenBit::new(4).expect("program") },
        ),
        event(
            6,
            MidiMessage::ChannelPressure {
                channel,
                pressure: SevenBit::new(50).expect("pressure"),
            },
        ),
        event(7, MidiMessage::PitchBend { channel, value: FourteenBit::new(8192).expect("bend") }),
        event(
            8,
            MidiMessage::SystemCommon(SystemCommonMessage::SongSelect(
                SevenBit::new(3).expect("song"),
            )),
        ),
        event(9, MidiMessage::Realtime(RealtimeMessage::Clock)),
        event(10, MidiMessage::sysex([0x06, 0x00, 0x10]).expect("SysEx")),
    ];
    let candidates = infer_midi_candidates(&messages);
    assert_eq!(candidates.len(), 9);
    let cc = candidates
        .iter()
        .find(|candidate| candidate.kind == LearnMessageKind::ControlChange)
        .expect("CC candidate");
    assert_eq!((cc.channel, cc.number, cc.observations), (Some(2), Some(7), 2));
    assert_eq!((cc.minimum, cc.maximum), (Some(10), Some(100)));
    let sysex = candidates
        .iter()
        .find(|candidate| candidate.kind == LearnMessageKind::SysEx)
        .expect("SysEx candidate");
    assert_eq!(sysex.raw, vec![0xF0, 0x06, 0x00, 0x10, 0xF7]);
    assert!(candidates.iter().all(|candidate| !candidate.raw.is_empty()));
}

#[test]
fn scaling_is_bounded_and_invertible_at_endpoints() {
    assert_eq!(scale_value(0, (0, 127), (0, 100), false), Ok(0));
    assert_eq!(scale_value(127, (0, 127), (0, 100), false), Ok(100));
    assert_eq!(scale_value(0, (0, 127), (0, 100), true), Ok(100));
    assert!(scale_value(128, (0, 127), (0, 100), false).is_err());
}

#[test]
fn pickup_accepts_only_within_tolerance() {
    assert!(pickup_accept(64, 60, 4));
    assert!(!pickup_accept(65, 60, 4));
    let mut state = PickupState::new(60, 2);
    assert!(!state.accept(64));
    assert!(state.accept(62));
    assert!(state.accept(100));
    state.reset(100);
    assert!(!state.accept(60));
}

#[test]
fn takeover_modes_prevent_scene_jumps_and_bound_relative_values() {
    let mut jump = TakeoverState::new(TakeoverMode::Jump, 100).expect("jump");
    assert_eq!(jump.apply(10), Ok(Some(10)));

    let mut pickup = TakeoverState::new(TakeoverMode::Pickup { tolerance: 2 }, 60).expect("pickup");
    assert_eq!(pickup.apply(20), Ok(None));
    assert_eq!(pickup.apply(58), Ok(Some(58)));
    assert_eq!(pickup.apply(100), Ok(Some(100)));
    pickup.reset(40).expect("new scene");
    assert_eq!(pickup.apply(100), Ok(None));

    let mut scaled = TakeoverState::new(
        TakeoverMode::ScaledPickup { input: (0, 127), output: (0, 1000), tolerance: 8 },
        500,
    )
    .expect("scaled pickup");
    assert_eq!(scaled.apply(10), Ok(None));
    assert_eq!(scaled.apply(64), Ok(Some(503)));

    let mut relative = TakeoverState::new(TakeoverMode::Relative { step: 2, range: (0, 100) }, 50)
        .expect("relative");
    assert_eq!(relative.apply(3), Ok(Some(56)));
    assert_eq!(relative.apply(126), Ok(Some(52)));
    assert_eq!(relative.apply(64), Ok(Some(52)));
    assert_eq!(relative.apply(63), Ok(Some(100)));
    assert_eq!(relative.apply(127), Ok(Some(98)));
    assert_eq!(relative.target(), 98);
    assert!(relative.apply(128).is_err());
}

#[test]
fn mapping_state_store_is_page_isolated_and_edge_triggered() {
    let mut store = MappingStateStore::default();
    store
        .register(
            "effect",
            "practice",
            StatefulMode::Toggle { on_value: 127 },
            MappingReset::Off,
            0,
        )
        .expect("toggle");
    store
        .register(
            "effect",
            "device",
            StatefulMode::Latch { on_value: 100 },
            MappingReset::Preserve,
            0,
        )
        .expect("page-isolated latch");
    assert_eq!(store.apply("effect", "practice", 127).expect("press")[0].value, 127);
    assert!(store.apply("effect", "practice", 127).expect("held").is_empty());
    assert!(store.apply("effect", "practice", 0).expect("release").is_empty());
    assert_eq!(store.apply("effect", "practice", 127).expect("second press")[0].value, 0);
    assert_eq!(store.apply("effect", "device", 127).expect("latch press")[0].value, 100);
    assert_eq!(store.apply("effect", "device", 0).expect("latch release")[0].value, 0);
}

#[test]
fn radio_step_and_scene_reset_are_deterministic() {
    let mut store = MappingStateStore::default();
    for id in ["scene-a", "scene-b"] {
        store
            .register(
                id,
                "practice",
                StatefulMode::Radio { group: "scenes".into(), on_value: 127 },
                MappingReset::Off,
                0,
            )
            .expect("radio");
    }
    store
        .register(
            "division",
            "practice",
            StatefulMode::Step { values: vec![0, 32, 64, 127] },
            MappingReset::SceneDefault,
            32,
        )
        .expect("step");
    assert_eq!(store.apply("scene-a", "practice", 127).expect("radio a").len(), 1);
    store.apply("scene-a", "practice", 0).expect("release a");
    let changes = store.apply("scene-b", "practice", 127).expect("radio b");
    assert_eq!(
        changes.iter().map(|change| (&*change.mapping_id, change.value)).collect::<Vec<_>>(),
        vec![("scene-a", 0), ("scene-b", 127)]
    );
    assert_eq!(store.apply("division", "practice", 127).expect("step")[0].value, 64);
    assert_eq!(
        store
            .reset_scene("practice")
            .iter()
            .map(|change| (&*change.mapping_id, change.value))
            .collect::<Vec<_>>(),
        vec![("division", 32), ("scene-b", 0)]
    );
    assert_eq!(store.value("division", "practice"), Some(32));
}

#[test]
fn curves_preserve_midi_endpoints() {
    for curve in [Curve::Linear, Curve::Square, Curve::SquareRoot] {
        assert_eq!(apply_curve(0, curve), 0);
        assert_eq!(apply_curve(127, curve), 127);
    }
    assert!(apply_curve(64, Curve::Square) < 64);
}

#[test]
fn parameter_mapping_requires_exact_source_and_scales_cc_values() {
    let source = EndpointId::new(1).expect("source");
    let destination = EndpointId::new(2).expect("destination");
    let mapping = ParameterMapping {
        source_endpoint: source,
        destination_endpoint: destination,
        class: MessageClass::ControlChange,
        number: 21,
        channel: Some(0),
        source_range: (0, 127),
        destination_range: (10, 100),
        invert: false,
        curve: Curve::Linear,
    };
    let event = MidiEvent {
        timestamp: TimestampNanos::new(1),
        sequence: 2,
        endpoint: source,
        message: MidiMessage::ControlChange {
            channel: MidiChannel::new(1).expect("channel"),
            controller: SevenBit::new(21).expect("controller"),
            value: SevenBit::new(64).expect("value"),
        },
    };
    let mapped = mapping.evaluate(&event).expect("exact match");
    assert_eq!(mapped.endpoint, destination);
    assert!(
        matches!(mapped.message, MidiMessage::ControlChange { value, .. } if value.get() == 55)
    );
    assert!(mapping.evaluate(&MidiEvent { endpoint: source, ..event.clone() }).is_some());
    assert!(mapping.evaluate(&MidiEvent { endpoint: destination, ..event.clone() }).is_none());
    let wrong_channel = MidiEvent {
        message: MidiMessage::ControlChange {
            channel: MidiChannel::new(2).expect("channel"),
            controller: SevenBit::new(21).expect("controller"),
            value: SevenBit::new(64).expect("value"),
        },
        ..event
    };
    assert!(mapping.evaluate(&wrong_channel).is_none());
    let wide = ParameterMapping { destination_range: (100, 1000), ..mapping };
    assert_eq!(wide.evaluate_with_value(&event).expect("wide value").1, 553);
}

#[test]
fn parameter_mapping_handles_exact_program_change_buttons() {
    let mapping = ParameterMapping {
        source_endpoint: EndpointId::new(1).expect("source"),
        destination_endpoint: EndpointId::new(2).expect("destination"),
        class: MessageClass::ProgramChange,
        number: 7,
        channel: Some(0),
        source_range: (0, 127),
        destination_range: (1, 1),
        invert: false,
        curve: Curve::Linear,
    };
    let event = MidiEvent {
        timestamp: TimestampNanos::new(1),
        sequence: 1,
        endpoint: EndpointId::new(1).expect("source"),
        message: MidiMessage::ProgramChange {
            channel: MidiChannel::new(1).expect("channel"),
            program: SevenBit::new(7).expect("program"),
        },
    };
    let mapped = mapping.evaluate(&event).expect("button match");
    assert_eq!(mapped.endpoint, EndpointId::new(2).expect("destination"));
    assert!(
        matches!(mapped.message, MidiMessage::ProgramChange { program, .. } if program.get() == 1)
    );
    let near_miss = MidiEvent {
        message: MidiMessage::ProgramChange {
            channel: MidiChannel::new(1).expect("channel"),
            program: SevenBit::new(8).expect("program"),
        },
        ..event
    };
    assert!(mapping.evaluate(&near_miss).is_none());
}

#[test]
fn scheduler_orders_by_due_time_then_insertion_and_cancels() {
    let event = |sequence| MidiEvent {
        timestamp: TimestampNanos::new(1),
        sequence,
        endpoint: mackes_domain::EndpointId::new(1).expect("endpoint"),
        message: MidiMessage::Realtime(mackes_domain::RealtimeMessage::Clock),
    };
    let mut scheduler = Scheduler::default();
    scheduler.schedule(0, 20, event(1));
    scheduler.schedule(0, 10, event(2));
    scheduler.schedule(0, 10, event(3));
    scheduler.cancel_sequence(3);
    assert_eq!(
        scheduler.drain_due(10).iter().map(|item| item.sequence).collect::<Vec<_>>(),
        vec![2]
    );
    assert_eq!(
        scheduler.drain_due(20).iter().map(|item| item.sequence).collect::<Vec<_>>(),
        vec![1]
    );
}

#[test]
fn scheduler_cancel_all_clears_pending_chain() {
    let endpoint = mackes_domain::EndpointId::new(1).expect("endpoint");
    let event = |sequence| MidiEvent {
        timestamp: TimestampNanos::new(1),
        sequence,
        endpoint,
        message: MidiMessage::Realtime(mackes_domain::RealtimeMessage::Clock),
    };
    let mut scheduler = Scheduler::default();
    scheduler.schedule(0, 1, event(1));
    scheduler.schedule(0, 2, event(2));
    assert_eq!(scheduler.cancel_all(), 2);
    assert!(scheduler.drain_due(10).is_empty());
    assert_eq!(scheduler.cancel_all(), 0);
}

#[test]
fn retry_policy_is_capped_and_deterministic() {
    let policy = RetryPolicy { min_interval_ns: 100, retries: 3, retry_delay_ns: 50 };
    assert_eq!(policy.delay_for(0), 50);
    assert_eq!(policy.delay_for(3), 400);
    assert_eq!(policy.delay_for(255), 3_200);
}

#[test]
fn pending_transaction_enforces_timeout_pacing_and_retry_bound() {
    let policy = RetryPolicy { min_interval_ns: 10, retries: 1, retry_delay_ns: 5 };
    let mut pending = PendingTransaction::start(100, 50, policy).expect("transaction");
    assert!(!pending.timed_out(149));
    assert!(!pending.retry(105, 50, policy));
    assert!(pending.retry(110, 50, policy));
    assert_eq!(pending.attempts_sent, 2);
    assert!(!pending.retry(120, 50, policy));
    let matcher = ResponseMatcher::new(vec![0xf0, 1], vec![0xff, 0xff]).expect("matcher");
    assert!(pending.complete_if_matches(115, &matcher, &[0xf0, 1]));
    assert!(pending.completed);
    assert!(!pending.accepts_response(115, &matcher, &[0xf0, 1]));
    assert!(!pending.accepts_response(200, &matcher, &[0xf0, 1]));
    assert!(!pending.retry(130, 50, policy));
    assert!(PendingTransaction::start(0, 0, policy).is_none());
}

#[test]
fn response_matcher_is_exact_and_masked() {
    let matcher =
        ResponseMatcher::new(vec![0xf0, 0x01, 0x10], vec![0xff, 0xff, 0xf0]).expect("matcher");
    assert!(matcher.matches(&[0xf0, 0x01, 0x1f]));
    assert!(!matcher.matches(&[0xf0, 0x02, 0x1f]));
    assert!(!matcher.matches(&[0xf0, 0x01]));
    assert!(ResponseMatcher::new(Vec::new(), Vec::new()).is_err());
    assert!(ResponseMatcher::new(
        vec![0; ResponseMatcher::MAX_BYTES + 1],
        vec![0; ResponseMatcher::MAX_BYTES + 1]
    )
    .is_err());
}

#[test]
fn capture_correlation_retains_unsolicited_and_decodes_diffs() {
    let endpoint = EndpointId::new(1).expect("endpoint");
    let other = EndpointId::new(2).expect("other");
    let mut store = CaptureStore::new(3).expect("store");
    for record in [
        CaptureRecord { endpoint: other, timestamp_ns: 10, bytes: vec![1, 2, 3], matched: false },
        CaptureRecord { endpoint, timestamp_ns: 11, bytes: vec![1, 2, 4], matched: false },
        CaptureRecord { endpoint, timestamp_ns: 12, bytes: vec![1, 2, 3], matched: false },
    ] {
        store.push(record).expect("capture");
    }
    let matcher = ResponseMatcher::new(vec![1, 2, 3], vec![0x7f; 3]).expect("matcher");
    assert_eq!(
        store.correlate(endpoint, 10, 12, &matcher).map(|record| record.timestamp_ns),
        Some(12)
    );
    assert_eq!(store.unmatched().count(), 2);
    store
        .push(CaptureRecord { endpoint, timestamp_ns: 13, bytes: vec![7], matched: false })
        .expect("evict oldest");
    assert_eq!(store.records().front().map(|record| record.timestamp_ns), Some(11));
    assert!(store.correlate(endpoint, 20, 10, &matcher).is_none());

    let decoded = decode_capture_fields(
        &[0x01, 0x02, 0x03],
        &[
            CaptureField { name: "manufacturer".into(), offset: 0, length: 1 },
            CaptureField { name: "value".into(), offset: 1, length: 2 },
        ],
    )
    .expect("decode fields");
    assert_eq!(decoded, vec![("manufacturer".into(), 1), ("value".into(), 259)]);
    assert_eq!(diff_capture_bytes(&[1, 2, 3], &[1, 4]), vec![1, 2]);
    assert!(decode_capture_fields(
        &[0x80],
        &[CaptureField { name: "bad".into(), offset: 0, length: 1 }]
    )
    .is_err());
}

#[test]
fn device_request_validates_and_starts_transaction() {
    let matcher = ResponseMatcher::new(vec![0xf0, 1], vec![0xff, 0xff]).expect("matcher");
    let request = DeviceRequest::new(
        vec![0xf0, 1, 0xf7],
        matcher,
        RetryPolicy { min_interval_ns: 1, retries: 2, retry_delay_ns: 1 },
        10,
    )
    .expect("request");
    assert_eq!(request.begin(5).expect("pending").deadline_ns, 15);
    let event = request.to_event(3, 20, 9).expect("event");
    assert_eq!(event.endpoint.get(), 3);
    assert_eq!(event.message.wire_bytes(), vec![0xf0, 1, 0xf7]);
    assert!(DeviceRequest::new(Vec::new(), request.response.clone(), request.policy, 1).is_err());
}

#[test]
fn endpoint_ids_are_stable_and_direction_scoped() {
    let input = stable_endpoint_id("Launch Control XL", EndpointDirection::Input);
    assert_eq!(input, stable_endpoint_id("Launch Control XL", EndpointDirection::Input));
    assert_ne!(input, stable_endpoint_id("Launch Control XL", EndpointDirection::Output));
    assert_ne!(input, stable_endpoint_id("Launch Control XL HUI", EndpointDirection::Input));
    assert!(input.starts_with("midir-in-"));
}

#[test]
fn native_alsa_address_is_runtime_only_and_lifecycle_is_typed() {
    let address = AlsaSequencerAddress::new(24, 0);
    assert_eq!(address, AlsaSequencerAddress { client: 24, port: 0 });
    assert_ne!(AlsaSequencerLifecycle::Started, AlsaSequencerLifecycle::Exited);
    assert_eq!(AlsaSequencerLifecycle::Subscribed, AlsaSequencerLifecycle::Subscribed);
}

#[test]
fn virtual_port_names_are_stable_product_contracts() {
    assert_eq!(VIRTUAL_INPUT_NAME, "MACKES DAW In");
    assert_eq!(VIRTUAL_OUTPUT_NAME, "MACKES DAW Out");
}

#[test]
fn apple_midi_control_parser_rejects_truncation_and_unknown_commands() {
    let mut invitation = vec![0xff, 0xff, b'I', b'N'];
    invitation.resize(16, 0);
    assert_eq!(parse_apple_midi_command(&invitation), Ok(AppleMidiCommand::Invitation));
    assert_eq!(parse_apple_midi_command(&invitation[..8]), Err("truncated AppleMIDI command"));
    assert_eq!(
        parse_apple_midi_command(&[0xff, 0xff, b'X', b'X', 0, 0]),
        Err("unknown AppleMIDI command")
    );
    assert_eq!(
        parse_apple_midi_command(&[0x80, 0x60, 0x00, 0x01]),
        Err("not an AppleMIDI control packet")
    );
}

#[test]
fn apple_midi_session_requires_invitation_and_matching_identity() {
    let mut session = AppleMidiSession::new(42);
    assert!(!session.accepts(42, 7));
    assert!(session.establish(42, 7).is_err());
    session.invite("peer");
    assert!(session.establish(41, 7).is_err());
    assert!(session.establish(42, 0).is_err());
    session.establish(42, 7).expect("establish");
    assert!(session.accepts(42, 7));
    assert!(!session.accepts(42, 8));
    session.invite("replacement");
    assert_eq!(session.remote_name.as_deref(), Some("peer"));
    assert!(session.end_session(42, 8).is_err());
    session.end_session(42, 7).expect("end");
    session.invite("");
    assert_eq!(session.state, SessionState::Disconnected);
    session.disconnect();
    assert_eq!(session.state, SessionState::Disconnected);
}

#[test]
fn rtp_peer_binds_identity_to_sequence_and_resets_on_reconnect() {
    let mut peer = RtpMidiPeer::new(9, 4).expect("peer");
    assert_eq!(peer.state(), SessionState::Disconnected);
    peer.invite("peer");
    assert_eq!(peer.remote_name(), Some("peer"));
    peer.establish(9, 17).expect("establish");
    assert_eq!(peer.state(), SessionState::Established);
    assert_eq!(peer.remote_ssrc(), Some(17));
    assert_eq!(peer.observe(8, 17, 1), None);
    assert_eq!(peer.observe(9, 17, 1), Some(SequenceDisposition::InOrder));
    peer.disconnect();
    assert_eq!(peer.state(), SessionState::Disconnected);
    assert_eq!(peer.remote_ssrc(), None);
    peer.invite("peer");
    peer.establish(9, 17).expect("reconnect");
    assert_eq!(peer.observe(9, 17, 500), Some(SequenceDisposition::InOrder));
}

#[test]
fn rtp_peer_packet_ingest_enforces_allowlist_and_framing() {
    let mut peer = RtpMidiPeer::new(9, 4).expect("peer");
    peer.invite("peer");
    peer.establish(9, 17).expect("establish");
    let packet = build_rtp_packet(1, 2, 17, &[0, 1, 0x90]).expect("packet");
    let allowed = ["127.0.0.1:9000".parse().expect("address")];
    let denied = "127.0.0.1:9001".parse().expect("address");
    assert!(peer.receive_packet(&packet, denied, &allowed, 9, 17).is_err());
    let (parsed, disposition) =
        peer.receive_packet(&packet, allowed[0], &allowed, 9, 17).expect("ingest");
    assert_eq!(parsed.rtp.sequence, 1);
    assert_eq!(disposition, SequenceDisposition::InOrder);
}

#[test]
fn udp_transport_binds_nonblocking_and_reports_empty_queue() {
    let transport =
        UdpMidiTransport::bind("127.0.0.1:0".parse().expect("address"), 128).expect("bind");
    assert!(transport.local_addr().expect("local").port() > 0);
    assert!(transport.receive().expect("receive").is_none());
    assert!(UdpMidiTransport::bind("127.0.0.1:0".parse().expect("address"), 0).is_err());
    assert!(transport.send_to(&[0; 129], "127.0.0.1:9".parse().expect("peer")).is_err());
    let peer = "127.0.0.1:9".parse().expect("peer");
    assert_eq!(
        transport.send_to_allowed(&[1], peer, &[]).expect_err("deny").kind(),
        std::io::ErrorKind::PermissionDenied
    );
    let receiver = std::net::UdpSocket::bind("127.0.0.1:0").expect("receiver");
    receiver.set_read_timeout(Some(std::time::Duration::from_secs(1))).expect("timeout");
    let target = receiver.local_addr().expect("target");
    assert_eq!(transport.send_to(&[9, 8, 7], target).expect("send"), 3);
    let mut bytes = [0; 3];
    receiver.recv_from(&mut bytes).expect("datagram");
    assert_eq!(bytes, [9, 8, 7]);
    let sender = std::net::UdpSocket::bind("127.0.0.1:0").expect("sender");
    let sender_addr = sender.local_addr().expect("sender address");
    sender
        .send_to(&[4, 5], transport.local_addr().expect("transport address"))
        .expect("send inbound");
    assert!(transport.receive_from_allowed(&[]).expect("denied inbound").is_none());
    sender
        .send_to(&[4, 5], transport.local_addr().expect("transport address"))
        .expect("send inbound");
    assert_eq!(
        transport.receive_from_allowed(&[sender_addr]).expect("allowed inbound").expect("packet").0,
        vec![4, 5]
    );
    let mut peer_session = RtpMidiPeer::new(3, 4).expect("peer");
    peer_session.invite("loopback");
    peer_session.establish(3, 22).expect("establish");
    let packet = build_rtp_packet(8, 9, 22, &[0, 1, 0x90]).expect("packet");
    let local = transport.local_addr().expect("local");
    transport.send_to(&packet, local).expect("packet send");
    let allowlist = PeerAllowlist::new(vec![local]).expect("allowlist");
    let peer_packet = peer_session
        .receive_from_transport(&transport, &allowlist, 3, 22)
        .expect("peer receive")
        .expect("packet available");
    assert_eq!(peer_packet.1, SequenceDisposition::InOrder);
}

#[test]
fn rtp_header_parser_handles_csrc_extension_and_padding() {
    let packet =
        [0xB1, 0x61, 0x12, 0x34, 0, 0, 0, 9, 0, 0, 0, 7, 1, 2, 3, 4, 0, 1, 0, 0, 0x55, 0, 2];
    let parsed = parse_rtp_header(&packet).expect("RTP header");
    assert_eq!((parsed.sequence, parsed.timestamp, parsed.ssrc), (0x1234, 9, 7));
    assert_eq!(parsed.payload, &[0x55]);
    assert_eq!(parse_rtp_header(&[0; 11]), Err("invalid RTP header"));
}

#[test]
fn rtp_packet_builder_round_trips_header_parser() {
    let packet = build_rtp_packet(7, 99, 123, &[0x80, 0x01, 0x90]).expect("packet");
    let parsed = parse_rtp_header(&packet).expect("header");
    assert_eq!((parsed.sequence, parsed.timestamp, parsed.ssrc), (7, 99, 123));
    assert_eq!(parsed.payload, &[0x80, 0x01, 0x90]);
    assert_eq!(build_rtp_packet(0, 0, 0, &[]), Err("RTP payload must not be empty"));
    let combined = build_rtp_packet(8, 10, 12, &[0x80, 0x03, 0x90, 60, 127]).expect("packet");
    let parsed = parse_rtp_midi_packet(&combined).expect("combined");
    assert_eq!(parsed.rtp.sequence, 8);
    assert_eq!(parsed.midi.commands, &[0x90, 60, 127]);
    let peer = "127.0.0.1:5000".parse().expect("peer");
    assert!(validate_inbound_rtp_midi(&combined, peer, &[]).is_err());
    assert!(validate_inbound_rtp_midi(&combined, peer, &[peer]).is_ok());
}

#[test]
fn rtp_midi_payload_parser_enforces_command_length() {
    let payload = [0x80, 0x03, 0x90, 60, 127];
    let parsed = parse_rtp_midi_payload(&payload).expect("RTP-MIDI payload");
    assert!(parsed.begin);
    assert!(!parsed.dropped);
    assert_eq!(parsed.commands, &[0x90, 60, 127]);
    assert_eq!(
        parse_rtp_midi_payload(&[0x80, 0x04, 0x90, 60, 127]),
        Err("RTP-MIDI command length mismatch")
    );
    assert_eq!(parse_rtp_midi_payload(&[0x80]), Err("RTP-MIDI payload is truncated"));
}

#[test]
fn rtp_midi_channel_voice_decoder_supports_running_status() {
    let commands =
        decode_rtp_midi_channel_voice(&[0x90, 60, 127, 61, 0, 0xC0, 7]).expect("commands");
    assert_eq!(commands.len(), 3);
    assert_eq!(commands[1].status, 0x90);
    assert_eq!(commands[1].data, [61, 0]);
    assert_eq!(commands[2].data_len, 1);
    assert!(decode_rtp_midi_channel_voice(&[60, 1]).is_err());
    assert!(decode_rtp_midi_channel_voice(&[0xF8]).is_err());
}

#[test]
fn rtp_channel_command_converts_to_domain_message() {
    let message =
        rtp_command_to_message(RtpMidiCommand { status: 0x91, data: [60, 100], data_len: 2 })
            .expect("note-on");
    assert_eq!(message.wire_bytes(), vec![0x91, 60, 100]);
    assert!(
        rtp_command_to_message(RtpMidiCommand { status: 0xc0, data: [7, 0], data_len: 2 }).is_err()
    );
}

#[test]
fn rtp_channel_batch_decoder_preserves_order() {
    let messages = decode_rtp_channel_messages(&[0x90, 60, 100, 61, 0]).expect("batch");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].wire_bytes(), vec![0x90, 60, 100]);
    assert_eq!(messages[1].wire_bytes(), vec![0x90, 61, 0]);
}

#[test]
fn rtp_channel_events_assign_identity_and_sequence() {
    let events = rtp_channel_events(&[0x90, 60, 100, 61, 0], 7, 42, 10).expect("events");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].endpoint.get(), 7);
    assert_eq!(events[0].timestamp.get(), 42);
    assert_eq!(events[0].sequence, 10);
    assert_eq!(events[1].sequence, 11);
}

#[test]
fn rtp_system_events_assign_identity_and_sequence() {
    let messages = vec![
        RtpMidiSystemMessage { status: 0xf8, data: [0, 0], data_len: 0 },
        RtpMidiSystemMessage { status: 0xf6, data: [0, 0], data_len: 0 },
    ];
    let events = rtp_system_events(&messages, 9, 77, 20).expect("events");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].endpoint.get(), 9);
    assert_eq!(events[1].sequence, 21);
}

#[test]
fn rtp_sysex_requires_complete_framing() {
    let message = rtp_sysex_to_message(&[0xf0, 0x01, 0x7f, 0xf7]).expect("sysex");
    assert_eq!(message.wire_bytes(), vec![0xf0, 0x01, 0x7f, 0xf7]);
    assert!(rtp_sysex_to_message(&[0xf0, 0x01]).is_err());
    assert!(rtp_sysex_to_message(&[0xf0, 0x80, 0xf7]).is_err());
    let mut oversized = vec![0x01; 4097];
    oversized[0] = 0xf0;
    oversized[4096] = 0xf7;
    assert!(rtp_sysex_to_message(&oversized).is_err());
}

#[test]
fn rtp_sysex_event_assigns_metadata() {
    let event = rtp_sysex_event(&[0xf0, 1, 2, 0xf7], 4, 99, 8).expect("event");
    assert_eq!(event.endpoint.get(), 4);
    assert_eq!(event.timestamp.get(), 99);
    assert_eq!(event.sequence, 8);
}

#[test]
fn rtp_midi_system_decoder_handles_common_and_realtime() {
    let messages = decode_rtp_midi_system(&[0xF1, 3, 0xF2, 1, 2, 0xF8, 0xF6]).expect("system");
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[1].data, [1, 2]);
    assert_eq!(messages[2].data_len, 0);
    assert!(decode_rtp_midi_system(&[0xF2, 1]).is_err());
    assert!(decode_rtp_midi_system(&[0xF4]).is_err());
}

#[test]
fn sysex_reassembler_emits_only_complete_bounded_messages() {
    let mut reassembler = SysexReassembler::new(4).expect("limit");
    assert_eq!(reassembler.push(&[1, 2], true, false).expect("start"), None);
    assert_eq!(reassembler.push(&[3, 4], false, true).expect("end"), Some(vec![1, 2, 3, 4]));
    assert!(reassembler.push(&[1], false, true).is_err());
    assert!(reassembler.push(&[1, 2, 3, 4, 5], true, true).is_err());
    assert!(SysexReassembler::new(0).is_err());
    assert!(reassembler.push(&[0x80], true, true).is_err());
}

#[test]
fn sequence_tracker_classifies_gaps_duplicates_late_and_wraparound() {
    let mut tracker = SequenceTracker::new(4).expect("window");
    assert_eq!(tracker.observe(u16::MAX), SequenceDisposition::InOrder);
    assert_eq!(tracker.observe(0), SequenceDisposition::InOrder);
    assert_eq!(tracker.observe(2), SequenceDisposition::ForwardGap { missing: 1 });
    assert_eq!(tracker.observe(2), SequenceDisposition::Late);
    assert_eq!(tracker.observe(1), SequenceDisposition::Late);
    tracker.reset();
    assert_eq!(tracker.observe(500), SequenceDisposition::InOrder);
    assert_eq!(SequenceTracker::new(0), Err("RTP reorder window is out of bounds"));
}

#[test]
fn jitter_buffer_is_bounded_and_ordered() {
    let mut buffer = JitterBuffer::new(2).expect("capacity");
    buffer.push(JitterPacket { timestamp: 20, sequence: 2, payload: "late" }).expect("insert");
    buffer.push(JitterPacket { timestamp: 10, sequence: 1, payload: "first" }).expect("insert");
    assert_eq!(buffer.pop().expect("first").payload, "first");
    assert_eq!(buffer.pop().expect("second").payload, "late");
    assert!(buffer.push(JitterPacket { timestamp: 1, sequence: 1, payload: "x" }).is_ok());
    assert!(buffer.push(JitterPacket { timestamp: 2, sequence: 2, payload: "y" }).is_ok());
    assert!(buffer.push(JitterPacket { timestamp: 3, sequence: 3, payload: "z" }).is_err());
    assert_eq!(buffer.drain_until(1).len(), 1);
    assert!(buffer.pop().is_some());
}

#[test]
fn transport_stats_are_saturating_and_explicit() {
    let mut stats = TransportStats::default();
    stats.record_received();
    stats.record_sent();
    stats.record_malformed();
    stats.record_dropped();
    stats.record_late();
    stats.record_overflow();
    assert_eq!(
        stats,
        TransportStats { received: 1, sent: 1, malformed: 1, dropped: 1, late: 1, overflow: 1 }
    );
    stats.received = u64::MAX;
    stats.record_received();
    assert_eq!(stats.received, u64::MAX);
}

#[test]
fn physical_devices_group_matching_ports_deterministically() {
    let endpoints = vec![
        EndpointInfo {
            id: "out-b".into(),
            name: "Launch Control XL".into(),
            direction: EndpointDirection::Output,
        },
        EndpointInfo {
            id: "in-b".into(),
            name: "Launch Control XL".into(),
            direction: EndpointDirection::Input,
        },
        EndpointInfo {
            id: "in-a".into(),
            name: "MIDI Interface".into(),
            direction: EndpointDirection::Input,
        },
    ];
    let devices = group_physical_devices(&endpoints);
    assert_eq!(devices.len(), 2);
    assert_eq!(devices[0].name, "Launch Control XL");
    assert_eq!(devices[0].inputs, ["in-b"]);
    assert_eq!(devices[0].outputs, ["out-b"]);
    assert_eq!(devices[0].state, PhysicalDeviceState::Connected);
    assert_eq!(devices[1].name, "MIDI Interface");
    assert_eq!(devices[1].outputs, Vec::<String>::new());
}

#[test]
fn physical_device_grouping_does_not_merge_distinct_names() {
    let endpoints = vec![
        EndpointInfo {
            id: "a".into(),
            name: "Controller A".into(),
            direction: EndpointDirection::Input,
        },
        EndpointInfo {
            id: "b".into(),
            name: "Controller B".into(),
            direction: EndpointDirection::Input,
        },
    ];
    let devices = group_physical_devices(&endpoints);
    assert_eq!(
        devices.iter().map(|device| device.name.as_str()).collect::<Vec<_>>(),
        ["Controller A", "Controller B"]
    );
}

#[test]
fn physical_device_grouping_normalizes_alsa_device_prefixes() {
    let endpoints = vec![
        EndpointInfo {
            id: "in-a".into(),
            name: "Launch Control XL:Launch Control XL Launch Contro 28:0".into(),
            direction: EndpointDirection::Input,
        },
        EndpointInfo {
            id: "in-b".into(),
            name: "Launch Control XL:Launch Control XL HUI 28:1".into(),
            direction: EndpointDirection::Input,
        },
        EndpointInfo {
            id: "out-a".into(),
            name: "Launch Control XL:Launch Control XL Launch Contro 28:0".into(),
            direction: EndpointDirection::Output,
        },
    ];
    let devices = group_physical_devices(&endpoints);
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].name, "Launch Control XL");
    assert_eq!(devices[0].inputs, vec!["in-a", "in-b"]);
    assert_eq!(devices[0].outputs, vec!["out-a"]);
}

#[test]
fn activity_coalescer_keeps_newest_value_per_control() {
    let endpoint = mackes_domain::EndpointId::new(4).expect("endpoint");
    let event = |sequence, value| mackes_domain::MidiEvent {
        timestamp: mackes_domain::TimestampNanos::new(sequence),
        sequence,
        endpoint,
        message: mackes_domain::MidiMessage::ControlChange {
            channel: mackes_domain::MidiChannel::new(1).expect("channel"),
            controller: mackes_domain::SevenBit::new(21).expect("controller"),
            value: mackes_domain::SevenBit::new(value).expect("value"),
        },
    };
    let mut coalescer = ActivityCoalescer::new(2).expect("capacity");
    assert!(coalescer.push(&event(1, 10)));
    assert!(coalescer.push(&event(3, 99)));
    assert!(!coalescer.push(&event(2, 50)));
    assert_eq!(coalescer.len(), 1);
    let samples = coalescer.drain();
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].sequence, 3);
    assert_eq!(samples[0].value, Some(99));
    assert_eq!(coalescer.len(), 0);
}

#[test]
fn activity_coalescer_bounds_distinct_controls() {
    let event = |endpoint: u64, number: u8| mackes_domain::MidiEvent {
        timestamp: mackes_domain::TimestampNanos::new(u64::from(number)),
        sequence: u64::from(number),
        endpoint: mackes_domain::EndpointId::new(endpoint).expect("endpoint"),
        message: mackes_domain::MidiMessage::ControlChange {
            channel: mackes_domain::MidiChannel::new(1).expect("channel"),
            controller: mackes_domain::SevenBit::new(number.into()).expect("controller"),
            value: mackes_domain::SevenBit::new(number.into()).expect("value"),
        },
    };
    let mut coalescer = ActivityCoalescer::new(1).expect("capacity");
    assert!(coalescer.push(&event(2, 2)));
    assert!(!coalescer.push(&event(1, 1)));
    assert_eq!(coalescer.drain()[0].key.endpoint.get(), 2);
    assert!(ActivityCoalescer::new(0).is_none());
}

#[test]
fn peer_allowlist_is_bounded_unique_and_order_preserving() {
    let first = "127.0.0.1:5000".parse().expect("peer");
    let second = "127.0.0.1:5001".parse().expect("peer");
    let allowlist = PeerAllowlist::new(vec![first, second]).expect("allowlist");
    assert!(allowlist.contains(&first));
    assert_eq!(allowlist.peers(), &[first, second]);
    assert!(PeerAllowlist::new(vec![first, first]).is_err());
    assert!(PeerAllowlist::new(Vec::new()).is_err());
}
