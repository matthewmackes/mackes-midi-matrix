//! Small same-origin HTTP adapter forwarding authoritative reads to the daemon.

use mackes_ipc::{Command, Envelope, LocalClient, ProtocolVersion, RequestId};
use mackes_web_contract::{HttpRequest, HttpResponse, OperationRequest};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    thread,
    time::{Duration, Instant},
};

const DEFAULT_BIND: &str = "0.0.0.0:8081";
const DEFAULT_SOCKET: &str = "/run/mackes-midi-matrix/control.sock";
const DEFAULT_ORIGIN: &str = "http://localhost:8081";
const CLIENT_IO_TIMEOUT: Duration = Duration::from_secs(5);
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);
static OPERATION_CACHE: OnceLock<Mutex<VecDeque<(String, String, HttpResponse)>>> = OnceLock::new();
type ReadCacheEntry = (String, String, Instant, Vec<u8>);
static READ_CACHE: OnceLock<Mutex<Vec<ReadCacheEntry>>> = OnceLock::new();
static READ_FLIGHT: OnceLock<Mutex<()>> = OnceLock::new();
const OPERATION_CACHE_CAPACITY: usize = 256;
const OPERATION_CACHE_PATH: &str = "/var/lib/mackes-midi-matrix/web-operation-cache.json";
const READ_CACHE_TTL: Duration = Duration::from_millis(100);

fn main() {
    let (bind, socket, origin) = arguments();
    let listener = TcpListener::bind(&bind).unwrap_or_else(|error| {
        eprintln!("mackes-web: cannot bind {bind}: {error}");
        std::process::exit(1);
    });
    eprintln!("mackes-web: listening on {bind}");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let socket = socket.clone();
                let origin = origin.clone();
                thread::spawn(move || serve(stream, &socket, &origin));
            }
            Err(error) => eprintln!("mackes-web: accept failed: {error}"),
        }
    }
}

fn arguments() -> (String, PathBuf, String) {
    let mut bind = std::env::var("MACKES_WEB_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_owned());
    let mut socket = std::env::var_os("MACKES_SOCKET")
        .map_or_else(|| PathBuf::from(DEFAULT_SOCKET), PathBuf::from);
    let mut origin =
        std::env::var("MACKES_WEB_ORIGIN").unwrap_or_else(|_| DEFAULT_ORIGIN.to_owned());
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--bind" => bind = args.next().unwrap_or_else(|| usage("--bind requires an address")),
            "--socket" => {
                socket =
                    PathBuf::from(args.next().unwrap_or_else(|| usage("--socket requires a path")));
            }
            "--origin" => origin = args.next().unwrap_or_else(|| usage("--origin requires a URL")),
            "--help" => {
                println!("mackes-web [--bind ADDRESS] [--socket PATH] [--origin URL]");
                std::process::exit(0);
            }
            _ => usage("unknown argument"),
        }
    }
    (bind, socket, origin)
}

fn usage(message: &str) -> ! {
    eprintln!("mackes-web: {message}");
    eprintln!("usage: mackes-web [--bind ADDRESS] [--socket PATH] [--origin URL]");
    std::process::exit(2);
}

fn cached_read(socket: &Path, command: Command) -> Option<Vec<u8>> {
    if !matches!(
        command,
        Command::DeviceQuery
            | Command::Endpoints
            | Command::Health
            | Command::Mappings
            | Command::Snapshot
    ) {
        return None;
    }
    let key = socket.display().to_string();
    let command = format!("{command:?}");
    let cache = READ_CACHE.get_or_init(|| Mutex::new(Vec::new()));
    let mut entries = cache.lock().ok()?;
    entries.retain(|(_, tag, created, _)| {
        let ttl = if tag == "Mappings" { Duration::from_secs(2) } else { READ_CACHE_TTL };
        created.elapsed() < ttl
    });
    entries
        .iter()
        .find(|(path, tag, _, _)| path == &key && tag == &command)
        .map(|(_, _, _, body)| body.clone())
}

fn store_read(socket: &Path, command: Command, body: &[u8]) {
    if !matches!(
        command,
        Command::DeviceQuery
            | Command::Endpoints
            | Command::Health
            | Command::Mappings
            | Command::Snapshot
    ) {
        return;
    }
    let key = socket.display().to_string();
    let tag = format!("{command:?}");
    let cache = READ_CACHE.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(mut entries) = cache.lock() {
        entries.retain(|(path, command, created, _)| {
            let ttl = if command == "Mappings" { Duration::from_secs(2) } else { READ_CACHE_TTL };
            created.elapsed() < ttl && !(path == &key && command == &tag)
        });
        entries.push((key, tag, Instant::now(), body.to_vec()));
    }
}

fn serve(mut stream: TcpStream, socket: &PathBuf, origin: &str) {
    let _ = stream.set_read_timeout(Some(CLIENT_IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(CLIENT_IO_TIMEOUT));
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let count = match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(count) => count,
        };
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if bytes.len() > mackes_web_contract::MAX_HTTP_HEADER_BYTES {
            write_response(
                &mut stream,
                &HttpResponse::new(413, b"request too large".to_vec(), false).expect("response"),
            );
            return;
        }
    }
    let header_end =
        bytes.windows(4).position(|window| window == b"\r\n\r\n").map(|offset| offset + 4);
    let Some(header_end) = header_end else { return };
    let content_length = header_value(&bytes[..header_end], "content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    if content_length > mackes_web_contract::MAX_HTTP_BODY_BYTES {
        write_response(
            &mut stream,
            &HttpResponse::new(413, b"request too large".to_vec(), false).expect("response"),
        );
        return;
    }
    while bytes.len() < header_end + content_length {
        let count = match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(count) => count,
        };
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.len() > header_end + mackes_web_contract::MAX_HTTP_BODY_BYTES {
            write_response(
                &mut stream,
                &HttpResponse::new(413, b"request too large".to_vec(), false).expect("response"),
            );
            return;
        }
    }
    let request = match HttpRequest::parse(&bytes[..header_end + content_length]) {
        Ok(request) => request,
        Err(error) => {
            write_response(
                &mut stream,
                &HttpResponse::new(400, error.as_bytes().to_vec(), false).expect("response"),
            );
            return;
        }
    };
    if request.method == "GET" && is_event_stream_path(&request.path) {
        stream_events(stream, &request, socket, origin);
        return;
    }
    let response = route(&request, socket, origin);
    write_response(&mut stream, &response);
}

fn is_event_stream_path(path: &str) -> bool {
    path == "/api/v1/events/stream" || path.starts_with("/api/v1/events/stream?")
}

fn stream_events(mut stream: TcpStream, request: &HttpRequest, socket: &PathBuf, origin: &str) {
    let expected_host = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
        .unwrap_or(origin);
    if request.validate_same_origin(expected_host, origin).is_err() {
        write_response(
            &mut stream,
            &HttpResponse::new(400, b"origin validation failed".to_vec(), false).expect("response"),
        );
        return;
    }
    let poll_path = request.path.replacen("/api/v1/events/stream", "/api/v1/events", 1);
    let mut cursor = match event_cursor(&poll_path) {
        Ok(cursor) => cursor,
        Err(error) => {
            write_response(
                &mut stream,
                &HttpResponse::new(400, error.as_bytes().to_vec(), false).expect("response"),
            );
            return;
        }
    };
    if !poll_path.contains('?') {
        if let Some(last_event_id) = request
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("last-event-id"))
            .and_then(|(_, value)| value.parse::<u64>().ok())
        {
            cursor = last_event_id;
        }
    }
    let _ = stream.set_write_timeout(Some(CLIENT_IO_TIMEOUT));
    if stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n")
        .is_err()
    {
        return;
    }
    // The client reconnects with the last sequence after this bounded session.
    for _ in 0..300 {
        let Ok(poll_request) = HttpRequest::parse(
            format!(
                "GET /api/v1/events?after_sequence={cursor} HTTP/1.1\r\nHost: {expected_host}\r\nOrigin: {origin}\r\n\r\n"
            )
            .as_bytes(),
        ) else { return };
        let response = route(&poll_request, socket, origin);
        if response.status != 200 {
            let _ = stream.write_all(
                format!("event: error\ndata: {}\n\n", String::from_utf8_lossy(&response.body))
                    .as_bytes(),
            );
            return;
        }
        let Ok(body) = serde_json::from_slice::<serde_json::Value>(&response.body) else { return };
        if body.get("snapshot_required").and_then(serde_json::Value::as_bool) == Some(true) {
            let _ = stream.write_all(
                format!("event: resnapshot\ndata: {}\n\n", response_body(&body)).as_bytes(),
            );
            return;
        }
        if let Some(events) = body.get("events").and_then(serde_json::Value::as_array) {
            for event in events {
                let sequence =
                    event.get("sequence").and_then(serde_json::Value::as_u64).unwrap_or(cursor);
                cursor = cursor.max(sequence);
                if stream
                    .write_all(
                        format!("id: {sequence}\ndata: {}\n\n", response_body(event)).as_bytes(),
                    )
                    .is_err()
                {
                    return;
                }
            }
        }
        if stream.write_all(b": heartbeat\n\n").is_err() {
            return;
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn response_body(value: &serde_json::Value) -> String {
    value.to_string()
}

#[allow(clippy::too_many_lines)]
fn route(request: &HttpRequest, socket: &PathBuf, origin: &str) -> HttpResponse {
    if request.method == "OPTIONS" {
        return HttpResponse::new(204, Vec::new(), false).expect("response");
    }
    let expected_host = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
        .unwrap_or(origin);
    if let Err(error) = request.validate_same_origin(expected_host, origin) {
        return HttpResponse::new(400, error.as_bytes().to_vec(), false).expect("response");
    }
    let command = match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => {
            return HttpResponse::asset(
                200,
                include_bytes!("../static/index.html").to_vec(),
                "text/html; charset=utf-8",
            )
            .expect("bounded index response");
        }
        (
            "GET",
            "/state"
            | "/mappings"
            | "/routes"
            | "/scenes"
            | "/devices"
            | "/devices/novation"
            | "/recovery"
            | "/system"
            | "/system/configuration"
            | "/system/configuration/raw"
            | "/system/backups"
            | "/system/diagnostics"
            | "/monitor",
        ) => {
            return HttpResponse::asset(
                200,
                include_bytes!("../static/index.html").to_vec(),
                "text/html; charset=utf-8",
            )
            .expect("bounded deep-link response");
        }
        ("GET", "/api/v1/capabilities") => {
            return HttpResponse::new(
                200,
                serde_json::json!({
                    "schema_version": 1,
                    "generation": 0,
                    "capabilities": [
                        {"id": "configuration", "state": "available", "operations": ["read", "draft", "validate", "diff", "apply", "operation"]},
                        {"id": "events", "state": "available", "operations": ["poll", "stream"]},
                        {"id": "health", "state": "available", "operations": ["read"]},
                        {"id": "state", "state": "available", "operations": ["read"]},
                        {"id": "endpoints", "state": "available", "operations": ["read"]},
                        {"id": "routes", "state": "available", "operations": ["read", "apply"]},
                        {"id": "scenes", "state": "available", "operations": ["read", "apply"]},
                        {"id": "devices", "state": "available", "operations": ["read"]},
                        {"id": "assignment", "state": "available", "operations": ["read", "apply"]},
                        {"id": "mappings", "state": "available", "operations": ["read", "apply"]},
                        {"id": "pipedal", "state": "degraded", "operations": ["read", "apply"]},
                        {"id": "monitor", "state": "available", "operations": ["read"]},
                        {"id": "validation", "state": "available", "operations": ["read"]},
                        {"id": "diagnostics", "state": "available", "operations": ["read"]}
                    ],
                    "api": "v1",
                    "port": 8081,
                    "reads": ["health", "state", "capabilities", "endpoints", "routes", "scenes", "devices", "novation", "assignment", "mappings", "pipedal", "monitor", "backups", "configuration", "validation", "diagnostics", "diagnostics_bundle"],
                    "operations": {"rescan": "implemented", "panic": "implemented_with_confirmation", "device_control": "implemented_with_confirmation", "sysex": "implemented_with_confirmation", "assignment": "implemented_as_typed_ipc", "mappings": "implemented_as_typed_ipc", "routes": "implemented_as_daemon_validated", "scenes": "implemented_as_daemon_validated", "configuration": "implemented_as_bounded_daemon_ipc", "pipedal": "implemented_as_typed_ipc", "events": "implemented_as_poll_and_sse"},
                    "unsupported": {"remaining_mutations": "W138-W139"}
                })
                .to_string()
                .into_bytes(),
                true,
            )
            .expect("bounded capability response");
        }
        ("GET", "/api/v1/diagnostics") => {
            return HttpResponse::new(
                200,
                serde_json::json!({
                    "web": {"version": env!("CARGO_PKG_VERSION"), "api": "v1", "port": 8081, "authentication": "none"},
                    "service": {"bind": "MACKES_WEB_BIND (default 0.0.0.0:8081)", "origin": origin, "preview_url": origin, "restart_required_after_bind_change": true, "unit": "mackes-web.service", "restart": "always with 3s backoff", "boot_enabled_by": "systemd install workflow"},
                    "daemon_ipc": {"socket": socket.display().to_string(), "health_route": "/api/v1/health"},
                    "recovery_catalog": {"port_conflict": "change MACKES_WEB_BIND and restart mackes-web.service", "missing_device": "inspect /api/v1/novation then run rescan", "disk_full": "free space and retry the daemon-owned save", "malformed_config": "validate configuration and restore a verified backup", "permission": "repair service-account ownership and permissions"},
                    "limitations": ["daemon health is queried separately", "long-lived event streaming is not enabled"]
                })
                .to_string()
                .into_bytes(),
                true,
            )
            .expect("bounded diagnostics response");
        }
        ("GET", "/api/v1/diagnostics/bundle") => {
            return HttpResponse::new(
                200,
                serde_json::json!({
                    "format": "mackes-diagnostics-v1",
                    "inventory": ["web_build", "api_version", "bind_policy", "ipc_socket", "health_route", "recovery_catalog", "limitations"],
                    "web": {"version": env!("CARGO_PKG_VERSION"), "api": "v1", "port": 8081, "authentication": "none"},
                    "service": {"bind": "MACKES_WEB_BIND (default 0.0.0.0:8081)", "origin": origin, "preview_url": origin, "restart_required_after_bind_change": true, "unit": "mackes-web.service", "restart": "always with 3s backoff", "boot_enabled_by": "systemd install workflow"},
                    "ipc": {"socket": socket.display().to_string(), "health_route": "/api/v1/health"},
                    "recovery_catalog": {"port_conflict": "change MACKES_WEB_BIND and restart mackes-web.service", "missing_device": "inspect /api/v1/novation then run rescan", "disk_full": "free space and retry the daemon-owned save", "malformed_config": "validate configuration and restore a verified backup", "permission": "repair service-account ownership and permissions"},
                    "limitations": ["daemon health and host logs are not collected by this bounded bundle", "long-lived event streaming is not enabled"]
                })
                .to_string()
                .into_bytes(),
                true,
            )
            .expect("bounded diagnostics bundle response");
        }
        ("GET", "/api/v1/novation") => return novation_snapshot(socket),
        ("GET", "/api/v1/rtp") => return rtp_snapshot(socket),
        ("GET", "/favicon.ico") => {
            return HttpResponse::asset(
                200,
                br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect width="16" height="16" rx="3" fill="#161616"/><path d="M3 4h10v2H9v6H7V6H3z" fill="#78a9ff"/></svg>"##.to_vec(),
                "image/svg+xml",
            )
            .expect("bounded favicon response");
        }
        ("GET", "/assets/app.js") => {
            return HttpResponse::asset(
                200,
                include_bytes!("../static/app.js").to_vec(),
                "text/javascript; charset=utf-8",
            )
            .expect("bounded script response");
        }
        ("GET", "/assets/navigation.js") => {
            return HttpResponse::asset(
                200,
                include_bytes!("../static/navigation.js").to_vec(),
                "text/javascript; charset=utf-8",
            )
            .expect("bounded navigation script response");
        }
        ("GET", "/assets/health.js") => {
            return HttpResponse::asset(
                200,
                include_bytes!("../static/health.js").to_vec(),
                "text/javascript; charset=utf-8",
            )
            .expect("bounded health script response");
        }
        ("GET", "/assets/feature_catalog.js") => {
            return HttpResponse::asset(
                200,
                include_bytes!("../static/feature_catalog.js").to_vec(),
                "text/javascript; charset=utf-8",
            )
            .expect("bounded feature catalog script response");
        }
        ("GET", "/assets/feature_renderer.js") => {
            return HttpResponse::asset(
                200,
                include_bytes!("../static/feature_renderer.js").to_vec(),
                "text/javascript; charset=utf-8",
            )
            .expect("bounded feature renderer script response");
        }
        ("GET", "/assets/device_renderer.js") => {
            return HttpResponse::asset(
                200,
                include_bytes!("../static/device_renderer.js").to_vec(),
                "text/javascript; charset=utf-8",
            )
            .expect("bounded device renderer script response");
        }
        ("GET", "/assets/state_store.js") => {
            return HttpResponse::asset(
                200,
                include_bytes!("../static/state_store.js").to_vec(),
                "text/javascript; charset=utf-8",
            )
            .expect("bounded state store script response");
        }
        ("GET", "/assets/app.css") => {
            return HttpResponse::asset(
                200,
                include_bytes!("../static/app.css").to_vec(),
                "text/css; charset=utf-8",
            )
            .expect("bounded stylesheet response");
        }
        ("GET", "/api/v1/state") => Command::Snapshot,
        ("GET", "/api/v1/health") => Command::Health,
        ("GET", "/api/v1/endpoints") => Command::Endpoints,
        ("GET", "/api/v1/routes") => Command::Routes,
        ("POST", "/api/v1/routes") => return routes_operation(request, socket),
        ("GET", "/api/v1/scenes") => Command::Scenes,
        ("POST", "/api/v1/scenes") => return scenes_operation(request, socket),
        ("GET", "/api/v1/devices") => Command::DeviceQuery,
        ("GET", "/api/v1/assignment") => Command::Assignment,
        ("POST", "/api/v1/assignment") => return assignment_operation(request, socket),
        ("GET", "/api/v1/monitor") => Command::Monitor,
        ("GET", "/api/v1/backups") => Command::Backups,
        ("POST", "/api/v1/backups") => return backup_operation(request, socket),
        ("GET", "/api/v1/configuration") => Command::Configuration,
        ("POST", "/api/v1/configuration") => return configuration_operation(request, socket),
        ("GET", "/api/v1/validation") => Command::Validate,
        ("GET", "/api/v1/mappings") => Command::Mappings,
        ("GET", "/api/v1/pipedal") => return pipedal_snapshot(socket),
        ("POST", "/api/v1/mappings") => return mapping_operation(request, socket),
        ("POST", "/api/v1/sysex") => return sysex_operation(request, socket),
        ("POST", "/api/v1/pipedal") => return pipedal_operation(request, socket),
        ("POST", "/api/v1/operations") => return operation(request, socket),
        ("GET", path) if path == "/api/v1/events" || path.starts_with("/api/v1/events?") => {
            Command::Subscribe
        }
        _ => return HttpResponse::new(404, b"not found".to_vec(), false).expect("response"),
    };
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed).max(1);
    // Keep the read-only browser projection aligned with the shared AssignmentAction::Snapshot
    // contract; the web adapter deliberately forwards the bounded JSON request over IPC.
    let payload = if command == Command::Assignment {
        serde_json::json!({"generation": 0, "action": "Snapshot"}).to_string().into_bytes()
    } else if command == Command::Subscribe {
        let after_sequence = match event_cursor(&request.path) {
            Ok(value) => value,
            Err(error) => {
                return HttpResponse::new(400, error.as_bytes().to_vec(), false)
                    .expect("bounded cursor error");
            }
        };
        serde_json::json!({"after_sequence": after_sequence}).to_string().into_bytes()
    } else {
        b"{}".to_vec()
    };
    let envelope = Envelope {
        version: ProtocolVersion::current(),
        request_id: RequestId::new(request_id).expect("nonzero request ID"),
        command,
        payload,
    };
    let _read_flight = if matches!(
        command,
        Command::DeviceQuery
            | Command::Endpoints
            | Command::Health
            | Command::Mappings
            | Command::Snapshot
    ) {
        Some(READ_FLIGHT.get_or_init(|| Mutex::new(())).lock().expect("read flight lock"))
    } else {
        None
    };
    if let Some(body) = cached_read(socket, command) {
        return HttpResponse::new(200, body, true).expect("bounded cached daemon response");
    }
    let policy =
        mackes_ipc::ReconnectPolicy::new(3, Duration::from_millis(25), Duration::from_millis(250))
            .expect("valid reconnect policy");
    match LocalClient::request_with_policy(socket, policy, &envelope) {
        Ok((body, _)) => {
            store_read(socket, command, &body);
            HttpResponse::new(200, body, true).expect("bounded daemon response")
        }
        Err(error) => HttpResponse::new(
            503,
            serde_json::json!({"code":"daemon_unavailable","message":error})
                .to_string()
                .into_bytes(),
            true,
        )
        .expect("bounded unavailable response"),
    }
}

fn mapping_operation(request: &HttpRequest, socket: &PathBuf) -> HttpResponse {
    let json_content_type =
        request.headers.iter().find(|(name, _)| name == "content-type").is_some_and(
            |(_, value)| {
                value.split(';').next().is_some_and(|media_type| {
                    media_type.trim().eq_ignore_ascii_case("application/json")
                })
            },
        );
    if !json_content_type {
        return HttpResponse::new(415, br#"{"code":"unsupported_media_type"}"#.to_vec(), true)
            .expect("bounded media-type response");
    }
    let mapping = match serde_json::from_slice::<mackes_ipc::MappingRequest>(&request.body)
        .map_err(|_| "invalid mapping request".to_owned())
        .and_then(|request| request.validate().map_err(str::to_owned))
    {
        Ok(mapping) => mapping,
        Err(error) => {
            return HttpResponse::new(400, error.into_bytes(), false)
                .expect("bounded error response");
        }
    };
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed).max(1);
    let envelope = Envelope {
        version: ProtocolVersion::current(),
        request_id: RequestId::new(request_id).expect("nonzero request ID"),
        command: Command::Mappings,
        payload: serde_json::to_vec(&mapping).expect("typed mapping request serializes"),
    };
    let policy =
        mackes_ipc::ReconnectPolicy::new(3, Duration::from_millis(25), Duration::from_millis(250))
            .expect("valid reconnect policy");
    match LocalClient::request_with_policy(socket, policy, &envelope) {
        Ok((body, _)) => {
            let status =
                serde_json::from_slice::<mackes_ipc::MappingResult>(&body).map_or(502, |result| {
                    match result.outcome {
                        mackes_ipc::MappingOutcome::Applied => 200,
                        mackes_ipc::MappingOutcome::GenerationConflict
                        | mackes_ipc::MappingOutcome::Conflict
                        | mackes_ipc::MappingOutcome::PersistenceFailed
                        | mackes_ipc::MappingOutcome::Invalid
                        | mackes_ipc::MappingOutcome::NothingToUndo => 409,
                    }
                });
            HttpResponse::new(status, body, true).expect("bounded mapping response")
        }
        Err(error) => HttpResponse::new(
            503,
            serde_json::json!({"code":"daemon_unavailable","message":error})
                .to_string()
                .into_bytes(),
            true,
        )
        .expect("bounded unavailable response"),
    }
}

fn sysex_operation(request: &HttpRequest, socket: &PathBuf) -> HttpResponse {
    let json_content_type =
        request.headers.iter().find(|(name, _)| name == "content-type").is_some_and(
            |(_, value)| {
                value
                    .split(';')
                    .next()
                    .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("application/json"))
            },
        );
    if !json_content_type {
        return HttpResponse::new(415, br#"{"code":"unsupported_media_type"}"#.to_vec(), true)
            .expect("bounded media-type response");
    }
    let value = match serde_json::from_slice::<serde_json::Value>(&request.body) {
        Ok(value) if value.is_object() => value,
        _ => {
            return HttpResponse::new(400, br#"{"code":"invalid_request"}"#.to_vec(), true)
                .expect("bounded error response")
        }
    };
    let destination = value.get("destination").and_then(serde_json::Value::as_str);
    let bytes = value.get("bytes").and_then(serde_json::Value::as_array);
    let valid_bytes = bytes.is_some_and(|bytes| {
        (1..=1024).contains(&bytes.len())
            && bytes.iter().all(|byte| byte.as_u64().is_some_and(|value| value <= 127))
    });
    if value.get("confirm").and_then(serde_json::Value::as_bool) != Some(true)
        || destination.is_none_or(|value| value.is_empty() || value.len() > 128)
        || !valid_bytes
    {
        return HttpResponse::new(400, br#"{"code":"invalid_sysex_request"}"#.to_vec(), true)
            .expect("bounded error response");
    }
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed).max(1);
    let envelope = Envelope {
        version: ProtocolVersion::current(),
        request_id: RequestId::new(request_id).expect("nonzero request ID"),
        command: Command::Sysex,
        payload: serde_json::to_vec(&value).expect("SysEx payload serializes"),
    };
    let policy =
        mackes_ipc::ReconnectPolicy::new(3, Duration::from_millis(25), Duration::from_millis(250))
            .expect("valid reconnect policy");
    match LocalClient::request_with_policy(socket, policy, &envelope) {
        Ok((body, _)) => HttpResponse::new(200, body, true).expect("bounded SysEx response"),
        Err(error) => HttpResponse::new(
            503,
            serde_json::json!({"code":"daemon_unavailable","message":error})
                .to_string()
                .into_bytes(),
            true,
        )
        .expect("bounded unavailable response"),
    }
}

#[allow(clippy::too_many_lines)]
fn configuration_operation(request: &HttpRequest, socket: &PathBuf) -> HttpResponse {
    let json = request.headers.iter().find(|(name, _)| name == "content-type").is_some_and(
        |(_, value)| {
            value
                .split(';')
                .next()
                .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("application/json"))
        },
    );
    if !json || request.body.len() > 1024 * 1024 {
        return HttpResponse::new(
            400,
            br#"{"code":"invalid_configuration_request"}"#.to_vec(),
            true,
        )
        .expect("bounded configuration error");
    }
    let mut value = match serde_json::from_slice::<serde_json::Value>(&request.body) {
        Ok(value) if value.is_object() => value,
        _ => {
            return HttpResponse::new(
                400,
                br#"{"code":"invalid_configuration_request"}"#.to_vec(),
                true,
            )
            .expect("bounded configuration error")
        }
    };
    let operation = value.get("operation").and_then(serde_json::Value::as_str).unwrap_or("apply");
    if !matches!(operation, "draft" | "validate" | "diff" | "apply") {
        return HttpResponse::new(
            400,
            br#"{"code":"invalid_configuration_operation"}"#.to_vec(),
            true,
        )
        .expect("bounded configuration operation error");
    }
    for field in ["draft_id", "operation_id"] {
        if let Some(value) = value.get(field) {
            if !value.is_string()
                || value.as_str().is_some_and(|value| value.is_empty() || value.len() > 128)
            {
                return HttpResponse::new(
                    400,
                    br#"{"code":"invalid_configuration_identifier"}"#.to_vec(),
                    true,
                )
                .expect("bounded configuration identifier error");
            }
        }
    }
    let has_document =
        value.get("configuration").is_some() || value.get("configuration_json5").is_some();
    let has_legacy_patch =
        value.get("setlists").is_some() || value.get("learned_mappings").is_some();
    if (operation == "apply"
        && value.get("confirm").and_then(serde_json::Value::as_bool) != Some(true))
        || (!has_document && !has_legacy_patch)
    {
        return HttpResponse::new(
            400,
            br#"{"code":"configuration_confirmation_required"}"#.to_vec(),
            true,
        )
        .expect("bounded configuration confirmation error");
    }
    if let Some(revision) = value.get("configuration_revision") {
        if !revision.is_string() || revision.as_str().is_some_and(|value| value.len() > 128) {
            return HttpResponse::new(
                400,
                br#"{"code":"invalid_configuration_revision"}"#.to_vec(),
                true,
            )
            .expect("bounded configuration revision error");
        }
    }
    value.as_object_mut().expect("configuration object").remove("confirm");
    let envelope = Envelope {
        version: ProtocolVersion::current(),
        request_id: RequestId::new(NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed).max(1))
            .expect("nonzero request ID"),
        command: Command::Configuration,
        payload: serde_json::to_vec(&value).expect("configuration payload serializes"),
    };
    let policy =
        mackes_ipc::ReconnectPolicy::new(3, Duration::from_millis(25), Duration::from_millis(250))
            .expect("valid reconnect policy");
    match LocalClient::request_with_policy(socket, policy, &envelope) {
        Ok((body, _)) => {
            let parsed = serde_json::from_slice::<serde_json::Value>(&body).ok();
            let status = parsed
                .as_ref()
                .and_then(|value| {
                    value.get("ok").and_then(serde_json::Value::as_bool).map(|ok| {
                        if ok {
                            200
                        } else {
                            let error = value
                                .get("error")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default();
                            if error.contains("concurrent")
                                || error.contains("revision")
                                || error.contains("conflict")
                            {
                                409
                            } else {
                                422
                            }
                        }
                    })
                })
                .unwrap_or(502);
            HttpResponse::new(status, body, true).expect("bounded configuration response")
        }
        Err(error) => HttpResponse::new(
            503,
            serde_json::json!({"code":"daemon_unavailable","message":error})
                .to_string()
                .into_bytes(),
            true,
        )
        .expect("bounded unavailable response"),
    }
}

fn backup_operation(request: &HttpRequest, socket: &PathBuf) -> HttpResponse {
    let json = request.headers.iter().find(|(name, _)| name == "content-type").is_some_and(
        |(_, value)| {
            value
                .split(';')
                .next()
                .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("application/json"))
        },
    );
    if !json {
        return HttpResponse::new(415, br#"{"code":"unsupported_media_type"}"#.to_vec(), true)
            .expect("bounded media response");
    }
    let value = match serde_json::from_slice::<serde_json::Value>(&request.body) {
        Ok(value)
            if matches!(
                value.get("action").and_then(serde_json::Value::as_str),
                Some("export" | "portable_export")
            ) || (matches!(
                value.get("action").and_then(serde_json::Value::as_str),
                Some("create" | "restore")
            ) && value.get("confirm").and_then(serde_json::Value::as_bool)
                == Some(true)
                || (value.get("action").and_then(serde_json::Value::as_str)
                    == Some("portable_import")
                    && value.get("confirm").and_then(serde_json::Value::as_bool)
                        == Some(true)
                    && value.get("content").and_then(serde_json::Value::as_str).is_some_and(
                        |content| !content.is_empty() && content.len() <= 1024 * 1024,
                    ))) =>
        {
            value
        }
        _ => {
            return HttpResponse::new(400, br#"{"code":"invalid_backup_request"}"#.to_vec(), true)
                .expect("bounded backup error")
        }
    };
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed).max(1);
    let envelope = Envelope {
        version: ProtocolVersion::current(),
        request_id: RequestId::new(request_id).expect("nonzero request ID"),
        command: Command::Backups,
        payload: serde_json::to_vec(&value).expect("backup request serializes"),
    };
    let policy =
        mackes_ipc::ReconnectPolicy::new(3, Duration::from_millis(25), Duration::from_millis(250))
            .expect("valid reconnect policy");
    match LocalClient::request_with_policy(socket, policy, &envelope) {
        Ok((body, _)) => HttpResponse::new(200, body, true).expect("bounded backup response"),
        Err(error) => HttpResponse::new(
            503,
            serde_json::json!({"code":"daemon_unavailable","message":error})
                .to_string()
                .into_bytes(),
            true,
        )
        .expect("bounded unavailable response"),
    }
}

fn pipedal_operation(request: &HttpRequest, socket: &PathBuf) -> HttpResponse {
    let json_content_type =
        request.headers.iter().find(|(name, _)| name == "content-type").is_some_and(
            |(_, value)| {
                value.split(';').next().is_some_and(|media_type| {
                    media_type.trim().eq_ignore_ascii_case("application/json")
                })
            },
        );
    if !json_content_type {
        return HttpResponse::new(415, br#"{"code":"unsupported_media_type"}"#.to_vec(), true)
            .expect("bounded media-type response");
    }
    let pipedal = match serde_json::from_slice::<mackes_ipc::PiPedalRequest>(&request.body) {
        Ok(request) => request,
        Err(error) => {
            return HttpResponse::new(
                400,
                serde_json::json!({"code":"invalid_request","message":error.to_string()})
                    .to_string()
                    .into_bytes(),
                true,
            )
            .expect("bounded error response");
        }
    };
    if pipedal.is_mutation() && !pipedal.confirm {
        return HttpResponse::new(400, br#"{"code":"confirmation_required"}"#.to_vec(), true)
            .expect("bounded confirmation response");
    }
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed).max(1);
    let envelope = Envelope {
        version: ProtocolVersion::current(),
        request_id: RequestId::new(request_id).expect("nonzero request ID"),
        command: Command::PiPedal,
        payload: serde_json::to_vec(&pipedal).expect("typed PiPedal request serializes"),
    };
    let policy =
        mackes_ipc::ReconnectPolicy::new(3, Duration::from_millis(25), Duration::from_millis(250))
            .expect("valid reconnect policy");
    match LocalClient::request_with_policy(socket, policy, &envelope) {
        Ok((body, _)) => {
            let status = pipedal_result_status(&body);
            HttpResponse::new(status, body, true).expect("bounded PiPedal response")
        }
        Err(error) => HttpResponse::new(
            503,
            serde_json::json!({"code":"daemon_unavailable","message":error})
                .to_string()
                .into_bytes(),
            true,
        )
        .expect("bounded unavailable response"),
    }
}

fn assignment_operation(request: &HttpRequest, socket: &PathBuf) -> HttpResponse {
    let media_type = request
        .headers
        .iter()
        .find(|(name, _)| name == "content-type")
        .and_then(|(_, value)| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"));
    if !media_type {
        return HttpResponse::new(415, br#"{"code":"unsupported_media_type"}"#.to_vec(), true)
            .expect("bounded media-type response");
    }
    let Some(assignment) = serde_json::from_slice::<mackes_ipc::AssignmentRequest>(&request.body)
        .ok()
        .and_then(|value| value.validate().ok())
    else {
        return HttpResponse::new(400, br#"{"code":"invalid_request"}"#.to_vec(), true)
            .expect("bounded assignment error response");
    };
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed).max(1);
    let envelope = Envelope {
        version: ProtocolVersion::current(),
        request_id: RequestId::new(request_id).expect("nonzero request ID"),
        command: Command::Assignment,
        payload: serde_json::to_vec(&assignment).expect("typed assignment request serializes"),
    };
    let policy =
        mackes_ipc::ReconnectPolicy::new(3, Duration::from_millis(25), Duration::from_millis(250))
            .expect("valid reconnect policy");
    match LocalClient::request_with_policy(socket, policy, &envelope) {
        Ok((body, _)) => {
            let status = serde_json::from_slice::<mackes_ipc::AssignmentResult>(&body)
                .ok()
                .map_or(502, |result| if result.applied { 200 } else { 409 });
            HttpResponse::new(status, body, true).expect("bounded assignment response")
        }
        Err(error) => HttpResponse::new(
            503,
            serde_json::json!({"code":"daemon_unavailable","message":error})
                .to_string()
                .into_bytes(),
            true,
        )
        .expect("bounded unavailable response"),
    }
}

fn pipedal_result_status(body: &[u8]) -> u16 {
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.get("ok").and_then(serde_json::Value::as_bool))
        .map_or(200, |accepted| if accepted { 200 } else { 409 })
}

fn novation_snapshot(socket: &PathBuf) -> HttpResponse {
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed).max(1);
    let envelope = Envelope {
        version: ProtocolVersion::current(),
        request_id: RequestId::new(request_id).expect("nonzero request ID"),
        command: Command::NovationSnapshot,
        payload: b"{}".to_vec(),
    };
    let policy =
        mackes_ipc::ReconnectPolicy::new(3, Duration::from_millis(25), Duration::from_millis(250))
            .expect("valid reconnect policy");
    match LocalClient::request_with_policy(socket, policy, &envelope) {
        Ok((body, _)) => {
            let Ok(snapshot) = serde_json::from_slice::<serde_json::Value>(&body) else {
                return HttpResponse::new(
                    502,
                    br#"{"code":"malformed_daemon_response"}"#.to_vec(),
                    true,
                )
                .expect("bounded response");
            };
            let response = serde_json::json!({
                "ok": snapshot.get("ok").and_then(serde_json::Value::as_bool).unwrap_or(false),
                "generation": snapshot.get("generation").cloned().unwrap_or(serde_json::Value::Null),
                "novation_device": snapshot.get("novation_device").cloned().unwrap_or(serde_json::Value::Null),
                "novation_capabilities": snapshot.get("novation_capabilities").cloned().unwrap_or(serde_json::Value::Null),
                "led": snapshot.get("led").cloned().unwrap_or(serde_json::Value::Null),
            });
            HttpResponse::new(200, response.to_string().into_bytes(), true)
                .expect("bounded response")
        }
        Err(error) => HttpResponse::new(
            503,
            serde_json::json!({"code":"daemon_unavailable","message":error})
                .to_string()
                .into_bytes(),
            true,
        )
        .expect("bounded unavailable response"),
    }
}

fn rtp_snapshot(socket: &PathBuf) -> HttpResponse {
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed).max(1);
    let envelope = Envelope {
        version: ProtocolVersion::current(),
        request_id: RequestId::new(request_id).expect("nonzero request ID"),
        command: Command::Snapshot,
        payload: b"{}".to_vec(),
    };
    let policy =
        mackes_ipc::ReconnectPolicy::new(3, Duration::from_millis(25), Duration::from_millis(250))
            .expect("valid reconnect policy");
    match LocalClient::request_with_policy(socket, policy, &envelope) {
        Ok((body, _)) => {
            let Ok(snapshot) = serde_json::from_slice::<serde_json::Value>(&body) else {
                return HttpResponse::new(
                    502,
                    br#"{"code":"malformed_daemon_response"}"#.to_vec(),
                    true,
                )
                .expect("bounded response");
            };
            HttpResponse::new(200, serde_json::json!({"ok": snapshot.get("ok").and_then(serde_json::Value::as_bool).unwrap_or(false), "generation": snapshot.get("generation"), "rtp_midi": snapshot.get("rtp_midi")}).to_string().into_bytes(), true).expect("bounded response")
        }
        Err(error) => HttpResponse::new(
            503,
            serde_json::json!({"code":"daemon_unavailable","message":error})
                .to_string()
                .into_bytes(),
            true,
        )
        .expect("bounded unavailable response"),
    }
}

fn pipedal_snapshot(socket: &PathBuf) -> HttpResponse {
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed).max(1);
    let pipedal = mackes_ipc::PiPedalRequest {
        operation: mackes_ipc::PiPedalOperation::Snapshot,
        generation: 0,
        confirm: false,
        mapping: None,
        physical_control_id: None,
        instance_id: None,
        client_id: None,
        value: None,
        enabled: None,
        use_mod_ui: None,
        title: None,
        color_key: None,
        volume_db: None,
        preview_input: None,
        midi_listener_handle: None,
        cancel_midi_listener: None,
        monitor_client_handle: None,
        property_uri: None,
        cancel_patch_monitor: None,
        favorites: None,
        query_show_status_monitor: None,
        bank_instance_id: None,
        preset_name: None,
        save_after_instance_id: None,
        plugin_instance_id: None,
        plugin_preset_name: None,
        load_preset_instance_id: None,
    };
    let envelope = Envelope {
        version: ProtocolVersion::current(),
        request_id: RequestId::new(request_id).expect("nonzero request ID"),
        command: Command::PiPedal,
        payload: serde_json::to_vec(&pipedal).expect("typed PiPedal snapshot serializes"),
    };
    let policy =
        mackes_ipc::ReconnectPolicy::new(3, Duration::from_millis(25), Duration::from_millis(250))
            .expect("valid reconnect policy");
    match LocalClient::request_with_policy(socket, policy, &envelope) {
        Ok((body, _)) => HttpResponse::new(200, body, true).expect("bounded PiPedal snapshot"),
        Err(error) => HttpResponse::new(
            503,
            serde_json::json!({"code":"daemon_unavailable","message":error})
                .to_string()
                .into_bytes(),
            true,
        )
        .expect("bounded unavailable response"),
    }
}

fn routes_operation(request: &HttpRequest, socket: &PathBuf) -> HttpResponse {
    let json_content_type =
        request.headers.iter().find(|(name, _)| name == "content-type").is_some_and(
            |(_, value)| {
                value.split(';').next().is_some_and(|media_type| {
                    media_type.trim().eq_ignore_ascii_case("application/json")
                })
            },
        );
    if !json_content_type {
        return HttpResponse::new(415, br#"{"code":"unsupported_media_type"}"#.to_vec(), true)
            .expect("bounded media-type response");
    }
    let value = match serde_json::from_slice::<serde_json::Value>(&request.body) {
        Ok(value) if value.is_object() => value,
        _ => {
            return HttpResponse::new(400, b"invalid route request".to_vec(), false)
                .expect("response")
        }
    };
    let valid_fields = value.as_object().is_some_and(|object| {
        object.keys().all(|key| {
            matches!(
                key.as_str(),
                "action" | "routes" | "route_generation" | "hop_limit" | "token" | "ssrc"
            )
        })
    });
    let is_undo = value.get("action").and_then(serde_json::Value::as_str) == Some("undo");
    let rtp_action = matches!(
        value.get("action").and_then(serde_json::Value::as_str),
        Some("rtp_establish" | "rtp_end")
    );
    let valid = valid_fields
        && (rtp_action
            || value.get("route_generation").and_then(serde_json::Value::as_u64).is_some())
        && if rtp_action {
            value
                .get("token")
                .and_then(serde_json::Value::as_u64)
                .is_some_and(|v| u32::try_from(v).is_ok())
                && value
                    .get("ssrc")
                    .and_then(serde_json::Value::as_u64)
                    .is_some_and(|v| u32::try_from(v).is_ok())
                && value.get("routes").is_none()
                && value.get("hop_limit").is_none()
        } else if is_undo {
            value.get("routes").is_none() && value.get("hop_limit").is_none()
        } else {
            value.get("action").is_none()
                && value.get("routes").is_some_and(serde_json::Value::is_array)
                && value
                    .get("hop_limit")
                    .and_then(serde_json::Value::as_u64)
                    .is_some_and(|limit| (1..=16).contains(&limit))
        };
    if !valid {
        return HttpResponse::new(
            400,
            b"route request requires bounded routes, generation, and hop_limit".to_vec(),
            false,
        )
        .expect("response");
    }
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed).max(1);
    let envelope = Envelope {
        version: ProtocolVersion::current(),
        request_id: RequestId::new(request_id).expect("nonzero request ID"),
        command: Command::Routes,
        payload: serde_json::to_vec(&value).expect("route request serializes"),
    };
    let policy =
        mackes_ipc::ReconnectPolicy::new(3, Duration::from_millis(25), Duration::from_millis(250))
            .expect("valid reconnect policy");
    match LocalClient::request_with_policy(socket, policy, &envelope) {
        Ok((body, _)) => {
            let status = daemon_result_status(&body);
            HttpResponse::new(status, body, true).expect("bounded route response")
        }
        Err(error) => HttpResponse::new(
            503,
            serde_json::json!({"code":"daemon_unavailable","message":error})
                .to_string()
                .into_bytes(),
            true,
        )
        .expect("bounded unavailable response"),
    }
}

#[allow(clippy::too_many_lines)]
fn scenes_operation(request: &HttpRequest, socket: &PathBuf) -> HttpResponse {
    let json_content_type =
        request.headers.iter().find(|(name, _)| name == "content-type").is_some_and(
            |(_, value)| {
                value.split(';').next().is_some_and(|media_type| {
                    media_type.trim().eq_ignore_ascii_case("application/json")
                })
            },
        );
    if !json_content_type {
        return HttpResponse::new(415, br#"{"code":"unsupported_media_type"}"#.to_vec(), true)
            .expect("bounded media-type response");
    }
    let value = match serde_json::from_slice::<serde_json::Value>(&request.body) {
        Ok(value) if value.is_object() => value,
        _ => {
            return HttpResponse::new(400, b"invalid scene request".to_vec(), false)
                .expect("response")
        }
    };
    let valid_fields = value.as_object().is_some_and(|object| {
        object.keys().all(|key| {
            matches!(
                key.as_str(),
                "scene"
                    | "direction"
                    | "actions"
                    | "project"
                    | "setlist"
                    | "setlist_create"
                    | "setlist_delete"
                    | "project_copy"
                    | "setlist_copy"
                    | "preview_scene"
                    | "preview_setlist"
                    | "execute_scene"
            )
        })
    });
    let scene_valid = value.get("scene").and_then(serde_json::Value::as_str).is_some_and(|scene| {
        !scene.is_empty() && scene.len() <= mackes_web_contract::MAX_ID_LENGTH
    });
    let direction_valid = value
        .get("direction")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|direction| matches!(direction, "next" | "previous"));
    let has_scene = value.get("scene").is_some();
    let has_direction = value.get("direction").is_some();
    let has_project = value.get("project").is_some();
    let has_setlist = value.get("setlist").is_some();
    let has_setlist_create = value.get("setlist_create").is_some();
    let has_setlist_delete = value.get("setlist_delete").is_some();
    let has_project_copy = value.get("project_copy").is_some();
    let has_setlist_copy = value.get("setlist_copy").is_some();
    let has_preview_scene = value.get("preview_scene").is_some();
    let has_preview_setlist = value.get("preview_setlist").is_some();
    let has_execute_scene = value.get("execute_scene").is_some();
    let execute_scene_valid = value
        .get("execute_scene")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|id| !id.is_empty() && id.len() <= mackes_web_contract::MAX_ID_LENGTH);
    let actions_valid = value
        .get("actions")
        .is_none_or(|actions| actions.as_array().is_some_and(|items| items.len() <= 128));
    let project_valid = value.get("project").is_some_and(serde_json::Value::is_object);
    let setlist_valid = value.get("setlist").is_some_and(serde_json::Value::is_object);
    let setlist_create_valid =
        value.get("setlist_create").is_some_and(serde_json::Value::is_object);
    let setlist_delete_valid = value
        .get("setlist_delete")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|id| !id.is_empty() && id.len() <= mackes_web_contract::MAX_ID_LENGTH);
    let project_copy_valid = value.get("project_copy").is_some_and(|copy| {
        copy.get("source").and_then(serde_json::Value::as_str).is_some_and(|id| !id.is_empty())
            && copy
                .get("new_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| !id.is_empty() && id.len() <= mackes_web_contract::MAX_ID_LENGTH)
    });
    let setlist_copy_valid = value.get("setlist_copy").is_some_and(|copy| {
        copy.get("source").and_then(serde_json::Value::as_str).is_some_and(|id| !id.is_empty())
            && copy
                .get("new_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| !id.is_empty() && id.len() <= mackes_web_contract::MAX_ID_LENGTH)
    });
    let preview_scene_valid = value
        .get("preview_scene")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|id| !id.is_empty() && id.len() <= mackes_web_contract::MAX_ID_LENGTH);
    let preview_setlist_valid = value
        .get("preview_setlist")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|id| !id.is_empty() && id.len() <= mackes_web_contract::MAX_ID_LENGTH);
    let valid_action =
        (has_scene && !has_direction && !has_project && scene_valid && actions_valid)
            || (!has_scene && !has_direction && has_project && project_valid)
            || (!has_scene && !has_direction && !has_project && has_setlist && setlist_valid)
            || (!has_scene
                && !has_direction
                && !has_project
                && !has_setlist
                && has_setlist_create
                && setlist_create_valid)
            || (!has_scene
                && !has_direction
                && !has_project
                && !has_setlist
                && !has_setlist_create
                && has_setlist_delete
                && setlist_delete_valid)
            || (!has_scene
                && !has_direction
                && !has_project
                && !has_setlist
                && !has_setlist_create
                && !has_setlist_delete
                && has_project_copy
                && project_copy_valid)
            || (!has_scene
                && !has_direction
                && !has_project
                && !has_setlist
                && !has_setlist_create
                && !has_setlist_delete
                && !has_project_copy
                && !has_setlist_copy
                && has_preview_scene
                && preview_scene_valid)
            || (!has_scene
                && !has_direction
                && !has_project
                && !has_setlist
                && !has_setlist_create
                && !has_setlist_delete
                && !has_project_copy
                && !has_setlist_copy
                && !has_preview_scene
                && has_preview_setlist
                && preview_setlist_valid)
            || (!has_scene
                && !has_direction
                && !has_project
                && !has_setlist
                && !has_setlist_create
                && !has_setlist_delete
                && !has_project_copy
                && !has_setlist_copy
                && !has_preview_scene
                && has_execute_scene
                && execute_scene_valid)
            || (!has_scene
                && !has_direction
                && !has_project
                && !has_setlist
                && !has_setlist_create
                && !has_setlist_delete
                && !has_project_copy
                && has_setlist_copy
                && setlist_copy_valid)
            || (has_direction && !has_scene && direction_valid);
    if !valid_fields || !valid_action {
        return HttpResponse::new(
            400,
            b"scene request requires exactly one valid scene or direction".to_vec(),
            false,
        )
        .expect("response");
    }
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed).max(1);
    let envelope = Envelope {
        version: ProtocolVersion::current(),
        request_id: RequestId::new(request_id).expect("nonzero request ID"),
        command: Command::Scenes,
        payload: serde_json::to_vec(&value).expect("scene request serializes"),
    };
    let policy =
        mackes_ipc::ReconnectPolicy::new(3, Duration::from_millis(25), Duration::from_millis(250))
            .expect("valid reconnect policy");
    match LocalClient::request_with_policy(socket, policy, &envelope) {
        Ok((body, _)) => {
            let status = daemon_result_status(&body);
            HttpResponse::new(status, body, true).expect("bounded scene response")
        }
        Err(error) => HttpResponse::new(
            503,
            serde_json::json!({"code":"daemon_unavailable","message":error})
                .to_string()
                .into_bytes(),
            true,
        )
        .expect("bounded unavailable response"),
    }
}

fn daemon_result_status(body: &[u8]) -> u16 {
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.get("ok").and_then(serde_json::Value::as_bool))
        .map_or(502, |ok| if ok { 200 } else { 409 })
}

fn event_cursor(path: &str) -> Result<u64, &'static str> {
    let Some((_, query)) = path.split_once('?') else { return Ok(0) };
    let mut value = None;
    for part in query.split('&') {
        let Some((name, candidate)) = part.split_once('=') else {
            return Err("malformed event query parameter");
        };
        if name != "after_sequence" || value.replace(candidate).is_some() {
            return Err("only one after_sequence query parameter is supported");
        }
    }
    let value = value.ok_or("after_sequence query parameter is required")?;
    value.parse::<u64>().map_err(|_| "after_sequence must be a nonnegative integer")
}

fn validate_device_control_payload(
    payload: Option<&serde_json::Value>,
) -> Result<(), &'static str> {
    let object = payload
        .and_then(serde_json::Value::as_object)
        .ok_or("device_control payload is required")?;
    if object.keys().any(|key| {
        !matches!(key.as_str(), "profile_id" | "control" | "channel" | "value" | "destination")
    }) {
        return Err("device_control payload contains an unknown field");
    }
    let profile = object.get("profile_id").and_then(serde_json::Value::as_str);
    let control = object.get("control").and_then(serde_json::Value::as_str);
    let destination = object.get("destination").and_then(serde_json::Value::as_str);
    if !profile
        .is_some_and(|value| !value.is_empty() && value.len() <= mackes_web_contract::MAX_ID_LENGTH)
        || !control.is_some_and(|value| {
            !value.is_empty() && value.len() <= mackes_web_contract::MAX_ID_LENGTH
        })
        || !destination.is_some_and(|value| {
            !value.is_empty() && value.len() <= mackes_web_contract::MAX_ID_LENGTH
        })
    {
        return Err("device_control identity fields are invalid");
    }
    // Reflex reset is a documented device operation rather than a MIDI
    // parameter.  The daemon deliberately accepts it without channel/value;
    // keep the browser boundary aligned with that typed contract.
    if profile == Some("lexicon.reflex") && control == Some("system-reset") {
        return Ok(());
    }
    if object.get("channel").and_then(serde_json::Value::as_u64).is_none_or(|value| value > 15)
        || object
            .get("value")
            .and_then(serde_json::Value::as_u64)
            .is_none_or(|value| value > u16::MAX.into())
    {
        return Err("device_control channel or value is out of range");
    }
    Ok(())
}

fn validate_rebind_payload(payload: Option<&serde_json::Value>) -> Result<(), &'static str> {
    let object =
        payload.and_then(serde_json::Value::as_object).ok_or("rebind payload is required")?;
    let stable_id = object
        .get("stable_id")
        .and_then(serde_json::Value::as_str)
        .ok_or("rebind stable_id is required")?;
    if stable_id.is_empty()
        || stable_id.len() > mackes_web_contract::MAX_ID_LENGTH
        || stable_id != stable_id.trim()
    {
        return Err("rebind stable_id is invalid");
    }
    if object.get("template").and_then(serde_json::Value::as_u64).is_none_or(|value| value >= 16) {
        return Err("rebind template must be 0..15");
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn operation(request: &HttpRequest, socket: &PathBuf) -> HttpResponse {
    let json_content_type =
        request.headers.iter().find(|(name, _)| name == "content-type").is_some_and(
            |(_, value)| {
                value.split(';').next().is_some_and(|media_type| {
                    media_type.trim().eq_ignore_ascii_case("application/json")
                })
            },
        );
    if !json_content_type {
        return HttpResponse::new(415, br#"{"code":"unsupported_media_type"}"#.to_vec(), true)
            .expect("bounded media-type response");
    }
    let parsed = match serde_json::from_slice::<OperationRequest>(&request.body) {
        Ok(request) => request,
        Err(error) => {
            return HttpResponse::new(
                400,
                serde_json::json!({"code":"invalid_request","message":error.to_string()})
                    .to_string()
                    .into_bytes(),
                true,
            )
            .expect("bounded error response");
        }
    };
    if let Err(error) = parsed.validate() {
        return HttpResponse::new(
            400,
            serde_json::json!({"code":"invalid_request","message":error}).to_string().into_bytes(),
            true,
        )
        .expect("bounded error response");
    }
    let cache = OPERATION_CACHE.get_or_init(|| Mutex::new(load_operation_cache()));
    let request_fingerprint = serde_json::to_string(&parsed).expect("typed request fingerprints");
    // Hold the cache mutex through daemon execution. This makes the request-id
    // check and response publication one bounded critical section, so two
    // simultaneous retries cannot both reach a hardware-affecting command.
    let mut cache_guard = cache.lock().expect("operation cache lock");
    if let Some((_, fingerprint, response)) =
        cache_guard.iter().find(|(request_id, _, _)| request_id == &parsed.request_id)
    {
        if fingerprint == &request_fingerprint {
            return response.clone();
        }
        return HttpResponse::new(409, br#"{"code":"request_id_reuse"}"#.to_vec(), true)
            .expect("bounded request reuse response");
    }
    let command = match parsed.operation.as_str() {
        "rescan" | "rebind_undo" => Command::Rescan,
        "rebind" => {
            if let Err(error) = validate_rebind_payload(parsed.payload.as_ref()) {
                return HttpResponse::new(
                    400,
                    serde_json::json!({"code":"invalid_request","message":error})
                        .to_string()
                        .into_bytes(),
                    true,
                )
                .expect("bounded error response");
            }
            Command::Rescan
        }
        "panic" if parsed.confirm => Command::Panic,
        "device_control" if parsed.confirm => {
            if let Err(error) = validate_device_control_payload(parsed.payload.as_ref()) {
                return HttpResponse::new(
                    400,
                    serde_json::json!({"code":"invalid_request","message":error})
                        .to_string()
                        .into_bytes(),
                    true,
                )
                .expect("bounded error response");
            }
            Command::DeviceControl
        }
        "panic" | "device_control" => {
            return HttpResponse::new(409, br#"{"code":"confirmation_required"}"#.to_vec(), true)
                .expect("bounded error response");
        }
        _ => {
            return HttpResponse::new(
                501,
                br#"{"code":"unsupported","work_item":"W132"}"#.to_vec(),
                true,
            )
            .expect("bounded error response");
        }
    };
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed).max(1);
    let payload = if command == Command::DeviceControl {
        serde_json::to_vec(parsed.payload.as_ref().expect("validated control payload"))
            .expect("typed control payload serializes")
    } else if parsed.operation == "rebind" || parsed.operation == "rebind_undo" {
        let mut value = parsed.payload.clone().unwrap_or_else(|| serde_json::json!({}));
        value.as_object_mut().expect("validated rebind object").insert(
            "action".into(),
            serde_json::Value::String(
                if parsed.operation == "rebind" { "rebind" } else { "rebind_undo" }.into(),
            ),
        );
        serde_json::to_vec(&value).expect("typed rebind payload serializes")
    } else {
        serde_json::to_vec(&parsed).expect("typed request serializes")
    };
    let envelope = Envelope {
        version: ProtocolVersion::current(),
        request_id: RequestId::new(request_id).expect("nonzero request ID"),
        command,
        payload,
    };
    let policy =
        mackes_ipc::ReconnectPolicy::new(3, Duration::from_millis(25), Duration::from_millis(250))
            .expect("valid reconnect policy");
    match LocalClient::request_with_policy(socket, policy, &envelope) {
        Ok((body, _)) => {
            let Ok(daemon) = serde_json::from_slice::<serde_json::Value>(&body) else {
                return HttpResponse::new(
                    502,
                    br#"{"code":"malformed_daemon_response"}"#.to_vec(),
                    true,
                )
                .expect("bounded malformed response");
            };
            if !daemon.is_object() {
                return HttpResponse::new(
                    502,
                    br#"{"code":"malformed_daemon_response"}"#.to_vec(),
                    true,
                )
                .expect("bounded malformed response");
            }
            let accepted = daemon.get("ok").and_then(serde_json::Value::as_bool).unwrap_or(false);
            let status = if accepted { 202 } else { 409 };
            let response = HttpResponse::new(
                status,
                serde_json::json!({
                    "operation_id": format!("web-{request_id}"),
                    "generation": daemon.get("generation").and_then(serde_json::Value::as_u64).unwrap_or(parsed.generation),
                    "accepted": accepted,
                    "daemon": daemon
                })
                .to_string()
                .into_bytes(),
                true,
            )
            .expect("bounded operation response");
            if accepted {
                if cache_guard.len() == OPERATION_CACHE_CAPACITY {
                    cache_guard.pop_front();
                }
                cache_guard.push_back((parsed.request_id, request_fingerprint, response.clone()));
                persist_operation_cache(&cache_guard);
            }
            drop(cache_guard);
            response
        }
        Err(error) => HttpResponse::new(
            503,
            serde_json::json!({"code":"daemon_unavailable","message":error})
                .to_string()
                .into_bytes(),
            true,
        )
        .expect("bounded unavailable response"),
    }
}

fn load_operation_cache() -> VecDeque<(String, String, HttpResponse)> {
    let Ok(bytes) = std::fs::read(OPERATION_CACHE_PATH) else { return VecDeque::new() };
    let Ok(entries) = serde_json::from_slice::<Vec<(String, String, u16, String, bool)>>(&bytes)
    else {
        return VecDeque::new();
    };
    entries
        .into_iter()
        .filter_map(|(request_id, fingerprint, status, body, json)| {
            HttpResponse::new(status, body.into_bytes(), json)
                .ok()
                .map(|response| (request_id, fingerprint, response))
        })
        .collect()
}

fn persist_operation_cache(cache: &VecDeque<(String, String, HttpResponse)>) {
    let entries: Vec<_> = cache
        .iter()
        .filter_map(|(request_id, fingerprint, response)| {
            String::from_utf8(response.body.clone())
                .ok()
                .map(|body| (request_id, fingerprint, response.status, body, response.json))
        })
        .collect();
    let Ok(bytes) = serde_json::to_vec(&entries) else { return };
    let temporary = format!("{OPERATION_CACHE_PATH}.tmp-{}", std::process::id());
    if std::fs::write(&temporary, bytes).is_ok() {
        let _ = std::fs::rename(temporary, OPERATION_CACHE_PATH);
    }
}

fn header_value(bytes: &[u8], wanted: &str) -> Option<String> {
    std::str::from_utf8(bytes).ok()?.split("\r\n").skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        (name.eq_ignore_ascii_case(wanted)).then(|| value.trim().to_owned())
    })
}

fn write_response(stream: &mut TcpStream, response: &HttpResponse) {
    if let Ok(bytes) = response.encode() {
        let _ = stream.write_all(&bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guided_configuration_workspace_replaces_raw_editor() {
        let html = include_str!("../static/index.html");
        let js = include_str!("../static/app.js");
        assert!(html.contains("configuration-builder"));
        assert!(html.contains("open-guided-settings"));
        assert!(!html.contains("configuration-json5-draft"));
        assert!(!html.contains("apply-configuration-json5"));
        assert!(!js.contains("configuration_json5: draft"));
        assert!(js.contains("/api/v1/configuration"));
        assert!(html.contains("Export raw configuration"));
    }

    #[test]
    fn bundled_shell_loads_feature_catalog_before_application() {
        let html = include_str!("../static/index.html");
        let js = include_str!("../static/app.js");
        let catalog = html.find("/assets/feature_catalog.js").expect("feature catalog asset");
        let app = html.find("/assets/app.js").expect("application asset");
        assert!(catalog < app, "feature catalog must initialize before app.js");
        assert!(js.contains("endpoints.some(device => /pipedal/i.test(JSON.stringify(device)))"));
        assert!(js.contains("Novation ${lifecycle} · LED ${ledPhase} · feedback ${feedback}"));
        assert!(!js.contains("identity ${stableId}"));
        assert!(js.contains("Implemented operations"));
        assert!(js.contains("runOperation(entry.operation)"));
        assert!(js.contains("renderFaceplate(body);"));
        assert!(js.contains("PiPedal authoritative snapshot"));
        assert!(js.contains("pipedalCatalogTargets"));
        assert!(js.contains("Refresh status monitor"));
        assert!(js.contains("Save current preset as"));
        assert!(js.contains("Save plugin preset as"));
        assert!(js.contains("controls: ${controlRows.join"));
        assert!(js.contains("range ${control.min_value}..${control.max_value}"));
        assert!(js.contains("daemon ${body.pipedal.phase}"));
        assert!(js.contains("Readback is current for this snapshot"));
    }

    #[test]
    fn event_stream_reconnect_is_sequence_aware() {
        let js = include_str!("../static/app.js");
        assert!(js.contains("/api/v1/events/stream?after_sequence=${lastSequence}"));
        assert!(js.contains("eventStreamRetry"));
        assert!(js.contains("snapshot_required"));
    }

    #[test]
    fn capabilities_response_conforms_to_versioned_envelope() {
        let request = HttpRequest::parse(
            b"GET /api/v1/capabilities HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\n\r\n",
        )
        .expect("request");
        let response = route(&request, &PathBuf::from("/unused"), "http://localhost:8081");
        let value: serde_json::Value = serde_json::from_slice(&response.body).expect("JSON");
        assert_eq!(value["schema_version"], 1);
        assert!(value["generation"].is_u64());
        assert!(value["capabilities"].as_array().is_some_and(|items| !items.is_empty()));
    }

    #[test]
    fn unavailable_daemon_is_not_reported_as_success() {
        let request =
            HttpRequest::parse(b"GET /api/v1/health HTTP/1.1\r\nHost: localhost:8081\r\n\r\n")
                .expect("request");
        let response = route(
            &request,
            &std::env::temp_dir().join(format!("mackes-web-missing-{}", std::process::id())),
            "http://localhost:8081",
        );
        assert_eq!(response.status, 503);
        assert!(String::from_utf8(response.body).expect("JSON").contains("daemon_unavailable"));
    }

    #[test]
    fn mutation_is_explicitly_unsupported() {
        let request = HttpRequest::parse(
            b"POST /api/v1/operations HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: 54\r\n\r\n{\"request_id\":\"x\",\"operation\":\"future\",\"generation\":0}",
        )
        .expect("request");
        let response = route(
            &request,
            &std::env::temp_dir().join(format!("mackes-web-unused-{}", std::process::id())),
            "http://localhost:8081",
        );
        assert_eq!(response.status, 501);
    }

    #[test]
    fn mutation_rejects_non_json_media_type() {
        let request = HttpRequest::parse(
            b"POST /api/v1/operations HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\n{}",
        )
        .expect("request");
        assert_eq!(operation(&request, &PathBuf::from("/missing")).status, 415);
    }

    #[test]
    fn mutation_accepts_json_media_type_parameters() {
        let request = HttpRequest::parse(
            b"POST /api/v1/operations HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: 54\r\n\r\n{\"request_id\":\"x\",\"operation\":\"future\",\"generation\":0}",
        )
        .expect("request");
        assert_eq!(operation(&request, &PathBuf::from("/missing")).status, 501);
    }

    #[cfg(unix)]
    #[test]
    fn mutation_rejects_malformed_daemon_response() {
        use std::{os::unix::net::UnixListener, thread};
        let socket = std::env::temp_dir()
            .join(format!("mackes-web-malformed-operation-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("IPC UTF-8");
            assert!(request.contains("\"command\":\"rescan\""));
            stream.write_all(b"not-json\n").expect("write malformed response");
        });
        let body =
            br#"{"request_id":"malformed-daemon-response","operation":"rescan","generation":0}"#;
        let request = HttpRequest::parse(
            format!(
                "POST /api/v1/operations HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                std::str::from_utf8(body).expect("JSON")
            )
            .as_bytes(),
        )
        .expect("request");
        let response = operation(&request, &socket);
        assert_eq!(response.status, 502);
        assert_eq!(response.body, br#"{"code":"malformed_daemon_response"}"#);
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[test]
    fn assignment_operation_rejects_invalid_boundary_input_before_ipc() {
        let request = HttpRequest::parse(
            b"POST /api/v1/assignment HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\n{}",
        )
        .expect("request");
        assert_eq!(assignment_operation(&request, &PathBuf::from("/missing")).status, 415);

        let request = HttpRequest::parse(
            b"POST /api/v1/assignment HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}",
        )
        .expect("request");
        assert_eq!(assignment_operation(&request, &PathBuf::from("/missing")).status, 400);
    }

    #[cfg(unix)]
    #[test]
    fn novation_route_projects_only_device_contract() {
        use std::{os::unix::net::UnixListener, thread};
        let socket =
            std::env::temp_dir().join(format!("mackes-web-novation-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            assert!(String::from_utf8(request)
                .expect("IPC UTF-8")
                .contains("\"command\":\"novation_snapshot\""));
            stream.write_all(b"{\"ok\":true,\"generation\":7,\"received\":99,\"novation_device\":{\"lifecycle\":\"Ready\"},\"novation_capabilities\":{\"led_count\":48},\"led\":{\"phase\":\"ready\"}}\n").expect("write response");
        });
        let response = novation_snapshot(&socket);
        assert_eq!(response.status, 200);
        let body: serde_json::Value = serde_json::from_slice(&response.body).expect("JSON");
        assert_eq!(body["generation"], 7);
        assert_eq!(body["novation_device"]["lifecycle"], "Ready");
        assert!(body.get("received").is_none());
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[cfg(unix)]
    #[test]
    fn assignment_snapshot_route_forwards_typed_ipc_request() {
        use std::{os::unix::net::UnixListener, thread};
        let socket =
            std::env::temp_dir().join(format!("mackes-web-assignment-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("IPC UTF-8");
            assert!(request.contains("\"command\":\"assignment\""));
            assert!(request.contains("\"action\":\"Snapshot\""));
            stream.write_all(b"{}\n").expect("write response");
        });
        let request =
            HttpRequest::parse(b"GET /api/v1/assignment HTTP/1.1\r\nHost: localhost:8081\r\n\r\n")
                .expect("request");
        let response = route(&request, &socket, "http://localhost:8081");
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
        assert_eq!(response.status, 200);
    }

    #[test]
    fn device_control_payload_validation_is_strict_and_bounded() {
        let valid = serde_json::json!({
            "profile_id": "default",
            "control": "cutoff",
            "channel": 15,
            "value": 16383,
            "destination": "synth"
        });
        assert!(validate_device_control_payload(Some(&valid)).is_ok());
        assert!(validate_device_control_payload(None).is_err());
        assert!(validate_device_control_payload(Some(&serde_json::json!({
            "profile_id": "default",
            "control": "cutoff",
            "channel": 16,
            "value": 1,
            "destination": "synth"
        })))
        .is_err());
        assert!(validate_device_control_payload(Some(&serde_json::json!({
            "profile_id": "lexicon.reflex",
            "control": "system-reset",
            "destination": "lexicon-midi"
        })))
        .is_ok());
        assert!(validate_device_control_payload(Some(&serde_json::json!({
            "profile_id": "default",
            "control": "cutoff",
            "channel": 1,
            "value": 1,
            "destination": "synth",
            "extra": true
        })))
        .is_err());
    }

    #[test]
    fn sysex_boundary_rejects_unconfirmed_and_invalid_bytes_before_ipc() {
        for body in [
            br#"{"destination":"eventide","bytes":[1,2]}"#.as_slice(),
            br#"{"destination":"eventide","bytes":[128],"confirm":true}"#.as_slice(),
        ] {
            let request = HttpRequest::parse(
                format!(
                    "POST /api/v1/sysex HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    std::str::from_utf8(body).expect("JSON")
                )
                .as_bytes(),
            )
            .expect("request");
            assert_eq!(sysex_operation(&request, &PathBuf::from("/never-open")).status, 400);
        }
    }

    #[test]
    fn configuration_boundary_rejects_unconfirmed_or_empty_writes_before_ipc() {
        for body in [
            br#"{"confirm":false,"setlists":[]}"#.as_slice(),
            br#"{"confirm":true}"#.as_slice(),
            br#"{"confirm":true,"setlists":[],"configuration_revision":7}"#.as_slice(),
        ] {
            let request = HttpRequest::parse(format!(
                "POST /api/v1/configuration HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                std::str::from_utf8(body).expect("JSON")
            ).as_bytes()).expect("request");
            assert_eq!(
                configuration_operation(&request, &PathBuf::from("/never-open")).status,
                400
            );
        }
    }

    #[test]
    fn configuration_boundary_accepts_full_documents_and_draft_operations() {
        for body in [
            br#"{"operation":"draft","draft_id":"draft-1","operation_id":"op-1","configuration":{"schema_version":1}}"#.as_slice(),
            br#"{"operation":"validate","configuration":{"schema_version":1}}"#.as_slice(),
        ] {
            let request = HttpRequest::parse(format!(
                "POST /api/v1/configuration HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                std::str::from_utf8(body).expect("JSON")
            ).as_bytes()).expect("request");
            // Validation succeeds at the web boundary and reaches the unavailable-daemon path.
            assert_eq!(configuration_operation(&request, &PathBuf::from("/never-open")).status, 503);
        }
    }

    #[test]
    fn configuration_boundary_rejects_unknown_operation_and_unbounded_identifier() {
        for body in [
            br#"{"operation":"delete","configuration":{"schema_version":1}}"#.as_slice(),
            br#"{"operation":"draft","draft_id":"","configuration":{"schema_version":1}}"#
                .as_slice(),
        ] {
            let request = HttpRequest::parse(format!(
                "POST /api/v1/configuration HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                std::str::from_utf8(body).expect("JSON")
            ).as_bytes()).expect("request");
            assert_eq!(
                configuration_operation(&request, &PathBuf::from("/never-open")).status,
                400
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn sysex_route_forwards_confirmed_payload_to_daemon() {
        use std::{os::unix::net::UnixListener, thread};
        let socket = std::env::temp_dir().join(format!("mackes-web-sysex-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            assert!(String::from_utf8(request)
                .expect("IPC UTF-8")
                .contains("\"command\":\"sysex\""));
            stream
                .write_all(
                    br#"{"ok":true,"generation":8,"sent":true}
"#,
                )
                .expect("write response");
        });
        let body = br#"{"destination":"eventide","bytes":[1,2,3],"confirm":true}"#;
        let request = HttpRequest::parse(format!("POST /api/v1/sysex HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", body.len(), std::str::from_utf8(body).expect("JSON")).as_bytes()).expect("request");
        let response = sysex_operation(&request, &socket);
        assert_eq!(response.status, 200);
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[test]
    fn mapping_route_rejects_untyped_payload_before_ipc() {
        let request = HttpRequest::parse(
            b"POST /api/v1/mappings HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}",
        )
        .expect("request");
        assert_eq!(mapping_operation(&request, &PathBuf::from("/missing")).status, 400);
    }

    #[test]
    fn pipedal_route_rejects_untyped_payload_before_ipc() {
        let request = HttpRequest::parse(
            b"POST /api/v1/pipedal HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}",
        )
        .expect("request");
        assert_eq!(pipedal_operation(&request, &PathBuf::from("/missing")).status, 400);
    }

    #[cfg(unix)]
    #[test]
    fn pipedal_snapshot_forwards_typed_request_to_daemon() {
        use std::{os::unix::net::UnixListener, thread};
        let socket =
            std::env::temp_dir().join(format!("mackes-web-pipedal-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("IPC UTF-8");
            assert!(request.contains("\"command\":\"pipedal\""));
            assert!(request.contains("\"operation\":\"snapshot\""));
            stream
                .write_all(
                    br#"{"ok":true,"phase":"ready","supported_operations":["setControl"]}
"#,
                )
                .expect("write response");
        });
        let response = pipedal_snapshot(&socket);
        assert_eq!(response.status, 200);
        let body = String::from_utf8(response.body).expect("JSON");
        assert!(body.contains("\"phase\":\"ready\""));
        assert!(body.contains("supported_operations"));
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[cfg(unix)]
    #[test]
    fn assignment_commit_forwards_typed_destination_fields() {
        use std::{os::unix::net::UnixListener, thread};
        let socket = std::env::temp_dir().join(format!("mackes-web-commit-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("IPC UTF-8");
            assert!(request.contains("\"action\":\"Commit\""));
            assert!(request.contains("\"physical_control_id\":\"knob-r3-c4\""));
            assert!(request.contains("\"destination_profile\":\"pipedal\""));
            stream.write_all(b"not-json\n").expect("write response");
        });
        let body = br#"{"generation":4,"action":"Commit","physical_control_id":"knob-r3-c4","destination_profile":"pipedal","destination_effect":"eq","destination_parameter":"gain"}"#;
        let request = HttpRequest::parse(format!("POST /api/v1/assignment HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", body.len(), std::str::from_utf8(body).expect("JSON")).as_bytes()).expect("request");
        assert_eq!(assignment_operation(&request, &socket).status, 502);
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[test]
    fn pipedal_result_status_preserves_daemon_truth() {
        assert_eq!(pipedal_result_status(br#"{"ok":true}"#), 200);
        assert_eq!(pipedal_result_status(br#"{"ok":false,"error":"rejected"}"#), 409);
        assert_eq!(pipedal_result_status(br"not-json"), 200);
    }

    #[test]
    fn daemon_result_status_maps_rejections_to_conflict() {
        assert_eq!(daemon_result_status(br#"{"ok":true}"#), 200);
        assert_eq!(daemon_result_status(br#"{"ok":false}"#), 409);
        assert_eq!(daemon_result_status(b"not-json"), 502);
    }

    #[test]
    fn route_undo_is_accepted_by_web_boundary() {
        let body = br#"{"action":"undo","route_generation":4}"#;
        let request = HttpRequest::parse(
            format!(
                "POST /api/v1/routes HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                std::str::from_utf8(body).expect("JSON")
            )
            .as_bytes(),
        )
        .expect("request");
        assert_eq!(routes_operation(&request, &PathBuf::from("/missing")).status, 503);
    }

    #[cfg(unix)]
    #[test]
    fn route_replacement_forwards_bounded_request_to_daemon() {
        use std::{os::unix::net::UnixListener, thread};
        let socket = std::env::temp_dir().join(format!("mackes-web-routes-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("IPC UTF-8");
            assert!(request.contains("\"command\":\"routes\""));
            assert!(request.contains("\"route_generation\":4"));
            stream
                .write_all(
                    br#"{"ok":true,"route_generation":5}
"#,
                )
                .expect("write response");
        });
        let body = br#"{"routes":[],"route_generation":4,"hop_limit":8}"#;
        let request = HttpRequest::parse(
            format!(
                "POST /api/v1/routes HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(), std::str::from_utf8(body).expect("JSON")
            )
            .as_bytes(),
        )
        .expect("request");
        let response = routes_operation(&request, &socket);
        assert_eq!(response.status, 200);
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[cfg(unix)]
    #[test]
    fn scene_selection_forwards_typed_request_to_daemon() {
        use std::{os::unix::net::UnixListener, thread};
        let socket = std::env::temp_dir().join(format!("mackes-web-scenes-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("IPC UTF-8");
            assert!(request.contains("\"command\":\"scenes\""));
            assert!(request.contains("\"scene\":\"scene-a\""));
            stream
                .write_all(
                    br#"{"ok":true,"generation":9,"active_scene":"scene-a"}
"#,
                )
                .expect("write response");
        });
        let body = br#"{"scene":"scene-a"}"#;
        let request = HttpRequest::parse(
            format!(
                "POST /api/v1/scenes HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                std::str::from_utf8(body).expect("JSON")
            )
            .as_bytes(),
        )
        .expect("request");
        let response = scenes_operation(&request, &socket);
        assert_eq!(response.status, 200);
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[cfg(unix)]
    #[test]
    fn scene_execution_forwards_explicit_typed_request() {
        use std::{os::unix::net::UnixListener, thread};
        let socket =
            std::env::temp_dir().join(format!("mackes-web-execute-scene-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("IPC UTF-8");
            assert!(request.contains("\"command\":\"scenes\""));
            assert!(request.contains("\"execute_scene\":\"scene-a\""));
            stream.write_all(br#"{"ok":true,"generation":10,"executed_scene":"scene-a","activation_outcomes":[{"id":"a","outcome":"Succeeded"}]}
"#).expect("write response");
        });
        let body = br#"{"execute_scene":"scene-a"}"#;
        let request = HttpRequest::parse(format!(
            "POST /api/v1/scenes HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(), std::str::from_utf8(body).expect("JSON")
        ).as_bytes()).expect("request");
        let response = scenes_operation(&request, &socket);
        assert_eq!(response.status, 200);
        assert!(String::from_utf8_lossy(&response.body).contains("activation_outcomes"));
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[cfg(unix)]
    #[test]
    fn mutation_retry_is_idempotent_and_reused_id_is_rejected() {
        use std::{os::unix::net::UnixListener, thread};
        let socket = std::env::temp_dir().join(format!(
            "mackes-web-idempotent-operation-{}-{}",
            std::process::id(),
            NEXT_REQUEST_ID.load(Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client once");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            assert!(String::from_utf8(request)
                .expect("IPC UTF-8")
                .contains("\"command\":\"rescan\""));
            stream
                .write_all(
                    br#"{"ok":true,"generation":4}
"#,
                )
                .expect("write accepted response");
        });
        let request_for = |generation: u64| {
            let body = format!(
                "{{\"request_id\":\"idempotent-operation-{}\",\"operation\":\"rescan\",\"generation\":{generation}}}",
                std::process::id()
            );
            HttpRequest::parse(
                format!(
                    "POST /api/v1/operations HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(), body
                )
                .as_bytes(),
            )
            .expect("request")
        };
        let first = operation(&request_for(0), &socket);
        let retry = operation(&request_for(0), &socket);
        let changed = operation(&request_for(1), &socket);
        assert_eq!(first.status, 202);
        assert_eq!(retry.status, 202);
        assert_eq!(retry.body, first.body);
        assert_eq!(changed.status, 409);
        assert_eq!(changed.body, br#"{"code":"request_id_reuse"}"#);
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[cfg(unix)]
    #[test]
    fn simultaneous_duplicate_mutations_reach_daemon_once() {
        use std::{
            os::unix::net::UnixListener,
            sync::{Arc, Barrier},
            thread,
        };
        let socket = std::env::temp_dir().join(format!(
            "mackes-web-concurrent-idempotency-{}-{}",
            std::process::id(),
            NEXT_REQUEST_ID.load(Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept one web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            assert!(String::from_utf8(request)
                .expect("IPC UTF-8")
                .contains("\"command\":\"rescan\""));
            stream
                .write_all(
                    br#"{"ok":true,"generation":4}
"#,
                )
                .expect("write response");
        });
        let id = format!("concurrent-idempotency-{}", std::process::id());
        let body = format!(r#"{{"request_id":"{id}","operation":"rescan","generation":0}}"#);
        let request = HttpRequest::parse(format!(
            "POST /api/v1/operations HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", body.len(), body
        ).as_bytes()).expect("request");
        let barrier = Arc::new(Barrier::new(3));
        let first_barrier = Arc::clone(&barrier);
        let first_request = request.clone();
        let first_socket = socket.clone();
        let first = thread::spawn(move || {
            first_barrier.wait();
            operation(&first_request, &first_socket)
        });
        let second_barrier = Arc::clone(&barrier);
        let second_socket = socket.clone();
        let second = thread::spawn(move || {
            second_barrier.wait();
            operation(&request, &second_socket)
        });
        barrier.wait();
        let first_response = first.join().expect("first caller");
        let second_response = second.join().expect("second caller");
        assert_eq!(first_response.status, 202);
        assert_eq!(second_response.status, 202);
        assert_eq!(first_response.body, second_response.body);
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[cfg(unix)]
    #[test]
    fn mapping_route_forwards_typed_snapshot_to_daemon() {
        use std::{os::unix::net::UnixListener, thread};
        let socket =
            std::env::temp_dir().join(format!("mackes-web-mappings-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("IPC UTF-8");
            assert!(request.contains("\"command\":\"mappings\""));
            assert!(request.contains("\"operation\":\"Snapshot\""));
            stream
                .write_all(
                    br#"{"generation":7,"undo_available":false,"outcome":"Applied"}
"#,
                )
                .expect("write response");
        });
        let body = serde_json::to_vec(&mackes_ipc::MappingRequest {
            operation: mackes_ipc::MappingOperation::Snapshot,
            generation: 3,
            payload: None,
        })
        .expect("mapping request");
        let request = HttpRequest::parse(
            format!(
                "POST /api/v1/mappings HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                String::from_utf8(body).expect("JSON")
            )
            .as_bytes(),
        )
        .expect("request");
        let response = mapping_operation(&request, &socket);
        assert_eq!(response.status, 200);
        assert!(String::from_utf8(response.body).expect("JSON").contains("\"generation\":7"));
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[cfg(unix)]
    #[test]
    fn mapping_route_forwards_typed_lifecycle_mutation_to_daemon() {
        use std::{os::unix::net::UnixListener, thread};
        let socket = std::env::temp_dir().join(format!(
            "mackes-web-mapping-lifecycle-{}-{}",
            std::process::id(),
            NEXT_REQUEST_ID.load(Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("IPC UTF-8");
            assert!(request.contains("\"command\":\"mappings\""));
            assert!(request.contains("\"operation\":\"Enabled\""));
            assert!(request.contains("\"mapping_id\":\"mapping-a\""));
            stream
                .write_all(
                    br#"{"generation":8,"undo_available":true,"outcome":"Applied"}
"#,
                )
                .expect("write response");
        });
        let body = br#"{"operation":"Enabled","generation":7,"payload":{"kind":"Enabled","mapping_id":"mapping-a","enabled":false}}"#;
        let request = HttpRequest::parse(
            format!(
                "POST /api/v1/mappings HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                std::str::from_utf8(body).expect("JSON")
            )
            .as_bytes(),
        )
        .expect("request");
        let response = mapping_operation(&request, &socket);
        assert_eq!(response.status, 200);
        assert!(String::from_utf8(response.body).expect("JSON").contains("\"generation\":8"));
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[test]
    fn diagnostics_bundle_is_bounded_and_daemon_independent() {
        let request = HttpRequest::parse(
            b"GET /api/v1/diagnostics/bundle HTTP/1.1\r\nHost: localhost:8081\r\n\r\n",
        )
        .expect("request");
        let response = route(&request, &PathBuf::from("/private/socket"), "http://localhost:8081");
        let body = String::from_utf8(response.body).expect("JSON");
        assert_eq!(response.status, 200);
        assert!(body.contains("mackes-diagnostics-v1"));
        assert!(body.contains("host logs are not collected"));
        for condition in
            ["port_conflict", "missing_device", "disk_full", "malformed_config", "permission"]
        {
            assert!(body.contains(condition), "missing recovery condition {condition}");
        }
        assert!(body.contains("preview_url"));
        assert!(body.contains("restart_required_after_bind_change"));
    }

    #[test]
    fn event_route_does_not_accept_prefix_collision() {
        let request =
            HttpRequest::parse(b"GET /api/v1/eventsfoo HTTP/1.1\r\nHost: localhost:8081\r\n\r\n")
                .expect("request");
        let response = route(&request, &PathBuf::from("/missing"), "http://localhost:8081");
        assert_eq!(response.status, 404);
    }

    #[test]
    fn event_stream_path_requires_exact_boundary() {
        assert!(is_event_stream_path("/api/v1/events/stream"));
        assert!(is_event_stream_path("/api/v1/events/stream?after_sequence=4"));
        assert!(!is_event_stream_path("/api/v1/events/streamfoo"));
        assert!(!is_event_stream_path("/api/v1/events/stream/extra"));
    }

    #[test]
    fn event_route_rejects_invalid_sequence_cursor_before_ipc() {
        let request = HttpRequest::parse(
            b"GET /api/v1/events?after_sequence=not-a-number HTTP/1.1\r\nHost: localhost:8081\r\n\r\n",
        )
        .expect("request");
        let response = route(&request, &PathBuf::from("/missing"), "http://localhost:8081");
        assert_eq!(response.status, 400);
        assert!(String::from_utf8(response.body).expect("error").contains("nonnegative"));
    }

    #[test]
    fn event_route_rejects_ambiguous_query_before_ipc() {
        for path in
            ["/api/v1/events?after_sequence=1&after_sequence=2", "/api/v1/events?unexpected=1"]
        {
            let request = HttpRequest::parse(
                format!("GET {path} HTTP/1.1\r\nHost: localhost:8081\r\n\r\n").as_bytes(),
            )
            .expect("request");
            assert_eq!(
                route(&request, &PathBuf::from("/missing"), "http://localhost:8081").status,
                400
            );
        }
    }

    #[test]
    fn oversized_content_length_is_rejected_before_daemon_access() {
        let request = HttpRequest::parse(
            format!(
                "GET /api/v1/state HTTP/1.1\r\nHost: localhost:8081\r\nContent-Length: {}\r\n\r\n",
                mackes_web_contract::MAX_HTTP_BODY_BYTES + 1
            )
            .as_bytes(),
        );
        assert!(request.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn health_route_forwards_to_daemon_ipc() {
        use std::{os::unix::net::UnixListener, thread};
        let socket = std::env::temp_dir().join(format!("mackes-web-ipc-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            assert!(String::from_utf8(request)
                .expect("IPC UTF-8")
                .contains("\"command\":\"health\""));
            stream
                .write_all(
                    br#"{"ok":true,"generation":7,"health":"ready"}
"#,
                )
                .expect("write response");
        });
        let request =
            HttpRequest::parse(b"GET /api/v1/health HTTP/1.1\r\nHost: localhost:8081\r\n\r\n")
                .expect("request");
        let response = route(&request, &socket, "http://localhost:8081");
        assert_eq!(response.status, 200);
        assert!(String::from_utf8(response.body).expect("JSON").contains("ready"));
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[cfg(unix)]
    #[test]
    fn backup_inventory_route_forwards_to_daemon_ipc() {
        use std::{os::unix::net::UnixListener, thread};
        let socket =
            std::env::temp_dir().join(format!("mackes-web-backups-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            assert!(String::from_utf8(request)
                .expect("IPC UTF-8")
                .contains("\"command\":\"backups\""));
            stream
                .write_all(
                    br#"{"ok":true,"generation":3,"backups":[]}
"#,
                )
                .expect("write response");
        });
        let request =
            HttpRequest::parse(b"GET /api/v1/backups HTTP/1.1\r\nHost: localhost:8081\r\n\r\n")
                .expect("request");
        let response = route(&request, &socket, "http://localhost:8081");
        assert_eq!(response.status, 200);
        assert!(String::from_utf8(response.body).expect("JSON").contains("backups"));
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[test]
    fn portable_import_requires_confirmation_and_content_before_ipc() {
        for body in [
            br#"{"action":"portable_import","content":"{}"}"#.as_slice(),
            br#"{"action":"portable_import","confirm":true}"#.as_slice(),
        ] {
            let request = HttpRequest::parse(
                format!(
                    "POST /api/v1/backups HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    std::str::from_utf8(body).expect("JSON")
                )
                .as_bytes(),
            )
            .expect("request");
            assert_eq!(
                route(&request, &PathBuf::from("/never-open"), "http://localhost:8081").status,
                400
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn backup_restore_route_forwards_confirmed_request() {
        use std::{os::unix::net::UnixListener, thread};
        let socket =
            std::env::temp_dir().join(format!("mackes-web-backup-restore-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("IPC UTF-8");
            assert!(
                request.contains("\"command\":\"backups\"")
                    && request.contains("\"action\":\"restore\"")
            );
            stream
                .write_all(
                    br#"{"ok":true,"generation":4,"restored":true}
"#,
                )
                .expect("write response");
        });
        let body = br#"{"action":"restore","name":"mackes.json5.backup","confirm":true}"#;
        let request = HttpRequest::parse(format!("POST /api/v1/backups HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", body.len(), std::str::from_utf8(body).expect("JSON")).as_bytes()).expect("request");
        let response = route(&request, &socket, "http://localhost:8081");
        assert_eq!(response.status, 200);
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[cfg(unix)]
    #[test]
    fn portable_import_route_forwards_confirmed_content() {
        use std::{os::unix::net::UnixListener, thread};
        let socket =
            std::env::temp_dir().join(format!("mackes-web-portable-import-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("IPC UTF-8");
            assert!(
                request.contains("\"command\":\"backups\"")
                    && request.contains("\"action\":\"portable_import\"")
                    && request.contains("\"confirm\":true")
            );
            stream
                .write_all(
                    br#"{"ok":true,"generation":5,"portable_imported":true}
"#,
                )
                .expect("write response");
        });
        let body = br#"{"action":"portable_import","content":"{schema_version:1}","confirm":true}"#;
        let request = HttpRequest::parse(
            format!(
                "POST /api/v1/backups HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                std::str::from_utf8(body).expect("JSON")
            )
            .as_bytes(),
        )
        .expect("request");
        let response = route(&request, &socket, "http://localhost:8081");
        assert_eq!(response.status, 200);
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[test]
    fn capability_route_is_available_without_daemon() {
        let request = HttpRequest::parse(
            b"GET /api/v1/capabilities HTTP/1.1\r\nHost: localhost:8081\r\n\r\n",
        )
        .expect("request");
        let response = route(&request, &PathBuf::from("/missing"), "http://localhost:8081");
        let body = String::from_utf8(response.body).expect("JSON");
        assert_eq!(response.status, 200);
        assert!(body.contains("implemented_with_confirmation"));
        assert!(body.contains("typed_ipc"));
        assert!(body.contains("\"pipedal\":\"implemented_as_typed_ipc\""));
        assert!(body.contains("\"device_control\":\"implemented_with_confirmation\""));
        assert!(body.contains("\"sysex\":\"implemented_with_confirmation\""));
        assert!(body.contains("\"assignment\":\"implemented_as_typed_ipc\""));
        assert!(body.contains("\"events\":\"implemented_as_poll_and_sse\""));
        assert!(!body.contains("long_lived_events"));
        assert!(!body.contains("W135"));
        assert!(!body.contains("W137-W139"));
        assert!(body.contains("W138-W139"));
        assert!(body.contains("remaining_mutations"));
        assert!(body.contains("\"operation\""));
    }

    #[cfg(unix)]
    #[test]
    fn event_poll_forwards_sequence_cursor() {
        use std::{os::unix::net::UnixListener, thread};
        let socket = std::env::temp_dir().join(format!("mackes-web-events-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind fake daemon");
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept web client");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while stream.read(&mut byte).expect("read request") == 1 {
                request.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("IPC UTF-8");
            assert!(request.contains("\"command\":\"subscribe\""));
            assert!(request.contains("after_sequence"));
            assert!(request.contains("42"));
            stream
                .write_all(
                    br#"{"ok":true,"events":[],"last_sequence":42}
"#,
                )
                .expect("write response");
        });
        let request = HttpRequest::parse(
            b"GET /api/v1/events?after_sequence=42 HTTP/1.1\r\nHost: localhost:8081\r\n\r\n",
        )
        .expect("request");
        let response = route(&request, &socket, "http://localhost:8081");
        assert_eq!(response.status, 200);
        assert!(String::from_utf8(response.body).expect("JSON").contains("last_sequence"));
        worker.join().expect("fake daemon");
        let _ = std::fs::remove_file(socket);
    }

    #[test]
    fn bundled_shell_is_served_from_same_origin() {
        let request =
            HttpRequest::parse(b"GET / HTTP/1.1\r\nHost: localhost:8081\r\n\r\n").expect("request");
        let response = route(
            &request,
            &std::env::temp_dir().join(format!("mackes-web-shell-{}", std::process::id())),
            "http://localhost:8081",
        );
        assert_eq!(response.status, 200);
        assert!(!response.json);
        let html = String::from_utf8(response.body).expect("HTML");
        assert!(html.contains("MACKES MIDI Matrix"));
        assert!(html.contains("Panic all outputs"));
        assert!(html.contains("Previous scene"));
        assert!(html.contains("Next scene"));
        assert!(html.contains("Monitor"));
        assert!(html.contains("Map Controls"));
        assert!(html.contains("Scenes &amp; Setlists"));
        assert!(html.contains("System"));
        assert!(html.contains("Validate configuration"));
        assert!(html.contains("Inspect backups"));
        assert!(html.contains("id=\"backup-choice\""));
        assert!(html.contains("Create configuration backup"));
        assert!(html.contains("Restore configuration backup"));
        assert!(html.contains("Qualified device commands"));
        assert!(html.contains("open-qualified-commands"));
        assert!(!html.contains("id=\"sysex-destination\""));
        assert!(!html.contains("id=\"sysex-bytes\""));
        assert!(!html.contains("id=\"sysex-confirm\""));
        assert!(html.contains("Export configuration"));
        assert!(html.contains("Export raw configuration"));
        assert!(html.contains("Export portable configuration"));
        assert!(html.contains("Import portable configuration"));
        assert!(html.contains("id=\"portable-import-file\""));
        assert!(html.contains("Send device control"));
        assert!(html.contains("Refresh PiPedal catalog"));
        assert!(html.contains("Run PiPedal operation"));
        assert!(html.contains("id=\"pipedal-operation-choice\""));
        assert!(html.contains("id=\"pipedal-instance-id\""));
        assert!(html.contains("id=\"pipedal-mapping-choice\""));
        assert!(html.contains("id=\"pipedal-instance-id\""));
        assert!(html.contains("id=\"pipedal-value\""));
        assert!(html.contains("id=\"pipedal-confirm\""));
        assert!(html.contains("Start assignment"));
        assert!(html.contains("Capture selected control"));
        assert!(html.contains("assignment-choice"));
        assert!(html.contains("Undo last route change"));
        assert!(html.contains("id=\"routing-preview\""));
        assert!(html.contains("Apply routes"));
        assert!(html.contains("Undo last mapping"));
        assert!(html.contains("Commit assignment"));
        assert!(html.contains("Download capture"));
        assert!(html.contains("Novation physical control faceplate"));
        assert!(html.contains("viewBox=\"0 0 1000 560\""));
        assert!(html.contains("novation-diagnostics"));
        assert!(html.contains("Novation control grid"));
        assert!(html.contains("class=\"action-group\"><h3>Recovery"));
        assert!(html.contains("class=\"action-group\"><summary>Configuration"));
        assert!(html.contains("Select scene"));
        assert!(html.contains("id=\"preview-scene\""));
        assert!(html.contains("id=\"scene-id\""));
        assert!(html.contains("Refresh scenes first"));
        assert!(html.contains("Refresh scenes and setlists"));
        assert!(html.contains("Use light theme"));
        assert!(html.contains("Keyboard help"));
        assert!(html.contains("reconnect-banner"));
        assert!(html.contains("Daemon connection lost"));
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn bundled_shell_script_preserves_deep_links_and_history() {
        let request =
            HttpRequest::parse(b"GET /assets/app.js HTTP/1.1\r\nHost: localhost:8081\r\n\r\n")
                .expect("request");
        let response = route(&request, &PathBuf::from("/missing"), "http://localhost:8081");
        let script = String::from_utf8(response.body).expect("JavaScript");
        assert_eq!(response.status, 200);
        assert!(script.contains("window.history.pushState"));
        assert!(script.contains("window.addEventListener('popstate'"));
        assert!(script.contains("viewFromLocation()"));
        assert!(script.contains("beforeunload"));
        assert!(script.contains("dirtyForm"));
        assert!(script.contains("setHidden(reconnectBanner"));
        assert!(script.contains("window.addEventListener('offline'"));
        assert!(script.contains("window.addEventListener('online'"));
        assert!(script.contains("if (eventLog.length > 256) eventLog.shift()"));
        assert!(script.contains("monitorPaused = !monitorPaused"));
        assert!(script.contains("eventLog.length = 0"));
        assert!(script.contains("format: 'mackes-monitor-v1'"));
        assert!(script.contains("Downloaded ${visibleEvents().length} bounded monitor events."));
        assert!(script.contains("const monitorFilter"));
        assert!(script.contains("visibleEvents()"));
        assert!(script.contains("Event rate:"));
        assert!(script.contains("presentation only"));
        assert!(script.contains("eventTimes.length = 0"));
        assert!(script.contains("Monitor resynchronized from authoritative state."));
        assert!(
            script.contains("Conflict: state changed elsewhere; reloading authoritative state.")
        );
        assert!(script.contains("Operation outcome unknown; inspect state before retrying"));
        assert!(script.contains("operation.dataset.state = 'pending'"));
        assert!(script.contains("operation.setAttribute('aria-busy', 'true')"));
        assert!(script.contains("operation.dataset.state = 'unknown'"));
        assert!(script.contains("request.enabled = operationName === 'enable'"));
        assert!(script.contains("supportsBypass"));
        assert!(script.contains("['Enable', 'enable']"));
        assert!(script.contains("use_mod_ui = true"));
        assert!(script.contains("supportsTitle"));
        assert!(script.contains("['Rename', 'rename']"));
        assert!(script.contains("request.color_key = colorKey"));
        assert!(script.contains("body: JSON.stringify({ preview_scene: scene })"));
        assert!(script.contains(
            "Scene preview outcome unknown; inspect authoritative state before retrying"
        ));
        assert!(script.contains("body: JSON.stringify({ preview_setlist: id })"));
        assert!(script.contains("Dry-run only; no scene selected or executed."));
        assert!(script.contains(
            "Setlist preview outcome unknown; inspect authoritative state before retrying"
        ));
        assert!(script
            .contains("body: JSON.stringify({ setlist_copy: { source: id, new_id: newId } })"));
        assert!(script
            .contains("Setlist copy outcome unknown; inspect authoritative state before retrying"));
        assert!(script.contains("body: JSON.stringify({ setlist_delete: id })"));
        assert!(script.contains("Delete setlist ${id}? This does not delete its projects."));
        assert!(script.contains(
            "Setlist deletion outcome unknown; inspect authoritative state before retrying"
        ));
        assert!(script
            .contains("body: JSON.stringify({ project_copy: { source: id, new_id: newId } })"));
        assert!(script
            .contains("Project copy outcome unknown; inspect authoritative state before retrying"));
        assert!(script.contains("schema_version: 1, project: entry"));
        assert!(script.contains("Project ${id} exported from authoritative state."));
        assert!(script.contains("Refreshing authoritative scenes, projects, and setlists"));
        assert!(script.contains("body: JSON.stringify({ setlist: { id, projects } })"));
        assert!(script.contains(
            "Setlist reorder outcome unknown; inspect authoritative state before retrying"
        ));
        assert!(script.contains("schema_version: 1, setlist: { id, projects"));
        assert!(script.contains("Setlist ${id} exported from authoritative state."));
        assert!(script.contains("Replace projects in setlist ${id} with imported ordering?"));
        assert!(script.contains(
            "Setlist import outcome unknown; inspect authoritative state before retrying"
        ));
        assert!(script.contains("body: JSON.stringify({ setlist_create: { id, projects: [] } })"));
        assert!(script.contains(
            "Setlist creation outcome unknown; inspect authoritative state before retrying"
        ));
        assert!(script.contains("Assignment outcome unknown; inspect state before retrying"));
        assert!(script
            .contains("new EventSource(`/api/v1/events/stream?after_sequence=${lastSequence}`)"));
        assert!(script.contains("consumeStreamEvent"));
        assert!(script.contains("if (!eventStream) pollEvents()"));
        assert!(script.contains("event.type === 'resnapshot'"));
        assert!(script.contains("portableImportFile.files?.[0]"));
        assert!(script.contains("file.size > 1024 * 1024"));
        assert!(script.contains("new TextEncoder().encode(content).length"));
        assert!(script.contains("portableImportFile.value = ''"));
        assert!(script.contains("open-qualified-commands"));
        assert!(!script.contains("#sysex-confirm').checked"));
        assert!(!script.contains("bytes.length > 1024"));
        assert!(!script.contains("sysex-bytes').value = ''"));
        assert!(script.contains("renderFaceplate(body)"));
        assert!(script.contains("function boundedFetch("));
        assert!(script.contains("const browserSmoke = window.location.hash.includes"));
        assert!(script.contains("if (!browserSmoke) startEventStream()"));
        assert!(script.contains(
            "boundedFetch('/api/v1/novation', { signal: abortController.signal }, 12000)"
        ));
        assert!(script.contains("let viewLoadSequence = 0"));
        assert!(script.contains("let activeViewAbortController = null"));
        assert!(script.contains("activeViewAbortController?.abort()"));
        assert!(script.contains("new AbortController()"));
        assert!(script.contains("document.addEventListener('visibilitychange'"));
        assert!(script.contains("if (document.hidden || browserSmoke) return;"));
        assert!(script.contains("const loadSequence = ++viewLoadSequence"));
        assert!(script.contains("if (loadSequence !== viewLoadSequence) return;"));
        assert!(script.contains("fetch('/api/v1/mappings', { signal: controller.signal })"));
        assert!(script.contains("renderFaceplate({ ...novationBody, mapping_registry:"));
        assert!(script.contains("setHidden(novationGridHeading"));
        assert!(script.contains("body.mapping_registry"));
        assert!(script.contains("led=${mapping.led || 'unspecified'}"));
        assert!(script.contains("knob-r${row}-c${col + 1}"));
        assert!(script.contains("document.createElementNS(namespace, name)"));
        assert!(script.contains("tabindex: 0, role: 'button'"));
        assert!(script.contains("event.key === 'Enter' || event.key === ' '"));
        assert!(script.contains("control.addEventListener('click', select)"));
        assert!(script.contains("control.addEventListener('keydown'"));
        assert!(script.contains("class: 'assignment-label'"));
        assert!(script.contains("assignment.length > 14"));
        assert!(script.contains("Novation assignments: ${assigned.length} active of ${ids.length}"));
        assert!(script.contains("boundedFetch('/api/v1/health', {}, 5500)"));
        assert!(script.contains("window.MackesHealth.status"));
        assert!(script.contains("window.setInterval(refreshActiveView, 10000)"));
        assert!(script.contains("mapping-save-behavior"));
        assert!(script.contains("mapping-preview"));
        assert!(script.contains("explicit Apply remains required"));
        assert!(script.contains("acknowledged; awaiting observed state."));
        assert!(script.contains("observed after authoritative refresh."));
        assert!(script.contains("rejected; draft remains available for correction."));
        assert!(script.contains("outcome unknown; inspect authoritative state before retrying."));
        assert!(script.contains("operation: 'Behavior'"));
        assert!(script.contains("mappingMutation('Enabled'"));
        assert!(script.contains("mappingMutation('Delete'"));
        assert!(script.contains("mapping-toggle-enabled"));
        assert!(script.contains("workspaceInspector"));
        assert!(script.contains("showInspector("));
        assert!(script.contains("Destination: ${destination}"));
        assert!(script.contains("Source: ${source}"));
        assert!(script.contains("Current value: unavailable until authoritative device readback."));
        assert!(!script.contains("mapping id: ${mapping.id"));
        assert!(script.contains("routeEndpointLossless"));
        assert!(script.contains(
            "Route apply blocked: endpoint identifiers require a lossless numeric contract."
        ));
        assert!(script.contains(
            "Readback and value domain are unknown until the authoritative profile supplies them."
        ));
        assert!(script.contains("readback/domain unknown"));
        assert!(script.contains("workspace selected. Refresh to inspect authoritative state."));
        assert!(script.contains("focusedId"));
        assert!(script.contains("preventScroll: true"));
        assert!(script.contains("Novation device refreshed"));
        assert!(script.contains("Novation diagnostics unavailable in authoritative response."));
        assert!(script.contains("favoritesReadback"));
        assert!(script.contains("favorite plugin identities"));
        assert!(script.contains("systemMidiReadback"));
        assert!(script.contains("system MIDI bindings"));
        assert!(script.contains("aria-description"));
        assert!(script.contains("value range 0 to 127"));
        assert!(script.contains("Unavailable until the qualified device is connected."));
        assert!(script.contains("Route ${Number(card.dataset.routeIndex) + 1} selected"));
        let shell_request =
            HttpRequest::parse(b"GET / HTTP/1.1\r\nHost: localhost:8081\r\n\r\n").expect("request");
        let shell = String::from_utf8(
            route(&shell_request, &PathBuf::from("/missing"), "http://localhost:8081").body,
        )
        .expect("HTML");
        assert!(shell.contains("mapping-source-min"));
        assert!(shell.contains("mapping-delete"));
        assert!(shell.contains("workspace-inspector"));
    }

    #[test]
    fn workspace_deep_links_serve_the_shell() {
        for path in [
            "/state",
            "/mappings",
            "/routes",
            "/scenes",
            "/devices",
            "/devices/novation",
            "/recovery",
            "/system",
            "/system/configuration",
            "/system/configuration/raw",
            "/system/backups",
            "/system/diagnostics",
            "/monitor",
        ] {
            let request = HttpRequest::parse(
                format!("GET {path} HTTP/1.1\r\nHost: localhost:8081\r\n\r\n").as_bytes(),
            )
            .expect("request");
            let response = route(&request, &PathBuf::from("/missing"), "http://localhost:8081");
            assert_eq!(response.status, 200, "deep link {path}");
            assert!(!response.json, "deep link {path} returned JSON");
        }
    }

    #[test]
    fn bundled_styles_expose_shared_design_tokens() {
        let request =
            HttpRequest::parse(b"GET /assets/app.css HTTP/1.1\r\nHost: localhost:8081\r\n\r\n")
                .expect("request");
        let response = route(&request, &PathBuf::from("/missing"), "http://localhost:8081");
        let css = String::from_utf8(response.body).expect("CSS");
        assert_eq!(response.status, 200);
        for token in [
            "--cds-background",
            "--cds-spacing-01",
            "--cds-spacing-06",
            "--cds-body-compact-01",
            "--cds-layer",
            "--cds-text-primary",
            "--cds-interactive",
            "--cds-support-error",
            "--cds-focus",
        ] {
            assert!(css.contains(token), "missing shared token {token}");
        }
        assert!(css.contains("body.light"));
        assert!(css.contains("IBM Plex Sans"));
        assert!(css.contains("min-height: 2.75rem"));
        assert!(css.contains("@media (max-width: 42rem)"));
        assert!(css.contains("prefers-reduced-motion"));
    }

    #[test]
    fn bundled_navigation_asset_is_served_same_origin() {
        let request = HttpRequest::parse(
            b"GET /assets/navigation.js HTTP/1.1\r\nHost: localhost:8081\r\n\r\n",
        )
        .expect("request");
        let response = route(&request, &PathBuf::from("/missing"), "http://localhost:8081");
        let script = String::from_utf8(response.body).expect("JavaScript");
        assert_eq!(response.status, 200);
        assert!(script.contains("MackesNavigation"));
    }

    #[test]
    fn bundled_feature_catalog_asset_is_served_same_origin() {
        let request = HttpRequest::parse(
            b"GET /assets/feature_catalog.js HTTP/1.1\r\nHost: localhost:8081\r\n\r\n",
        )
        .expect("request");
        let response = route(&request, &PathBuf::from("/missing"), "http://localhost:8081");
        let script = String::from_utf8(response.body).expect("JavaScript");
        assert_eq!(response.status, 200);
        assert!(script.contains("MackesFeatureCatalog"));
        assert!(script.contains("Novation Launch Control XL"));
        assert!(script.contains("entriesFor"));
    }

    #[test]
    fn bundled_feature_renderer_asset_is_served_same_origin() {
        let request = HttpRequest::parse(
            b"GET /assets/feature_renderer.js HTTP/1.1\r\nHost: localhost:8081\r\n\r\n",
        )
        .expect("request");
        let response = route(&request, &PathBuf::from("/missing"), "http://localhost:8081");
        let script = String::from_utf8(response.body).expect("JavaScript");
        assert_eq!(response.status, 200);
        assert!(script.contains("MackesFeatureRenderer"));
        assert!(script.contains("searchableText"));
    }

    #[test]
    fn bundled_device_renderer_asset_is_served_same_origin() {
        let request = HttpRequest::parse(
            b"GET /assets/device_renderer.js HTTP/1.1\r\nHost: localhost:8081\r\n\r\n",
        )
        .expect("request");
        let response = route(&request, &PathBuf::from("/missing"), "http://localhost:8081");
        let script = String::from_utf8(response.body).expect("JavaScript");
        assert_eq!(response.status, 200);
        assert!(script.contains("MackesDeviceRenderer"));
        assert!(script.contains("generic.endpoint"));
    }

    #[test]
    fn bundled_state_store_asset_is_served_same_origin() {
        let request = HttpRequest::parse(
            b"GET /assets/state_store.js HTTP/1.1\r\nHost: localhost:8081\r\n\r\n",
        )
        .expect("request");
        let response = route(&request, &PathBuf::from("/missing"), "http://localhost:8081");
        let script = String::from_utf8(response.body).expect("JavaScript");
        assert_eq!(response.status, 200);
        assert!(script.contains("MackesStateStore"));
        assert!(script.contains("generation"));
    }

    #[test]
    fn favicon_is_served_same_origin_for_clean_browser_console() {
        let request =
            HttpRequest::parse(b"GET /favicon.ico HTTP/1.1\r\nHost: localhost:8081\r\n\r\n")
                .expect("request");
        let response = route(&request, &PathBuf::from("/missing"), "http://localhost:8081");
        assert_eq!(response.status, 200);
        assert!(String::from_utf8_lossy(&response.body).contains("<svg"));
    }

    #[test]
    fn read_cache_coalesces_identical_reads_per_socket_and_command() {
        let socket = PathBuf::from(format!("/tmp/mackes-read-cache-{}", std::process::id()));
        store_read(&socket, Command::Health, br#"{"ok":true}"#);
        assert_eq!(cached_read(&socket, Command::Health), Some(br#"{"ok":true}"#.to_vec()));
        store_read(&socket, Command::DeviceQuery, br#"{"devices":[]}"#);
        assert_eq!(cached_read(&socket, Command::DeviceQuery), Some(br#"{"devices":[]}"#.to_vec()));
        store_read(&socket, Command::Endpoints, br#"{"endpoints":[]}"#);
        assert_eq!(cached_read(&socket, Command::Endpoints), Some(br#"{"endpoints":[]}"#.to_vec()));
        assert!(cached_read(&socket, Command::Mappings).is_none());
        assert!(cached_read(&PathBuf::from("/tmp/other-socket"), Command::Health).is_none());
    }
}
