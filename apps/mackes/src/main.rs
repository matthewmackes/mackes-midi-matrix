//! MACKES operator entry point.

mod cli;
mod interactive;

use cli::*;
use interactive::run_tui;

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
                "mackes-midi-matrix: TUI/CLI\n\nUsage:\n  mackes-midi-matrix tui\n  mackes-midi-matrix validate <path> [--json]\n  mackes-midi-matrix export <config> <directory>\n  mackes-midi-matrix doctor [--json]\n  mackes-midi-matrix status [--json]\n  mackes-midi-matrix panic\n  mackes-midi-matrix endpoints [--json]\n  mackes-midi-matrix default get <config> <capability> [--json]\n  mackes-midi-matrix default set <config> <capability> <profile-id>\n  mackes-midi-matrix reflex preset <id> --dry-run\n  mackes-midi-matrix reflex preset <id> <destination-id> --confirm\n  mackes-midi-matrix scenes|devices|routes|monitor [--json]\n  mackes-midi-matrix scene list <config> [--json]\n  mackes-midi-matrix backup list|inspect ...\n  mackes-midi-matrix profile validate [--json]\n  mackes-midi-matrix --version"
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

#[cfg(test)]
mod tests {
    use crate::cli::{parse_midi_hex, project_observability, project_routes, project_setlists};

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
