//! MACKES operator entry point.

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
    execute!(output, EnterAlternateScreen).map_err(|error| error.to_string())?;
    let backend = CrosstermBackend::new(output);
    let mut terminal = Terminal::new(backend).map_err(|error| error.to_string())?;
    let mut dashboard = mackes_tui::DashboardState::initial();
    let mut client_state = mackes_tui::ClientState::default();
    let learn_workspace = mackes_tui::LearnWorkspace::new();
    let reflex_workspace =
        mackes_tui::ReflexWorkspace::from_compiled_algorithm(1).map_err(str::to_owned)?;
    let eventide_workspace = mackes_tui::DeviceWorkspace::eventide_micropitch();
    let mut workspace = 1_u8;
    let mut needs_snapshot = true;
    let result = loop {
        let synchronized = if needs_snapshot {
            synchronize_snapshot(&mut client_state, &mut dashboard)
        } else {
            synchronize_events(&mut client_state, &mut dashboard)
        };
        if let Ok(health) = synchronized {
            dashboard.health = health;
            needs_snapshot = false;
        } else {
            "offline".clone_into(&mut dashboard.health);
            client_state.begin_reconnect();
            needs_snapshot = true;
        }
        terminal
            .draw(|frame| match workspace {
                2 => mackes_tui::draw_learn(frame, frame.area(), &learn_workspace),
                3 => mackes_tui::draw_reflex(frame, frame.area(), &reflex_workspace),
                4 => mackes_tui::draw_device(frame, frame.area(), &eventide_workspace),
                _ => mackes_tui::draw_dashboard(frame, frame.area(), &dashboard),
            })
            .map_err(|error| error.to_string())?;
        if event::poll(std::time::Duration::from_millis(250)).map_err(|error| error.to_string())? {
            if let Event::Key(key) = event::read().map_err(|error| error.to_string())? {
                match key.code {
                    KeyCode::Char('q') => break Ok(()),
                    KeyCode::Char(value) if ('1'..='4').contains(&value) => {
                        workspace = value
                            .to_digit(10)
                            .and_then(|number| u8::try_from(number).ok())
                            .unwrap_or(1);
                    }
                    KeyCode::Char('n' | 'p') => {
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
                    _ => {}
                }
            }
        }
    };
    disable_raw_mode().map_err(|error| error.to_string())?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(|error| error.to_string())?;
    terminal.show_cursor().map_err(|error| error.to_string())?;
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

#[allow(clippy::too_many_lines)]
fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.as_slice() {
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
                "mackes-midi-matrix: TUI/CLI\n\nUsage:\n  mackes-midi-matrix tui\n  mackes-midi-matrix validate <path> [--json]\n  mackes-midi-matrix export <config> <directory>\n  mackes-midi-matrix doctor [--json]\n  mackes-midi-matrix status [--json]\n  mackes-midi-matrix panic\n  mackes-midi-matrix endpoints [--json]\n  mackes-midi-matrix default get <config> <capability> [--json]\n  mackes-midi-matrix default set <config> <capability> <profile-id>\n  mackes-midi-matrix mvave preset <1-32> [--dry-run]\n  mackes-midi-matrix mvave ir|eq on|off --confirm-unverified\n  mackes-midi-matrix scenes|devices|routes|monitor [--json]\n  mackes-midi-matrix scene list <config> [--json]\n  mackes-midi-matrix backup list|inspect ...\n  mackes-midi-matrix profile validate [--json]\n  mackes-midi-matrix --version"
            );
            println!("  mackes-midi-matrix learn <endpoint-id> [limit]");
            println!("  mackes-midi-matrix scene next|previous");
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
            let payload = serde_json::to_vec(&serde_json::json!({"direction": direction}))
                .expect("scene navigation payload is serializable");
            let response = daemon_request(mackes_ipc::Command::Scenes, &payload);
            println!("{response}");
            if response.contains("\"ok\":false") {
                std::process::exit(2);
            }
        }
        [command] if command == "monitor" => {
            print_daemon_command(mackes_ipc::Command::Monitor);
        }
        [command] if command == "routes" => {
            print_daemon_command(mackes_ipc::Command::Routes);
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
            eprintln!("mackes-midi-matrix: invalid arguments\n\nUsage:\n  mackes-midi-matrix validate <path> [--json]\n  mackes-midi-matrix export <config> <directory>\n  mackes-midi-matrix doctor [--json]\n  mackes-midi-matrix status [--json]\n  mackes-midi-matrix panic\n  mackes-midi-matrix endpoints [--json]\n  mackes-midi-matrix profile validate [--json]\n  mackes-midi-matrix --help");
            eprintln!("  mackes-midi-matrix learn <endpoint-id> [limit]");
            std::process::exit(64);
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
        command: mackes_ipc::Command::Health,
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

fn synchronize_snapshot(
    state: &mut mackes_tui::ClientState,
    dashboard: &mut mackes_tui::DashboardState,
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

fn synchronize_events(
    state: &mut mackes_tui::ClientState,
    dashboard: &mut mackes_tui::DashboardState,
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
