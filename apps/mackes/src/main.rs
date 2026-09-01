//! MACKES operator entry point.

#[allow(clippy::too_many_lines)]
fn run_tui() -> Result<(), String> {
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
    let mut assignment_choices = mackes_tui::AssignmentChoiceBrowser::from_profiles(
        &["lexicon.reflex", "eventide.micropitch"],
        Some("lexicon.reflex"),
        mackes_profiles::SourceRole::Continuous,
    );
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
                let mut lines = mackes_tui::assignment_wizard_lines(
                    &assignment_wizard,
                    mackes_tui::Viewport::new(terminal.size().map_or(80, |size| size.width), 24),
                );
                if !assignment_choices.parameters.is_empty() {
                    lines.push("CATALOG  Device > Preset > Effect > Type > Parameter".into());
                    lines.push(format!("DEVICE   {}", assignment_choices.devices.join(", ")));
                    if !assignment_choices.presets.is_empty() {
                        lines.push(format!(
                            "PRESETS  {}",
                            assignment_choices
                                .presets
                                .iter()
                                .map(|(_, label)| label.as_str())
                                .collect::<Vec<_>>()
                                .join(" | ")
                        ));
                    }
                    lines.push(format!(
                        "EFFECTS  {}",
                        assignment_choices
                            .effects
                            .iter()
                            .map(|(_, label)| label.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                    lines.push(format!("TYPES    {}", assignment_choices.types.join(", ")));
                    lines.push("DESTINATION / EFFECT / PARAMETER".into());
                    for (index, choice) in assignment_choices.parameters.iter().enumerate() {
                        lines.push(format!(
                            "{} {} / {} / {}",
                            if index == assignment_choices.selected { ">" } else { " " },
                            choice.profile_id,
                            choice.effect_id,
                            choice.id
                        ));
                    }
                }
                lines
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
                                match assignment_action {
                                    mackes_ipc::AssignmentAction::Up => {
                                        assignment_choices.move_selection(false);
                                    }
                                    mackes_ipc::AssignmentAction::Down => {
                                        assignment_choices.move_selection(true);
                                    }
                                    _ => {}
                                }
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
                            assignment_choices.selected = 0;
                            assignment_wizard.start(task_shell.focus.section)
                        } else if assignment_wizard.session.phase
                            == mackes_ipc::AssignmentPhase::ChooseParameter
                        {
                            assignment_wizard
                                .selected_destination_request(&assignment_choices)
                                .unwrap_or_else(|| {
                                    assignment_wizard
                                        .request(mackes_ipc::AssignmentAction::Enter, None)
                                })
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

fn mvave_ir_box_preset(preset: u8, dry_run: bool) -> Result<String, String> {
    let bytes = mackes_profiles::mvave_ir_box_preset_sysex(preset).map_err(str::to_owned)?;
    let hex = bytes.iter().map(|byte| format!("{byte:02X}")).collect::<Vec<_>>().join(" ");
    if dry_run {
        return Ok(hex);
    }
    let endpoint = mackes_midi_engine::enumerate_midir_ports()?
        .into_iter()
        .find(|endpoint| {
            endpoint.direction == mackes_midi_engine::EndpointDirection::Output
                && endpoint.name.contains("SINCO MIDI 1")
        })
        .ok_or("M-VAVE IR Box/SINCO MIDI output is unavailable")?;
    let mut output = mackes_midi_engine::MidirOutputAdapter::open_id(&endpoint.id)?;
    let message = mackes_domain::MidiMessage::from_wire(&bytes).map_err(str::to_owned)?;
    let event = mackes_domain::MidiEvent {
        timestamp: mackes_domain::TimestampNanos::new(0),
        sequence: 0,
        endpoint: mackes_domain::EndpointId::new(1).expect("nonzero endpoint"),
        message,
    };
    output.send_checked(&event)?;
    Ok(format!("preset {preset} sent to {}", endpoint.name))
}

fn reflex_pcm70_preset(
    preset_id: &str,
    destination: Option<&str>,
    dry_run: bool,
) -> Result<String, String> {
    let preset = mackes_profiles::lexicon_reflex::pcm70_translations()
        .iter()
        .find(|preset| preset.id.eq_ignore_ascii_case(preset_id))
        .ok_or_else(|| format!("unknown PCM70 preset translation: {preset_id}"))?;
    let bytes = mackes_profiles::lexicon_reflex::encode_pcm70_translation(preset.id, 0)
        .map_err(str::to_owned)?;
    let hex = bytes.iter().map(|byte| format!("{byte:02X}")).collect::<Vec<_>>().join(" ");
    if dry_run {
        return Ok(format!("{} [{}] — {}\n{hex}", preset.name, preset.source_program, preset.note));
    }
    let destination = destination.ok_or("Reflex destination is required")?;
    let payload = serde_json::json!({
        "profile_id": "lexicon.reflex",
        "control": preset.name,
        "channel": 1,
        "value": 1,
        "destination": destination,
        "confirm": true,
    });
    let response = daemon_request(
        mackes_ipc::Command::DeviceControl,
        &serde_json::to_vec(&payload).map_err(|error| error.to_string())?,
    );
    if response.contains("\"ok\":true") {
        Ok(format!("{} sent to {destination}: {response}", preset.name))
    } else {
        Err(format!("Reflex preset send failed: {response}"))
    }
}

fn mvave_ir_box_module(module: &str, enabled: bool) -> Result<String, String> {
    let selector = match module {
        "ir" => mackes_profiles::MvaveIrBoxModule::Ir,
        "eq" => mackes_profiles::MvaveIrBoxModule::Eq,
        _ => return Err("IR Box module must be ir or eq".into()),
    };
    let bytes = mackes_profiles::mvave_ir_box_module_sysex(selector, enabled);
    let endpoint = mackes_midi_engine::enumerate_midir_ports()?
        .into_iter()
        .find(|endpoint| {
            endpoint.direction == mackes_midi_engine::EndpointDirection::Output
                && endpoint.name.contains("SINCO MIDI 1")
        })
        .ok_or("M-VAVE IR Box/SINCO MIDI output is unavailable")?;
    let mut output = mackes_midi_engine::MidirOutputAdapter::open_id(&endpoint.id)?;
    let message = mackes_domain::MidiMessage::from_wire(&bytes).map_err(str::to_owned)?;
    output.send_checked(&mackes_domain::MidiEvent {
        timestamp: mackes_domain::TimestampNanos::new(0),
        sequence: 0,
        endpoint: mackes_domain::EndpointId::new(1).expect("nonzero endpoint"),
        message,
    })?;
    Ok(format!(
        "{module} {} sent-unverified to {}",
        if enabled { "on" } else { "off" },
        endpoint.name
    ))
}

fn set_default_provider_cli(path: &str, capability: &str, profile_id: &str) -> Result<(), String> {
    let profile = mackes_profiles::builtin_profile(profile_id)
        .ok_or_else(|| format!("unknown built-in profile '{profile_id}'"))?;
    if !profile.provides_capability(capability) {
        return Err(format!("profile '{profile_id}' does not provide '{capability}'"));
    }
    let path = std::path::Path::new(path);
    let document = mackes_config::load(path).map_err(|error| error.to_string())?;
    let updated = mackes_config::set_default_provider(&document, capability, profile_id)?;
    mackes_config::save(path, &updated, 10).map_err(|error| error.to_string())
}

fn print_default_provider(path: &str, capability: &str, json: bool) -> Result<(), String> {
    let document =
        mackes_config::load(std::path::Path::new(path)).map_err(|error| error.to_string())?;
    let configured = mackes_config::default_provider(&document, capability);
    let effective = configured.map(str::to_owned).or_else(|| {
        mackes_profiles::default_capability_provider(capability).map(|profile| profile.id)
    });
    if json {
        println!(
            "{}",
            serde_json::json!({
                "capability": capability.trim().to_ascii_lowercase(),
                "profile_id": effective,
                "configured": configured.is_some(),
            })
        );
    } else if let Some(profile_id) = effective {
        println!(
            "{}: {}{}",
            capability.trim(),
            profile_id,
            if configured.is_some() { " (configured)" } else { " (catalog default)" }
        );
    } else {
        return Err(format!("no provider is available for '{capability}'"));
    }
    Ok(())
}

fn print_effects_faceplate(json: bool) {
    let faceplate = mackes_profiles::launch_control_effects_faceplate();
    if json {
        println!("{}", serde_json::to_string(&faceplate).expect("faceplate is serializable"));
        return;
    }
    println!("Effects faceplate: 6 groups; faders F01-F08; unused C37-C40");
    for group in faceplate.groups {
        println!(
            "{} {} [{}] enable #{} type #{}",
            group.row, group.label, group.owner, group.enable_index, group.type_index
        );
    }
}

fn print_effects_demo(json: bool) {
    let frames = mackes_profiles::effects_demo_frames();
    if json {
        println!("{}", serde_json::to_string(&frames).expect("demo frames are serializable"));
        return;
    }
    for frame in frames {
        println!(
            "frame {}: groups={} faders={:?}{}",
            frame.sequence,
            frame.groups.len(),
            frame.faders,
            if frame.resync { " RESYNC" } else { "" }
        );
    }
}

fn print_effects_assignments(profile_id: &str, json: bool) {
    let Some(profile) = mackes_profiles::builtin_profile(profile_id) else {
        eprintln!("unknown profile: {profile_id}");
        std::process::exit(2);
    };
    let assignments = mackes_profiles::effects_parameter_assignments(
        &mackes_profiles::launch_control_effects_faceplate(),
        &profile,
    );
    if json {
        println!("{}", serde_json::json!({"profile_id": profile.id, "assignments": assignments}));
    } else {
        println!("profile owner: {}", profile.id);
        for assignment in assignments {
            println!(
                "{} => {} range={:?} unit={}",
                assignment.parameter_id,
                assignment
                    .control_index
                    .map_or_else(|| "UNASSIGNED".into(), |index| format!("C{index:02}")),
                assignment.range,
                assignment.unit
            );
        }
    }
}

fn print_effects_plan(groups: &[String], json: bool) {
    let plan = mackes_profiles::plan_effects_automation(
        &mackes_profiles::launch_control_effects_faceplate(),
        groups,
        false,
    );
    if json {
        println!("{}", serde_json::to_string(&plan).expect("plan is serializable"));
    } else {
        for operation in plan {
            println!(
                "{} [{}] {}",
                operation.group_id,
                operation.owner,
                operation.reason.unwrap_or_else(|| "supported".into())
            );
        }
    }
}

#[allow(clippy::too_many_lines)]
fn main() {
    use owo_colors::OwoColorize;

    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.as_slice() {
        [] => {
            if let Err(error) = run_tui() {
                eprintln!("tui failed: {error}");
                std::process::exit(2);
            }
        }
        [command] if command == "tui" => {
            if let Err(error) = run_tui() {
                eprintln!("tui failed: {error}");
                std::process::exit(2);
            }
        }
        [command] if command == "--version" || command == "version" => {
            println!("mackes-midi-matrix {}", env!("CARGO_PKG_VERSION"));
        }
        [command] if command == "--help" || command == "help" => {
            println!(
                "mackes-midi-matrix: TUI/CLI\n\nUsage:\n  mackes-midi-matrix tui\n  mackes-midi-matrix validate <path> [--json]\n  mackes-midi-matrix export <config> <directory>\n  mackes-midi-matrix doctor [--json]\n  mackes-midi-matrix status [--json]\n  mackes-midi-matrix panic\n  mackes-midi-matrix endpoints [--json]\n  mackes-midi-matrix default get <config> <capability> [--json]\n  mackes-midi-matrix default set <config> <capability> <profile-id>\n  mackes-midi-matrix reflex preset <id> --dry-run\n  mackes-midi-matrix reflex preset <id> <destination-id> --confirm\n  mackes-midi-matrix mvave preset <1-32> [--dry-run]\n  mackes-midi-matrix mvave ir|eq on|off --confirm-unverified\n  mackes-midi-matrix scenes|devices|routes|monitor [--json]\n  mackes-midi-matrix scene list <config> [--json]\n  mackes-midi-matrix backup list|inspect ...\n  mackes-midi-matrix profile validate [--json]\n  mackes-midi-matrix --version"
            );
            println!("  mackes-midi-matrix learn <endpoint-id> [limit]");
            println!("  mackes-midi-matrix effects faceplate [--json]");
            println!("  mackes-midi-matrix effects demo [--json]");
            println!("  mackes-midi-matrix effects assignments <profile-id> [--json]");
            println!("  mackes-midi-matrix effects plan <group>... [--json]");
            println!("  mackes-midi-matrix sysex <destination-id> <hex-bytes> --confirm");
            println!("  mackes-midi-matrix device-control <profile-id> <control> <channel> <value> <destination-id> --confirm");
            println!("  mackes-midi-matrix device-query <profile-id>");
            println!("  mackes-midi-matrix device-query <profile-id> <query-id>");
            println!("  mackes-midi-matrix scene next|previous");
            println!("  mackes-midi-matrix scene select <scene-id>");
            println!("  mackes-midi-matrix scene action-add <config> <project> <scene> <action-id> <description> <destination> <midi-hex> [--unsafe|--depends-on=<action-id>]");
            println!(
                "  mackes-midi-matrix scene action-remove <config> <project> <scene> <action-id>"
            );
            println!("  mackes-midi-matrix scene actions <config> <project> <scene> [--json]");
            println!("  mackes-midi-matrix scene plan <config> <project> <scene> [--json]");
            println!("  mackes-midi-matrix routes apply <routes.json>");
        }
        [command, action] if command == "effects" && action == "faceplate" => {
            print_effects_faceplate(false);
        }
        [command, action, flag]
            if command == "effects" && action == "faceplate" && flag == "--json" =>
        {
            print_effects_faceplate(true);
        }
        [command, action] if command == "effects" && action == "demo" => {
            print_effects_demo(false);
        }
        [command, action, flag] if command == "effects" && action == "demo" && flag == "--json" => {
            print_effects_demo(true);
        }
        [command, action, profile] if command == "effects" && action == "assignments" => {
            print_effects_assignments(profile, false);
        }
        [command, action, profile, flag]
            if command == "effects" && action == "assignments" && flag == "--json" =>
        {
            print_effects_assignments(profile, true);
        }
        [command, action, groups @ ..]
            if command == "effects"
                && action == "plan"
                && groups.last().map(String::as_str) != Some("--json") =>
        {
            print_effects_plan(groups, false);
        }
        [command, action, groups @ .., flag]
            if command == "effects" && action == "plan" && flag == "--json" =>
        {
            print_effects_plan(groups, true);
        }
        [command, action, path, capability] if command == "default" && action == "get" => {
            if let Err(error) = print_default_provider(path, capability, false) {
                eprintln!("default provider failed: {error}");
                std::process::exit(2);
            }
        }
        [command, action, path, capability, flag]
            if command == "default" && action == "get" && flag == "--json" =>
        {
            if let Err(error) = print_default_provider(path, capability, true) {
                eprintln!("default provider failed: {error}");
                std::process::exit(2);
            }
        }
        [command, action, path, capability, profile_id]
            if command == "default" && action == "set" =>
        {
            if let Err(error) = set_default_provider_cli(path, capability, profile_id) {
                eprintln!("default provider failed: {error}");
                std::process::exit(2);
            }
            println!("{}: {} (configured)", capability.trim(), profile_id);
        }
        [device, action, value] if device == "mvave" && action == "preset" => {
            let preset = value.parse::<u8>().unwrap_or(0);
            match mvave_ir_box_preset(preset, false) {
                Ok(result) => println!("{result}"),
                Err(error) => {
                    eprintln!("mackes-midi-matrix: {error}");
                    std::process::exit(2);
                }
            }
        }
        [device, action, preset, flag]
            if device == "reflex" && action == "preset" && flag == "--dry-run" =>
        {
            match reflex_pcm70_preset(preset, None, true) {
                Ok(result) => println!("{result}"),
                Err(error) => {
                    eprintln!("mackes-midi-matrix: {error}");
                    std::process::exit(2);
                }
            }
        }
        [device, action, preset, destination, flag]
            if device == "reflex" && action == "preset" && flag == "--confirm" =>
        {
            match reflex_pcm70_preset(preset, Some(destination), false) {
                Ok(result) => println!("{result}"),
                Err(error) => {
                    eprintln!("mackes-midi-matrix: {error}");
                    std::process::exit(2);
                }
            }
        }
        [device, action, value, flag]
            if device == "mvave" && action == "preset" && flag == "--dry-run" =>
        {
            let preset = value.parse::<u8>().unwrap_or(0);
            match mvave_ir_box_preset(preset, true) {
                Ok(result) => println!("{result}"),
                Err(error) => {
                    eprintln!("mackes-midi-matrix: {error}");
                    std::process::exit(2);
                }
            }
        }
        [device, module, state, confirmation]
            if device == "mvave"
                && matches!(module.as_str(), "ir" | "eq")
                && matches!(state.as_str(), "on" | "off")
                && confirmation == "--confirm-unverified" =>
        {
            match mvave_ir_box_module(module, state == "on") {
                Ok(result) => println!("{result}"),
                Err(error) => {
                    eprintln!("mackes-midi-matrix: {error}");
                    std::process::exit(2);
                }
            }
        }
        [command] if command == "status" => {
            println!("mackes-midi-matrix status: {}", daemon_status(false));
        }
        [command, flag] if command == "status" && flag == "--json" => {
            println!("{}", daemon_status(true));
        }
        [command] if command == "panic" => {
            let response = daemon_command(mackes_ipc::Command::Panic);
            println!("{response}");
            if response.contains("\"ok\":false") {
                std::process::exit(2);
            }
        }
        [command, endpoint] if command == "learn" => {
            print_learn(endpoint, 128);
        }
        [command, endpoint, limit] if command == "learn" => {
            print_learn(endpoint, limit.parse().unwrap_or(0));
        }
        [command, endpoint, limit, flag] if command == "learn" && flag == "--json" => {
            print_learn(endpoint, limit.parse().unwrap_or(0));
        }
        [command, destination, hex, flag] if command == "sysex" && flag == "--confirm" => {
            send_sysex_cli(destination, hex);
        }
        [command, profile, control, channel, value, destination, flag]
            if command == "device-control" && flag == "--confirm" =>
        {
            send_device_control_cli(profile, control, channel, value, destination);
        }
        [command, profile] if command == "device-query" => {
            let payload = serde_json::json!({ "profile_id": profile });
            let response = daemon_request(
                mackes_ipc::Command::DeviceQuery,
                &serde_json::to_vec(&payload).expect("device query payload is serializable"),
            );
            println!("{response}");
            if response.contains("\"ok\":false") {
                std::process::exit(2);
            }
        }
        [command, profile, query] if command == "device-query" => {
            let payload =
                serde_json::json!({ "profile_id": profile, "query_id": query, "parameters": [] });
            let response = daemon_request(
                mackes_ipc::Command::DeviceQuery,
                &serde_json::to_vec(&payload).expect("device query payload is serializable"),
            );
            println!("{response}");
            if response.contains("\"ok\":false") {
                std::process::exit(2);
            }
        }
        [command] if command == "scenes" || command == "devices" => {
            let ipc_command = if command == "scenes" {
                mackes_ipc::Command::Scenes
            } else {
                mackes_ipc::Command::DeviceQuery
            };
            print_daemon_command(ipc_command);
        }
        [command, direction]
            if command == "scene" && matches!(direction.as_str(), "next" | "previous") =>
        {
            navigate_scene_cli(direction);
        }
        [command, subcommand, scene] if command == "scene" && subcommand == "select" => {
            let payload = serde_json::to_vec(&serde_json::json!({"scene": scene}))
                .expect("scene selection payload is serializable");
            let response = daemon_request(mackes_ipc::Command::Scenes, &payload);
            println!("{response}");
            if response.contains("\"ok\":false") {
                std::process::exit(2);
            }
        }
        [command, direction, flag]
            if command == "scene"
                && matches!(direction.as_str(), "next" | "previous")
                && flag == "--json" =>
        {
            navigate_scene_cli(direction);
        }
        [command] if command == "monitor" => {
            print_daemon_command(mackes_ipc::Command::Monitor);
        }
        [command] if command == "routes" => {
            print_daemon_command(mackes_ipc::Command::Routes);
        }
        [command, action, path] if command == "routes" && action == "apply" => {
            apply_routes_cli(path);
        }
        [command, flag] if command == "monitor" && flag == "--json" => {
            print_daemon_command(mackes_ipc::Command::Monitor);
        }
        [command, flag] if command == "routes" && flag == "--json" => {
            print_daemon_command(mackes_ipc::Command::Routes);
        }
        [command, flag] if (command == "scenes" || command == "devices") && flag == "--json" => {
            let ipc_command = if command == "scenes" {
                mackes_ipc::Command::Scenes
            } else {
                mackes_ipc::Command::DeviceQuery
            };
            print_daemon_command(ipc_command);
        }
        [command, subcommand, directory] if command == "backup" && subcommand == "list" => {
            for entry in backup_entries(std::path::Path::new(directory)) {
                println!("{entry}");
            }
        }
        [command, subcommand, directory, flag]
            if command == "backup" && subcommand == "list" && flag == "--json" =>
        {
            println!(
                "{}",
                serde_json::json!({"backups": backup_entries(std::path::Path::new(directory))})
            );
        }
        [command, subcommand, path] if command == "backup" && subcommand == "inspect" => {
            match mackes_config::load_backup(std::path::Path::new(path)) {
                Ok((payload, manifest)) => println!(
                    "{path}: status={}, profile={}, device={}, bytes={}",
                    backup_status_label(&manifest.status),
                    manifest.profile,
                    manifest.device_identity,
                    payload.len()
                ),
                Err(error) => {
                    eprintln!("backup inspect failed: {error}");
                    std::process::exit(2);
                }
            }
        }
        [command, subcommand, path, flag]
            if command == "backup" && subcommand == "inspect" && flag == "--json" =>
        {
            match mackes_config::load_backup(std::path::Path::new(path)) {
                Ok((payload, manifest)) => println!(
                    "{}",
                    serde_json::json!({"path":path,"status":backup_status_label(&manifest.status),"profile":manifest.profile,"device_identity":manifest.device_identity,"bytes":payload.len()})
                ),
                Err(error) => {
                    println!("{}", serde_json::json!({"path":path,"verified":false,"error":error}));
                    std::process::exit(2);
                }
            }
        }
        [command, subcommand, backup, target, profile, identity]
            if command == "backup" && subcommand == "restore" =>
        {
            restore_cli(backup, target, profile, identity, false);
        }
        [command, subcommand, backup, target, profile, identity, flag]
            if command == "backup" && subcommand == "restore" && flag == "--apply" =>
        {
            restore_cli(backup, target, profile, identity, true);
        }
        [command, subcommand, path] if command == "scene" && subcommand == "list" => {
            match mackes_config::load(std::path::Path::new(path)) {
                Ok(document) => {
                    for project in document.projects {
                        println!(
                            "{}: {}",
                            project.id,
                            project
                                .scenes
                                .iter()
                                .map(|scene| scene.id.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                }
                Err(error) => {
                    eprintln!("scene list failed: {error:?}");
                    std::process::exit(2);
                }
            }
        }
        [command, subcommand, path, project, scene]
            if command == "scene" && subcommand == "plan" =>
        {
            scene_plan_cli(path, project, scene, false);
        }
        [command, subcommand, path, project, scene, action_id, description, destination, hex]
            if command == "scene" && subcommand == "action-add" =>
        {
            scene_action_add_cli(
                path,
                project,
                scene,
                action_id,
                description,
                destination,
                hex,
                false,
                None,
            );
        }
        [command, subcommand, path, project, scene, action_id]
            if command == "scene" && subcommand == "action-remove" =>
        {
            scene_action_remove_cli(path, project, scene, action_id);
        }
        [command, subcommand, path, project, scene]
            if command == "scene" && subcommand == "actions" =>
        {
            scene_actions_cli(path, project, scene, false);
        }
        [command, subcommand, path, project, scene, flag]
            if command == "scene" && subcommand == "actions" && flag == "--json" =>
        {
            scene_actions_cli(path, project, scene, true);
        }
        [command, subcommand, path, project, scene, action_id, description, destination, hex, flag]
            if command == "scene" && subcommand == "action-add" && flag == "--unsafe" =>
        {
            scene_action_add_cli(
                path,
                project,
                scene,
                action_id,
                description,
                destination,
                hex,
                true,
                None,
            );
        }
        [command, subcommand, path, project, scene, action_id, description, destination, hex, flag]
            if command == "scene"
                && subcommand == "action-add"
                && flag.starts_with("--depends-on=") =>
        {
            scene_action_add_cli(
                path,
                project,
                scene,
                action_id,
                description,
                destination,
                hex,
                false,
                flag.strip_prefix("--depends-on=").map(str::to_owned),
            );
        }
        [command, subcommand, path, project, scene, flag]
            if command == "scene" && subcommand == "plan" && flag == "--json" =>
        {
            scene_plan_cli(path, project, scene, true);
        }
        [command, subcommand, path, flag]
            if command == "scene" && subcommand == "list" && flag == "--json" =>
        {
            match mackes_config::load(std::path::Path::new(path)) {
                Ok(document) => {
                    let projects = document.projects.iter().map(|project| serde_json::json!({"id": project.id, "scenes": project.scenes.iter().map(|scene| scene.id.clone()).collect::<Vec<_>>() })).collect::<Vec<_>>();
                    println!("{}", serde_json::json!({"projects": projects}));
                }
                Err(error) => {
                    println!("{}", serde_json::json!({"error": format!("{error:?}")}));
                    std::process::exit(2);
                }
            }
        }
        [command] if command == "endpoints" => {
            let endpoints = discovered_endpoints();
            if endpoints.is_empty() {
                println!("mackes endpoints: none exposed by ALSA");
            } else {
                println!("mackes endpoints:");
                for endpoint in endpoints {
                    println!("  {endpoint}");
                }
            }
        }
        [command, flag] if command == "endpoints" && flag == "--json" => {
            let endpoints = discovered_endpoints();
            let entries =
                endpoints.iter().map(|endpoint| format!("\"{endpoint}\"")).collect::<Vec<_>>();
            println!("{{\"endpoints\":[{}]}}", entries.join(","));
        }
        [command, subcommand] if command == "profile" && subcommand == "validate" => {
            let profiles = mackes_profiles::builtin_profiles();
            let valid = profiles.iter().filter(|profile| profile.validate().is_ok()).count();
            println!("mackes profile validate: {valid}/{} built-in profiles valid", profiles.len());
            if valid != profiles.len() {
                std::process::exit(2);
            }
        }
        [command, subcommand] if command == "profile" && subcommand == "list" => {
            for profile in mackes_profiles::builtin_profiles() {
                println!("{}", profile.id);
            }
        }
        [command, subcommand, flag]
            if command == "profile" && subcommand == "list" && flag == "--json" =>
        {
            let profiles = mackes_profiles::builtin_profiles();
            println!(
                "{}",
                serde_json::json!({"profiles": profiles.iter().map(|profile| profile.id.clone()).collect::<Vec<_>>()})
            );
        }
        [command, subcommand] if command == "profile" && subcommand == "test" => {
            let profiles = mackes_profiles::builtin_profiles();
            let failures = profiles
                .iter()
                .filter_map(|profile| {
                    profile.validate().err().map(|error| format!("{}: {error}", profile.id))
                })
                .collect::<Vec<_>>();
            if failures.is_empty() {
                println!("mackes profile test: PASS ({} profiles)", profiles.len());
            } else {
                for failure in failures {
                    eprintln!("{failure}");
                }
                std::process::exit(2);
            }
        }
        [command, subcommand, flag]
            if command == "profile" && subcommand == "test" && flag == "--json" =>
        {
            let profiles = mackes_profiles::builtin_profiles();
            let failures = profiles
                .iter()
                .filter_map(|profile| {
                    profile.validate().err().map(|error| format!("{}: {error}", profile.id))
                })
                .collect::<Vec<_>>();
            println!(
                "{}",
                serde_json::json!({"ok":failures.is_empty(),"profiles":profiles.len(),"failures":failures})
            );
            if !failures.is_empty() {
                std::process::exit(2);
            }
        }
        [command, subcommand, flag]
            if command == "profile" && subcommand == "validate" && flag == "--json" =>
        {
            let profiles = mackes_profiles::builtin_profiles();
            let valid = profiles.iter().filter(|profile| profile.validate().is_ok()).count();
            println!("{{\"valid\":{},\"total\":{}}}", valid, profiles.len());
            if valid != profiles.len() {
                std::process::exit(2);
            }
        }
        [command, flag] if command == "doctor" && flag == "--json" => {
            println!(
                "{{\"platform\":\"{}\",\"architecture\":\"{}\",\"config\":\"deferred\"}}",
                std::env::consts::OS,
                std::env::consts::ARCH
            );
        }
        [command] if command == "doctor" => {
            println!(
                "mackes doctor: platform={}, architecture={}, config=deferred",
                std::env::consts::OS,
                std::env::consts::ARCH
            );
        }
        [command, path] if command == "validate" => {
            let output = mackes_config::validate_report(std::path::Path::new(path), false);
            print!("{output}");
            if !output.starts_with("valid:") {
                std::process::exit(2);
            }
        }
        [command, path, flag] if command == "validate" && flag == "--json" => {
            let output = mackes_config::validate_report(std::path::Path::new(path), true);
            println!("{output}");
            if !output.contains("\"valid\":true") {
                std::process::exit(2);
            }
        }
        [command, input, directory] if command == "export" => {
            match mackes_config::load(std::path::Path::new(input)) {
                Ok(document) => {
                    match mackes_config::export_portable(&document, std::path::Path::new(directory))
                    {
                        Ok(path) => println!("exported {}", path.display()),
                        Err(error) => {
                            eprintln!("mackes export failed: {error:?}");
                            std::process::exit(2);
                        }
                    }
                }
                Err(error) => {
                    eprintln!("mackes export failed: {error:?}");
                    std::process::exit(2);
                }
            }
        }
        _ => {
            eprintln!(
                "{}\n\n{}\n  {} validate <path> [--json]\n  {} export <config> <directory>\n  {} doctor [--json]\n  {} status [--json]\n  {} panic\n  {} endpoints [--json]\n  {} profile validate [--json]\n  {} --help\n  {} learn <endpoint-id> [limit]",
                "mackes-midi-matrix: invalid arguments".red().bold(),
                "Usage:".bright_cyan().bold(),
                "mackes-midi-matrix".bright_white().bold(),
                "mackes-midi-matrix".bright_white().bold(),
                "mackes-midi-matrix".bright_white().bold(),
                "mackes-midi-matrix".bright_white().bold(),
                "mackes-midi-matrix".bright_white().bold(),
                "mackes-midi-matrix".bright_white().bold(),
                "mackes-midi-matrix".bright_white().bold(),
                "mackes-midi-matrix".bright_white().bold(),
                "mackes-midi-matrix".bright_white().bold()
            );
            std::process::exit(64);
        }
    }
}

fn navigate_scene_cli(direction: &str) {
    let payload = serde_json::to_vec(&serde_json::json!({"direction": direction}))
        .expect("scene navigation payload is serializable");
    let response = daemon_request(mackes_ipc::Command::Scenes, &payload);
    println!("{response}");
    if response.contains("\"ok\":false") {
        std::process::exit(2);
    }
}

fn scene_plan_cli(path: &str, project_id: &str, scene_id: &str, json: bool) {
    let result = (|| -> Result<serde_json::Value, String> {
        let document =
            mackes_config::load(std::path::Path::new(path)).map_err(|error| error.to_string())?;
        let project = document
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| format!("project '{project_id}' was not found"))?;
        let scene =
            project.scenes.iter().find(|scene| scene.id == scene_id).ok_or_else(|| {
                format!("scene '{scene_id}' was not found in project '{project_id}'")
            })?;
        let plan = mackes_scene_engine::ActivationPlan::compile(
            scene
                .actions
                .iter()
                .map(|action| mackes_scene_engine::ActivationAction {
                    id: action.id.clone(),
                    description: action.description.clone(),
                    unsafe_action: action.unsafe_action,
                    depends_on: action.depends_on.clone(),
                    destination: action.destination.clone(),
                    message: action.message.clone(),
                })
                .collect(),
        )
        .map_err(str::to_owned)?;
        Ok(serde_json::json!({
            "ok": true,
            "project": project_id,
            "scene": scene_id,
            "actions": plan.actions.iter().map(|action| serde_json::json!({
                "id": action.id,
                "description": action.description,
                "unsafe": action.unsafe_action,
                "depends_on": action.depends_on,
            })).collect::<Vec<_>>(),
        }))
    })();
    match result {
        Ok(value) if json => println!("{value}"),
        Ok(value) => {
            println!("scene plan: {project_id}/{scene_id}");
            for action in value["actions"].as_array().into_iter().flatten() {
                println!(
                    "  {}{}: {}",
                    action["id"].as_str().unwrap_or("?"),
                    if action["unsafe"].as_bool() == Some(true) { " [unsafe]" } else { "" },
                    action["description"].as_str().unwrap_or("?")
                );
            }
        }
        Err(error) => {
            if json {
                println!("{}", serde_json::json!({"ok": false, "error": error}));
            } else {
                eprintln!("scene plan failed: {error}");
            }
            std::process::exit(2);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn scene_action_add_cli(
    path: &str,
    project_id: &str,
    scene_id: &str,
    action_id: &str,
    description: &str,
    destination: &str,
    hex: &str,
    unsafe_action: bool,
    depends_on: Option<String>,
) {
    let result = (|| -> Result<(), String> {
        let document =
            mackes_config::load(std::path::Path::new(path)).map_err(|error| error.to_string())?;
        let project = document
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| format!("project '{project_id}' was not found"))?;
        let message = parse_midi_hex(hex)?;
        let updated_project = mackes_config::add_scene_action(
            project,
            scene_id,
            mackes_config::SceneAction {
                id: action_id.to_owned(),
                description: description.to_owned(),
                unsafe_action,
                depends_on,
                destination: Some(destination.to_owned()),
                message: Some(message),
            },
        )?;
        let updated = mackes_config::replace_project(&document, updated_project)?;
        mackes_config::save(std::path::Path::new(path), &updated, 10)
            .map_err(|error| error.to_string())?;
        Ok(())
    })();
    match result {
        Ok(()) => println!("scene action added: {project_id}/{scene_id}/{action_id}"),
        Err(error) => {
            eprintln!("scene action add failed: {error}");
            std::process::exit(2);
        }
    }
}

fn parse_midi_hex(input: &str) -> Result<Vec<u8>, String> {
    let bytes = input
        .split_whitespace()
        .map(|token| u8::from_str_radix(token.trim_start_matches("0x"), 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "MIDI payload contains invalid hexadecimal".to_owned())?;
    if bytes.is_empty() || bytes.len() > 8192 {
        return Err("MIDI payload must contain 1..=8192 bytes".into());
    }
    mackes_domain::MidiMessage::from_wire(&bytes)
        .map_err(|error| format!("MIDI payload is invalid: {error}"))?;
    Ok(bytes)
}

fn scene_action_remove_cli(path: &str, project_id: &str, scene_id: &str, action_id: &str) {
    let result = (|| -> Result<(), String> {
        let document =
            mackes_config::load(std::path::Path::new(path)).map_err(|error| error.to_string())?;
        let project = document
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| format!("project '{project_id}' was not found"))?;
        let updated_project = mackes_config::remove_scene_action(project, scene_id, action_id)?;
        let updated = mackes_config::replace_project(&document, updated_project)?;
        mackes_config::save(std::path::Path::new(path), &updated, 10)
            .map_err(|error| error.to_string())?;
        Ok(())
    })();
    match result {
        Ok(()) => println!("scene action removed: {project_id}/{scene_id}/{action_id}"),
        Err(error) => {
            eprintln!("scene action remove failed: {error}");
            std::process::exit(2);
        }
    }
}

fn scene_actions_cli(path: &str, project_id: &str, scene_id: &str, json: bool) {
    let result = (|| -> Result<serde_json::Value, String> {
        let document =
            mackes_config::load(std::path::Path::new(path)).map_err(|error| error.to_string())?;
        let project = document
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| format!("project '{project_id}' was not found"))?;
        let scene = project
            .scenes
            .iter()
            .find(|scene| scene.id == scene_id)
            .ok_or_else(|| format!("scene '{scene_id}' was not found"))?;
        Ok(
            serde_json::json!({"ok": true, "project": project_id, "scene": scene_id, "actions": scene.actions}),
        )
    })();
    match result {
        Ok(value) if json => println!("{value}"),
        Ok(value) => {
            println!("scene actions: {project_id}/{scene_id}");
            for action in value["actions"].as_array().into_iter().flatten() {
                println!(
                    "  {}{}: {}",
                    action["id"].as_str().unwrap_or("?"),
                    if action["unsafe_action"].as_bool() == Some(true) { " [unsafe]" } else { "" },
                    action["description"].as_str().unwrap_or("?")
                );
            }
        }
        Err(error) => {
            eprintln!("scene actions failed: {error}");
            std::process::exit(2);
        }
    }
}

fn daemon_status(json: bool) -> String {
    let socket = std::env::var("MACKES_MIDI_MATRIX_SOCKET")
        .or_else(|_| std::env::var("MACKES_SOCKET"))
        .unwrap_or_else(|_| "/run/mackes-midi-matrix/control.sock".into());
    let request = mackes_ipc::Envelope {
        version: mackes_ipc::ProtocolVersion::current(),
        request_id: mackes_ipc::RequestId::new(1).expect("nonzero request ID"),
        command: mackes_ipc::Command::Snapshot,
        payload: b"{}".to_vec(),
    };
    let response = mackes_ipc::LocalClient::connect(&socket)
        .and_then(|mut client| {
            client
                .send(&request)
                .map_err(std::io::Error::other)
                .and_then(|()| client.receive().map_err(std::io::Error::other))
        })
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok());
    match (json, response) {
        (true, Some(response)) => response,
        (false, Some(response)) => format!("daemon={response}"),
        (true, None) => {
            "{\"daemon\":\"unavailable\",\"reason\":\"runtime IPC is not connected\"}".into()
        }
        (false, None) => "daemon=unavailable (runtime IPC is not connected)".into(),
    }
}

fn remember_mapping_undo(
    history: &mut Vec<Vec<mackes_tui::MappingDraft>>,
    editor: &mackes_tui::RoutingEditor,
) {
    history.push(editor.drafts.clone());
    if history.len() > 32 {
        history.remove(0);
    }
}

fn daemon_command(command: mackes_ipc::Command) -> String {
    daemon_request(command, b"{}")
}

fn daemon_request(command: mackes_ipc::Command, payload: &[u8]) -> String {
    let socket = std::env::var("MACKES_MIDI_MATRIX_SOCKET")
        .or_else(|_| std::env::var("MACKES_SOCKET"))
        .unwrap_or_else(|_| "/run/mackes-midi-matrix/control.sock".into());
    let request = mackes_ipc::Envelope {
        version: mackes_ipc::ProtocolVersion::current(),
        request_id: mackes_ipc::RequestId::new(2).expect("nonzero request ID"),
        command,
        payload: payload.to_vec(),
    };
    mackes_ipc::LocalClient::connect(&socket)
        .and_then(|mut client| {
            client.send(&request).map_err(std::io::Error::other)?;
            client.receive().map_err(std::io::Error::other)
        })
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or_else(|| "{\"ok\":false,\"error\":\"runtime IPC is not connected\"}".into())
}

#[allow(clippy::needless_pass_by_value)]
fn mapping_store_request(generation: u64, operation: &str, payload: serde_json::Value) -> String {
    let request = serde_json::json!({
        "operation": operation,
        "generation": generation,
        "payload": payload,
    });
    daemon_request(mackes_ipc::Command::Mappings, &serde_json::to_vec(&request).unwrap_or_default())
}

fn mapping_result_health(response: &str) -> String {
    format!("mapping: {}", mackes_tui::mapping_response_notice(response))
}

#[allow(clippy::too_many_arguments)]
fn synchronize_snapshot(
    state: &mut mackes_tui::ClientState,
    dashboard: &mut mackes_tui::DashboardState,
    routing: &mut mackes_tui::RoutingEditor,
    route_generation: &mut u64,
    monitor: &mut mackes_tui::MonitorState,
    diagnostics: &mut mackes_tui::DiagnosticsState,
    setlists: &mut mackes_tui::SetlistEditor,
    learn: &mut mackes_tui::LearnWorkspace,
    assignment_wizard: &mut mackes_tui::AssignmentWizard,
) -> Result<String, String> {
    let response = daemon_request(mackes_ipc::Command::Snapshot, b"{}");
    let value: serde_json::Value =
        serde_json::from_str(&response).map_err(|error| error.to_string())?;
    if value.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err("daemon snapshot failed".into());
    }
    let last_sequence = value
        .get("last_sequence")
        .and_then(serde_json::Value::as_u64)
        .ok_or("snapshot sequence is missing")?;
    let health =
        value.get("health").and_then(serde_json::Value::as_str).unwrap_or("degraded").to_owned();
    state.apply_snapshot(mackes_ipc::StateSnapshot {
        last_sequence,
        payload: serde_json::to_vec(&value).map_err(|error| error.to_string())?,
    });
    // Project the same authoritative snapshot into widgets before replay begins.
    // Event replay then applies only subsequent changes.
    //
    // The dashboard is intentionally not reconstructed from local files or MIDI state.
    apply_dashboard_payload(dashboard, &value);
    project_routes(routing, &value);
    project_observability(monitor, diagnostics, &value);
    project_setlists(setlists, &value);
    project_learn_alias(learn, &value);
    reconcile_assignment_session(assignment_wizard, &value);
    if let Some(generation) = value.get("route_generation").and_then(serde_json::Value::as_u64) {
        *route_generation = generation;
    }
    Ok(health)
}

fn apply_dashboard_payload(
    dashboard: &mut mackes_tui::DashboardState,
    payload: &serde_json::Value,
) {
    for event in mackes_tui::DashboardEvent::from_payload(payload) {
        dashboard.apply_event(event);
    }
    /*
    if let Some(scene) = payload.get("active_scene").and_then(|value| {
        if value.is_null() {
            Some(None)
        } else {
            value.as_str().map(|value| Some(value.to_owned()))
        }
    }) {
        dashboard.apply_event(mackes_tui::DashboardEvent::ActiveScene(scene));
    }
    if let Some(generation) = payload.get("route_generation").and_then(serde_json::Value::as_u64) {
        dashboard.apply_event(mackes_tui::DashboardEvent::RouteGeneration(generation));
    }
    if let (Some(received), Some(sent), Some(dropped)) = (
        payload.get("received").and_then(serde_json::Value::as_u64),
        payload.get("sent").and_then(serde_json::Value::as_u64),
        payload.get("dropped").and_then(serde_json::Value::as_u64),
    ) {
        dashboard.apply_event(mackes_tui::DashboardEvent::Activity { received, sent, dropped });
    }
    if let (Some(completed), Some(total)) = (
        payload.get("activation_completed").and_then(serde_json::Value::as_u64),
        payload.get("activation_total").and_then(serde_json::Value::as_u64),
    ) {
        if let (Ok(completed), Ok(total)) = (u32::try_from(completed), u32::try_from(total)) {
            dashboard
                .apply_event(mackes_tui::DashboardEvent::ActivationProgress { completed, total });
        }
    }
    if let Some(result) = payload.get("activation_result").and_then(serde_json::Value::as_str) {
        dashboard.apply_event(mackes_tui::DashboardEvent::ActivationResult(result.to_owned()));
    }
    */
}

fn project_observability(
    monitor: &mut mackes_tui::MonitorState,
    diagnostics: &mut mackes_tui::DiagnosticsState,
    payload: &serde_json::Value,
) {
    if let Some(latest) =
        payload.get("audit").and_then(serde_json::Value::as_array).and_then(|audit| audit.first())
    {
        if latest.get("allowed").and_then(serde_json::Value::as_bool) == Some(false) {
            let action =
                latest.get("action").and_then(serde_json::Value::as_str).unwrap_or("mutation");
            diagnostics.push(mackes_tui::HealthDiagnostic {
                subject: "policy".into(),
                severity: mackes_tui::MonitorSeverity::Warning,
                reason: format!("{action} was denied"),
                remediation: "review performance lock and operator authorization state".into(),
            });
        }
    }
    let Some(health) = payload.get("health").and_then(serde_json::Value::as_str) else {
        return;
    };
    let severity = if health == "ready" || health == "online" {
        mackes_tui::MonitorSeverity::Info
    } else {
        mackes_tui::MonitorSeverity::Warning
    };
    monitor
        .push(mackes_tui::MonitorEntry { severity, message: format!("daemon health: {health}") });
    if severity >= mackes_tui::MonitorSeverity::Warning {
        diagnostics.push(mackes_tui::HealthDiagnostic {
            subject: "daemon".into(),
            severity,
            reason: format!("health is {health}"),
            remediation: "inspect the latest daemon status and event details".into(),
        });
    }
}

fn project_setlists(editor: &mut mackes_tui::SetlistEditor, payload: &serde_json::Value) {
    if let Some(projects) = payload
        .get("catalog")
        .and_then(|catalog| catalog.get("projects"))
        .and_then(serde_json::Value::as_array)
    {
        editor.available_projects = projects
            .iter()
            .filter_map(|project| project.get("id").and_then(serde_json::Value::as_str))
            .map(str::to_owned)
            .collect();
    }
    let Some(values) = payload
        .get("catalog")
        .and_then(|v| v.get("setlists"))
        .and_then(serde_json::Value::as_array)
    else {
        return;
    };
    let Ok(setlists) = serde_json::from_value::<Vec<mackes_config::Setlist>>(
        serde_json::Value::Array(values.clone()),
    ) else {
        return;
    };
    editor.drafts = setlists;
    editor.selected = None;
}

fn project_learn_alias(learn: &mut mackes_tui::LearnWorkspace, payload: &serde_json::Value) {
    let Some(alias) = payload
        .get("catalog")
        .and_then(|catalog| catalog.get("learn_input_alias"))
        .and_then(serde_json::Value::as_str)
    else {
        return;
    };
    if learn.learn_input_alias.is_none() {
        let _ = learn.set_input_alias(alias);
    }
    if let Some(endpoint_id) = payload
        .get("catalog")
        .and_then(|catalog| catalog.get("learn_endpoint_id"))
        .and_then(serde_json::Value::as_str)
    {
        learn.set_endpoint_id(endpoint_id);
    }
}

fn poll_learn_capture(learn: &mut mackes_tui::LearnWorkspace) {
    let Some(endpoint_id) = learn.learn_endpoint_id.as_deref() else { return };
    let payload = serde_json::json!({"endpoint_id": endpoint_id, "limit": 128});
    let Ok(bytes) = serde_json::to_vec(&payload) else { return };
    let response = daemon_request(mackes_ipc::Command::Learn, &bytes);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&response) else { return };
    let Some(candidates) = value.get("candidates").cloned() else { return };
    let candidates = candidates
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|candidate| {
            let kind = match candidate.get("kind")?.as_str()? {
                "noteon" => mackes_midi_engine::LearnMessageKind::NoteOn,
                "noteoff" => mackes_midi_engine::LearnMessageKind::NoteOff,
                "polypressure" => mackes_midi_engine::LearnMessageKind::PolyPressure,
                "controlchange" => mackes_midi_engine::LearnMessageKind::ControlChange,
                "programchange" => mackes_midi_engine::LearnMessageKind::ProgramChange,
                "channelpressure" => mackes_midi_engine::LearnMessageKind::ChannelPressure,
                "pitchbend" => mackes_midi_engine::LearnMessageKind::PitchBend,
                "systemcommon" => mackes_midi_engine::LearnMessageKind::SystemCommon,
                "realtime" => mackes_midi_engine::LearnMessageKind::Realtime,
                "sysex" => mackes_midi_engine::LearnMessageKind::SysEx,
                _ => return None,
            };
            let raw = candidate
                .get("raw")?
                .as_array()?
                .iter()
                .map(|v| v.as_u64().and_then(|value| u8::try_from(value).ok()))
                .collect::<Option<Vec<_>>>()?;
            Some(mackes_midi_engine::MidiLearnCandidate {
                kind,
                channel: candidate
                    .get("channel")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|v| u8::try_from(v).ok()),
                number: candidate
                    .get("number")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|v| u8::try_from(v).ok()),
                observations: u32::try_from(candidate.get("observations")?.as_u64()?).ok()?,
                minimum: candidate
                    .get("minimum")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|v| u16::try_from(v).ok()),
                maximum: candidate
                    .get("maximum")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|v| u16::try_from(v).ok()),
                raw,
            })
        })
        .collect::<Vec<_>>();
    learn.finish_capture(candidates);
}

fn save_routes(editor: &mackes_tui::RoutingEditor, current_generation: u64) -> String {
    let routes = editor
        .drafts
        .iter()
        .filter_map(|draft| {
            let source = draft.source.trim().parse::<u64>().ok()?;
            let destination = draft.destination.trim().parse::<u64>().ok()?;
            let class = match draft.mode {
                mackes_tui::MappingMode::Cc => "ControlChange",
                mackes_tui::MappingMode::ProgramChange => "ProgramChange",
                mackes_tui::MappingMode::Note => "Note",
                mackes_tui::MappingMode::PitchBend => "PitchBend",
                mackes_tui::MappingMode::Sysex => "SysEx",
            };
            Some(serde_json::json!({
                "source": source,
                "destination": destination,
                "destination_parameter": draft.destination_parameter,
                "channel": draft.channel,
                "class": class,
                // Preserve the daemon route contract even when the current editor
                // does not expose cycle authorization as a separate control.
                "allow_cycle": draft.allow_cycle,
                "enabled": draft.enabled,
                "priority": draft.priority,
                "curve": match draft.curve {
                    mackes_midi_engine::Curve::Linear => "linear",
                    mackes_midi_engine::Curve::Square => "square",
                    mackes_midi_engine::Curve::SquareRoot => "square_root",
                },
                "predicates": serde_json::to_value(&draft.filters.predicates).ok()?,
            }))
        })
        .collect::<Vec<_>>();
    if routes.len() != editor.drafts.len() {
        return "{\"ok\":false,\"error\":\"routing draft contains invalid endpoint IDs\"}".into();
    }
    let payload = match serde_json::to_vec(&serde_json::json!({
        "routes": routes,
        "route_generation": current_generation.saturating_add(1),
    })) {
        Ok(payload) => payload,
        Err(error) => {
            return format!(
                "{{\"ok\":false,\"error\":\"route payload serialization failed: {error}\"}}"
            )
        }
    };
    daemon_request(mackes_ipc::Command::Routes, &payload)
}

fn apply_routes_cli(path: &str) {
    let bytes = std::fs::read(path).unwrap_or_else(|error| {
        eprintln!("mackes-midi-matrix: cannot read routes file: {error}");
        std::process::exit(2);
    });
    let value = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap_or_else(|error| {
        eprintln!("mackes-midi-matrix: invalid routes JSON: {error}");
        std::process::exit(2);
    });
    let document = if value.as_array().is_some() {
        serde_json::json!({ "routes": value })
    } else if value.get("routes").and_then(serde_json::Value::as_array).is_some() {
        value
    } else {
        eprintln!("mackes-midi-matrix: routes file must be an array or contain a routes array");
        std::process::exit(2);
    };
    let encoded = serde_json::to_vec(&document).expect("validated routes JSON is serializable");
    let response = daemon_request(mackes_ipc::Command::Routes, &encoded);
    println!("{response}");
    if response.contains("\"ok\":false") {
        std::process::exit(2);
    }
}

fn endpoint_pair_for_new_route() -> Option<(u64, u64)> {
    let response = daemon_request(mackes_ipc::Command::Endpoints, b"{}");
    let value = serde_json::from_str::<serde_json::Value>(&response).ok()?;
    let endpoints = value.get("endpoints")?.as_array()?;
    let input = endpoints
        .iter()
        .find(|endpoint| {
            endpoint.get("direction").and_then(serde_json::Value::as_str) == Some("input")
        })?
        .get("id")?
        .as_u64()?;
    let output = endpoints
        .iter()
        .find(|endpoint| {
            endpoint.get("direction").and_then(serde_json::Value::as_str) == Some("output")
        })?
        .get("id")?
        .as_u64()?;
    Some((input, output))
}

fn first_output_endpoint() -> Option<String> {
    let response = daemon_request(mackes_ipc::Command::Endpoints, b"{}");
    let value = serde_json::from_str::<serde_json::Value>(&response).ok()?;
    value
        .get("endpoints")?
        .as_array()?
        .iter()
        .find(|endpoint| {
            endpoint.get("direction").and_then(serde_json::Value::as_str) == Some("output")
        })?
        .get("id")?
        .as_str()
        .map(str::to_owned)
}

fn save_setlists(editor: &mackes_tui::SetlistEditor) -> String {
    let payload = serde_json::to_vec(&serde_json::json!({"setlists": editor.drafts}))
        .expect("setlist save payload is serializable");
    daemon_request(mackes_ipc::Command::Configuration, &payload)
}

fn save_learned_mapping(mapping: &mackes_config::LearnedMapping) -> String {
    let payload = serde_json::to_vec(&serde_json::json!({"learned_mappings": [mapping]}))
        .expect("learned mapping payload is serializable");
    daemon_request(mackes_ipc::Command::Configuration, &payload)
}

fn project_routes(editor: &mut mackes_tui::RoutingEditor, payload: &serde_json::Value) {
    let Some(routes) = payload.get("routes").and_then(serde_json::Value::as_array) else {
        return;
    };
    let drafts = routes
        .iter()
        .filter_map(|route| {
            let source = route.get("source")?.as_u64()?;
            let destination = route.get("destination")?.as_u64()?;
            if source == 0 || destination == 0 {
                return None;
            }
            let mode = match route.get("class")?.as_str()? {
                "ControlChange" => mackes_tui::MappingMode::Cc,
                "ProgramChange" => mackes_tui::MappingMode::ProgramChange,
                "Note" | "NoteOn" | "NoteOff" => mackes_tui::MappingMode::Note,
                "PitchBend" => mackes_tui::MappingMode::PitchBend,
                "SysEx" => mackes_tui::MappingMode::Sysex,
                _ => return None,
            };
            let channel = route
                .get("channel")
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| u8::try_from(value).ok());
            Some(mackes_tui::MappingDraft {
                source: source.to_string(),
                destination: destination.to_string(),
                destination_parameter: route
                    .get("destination_parameter")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                channel,
                enabled: route.get("enabled").and_then(serde_json::Value::as_bool).unwrap_or(true),
                mode,
                priority: route
                    .get("priority")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|v| u16::try_from(v).ok())
                    .unwrap_or(0),
                curve: match route.get("curve").and_then(serde_json::Value::as_str) {
                    Some("square") => mackes_midi_engine::Curve::Square,
                    Some("square_root") => mackes_midi_engine::Curve::SquareRoot,
                    _ => mackes_midi_engine::Curve::Linear,
                },
                filters: mackes_tui::MappingFilterDraft {
                    predicates: route
                        .get("predicates")
                        .and_then(|value| serde_json::from_value(value.clone()).ok())
                        .unwrap_or_default(),
                },
                allow_cycle: route
                    .get("allow_cycle")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect::<Vec<_>>();
    if drafts.len() == routes.len() && mackes_tui::validate_mapping_batch(&drafts).is_ok() {
        editor.drafts = drafts;
        editor.selected = None;
    }
}

#[allow(clippy::too_many_arguments)]
fn synchronize_events(
    state: &mut mackes_tui::ClientState,
    dashboard: &mut mackes_tui::DashboardState,
    routing: &mut mackes_tui::RoutingEditor,
    route_generation: &mut u64,
    monitor: &mut mackes_tui::MonitorState,
    diagnostics: &mut mackes_tui::DiagnosticsState,
    setlists: &mut mackes_tui::SetlistEditor,
    learn: &mut mackes_tui::LearnWorkspace,
    assignment_wizard: &mut mackes_tui::AssignmentWizard,
    assignment_choices: &mut mackes_tui::AssignmentChoiceBrowser,
    task_shell: &mut mackes_tui::TaskShellState,
) -> Result<String, String> {
    let payload = serde_json::to_vec(&serde_json::json!({
        "after_sequence": state.last_sequence,
    }))
    .map_err(|error| error.to_string())?;
    let response = daemon_request(mackes_ipc::Command::Subscribe, &payload);
    let value: serde_json::Value =
        serde_json::from_str(&response).map_err(|error| error.to_string())?;
    if value.get("snapshot_required").and_then(serde_json::Value::as_bool) == Some(true) {
        return Err("daemon event continuity was lost".into());
    }
    if value.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err("daemon subscription failed".into());
    }
    let events = value
        .get("events")
        .and_then(serde_json::Value::as_array)
        .ok_or("subscription events are missing")?;
    let mut health = serde_json::from_slice::<serde_json::Value>(&state.payload)
        .ok()
        .and_then(|payload| {
            payload.get("health").and_then(serde_json::Value::as_str).map(str::to_owned)
        })
        .unwrap_or_else(|| "online".to_owned());
    for event in events {
        let sequence = event
            .get("sequence")
            .and_then(serde_json::Value::as_u64)
            .ok_or("event sequence is missing")?;
        let payload = event.get("payload").ok_or("event payload is missing")?;
        apply_dashboard_payload(dashboard, payload);
        project_routes(routing, payload);
        project_observability(monitor, diagnostics, payload);
        project_setlists(setlists, payload);
        project_learn_alias(learn, payload);
        reconcile_assignment_session(assignment_wizard, payload);
        if let Some(action) = payload.get("ui_navigation").and_then(serde_json::Value::as_str) {
            if assignment_wizard.session.phase == mackes_ipc::AssignmentPhase::Idle {
                let shell_action = match action {
                    "up" => Some(mackes_tui::ShellAction::Up),
                    "down" => Some(mackes_tui::ShellAction::Down),
                    "left" => Some(mackes_tui::ShellAction::Left),
                    "right" => Some(mackes_tui::ShellAction::Right),
                    _ => None,
                };
                if let Some(shell_action) = shell_action {
                    task_shell.apply(shell_action);
                }
            } else {
                match action {
                    "up" => {
                        assignment_choices.move_selection(false);
                    }
                    "down" => {
                        assignment_choices.move_selection(true);
                    }
                    _ => {}
                }
            }
        }
        if let Some(generation) =
            payload.get("route_generation").and_then(serde_json::Value::as_u64)
        {
            *route_generation = generation;
        }
        if let Some(event_health) = payload.get("health").and_then(serde_json::Value::as_str) {
            event_health.clone_into(&mut health);
        }
        state
            .apply_event(mackes_ipc::StateEvent {
                sequence,
                payload: serde_json::to_vec(payload).map_err(|error| error.to_string())?,
            })
            .map_err(|_| "daemon event continuity was lost".to_owned())?;
    }
    Ok(health)
}

fn print_daemon_command(command: mackes_ipc::Command) {
    let response = daemon_command(command);
    let unavailable = response.contains("\"ok\":false");
    println!("{response}");
    if unavailable {
        std::process::exit(2);
    }
}

fn reconcile_assignment_session(
    wizard: &mut mackes_tui::AssignmentWizard,
    payload: &serde_json::Value,
) {
    let Some(session) = payload.get("assignment_session").and_then(|value| {
        serde_json::from_value::<mackes_ipc::AssignmentSession>(value.clone()).ok()
    }) else {
        return;
    };
    wizard.reconcile(mackes_ipc::AssignmentResult {
        generation: payload
            .get("generation")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default(),
        session,
        applied: true,
        reason: None,
    });
}

fn send_sysex_cli(destination: &str, hex: &str) {
    let bytes = match mackes_profiles::parse_sysex_hex(hex) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("mackes-midi-matrix: {error}");
            std::process::exit(2);
        }
    };
    let payload = serde_json::json!({
        "destination": destination,
        "bytes": bytes,
        "confirm": true,
    });
    let encoded = match serde_json::to_vec(&payload) {
        Ok(encoded) => encoded,
        Err(error) => {
            eprintln!("mackes-midi-matrix: cannot encode SysEx request: {error}");
            std::process::exit(2);
        }
    };
    let response = daemon_request(mackes_ipc::Command::Sysex, &encoded);
    println!("{response}");
    if response.contains("\"ok\":false") {
        std::process::exit(2);
    }
}

fn send_device_control_cli(
    profile: &str,
    control: &str,
    channel: &str,
    value: &str,
    destination: &str,
) {
    let channel = channel.parse::<u8>().unwrap_or(0);
    let value = value.parse::<u16>().unwrap_or(u16::MAX);
    let payload = serde_json::json!({
        "profile_id": profile,
        "control": control,
        "channel": channel,
        "value": value,
        "destination": destination,
        "confirm": true,
    });
    let encoded = match serde_json::to_vec(&payload) {
        Ok(encoded) => encoded,
        Err(error) => {
            eprintln!("mackes-midi-matrix: cannot encode device-control request: {error}");
            std::process::exit(2);
        }
    };
    let response = daemon_request(mackes_ipc::Command::DeviceControl, &encoded);
    println!("{response}");
    if response.contains("\"ok\":false") {
        std::process::exit(2);
    }
}

fn print_learn(endpoint: &str, limit: usize) {
    let endpoint = endpoint.parse::<u64>().unwrap_or(0);
    if endpoint == 0 || !(1..=128).contains(&limit) {
        eprintln!("mackes-midi-matrix: learn requires endpoint-id and limit 1..=128");
        std::process::exit(64);
    }
    let payload = serde_json::to_vec(&serde_json::json!({
        "endpoint": endpoint,
        "limit": limit,
    }))
    .expect("learn payload is serializable");
    let response = daemon_request(mackes_ipc::Command::Learn, &payload);
    println!("{response}");
    if response.contains("\"ok\":false") {
        std::process::exit(2);
    }
}

fn dispatch_ui_command(command: mackes_tui::UiCommand) -> String {
    let Some(ipc_command) = mackes_tui::ipc_command_for(command) else {
        return "{\"ok\":false,\"error\":\"UI command is local-only\"}".to_owned();
    };
    daemon_command(ipc_command)
}

fn discovered_endpoints() -> Vec<String> {
    if let Ok(ports) = mackes_midi_engine::enumerate_midir_ports() {
        if !ports.is_empty() {
            return ports
                .into_iter()
                .map(|port| {
                    let direction = match port.direction {
                        mackes_midi_engine::EndpointDirection::Input => "input",
                        mackes_midi_engine::EndpointDirection::Output => "output",
                    };
                    format!("{} [{direction}]", port.name)
                })
                .collect();
        }
    }
    midi_nodes()
}

fn midi_nodes() -> Vec<String> {
    let Ok(entries) = std::fs::read_dir("/dev/snd") else { return Vec::new() };
    let mut nodes = entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with("midiC") && name.contains('D'))
        .collect::<Vec<_>>();
    nodes.sort();
    nodes
}

fn backup_entries(directory: &std::path::Path) -> Vec<String> {
    let mut entries = std::fs::read_dir(directory)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".backup") || name.ends_with(".manifest.json"))
        .collect::<Vec<_>>();
    entries.sort();
    entries
}

const fn backup_status_label(status: &mackes_config::BackupStatus) -> &'static str {
    match status {
        mackes_config::BackupStatus::Verified => "verified",
        mackes_config::BackupStatus::SentUnverified => "sent_unverified",
        mackes_config::BackupStatus::Failed => "failed",
    }
}

fn restore_cli(backup: &str, target: &str, profile: &str, identity: &str, apply: bool) {
    let mode =
        if apply { mackes_config::RestoreMode::Apply } else { mackes_config::RestoreMode::DryRun };
    match mackes_config::restore_backup(
        std::path::Path::new(backup),
        std::path::Path::new(target),
        profile,
        identity,
        mode,
    ) {
        Ok(result) => println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "result": format!("{result:?}"),
                "status": restore_result_status(&result),
                "applied": apply
            })
        ),
        Err(error) => {
            eprintln!("backup restore failed: {error}");
            std::process::exit(2);
        }
    }
}

const fn restore_result_status(result: &mackes_config::RestoreResult) -> &'static str {
    match result {
        mackes_config::RestoreResult::Planned { status, .. }
        | mackes_config::RestoreResult::Applied { status, .. } => backup_status_label(status),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_midi_hex, project_observability, project_routes, project_setlists};

    #[test]
    fn scene_midi_payload_parser_accepts_all_supported_wire_families() {
        assert_eq!(parse_midi_hex("B0 14 40").expect("CC"), vec![0xB0, 0x14, 0x40]);
        assert_eq!(parse_midi_hex("C0 05").expect("program"), vec![0xC0, 0x05]);
        assert!(parse_midi_hex("F0 7D 01 F7").is_ok());
        assert!(parse_midi_hex("B0 14").is_err());
        assert!(parse_midi_hex("F0 80 F7").is_err());
    }

    #[test]
    fn route_projection_converts_supported_daemon_routes() {
        let mut editor = mackes_tui::RoutingEditor::from_bank(&mackes_tui::MappingBank::new());
        project_routes(
            &mut editor,
            &serde_json::json!({
                "routes": [{"source": 11, "destination": 22, "channel": 3, "class": "ControlChange"}]
            }),
        );
        assert_eq!(editor.drafts.len(), 1);
        assert_eq!(editor.drafts[0].source, "11");
        assert_eq!(editor.drafts[0].destination, "22");
        assert_eq!(editor.drafts[0].channel, Some(3));
        assert_eq!(editor.drafts[0].mode, mackes_tui::MappingMode::Cc);
    }

    #[test]
    fn route_projection_preserves_drafts_when_payload_is_not_projectable() {
        let mut editor = mackes_tui::RoutingEditor::from_bank(&mackes_tui::MappingBank::new());
        editor.drafts.push(mackes_tui::MappingDraft {
            source: "old".into(),
            destination: "target".into(),
            destination_parameter: None,
            channel: Some(1),
            enabled: true,
            mode: mackes_tui::MappingMode::Cc,
            priority: 0,
            curve: mackes_midi_engine::Curve::Linear,
            filters: mackes_tui::MappingFilterDraft::default(),
            allow_cycle: false,
        });
        project_routes(
            &mut editor,
            &serde_json::json!({
                "routes": [{"source": 0, "destination": 22, "class": "ControlChange"}]
            }),
        );
        assert_eq!(editor.drafts.len(), 1);
        assert_eq!(editor.drafts[0].source, "old");
    }

    #[test]
    fn observability_projection_is_bounded_and_actionable() {
        let mut monitor = mackes_tui::MonitorState::new(1).expect("capacity");
        let mut diagnostics = mackes_tui::DiagnosticsState::new();
        project_observability(
            &mut monitor,
            &mut diagnostics,
            &serde_json::json!({"health": "degraded"}),
        );
        assert_eq!(monitor.entries.len(), 1);
        assert_eq!(diagnostics.entries.len(), 1);
        assert!(diagnostics.entries[0].line().contains("inspect"));
        project_observability(
            &mut monitor,
            &mut diagnostics,
            &serde_json::json!({
                "health": "ready",
                "audit": [{"action": "route_replace", "allowed": false}]
            }),
        );
        assert!(diagnostics
            .entries
            .iter()
            .any(|entry| { entry.subject == "policy" && entry.reason.contains("route_replace") }));
    }

    #[test]
    fn setlist_projection_accepts_valid_catalog_and_preserves_on_malformed_data() {
        let mut editor = mackes_tui::SetlistEditor::from_snapshot(&[]);
        project_setlists(
            &mut editor,
            &serde_json::json!({
                "catalog": {"setlists": [{"id": "live", "projects": ["demo"]}]}
            }),
        );
        assert_eq!(editor.drafts.len(), 1);
        assert_eq!(editor.drafts[0].id, "live");
        project_setlists(
            &mut editor,
            &serde_json::json!({"catalog": {"setlists": [{"id": 4, "projects": []}]}}),
        );
        assert_eq!(editor.drafts.len(), 1);
        assert_eq!(editor.drafts[0].id, "live");
    }
}
