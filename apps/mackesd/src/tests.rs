//! Daemon unit tests extracted from lib.rs.
use super::*;

#[test]
fn single_instance_lock_rejects_second_owner() {
    let path = std::env::temp_dir().join(format!("mackes-lock-{}", std::process::id()));
    let first = InstanceLock::acquire(&path).expect("first lock");
    assert!(InstanceLock::acquire(&path).is_err());
    drop(first);
    assert!(InstanceLock::acquire(&path).is_ok());
    let _ = fs::remove_file(path);
}

#[test]
fn health_operational_states_are_explicit() {
    assert!(!Health::Starting.is_operational());
    assert!(Health::Ready.is_operational());
    assert!(Health::Degraded.is_operational());
    assert!(!Health::Stopping.is_operational());
    assert_eq!(
        mapping_runtime::health_after_authorized_command(Health::Starting, Some(Command::Health)),
        Health::Starting
    );
    assert_eq!(
        mapping_runtime::health_after_authorized_command(Health::Starting, Some(Command::Snapshot)),
        Health::Ready
    );
    assert_eq!(
        mapping_runtime::health_after_authorized_command(Health::Degraded, Some(Command::Snapshot)),
        Health::Degraded
    );
}

#[test]
fn structured_log_line_is_json_and_bounded() {
    let line = structured_log_line("error", "restore_failed", &"x".repeat(600));
    let value: serde_json::Value = serde_json::from_str(line.trim()).expect("json");
    assert_eq!(value["event"], "restore_failed");
    assert_eq!(value["detail"].as_str().expect("detail").len(), 512);
}

#[test]
fn snapshot_reports_configuration_persistence_state() {
    let socket =
        std::env::temp_dir().join(format!("mackes-config-state-{}.sock", std::process::id()));
    let config =
        std::env::temp_dir().join(format!("mackes-config-state-{}.json5", std::process::id()));
    let daemon = Daemon::bind(&socket).expect("daemon");
    let unconfigured: serde_json::Value =
        serde_json::from_str(&daemon.snapshot_response()).expect("snapshot");
    assert_eq!(unconfigured["config_persistence"]["state"], "unconfigured");
    assert_eq!(unconfigured["native_rescan_interval_ms"], NATIVE_RESCAN_INTERVAL_MS);
    drop(daemon);
    fs::write(&config, b"{schema_version: 1}").expect("config");
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    daemon.set_config_path(&config);
    let ready: serde_json::Value =
        serde_json::from_str(&daemon.snapshot_response()).expect("snapshot");
    assert_eq!(ready["config_persistence"]["state"], "ready");
    let directory =
        std::env::temp_dir().join(format!("mackes-config-state-dir-{}", std::process::id()));
    fs::create_dir(&directory).expect("directory");
    daemon.set_config_path(&directory);
    let invalid: serde_json::Value =
        serde_json::from_str(&daemon.snapshot_response()).expect("snapshot");
    assert_eq!(invalid["config_persistence"]["state"], "unreadable");
    fs::write(&config, b"{invalid").expect("corrupt config");
    daemon.set_config_path(&config);
    let corrupt: serde_json::Value =
        serde_json::from_str(&daemon.snapshot_response()).expect("snapshot");
    assert_eq!(corrupt["config_persistence"]["state"], "corrupt");
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(config);
    let _ = fs::remove_dir(directory);
}

#[test]
fn snapshot_projects_identity_only_endpoint_binding_status() {
    let socket = std::env::temp_dir().join(format!("mackes-bindings-{}.sock", std::process::id()));
    let config = std::env::temp_dir().join(format!("mackes-bindings-{}.json5", std::process::id()));
    fs::write(
        &config,
        "{schema_version: 1, endpoints: [{id: 'known', stable_id: 'known-input', direction: 'input'}, {id: 'both', stable_id: 'known-input'}, {id: 'legacy', name: 'Launch Control'}], projects: [], profiles: []}",
    )
    .expect("config");
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    daemon.set_config_path(&config);
    daemon
        .register_input(Box::new(mackes_midi_engine::VirtualEndpoint::new(
            "known-input",
            "different runtime name",
            mackes_midi_engine::EndpointDirection::Input,
        )))
        .expect("input");
    daemon
        .register_output(Box::new(mackes_midi_engine::VirtualEndpoint::new(
            "known-input",
            "different output name",
            mackes_midi_engine::EndpointDirection::Output,
        )))
        .expect("output");
    let snapshot: serde_json::Value =
        serde_json::from_str(&daemon.snapshot_response()).expect("snapshot");
    let bindings = snapshot["endpoint_bindings"].as_array().expect("bindings");
    assert_eq!(bindings[0]["state"], "connected");
    assert_eq!(bindings[1]["state"], "ambiguous");
    assert!(bindings[1]["action"].as_str().expect("action").contains("direction"));
    assert_eq!(bindings[2]["state"], "missing");
    assert!(bindings[2]["action"].as_str().expect("action").contains("stable device identity"));
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(config);
}

#[test]
fn binding_generation_rejects_delayed_output_after_rebind() {
    let socket =
        std::env::temp_dir().join(format!("mackes-generation-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    daemon
        .register_output(Box::new(mackes_midi_engine::VirtualEndpoint::new(
            "bound-output",
            "runtime name",
            mackes_midi_engine::EndpointDirection::Output,
        )))
        .expect("output");
    daemon
        .set_profile_bindings(vec![("eventide.micropitch".into(), "bound-output".into())])
        .expect("binding");
    let generation = daemon.snapshot_response();
    let current = serde_json::from_str::<serde_json::Value>(&generation).expect("snapshot")
        ["binding_generation"]
        .as_u64()
        .expect("generation");
    daemon.set_profile_bindings(Vec::new()).expect("unbind");
    let event = mackes_domain::MidiEvent {
        timestamp: mackes_domain::TimestampNanos::new(0),
        sequence: 0,
        endpoint: mackes_midi_engine::numeric_endpoint_id("bound-output").expect("endpoint"),
        message: mackes_domain::MidiMessage::NoteOn {
            channel: mackes_domain::MidiChannel::new(1).expect("channel"),
            note: mackes_domain::SevenBit::new(1).expect("note"),
            velocity: mackes_domain::SevenBit::new(1).expect("velocity"),
        },
    };
    assert!(daemon
        .send_event_to_endpoint_at_generation(event.endpoint, event.clone(), current)
        .is_err());
    daemon
        .set_profile_bindings(vec![("eventide.micropitch".into(), "bound-output".into())])
        .expect("rebind");
    assert!(daemon
        .send_event_to_endpoint_at_generation(event.endpoint, event, current + 2)
        .is_ok());
    let _ = fs::remove_file(socket);
}

#[test]
fn endpoint_settle_policy_is_bounded_and_defaults_to_five_seconds() {
    assert_eq!(EndpointSettlePolicy::default().window_ms, 5_000);
    assert_eq!(EndpointSettlePolicy::default().deadline_ms(1_000), 6_000);
    assert_eq!(EndpointSettlePolicy::new(0), None);
    assert_eq!(EndpointSettlePolicy::new(250).expect("policy").deadline_ms(u64::MAX), u64::MAX);
    let policy = EndpointSettlePolicy::default();
    assert_eq!(policy.classify(1_000, 1_001, false), SettleState::Settling);
    assert_eq!(policy.classify(1_000, 6_000, false), SettleState::TimedOut);
    assert_eq!(policy.classify(1_000, 6_000, true), SettleState::Ready);
    assert_eq!(policy.classify(1_000, 6_001, false), SettleState::TimedOut);
}

#[test]
fn native_rescan_interval_is_explicitly_bounded() {
    assert_eq!(NATIVE_RESCAN_INTERVAL_MS, 250);
    const { assert!(NATIVE_RESCAN_INTERVAL_MS <= 250) };
}

#[test]
fn experimental_mapping_safety_is_bounded_and_restart_clears_it() {
    let socket = std::env::temp_dir().join(format!("mackes-safety-{}", std::process::id()));
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    daemon.arm_experimental_mappings();
    let now = daemon.safety_clock.elapsed().as_nanos().try_into().unwrap_or(u64::MAX);
    assert!(daemon.safety.unsafe_armed(now));
    daemon.safety.arm_unsafe(0);
    assert!(!daemon.safety.unsafe_armed(1));
    drop(daemon);
    let restarted_socket = socket.with_extension("restart");
    let mut restarted = Daemon::bind(&restarted_socket).expect("restarted daemon");
    assert!(!restarted.safety.unsafe_armed(1));
}

#[test]
fn daemon_owns_generation_checked_assignment_session() {
    let socket = std::env::temp_dir().join(format!("mackes-session-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    let started = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: 0,
        action: mackes_ipc::AssignmentAction::Start,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert!(started.applied);
    assert_eq!(started.session.phase, mackes_ipc::AssignmentPhase::AwaitControl);
    assert_eq!(daemon.assignment_led_state_at(0).color, mackes_profiles::LedColor::Yellow);
    let stale = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: 0,
        action: mackes_ipc::AssignmentAction::Cancel,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert!(!stale.applied);
    assert_eq!(stale.reason.as_deref(), Some("assignment generation conflict"));
    let reserved = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: 1,
        action: mackes_ipc::AssignmentAction::ControlCaptured,
        physical_control_id: Some("utility-1".into()),
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert!(!reserved.applied);
    assert_eq!(reserved.reason.as_deref(), Some("assignment control is reserved or unknown"));
    let unknown = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: 1,
        action: mackes_ipc::AssignmentAction::ControlCaptured,
        physical_control_id: Some("syntactically-valid-but-unknown".into()),
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert!(!unknown.applied);
    assert_eq!(unknown.reason.as_deref(), Some("assignment control is reserved or unknown"));
    let _ = fs::remove_file(socket);
}

#[test]
fn eventide_assignment_skips_preset_and_type_levels() {
    let socket =
        std::env::temp_dir().join(format!("mackes-eventide-flow-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    let mut generation = daemon
        .apply_assignment_request(mackes_ipc::AssignmentRequest {
            generation: 0,
            action: mackes_ipc::AssignmentAction::Start,
            physical_control_id: None,
            destination_profile: None,
            destination_effect: None,
            destination_parameter: None,
        })
        .generation;
    let captured = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation,
        action: mackes_ipc::AssignmentAction::ControlCaptured,
        physical_control_id: Some("button-r1-c2".into()),
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert!(captured.applied);
    generation = captured.generation;
    let selected_device = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation,
        action: mackes_ipc::AssignmentAction::Down,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    generation = selected_device.generation;
    let effect = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert_eq!(effect.session.phase, mackes_ipc::AssignmentPhase::ChooseEffect);
    generation = effect.generation;
    let parameter = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert_eq!(parameter.session.phase, mackes_ipc::AssignmentPhase::ChooseParameter);
    let _ = fs::remove_file(socket);
}

#[test]
fn daemon_catalog_snapshot_reconstructs_learn_and_commits_once() {
    let socket = std::env::temp_dir().join(format!("mackes-catalog-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    let start = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: 0,
        action: mackes_ipc::AssignmentAction::Start,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let capture = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: start.generation,
        action: mackes_ipc::AssignmentAction::ControlCaptured,
        physical_control_id: Some("button-r1-c1".into()),
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert_eq!(capture.session.catalog.captured_control_id.as_deref(), Some("button-r1-c1"));
    assert_eq!(capture.session.catalog.source_channel, Some(8));
    assert!(capture.session.catalog.devices.iter().any(|row| row.id == "lexicon.reflex"));
    let encoded = serde_json::to_vec(&capture.session).expect("snapshot");
    let restored: mackes_ipc::AssignmentSession =
        serde_json::from_slice(&encoded).expect("restore");
    assert_eq!(restored.catalog.captured_control_id, capture.session.catalog.captured_control_id);
    let mut generation = capture.generation;
    let device = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert!(device.applied, "{device:?}");
    generation = device.generation;
    let down = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation,
        action: mackes_ipc::AssignmentAction::Down,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert!(down.applied, "{down:?}");
    assert_eq!(down.session.phase, mackes_ipc::AssignmentPhase::ChoosePreset);
    assert_eq!(down.session.cursors.preset, 1);
    generation = down.generation;
    let committed = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert!(committed.applied, "{committed:?}");
    assert_eq!(committed.session.phase, mackes_ipc::AssignmentPhase::Succeeded);
    assert_eq!(daemon.mapping_store.active.len(), 1);
    assert!(daemon.mapping_store.active[0].destination_parameter.starts_with("pcm70_reflex:"));
    assert_eq!(daemon.mapping_store.active[0].source_channel, 8);
    assert_ne!(daemon.mapping_store.active[0].source_endpoint, "controller");
    let duplicate = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: committed.generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert!(!duplicate.applied);
    let _ = fs::remove_file(socket);
}

#[test]
fn daemon_assignment_result_uses_scheduler_overlay_then_restores_base() {
    let socket =
        std::env::temp_dir().join(format!("mackes-led-session-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    let start = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: 0,
        action: mackes_ipc::AssignmentAction::Start,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert!(start.applied);
    let capture = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: 1,
        action: mackes_ipc::AssignmentAction::ControlCaptured,
        physical_control_id: Some("knob-r1-c1".into()),
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert!(capture.applied, "{capture:?}");
    assert!(capture.applied);
    assert_eq!(daemon.assignment_led_state_at(0).color, mackes_profiles::LedColor::Yellow);
    let mut generation = capture.generation;
    for action in [
        mackes_ipc::AssignmentAction::Enter,
        mackes_ipc::AssignmentAction::Enter,
        mackes_ipc::AssignmentAction::Enter,
        mackes_ipc::AssignmentAction::Enter,
        mackes_ipc::AssignmentAction::Commit,
    ] {
        let result = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
            generation,
            action,
            physical_control_id: (action == mackes_ipc::AssignmentAction::Commit)
                .then_some("knob-r1-c1".into()),
            destination_profile: (action == mackes_ipc::AssignmentAction::Commit)
                .then_some("lexicon.reflex".into()),
            destination_effect: (action == mackes_ipc::AssignmentAction::Commit)
                .then_some("algorithm-1".into()),
            destination_parameter: (action == mackes_ipc::AssignmentAction::Commit)
                .then_some("reflex.parameter-1".into()),
        });
        assert!(result.applied, "assignment action should apply: {action:?}");
        generation = result.generation;
    }
    assert_eq!(daemon.assignment_led_state_at(0).color, mackes_profiles::LedColor::Green);
    assert_eq!(daemon.mapping_store.active.len(), 1);
    assert_eq!(daemon.mapping_store.active[0].destination_parameter, "reflex.parameter-1");
    assert_eq!(daemon.assignment_led_state_at(1_600).color, mackes_profiles::LedColor::Amber);
    assert_eq!(
        daemon.assignment_led_frame_at(8, 0, 1_600).expect("base frame"),
        vec![0xf0, 0x00, 0x20, 0x29, 0x02, 0x11, 0x78, 0x08, 0x00, 0x33, 0xf7]
    );
    assert!(daemon.assignment_led_frame_at(16, 24, 0).is_none());
    let _ = fs::remove_file(socket);
}

#[test]
fn daemon_rejects_incomplete_assignment_commit_without_state_change() {
    let socket =
        std::env::temp_dir().join(format!("mackes-incomplete-commit-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    let start = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: 0,
        action: mackes_ipc::AssignmentAction::Start,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let result = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: start.generation,
        action: mackes_ipc::AssignmentAction::Commit,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert!(!result.applied);
    assert_eq!(result.reason.as_deref(), Some("assignment commit requires a complete destination"));
    assert_eq!(result.session.phase, mackes_ipc::AssignmentPhase::AwaitControl);
    assert_eq!(result.session.catalog.last_error.as_deref(), result.reason.as_deref());
    let _ = fs::remove_file(socket);
}

#[test]
fn occupied_assignment_enters_confirm_replace_and_replaces_atomically() {
    let socket =
        std::env::temp_dir().join(format!("mackes-assignment-replace-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    daemon.mapping_store.active.push(mackes_config::ControlMapping {
        id: "existing-map".into(),
        controller_profile: "launch-control-xl-mk2".into(),
        physical_control_id: "knob-r1-c1".into(),
        source_endpoint: "controller".into(),
        source_kind: "cc".into(),
        source_channel: 8,
        destination_channel: None,
        source_number: 21,
        destination_endpoint: "processor".into(),
        destination_profile: "lexicon.reflex".into(),
        destination_effect: "algorithm-1".into(),
        destination_parameter: "reflex.parameter-1".into(),
        behavior: mackes_config::MappingBehavior {
            source_range: (0, 127),
            destination_range: (0, 127),
            invert: false,
            curve: "linear".into(),
        },
        enabled: true,
        profile_version: 1,
    });
    let start = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: 0,
        action: mackes_ipc::AssignmentAction::Start,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let capture = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: start.generation,
        action: mackes_ipc::AssignmentAction::ControlCaptured,
        physical_control_id: Some("knob-r1-c1".into()),
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let device = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: capture.generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let effect = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: device.generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let conflict = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: effect.generation,
        action: mackes_ipc::AssignmentAction::Commit,
        physical_control_id: Some("knob-r1-c1".into()),
        destination_profile: Some("eventide.micropitch".into()),
        destination_effect: Some("modulation".into()),
        destination_parameter: Some("control-4".into()),
    });
    assert!(conflict.applied, "{conflict:?}");
    assert_eq!(conflict.session.phase, mackes_ipc::AssignmentPhase::ConfirmReplace);
    assert_eq!(daemon.mapping_store.active.len(), 1);
    let confirmed = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: conflict.generation,
        action: mackes_ipc::AssignmentAction::ConfirmReplace,
        physical_control_id: Some("knob-r1-c1".into()),
        destination_profile: Some("eventide.micropitch".into()),
        destination_effect: Some("modulation".into()),
        destination_parameter: Some("control-4".into()),
    });
    assert!(confirmed.applied, "{confirmed:?}");
    assert_eq!(confirmed.session.phase, mackes_ipc::AssignmentPhase::Succeeded);
    assert_eq!(daemon.mapping_store.active.len(), 1);
    assert_eq!(daemon.mapping_store.active[0].id, "existing-map");
    assert_eq!(daemon.mapping_store.active[0].destination_parameter, "control-4");
    let _ = fs::remove_file(socket);
}

#[test]
fn complete_assignment_commit_persists_to_config() {
    let socket =
        std::env::temp_dir().join(format!("mackes-assignment-persist-{}.sock", std::process::id()));
    let config = std::env::temp_dir()
        .join(format!("mackes-assignment-persist-{}.json5", std::process::id()));
    fs::copy("../../fixtures/config-valid.json5", &config).expect("fixture copy");
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    daemon.set_config_path(&config);
    let start = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: 0,
        action: mackes_ipc::AssignmentAction::Start,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let capture = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: start.generation,
        action: mackes_ipc::AssignmentAction::ControlCaptured,
        physical_control_id: Some("knob-r1-c1".into()),
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let device = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: capture.generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let chooser = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: device.generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let type_level = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: chooser.generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let parameter_level = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: type_level.generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let result = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: parameter_level.generation,
        action: mackes_ipc::AssignmentAction::Commit,
        physical_control_id: Some("knob-r1-c1".into()),
        destination_profile: Some("lexicon.reflex".into()),
        destination_effect: Some("algorithm-1".into()),
        destination_parameter: Some("reflex.parameter-1".into()),
    });
    assert!(result.applied, "{result:?}");
    let loaded = mackes_config::load(&config).expect("reloaded config");
    assert_eq!(loaded.control_mappings.len(), 1);
    assert_eq!(loaded.control_mappings[0].physical_control_id, "knob-r1-c1");
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(config);
}

#[test]
fn failed_assignment_restores_previous_mapping_store() {
    let socket = std::env::temp_dir()
        .join(format!("mackes-assignment-rollback-{}.sock", std::process::id()));
    let config = std::env::temp_dir()
        .join(format!("mackes-assignment-rollback-{}.json5", std::process::id()));
    fs::copy("../../fixtures/config-valid.json5", &config).expect("fixture copy");
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    daemon.set_config_path(&config);
    let original = daemon.mapping_store.clone();
    let start = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: 0,
        action: mackes_ipc::AssignmentAction::Start,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let capture = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: start.generation,
        action: mackes_ipc::AssignmentAction::ControlCaptured,
        physical_control_id: Some("knob-r1-c1".into()),
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let device = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: capture.generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let chooser = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: device.generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let type_level = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: chooser.generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let parameter_level = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: type_level.generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let commit = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: parameter_level.generation,
        action: mackes_ipc::AssignmentAction::Commit,
        physical_control_id: Some("knob-r1-c1".into()),
        destination_profile: Some("lexicon.reflex".into()),
        destination_effect: Some("algorithm-1".into()),
        destination_parameter: Some("reflex.parameter-1".into()),
    });
    assert!(commit.applied, "{commit:?}");
    assert_eq!(commit.session.phase, mackes_ipc::AssignmentPhase::Succeeded);
    assert_ne!(daemon.mapping_store, original);
    let failed = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: commit.generation,
        action: mackes_ipc::AssignmentAction::Fail,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert!(!failed.applied);
    assert_eq!(failed.session.phase, mackes_ipc::AssignmentPhase::Succeeded);
    let reloaded = mackes_config::load(&config).expect("reloaded config");
    assert_eq!(reloaded.control_mappings.len(), 1);
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(config);
}

#[test]
fn interrupted_assignment_draft_survives_daemon_rebind() {
    let socket =
        std::env::temp_dir().join(format!("mackes-assignment-resume-{}.sock", std::process::id()));
    let config =
        std::env::temp_dir().join(format!("mackes-assignment-resume-{}.json5", std::process::id()));
    fs::copy("../../fixtures/config-valid.json5", &config).expect("fixture copy");
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    daemon.set_config_path(&config);
    let start = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: 0,
        action: mackes_ipc::AssignmentAction::Start,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let capture = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: start.generation,
        action: mackes_ipc::AssignmentAction::ControlCaptured,
        physical_control_id: Some("knob-r1-c1".into()),
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let interrupted = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: capture.generation,
        action: mackes_ipc::AssignmentAction::Interrupt,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert!(interrupted.applied, "{interrupted:?}");
    assert_eq!(interrupted.session.phase, mackes_ipc::AssignmentPhase::Interrupted);
    drop(daemon);
    let mut rebound = Daemon::bind(&socket).expect("rebound daemon");
    rebound.set_config_path(&config);
    assert!(rebound.assignment_session.has_draft);
    assert_eq!(rebound.mapping_store.drafts.len(), 1);
    assert_eq!(rebound.mapping_store.drafts[0].physical_control_id.as_deref(), Some("knob-r1-c1"));
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(config);
}

#[test]
fn mapping_activity_reports_disconnected_destination_without_fallback() {
    let socket = std::env::temp_dir().join(format!("mackes-activity-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    daemon.mapping_store.active.push(mackes_config::ControlMapping {
        id: "activity-map".into(),
        controller_profile: "controller".into(),
        physical_control_id: "knob-r1-c1".into(),
        source_endpoint: "1".into(),
        source_kind: "cc".into(),
        source_channel: 0,
        destination_channel: None,
        source_number: 21,
        destination_endpoint: "2".into(),
        destination_profile: "eventide.micropitch".into(),
        destination_effect: "modulation".into(),
        destination_parameter: "control-4".into(),
        behavior: mackes_config::MappingBehavior {
            source_range: (0, 127),
            destination_range: (0, 127),
            invert: false,
            curve: "linear".into(),
        },
        enabled: true,
        profile_version: 1,
    });
    let event = mackes_domain::MidiEvent {
        timestamp: mackes_domain::TimestampNanos::new(1),
        sequence: 1,
        endpoint: mackes_midi_engine::numeric_endpoint_id("1").expect("source"),
        message: mackes_domain::MidiMessage::ControlChange {
            channel: mackes_domain::MidiChannel::new(1).expect("channel"),
            controller: mackes_domain::SevenBit::new(21).expect("controller"),
            value: mackes_domain::SevenBit::new(64).expect("value"),
        },
    };
    assert_eq!(daemon.dispatch_registered(&event), (0, 1));
    assert_eq!(
        daemon.last_mapping_activity.as_ref().expect("activity")["reason"],
        "destination_disconnected"
    );
    let _ = fs::remove_file(socket);
}

#[test]
fn profile_destination_mapping_dispatches_to_the_unique_eventide_output() {
    let socket =
        std::env::temp_dir().join(format!("mackes-eventide-map-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    daemon
        .register_output(Box::new(mackes_midi_engine::VirtualEndpoint::new(
            "eventide-out",
            "MicroPitch Pedal",
            mackes_midi_engine::EndpointDirection::Output,
        )))
        .expect("register eventide output");
    daemon
        .set_profile_bindings(vec![("eventide.micropitch".into(), "eventide-out".into())])
        .expect("binding");
    daemon.mapping_store.active.push(mackes_config::ControlMapping {
        id: "eventide-map".into(),
        controller_profile: "launch-control-xl-mk2".into(),
        physical_control_id: "knob-r3-c8".into(),
        source_endpoint: "launch-input".into(),
        source_kind: "cc".into(),
        source_channel: 0,
        destination_channel: None,
        source_number: 56,
        destination_endpoint: "eventide.micropitch".into(),
        destination_profile: "eventide.micropitch".into(),
        destination_effect: "modulation".into(),
        destination_parameter: "control-1".into(),
        behavior: mackes_config::MappingBehavior {
            source_range: (0, 127),
            destination_range: (0, 127),
            invert: false,
            curve: "linear".into(),
        },
        enabled: true,
        profile_version: 1,
    });
    let event = mackes_domain::MidiEvent {
        timestamp: mackes_domain::TimestampNanos::new(1),
        sequence: 1,
        endpoint: mackes_midi_engine::numeric_endpoint_id("launch-input").expect("source"),
        message: mackes_domain::MidiMessage::ControlChange {
            channel: mackes_domain::MidiChannel::new(1).expect("channel"),
            controller: mackes_domain::SevenBit::new(56).expect("controller"),
            value: mackes_domain::SevenBit::new(64).expect("value"),
        },
    };
    assert_eq!(daemon.dispatch_registered(&event), (1, 0));
    assert_eq!(daemon.last_mapping_activity.as_ref().expect("activity")["outcome"], "sent");
    let _ = fs::remove_file(socket);
}

#[test]
fn eventide_bypass_button_toggles_once_per_press() {
    let socket =
        std::env::temp_dir().join(format!("mackes-eventide-toggle-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    daemon
        .register_input(Box::new(mackes_midi_engine::VirtualEndpoint::new(
            "launch-input",
            "Launch Control XL MIDI",
            mackes_midi_engine::EndpointDirection::Input,
        )))
        .expect("register launch input");
    daemon
        .register_output(Box::new(mackes_midi_engine::VirtualEndpoint::new(
            "eventide-out",
            "MicroPitch Pedal",
            mackes_midi_engine::EndpointDirection::Output,
        )))
        .expect("register eventide output");
    daemon
        .set_profile_bindings(vec![("eventide.micropitch".into(), "eventide-out".into())])
        .expect("binding");
    daemon.mapping_store.active.push(mackes_config::ControlMapping {
        id: "eventide-bypass".into(),
        controller_profile: "launch-control-xl-mk2".into(),
        physical_control_id: "button-r1-c1".into(),
        source_endpoint: "launch-input".into(),
        source_kind: "note".into(),
        source_channel: 8,
        destination_channel: Some(6),
        source_number: 41,
        destination_endpoint: "eventide.micropitch".into(),
        destination_profile: "eventide.micropitch".into(),
        destination_effect: "global".into(),
        destination_parameter: "control-2".into(),
        behavior: mackes_config::MappingBehavior {
            source_range: (0, 127),
            destination_range: (0, 127),
            invert: false,
            curve: "linear".into(),
        },
        enabled: true,
        profile_version: 1,
    });
    let event = |velocity| mackes_domain::MidiEvent {
        timestamp: mackes_domain::TimestampNanos::new(1),
        sequence: 1,
        endpoint: mackes_midi_engine::numeric_endpoint_id("launch-input").expect("source"),
        message: mackes_domain::MidiMessage::NoteOn {
            channel: mackes_domain::MidiChannel::new(9).expect("channel"),
            note: mackes_domain::SevenBit::new(41).expect("note"),
            velocity: mackes_domain::SevenBit::new(velocity).expect("velocity"),
        },
    };

    assert_eq!(daemon.dispatch_registered(&event(127)), (1, 0));
    assert_eq!(daemon.last_mapping_activity.as_ref().expect("bypass")["source_value"], 0);
    assert_eq!(
        daemon.last_mapping_activity.as_ref().expect("wire")["wire_bytes"],
        serde_json::json!([182, 14, 0])
    );
    assert_eq!(daemon.dispatch_registered(&event(0)), (0, 0));
    assert_eq!(daemon.dispatch_registered(&event(127)), (1, 0));
    assert_eq!(daemon.last_mapping_activity.as_ref().expect("active")["source_value"], 127);
    let unrelated = mackes_domain::MidiEvent {
        endpoint: mackes_midi_engine::numeric_endpoint_id("unrelated-controller")
            .expect("endpoint"),
        ..event(127)
    };
    assert_eq!(daemon.dispatch_registered(&unrelated), (0, 0));
    let _ = fs::remove_file(socket);
}

#[test]
fn device_cursor_rebinds_exact_destination_output() {
    let socket =
        std::env::temp_dir().join(format!("mackes-profile-binding-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    for (id, name) in [("port-d", "MidiSport 4x4 MIDI 4"), ("eventide-out", "MicroPitch Pedal")] {
        daemon
            .register_output(Box::new(mackes_midi_engine::VirtualEndpoint::new(
                id,
                name,
                mackes_midi_engine::EndpointDirection::Output,
            )))
            .expect("output");
    }
    daemon.physical_devices = serde_json::json!([
        {"name": "MidiSport 4x4"},
        {"name": "MicroPitch Pedal"}
    ]);
    daemon
        .set_profile_bindings(vec![
            ("lexicon.reflex".into(), "port-d".into()),
            ("eventide.micropitch".into(), "eventide-out".into()),
        ])
        .expect("binding");
    let start = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: 0,
        action: mackes_ipc::AssignmentAction::Start,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let capture = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: start.generation,
        action: mackes_ipc::AssignmentAction::ControlCaptured,
        physical_control_id: Some("knob-r1-c1".into()),
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert_eq!(capture.session.catalog.selected_device.as_deref(), Some("lexicon.reflex"));
    assert_eq!(capture.session.catalog.destination_endpoint.as_deref(), Some("port-d"));
    let moved = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: capture.generation,
        action: mackes_ipc::AssignmentAction::Down,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let eventide = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: moved.generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert_eq!(eventide.session.catalog.selected_device.as_deref(), Some("eventide.micropitch"));
    assert_eq!(eventide.session.catalog.destination_endpoint.as_deref(), Some("eventide-out"));
    let _ = fs::remove_file(socket);
}

#[test]
fn factory1_device_press_is_reserved_for_assignment_start() {
    let event = mackes_domain::MidiEvent {
        timestamp: mackes_domain::TimestampNanos::new(1),
        sequence: 1,
        endpoint: mackes_domain::EndpointId::new(1).expect("endpoint"),
        message: mackes_domain::MidiMessage::NoteOn {
            channel: mackes_domain::MidiChannel::new(9).expect("channel"),
            note: mackes_domain::SevenBit::new(105).expect("note"),
            velocity: mackes_domain::SevenBit::new(127).expect("velocity"),
        },
    };
    assert!(Daemon::is_launch_control_factory1_device_press(&event));
}

#[test]
fn factory1_assignable_controls_resolve_to_stable_physical_ids() {
    let event = |message| mackes_domain::MidiEvent {
        timestamp: mackes_domain::TimestampNanos::new(1),
        sequence: 1,
        endpoint: mackes_domain::EndpointId::new(1).expect("endpoint"),
        message,
    };
    let channel = mackes_domain::MidiChannel::new(9).expect("channel");
    assert_eq!(
        Daemon::launch_control_factory1_control_id(&event(
            mackes_domain::MidiMessage::ControlChange {
                channel,
                controller: mackes_domain::SevenBit::new(13).expect("controller"),
                value: mackes_domain::SevenBit::new(1).expect("value"),
            }
        )),
        Some("knob-r1-c1".into())
    );
    assert_eq!(
        Daemon::launch_control_factory1_control_id(&event(
            mackes_domain::MidiMessage::ControlChange {
                channel,
                controller: mackes_domain::SevenBit::new(77).expect("controller"),
                value: mackes_domain::SevenBit::new(127).expect("value"),
            }
        )),
        Some("fader-1".into())
    );
    assert_eq!(
        Daemon::launch_control_factory1_control_id(&event(mackes_domain::MidiMessage::NoteOn {
            channel,
            note: mackes_domain::SevenBit::new(41).expect("note"),
            velocity: mackes_domain::SevenBit::new(127).expect("velocity"),
        })),
        Some("button-r1-c1".into())
    );
}

#[test]
fn required_endpoint_readiness_is_complete_and_fail_closed() {
    let endpoints = vec![
        mackes_midi_engine::EndpointInfo {
            id: "donner".into(),
            name: "Donner".into(),
            direction: mackes_midi_engine::EndpointDirection::Output,
        },
        mackes_midi_engine::EndpointInfo {
            id: "lexicon".into(),
            name: "Lexicon".into(),
            direction: mackes_midi_engine::EndpointDirection::Output,
        },
    ];
    assert!(required_endpoints_ready(&[], &endpoints));
    assert!(!required_endpoints_ready(&["donner", "missing"], &endpoints));
    assert!(required_endpoints_ready(&["donner", "lexicon"], &endpoints));
    assert_eq!(
        settle_required_endpoints(EndpointSettlePolicy::default(), 0, 1, &["missing"], &endpoints),
        SettleState::Settling
    );
    assert_eq!(
        settle_required_endpoints(
            EndpointSettlePolicy::default(),
            0,
            5_000,
            &["missing"],
            &endpoints
        ),
        SettleState::TimedOut
    );
    let fixture = std::path::Path::new("../../fixtures/config-valid.json5");
    let readiness =
        startup_restore_readiness(fixture, EndpointSettlePolicy::default(), 0, 1, &[], &endpoints)
            .or_else(|_| {
                startup_restore_readiness(
                    std::path::Path::new("fixtures/config-valid.json5"),
                    EndpointSettlePolicy::default(),
                    0,
                    1,
                    &[],
                    &endpoints,
                )
            })
            .expect("restore readiness");
    assert_eq!(readiness.restore.activation_scene(), Some("intro"));
    assert_eq!(readiness.endpoints, SettleState::Ready);
    assert!(readiness.may_activate());
    let timed_out = RestoreReadiness { endpoints: SettleState::TimedOut, ..readiness };
    assert!(!timed_out.may_activate());
}

#[cfg(target_os = "linux")]
#[test]
fn shutdown_request_is_idempotent_and_non_operational() {
    let path = std::env::temp_dir().join(format!("mackes-shutdown-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    daemon.request_shutdown();
    daemon.request_shutdown();
    assert_eq!(daemon.health(), Health::Stopping);
    assert!(!daemon.health().is_operational());
    let _ = fs::remove_file(path);
}

#[cfg(target_os = "linux")]
#[test]
fn nonblocking_control_socket_returns_without_client() {
    let path = std::env::temp_dir().join(format!("mackes-nonblocking-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    daemon.set_nonblocking(true).expect("nonblocking listener");
    let error = daemon
        .serve_once(AccessPolicy { control_gid: 0, daemon_uid: 0 })
        .expect_err("no client should produce WouldBlock");
    assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
    let _ = fs::remove_file(path);
}

#[cfg(target_os = "linux")]
#[test]
fn mapping_ipc_draft_persists_through_typed_request() {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    let socket =
        std::env::temp_dir().join(format!("mackes-mapping-ipc-{}.sock", std::process::id()));
    let config =
        std::env::temp_dir().join(format!("mackes-mapping-ipc-{}.json5", std::process::id()));
    fs::copy("../../fixtures/config-valid.json5", &config).expect("fixture copy");
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    daemon.set_config_path(&config);
    let client_socket = socket.clone();
    let worker = std::thread::spawn(move || {
        let mut client = UnixStream::connect(client_socket).expect("connect");
        client.write_all(br#"{"command":"mappings","operation":"Draft","generation":0,"payload":{"kind":"Draft","draft":{"id":"draft-ipc","step":"source","physical_control_id":"knob-r1-c1"}}}
"#).expect("write draft");
        let mut response = String::new();
        client.read_to_string(&mut response).expect("read response");
        response
    });
    daemon.serve_once(AccessPolicy { control_gid: 0, daemon_uid: 0 }).expect("serve draft");
    let response = worker.join().expect("client worker");
    let result: serde_json::Value = serde_json::from_str(response.trim()).expect("result json");
    assert_eq!(result["outcome"], "Applied");
    assert_eq!(result["generation"], 1);
    assert_eq!(mackes_config::load(&config).expect("reload").control_mapping_drafts.len(), 1);
    let client_socket = socket.clone();
    let stale = std::thread::spawn(move || {
        let mut client = UnixStream::connect(client_socket).expect("connect stale");
        client.write_all(br#"{"command":"mappings","operation":"Draft","generation":0,"payload":{"kind":"Draft","draft":{"id":"draft-stale","step":"source"}}}
"#).expect("write stale");
        let mut response = String::new();
        client.read_to_string(&mut response).expect("read stale");
        response
    });
    daemon.serve_once(AccessPolicy { control_gid: 0, daemon_uid: 0 }).expect("serve stale");
    let stale_result: serde_json::Value =
        serde_json::from_str(stale.join().expect("stale worker").trim()).expect("stale json");
    assert_eq!(stale_result["outcome"], "GenerationConflict");
    assert_eq!(mackes_config::load(&config).expect("reload stale").control_mapping_drafts.len(), 1);
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(config);
}

#[cfg(target_os = "linux")]
#[test]
fn assignment_ipc_accepts_documented_nested_payload() {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    let socket =
        std::env::temp_dir().join(format!("mackes-assignment-ipc-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    let started = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: 0,
        action: mackes_ipc::AssignmentAction::Start,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let client_socket = socket.clone();
    let worker = std::thread::spawn(move || {
        let mut client = UnixStream::connect(client_socket).expect("connect");
        client
            .write_all(
                format!(
                    "{{\"protocol_major\":1,\"protocol_minor\":0,\"request_id\":7,\"command\":\"assignment\",\"payload\":{{\"generation\":{},\"action\":\"Cancel\",\"physical_control_id\":null,\"destination_profile\":null,\"destination_effect\":null,\"destination_parameter\":null}}}}\n",
                    started.generation
                )
                .as_bytes(),
            )
            .expect("write cancel");
        let mut response = String::new();
        client.read_to_string(&mut response).expect("read response");
        response
    });
    daemon.serve_once(AccessPolicy { control_gid: 0, daemon_uid: 0 }).expect("serve cancel");
    let result: mackes_ipc::AssignmentResult =
        serde_json::from_str(worker.join().expect("worker").trim()).expect("assignment result");
    assert!(result.applied, "{result:?}");
    assert_eq!(result.session.phase, mackes_ipc::AssignmentPhase::Idle);
    let _ = fs::remove_file(socket);
}

#[cfg(target_os = "linux")]
#[test]
fn active_scene_persistence_round_trips_through_config() {
    let source = std::path::Path::new("../../fixtures/config-valid.json5");
    let path =
        std::env::temp_dir().join(format!("mackes-scene-persist-{}.json5", std::process::id()));
    fs::copy(source, &path).expect("fixture copy");
    persist_active_scene(&path, Some("intro")).expect("persist scene");
    let document = mackes_config::load(&path).expect("reload config");
    assert_eq!(document.settings.active_scene.as_deref(), Some("intro"));
    let _ = fs::remove_file(path);
}

#[cfg(target_os = "linux")]
#[test]
fn invalid_restore_can_leave_daemon_degraded_without_stopping() {
    let path = std::env::temp_dir().join(format!("mackes-degraded-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    daemon.mark_degraded();
    assert_eq!(daemon.health(), Health::Degraded);
    assert!(daemon.health().is_operational());
    daemon.request_shutdown();
    daemon.mark_degraded();
    assert_eq!(daemon.health(), Health::Stopping);
    let _ = fs::remove_file(path);
}

#[cfg(target_os = "linux")]
#[test]
fn physical_device_refresh_retains_disconnected_identity() {
    let path =
        std::env::temp_dir().join(format!("mackes-device-retain-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    let endpoints = vec![
        mackes_midi_engine::EndpointInfo {
            id: "input-1".into(),
            name: "Launch Control XL Mk2".into(),
            direction: mackes_midi_engine::EndpointDirection::Input,
        },
        mackes_midi_engine::EndpointInfo {
            id: "output-1".into(),
            name: "Launch Control XL Mk2".into(),
            direction: mackes_midi_engine::EndpointDirection::Output,
        },
    ];
    daemon.set_physical_devices(&endpoints);
    daemon.set_physical_devices(&[]);
    let snapshot: serde_json::Value =
        serde_json::from_str(&daemon.snapshot_response()).expect("snapshot");
    assert_eq!(snapshot["physical_devices"][0]["id"], "launch control xl mk2");
    assert_eq!(snapshot["physical_devices"][0]["state"], "offline");
    assert_eq!(snapshot["physical_devices"][0]["inputs"][0], "input-1");

    let saturated = (0..40)
        .map(|index| mackes_midi_engine::EndpointInfo {
            id: format!("input-{index}"),
            name: format!("Synthetic Device {index}"),
            direction: mackes_midi_engine::EndpointDirection::Input,
        })
        .collect::<Vec<_>>();
    daemon.set_physical_devices(&saturated);
    let saturated_snapshot: serde_json::Value =
        serde_json::from_str(&daemon.snapshot_response()).expect("snapshot");
    let devices = saturated_snapshot["physical_devices"].as_array().expect("devices");
    assert_eq!(devices.len(), MAX_PHYSICAL_DEVICE_RECORDS);
    assert!(devices.iter().all(|device| device["state"] == "connected"));
    let _ = fs::remove_file(path);
}

#[cfg(target_os = "linux")]
#[test]
fn daemon_state_journal_supports_snapshot_replay_and_gap_detection() {
    let path = std::env::temp_dir().join(format!("mackes-events-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    daemon.generation = 1;
    daemon.record_state_event(Command::Routes);
    let snapshot: serde_json::Value =
        serde_json::from_str(&daemon.snapshot_response()).expect("snapshot");
    assert_eq!(snapshot["last_sequence"], 1);
    assert_eq!(snapshot["received"], 0);
    assert_eq!(snapshot["sent"], 0);
    assert_eq!(snapshot["dropped"], 0);
    assert_eq!(snapshot["audit_count"], 0);
    assert!(snapshot["audit"].as_array().is_some_and(Vec::is_empty));
    let replay: serde_json::Value =
        serde_json::from_str(&daemon.subscribe_response(br#"{"after_sequence":0}"#))
            .expect("replay");
    assert_eq!(replay["events"].as_array().expect("events").len(), 1);
    let enveloped_replay: serde_json::Value =
        serde_json::from_str(&daemon.subscribe_response(br#"{"payload":{"after_sequence":1}}"#))
            .expect("enveloped replay");
    assert!(enveloped_replay["events"].as_array().expect("events").is_empty());
    for _ in 0..256 {
        daemon.record_state_event(Command::Monitor);
    }
    let gap: serde_json::Value =
        serde_json::from_str(&daemon.subscribe_response(br#"{"after_sequence":0}"#)).expect("gap");
    assert_eq!(gap["snapshot_required"], true);
    assert_eq!(daemon.state_events.len(), 256);
    let _ = fs::remove_file(path);
}

#[cfg(target_os = "linux")]
#[test]
fn registered_dispatch_updates_activity_and_publishes_live_event() {
    let path = std::env::temp_dir().join(format!("mackes-activity-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    let event = mackes_domain::MidiEvent {
        timestamp: mackes_domain::TimestampNanos::new(1),
        sequence: 1,
        endpoint: mackes_domain::EndpointId::new(1).expect("endpoint"),
        message: mackes_domain::MidiMessage::ControlChange {
            channel: mackes_domain::MidiChannel::new(9).expect("channel"),
            controller: mackes_domain::SevenBit::new(1).expect("controller"),
            value: mackes_domain::SevenBit::new(2).expect("value"),
        },
    };
    assert_eq!(daemon.dispatch_registered(&event), (0, 0));
    assert_eq!(daemon.activity_counters(), (1, 0, 0));
    assert_eq!(daemon.state_sequence, 1);
    let event_payload = serde_json::from_slice::<serde_json::Value>(
        &daemon.state_events.back().expect("event").payload,
    )
    .expect("payload");
    assert_eq!(event_payload["received"], 1);
    assert_eq!(event_payload["last_activity"]["kind"], "control_change");
    assert_eq!(event_payload["last_activity"]["control_id"], "endpoint:1:control_change:1");
    assert_eq!(event_payload["last_activity"]["timestamp_nanos"], 1);
    assert_eq!(event_payload["last_activity"]["number"], 1);
    assert_eq!(event_payload["last_activity"]["value"], 2);
    assert_eq!(event_payload["last_activity"]["sequence"], 1);
    assert_eq!(event_payload["config_persistence"]["state"], "unconfigured");
    let mut burst_event = event.clone();
    burst_event.sequence = 2;
    assert_eq!(daemon.dispatch_registered(&burst_event), (0, 0));
    assert_eq!(daemon.state_events.len(), 1, "activity journal must be rate-limited");
    let _ = fs::remove_file(path);
}

#[cfg(target_os = "linux")]
#[test]
fn active_scene_is_published_to_snapshot_and_journal() {
    let path = std::env::temp_dir().join(format!("mackes-scene-state-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    daemon.set_active_scene(Some("intro".to_owned()));
    let snapshot =
        serde_json::from_str::<serde_json::Value>(&daemon.snapshot_response()).expect("snapshot");
    assert_eq!(snapshot["active_scene"], "intro");
    let event = serde_json::from_slice::<serde_json::Value>(
        &daemon.state_events.back().expect("event").payload,
    )
    .expect("event payload");
    assert_eq!(event["active_scene"], "intro");
    let _ = fs::remove_file(path);
}

#[cfg(target_os = "linux")]
#[test]
fn dashboard_command_polling_is_bounded_and_requires_registered_input() {
    let path =
        std::env::temp_dir().join(format!("mackes-dashboard-input-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    let binding = mackes_config::DashboardMidiBinding {
        trigger: mackes_config::DashboardMidiTrigger::NoteOn { channel: 1, note: 36 },
        command: "panic".into(),
    };
    assert!(daemon.poll_dashboard_commands(&[binding], 128).is_empty());
    assert!(daemon.poll_dashboard_commands(&[], 0).is_empty());
    let response = daemon.handle_dashboard_command(Command::Panic);
    assert!(response.contains("\"panic\":true"));
    assert_eq!(daemon.state_sequence, 1);
    assert!(daemon.handle_dashboard_command(Command::Shutdown).contains("not allowed"));
    let _ = fs::remove_file(path);
}

#[cfg(target_os = "linux")]
#[test]
fn dashboard_poll_defers_unmatched_device_press_for_assignment_dispatch() {
    use mackes_midi_engine::MidiOutputAdapter;

    let path =
        std::env::temp_dir().join(format!("mackes-dashboard-device-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    let mut input = mackes_midi_engine::VirtualEndpoint::new(
        "launch-control",
        "Launch Control XL",
        mackes_midi_engine::EndpointDirection::Input,
    );
    input.send(mackes_domain::MidiEvent {
        timestamp: mackes_domain::TimestampNanos::new(1),
        sequence: 1,
        endpoint: mackes_domain::EndpointId::new(1).expect("endpoint"),
        message: mackes_domain::MidiMessage::NoteOn {
            channel: mackes_domain::MidiChannel::new(9).expect("channel"),
            note: mackes_domain::SevenBit::new(105).expect("note"),
            velocity: mackes_domain::SevenBit::new(127).expect("velocity"),
        },
    });
    daemon.register_input(Box::new(input)).expect("register input");

    assert!(daemon.process_dashboard_commands(&[], 128).is_empty());
    assert_eq!(daemon.assignment_session.phase, mackes_ipc::AssignmentPhase::Idle);
    assert_eq!(daemon.poll_and_dispatch_inputs(128).0, 1);
    assert_eq!(daemon.assignment_session.phase, mackes_ipc::AssignmentPhase::AwaitControl);
    let _ = fs::remove_file(path);
}

#[cfg(target_os = "linux")]
#[test]
fn learn_capture_is_observational_bounded_and_endpoint_scoped() {
    let path = std::env::temp_dir().join(format!("mackes-learn-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    let endpoint = mackes_domain::EndpointId::new(1).expect("endpoint");
    assert!(daemon.capture_learn_candidates(endpoint, 0).is_empty());
    assert!(daemon.capture_learn_candidates(endpoint, 128).is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn daemon_scene_boundary_enforces_deadline_before_device_executor() {
    let path = std::env::temp_dir().join(format!("mackes-scene-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    let plan =
        mackes_scene_engine::ActivationPlan::compile(vec![mackes_scene_engine::ActivationAction {
            id: "write".into(),
            description: "write".into(),
            unsafe_action: false,
            depends_on: None,
            destination: None,
            message: None,
        }])
        .expect("plan");
    let mut calls = 0;
    let result = daemon.execute_scene_with_deadline(&plan, false, false, 5, 5, |_| {
        calls += 1;
        mackes_scene_engine::ActionResult::Succeeded
    });
    assert_eq!(calls, 0);
    assert_eq!(result[0].1, mackes_scene_engine::ActionResult::TimedOut);
}

#[cfg(target_os = "linux")]
#[test]
fn startup_restore_uses_ordinary_planner_and_holds_unsafe_actions() {
    let path =
        std::env::temp_dir().join(format!("mackes-startup-plan-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    let plan = mackes_scene_engine::ActivationPlan::compile(vec![
        mackes_scene_engine::ActivationAction {
            id: "safe".into(),
            description: "safe".into(),
            unsafe_action: false,
            depends_on: None,
            destination: None,
            message: None,
        },
        mackes_scene_engine::ActivationAction {
            id: "unsafe".into(),
            description: "unsafe".into(),
            unsafe_action: true,
            depends_on: Some("safe".into()),
            destination: None,
            message: None,
        },
    ])
    .expect("plan");
    let mut calls = 0;
    let results = daemon.execute_startup_restore(&plan, |_| {
        calls += 1;
        mackes_scene_engine::ActionResult::Succeeded
    });
    assert_eq!(calls, 1);
    assert_eq!(results[1].1, mackes_scene_engine::ActionResult::SkippedDependency);
    let _ = fs::remove_file(path);
}

#[cfg(target_os = "linux")]
#[test]
fn command_classifier_rejects_unknown_tags() {
    assert_eq!(classify_command(br#"{"command":"health"}"#), Some(Command::Health));
    assert_eq!(classify_command(br#"{"command":"snapshot"}"#), Some(Command::Snapshot));
    assert_eq!(classify_command(br#"{"command":"panic"}"#), Some(Command::Panic));
    assert_eq!(classify_command(b"not-json"), None);
    for (tag, expected) in [
        ("hello", Command::Hello),
        ("subscribe", Command::Subscribe),
        ("validate", Command::Validate),
        ("configuration", Command::Configuration),
        ("endpoints", Command::Endpoints),
        ("routes", Command::Routes),
        ("learn", Command::Learn),
        ("scenes", Command::Scenes),
        ("device_query", Command::DeviceQuery),
        ("device_control", Command::DeviceControl),
        ("sysex", Command::Sysex),
        ("backups", Command::Backups),
        ("monitor", Command::Monitor),
        ("unsafe_mode", Command::UnsafeMode),
        ("shutdown", Command::Shutdown),
    ] {
        let request = format!(r#"{{"command":"{tag}"}}"#);
        assert_eq!(classify_command(request.as_bytes()), Some(expected));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn command_acknowledgments_are_stable_and_operation_specific() {
    assert_eq!(
        command_ack(Command::Health, Health::Ready, 3, &[], None, &[]),
        "{\"ok\":true,\"generation\":3,\"health\":\"ready\"}\n"
    );
    assert_eq!(
        command_ack(Command::Panic, Health::Ready, 4, &[], None, &[]),
        "{\"ok\":true,\"generation\":4,\"panic\":true}\n"
    );
    assert_eq!(
        command_ack(Command::Hello, Health::Ready, 5, &[], None, &[]),
        "{\"ok\":true,\"generation\":5,\"protocol\":1}\n"
    );
    assert_eq!(
        command_ack(Command::Routes, Health::Ready, 6, &[], Some(0), &[]),
        "{\"ok\":true,\"generation\":6,\"routes\":[],\"route_generation\":0}\n"
    );
    assert_eq!(
        command_ack(Command::Learn, Health::Ready, 7, &[], None, &[]),
        "{\"ok\":true,\"generation\":7,\"learn\":true}\n"
    );
    assert_eq!(
        command_ack(Command::Scenes, Health::Ready, 7, &[], None, &[]),
        "{\"ok\":true,\"generation\":7,\"scenes\":[]}\n"
    );
    assert_eq!(
        command_ack(Command::DeviceQuery, Health::Ready, 8, &[], None, &[]),
        "{\"ok\":true,\"generation\":8,\"devices\":[],\"physical_devices\":[]}\n"
    );
    assert_eq!(
        command_ack(Command::Monitor, Health::Ready, 9, &[], None, &[]),
        "{\"ok\":true,\"generation\":9,\"monitor\":[]}\n"
    );
    assert_eq!(
        command_ack(Command::UnsafeMode, Health::Ready, 10, &[], None, &[]),
        "{\"ok\":true,\"generation\":10,\"unsafe_mode\":\"disarmed\"}\n"
    );
    assert_eq!(
        command_ack(Command::Sysex, Health::Ready, 10, &[], None, &[]),
        "{\"ok\":true,\"generation\":10,\"sysex\":true,\"unsafe_required\":true}\n"
    );
    assert_eq!(
        command_ack(Command::Health, Health::Degraded, 11, &[], None, &[]),
        "{\"ok\":true,\"generation\":11,\"health\":\"degraded\"}\n"
    );
    assert_eq!(
        command_ack(Command::Rescan, Health::Ready, 12, &[], None, &[]),
        "{\"ok\":true,\"generation\":12,\"rescan\":\"scheduled\"}\n"
    );
}

#[test]
fn scenes_query_projects_daemon_scene_catalog() {
    let path = std::env::temp_dir().join(format!("mackes-scenes-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    daemon.set_scene_ids(vec!["intro".into(), "verse".into()]);
    daemon.set_active_scene(Some("verse".into()));
    assert_eq!(daemon.navigate_scene(true).as_deref(), Some("intro"));
    assert_eq!(daemon.navigate_scene(false).as_deref(), Some("verse"));
    let response: serde_json::Value =
        serde_json::from_str(&daemon.scenes_response()).expect("response");
    assert_eq!(response["scenes"], serde_json::json!(["intro", "verse"]));
    assert_eq!(response["active_scene"], "verse");
}

#[test]
fn persisted_scene_actions_compile_to_ordinary_activation_plan() {
    let scene = mackes_config::SceneRef {
        id: "intro".into(),
        name: Some("Intro".into()),
        category: Some("opening".into()),
        actions: vec![mackes_config::SceneAction {
            id: "set-level".into(),
            description: "Set level".into(),
            unsafe_action: true,
            depends_on: None,
            destination: None,
            message: None,
        }],
    };
    let plan = compile_scene_actions(&scene).expect("compile");
    assert_eq!(plan.actions.len(), 1);
    assert_eq!(plan.actions[0].id, "set-level");
    assert!(plan.actions[0].unsafe_action);
}

#[test]
fn startup_restore_loads_last_scene_without_unsafe_actions() {
    let result = startup_restore(std::path::Path::new("../../fixtures/config-valid.json5"));
    // Cargo runs this test from the workspace root; use the repository-relative fixture.
    let result =
        result.or_else(|_| startup_restore(std::path::Path::new("fixtures/config-valid.json5")));
    let result = result.expect("valid persisted state");
    assert_eq!(result.active_project.as_deref(), Some("demo"));
    assert_eq!(result.active_scene, None);
    assert_eq!(result.scenes, vec!["intro"]);
    assert!(result.should_activate);
    assert_eq!(result.activation_scene(), Some("intro"));
    assert_eq!(result.unsafe_actions_blocked, 1);
}

#[test]
fn startup_restore_rejects_missing_active_project() {
    let path =
        std::env::temp_dir().join(format!("mackes-startup-{}-missing.json5", std::process::id()));
    std::fs::write(
        &path,
        "{ schema_version: 1, settings: { active_project: 'missing' }, endpoints: [], projects: [{ id: 'other', scenes: [] }], profiles: [] }",
    )
    .expect("write state");
    let result = startup_restore(&path);
    let _ = std::fs::remove_file(&path);
    assert!(
        matches!(result, Err(ConfigError::Semantic { message, .. }) if message.contains("active project"))
    );
}

#[test]
fn startup_restore_does_not_activate_empty_project() {
    let path =
        std::env::temp_dir().join(format!("mackes-startup-{}-empty.json5", std::process::id()));
    std::fs::write(
        &path,
        "{ schema_version: 1, settings: { active_project: 'empty' }, endpoints: [], projects: [{ id: 'empty', scenes: [] }], profiles: [] }",
    )
    .expect("write state");
    let result = startup_restore(&path).expect("valid state");
    let _ = std::fs::remove_file(&path);
    assert!(!result.should_activate);
    assert_eq!(result.activation_scene(), None);
    assert_eq!(result.unsafe_actions_blocked, 0);
}

#[cfg(target_os = "linux")]
#[test]
fn route_json_replacement_is_bounded_and_atomic() {
    let path = std::env::temp_dir().join(format!(
        "mackes-routes-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let daemon = Daemon::bind(&path).expect("daemon");
    daemon
        .replace_routes_json(
            br#"[{"source":1,"destination":2,"destination_parameter":"Mix","channel":1,"class":"ControlChange","enabled":false,"priority":9,"curve":"square","allow_cycle":true,"predicates":[{"NumberRange":{"minimum":10,"maximum":20}}]}]"#,
            4,
            8,
        )
        .expect("routes");
    assert_eq!(daemon.route_generation(), Some(4));
    assert_eq!(daemon.router.routes().len(), 1);
    let route = &daemon.router.routes()[0];
    assert!(!route.enabled);
    assert_eq!(route.priority, 9);
    assert_eq!(route.curve, mackes_midi_engine::Curve::Square);
    assert_eq!(route.destination_parameter.as_deref(), Some("Mix"));
    assert!(route.allow_cycle);
    assert_eq!(route.predicates.len(), 1);
    assert!(daemon.replace_routes_json(br#"[{"source":1,"destination":1}]"#, 5, 8).is_err());
    assert_eq!(daemon.route_generation(), Some(4));
    assert!(daemon
        .replace_routes_json(br#"[{"source":1,"destination":2,"class":"Note"}]"#, 4, 8,)
        .is_err());
    assert_eq!(daemon.route_generation(), Some(4));
    drop(daemon);
    let _ = fs::remove_file(path);
}

#[cfg(target_os = "linux")]
#[test]
fn route_mutation_policy_denies_under_performance_lock_and_audits() {
    let path =
        std::env::temp_dir().join(format!("mackes-route-policy-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    daemon.safety.set_performance_lock(true);

    assert_eq!(
        daemon.authorize_route_mutation("route_replace"),
        Err("route mutation denied by performance lock")
    );
    assert_eq!(daemon.audit.newest_first().count(), 1);
    assert_eq!(
        daemon.audit.newest_first().next().map(|record| record.action_id.as_str()),
        Some("route_replace")
    );
    let snapshot =
        serde_json::from_str::<serde_json::Value>(&daemon.snapshot_response()).expect("snapshot");
    assert_eq!(snapshot["audit_count"], 1);
    assert_eq!(snapshot["audit"][0]["action"], "route_replace");
    let _ = std::fs::remove_file(path);
}

#[test]
fn daemon_device_control_send_is_counted_and_audited() {
    let path =
        std::env::temp_dir().join(format!("mackes-device-audit-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    assert_eq!(daemon.activity_counters().1, 0);
    daemon.record_physical_send("eventide-out", "device-control:eventide.micropitch:Mix");
    assert_eq!(daemon.activity_counters().1, 1);
    let record = daemon.audit.newest_first().next().expect("audit record");
    assert_eq!(record.action_id, "device-control:eventide.micropitch:Mix");
    assert_eq!(record.target_alias, "eventide-out");
    assert!(record.allowed);
    let _ = std::fs::remove_file(path);
}

#[cfg(target_os = "linux")]
#[test]
fn configured_route_store_restores_after_daemon_rebind() {
    let config =
        std::env::temp_dir().join(format!("mackes-route-restore-{}.json5", std::process::id()));
    let persisted = routes_path(&config);
    fs::write(&persisted, br#"{"routes":[{"source":1,"destination":2,"class":"Note"}]}"#)
        .expect("write route store");
    let mut daemon = Daemon::bind(
        std::env::temp_dir().join(format!("mackes-route-sock-{}", std::process::id())),
    )
    .expect("daemon");
    daemon.set_config_path(&config);
    assert_eq!(daemon.router.routes().len(), 1);
    assert_eq!(daemon.route_generation(), Some(1));
    let _ = fs::remove_file(persisted);
}

#[test]
fn configured_route_undo_record_restores_after_daemon_rebind() {
    let socket =
        std::env::temp_dir().join(format!("mackes-route-undo-{}.sock", std::process::id()));
    let config =
        std::env::temp_dir().join(format!("mackes-route-undo-{}.json5", std::process::id()));
    let undo = routes_undo_path(&config);
    fs::write(&config, "{}\n").expect("config");
    fs::write(&undo, br#"[{"source":3,"destination":4,"class":"Note"}]"#).expect("undo record");
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    daemon.set_config_path(&config);
    assert!(daemon.route_undo.is_some());
    assert_eq!(
        daemon.route_undo.as_ref().and_then(serde_json::Value::as_array).map(Vec::len),
        Some(1)
    );
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(config);
    let _ = fs::remove_file(undo);
}

#[test]
fn device_control_requires_confirmation_and_registered_destination() {
    let path =
        std::env::temp_dir().join(format!("mackes-device-control-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    let request = serde_json::json!({
        "profile_id": "eventide.micropitch",
        "control": "Mix",
        "channel": 1,
        "value": 64,
        "destination": "eventide-out",
    });
    assert_eq!(
        daemon.apply_device_control(&request).unwrap_err(),
        "device control requires confirmation"
    );
    assert_eq!(daemon.activity_counters().1, 0);
    let mut confirmed = request;
    confirmed["confirm"] = serde_json::Value::Bool(true);
    assert_eq!(
        daemon.apply_device_control(&confirmed).unwrap_err(),
        "destination output is not registered"
    );
    assert_eq!(daemon.activity_counters().1, 0);
    assert!(daemon.audit.newest_first().next().is_none());
    let _ = fs::remove_file(path);
}

#[test]
fn reflex_system_reset_is_narrow_confirmed_and_daemon_owned() {
    let path =
        std::env::temp_dir().join(format!("mackes-reflex-reset-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&path).expect("daemon");
    daemon
        .register_output(Box::new(mackes_midi_engine::VirtualEndpoint::new(
            "reflex-port-a",
            "MidiSport 4x4 MIDI 1",
            mackes_midi_engine::EndpointDirection::Output,
        )))
        .expect("output");
    let request = serde_json::json!({
        "profile_id": "lexicon.reflex",
        "control": "system-reset",
        "destination": "reflex-port-a",
        "confirm": true,
    });
    assert_eq!(daemon.apply_device_control(&request).expect("reset"), vec![0xFF]);
    assert_eq!(daemon.activity_counters().1, 1);
    let audit = daemon.audit.newest_first().next().expect("audit");
    assert_eq!(audit.target_alias, "reflex-port-a");
    assert_eq!(audit.action_id, "device-control:lexicon.reflex:system-reset");
    let unrelated = serde_json::json!({
        "profile_id": "eventide.micropitch",
        "control": "system-reset",
        "destination": "reflex-port-a",
        "confirm": true,
    });
    assert_eq!(
        daemon.apply_device_control(&unrelated).unwrap_err(),
        "device control fields are required"
    );
    let _ = fs::remove_file(path);
}

#[test]
fn button_preset_mapping_survives_reload_and_knob_preset_is_stripped() {
    let socket =
        std::env::temp_dir().join(format!("mackes-preset-reload-{}.sock", std::process::id()));
    let config =
        std::env::temp_dir().join(format!("mackes-preset-reload-{}.json5", std::process::id()));
    fs::copy("../../fixtures/config-valid.json5", &config)
        .or_else(|_| fs::copy("fixtures/config-valid.json5", &config))
        .expect("fixture copy");
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    daemon.set_config_path(&config);
    let start = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: 0,
        action: mackes_ipc::AssignmentAction::Start,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let capture = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: start.generation,
        action: mackes_ipc::AssignmentAction::ControlCaptured,
        physical_control_id: Some("button-r1-c1".into()),
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let device = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: capture.generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let down = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: device.generation,
        action: mackes_ipc::AssignmentAction::Down,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    let committed = daemon.apply_assignment_request(mackes_ipc::AssignmentRequest {
        generation: down.generation,
        action: mackes_ipc::AssignmentAction::Enter,
        physical_control_id: None,
        destination_profile: None,
        destination_effect: None,
        destination_parameter: None,
    });
    assert!(committed.applied, "{committed:?}");
    assert_eq!(committed.session.phase, mackes_ipc::AssignmentPhase::Succeeded);
    assert_eq!(daemon.mapping_store.active.len(), 1);
    assert!(daemon.mapping_store.active[0].destination_parameter.starts_with("pcm70_reflex:"));
    drop(daemon);
    let mut reloaded = Daemon::bind(&socket).expect("reloaded daemon");
    reloaded.set_config_path(&config);
    assert_eq!(reloaded.mapping_store.active.len(), 1);
    assert_eq!(reloaded.mapping_store.active[0].physical_control_id, "button-r1-c1");
    reloaded.mapping_store.active.push(mackes_config::ControlMapping {
        id: "bad-knob-preset".into(),
        controller_profile: "launch-control-xl-mk2".into(),
        physical_control_id: "knob-r1-c1".into(),
        source_endpoint: "launch-control-xl-mk2".into(),
        source_kind: "cc".into(),
        source_channel: 8,
        destination_channel: None,
        source_number: 13,
        destination_endpoint: "lexicon.reflex".into(),
        destination_profile: "lexicon.reflex".into(),
        destination_effect: "reverb".into(),
        destination_parameter: "pcm70_reflex:concert-wave".into(),
        behavior: mackes_config::MappingBehavior {
            source_range: (0, 127),
            destination_range: (0, 127),
            invert: false,
            curve: "linear".into(),
        },
        enabled: true,
        profile_version: 1,
    });
    mackes_config::save_control_mapping_store(&config, &reloaded.mapping_store, 1)
        .expect("save mixed store");
    drop(reloaded);
    let mut filtered = Daemon::bind(&socket).expect("filtered daemon");
    filtered.set_config_path(&config);
    assert_eq!(filtered.mapping_store.active.len(), 1);
    assert_eq!(filtered.mapping_store.active[0].physical_control_id, "button-r1-c1");
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(config);
}

#[test]
fn led_surface_restores_owner_colors_from_mappings_and_exposes_zero_send() {
    let socket = std::env::temp_dir().join(format!("mackes-led-{}.sock", std::process::id()));
    let mut daemon = Daemon::bind(&socket).expect("daemon");
    daemon.flush_controller_leds_at(0);
    let snapshot: serde_json::Value =
        serde_json::from_str(daemon.snapshot_response().trim()).expect("snapshot");
    assert_eq!(snapshot["led"]["sent"], 0);
    assert_eq!(snapshot["led"]["retries"], 0);
    assert_eq!(snapshot["led"]["template"], 8);
    assert_eq!(
        snapshot["led"]["last_error"].as_str(),
        Some("no unique Launch Control XL MIDI output")
    );
    daemon
        .register_output(Box::new(mackes_midi_engine::VirtualEndpoint::new(
            "xl-midi",
            "Launch Control XL MIDI 1",
            mackes_midi_engine::EndpointDirection::Output,
        )))
        .expect("output");
    daemon.mapping_store.active.push(mackes_config::ControlMapping {
        id: "lex-knob".into(),
        controller_profile: "launch-control-xl-mk2".into(),
        physical_control_id: "knob-r1-c1".into(),
        source_endpoint: "launch-control-xl-mk2".into(),
        source_kind: "cc".into(),
        source_channel: 8,
        destination_channel: None,
        source_number: 13,
        destination_endpoint: "lexicon.reflex".into(),
        destination_profile: "lexicon.reflex".into(),
        destination_effect: "algorithm-1".into(),
        destination_parameter: "reflex.parameter-1".into(),
        behavior: mackes_config::MappingBehavior {
            source_range: (0, 127),
            destination_range: (0, 127),
            invert: false,
            curve: "linear".into(),
        },
        enabled: true,
        profile_version: 1,
    });
    daemon.replay_controller_leds();
    let restored: serde_json::Value =
        serde_json::from_str(daemon.snapshot_response().trim()).expect("restored");
    assert!(restored["led"]["sent"].as_u64().unwrap_or(0) > 0);
    assert_eq!(restored["led"]["template"], 8);
    assert_eq!(restored["led"]["retries"], 0);
    assert_eq!(restored["led"]["target_id"].as_str(), Some("xl-midi"));
    let first_sent = restored["led"]["sent"].as_u64().unwrap();
    let _ = daemon.snapshot_response();
    daemon.flush_controller_leds_at(5);
    let coalesced: serde_json::Value =
        serde_json::from_str(daemon.snapshot_response().trim()).expect("coalesced");
    assert_eq!(coalesced["led"]["sent"].as_u64(), Some(first_sent));
    daemon
        .register_output(Box::new(mackes_midi_engine::VirtualEndpoint::new(
            "xl-midi-2",
            "Launch Control XL MIDI 2",
            mackes_midi_engine::EndpointDirection::Output,
        )))
        .expect("duplicate");
    daemon.replay_controller_leds();
    let duplicate: serde_json::Value =
        serde_json::from_str(daemon.snapshot_response().trim()).expect("duplicate");
    assert_eq!(
        duplicate["led"]["last_error"].as_str(),
        Some("duplicate Launch Control XL MIDI outputs; LED writes refused")
    );
    let _ = fs::remove_file(socket);
}
