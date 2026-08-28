//! Persistent MACKES daemon entry point.

#[cfg(target_os = "linux")]
fn install_shutdown_signals() -> Result<std::sync::Arc<std::sync::atomic::AtomicBool>, String> {
    use std::sync::{atomic::AtomicBool, Arc};
    let shutdown = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&shutdown))
        .and_then(|_| {
            signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&shutdown))
        })
        .map(|_| shutdown)
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "linux")]
#[allow(clippy::too_many_lines)]
fn main() {
    use mackes_ipc::AccessPolicy;
    use mackesd::{Daemon, InstanceLock};
    use std::{env, fs, path::PathBuf, sync::atomic::Ordering};

    let mut socket = PathBuf::from("/run/mackes-midi-matrix/control.sock");
    let mut lock_path = PathBuf::from("/run/mackes-midi-matrix/mackes-midi-matrixd.lock");
    let mut config = PathBuf::from("/etc/mackes-midi-matrix/config.json5");
    let mut restore_degraded = false;
    let mut restored_scene = None;
    let mut dashboard_bindings = Vec::new();
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--socket" => {
                socket = PathBuf::from(args.next().unwrap_or_else(|| {
                    eprintln!("mackes-midi-matrixd: --socket requires a path");
                    std::process::exit(64);
                }));
            }
            "--lock" => {
                lock_path = PathBuf::from(args.next().unwrap_or_else(|| {
                    eprintln!("mackes-midi-matrixd: --lock requires a path");
                    std::process::exit(64);
                }));
            }
            "--config" => {
                config = PathBuf::from(args.next().unwrap_or_else(|| {
                    eprintln!("mackes-midi-matrixd: --config requires a path");
                    std::process::exit(64);
                }));
            }
            "--help" => {
                println!("mackes-midi-matrixd [--socket PATH] [--lock PATH] [--config PATH]");
                return;
            }
            other => {
                eprintln!("mackes-midi-matrixd: unknown argument {other}");
                std::process::exit(64);
            }
        }
    }
    if let Some(parent) = lock_path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            eprintln!("mackes-midi-matrixd: cannot create lock directory: {error}");
            std::process::exit(1);
        }
    }
    let _lock = match InstanceLock::acquire(&lock_path) {
        Ok(lock) => lock,
        Err(error) => {
            eprintln!("mackes-midi-matrixd: another instance owns the lock: {error}");
            std::process::exit(1);
        }
    };
    if let Some(parent) = socket.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            eprintln!("mackes-midi-matrixd: cannot create socket directory: {error}");
            std::process::exit(1);
        }
    }
    if config.exists() {
        match mackesd::startup_restore(&config) {
            Ok(result) => {
                restored_scene.clone_from(&result.active_scene);
                if let Ok(document) = mackes_config::load(&config) {
                    dashboard_bindings = document.settings.dashboard_midi_bindings;
                }
                let detail = format!(
                    "project={:?} scene={:?} scenes={} unsafe_actions_blocked={}",
                    result.active_project,
                    result.active_scene,
                    result.scenes.len(),
                    result.unsafe_actions_blocked
                );
                eprint!("{}", mackesd::structured_log_line("info", "startup_restore", &detail));
            }
            Err(error) => {
                eprint!(
                    "{}",
                    mackesd::structured_log_line(
                        "error",
                        "startup_restore_rejected",
                        &error.to_string()
                    )
                );
                restore_degraded = true;
            }
        }
    }
    let mut daemon = match Daemon::bind(&socket) {
        Ok(daemon) => daemon,
        Err(error) => {
            eprintln!("mackes-midi-matrixd: cannot bind {}: {error}", socket.display());
            std::process::exit(1);
        }
    };
    daemon.set_config_path(&config);
    if let Err(error) = daemon.set_nonblocking(true) {
        eprintln!("mackes-midi-matrixd: cannot configure nonblocking control socket: {error}");
        std::process::exit(1);
    }
    if restore_degraded {
        daemon.mark_degraded();
    }
    if let Ok(document) = mackes_config::load(&config) {
        let learn_endpoint_id = document
            .settings
            .learn_input_alias
            .as_deref()
            .and_then(|alias_id| document.endpoints.iter().find(|alias| alias.id == alias_id))
            .and_then(|alias| alias.name.as_deref())
            .and_then(|pattern| {
                mackes_midi_engine::enumerate_midir_ports().ok().and_then(|ports| {
                    ports
                        .into_iter()
                        .find(|port| {
                            port.direction == mackes_midi_engine::EndpointDirection::Input
                                && port.name.contains(pattern)
                        })
                        .map(|port| port.id)
                })
            });
        let catalog = serde_json::json!({
            "projects": document.projects.iter().map(|project| serde_json::json!({
                "id": project.id,
                "scenes": project.scenes.iter().map(|scene| scene.id.clone()).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "setlists": document.setlists,
            "learned_mappings": document.learned_mappings,
            "learn_input_alias": document.settings.learn_input_alias,
            "learn_endpoint_id": learn_endpoint_id,
        });
        daemon.set_catalog(catalog);
    }
    daemon.set_active_scene(restored_scene);
    let mut persisted_scene = daemon.active_scene().map(str::to_owned);
    if let Ok(document) = mackes_config::load(&config) {
        let scene_ids = document
            .settings
            .active_project
            .as_deref()
            .and_then(|project_id| {
                document.projects.iter().find(|project| project.id == project_id)
            })
            .map(|project| project.scenes.iter().map(|scene| scene.id.clone()).collect())
            .unwrap_or_default();
        daemon.set_scene_ids(scene_ids);
    }
    if let Err(error) = daemon.enable_virtual_ports() {
        eprintln!("mackes-midi-matrixd: virtual ALSA ports unavailable: {error}");
    }
    let daemon_uid = std::process::Command::new("id")
        .args(["-u", "mackes"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|value| value.trim().parse().ok())
        // Root remains an explicit recovery/installation operator when the
        // system account has not been created yet.
        .unwrap_or(0);
    let control_gid = std::process::Command::new("getent")
        .args(["group", "mackes-control"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|value| value.split(':').nth(2)?.parse().ok())
        .unwrap_or(0);
    let policy = AccessPolicy { control_gid, daemon_uid };
    let shutdown = match install_shutdown_signals() {
        Ok(shutdown) => shutdown,
        Err(error) => {
            eprint!("{}", mackesd::structured_log_line("error", "signal_handlers", &error));
            std::process::exit(1);
        }
    };
    loop {
        if shutdown.load(Ordering::Relaxed) {
            daemon.request_shutdown();
            break;
        }
        let mapped = daemon.process_dashboard_commands(&dashboard_bindings, 128);
        if !mapped.is_empty() {
            eprint!(
                "{}",
                mackesd::structured_log_line(
                    "info",
                    "mapped_dashboard_actions",
                    &format!("count={}", mapped.len()),
                )
            );
        }
        if let Err(error) = daemon.serve_once(policy) {
            if error.kind() != std::io::ErrorKind::WouldBlock {
                eprint!(
                    "{}",
                    mackesd::structured_log_line("error", "request_failed", &error.to_string())
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let current_scene = daemon.active_scene().map(str::to_owned);
        if current_scene != persisted_scene {
            match mackesd::persist_active_scene(&config, current_scene.as_deref()) {
                Ok(()) => persisted_scene = current_scene,
                Err(error) => eprint!(
                    "{}",
                    mackesd::structured_log_line("error", "active_scene_persist", &error)
                ),
            }
        }
        if daemon.health() == mackesd::Health::Stopping {
            break;
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("mackes-midi-matrixd: this daemon currently requires Linux");
    std::process::exit(78);
}
