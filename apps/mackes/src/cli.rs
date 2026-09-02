//! Daemon-backed operator CLI helpers.

pub(crate) fn reflex_pcm70_preset(
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

pub(crate) fn set_default_provider_cli(
    path: &str,
    capability: &str,
    profile_id: &str,
) -> Result<(), String> {
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

pub(crate) fn print_default_provider(
    path: &str,
    capability: &str,
    json: bool,
) -> Result<(), String> {
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

pub(crate) fn print_effects_faceplate(json: bool) {
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

pub(crate) fn print_effects_demo(json: bool) {
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

pub(crate) fn print_effects_assignments(profile_id: &str, json: bool) {
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

pub(crate) fn print_effects_plan(groups: &[String], json: bool) {
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
pub(crate) fn navigate_scene_cli(direction: &str) {
    let payload = serde_json::to_vec(&serde_json::json!({"direction": direction}))
        .expect("scene navigation payload is serializable");
    let response = daemon_request(mackes_ipc::Command::Scenes, &payload);
    println!("{response}");
    if response.contains("\"ok\":false") {
        std::process::exit(2);
    }
}

pub(crate) fn scene_plan_cli(path: &str, project_id: &str, scene_id: &str, json: bool) {
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
pub(crate) fn scene_action_add_cli(
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

pub(crate) fn parse_midi_hex(input: &str) -> Result<Vec<u8>, String> {
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

pub(crate) fn scene_action_remove_cli(
    path: &str,
    project_id: &str,
    scene_id: &str,
    action_id: &str,
) {
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

pub(crate) fn scene_actions_cli(path: &str, project_id: &str, scene_id: &str, json: bool) {
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

pub(crate) fn daemon_status(json: bool) -> String {
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

pub(crate) fn remember_mapping_undo(
    history: &mut Vec<Vec<mackes_tui::MappingDraft>>,
    editor: &mackes_tui::RoutingEditor,
) {
    history.push(editor.drafts.clone());
    if history.len() > 32 {
        history.remove(0);
    }
}

pub(crate) fn daemon_command(command: mackes_ipc::Command) -> String {
    daemon_request(command, b"{}")
}

pub(crate) fn daemon_request(command: mackes_ipc::Command, payload: &[u8]) -> String {
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
pub(crate) fn mapping_store_request(
    generation: u64,
    operation: &str,
    payload: serde_json::Value,
) -> String {
    let request = serde_json::json!({
        "operation": operation,
        "generation": generation,
        "payload": payload,
    });
    daemon_request(mackes_ipc::Command::Mappings, &serde_json::to_vec(&request).unwrap_or_default())
}

pub(crate) fn mapping_result_health(response: &str) -> String {
    format!("mapping: {}", mackes_tui::mapping_response_notice(response))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn synchronize_snapshot(
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

pub(crate) fn apply_dashboard_payload(
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

pub(crate) fn project_observability(
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

pub(crate) fn project_setlists(
    editor: &mut mackes_tui::SetlistEditor,
    payload: &serde_json::Value,
) {
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

pub(crate) fn project_learn_alias(
    learn: &mut mackes_tui::LearnWorkspace,
    payload: &serde_json::Value,
) {
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

pub(crate) fn poll_learn_capture(learn: &mut mackes_tui::LearnWorkspace) {
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

pub(crate) fn save_routes(editor: &mackes_tui::RoutingEditor, current_generation: u64) -> String {
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

pub(crate) fn apply_routes_cli(path: &str) {
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

pub(crate) fn endpoint_pair_for_new_route() -> Option<(u64, u64)> {
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

pub(crate) fn first_output_endpoint() -> Option<String> {
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

pub(crate) fn save_setlists(editor: &mackes_tui::SetlistEditor) -> String {
    let payload = serde_json::to_vec(&serde_json::json!({"setlists": editor.drafts}))
        .expect("setlist save payload is serializable");
    daemon_request(mackes_ipc::Command::Configuration, &payload)
}

pub(crate) fn save_learned_mapping(mapping: &mackes_config::LearnedMapping) -> String {
    let payload = serde_json::to_vec(&serde_json::json!({"learned_mappings": [mapping]}))
        .expect("learned mapping payload is serializable");
    daemon_request(mackes_ipc::Command::Configuration, &payload)
}

pub(crate) fn project_routes(editor: &mut mackes_tui::RoutingEditor, payload: &serde_json::Value) {
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

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(crate) fn synchronize_events(
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
        let daemon_assignment_active = payload
            .get("assignment_session")
            .and_then(|value| value.get("phase"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|phase| phase != "Idle");
        if let Some(action) = payload.get("ui_navigation").and_then(serde_json::Value::as_str) {
            if assignment_wizard.session.phase == mackes_ipc::AssignmentPhase::Idle
                && !daemon_assignment_active
            {
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
                let assignment_action = match action {
                    "up" => Some(mackes_ipc::AssignmentAction::Up),
                    "down" => Some(mackes_ipc::AssignmentAction::Down),
                    "left" => Some(mackes_ipc::AssignmentAction::Back),
                    "right" => Some(mackes_ipc::AssignmentAction::Enter),
                    _ => None,
                };
                if let Some(assignment_action) = assignment_action {
                    let request = assignment_wizard.request(assignment_action, None);
                    if let Ok(request_payload) = serde_json::to_vec(&request) {
                        let response =
                            daemon_request(mackes_ipc::Command::Assignment, &request_payload);
                        if let Ok(result) =
                            serde_json::from_str::<mackes_ipc::AssignmentResult>(&response)
                        {
                            assignment_wizard.reconcile(result);
                            *assignment_choices = mackes_tui::AssignmentChoiceBrowser::from_session(
                                &assignment_wizard.session,
                            );
                        }
                    }
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

pub(crate) fn print_daemon_command(command: mackes_ipc::Command) {
    let response = daemon_command(command);
    let unavailable = response.contains("\"ok\":false");
    println!("{response}");
    if unavailable {
        std::process::exit(2);
    }
}

pub(crate) fn reconcile_assignment_session(
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

pub(crate) fn send_sysex_cli(destination: &str, hex: &str) {
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

pub(crate) fn send_device_control_cli(
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

pub(crate) fn print_learn(endpoint: &str, limit: usize) {
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

pub(crate) fn dispatch_ui_command(command: mackes_tui::UiCommand) -> String {
    let Some(ipc_command) = mackes_tui::ipc_command_for(command) else {
        return "{\"ok\":false,\"error\":\"UI command is local-only\"}".to_owned();
    };
    daemon_command(ipc_command)
}

pub(crate) fn discovered_endpoints() -> Vec<String> {
    let response = daemon_request(mackes_ipc::Command::Endpoints, b"{}");
    serde_json::from_str::<serde_json::Value>(&response)
        .ok()
        .and_then(|value| value.get("endpoints").cloned())
        .and_then(|value| value.as_array().cloned())
        .map(|entries| {
            entries
                .into_iter()
                .filter_map(|entry| {
                    entry.get("name").and_then(serde_json::Value::as_str).map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn backup_entries(directory: &std::path::Path) -> Vec<String> {
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

pub(crate) const fn backup_status_label(status: &mackes_config::BackupStatus) -> &'static str {
    match status {
        mackes_config::BackupStatus::Verified => "verified",
        mackes_config::BackupStatus::SentUnverified => "sent_unverified",
        mackes_config::BackupStatus::Failed => "failed",
    }
}

pub(crate) fn restore_cli(backup: &str, target: &str, profile: &str, identity: &str, apply: bool) {
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

pub(crate) const fn restore_result_status(result: &mackes_config::RestoreResult) -> &'static str {
    match result {
        mackes_config::RestoreResult::Planned { status, .. }
        | mackes_config::RestoreResult::Applied { status, .. } => backup_status_label(status),
    }
}
