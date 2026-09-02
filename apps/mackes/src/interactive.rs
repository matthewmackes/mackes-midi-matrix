//! Interactive operator TUI session.
#![allow(clippy::redundant_pub_crate)]

use crate::cli::{
    daemon_request, dispatch_ui_command, endpoint_pair_for_new_route, first_output_endpoint,
    mapping_result_health, mapping_store_request, poll_learn_capture, remember_mapping_undo,
    save_learned_mapping, save_routes, save_setlists, synchronize_events, synchronize_snapshot,
};

#[allow(clippy::too_many_lines)]
pub(crate) fn run_tui() -> Result<(), String> {
    use crossterm::{
        event::{self, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{backend::CrosstermBackend, Terminal};
    use std::io::stdout;

    enable_raw_mode().map_err(|error| error.to_string())?;
    let mut output = stdout();
    execute!(
        output,
        EnterAlternateScreen,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )
    .map_err(|error| error.to_string())?;
    let backend = CrosstermBackend::new(output);
    let mut terminal = Terminal::new(backend).map_err(|error| error.to_string())?;
    let mut dashboard = mackes_tui::DashboardState::initial();
    if let Ok(config_path) = std::env::var("MACKES_CONFIG") {
        if let Ok(document) = mackes_config::load(std::path::Path::new(&config_path)) {
            dashboard.launch_control_template = document
                .settings
                .launch_control_template
                .as_ref()
                .and_then(mackes_tui::launch_control_template_from_config);
        }
    }
    let mut client_state = mackes_tui::ClientState::default();
    let mut learn_workspace = mackes_tui::LearnWorkspace::new();
    let reflex_workspace =
        mackes_tui::ReflexWorkspace::from_compiled_algorithm(1).map_err(str::to_owned)?;
    let mut eventide_workspace = mackes_tui::DeviceWorkspace::eventide_micropitch();
    let mut routing_editor = mackes_tui::RoutingEditor::from_bank(&mackes_tui::MappingBank::new());
    let mut mapping_undo: Vec<Vec<mackes_tui::MappingDraft>> = Vec::new();
    let mut diagnostics = mackes_tui::DiagnosticsState::default();
    let mut monitor = mackes_tui::MonitorState::new(128)
        .ok_or_else(|| "cannot initialize monitor: capacity rejected".to_owned())?;
    let backup_workspace = mackes_tui::BackupWorkspace::default();
    let mut setlist_editor = mackes_tui::SetlistEditor::from_snapshot(&[]);
    let mut task_shell = mackes_tui::TaskShellState::initial(5).map_err(str::to_owned)?;
    let mut mapping_replace_pending: Option<String> = None;
    let mut assignment_wizard = mackes_tui::AssignmentWizard::new();
    let mut assignment_choices =
        mackes_tui::AssignmentChoiceBrowser::from_session(&assignment_wizard.session);
    let mut workspace = 1_u8;
    let mut needs_snapshot = true;
    let mut route_generation = 0_u64;
    let result = (|| loop {
        dashboard.advance_live_activity_age(250_000_000);
        let synchronized = if needs_snapshot {
            synchronize_snapshot(
                &mut client_state,
                &mut dashboard,
                &mut routing_editor,
                &mut route_generation,
                &mut monitor,
                &mut diagnostics,
                &mut setlist_editor,
                &mut learn_workspace,
                &mut assignment_wizard,
            )
        } else {
            synchronize_events(
                &mut client_state,
                &mut dashboard,
                &mut routing_editor,
                &mut route_generation,
                &mut monitor,
                &mut diagnostics,
                &mut setlist_editor,
                &mut learn_workspace,
                &mut assignment_wizard,
                &mut assignment_choices,
                &mut task_shell,
            )
        };
        if let Ok(health) = synchronized {
            dashboard.health = health;
            needs_snapshot = false;
            if assignment_wizard.session.phase != mackes_ipc::AssignmentPhase::Idle {
                workspace = 2;
            }
        } else {
            "offline".clone_into(&mut dashboard.health);
            client_state.begin_reconnect();
            needs_snapshot = true;
        }
        if workspace == 2 && learn_workspace.phase == mackes_tui::LearnPhase::Capturing {
            poll_learn_capture(&mut learn_workspace);
        }
        dashboard.assignment_feedback =
            if assignment_wizard.session.phase == mackes_ipc::AssignmentPhase::Idle {
                Vec::new()
            } else {
                mackes_tui::assignment_catalog_lines(
                    &assignment_wizard,
                    &assignment_choices,
                    mackes_tui::Viewport::new(terminal.size().map_or(80, |size| size.width), 24),
                )
            };
        terminal
            .draw(|frame| {
                let content = match workspace {
                    2 => Some(
                        learn_workspace
                            .frame_lines(mackes_tui::Viewport::new(
                                frame.area().width,
                                frame.area().height,
                            ))
                            .join("\n"),
                    ),
                    3 => Some(
                        reflex_workspace
                            .frame_lines(mackes_tui::Viewport::new(
                                frame.area().width,
                                frame.area().height,
                            ))
                            .join("\n"),
                    ),
                    4 => Some(
                        eventide_workspace
                            .frame_lines(mackes_tui::Viewport::new(
                                frame.area().width,
                                frame.area().height,
                            ))
                            .join("\n"),
                    ),
                    5 => Some(
                        routing_editor
                            .frame_lines(mackes_tui::Viewport::new(
                                frame.area().width,
                                frame.area().height,
                            ))
                            .join("\n"),
                    ),
                    6 => Some(
                        diagnostics
                            .frame_lines(mackes_tui::Viewport::new(
                                frame.area().width,
                                frame.area().height,
                            ))
                            .join("\n"),
                    ),
                    7 => Some(
                        monitor
                            .frame_lines(mackes_tui::Viewport::new(
                                frame.area().width,
                                frame.area().height,
                            ))
                            .join("\n"),
                    ),
                    8 => Some(
                        backup_workspace
                            .frame_lines(mackes_tui::Viewport::new(
                                frame.area().width,
                                frame.area().height,
                            ))
                            .join("\n"),
                    ),
                    9 => Some(
                        setlist_editor
                            .frame_lines(mackes_tui::Viewport::new(
                                frame.area().width,
                                frame.area().height,
                            ))
                            .join("\n"),
                    ),
                    _ => None,
                };
                mackes_tui::draw_task_shell_with_content(
                    frame,
                    frame.area(),
                    &task_shell,
                    &dashboard,
                    content.as_deref(),
                );
            })
            .map_err(|error| error.to_string())?;
        if event::poll(std::time::Duration::from_millis(250)).map_err(|error| error.to_string())? {
            if let Event::Key(key) = event::read().map_err(|error| error.to_string())? {
                let shell_key = match key.code {
                    KeyCode::Up => Some(mackes_tui::ShellKey::Up),
                    KeyCode::Down => Some(mackes_tui::ShellKey::Down),
                    KeyCode::Left => Some(mackes_tui::ShellKey::Left),
                    KeyCode::Right => Some(mackes_tui::ShellKey::Right),
                    KeyCode::Enter => Some(mackes_tui::ShellKey::Enter),
                    KeyCode::Esc => Some(mackes_tui::ShellKey::Esc),
                    KeyCode::Char('?') => Some(mackes_tui::ShellKey::Help),
                    _ => None,
                };
                if let Some(shell_key) = shell_key {
                    if let Some(action) = mackes_tui::shell_action_for_key(shell_key) {
                        if assignment_wizard.session.phase != mackes_ipc::AssignmentPhase::Idle {
                            let assignment_action = match action {
                                mackes_tui::ShellAction::Up => {
                                    Some(mackes_ipc::AssignmentAction::Up)
                                }
                                mackes_tui::ShellAction::Down => {
                                    Some(mackes_ipc::AssignmentAction::Down)
                                }
                                mackes_tui::ShellAction::Enter | mackes_tui::ShellAction::Right => {
                                    Some(mackes_ipc::AssignmentAction::Enter)
                                }
                                mackes_tui::ShellAction::Left => {
                                    Some(mackes_ipc::AssignmentAction::Back)
                                }
                                mackes_tui::ShellAction::Back => {
                                    Some(mackes_ipc::AssignmentAction::Cancel)
                                }
                                mackes_tui::ShellAction::Help => None,
                            };
                            if let Some(assignment_action) = assignment_action {
                                let request = assignment_wizard.request(assignment_action, None);
                                if let Ok(payload) = serde_json::to_vec(&request) {
                                    let response =
                                        daemon_request(mackes_ipc::Command::Assignment, &payload);
                                    if let Ok(result) =
                                        serde_json::from_str::<mackes_ipc::AssignmentResult>(
                                            &response,
                                        )
                                    {
                                        assignment_wizard.reconcile(result);
                                        assignment_choices =
                                            mackes_tui::AssignmentChoiceBrowser::from_session(
                                                &assignment_wizard.session,
                                            );
                                    }
                                }
                            }
                            continue;
                        }
                        task_shell.apply(action);
                        if matches!(action, mackes_tui::ShellAction::Back) && workspace == 1 {
                            mapping_replace_pending = None;
                        }
                        workspace = match task_shell.focus.section {
                            mackes_tui::AppSection::Live => 1,
                            mackes_tui::AppSection::MapControls => 5,
                            mackes_tui::AppSection::Scenes => 9,
                            mackes_tui::AppSection::Devices => 4,
                            mackes_tui::AppSection::System => 6,
                        };
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') if workspace == 4 => {
                        let response = daemon_request(
                            mackes_ipc::Command::DeviceQuery,
                            br#"{"profile_id":"eventide.micropitch"}"#,
                        );
                        let populated = serde_json::from_str::<serde_json::Value>(&response)
                            .ok()
                            .and_then(|value| {
                                value
                                    .get("devices")
                                    .and_then(serde_json::Value::as_array)
                                    .map(Vec::len)
                                    .or_else(|| {
                                        value
                                            .get("profile")?
                                            .get("controls")?
                                            .as_array()
                                            .map(Vec::len)
                                    })
                            })
                            .is_some_and(|count| count > 0);
                        dashboard.health = if populated {
                            "device-query-complete".to_owned()
                        } else if response.contains("\"ok\":true") {
                            "device-query-empty".to_owned()
                        } else {
                            "device-query-failed".to_owned()
                        };
                    }
                    KeyCode::Char('W') if workspace == 4 => {
                        if let (Some(destination), Some((control, value))) =
                            (first_output_endpoint(), eventide_workspace.selected_control_request())
                        {
                            let payload = serde_json::json!({
                                "profile_id": "eventide.micropitch",
                                "control": control,
                                "channel": 1,
                                "value": value,
                                "destination": destination,
                                "confirm": true,
                            });
                            let Ok(encoded) = serde_json::to_vec(&payload) else {
                                "device-control-payload-failed".clone_into(&mut dashboard.health);
                                continue;
                            };
                            let response =
                                daemon_request(mackes_ipc::Command::DeviceControl, &encoded);
                            dashboard.health = if response.contains("\"ok\":true") {
                                "device-control-sent".to_owned()
                            } else {
                                "device-control-failed".to_owned()
                            };
                        } else {
                            "device-output-unavailable".clone_into(&mut dashboard.health);
                        }
                    }
                    KeyCode::Char('q') => break Ok(()),
                    KeyCode::Char(value) if ('1'..='9').contains(&value) => {
                        workspace = value
                            .to_digit(10)
                            .and_then(|number| u8::try_from(number).ok())
                            .unwrap_or(1);
                    }
                    KeyCode::Char('n' | 'p') if workspace != 9 => {
                        let response = dispatch_ui_command(if key.code == KeyCode::Char('n') {
                            mackes_tui::UiCommand::NextScene
                        } else {
                            mackes_tui::UiCommand::PreviousScene
                        });
                        dashboard.health = if response.contains("\"ok\":true") {
                            "online".to_owned()
                        } else {
                            "degraded".to_owned()
                        };
                    }
                    KeyCode::Char('!') => {
                        let response = dispatch_ui_command(mackes_tui::UiCommand::Panic);
                        dashboard.health = if response.contains("\"ok\":true") {
                            "panic-sent".to_owned()
                        } else {
                            "panic-failed".to_owned()
                        };
                    }
                    KeyCode::Char('d') if workspace != 5 && workspace != 9 => {
                        let request = if assignment_wizard.session.phase
                            == mackes_ipc::AssignmentPhase::Idle
                        {
                            assignment_wizard.start(task_shell.focus.section)
                        } else {
                            assignment_wizard.request(mackes_ipc::AssignmentAction::Enter, None)
                        };
                        if let Ok(payload) = serde_json::to_vec(&request) {
                            let response =
                                daemon_request(mackes_ipc::Command::Assignment, &payload);
                            if let Ok(result) =
                                serde_json::from_str::<mackes_ipc::AssignmentResult>(&response)
                            {
                                assignment_wizard.reconcile(result);
                                assignment_choices =
                                    mackes_tui::AssignmentChoiceBrowser::from_session(
                                        &assignment_wizard.session,
                                    );
                                dashboard.health = if assignment_wizard.session.phase
                                    == mackes_ipc::AssignmentPhase::AwaitControl
                                {
                                    "assignment-await-control"
                                } else {
                                    "assignment-progressed"
                                }
                                .into();
                            } else {
                                dashboard.health = "assignment-start-failed".into();
                            }
                        }
                    }
                    KeyCode::Char('e') if workspace == 1 => {
                        if let Some(index) = dashboard.mapping_browser.selected {
                            if let Some(row) = dashboard.mapping_browser.rows.get(index) {
                                let enabled =
                                    row.status != mackes_tui::MappingBrowserStatus::Enabled;
                                let response = mapping_store_request(
                                    dashboard.mapping_generation,
                                    "Enabled",
                                    serde_json::json!({"kind":"Enabled","mapping_id":row.id,"enabled":enabled}),
                                );
                                dashboard.health = mapping_result_health(&response);
                                needs_snapshot = true;
                            }
                        }
                    }
                    KeyCode::Char('r') if workspace == 1 => {
                        if let Some(index) = dashboard.mapping_browser.selected {
                            if let Some(row) = dashboard.mapping_browser.rows.get(index) {
                                if mapping_replace_pending.as_deref() == Some(row.id.as_str()) {
                                    let response = mapping_store_request(
                                        dashboard.mapping_generation,
                                        "Replace",
                                        serde_json::json!({
                                            "kind": "Mapping",
                                            "mapping": row.mapping,
                                        }),
                                    );
                                    dashboard.health = mapping_result_health(&response);
                                    mapping_replace_pending = None;
                                    needs_snapshot = true;
                                } else {
                                    mapping_replace_pending = Some(row.id.clone());
                                    dashboard.health =
                                        "mapping-replace-confirm-r-again-esc-cancel".into();
                                }
                            }
                        }
                    }
                    KeyCode::Char('x') if workspace == 1 => {
                        if let Some(index) = dashboard.mapping_browser.selected {
                            if let Some(row) = dashboard.mapping_browser.rows.get(index) {
                                let response = mapping_store_request(
                                    dashboard.mapping_generation,
                                    "Delete",
                                    serde_json::json!({"kind":"Delete","mapping_id":row.id}),
                                );
                                dashboard.health = mapping_result_health(&response);
                                needs_snapshot = true;
                            }
                        }
                    }
                    KeyCode::Char('u') if workspace == 1 => {
                        let response = mapping_store_request(
                            dashboard.mapping_generation,
                            "Undo",
                            serde_json::Value::Null,
                        );
                        dashboard.health = mapping_result_health(&response);
                        needs_snapshot = true;
                    }
                    KeyCode::Char('c') if workspace == 1 => {
                        if let Some(index) = dashboard.mapping_browser.selected {
                            if let Some(row) = dashboard.mapping_browser.rows.get(index) {
                                let mut behavior = row.mapping.behavior.clone();
                                behavior.curve = match behavior.curve.as_str() {
                                    "linear" => "square",
                                    "square" => "square_root",
                                    _ => "linear",
                                }
                                .into();
                                let response = mapping_store_request(
                                    dashboard.mapping_generation,
                                    "Behavior",
                                    serde_json::json!({
                                        "kind": "Behavior",
                                        "mapping_id": row.id,
                                        "behavior": behavior,
                                    }),
                                );
                                dashboard.health = mapping_result_health(&response);
                                needs_snapshot = true;
                            }
                        }
                    }
                    KeyCode::Char('i') if workspace == 1 => {
                        if let Some(index) = dashboard.mapping_browser.selected {
                            if let Some(row) = dashboard.mapping_browser.rows.get(index) {
                                let mut behavior = row.mapping.behavior.clone();
                                behavior.invert = !behavior.invert;
                                let response = mapping_store_request(
                                    dashboard.mapping_generation,
                                    "Behavior",
                                    serde_json::json!({"kind":"Behavior","mapping_id":row.id,"behavior":behavior}),
                                );
                                dashboard.health = mapping_result_health(&response);
                                needs_snapshot = true;
                            }
                        }
                    }
                    KeyCode::Char('[' | ']') if workspace == 1 => {
                        if let Some(index) = dashboard.mapping_browser.selected {
                            if let Some(row) = dashboard.mapping_browser.rows.get(index) {
                                let mut behavior = row.mapping.behavior.clone();
                                let upper = behavior.destination_range.1;
                                behavior.destination_range.1 = if key.code == KeyCode::Char('[') {
                                    upper.saturating_sub(1).max(behavior.destination_range.0)
                                } else {
                                    upper.saturating_add(1).min(16_383)
                                };
                                let response = mapping_store_request(
                                    dashboard.mapping_generation,
                                    "Behavior",
                                    serde_json::json!({"kind":"Behavior","mapping_id":row.id,"behavior":behavior}),
                                );
                                dashboard.health = mapping_result_health(&response);
                                needs_snapshot = true;
                            }
                        }
                    }
                    KeyCode::Char('s') if workspace == 5 || workspace == 1 => {
                        let response = save_routes(&routing_editor, route_generation);
                        dashboard.health = if response.contains("\"ok\":true") {
                            route_generation = route_generation.saturating_add(1);
                            dashboard.mapping_dirty = false;
                            needs_snapshot = true;
                            "routes-saved".to_owned()
                        } else {
                            "routes-save-failed".to_owned()
                        };
                    }
                    KeyCode::Char('u') if (workspace == 5 || workspace == 1) => {
                        let request =
                            format!(r#"{{"action":"undo","route_generation":{route_generation}}}"#);
                        let response =
                            daemon_request(mackes_ipc::Command::Routes, request.as_bytes());
                        dashboard.health = if response.contains("\"ok\":true") {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&response)
                            {
                                if let Some(generation) = value
                                    .get("route_generation")
                                    .and_then(serde_json::Value::as_u64)
                                {
                                    route_generation = generation;
                                }
                            }
                            dashboard.mapping_dirty = false;
                            routing_editor.selected = None;
                            needs_snapshot = true;
                            "mapping-undo".to_owned()
                        } else if mapping_undo.pop().is_some() {
                            "mapping-undo-local-only".to_owned()
                        } else {
                            "mapping-undo-empty".to_owned()
                        };
                    }
                    KeyCode::Char('D') if workspace == 1 => {
                        dashboard.cycle_destination();
                    }
                    KeyCode::Char('I') if workspace == 1 => {
                        dashboard.cycle_input();
                    }
                    KeyCode::Char('P') if workspace == 1 => {
                        dashboard.cycle_parameter();
                    }
                    KeyCode::Char('s') if workspace == 9 => {
                        let response = save_setlists(&setlist_editor);
                        dashboard.health = if response.contains("\"ok\":true") {
                            "setlists-saved".to_owned()
                        } else {
                            "setlists-save-failed".to_owned()
                        };
                    }
                    KeyCode::Char('l') if workspace == 2 => learn_workspace.start_capture(),
                    KeyCode::Enter
                        if workspace == 2
                            && learn_workspace.phase == mackes_tui::LearnPhase::Capturing =>
                    {
                        poll_learn_capture(&mut learn_workspace);
                    }
                    KeyCode::Char('j')
                        if workspace == 2
                            && learn_workspace.phase == mackes_tui::LearnPhase::Review
                            && !learn_workspace.candidates.is_empty() =>
                    {
                        learn_workspace.selected =
                            Some(learn_workspace.selected.map_or(0, |index| {
                                (index + 1).min(learn_workspace.candidates.len() - 1)
                            }));
                    }
                    KeyCode::Char('k')
                        if workspace == 2
                            && learn_workspace.phase == mackes_tui::LearnPhase::Review
                            && !learn_workspace.candidates.is_empty() =>
                    {
                        learn_workspace.selected =
                            Some(learn_workspace.selected.unwrap_or(0).saturating_sub(1));
                    }
                    KeyCode::Enter
                        if workspace == 2
                            && learn_workspace.phase == mackes_tui::LearnPhase::Review =>
                    {
                        if let Some(index) = learn_workspace.selected {
                            learn_workspace.select(index);
                        }
                    }
                    KeyCode::Char('r')
                        if workspace == 2
                            && learn_workspace.phase == mackes_tui::LearnPhase::Destination =>
                    {
                        if let Some(index) = learn_workspace.selected {
                            if let Some(candidate) = learn_workspace.candidates.get(index) {
                                let mode = match candidate.kind {
                                    mackes_midi_engine::LearnMessageKind::ControlChange => {
                                        mackes_tui::MappingMode::Cc
                                    }
                                    mackes_midi_engine::LearnMessageKind::ProgramChange => {
                                        mackes_tui::MappingMode::ProgramChange
                                    }
                                    mackes_midi_engine::LearnMessageKind::NoteOn
                                    | mackes_midi_engine::LearnMessageKind::NoteOff
                                    | mackes_midi_engine::LearnMessageKind::PolyPressure => {
                                        mackes_tui::MappingMode::Note
                                    }
                                    mackes_midi_engine::LearnMessageKind::PitchBend => {
                                        mackes_tui::MappingMode::PitchBend
                                    }
                                    mackes_midi_engine::LearnMessageKind::SysEx => {
                                        mackes_tui::MappingMode::Sysex
                                    }
                                    _ => continue,
                                };
                                let _ = learn_workspace.set_destination("route.default", mode);
                            }
                        }
                    }
                    KeyCode::Char('t')
                        if workspace == 2
                            && learn_workspace.phase == mackes_tui::LearnPhase::Destination =>
                    {
                        learn_workspace.begin_live_test();
                        if learn_workspace.phase == mackes_tui::LearnPhase::Testing {
                            let Some(source_endpoint_id) =
                                learn_workspace.learn_endpoint_id.clone()
                            else {
                                learn_workspace.finish_live_test(false);
                                "learn-live-test-unavailable".clone_into(&mut dashboard.health);
                                continue;
                            };
                            let Some((destination_id, _)) = learn_workspace.destination.clone()
                            else {
                                learn_workspace.finish_live_test(false);
                                "learn-live-test-unavailable".clone_into(&mut dashboard.health);
                                continue;
                            };
                            let Some(candidate) = learn_workspace
                                .selected
                                .and_then(|index| learn_workspace.candidates.get(index))
                            else {
                                learn_workspace.finish_live_test(false);
                                "learn-live-test-unavailable".clone_into(&mut dashboard.health);
                                continue;
                            };
                            let payload = serde_json::json!({
                                "action": "live_test",
                                "request_id": format!("learn-{}", dashboard.route_generation),
                                "source_endpoint_id": source_endpoint_id,
                                "destination_id": destination_id,
                                "candidate_kind": format!("{:?}", candidate.kind).to_lowercase(),
                                "candidate_number": candidate.number,
                                "candidate_channel": candidate.channel,
                                "generation": dashboard.route_generation,
                            });
                            let response = serde_json::to_vec(&payload).ok().map(|payload| {
                                daemon_request(mackes_ipc::Command::Learn, &payload)
                            });
                            let status = response
                                .as_deref()
                                .and_then(|response| {
                                    serde_json::from_str::<serde_json::Value>(response).ok()
                                })
                                .and_then(|value| {
                                    value
                                        .get("live_test")?
                                        .get("status")?
                                        .as_str()
                                        .map(str::to_owned)
                                })
                                .unwrap_or_else(|| "unavailable".to_owned());
                            learn_workspace.finish_live_test(status == "passed");
                            dashboard.health = format!("learn-live-test-{status}");
                        }
                    }
                    // A live test may only be completed by an explicit daemon/device result.
                    // Enter cannot stand in for hardware acknowledgment; Esc cancels the
                    // pending test until the transport contract is available.
                    KeyCode::Enter
                        if workspace == 2
                            && learn_workspace.phase == mackes_tui::LearnPhase::Destination =>
                    {
                        learn_workspace.commit();
                        if let Some(mapping) = learn_workspace.committed_learned_mapping() {
                            let response = save_learned_mapping(&mapping);
                            dashboard.health = if response.contains("\"ok\":true") {
                                "learn-saved".to_owned()
                            } else {
                                "learn-save-failed".to_owned()
                            };
                        }
                    }
                    KeyCode::Esc if workspace == 2 => learn_workspace.cancel(),
                    KeyCode::Char('d') if workspace == 5 => {
                        if let Some(index) = routing_editor.selected {
                            remember_mapping_undo(&mut mapping_undo, &routing_editor);
                            let _ = routing_editor.remove(index);
                            dashboard.mapping_dirty = true;
                        }
                    }
                    KeyCode::Char('a') if workspace == 5 || workspace == 1 => {
                        let pair = if workspace == 1 {
                            dashboard
                                .selected_input_endpoint()
                                .or_else(|| dashboard.first_input_endpoint())
                                .zip(dashboard.selected_destination_endpoint())
                        } else {
                            endpoint_pair_for_new_route()
                        };
                        match pair {
                            Some((source, destination)) => {
                                let draft = mackes_tui::MappingDraft {
                                    source: source.to_string(),
                                    destination: destination.to_string(),
                                    destination_parameter: dashboard.selected_parameter_label(),
                                    channel: None,
                                    enabled: true,
                                    mode: mackes_tui::MappingMode::Cc,
                                    priority: u16::try_from(routing_editor.drafts.len())
                                        .unwrap_or(u16::MAX),
                                    curve: mackes_midi_engine::Curve::Linear,
                                    filters: mackes_tui::MappingFilterDraft::default(),
                                    allow_cycle: false,
                                };
                                remember_mapping_undo(&mut mapping_undo, &routing_editor);
                                if routing_editor.add(draft).is_err() {
                                    "route-add-rejected".clone_into(&mut dashboard.health);
                                } else {
                                    dashboard.mapping_dirty = true;
                                }
                            }
                            None => "route-endpoints-unavailable".clone_into(&mut dashboard.health),
                        }
                    }
                    KeyCode::Char('m') if workspace == 5 => {
                        remember_mapping_undo(&mut mapping_undo, &routing_editor);
                        if routing_editor.cycle_selected_mode().is_err() {
                            "route-edit-rejected".clone_into(&mut dashboard.health);
                        } else {
                            dashboard.mapping_dirty = true;
                        }
                    }
                    KeyCode::Char('c') if workspace == 5 => {
                        remember_mapping_undo(&mut mapping_undo, &routing_editor);
                        if routing_editor.cycle_selected_channel().is_err() {
                            "route-channel-edit-rejected".clone_into(&mut dashboard.health);
                        } else {
                            dashboard.mapping_dirty = true;
                        }
                    }
                    KeyCode::Char('r') if workspace == 5 => {
                        remember_mapping_undo(&mut mapping_undo, &routing_editor);
                        if routing_editor.cycle_selected_curve().is_err() {
                            "route-curve-edit-rejected".clone_into(&mut dashboard.health);
                        } else {
                            dashboard.mapping_dirty = true;
                        }
                    }
                    KeyCode::Char('f') if workspace == 5 => {
                        remember_mapping_undo(&mut mapping_undo, &routing_editor);
                        if routing_editor.cycle_selected_filter().is_err() {
                            "route-filter-edit-rejected".clone_into(&mut dashboard.health);
                        } else {
                            dashboard.mapping_dirty = true;
                        }
                    }
                    KeyCode::Char('e') if workspace == 5 => {
                        remember_mapping_undo(&mut mapping_undo, &routing_editor);
                        if routing_editor.toggle_selected_enabled().is_err() {
                            "route-enable-edit-rejected".clone_into(&mut dashboard.health);
                        } else {
                            dashboard.mapping_dirty = true;
                        }
                    }
                    KeyCode::Char('y') if workspace == 5 => {
                        remember_mapping_undo(&mut mapping_undo, &routing_editor);
                        if routing_editor.toggle_selected_cycle().is_err() {
                            "route-cycle-edit-rejected".clone_into(&mut dashboard.health);
                        } else {
                            dashboard.mapping_dirty = true;
                        }
                    }
                    KeyCode::Char('+') if workspace == 5 => {
                        remember_mapping_undo(&mut mapping_undo, &routing_editor);
                        if routing_editor.adjust_selected_priority(1).is_err() {
                            "route-priority-edit-rejected".clone_into(&mut dashboard.health);
                        } else {
                            dashboard.mapping_dirty = true;
                        }
                    }
                    KeyCode::Char('-') if workspace == 5 => {
                        remember_mapping_undo(&mut mapping_undo, &routing_editor);
                        if routing_editor.adjust_selected_priority(-1).is_err() {
                            "route-priority-edit-rejected".clone_into(&mut dashboard.health);
                        } else {
                            dashboard.mapping_dirty = true;
                        }
                    }
                    KeyCode::Char('j') if workspace == 4 => eventide_workspace.move_control(1),
                    KeyCode::Char('k') if workspace == 4 => eventide_workspace.move_control(-1),
                    KeyCode::Char('+') if workspace == 4 => {
                        eventide_workspace.adjust_control_value(1);
                    }
                    KeyCode::Char('-') if workspace == 4 => {
                        eventide_workspace.adjust_control_value(-1);
                    }
                    KeyCode::Char('j') if workspace == 1 => {
                        if !dashboard.mapping_browser.rows.is_empty() {
                            let next = dashboard.mapping_browser.selected.map_or(0, |index| {
                                (index + 1).min(dashboard.mapping_browser.rows.len() - 1)
                            });
                            dashboard.mapping_browser.selected = Some(next);
                        }
                    }
                    KeyCode::Char('k') if workspace == 1 => {
                        if !dashboard.mapping_browser.rows.is_empty() {
                            dashboard.mapping_browser.selected = Some(
                                dashboard.mapping_browser.selected.unwrap_or(0).saturating_sub(1),
                            );
                        }
                    }
                    KeyCode::Char('j') if workspace == 5 => {
                        if !routing_editor.drafts.is_empty() {
                            let next = routing_editor.selected.map_or(0, |index| {
                                (index + 1).min(routing_editor.drafts.len() - 1)
                            });
                            routing_editor.selected = Some(next);
                        }
                    }
                    KeyCode::Char('k') if workspace == 5 && !routing_editor.drafts.is_empty() => {
                        let next = routing_editor.selected.unwrap_or(0).saturating_sub(1);
                        routing_editor.selected = Some(next);
                    }
                    KeyCode::Char('j') if workspace == 9 && !setlist_editor.drafts.is_empty() => {
                        let next = setlist_editor
                            .selected
                            .map_or(0, |index| (index + 1).min(setlist_editor.drafts.len() - 1));
                        setlist_editor.selected = Some(next);
                    }
                    KeyCode::Char('a') if workspace == 9 => {
                        setlist_editor.add_empty();
                    }
                    KeyCode::Char('p') if workspace == 9 => {
                        if let Some(index) = setlist_editor.selected {
                            let existing = &setlist_editor.drafts[index].projects;
                            if let Some(project) = setlist_editor
                                .available_projects
                                .iter()
                                .find(|project| !existing.iter().any(|value| value == *project))
                                .cloned()
                            {
                                let _ = setlist_editor.append_project(project);
                            } else {
                                "no-unused-project".clone_into(&mut dashboard.health);
                            }
                        }
                    }
                    KeyCode::Char('x') if workspace == 9 => {
                        if setlist_editor.remove_last_project().is_err() {
                            "setlist-project-remove-rejected".clone_into(&mut dashboard.health);
                        }
                    }
                    KeyCode::Char('[') if workspace == 9 => {
                        if setlist_editor.move_last_project(false).is_err() {
                            "setlist-project-reorder-rejected".clone_into(&mut dashboard.health);
                        }
                    }
                    KeyCode::Char(']') if workspace == 9 => {
                        if setlist_editor.move_last_project(true).is_err() {
                            "setlist-project-reorder-rejected".clone_into(&mut dashboard.health);
                        }
                    }
                    KeyCode::Char('k') if workspace == 9 && !setlist_editor.drafts.is_empty() => {
                        let next = setlist_editor.selected.unwrap_or(0).saturating_sub(1);
                        setlist_editor.selected = Some(next);
                    }
                    KeyCode::Char('<') if workspace == 9 => {
                        if let Some(index) = setlist_editor.selected {
                            if index > 0 {
                                setlist_editor.drafts.swap(index, index - 1);
                                setlist_editor.selected = Some(index - 1);
                            }
                        }
                    }
                    KeyCode::Char('>') if workspace == 9 => {
                        if let Some(index) = setlist_editor.selected {
                            if index + 1 < setlist_editor.drafts.len() {
                                setlist_editor.drafts.swap(index, index + 1);
                                setlist_editor.selected = Some(index + 1);
                            }
                        }
                    }
                    KeyCode::Char('c') if workspace == 9 => {
                        if let Some(index) = setlist_editor.selected {
                            let source = setlist_editor.drafts[index].id.clone();
                            let mut suffix = 1_u32;
                            loop {
                                let candidate = format!("{source}-copy-{suffix}");
                                if setlist_editor
                                    .drafts
                                    .iter()
                                    .all(|setlist| setlist.id != candidate)
                                    && setlist_editor.copy_selected(&candidate).is_ok()
                                {
                                    break;
                                }
                                suffix = suffix.saturating_add(1);
                            }
                        }
                    }
                    KeyCode::Char('d') if workspace == 9 => {
                        if let Some(index) = setlist_editor.selected {
                            setlist_editor.drafts.remove(index);
                            setlist_editor.selected = if setlist_editor.drafts.is_empty() {
                                None
                            } else {
                                Some(index.min(setlist_editor.drafts.len() - 1))
                            };
                        }
                    }
                    _ => {}
                }
            }
        }
    })();
    let cleanup_result = disable_raw_mode().map_err(|error| error.to_string());
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(|error| error.to_string())?;
    terminal.show_cursor().map_err(|error| error.to_string())?;
    cleanup_result?;
    result
}
