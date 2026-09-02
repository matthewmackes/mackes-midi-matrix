//! Characterization tests for public crate behavior.

use super::*;
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn rack_lamps_have_ansi_safe_markers_and_semantic_colors() {
    assert_eq!(RackLamp::Offline.marker(), "o");
    assert_eq!(RackLamp::Enabled.marker(), "*");
    assert_eq!(RackLamp::Error.color(), Color::Red);
    assert_ne!(RackLamp::Offline.marker(), RackLamp::Enabled.marker());
}

#[test]
fn faceplate_control_states_have_distinct_ascii_markers() {
    let states = [
        FaceplateControlState::Offline,
        FaceplateControlState::Unknown,
        FaceplateControlState::Unmapped,
        FaceplateControlState::Mapped,
        FaceplateControlState::Live,
    ];
    let markers: std::collections::HashSet<_> =
        states.into_iter().map(faceplate_state_marker).collect();
    assert_eq!(markers.len(), 5);
    assert_eq!(faceplate_state_marker(FaceplateControlState::Live), "LIVE");
}

#[test]
fn rack_value_bar_is_bounded_and_monotonic() {
    assert_eq!(RackValueBar::new(0, 100).cells(), (0, 64));
    assert_eq!(RackValueBar::new(127, 8).cells(), (8, 0));
    assert!(RackValueBar::new(71, 16).cells().0 < RackValueBar::new(72, 16).cells().0);
}

#[test]
fn rack_renderers_are_bounded_and_color_independent() {
    let lamp = rack_lamp_line("ONLINE", RackLamp::Enabled);
    assert_eq!(lamp.to_string(), "* ONLINE");
    let bar = rack_value_bar_line("Mix", RackValueBar::new(127, 8));
    assert_eq!(bar.to_string(), "Mix          [========] 127");
    assert!(!bar.to_string().contains('\u{1b}'));
}

#[test]
fn rack_shell_layout_collapses_only_below_required_viewport() {
    assert!(!RackShellLayout::for_terminal(100, 37).compact);
    assert!(RackShellLayout::for_terminal(80, 24).compact);
    let layout = RackShellLayout::for_terminal(80, 24);
    assert_eq!((layout.status_rows, layout.alert_rows, layout.footer_rows), (1, 1, 1));
}

#[test]
fn rack_shell_keeps_critical_bands_visible_at_required_sizes() {
    let expanded = rack_shell_lines(
        RackShellLayout::for_terminal(100, 37),
        "ready",
        Some("route saved"),
        true,
    );
    let compact = rack_shell_lines(
        RackShellLayout::for_terminal(80, 24),
        "offline",
        Some("device missing"),
        false,
    );
    assert_eq!(expanded.len(), 3);
    assert!(expanded[0].contains("PANIC READY"));
    assert!(expanded[1].contains("route saved"));
    assert!(expanded[2].contains("[!] PANIC"));
    assert!(compact[0].contains("PANIC HELD"));
    assert!(compact.iter().all(|line| line.len() <= 79));
}

#[test]
fn rack_semantic_states_and_long_alerts_remain_distinguishable() {
    let markers: Vec<&str> = [
        RackLamp::Offline,
        RackLamp::Disabled,
        RackLamp::Enabled,
        RackLamp::Warning,
        RackLamp::Error,
    ]
    .into_iter()
    .map(RackLamp::marker)
    .collect();
    assert_eq!(
        markers.len(),
        markers.iter().copied().collect::<std::collections::HashSet<_>>().len()
    );
    let long_alert = "warning: ".to_owned() + &"x".repeat(200);
    let lines =
        rack_shell_lines(RackShellLayout::for_terminal(100, 37), "ready", Some(&long_alert), true);
    assert!(lines[1].len() <= 99);
    assert!(lines[1].starts_with("ALERT"));
}

#[test]
fn rack_shell_golden_frames_are_stable() {
    assert_eq!(
        rack_shell_lines(RackShellLayout::for_terminal(100, 37), "ready", Some("none"), true),
        vec![
            "MACKES RACK | HEALTH ready | PANIC READY",
            "ALERT  none",
            "[1] HOME  [2] ROUTING  [3] SCENES  [4] DIAGNOSTICS  [!] PANIC",
        ]
    );
    assert_eq!(
        rack_shell_lines(RackShellLayout::for_terminal(80, 24), "offline", None, false),
        vec![
            "MACKES RACK | HEALTH offline | PANIC HELD",
            "ALERT  none",
            "[1] HOME  [2] ROUTING  [3] DIAGNOSTICS",
        ]
    );
}

#[test]
fn controller_renderer_draws_required_viewports_without_overflow() {
    for (width, height) in [(100, 37), (80, 24)] {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("test backend");
        let dashboard = DashboardState { health: "ready".into(), ..DashboardState::default() };
        let editor = RoutingEditor::from_bank(&MappingBank::new());
        terminal
            .draw(|frame| draw_controller_mapping(frame, frame.area(), &dashboard, &editor))
            .expect("draw");
        let buffer = terminal.backend().buffer();
        let rendered: String = buffer.content().iter().map(ratatui::buffer::Cell::symbol).collect();
        assert!(rendered.contains("CONNECTED MIDI RACK"));
        assert!(rendered.contains("PANIC"));
    }
}

#[test]
fn task_shell_renderer_keeps_rail_focus_and_footer_visible() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test backend");
    let mut shell = TaskShellState::initial(3).expect("shell");
    shell.apply(ShellAction::Help);
    terminal
        .draw(|frame| draw_task_shell(frame, frame.area(), &shell, &DashboardState::default()))
        .expect("draw");
    let rendered: String =
        terminal.backend().buffer().content().iter().map(ratatui::buffer::Cell::symbol).collect();
    assert!(rendered.contains("▶ Live"));
    assert!(rendered.contains("Map Controls"));
    assert!(rendered.contains("Esc back"));
    assert!(rendered.contains("HELP"));
    assert!(!rendered.to_ascii_lowercase().contains("workspace"));
    assert!(!rendered.contains("1-9"));
}

#[test]
fn task_shell_renderer_shows_assignment_feedback_at_compact_width() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test backend");
    let shell = TaskShellState::initial(3).expect("shell");
    let dashboard = DashboardState {
        assignment_feedback: vec![
            "ASSIGNMENT  MOVE ONLY ONE CONTROL".into(),
            "CHOOSE DEVICE  1/2".into(),
        ],
        ..DashboardState::default()
    };
    terminal.draw(|frame| draw_task_shell(frame, frame.area(), &shell, &dashboard)).expect("draw");
    let buffer = terminal.backend().buffer();
    let rendered: String = buffer.content().iter().map(ratatui::buffer::Cell::symbol).collect();
    assert!(rendered.contains("MOVE ONLY ONE CONTROL"));
    assert!(rendered.contains("CHOOSE DEVICE"));
}

#[test]
fn task_shell_renderer_surfaces_missing_controller_template() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test backend");
    let shell = TaskShellState::initial(3).expect("shell");
    terminal
        .draw(|frame| draw_task_shell(frame, frame.area(), &shell, &DashboardState::default()))
        .expect("draw");
    let rendered: String =
        terminal.backend().buffer().content().iter().map(ratatui::buffer::Cell::symbol).collect();
    assert!(rendered.contains("MACKES TEMPLATE REQUIRED"));
}

#[test]
fn controller_renderer_preserves_degraded_and_dirty_state() {
    let backend = TestBackend::new(100, 37);
    let mut terminal = Terminal::new(backend).expect("test backend");
    let dashboard = DashboardState {
        health: "offline".into(),
        mapping_dirty: true,
        ..DashboardState::default()
    };
    let editor = RoutingEditor::from_bank(&MappingBank::new());
    terminal
        .draw(|frame| draw_controller_mapping(frame, frame.area(), &dashboard, &editor))
        .expect("draw");
    let rendered: String =
        terminal.backend().buffer().content().iter().map(ratatui::buffer::Cell::symbol).collect();
    assert!(rendered.contains("DEGRADED"));
    assert!(rendered.contains("SAVE REQUIRED"));
}

#[test]
fn launch_control_renderer_exposes_documented_faceplate_banks() {
    let backend = TestBackend::new(100, 37);
    let mut terminal = Terminal::new(backend).expect("test backend");
    let dashboard = DashboardState {
        health: "ready".into(),
        physical_devices: vec![PhysicalDevice {
            id: "launch-control-xl".into(),
            name: "Launch Control XL Mk2".into(),
            inputs: vec!["lc-in".into()],
            outputs: vec!["lc-out".into()],
            state: "connected".into(),
        }],
        ..DashboardState::default()
    };
    let editor = RoutingEditor::from_bank(&MappingBank::new());
    terminal
        .draw(|frame| draw_controller_mapping(frame, frame.area(), &dashboard, &editor))
        .expect("draw");
    let rendered: String =
        terminal.backend().buffer().content().iter().map(ratatui::buffer::Cell::symbol).collect();
    for label in ["T01", "M01", "B01", "DEVICE", "MUTE", "SOLO", "RECORD ARM", "UP", "DOWN"] {
        assert!(rendered.contains(label), "missing faceplate label {label}");
    }
    for label in [
        "EFFECTS: 6 groups",
        "Gain",
        "Gate",
        "Compressor",
        "Modulation",
        "Delay",
        "Reverb",
        "FADERS F01–F08",
        "UNUSED C37–C40",
    ] {
        assert!(rendered.contains(label), "missing effects faceplate label {label}");
    }
}

#[test]
fn primary_mapping_surface_keeps_multi_device_workflow_visible() {
    let backend = TestBackend::new(160, 37);
    let mut terminal = Terminal::new(backend).expect("test backend");
    let dashboard = DashboardState {
        health: "ready".into(),
        physical_devices: vec![
            PhysicalDevice {
                name: "Launch Control XL Mk2".into(),
                outputs: vec!["lc".into()],
                ..Default::default()
            },
            PhysicalDevice {
                name: "MicroPitch Pedal".into(),
                outputs: vec!["pitch".into()],
                ..Default::default()
            },
            PhysicalDevice {
                name: "Lexicon Reflex".into(),
                outputs: vec!["reflex".into()],
                ..Default::default()
            },
        ],
        ..DashboardState::default()
    };
    let editor = RoutingEditor::from_bank(&MappingBank::new());
    terminal
        .draw(|frame| draw_controller_mapping(frame, frame.area(), &dashboard, &editor))
        .expect("draw");
    let rendered: String =
        terminal.backend().buffer().content().iter().map(ratatui::buffer::Cell::symbol).collect();
    for label in [
        "Launch Control XL Mk2",
        "MicroPitch Pedal",
        "Lexicon Reflex",
        "SOURCE:",
        "DESTINATIONS:",
        "PARAMETERS:",
        "PANIC",
    ] {
        assert!(rendered.contains(label), "missing primary workflow element {label}");
    }
}

#[test]
fn launch_control_faceplate_highlights_only_validated_live_assignment() {
    let dashboard = DashboardState {
        live_activity: Some(LiveActivity {
            kind: "control_change".into(),
            channel: Some(1),
            number: Some(17),
            ..LiveActivity::default()
        }),
        launch_control_template: Some(LaunchControlTemplate {
            template: 0,
            assignments: vec![mackes_profiles::LaunchControlAssignment {
                index: 3,
                channel: 1,
                number: 17,
                kind: LaunchControlMessageKind::Cc,
                destination: None,
            }],
        }),
        ..DashboardState::default()
    };
    assert_eq!(launch_control_activity_index(&dashboard), Some(3));
    let mut without_template = dashboard;
    without_template.launch_control_template = None;
    assert_eq!(launch_control_activity_index(&without_template), None);
}

#[test]
fn persisted_launch_control_template_converts_fail_closed() {
    let template = mackes_config::LaunchControlTemplateConfig {
        template: 2,
        assignments: vec![mackes_config::LaunchControlAssignmentConfig {
            index: 4,
            channel: 1,
            number: 21,
            kind: "cc".into(),
            destination: Some("mixer:gain".into()),
            physical_control_id: None,
            needs_review: false,
        }],
    };
    let converted = launch_control_template_from_config(&template).expect("valid template");
    assert_eq!(converted.assignment(4).map(|assignment| assignment.number), Some(21));
    let invalid = mackes_config::LaunchControlTemplateConfig {
        assignments: vec![mackes_config::LaunchControlAssignmentConfig {
            kind: "unknown".into(),
            ..template.assignments[0].clone()
        }],
        ..template
    };
    assert_eq!(launch_control_template_from_config(&invalid), None);
}

#[test]
fn launch_control_template_readiness_fails_closed_for_missing_and_incomplete_layouts() {
    assert_eq!(launch_control_template_readiness(None), "MACKES TEMPLATE REQUIRED");
    let incomplete = LaunchControlTemplate { template: 1, assignments: Vec::new() };
    assert_eq!(launch_control_template_readiness(Some(&incomplete)), "MACKES TEMPLATE REQUIRED");
    let wrong_slot = LaunchControlTemplate { template: 0, assignments: Vec::new() };
    assert_eq!(launch_control_template_readiness(Some(&wrong_slot)), "MACKES TEMPLATE MISMATCH");
}

#[test]
fn reducer_rejects_gaps_and_accepts_contiguous_events() {
    let mut state = ClientState::default();
    state.apply_snapshot(StateSnapshot { last_sequence: 4, payload: b"snapshot".to_vec() });
    state.begin_reconnect();
    assert_eq!(state.payload, b"snapshot");
    assert_eq!(state.last_sequence, 0);
    assert!(state
        .apply_event(StateEvent { sequence: 1, payload: b"reconnected".to_vec() })
        .is_ok());
    assert_eq!(state.payload, b"reconnected");
    state.apply_snapshot(StateSnapshot { last_sequence: 4, payload: b"snapshot".to_vec() });
    assert_eq!(
        state.apply_snapshot_if_newer(StateSnapshot {
            last_sequence: 3,
            payload: b"stale".to_vec()
        }),
        Err(ReducerError::SequenceGap)
    );
    assert_eq!(state.payload, b"snapshot");
    assert_eq!(
        state.apply_event(StateEvent { sequence: 6, payload: b"gap".to_vec() }),
        Err(ReducerError::SequenceGap)
    );
    assert!(state.apply_event(StateEvent { sequence: 5, payload: b"event".to_vec() }).is_ok());
    assert_eq!(state.last_sequence, 5);
}

#[test]
fn reconnect_is_transactional_on_gap() {
    let mut state = ClientState { last_sequence: 9, payload: b"old".to_vec() };
    assert_eq!(
        state.apply_reconnect(
            StateSnapshot { last_sequence: 2, payload: b"new".to_vec() },
            &[StateEvent { sequence: 4, payload: b"gap".to_vec() }]
        ),
        Err(ReducerError::SequenceGap)
    );
    assert_eq!(state.last_sequence, 9);
    assert_eq!(state.payload, b"old");
}

#[test]
fn reconnect_applies_snapshot_and_events() {
    let mut state = ClientState::default();
    state
        .apply_reconnect(
            StateSnapshot { last_sequence: 2, payload: b"new".to_vec() },
            &[StateEvent { sequence: 3, payload: b"event".to_vec() }],
        )
        .expect("contiguous reconnect");
    assert_eq!(state.last_sequence, 3);
    assert_eq!(state.payload, b"event");
}

#[test]
fn default_keymap_exposes_safe_commands() {
    let keymap = Keymap;
    assert_eq!(keymap.command_for('n'), Some(UiCommand::NextScene));
    assert_eq!(keymap.command_for('!'), Some(UiCommand::Panic));
    assert_eq!(keymap.command_for('h'), Some(UiCommand::MoveLeft));
    assert_eq!(keymap.command_for('j'), Some(UiCommand::MoveDown));
    assert_eq!(keymap.command_for('k'), Some(UiCommand::MoveUp));
    assert_eq!(keymap.command_for('l'), Some(UiCommand::MoveRight));
    assert_eq!(keymap.command_for('3'), Some(UiCommand::OpenWorkspace(3)));
    assert_eq!(workspace_name(1), Some("Dashboard"));
    assert_eq!(workspace_name(5), Some("Routing"));
    assert_eq!(workspace_name(6), Some("Diagnostics"));
    assert_eq!(workspace_name(9), Some("Setlists"));
    assert_eq!(workspace_name(10), None);
    assert_eq!(keymap.description('h'), Some("Left"));
    assert_eq!(keymap.description('!'), Some("Panic"));
    assert_eq!(keymap.description('x'), None);
    assert_eq!(keymap.command_for('x'), None);
}

#[test]
fn task_shell_has_five_stable_sections_and_exact_breadcrumbs() {
    assert_eq!(
        AppSection::all().map(AppSection::label),
        ["Live", "Map Controls", "Scenes", "Devices", "System"]
    );
    assert_eq!(AppSection::MapControls.purpose(), "Assign physical controls to parameters");
    let focus = FocusPath::new(AppSection::MapControls, "Source control").expect("focus");
    assert_eq!(focus.breadcrumb(), "Map Controls / Source control");
    assert!(FocusPath::new(AppSection::Live, "").is_err());
}

#[test]
fn task_shell_navigation_is_bounded_and_resets_target_on_section_change() {
    let mut shell = TaskShellState::initial(2).expect("shell");
    shell.apply(ShellAction::Down);
    shell.apply(ShellAction::Down);
    assert_eq!(shell.target_index, 1);
    assert_eq!(shell.focus.target, "Target 2");
    shell.apply(ShellAction::Right);
    assert_eq!(shell.focus.section, AppSection::MapControls);
    assert_eq!(shell.target_index, 0);
    shell.apply(ShellAction::Left);
    shell.apply(ShellAction::Up);
    assert_eq!(shell.focus.section, AppSection::Live);
    assert_eq!(shell.target_index, 0);
}

#[test]
fn task_shell_primary_keys_and_vim_aliases_share_actions() {
    assert_eq!(shell_action_for_key(ShellKey::Up), Some(ShellAction::Up));
    assert_eq!(shell_action_for_key(ShellKey::Enter), Some(ShellAction::Enter));
    assert_eq!(shell_action_for_key(ShellKey::Esc), Some(ShellAction::Back));
    assert_eq!(shell_action_for_char('h'), Some(ShellAction::Left));
    assert_eq!(shell_action_for_char('l'), Some(ShellAction::Right));
    assert_eq!(shell_action_for_key(ShellKey::Help), Some(ShellAction::Help));
}

#[test]
fn ui_commands_project_only_to_governed_ipc_operations() {
    assert_eq!(ipc_command_for(UiCommand::Panic), Some(mackes_ipc::Command::Panic));
    assert_eq!(ipc_command_for(UiCommand::NextScene), Some(mackes_ipc::Command::Scenes));
    assert_eq!(ipc_command_for(UiCommand::OpenWorkspace(3)), None);
}

#[test]
fn mapping_filter_draft_validates_and_previews_engine_predicates() {
    let draft = MappingFilterDraft {
        predicates: vec![mackes_midi_engine::RoutePredicate::NumberRange {
            minimum: 10,
            maximum: 20,
        }],
    };
    assert!(draft.validate().is_ok());
    assert_eq!(draft.preview(), draft.predicates);
    let invalid = MappingFilterDraft {
        predicates: vec![mackes_midi_engine::RoutePredicate::ValueRange {
            minimum: 20,
            maximum: 10,
        }],
    };
    assert_eq!(invalid.validate(), Err("invalid MIDI value range"));
    let persisted = vec![
        mackes_config::LearnedFilter::NumberRange { minimum: 1, maximum: 2 },
        mackes_config::LearnedFilter::ValueRange { minimum: 3, maximum: 4 },
        mackes_config::LearnedFilter::Realtime { message: mackes_config::LearnedRealtime::Reset },
        mackes_config::LearnedFilter::SysExMask { pattern: vec![1], mask: vec![127] },
    ];
    assert_eq!(predicates_from_learned_filters(&persisted).expect("convert").len(), 4);
}

#[test]
fn mapped_midi_dashboard_actions_require_one_explicit_match() {
    let note = mackes_domain::MidiMessage::NoteOn {
        channel: mackes_domain::MidiChannel::new(1).expect("channel"),
        note: mackes_domain::SevenBit::new(36).expect("note"),
        velocity: mackes_domain::SevenBit::new(127).expect("velocity"),
    };
    let binding = DashboardMidiBinding {
        trigger: DashboardMidiTrigger::NoteOn { channel: 1, note: 36 },
        command: UiCommand::Panic,
    };
    assert_eq!(ui_command_for_midi(&note, &[binding]), Some(UiCommand::Panic));
    assert_eq!(ui_command_for_midi(&note, &[]), None);
    assert_eq!(
        ui_command_for_midi(&note, &[binding, binding]),
        None,
        "ambiguous mappings must not dispatch"
    );
}

#[test]
fn persisted_dashboard_bindings_convert_to_runtime_commands() {
    let bindings = vec![mackes_config::DashboardMidiBinding {
        trigger: mackes_config::DashboardMidiTrigger::ControlChange {
            channel: 1,
            controller: 20,
            value: None,
        },
        command: "next_scene".into(),
    }];
    let runtime = dashboard_bindings_from_config(&bindings).expect("runtime bindings");
    assert_eq!(runtime[0].command, UiCommand::NextScene);
    assert!(dashboard_bindings_from_config(&[mackes_config::DashboardMidiBinding {
        trigger: mackes_config::DashboardMidiTrigger::NoteOn { channel: 0, note: 1 },
        command: "panic".into(),
    }])
    .is_err());
}

#[test]
fn dashboard_action_polling_is_bounded_and_observational() {
    struct FakeInput {
        info: mackes_midi_engine::EndpointInfo,
        events: std::collections::VecDeque<mackes_domain::MidiEvent>,
    }
    impl mackes_midi_engine::MidiInputAdapter for FakeInput {
        fn info(&self) -> &mackes_midi_engine::EndpointInfo {
            &self.info
        }
        fn receive(&mut self) -> Option<mackes_domain::MidiEvent> {
            self.events.pop_front()
        }
    }
    let event = mackes_domain::MidiEvent {
        timestamp: mackes_domain::TimestampNanos::new(1),
        sequence: 1,
        endpoint: mackes_domain::EndpointId::new(1).expect("endpoint"),
        message: mackes_domain::MidiMessage::NoteOn {
            channel: mackes_domain::MidiChannel::new(1).expect("channel"),
            note: mackes_domain::SevenBit::new(36).expect("note"),
            velocity: mackes_domain::SevenBit::new(127).expect("velocity"),
        },
    };
    let mut input = FakeInput {
        info: mackes_midi_engine::EndpointInfo {
            id: "learn-input".into(),
            name: "test input".into(),
            direction: mackes_midi_engine::EndpointDirection::Input,
        },
        events: std::collections::VecDeque::from([event.clone(), event]),
    };
    let binding = DashboardMidiBinding {
        trigger: DashboardMidiTrigger::NoteOn { channel: 1, note: 36 },
        command: UiCommand::NextScene,
    };
    assert_eq!(poll_dashboard_actions(&mut input, &[binding], 1), vec![UiCommand::NextScene]);
    assert_eq!(input.events.len(), 1);
}

#[test]
fn signal_flow_diagram_preserves_order_and_rejects_empty_nodes() {
    let diagram = SignalFlowDiagram::new(
        vec![SignalFlowNode { id: "reflex".into(), label: "Lexicon Reflex".into(), online: true }],
        3,
    )
    .expect("diagram");
    assert_eq!(diagram.generation, 3);
    assert_eq!(diagram.nodes[0].label, "Lexicon Reflex");
    assert!(SignalFlowDiagram::new(
        vec![SignalFlowNode { id: String::new(), label: "x".into(), online: false }],
        1
    )
    .is_err());
}

#[test]
fn blueprint_renderer_is_deterministic_and_labels_inferred_topology() {
    let diagram = SignalFlowDiagram::new(
        vec![
            SignalFlowNode { id: "input".into(), label: "Input".into(), online: true },
            SignalFlowNode { id: "cabinet".into(), label: "Cabinet".into(), online: false },
        ],
        3,
    )
    .expect("diagram");
    let lines = blueprint_lines(&diagram, true);
    assert_eq!(lines[0], "Inferred logical/control view — not authoritative DSP topology");
    assert_eq!(lines[1], "+--[Input]--+");
    assert!(lines[2].contains("input online"));
    assert_eq!(lines[3], "      |      v");
    assert!(lines[5].contains("cabinet offline"));
}

#[test]
fn viewport_clamps_and_selects_compact_layout() {
    assert_eq!(Viewport::new(0, 0), Viewport { width: 1, height: 1 });
    assert!(!Viewport::new(80, 24).compact());
    assert!(Viewport::new(79, 24).compact());
    assert!(dashboard_panels(false).contains(&DashboardPanel::RecentEvents));
    assert!(!dashboard_panels(true).contains(&DashboardPanel::SignalFlow));
    assert!(dashboard_panels(true).contains(&DashboardPanel::Panic));
}

#[test]
fn page_navigation_is_bounded_and_wraps() {
    let mut page = PageState::new(9, 3);
    assert_eq!(page.current, 2);
    page.next();
    assert_eq!(page.current, 0);
    page.previous();
    assert_eq!(page.current, 2);
    page.select(99);
    assert_eq!(page.current, 2);
}

#[test]
fn terminal_guard_restores_idempotently() {
    let mut guard = TerminalGuard::default();
    assert!(!guard.needs_restore());
    guard.acquire();
    assert!(guard.needs_restore());
    guard.restore();
    guard.restore();
    assert!(!guard.needs_restore());
}

#[test]
fn dashboard_initial_state_keeps_panic_available() {
    let dashboard = DashboardState::initial();
    assert_eq!(dashboard.health, "starting");
    assert!(dashboard.panic_available);
    assert!(dashboard.frame_lines().iter().any(|line| line.contains("PANIC: available")));
    assert!(dashboard.frame_lines_for(Viewport::new(20, 10)).iter().all(|line| line.len() <= 20));
    assert!(dashboard.frame_lines_for(Viewport::new(6, 10)).iter().any(|line| line == "PANIC "));
    assert!(dashboard.frame_lines().iter().any(|line| line.contains("keys:")));
    assert!(dashboard
        .frame_lines_for(Viewport::new(20, 10))
        .iter()
        .any(|line| line.starts_with("keys: 1-9")));
    assert!(!dashboard.performance_locked);
    assert_eq!(dashboard.activation_progress, (0, 0));
    let mut dashboard = dashboard;
    dashboard.set_activity(10, 8, 1);
    dashboard.set_activation_progress(9, 4);
    dashboard.set_activation_progress(2, 4);
    assert_eq!((dashboard.received, dashboard.sent, dashboard.dropped), (10, 8, 1));
    assert_eq!(dashboard.activation_progress, (4, 4));
}

#[test]
fn dashboard_event_projection_updates_widgets_and_keeps_panic() {
    let mut dashboard = DashboardState::initial();
    dashboard.apply_event(DashboardEvent::Health("ready".into()));
    dashboard.apply_event(DashboardEvent::ActiveScene(Some("intro".into())));
    dashboard.apply_event(DashboardEvent::PerformanceLock(true));
    dashboard.apply_event(DashboardEvent::ActivationProgress { completed: 2, total: 3 });
    dashboard.apply_event(DashboardEvent::ActivationResult("partial: 1 failed".into()));
    dashboard.apply_event(DashboardEvent::DeviceHealth(vec![("processor".into(), "ready".into())]));
    dashboard.apply_event(DashboardEvent::Notification {
        severity: SemanticToken::Warning,
        message: "endpoint degraded".into(),
    });
    assert_eq!(dashboard.health, "ready");
    assert_eq!(dashboard.active_scene.as_deref(), Some("intro"));
    assert!(dashboard.performance_locked);
    assert_eq!(dashboard.activation_progress, (2, 3));
    assert_eq!(dashboard.activation_result.as_deref(), Some("partial: 1 failed"));
    assert_eq!(dashboard.device_health, vec![("processor".into(), "ready".into())]);
    assert!(dashboard.frame_lines().iter().any(|line| line == "device=processor health=ready"));
    assert!(dashboard.frame_lines().iter().any(|line| line == "[WARN] endpoint degraded"));
    dashboard
        .set_device_health((0..33).map(|index| (format!("d{index}"), "ready".into())).collect());
    assert_eq!(dashboard.device_health.len(), 32);
    assert!(dashboard.panic_available);
}

#[test]
fn live_activity_age_advances_and_resets_on_new_activity() {
    let mut dashboard = DashboardState::initial();
    dashboard.advance_live_activity_age(1_500_000_000);
    assert_eq!(dashboard.live_activity_age_nanos, 1_500_000_000);
    dashboard.apply_event(DashboardEvent::LiveActivity(LiveActivity {
        control_id: "endpoint:1:cc:7".into(),
        ..LiveActivity::default()
    }));
    assert_eq!(dashboard.live_activity_age_nanos, 0);
    dashboard.advance_live_activity_age(u64::MAX);
    assert_eq!(dashboard.live_activity_age_nanos, u64::MAX);
}

#[test]
fn live_activity_age_label_decays_after_one_second() {
    assert_eq!(live_activity_age_label(0), "ACTIVE");
    assert_eq!(live_activity_age_label(999_999_999), "ACTIVE");
    assert_eq!(live_activity_age_label(1_000_000_000), "STALE");
}

#[test]
fn ready_health_is_rendered_as_online() {
    assert!(health_is_online("ready"));
    assert!(health_is_online("online"));
    assert!(!health_is_online("degraded"));
}

#[test]
fn dashboard_notifications_are_newest_first_and_bounded() {
    let mut dashboard = DashboardState::initial();
    dashboard.notify(SemanticToken::Warning, "peer reconnecting");
    dashboard.notify(SemanticToken::Success, "scene active");
    assert_eq!(dashboard.notifications[0].message, "scene active");
    assert!(dashboard.frame_lines().iter().any(|line| line == "[WARN] peer reconnecting"));
    for index in 0..20 {
        dashboard.notify(SemanticToken::Midi, format!("event-{index}"));
    }
    assert_eq!(dashboard.notifications.len(), 16);
    assert_eq!(dashboard.notifications[0].message, "event-19");
    assert!(!dashboard.notifications.iter().any(|notice| notice.message == "peer reconnecting"));
}

#[test]
fn dashboard_payload_projection_decodes_authoritative_fields() {
    let payload = serde_json::json!({
        "health": "ready",
        "active_scene": "intro",
        "route_generation": 7,
        "route_undo_available": true,
        "audit_count": 3,
        "audit": [{"action": "route_replace", "allowed": true}],
        "received": 10,
        "sent": 8,
        "dropped": 2,
        "activation_result": "total=2 succeeded=2 failed=0",
        "physical_devices": [{
            "id": "launch control xl",
            "name": "Launch Control XL",
            "inputs": ["input-1"],
            "outputs": ["output-1"],
            "state": "connected"
        }],
        "last_activity": {
            "source_endpoint": 11,
            "source_endpoint_id": "midir-in-source",
            "control_id": "endpoint:11:control_change:21",
            "timestamp_nanos": 123_456,
            "kind": "control_change",
            "number": 21,
            "value": 96,
            "destination_endpoints": [22, 23],
            "sequence": 44
        }
    });
    let mut dashboard = DashboardState::initial();
    for event in DashboardEvent::from_payload(&payload) {
        dashboard.apply_event(event);
    }
    assert_eq!(dashboard.health, "ready");
    assert_eq!(dashboard.active_scene.as_deref(), Some("intro"));
    assert_eq!(dashboard.route_generation, 7);
    assert_eq!((dashboard.received, dashboard.sent, dashboard.dropped), (10, 8, 2));
    assert_eq!(dashboard.activation_result.as_deref(), Some("total=2 succeeded=2 failed=0"));
    assert_eq!(dashboard.physical_devices.len(), 1);
    assert_eq!(dashboard.physical_devices[0].name, "Launch Control XL");
    assert_eq!(dashboard.physical_devices[0].inputs, vec!["input-1"]);
    assert_eq!(dashboard.physical_devices[0].outputs, vec!["output-1"]);
    assert!(dashboard.route_undo_available);
    assert_eq!(dashboard.audit_count, 3);
    assert_eq!(dashboard.latest_audit.as_deref(), Some("ALLOW route_replace"));
    let activity = dashboard.live_activity.expect("activity");
    assert_eq!(activity.source_endpoint, 11);
    assert_eq!(activity.source_endpoint_id.as_deref(), Some("midir-in-source"));
    assert_eq!(activity.control_id, "endpoint:11:control_change:21");
    assert_eq!(activity.timestamp_nanos, 123_456);
    assert_eq!(activity.kind, "control_change");
    assert_eq!(activity.number, Some(21));
    assert_eq!(activity.value, Some(96));
    assert_eq!(activity.destination_endpoints, vec![22, 23]);
    assert_eq!(activity.sequence, 44);
    assert!(DashboardEvent::from_payload(&serde_json::json!({
        "last_activity": {"kind": "invalid"}
    }))
    .iter()
    .all(|event| !matches!(event, DashboardEvent::LiveActivity(_))));
}

#[test]
fn mapping_chain_line_exposes_enabled_state_and_destination() {
    let draft = MappingDraft {
        source: "Launch Control XL / K01".into(),
        destination: "MicroPitch / Mix".into(),
        destination_parameter: None,
        enabled: true,
        ..MappingDraft::default()
    };
    let line = mapping_chain_line(&[draft]);
    assert!(line.contains("● Launch Control XL / K01→MicroPitch / Mix"));
    assert_eq!(mapping_chain_line(&[]), "NONE — press a to add a source → destination route");
}

#[test]
fn live_value_bar_is_bounded_and_monotonic() {
    assert_eq!(value_bar(0), "[----------------]");
    assert_eq!(value_bar(127), "[################]");
    assert!(value_bar(96).contains("############"));
    assert_eq!(value_bar(u16::MAX).len(), value_bar(127).len());
}

#[test]
fn destination_inventory_line_is_bounded_and_explicit() {
    let devices = vec![PhysicalDevice {
        name: "MicroPitch Pedal".into(),
        inputs: vec!["midir-in-pedal".into(), "midir-in-pedal-2".into()],
        outputs: vec!["midir-out-pedal".into()],
        ..PhysicalDevice::default()
    }];
    assert_eq!(destination_inventory_line(&devices, None), " MicroPitch Pedal:midir-out-pedal");
    assert_eq!(destination_inventory_line(&devices, Some(0)), ">MicroPitch Pedal:midir-out-pedal");
    assert_eq!(destination_inventory_line(&[], None), "NO OUTPUT DESTINATIONS");
    let mut dashboard = DashboardState::initial();
    dashboard.physical_devices = devices;
    dashboard.cycle_destination();
    assert_eq!(dashboard.selected_destination, Some(0));
    assert!(dashboard.first_input_endpoint().is_some());
    dashboard.cycle_input();
    assert_eq!(dashboard.selected_input, Some(0));
    assert!(dashboard.selected_input_endpoint().is_some());
    assert!(dashboard.selected_destination_endpoint().is_some());
    assert!(destination_parameter_line(&dashboard.physical_devices, Some(0), None).contains("Mix"));
    dashboard.cycle_parameter();
    assert!(destination_parameter_line(
        &dashboard.physical_devices,
        Some(0),
        dashboard.selected_parameter
    )
    .contains('>'));
}

#[test]
fn destination_browser_filters_unknown_devices_and_cycles_supported_profiles() {
    let names = ["Reflex"];
    for name in names {
        let device =
            PhysicalDevice { name: name.into(), outputs: vec!["out".into()], ..Default::default() };
        let line = destination_parameter_line(&[device], Some(0), None);
        assert!(!line.contains("UNVERIFIED"), "{name}: {line}");
        assert!(!line.is_empty());
    }
    let unknown = PhysicalDevice {
        name: "Unrecognized Processor".into(),
        outputs: vec!["out".into()],
        ..Default::default()
    };
    assert_eq!(
        destination_parameter_line(&[unknown], Some(0), None),
        "PROFILE PARAMETERS UNVERIFIED"
    );
}

#[test]
fn semantic_tokens_have_readable_contrast_contract() {
    assert!(contrast_is_readable((255, 255, 255), (0, 0, 0)));
    assert!(!contrast_is_readable((120, 120, 120), (128, 128, 128)));
    assert_eq!(SemanticToken::Hazard, SemanticToken::Hazard);
    assert_eq!(token_marker(SemanticToken::Setup), "[SETUP]");
    assert_eq!(token_marker(SemanticToken::Midi), "[MIDI]");
    assert_eq!(token_marker(SemanticToken::Sysex), "[SYSEX]");
    assert_eq!(token_marker(SemanticToken::Success), "[OK]");
    assert_eq!(token_marker(SemanticToken::Warning), "[WARN]");
    assert_eq!(token_marker(SemanticToken::Error), "[ERROR]");
    assert_eq!(token_marker(SemanticToken::Hazard), "[HAZARD]");
    assert_eq!(intensity_marker(TokenIntensity::Dim), "(dim)");
    assert_eq!(intensity_marker(TokenIntensity::Normal), "");
    assert_eq!(intensity_marker(TokenIntensity::Selected), "(selected)");
    assert_eq!(intensity_marker(TokenIntensity::Hazard), "(hazard)");
    let palette = [PaletteEntry {
        token: SemanticToken::Hazard,
        foreground: (255, 255, 255),
        background: (0, 0, 0),
    }];
    assert!(validate_palette(&palette).is_ok());
    assert!(validate_palette(&[
        palette[0],
        PaletteEntry {
            token: SemanticToken::Hazard,
            foreground: (255, 255, 255),
            background: (0, 0, 0)
        },
    ])
    .is_err());
    assert!(validate_palette(&[PaletteEntry {
        token: SemanticToken::Error,
        foreground: (120, 120, 120),
        background: (128, 128, 128)
    }])
    .is_err());
    assert_eq!(default_palette().len(), 7);
    assert!(validate_palette(&default_palette()).is_ok());
    assert_eq!(
        palette_entry(&default_palette(), SemanticToken::Midi).map(|entry| entry.foreground),
        Some((255, 255, 255))
    );
    assert!(palette_entry(&palette, SemanticToken::Midi).is_none());
    let theme = Theme { version: 1, palette: default_palette().to_vec() };
    assert!(theme.validate().is_ok());
    assert_eq!(
        Theme { version: 1, palette: theme.palette[..6].to_vec() }.validate(),
        Err("theme must cover every semantic token")
    );
    assert_eq!(
        Theme { version: 0, palette: default_palette().to_vec() }.validate(),
        Err("theme version must be nonzero")
    );
}

#[test]
fn learn_requires_explicit_candidate_and_enter_commit_path() {
    let mut learn = LearnWorkspace::new();
    assert!(learn.set_input_alias("launch-control").is_ok());
    learn.start_capture();
    learn.finish_capture(vec![MidiLearnCandidate {
        kind: mackes_midi_engine::LearnMessageKind::ControlChange,
        channel: Some(1),
        number: Some(7),
        observations: 3,
        minimum: Some(0),
        maximum: Some(127),
        raw: vec![0xB0, 7, 127],
    }]);
    assert_eq!(learn.phase, LearnPhase::Review);
    learn.select(99);
    assert_eq!(learn.phase, LearnPhase::Review);
    learn.select(0);
    learn.handle_key(LearnKey::Other);
    assert_eq!(learn.phase, LearnPhase::Destination);
    learn.handle_key(LearnKey::Enter);
    assert_eq!(learn.phase, LearnPhase::Destination);
    assert!(learn.set_destination("pedal.mix", MappingMode::Note).is_err());
    assert!(learn.set_destination("pedal.mix", MappingMode::Cc).is_ok());
    assert!(learn.set_channel_policy(LearnChannelPolicy::Any).is_ok());
    learn.begin_live_test();
    assert_eq!(learn.phase, LearnPhase::Testing);
    assert!(learn
        .frame_lines(Viewport::new(80, 24))
        .iter()
        .any(|line| line.contains("awaiting daemon/device result")));
    learn.finish_live_test(false);
    learn.handle_key(LearnKey::Enter);
    assert_eq!(learn.phase, LearnPhase::Destination);
    learn.begin_live_test();
    learn.finish_live_test(true);
    learn.handle_key(LearnKey::Enter);
    assert_eq!(learn.phase, LearnPhase::Committed);
    let mapping = learn.committed_mapping().expect("committed mapping");
    assert_eq!(mapping.source, "launch-control");
    assert_eq!(mapping.destination, "pedal.mix");
    assert_eq!(mapping.mode, MappingMode::Cc);
    assert_eq!(mapping.channel, None);
    let durable = learn.committed_learned_mapping().expect("durable mapping");
    assert_eq!(durable.message_kind, "control_change");
    assert_eq!(durable.number, Some(7));
    assert_eq!(durable.raw, vec![0xB0, 7, 127]);
    assert_eq!(durable.channel_policy, mackes_config::LearnedChannelPolicy::Any);
    let config = mackes_config::ConfigDocument {
        schema_version: mackes_config::CURRENT_SCHEMA_VERSION,
        endpoints: vec![mackes_config::EndpointAlias {
            id: "launch-control".into(),
            name: None,
            vendor_id: None,
            product_id: None,
            serial: None,
        }],
        ..mackes_config::ConfigDocument::default()
    };
    let persisted = learn.apply_to_config(&config).expect("persisted projection");
    assert_eq!(persisted.settings.learn_input_alias.as_deref(), Some("launch-control"));
    assert_eq!(persisted.learned_mappings, vec![durable]);
    assert!(learn.apply_to_config(&persisted).is_err());
    let frame = learn.frame_lines(Viewport::new(80, 24));
    assert!(frame.iter().any(|line| line.contains("raw=B0 07 7F")));
    assert!(frame.iter().any(|line| line.contains("destination=pedal.mix")));
    learn.handle_key(LearnKey::Escape);
    assert_eq!(learn.phase, LearnPhase::Cancelled);
}

#[test]
fn learn_refuses_capture_without_global_input_alias() {
    let mut learn = LearnWorkspace::new();
    learn.start_capture();
    assert_eq!(learn.phase, LearnPhase::Armed);
    assert!(learn.set_input_alias("").is_err());
    assert!(learn.set_input_alias("midi-in").is_ok());
    learn.start_capture();
    assert_eq!(learn.phase, LearnPhase::Capturing);
    assert!(learn.set_input_alias("other").is_err());
}

#[test]
fn learn_rollback_clears_transaction_but_retains_input_alias() {
    let mut learn = LearnWorkspace::new();
    learn.set_input_alias("launch-control").expect("alias");
    learn.start_capture();
    learn.finish_capture(Vec::new());
    learn.rollback();
    assert_eq!(learn.phase, LearnPhase::Armed);
    assert!(learn.candidates.is_empty());
    assert_eq!(learn.learn_input_alias.as_deref(), Some("launch-control"));
    assert!(learn.committed_mapping().is_none());
}

#[test]
fn mapping_draft_validates_without_partial_mutation() {
    let draft = MappingDraft {
        source: "launchpad".into(),
        destination: "reflex".into(),
        destination_parameter: None,
        channel: Some(1),
        enabled: true,
        mode: MappingMode::Cc,
        priority: 0,
        curve: mackes_midi_engine::Curve::Linear,
        filters: MappingFilterDraft::default(),
        allow_cycle: false,
    };
    assert!(draft.validate().is_ok());
    assert_eq!(draft.curve, mackes_midi_engine::Curve::Linear);
    let curved = MappingDraft { curve: mackes_midi_engine::Curve::Square, ..draft.clone() };
    assert!(curved.validate().is_ok());
    let filtered = MappingDraft {
        filters: MappingFilterDraft {
            predicates: vec![mackes_midi_engine::RoutePredicate::NumberRange {
                minimum: 10,
                maximum: 20,
            }],
        },
        ..draft.clone()
    };
    assert!(filtered.validate().is_ok());
    let invalid = MappingDraft { channel: Some(17), ..draft.clone() };
    assert!(invalid.validate().is_err());
    let duplicate = MappingDraft { priority: 1, ..draft.clone() };
    assert!(draft.conflicts_with(&duplicate));
    assert!(validate_mapping_batch(&[draft.clone(), duplicate]).is_err());
    assert!(reorder_mapping_drafts(std::slice::from_ref(&draft), &[0]).is_ok());
    assert!(reorder_mapping_drafts(std::slice::from_ref(&draft), &[1]).is_err());
    let reordered = order_mapping_drafts(&[
        MappingDraft { priority: 20, ..draft.clone() },
        MappingDraft { priority: 10, ..draft.clone() },
        MappingDraft { priority: 10, ..draft.clone() },
    ]);
    assert_eq!(reordered.iter().map(|draft| draft.priority).collect::<Vec<_>>(), vec![10, 10, 20]);
    let mut bank = MappingBank::new();
    assert_eq!(bank.commit(vec![draft.clone()]), Ok(1));
    assert_eq!(bank.drafts(), std::slice::from_ref(&draft));
    assert_eq!(
        bank.commit_if_generation(0, vec![draft.clone()]),
        Err("mapping generation changed concurrently".into())
    );
    assert_eq!(bank.generation(), 1);
    assert_eq!(bank.commit_if_generation(1, vec![draft.clone()]), Ok(2));
    let invalid = MappingDraft { source: String::new(), ..draft.clone() };
    assert!(bank.commit(vec![invalid]).is_err());
    assert_eq!(bank.generation(), 2);
    assert!(bank.undo_available());
    assert_eq!(bank.undo(), Ok(3));
    assert_eq!(bank.drafts(), std::slice::from_ref(&draft));
    let mut editor = RoutingEditor::from_bank(&bank);
    assert!(editor.reorder(&[0]).is_ok());
    editor.selected = Some(0);
    editor.cycle_selected_curve().expect("curve");
    assert_eq!(editor.drafts[0].curve, mackes_midi_engine::Curve::Square);
    editor.cycle_selected_filter().expect("number filter");
    assert_eq!(editor.drafts[0].filters.predicates.len(), 1);
    editor.cycle_selected_filter().expect("value filter");
    editor.cycle_selected_filter().expect("realtime filter");
    editor.cycle_selected_filter().expect("sysex filter");
    editor.cycle_selected_filter().expect("clear filter");
    assert!(editor.drafts[0].filters.predicates.is_empty());
    assert_eq!(editor.remove(4), Err("mapping index is out of range"));
}

#[test]
fn routing_editor_cycles_channels_transactionally() {
    let draft = MappingDraft {
        source: "1".into(),
        destination: "2".into(),
        destination_parameter: None,
        channel: None,
        enabled: true,
        mode: MappingMode::Cc,
        priority: 0,
        curve: mackes_midi_engine::Curve::Linear,
        filters: MappingFilterDraft::default(),
        allow_cycle: false,
    };
    let mut editor = RoutingEditor::from_bank(&MappingBank::new());
    editor.add(draft).expect("draft");
    assert_eq!(editor.drafts[0].channel, None);
    editor.cycle_selected_channel().expect("channel 1");
    assert_eq!(editor.drafts[0].channel, Some(1));
    for _ in 0..15 {
        editor.cycle_selected_channel().expect("channel");
    }
    assert_eq!(editor.drafts[0].channel, Some(16));
    editor.cycle_selected_channel().expect("any channel");
    assert_eq!(editor.drafts[0].channel, None);
    editor.toggle_selected_enabled().expect("disable");
    assert!(!editor.drafts[0].enabled);
    editor.adjust_selected_priority(7).expect("raise priority");
    assert_eq!(editor.drafts[0].priority, 7);
    editor.adjust_selected_priority(-3).expect("lower priority");
    assert_eq!(editor.drafts[0].priority, 4);
    editor.adjust_selected_priority(-i32::from(u16::MAX)).expect("saturate priority");
    assert_eq!(editor.drafts[0].priority, 0);
}

#[test]
fn backup_workspace_separates_plan_confirmation_and_result() {
    let mut workspace = BackupWorkspace::new();
    workspace.plan("nothing selected");
    assert_eq!(workspace.phase, BackupPhase::Listing);
    workspace.inspect("reflex.dump");
    workspace.set_identity_warning(true);
    workspace.plan("compatible");
    assert!(workspace.identity_warning);
    workspace.cancel();
    assert_eq!(workspace.phase, BackupPhase::Listing);
    workspace.inspect("reflex.dump");
    workspace.plan("compatible");
    workspace.confirm();
    workspace.begin_apply();
    workspace.finish(BackupPhase::Verified, "read-back matched");
    assert_eq!(workspace.phase, BackupPhase::Verified);
    assert_eq!(
        BackupPhase::from(mackes_config::BackupStatus::SentUnverified),
        BackupPhase::SentUnverified
    );
    assert_eq!(BackupPhase::from(mackes_config::BackupStatus::Failed), BackupPhase::Failed);
    workspace.cancel();
    assert_eq!(workspace.phase, BackupPhase::Verified);
}

#[test]
fn device_operation_preview_exposes_destination_and_risk_before_send() {
    let query =
        DeviceOperationPreview::new("reflex", "active setup query", DeviceOperationRisk::ReadOnly)
            .expect("query preview");
    assert!(!query.requires_confirmation());
    let write = DeviceOperationPreview::new(
        "reflex",
        "store register",
        DeviceOperationRisk::PersistentWrite,
    )
    .expect("write preview");
    assert!(write.requires_confirmation());
    assert_eq!(
        DeviceOperationPreview::new("", "query", DeviceOperationRisk::ReadOnly),
        Err("device operation preview fields must not be empty")
    );
}

#[test]
fn monitor_is_bounded_newest_first_and_filterable() {
    let mut monitor = MonitorState::new(2).expect("capacity");
    monitor.push(MonitorEntry { severity: MonitorSeverity::Info, message: "a".into() });
    monitor.push(MonitorEntry { severity: MonitorSeverity::Error, message: "b".into() });
    monitor.push(MonitorEntry { severity: MonitorSeverity::Warning, message: "c".into() });
    assert_eq!(monitor.entries[0].message, "c");
    assert_eq!(monitor.entries.len(), 2);
    assert_eq!(monitor.filtered(MonitorSeverity::Warning).len(), 2);
    monitor.set_paused(true);
    monitor.push(MonitorEntry { severity: MonitorSeverity::Error, message: "ignored".into() });
    assert_eq!(monitor.entries[0].message, "c");
    assert_eq!(monitor.export().len(), 2);
    assert_eq!(monitor.export_redacted(&[0])[0].message, "<redacted>");
    assert_eq!(monitor.export_redacted(&[0])[1].message, "b");
    monitor.set_paused(false);
    monitor.push(MonitorEntry { severity: MonitorSeverity::Error, message: "d".into() });
    assert_eq!(monitor.entries[0].message, "d");
    assert!(monitor.frame_lines(Viewport::new(30, 3))[0].contains("monitor=live"));
}

#[test]
fn diagnostics_explain_degraded_health_with_bounded_lines() {
    let mut diagnostics = DiagnosticsState::new();
    diagnostics.push(HealthDiagnostic {
        subject: "reflex".into(),
        severity: MonitorSeverity::Warning,
        reason: "endpoint offline".into(),
        remediation: "reconnect MIDI cable".into(),
    });
    assert_eq!(
        diagnostics.lines(),
        vec!["Warning reflex: endpoint offline -> reconnect MIDI cable".to_owned()]
    );
    for index in 0..40 {
        diagnostics.push(HealthDiagnostic {
            subject: format!("d{index}"),
            severity: MonitorSeverity::Info,
            reason: "ready".into(),
            remediation: "none".into(),
        });
    }
    assert_eq!(diagnostics.entries.len(), DiagnosticsState::CAPACITY);
    assert_eq!(diagnostics.entries[0].subject, "d39");
    assert!(diagnostics
        .frame_lines(Viewport::new(40, 4))
        .iter()
        .all(|line| line.chars().count() <= 40));
}

#[test]
fn setlist_editor_keeps_edits_transactional_and_validated() {
    let source =
        mackes_config::Setlist { id: "live".into(), projects: vec!["a".into(), "b".into()] };
    let mut editor = SetlistEditor::from_snapshot(std::slice::from_ref(&source));
    assert!(editor.reorder_selected(&["b", "a"]).is_err());
    editor.select(0).expect("selection");
    editor.reorder_selected(&["b", "a"]).expect("reorder");
    assert!(editor.append_project("c").is_ok());
    assert!(editor.append_project("c").is_err());
    assert_eq!(editor.remove_last_project(), Ok("c".into()));
    assert!(editor.append_project("c").is_ok());
    editor.move_last_project(false).expect("rotate left");
    assert_eq!(editor.drafts[0].projects, vec!["a", "c", "b"]);
    editor.move_last_project(true).expect("rotate right");
    assert_eq!(editor.drafts[0].projects, vec!["b", "a", "c"]);
    editor.copy_selected("encore").expect("copy");
    assert_eq!(editor.add_empty(), "new-setlist-1");
    assert_eq!(editor.add_empty(), "new-setlist-2");
    assert_eq!(editor.drafts[0].projects, vec!["b", "a", "c"]);
    assert_eq!(source.projects, vec!["a", "b"]);
    assert!(editor.frame_lines(Viewport::new(80, 24))[0].contains("uncommitted"));
    assert_eq!(editor.commit().len(), 4);
}

#[test]
fn device_control_availability_never_treats_unverified_state_as_actionable() {
    assert!(control_is_actionable(ControlAvailability::Available));
    assert!(control_is_actionable(ControlAvailability::Hazardous));
    assert!(!control_is_actionable(ControlAvailability::ReadOnly));
    assert!(!control_is_actionable(ControlAvailability::Unavailable));
}

#[test]
fn reflex_workspace_preserves_manual_labels_and_highlights_controls() {
    let diagram = SignalFlowDiagram::new(
        vec![
            SignalFlowNode { id: "input".into(), label: "Input".into(), online: true },
            SignalFlowNode { id: "algo".into(), label: "Algorithm".into(), online: true },
        ],
        4,
    )
    .expect("diagram");
    let mut workspace = ReflexWorkspace::new(
        "algorithm-1".into(),
        "Documented Algorithm".into(),
        vec![ReflexControl {
            id: "midi-channel".into(),
            label: "MIDI Channel".into(),
            node_id: "input".into(),
            available: true,
        }],
        vec![ReflexControl {
            id: "decay".into(),
            label: "Decay".into(),
            node_id: "algo".into(),
            available: true,
        }],
        diagram,
    )
    .expect("workspace");
    let linked = workspace.select_node("algo").expect("node");
    let linked_label = linked[0].label.clone();
    drop(linked);
    assert_eq!(workspace.selected_node.as_deref(), Some("algo"));
    assert_eq!(linked_label, "Decay");
    assert_eq!(workspace.shared_controls()[0].label, "MIDI Channel");
    let frame = workspace.frame_lines(Viewport::new(80, 24));
    assert_eq!(frame[1], "Logical/control view");
    assert!(frame.iter().any(|line| line == "> [algo] Algorithm"));
    assert!(frame.iter().any(|line| line.contains("[available] Decay")));
    assert!(workspace.select_node("missing").is_err());
}

#[test]
fn profile_device_frame_preserves_signal_and_control_order() {
    let diagram = SignalFlowDiagram::new(
        vec![
            SignalFlowNode { id: "pitch".into(), label: "Pitch Shift".into(), online: true },
            SignalFlowNode { id: "delay".into(), label: "Delay".into(), online: true },
        ],
        4,
    )
    .expect("diagram");
    let mut workspace = DeviceWorkspace::new(
        "eventide.micropitch".into(),
        "Eventide MicroPitch".into(),
        diagram,
        vec!["Bypass".into(), "MIDI Channel".into()],
        vec![
            DeviceControlGroup {
                id: "pitch".into(),
                label: "Pitch".into(),
                block_id: "pitch".into(),
                control_ids: vec!["Pitch A".into(), "Pitch B".into()],
            },
            DeviceControlGroup {
                id: "delay".into(),
                label: "Delay".into(),
                block_id: "delay".into(),
                control_ids: vec!["Delay A".into(), "Delay B".into()],
            },
        ],
        false,
    )
    .expect("workspace");
    workspace.select_block("delay").expect("selection");
    let frame = workspace.frame_lines(Viewport::new(80, 24));
    assert_eq!(frame[1], "Logical/control view");
    assert!(frame.iter().any(|line| line == "> [delay] Delay"));
    assert!(frame.iter().any(|line| line == "    Delay: Delay A, Delay B"));
    assert!(frame.iter().all(|line| line.chars().count() <= 80));
}

#[test]
fn eventide_workspace_exposes_every_documented_control_once() {
    let workspace = DeviceWorkspace::eventide_micropitch();
    let profile = mackes_profiles::eventide_micropitch_profile();
    let mut rendered = workspace.shared_controls.clone();
    rendered.extend(workspace.groups.iter().flat_map(|group| group.control_ids.clone()));
    let expected = profile.controls.iter().map(|control| control.label.clone()).collect::<Vec<_>>();
    assert_eq!(rendered.len(), expected.len());
    assert!(expected
        .iter()
        .all(|label| rendered.iter().filter(|item| *item == label).count() == 1));
    assert_eq!(workspace.diagram_notice(), None);
    let frame = workspace.frame_lines(Viewport::new(80, 24));
    assert!(frame.iter().any(|line| line.contains("Eventide MicroPitch")));
    assert!(frame.iter().any(|line| line.contains("Logical/control view")));
}

#[test]
fn compiled_reflex_algorithms_build_navigable_pages() {
    for algorithm in mackes_profiles::lexicon_reflex::algorithms() {
        let mut workspace =
            ReflexWorkspace::from_compiled_algorithm(algorithm.number).expect("compiled algorithm");
        assert_eq!(workspace.algorithm_label, algorithm.name);
        assert!(!workspace.controls.is_empty());
        let views =
            ReflexWorkspace::compiled_parameter_views(algorithm.number).expect("parameter views");
        assert_eq!(views.len(), workspace.controls.len());
        assert!(views
            .iter()
            .all(|view| { view.range.0 <= view.range.1 && view.effective_steps > 0 }));
        let node = format!("algorithm-{}", algorithm.number);
        assert_eq!(workspace.select_node(&node).expect("node").len(), workspace.controls.len());
        let frame = workspace.frame_lines(Viewport::new(80, 24));
        assert_eq!(frame[1], "Logical/control view");
        assert!(frame.iter().any(|line| line.contains(algorithm.name)));
    }
    assert_eq!(ReflexWorkspace::from_compiled_algorithm(0), Err("unknown Reflex algorithm"));
    assert_eq!(ReflexWorkspace::compiled_parameter_views(0), Err("unknown Reflex algorithm"));
}

#[test]
fn mapping_browser_is_stable_musical_and_compact() {
    let mapping = mackes_config::ControlMapping {
        id: "map-1".into(),
        controller_profile: "launch-control-xl-mk2".into(),
        physical_control_id: "knob-r1-c1".into(),
        source_endpoint: "controller".into(),
        source_kind: "cc".into(),
        source_channel: 0,
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
    };
    let activity = serde_json::json!({
        "mapping_id": "map-1", "source_value": 64, "destination_value": 32
    });
    let disabled = mackes_config::ControlMapping {
        id: "map-disabled".into(),
        physical_control_id: "knob-r1-c2".into(),
        enabled: false,
        ..mapping.clone()
    };
    let offline = mackes_config::ControlMapping {
        id: "map-offline".into(),
        physical_control_id: "knob-r1-c3".into(),
        destination_profile: "unknown.profile".into(),
        ..mapping.clone()
    };
    let mut browser =
        MappingBrowser::from_authoritative(&[mapping, disabled, offline], Some(&activity));
    assert_eq!(browser.rows[0].physical_label, "Top knob 1");
    assert_eq!(
        browser.rows[0].destination_path,
        "lexicon.reflex › algorithm-1 › reflex.parameter-1"
    );
    assert_eq!(browser.rows[0].status, MappingBrowserStatus::Enabled);
    assert_eq!(browser.rows[0].current_source_value, Some(64));
    assert_eq!(browser.rows[0].last_destination_result, Some(32));
    assert_eq!(browser.rows[1].status, MappingBrowserStatus::Disabled);
    assert_eq!(browser.rows[2].status, MappingBrowserStatus::Offline);
    assert_eq!(browser.select(0), Some("knob-r1-c1"));
    assert_eq!(browser.page(1).len(), 1);
    assert_eq!(browser.page(0).len(), 0);
    let lines = mapping_browser_lines(&browser, Viewport::new(80, 8));
    assert!(lines.iter().all(|line| line.chars().count() <= 80));
    assert!(lines[1].contains("Top knob 1") && lines[1].contains("lexicon.reflex"));
    assert!(lines.iter().any(|line| line.contains("| OFF |")));
    assert!(lines.iter().any(|line| line.contains("OFFLINE")));
    assert_eq!(mapping_browser_lines(&browser, Viewport::new(79, 8)).len(), 4);
}

#[test]
fn advanced_mapping_editor_exposes_profile_range_and_rejects_partial_edits() {
    let mapping = mackes_config::ControlMapping {
        id: "map-advanced".into(),
        controller_profile: "launch-control-xl-mk2".into(),
        physical_control_id: "knob-r1-c1".into(),
        source_endpoint: "controller".into(),
        source_kind: "cc".into(),
        source_channel: 0,
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
    };
    let mut editor = AdvancedMappingEditor::from_mapping(&mapping);
    assert_eq!(editor.button_mode, MappingButtonMode::ProfileDefault);
    assert_eq!(editor.behavior.destination_range, (0, 127));
    editor.set_source_range((8, 96)).expect("source range");
    editor.set_destination_range((12, 100)).expect("destination range");
    editor.set_invert(true).expect("invert");
    editor.set_curve("square").expect("curve");
    assert_eq!(editor.behavior.source_range, (8, 96));
    assert_eq!(editor.behavior.destination_range, (12, 100));
    assert!(editor.behavior.invert);
    assert_eq!(editor.behavior.curve, "square");
    let before = editor.behavior.clone();
    assert!(editor
        .set_behavior(mackes_config::MappingBehavior { source_range: (127, 0), ..before.clone() })
        .is_err());
    assert_eq!(editor.behavior, before);
    assert!(editor.error.is_some());
}

#[test]
fn mapping_outcomes_keep_recovery_action_inline_and_typed() {
    let result = mackes_ipc::MappingResult {
        generation: 4,
        undo_available: true,
        active: None,
        draft: None,
        outcome: mackes_ipc::MappingOutcome::Conflict,
    };
    let response = serde_json::to_string(&result).expect("mapping result");
    assert_eq!(
        mapping_response_notice(&response),
        "Occupied source/destination; choose Replace or Cancel"
    );
    assert_eq!(mapping_response_notice("{}"), "Mapping request failed; retry");
}

#[test]
fn task_sections_have_explicit_empty_and_populated_landings() {
    for section in AppSection::all() {
        assert!(!section_landing(section, false).is_empty());
        assert_eq!(section_landing(section, true), section.purpose());
    }
    assert!(section_landing(AppSection::MapControls, false).contains("Device"));
}

#[test]
fn every_task_section_has_actionable_contextual_help() {
    for section in AppSection::all() {
        let help = section_help(section);
        assert!(!help.is_empty());
        assert!(help.split_whitespace().count() >= 5);
    }
    assert!(section_help(AppSection::MapControls).contains("Advanced"));
    assert!(section_help(AppSection::Scenes).contains("setlists"));
    assert!(section_help(AppSection::Devices).contains("Reflex"));
    assert!(section_help(AppSection::System).contains("Legacy"));
}

#[test]
fn assignment_wizard_preserves_prior_section_and_debounces_capture() {
    let mut wizard = AssignmentWizard::new();
    let start = wizard.start(AppSection::Devices);
    assert_eq!(start.action, mackes_ipc::AssignmentAction::Start);
    assert_eq!(wizard.prior_section, AppSection::Devices);
    let capture = wizard.capture("knob-r1-c1").expect("first capture");
    assert_eq!(capture.physical_control_id.as_deref(), Some("knob-r1-c1"));
    assert!(wizard.capture("knob-r1-c1").is_none());
    assert!(wizard.capture("utility-1").is_none());
    assert!(wizard.capture("unknown-control").is_none());
    assert_eq!(AssignmentWizard::classify_capture(&[]), mackes_ipc::CandidateCapture::None);
    assert_eq!(
        AssignmentWizard::classify_capture(&["knob-r1-c1", "knob-r1-c1"]),
        mackes_ipc::CandidateCapture::Unique
    );
    assert_eq!(
        AssignmentWizard::classify_capture(&["knob-r1-c1", "fader-1"]),
        mackes_ipc::CandidateCapture::Ambiguous
    );
    wizard.reconcile(mackes_ipc::AssignmentResult {
        generation: 1,
        session: mackes_ipc::AssignmentSession {
            phase: mackes_ipc::AssignmentPhase::ChooseDevice,
            prior_screen: "Devices".into(),
            index: 0,
            total: 2,
            has_draft: true,
            interrupted_phase: None,
            cursors: mackes_ipc::AssignmentCursors::default(),
            catalog: mackes_ipc::AssignmentCatalog::default(),
        },
        applied: true,
        reason: None,
    });
    assert_eq!(wizard.generation, 1);
    assert_eq!(wizard.session.phase, mackes_ipc::AssignmentPhase::ChooseDevice);
    let lines = assignment_wizard_lines(&wizard, Viewport::new(32, 8));
    assert!(lines.iter().all(|line| line.chars().count() <= 32));
    assert!(lines.iter().any(|line| line.contains("CHOOSE DEVICE")));
    assert!(lines.iter().any(|line| line.contains("Position: 1 OF 2")));
}

#[test]
fn assignment_wizard_entry_is_available_from_every_task_section() {
    for section in AppSection::all() {
        let mut wizard = AssignmentWizard::new();
        let request = wizard.start(section);
        assert_eq!(request.action, mackes_ipc::AssignmentAction::Start);
        assert_eq!(wizard.prior_section, section);
        assert_eq!(wizard.session.prior_screen, "Live");
        wizard.reconcile(mackes_ipc::AssignmentResult {
            generation: 1,
            session: mackes_ipc::AssignmentSession {
                phase: mackes_ipc::AssignmentPhase::Idle,
                prior_screen: section.label().into(),
                index: 0,
                total: 0,
                has_draft: false,
                interrupted_phase: None,
                cursors: mackes_ipc::AssignmentCursors::default(),
                catalog: mackes_ipc::AssignmentCatalog::default(),
            },
            applied: true,
            reason: None,
        });
        assert_eq!(wizard.prior_section, section);
        assert_eq!(wizard.session.phase, mackes_ipc::AssignmentPhase::Idle);
    }
}

#[test]
fn assignment_wizard_builds_typed_destination_commit_payload() {
    let mut wizard = AssignmentWizard::new();
    wizard.start(AppSection::MapControls);
    wizard.capture("fader-1").expect("capture");
    wizard.generation = 9;
    let choice = AssignmentParameterChoice {
        profile_id: "lexicon.reflex".into(),
        id: "reflex.mix".into(),
        label: "Mix".into(),
        effect_id: "algorithm-1".into(),
        effect_label: "Algorithm 1".into(),
        reason: mackes_profiles::SupportReason::Compatible,
    };
    let request = wizard.destination_request(&choice);
    assert_eq!(request.generation, 9);
    assert_eq!(request.action, mackes_ipc::AssignmentAction::Commit);
    assert_eq!(request.physical_control_id.as_deref(), Some("fader-1"));
    assert_eq!(request.destination_profile.as_deref(), Some("lexicon.reflex"));
    assert_eq!(request.destination_effect.as_deref(), Some("algorithm-1"));
    assert_eq!(request.destination_parameter.as_deref(), Some("reflex.mix"));
    assert!(request.has_complete_destination());
    assert!(request.validate().is_ok());
}

#[test]
fn assignment_wizard_selects_reflex_preset_for_channel_button() {
    let mut wizard = AssignmentWizard::new();
    wizard.start(AppSection::Devices);
    wizard.capture("button-r1-c1").expect("capture");
    let choices = AssignmentChoiceBrowser::from_profiles(
        &["lexicon.reflex"],
        Some("lexicon.reflex"),
        mackes_profiles::SourceRole::ButtonAction,
    );
    let request = wizard.selected_destination_request(&choices).expect("preset request");
    assert_eq!(request.destination_effect.as_deref(), Some("reverb"));
    assert_eq!(request.destination_parameter.as_deref(), Some("pcm70_reflex:concert-wave"));
    assert!(request.validate().is_ok());
}

#[test]
fn assignment_frames_are_bounded_at_release_viewports() {
    let phases = [
        (mackes_ipc::AssignmentPhase::AwaitControl, "MOVE ONLY ONE CONTROL"),
        (mackes_ipc::AssignmentPhase::ChooseDevice, "CHOOSE DEVICE"),
        (mackes_ipc::AssignmentPhase::ChoosePreset, "CHOOSE PRESET"),
        (mackes_ipc::AssignmentPhase::ChooseEffect, "CHOOSE EFFECT"),
        (mackes_ipc::AssignmentPhase::ChooseType, "CHOOSE TYPE"),
        (mackes_ipc::AssignmentPhase::ChooseParameter, "CHOOSE PARAMETER"),
        (mackes_ipc::AssignmentPhase::ConfirmReplace, "REPLACE EXISTING MAPPING?"),
        (mackes_ipc::AssignmentPhase::Committing, "ASSIGNING"),
        (mackes_ipc::AssignmentPhase::Succeeded, "ASSIGNED"),
        (mackes_ipc::AssignmentPhase::Failed, "ASSIGNMENT FAILED"),
        (mackes_ipc::AssignmentPhase::Interrupted, "INTERRUPTED"),
    ];
    for (phase, marker) in phases {
        let wizard = AssignmentWizard {
            session: mackes_ipc::AssignmentSession {
                phase,
                prior_screen: "Devices".into(),
                index: 0,
                total: 1,
                has_draft: phase == mackes_ipc::AssignmentPhase::Interrupted,
                interrupted_phase: None,
                cursors: mackes_ipc::AssignmentCursors::default(),
                catalog: mackes_ipc::AssignmentCatalog::default(),
            },
            prior_section: AppSection::Devices,
            candidates: vec!["knob-r1-c1".into()],
            generation: 4,
        };
        for (width, height) in [(160, 37), (100, 37), (80, 24)] {
            let lines = assignment_wizard_lines(&wizard, Viewport::new(width, height));
            assert!(lines.iter().all(|line| line.chars().count() <= usize::from(width)));
            assert!(lines.len() <= usize::from(height));
            assert!(lines.iter().any(|line| line.contains(marker)));
        }
    }
}

#[test]
fn assignment_choices_are_profile_owned_filtered_and_bounded() {
    let mut choices = AssignmentChoiceBrowser::from_profiles(
        &["lexicon.reflex", "unknown-device", "eventide.micropitch"],
        Some("lexicon.reflex"),
        mackes_profiles::SourceRole::Continuous,
    );
    assert_eq!(choices.devices, vec!["lexicon.reflex", "eventide.micropitch"]);
    assert_eq!(choices.presets.len(), 5);
    assert!(choices.presets.iter().any(|(_, label)| label == "Concert Wave"));
    assert!(!choices.effects.is_empty());
    assert!(choices.parameters.iter().all(|choice| {
        matches!(
            choice.reason,
            mackes_profiles::SupportReason::Compatible
                | mackes_profiles::SupportReason::Experimental
        )
    }));
    assert!(choices
        .parameters
        .iter()
        .all(|choice| { choices.effects.iter().any(|(id, _)| id == &choice.effect_id) }));
    let eventide = AssignmentChoiceBrowser::from_profiles(
        &["eventide.micropitch"],
        Some("eventide.micropitch"),
        mackes_profiles::SourceRole::Continuous,
    );
    assert!(eventide.presets.is_empty());
    assert!(!eventide.parameters.is_empty());
    choices.selected = usize::MAX;
    assert!(choices.move_selection(true));
    assert!(!choices.move_selection(true));
}
