//! Persistent daemon lifecycle and local command boundary.

use mackes_config::{load, ConfigDocument, ConfigError};
use mackes_ipc::{authorize, AccessPolicy, Authorization, Command, LocalServer};
use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};

/// Daemon health state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Health {
    /// Daemon is loading configuration and endpoints.
    Starting,
    /// Daemon is accepting normal work.
    Ready,
    /// Daemon is operating with one or more degraded dependencies.
    Degraded,
    /// Daemon is shutting down.
    Stopping,
}

/// Formats a bounded JSON diagnostic line suitable for journald ingestion.
#[must_use]
pub fn structured_log_line(level: &str, event: &str, detail: &str) -> String {
    let bounded_detail: String = detail.chars().take(512).collect();
    serde_json::json!({"level": level, "event": event, "detail": bounded_detail}).to_string() + "\n"
}

const fn health_after_authorized_command(current: Health, command: Option<Command>) -> Health {
    if matches!(command, Some(Command::Health)) {
        current
    } else {
        Health::Ready
    }
}

impl Health {
    /// Returns whether the daemon may accept normal mutations.
    #[must_use]
    pub const fn is_operational(self) -> bool {
        matches!(self, Self::Ready | Self::Degraded)
    }
}

/// Startup restore result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreResult {
    /// Loaded project identifier, if any.
    pub active_project: Option<String>,
    /// Last active scene, when persisted and valid.
    pub active_scene: Option<String>,
    /// Ordered scene identifiers selected for ordinary activation.
    pub scenes: Vec<String>,
    /// Whether startup may transmit the ordinary scene plan.
    pub should_activate: bool,
    /// Actions held because unsafe mode starts disarmed.
    pub unsafe_actions_blocked: usize,
}

impl RestoreResult {
    /// Selects the persisted scene, falling back deterministically to the first scene.
    #[must_use]
    pub fn activation_scene(&self) -> Option<&str> {
        self.active_scene.as_deref().or_else(|| self.scenes.first().map(String::as_str))
    }
}

/// Bounded startup window used while required endpoint aliases settle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EndpointSettlePolicy {
    /// Maximum wait in milliseconds.
    pub window_ms: u64,
}

impl Default for EndpointSettlePolicy {
    fn default() -> Self {
        Self { window_ms: 5_000 }
    }
}

impl EndpointSettlePolicy {
    /// Creates a policy, rejecting an unbounded zero-length window.
    #[must_use]
    pub const fn new(window_ms: u64) -> Option<Self> {
        if window_ms == 0 {
            None
        } else {
            Some(Self { window_ms })
        }
    }

    /// Returns the monotonic deadline for the settle window.
    #[must_use]
    pub const fn deadline_ms(self, started_ms: u64) -> u64 {
        started_ms.saturating_add(self.window_ms)
    }

    /// Classifies endpoint readiness at a monotonic timestamp.
    #[must_use]
    pub const fn classify(self, started_ms: u64, now_ms: u64, required_ready: bool) -> SettleState {
        if required_ready {
            SettleState::Ready
        } else if now_ms < self.deadline_ms(started_ms) {
            SettleState::Settling
        } else {
            SettleState::TimedOut
        }
    }
}

/// Result of checking required endpoint readiness during startup restore.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettleState {
    /// All required endpoint aliases are available.
    Ready,
    /// The bounded settle window remains open.
    Settling,
    /// The settle window elapsed without all required endpoints.
    TimedOut,
}

/// Validated restore data paired with endpoint readiness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreReadiness {
    /// Validated persisted project and scene selection.
    pub restore: RestoreResult,
    /// Current endpoint prerequisite state.
    pub endpoints: SettleState,
}

impl RestoreReadiness {
    /// Returns whether ordinary startup activation may begin.
    #[must_use]
    pub const fn may_activate(&self) -> bool {
        self.restore.should_activate && matches!(self.endpoints, SettleState::Ready)
    }
}

/// Returns whether every required endpoint alias is present in discovery results.
#[must_use]
pub fn required_endpoints_ready(
    required_aliases: &[&str],
    endpoints: &[mackes_midi_engine::EndpointInfo],
) -> bool {
    required_aliases
        .iter()
        .all(|required| endpoints.iter().any(|endpoint| endpoint.id == *required))
}

/// Combines discovered endpoint readiness with the bounded startup settle window.
#[must_use]
pub fn settle_required_endpoints(
    policy: EndpointSettlePolicy,
    started_ms: u64,
    now_ms: u64,
    required_aliases: &[&str],
    endpoints: &[mackes_midi_engine::EndpointInfo],
) -> SettleState {
    policy.classify(started_ms, now_ms, required_endpoints_ready(required_aliases, endpoints))
}

/// Loads persisted restore state and evaluates endpoint prerequisites atomically.
///
/// # Errors
///
/// Returns the configuration error when persisted state cannot be loaded or validated.
pub fn startup_restore_readiness(
    path: &Path,
    policy: EndpointSettlePolicy,
    started_ms: u64,
    now_ms: u64,
    required_aliases: &[&str],
    endpoints: &[mackes_midi_engine::EndpointInfo],
) -> Result<RestoreReadiness, ConfigError> {
    Ok(RestoreReadiness {
        restore: startup_restore(path)?,
        endpoints: settle_required_endpoints(
            policy,
            started_ms,
            now_ms,
            required_aliases,
            endpoints,
        ),
    })
}

/// Single-instance lock held for the daemon lifetime.
#[derive(Debug)]
pub struct InstanceLock {
    path: PathBuf,
    _file: fs::File,
}

impl InstanceLock {
    /// Acquires an exclusive lock using atomic file creation.
    ///
    /// # Errors
    ///
    /// Returns `AlreadyExists` when another daemon owns the lock.
    pub fn acquire(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_owned();
        let file = OpenOptions::new().write(true).create_new(true).open(&path)?;
        Ok(Self { path, _file: file })
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Persistent daemon state and local IPC listener.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct Daemon {
    server: LocalServer,
    health: Health,
    generation: u64,
    active_scene: Option<String>,
    scene_ids: Vec<String>,
    catalog: serde_json::Value,
    physical_devices: serde_json::Value,
    config_path: Option<std::path::PathBuf>,
    router: mackes_midi_engine::RouterStore,
    rtp_peer: mackes_midi_engine::RtpMidiPeer,
    outputs: mackes_midi_engine::OutputRegistry,
    inputs: mackes_midi_engine::InputRegistry,
    virtual_ports: Option<mackes_midi_engine::VirtualMidiPorts>,
    virtual_ingress: Receiver<(u64, Vec<u8>)>,
    virtual_ingress_tx: Sender<(u64, Vec<u8>)>,
    virtual_sequence: u64,
    received_events: u64,
    sent_events: u64,
    dropped_events: u64,
    activity: mackes_midi_engine::ActivityCoalescer,
    last_activity: Option<serde_json::Value>,
    last_activity_publish: Instant,
    activation_result: Option<String>,
    state_sequence: u64,
    state_events: VecDeque<mackes_ipc::StateEvent>,
}

/// Classifies a bounded JSON command tag without deserializing untrusted payloads.
#[cfg(target_os = "linux")]
#[must_use]
pub fn classify_command(request: &[u8]) -> Option<Command> {
    const COMMANDS: &[(Command, &[u8])] = &[
        (Command::Hello, b"hello"),
        (Command::Snapshot, b"snapshot"),
        (Command::Subscribe, b"subscribe"),
        (Command::Validate, b"validate"),
        (Command::Configuration, b"configuration"),
        (Command::Endpoints, b"endpoints"),
        (Command::Routes, b"routes"),
        (Command::Learn, b"learn"),
        (Command::Scenes, b"scenes"),
        (Command::DeviceQuery, b"device_query"),
        (Command::DeviceControl, b"device_control"),
        (Command::Sysex, b"sysex"),
        (Command::Backups, b"backups"),
        (Command::Monitor, b"monitor"),
        (Command::Health, b"health"),
        (Command::Panic, b"panic"),
        (Command::UnsafeMode, b"unsafe_mode"),
        (Command::Shutdown, b"shutdown"),
    ];
    COMMANDS.iter().find_map(|(command, tag)| {
        let mut needle = Vec::with_capacity(tag.len() + 12);
        needle.extend_from_slice(b"\"command\":\"");
        needle.extend_from_slice(tag);
        needle.push(b'"');
        request.windows(needle.len()).any(|window| window == needle).then_some(*command)
    })
}

#[cfg(target_os = "linux")]
fn physical_devices_json(endpoints: &[mackes_midi_engine::EndpointInfo]) -> String {
    let devices = mackes_midi_engine::group_physical_devices(endpoints)
        .into_iter()
        .map(|device| {
            serde_json::json!({
                "id": device.id,
                "name": device.name,
                "inputs": device.inputs,
                "outputs": device.outputs,
                "state": format!("{:?}", device.state).to_lowercase(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&devices).unwrap_or_else(|_| "[]".to_owned())
}

#[cfg(target_os = "linux")]
fn physical_devices_value(endpoints: &[mackes_midi_engine::EndpointInfo]) -> serde_json::Value {
    serde_json::from_str(&physical_devices_json(endpoints))
        .unwrap_or_else(|_| serde_json::json!([]))
}

#[cfg(target_os = "linux")]
fn routes_path(config_path: &std::path::Path) -> std::path::PathBuf {
    config_path.with_extension("routes.json")
}

#[cfg(target_os = "linux")]
fn persist_routes(config_path: &std::path::Path, routes: &serde_json::Value) -> io::Result<()> {
    let path = routes_path(config_path);
    let temporary = path.with_extension("routes.json.tmp");
    let bytes = serde_json::to_vec_pretty(routes).map_err(io::Error::other)?;
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(temporary, path)
}

#[cfg(target_os = "linux")]
fn midi_activity_json(
    event: &mackes_domain::MidiEvent,
    routed: &[mackes_midi_engine::RoutedEvent],
    stable_endpoint: Option<&str>,
) -> serde_json::Value {
    let (kind, number, value) = match &event.message {
        mackes_domain::MidiMessage::ControlChange { controller, value, .. } => {
            ("control_change", Some(controller.as_u8()), Some(u16::from(value.as_u8())))
        }
        mackes_domain::MidiMessage::NoteOn { note, velocity, .. } => {
            ("note_on", Some(note.as_u8()), Some(u16::from(velocity.as_u8())))
        }
        mackes_domain::MidiMessage::NoteOff { note, velocity, .. } => {
            ("note_off", Some(note.as_u8()), Some(u16::from(velocity.as_u8())))
        }
        mackes_domain::MidiMessage::ProgramChange { program, .. } => {
            ("program_change", Some(program.as_u8()), None)
        }
        mackes_domain::MidiMessage::PitchBend { value, .. } => {
            ("pitch_bend", None, Some(value.get()))
        }
        mackes_domain::MidiMessage::PolyPressure { note, pressure, .. } => {
            ("poly_pressure", Some(note.as_u8()), Some(u16::from(pressure.as_u8())))
        }
        mackes_domain::MidiMessage::ChannelPressure { pressure, .. } => {
            ("channel_pressure", None, Some(u16::from(pressure.as_u8())))
        }
        mackes_domain::MidiMessage::SysEx(_) => ("sysex", None, None),
        mackes_domain::MidiMessage::SystemCommon(_) => ("system_common", None, None),
        mackes_domain::MidiMessage::Realtime(_) => ("realtime", None, None),
    };
    let endpoint_key =
        stable_endpoint.map_or_else(|| event.endpoint.get().to_string(), str::to_owned);
    let control_id = number.map_or_else(
        || format!("endpoint:{endpoint_key}:{kind}"),
        |number| format!("endpoint:{endpoint_key}:{kind}:{number}"),
    );
    serde_json::json!({
        "source_endpoint": event.endpoint.get(),
        "source_endpoint_id": stable_endpoint,
        "control_id": control_id,
        "timestamp_nanos": event.timestamp.get(),
        "kind": kind,
        "number": number,
        "value": value,
        "destination_endpoints": routed.iter().map(|item| item.event.endpoint.get()).collect::<Vec<_>>(),
        "sequence": event.sequence,
    })
}

/// Produces a stable acknowledgment for a recognized command.
#[cfg(target_os = "linux")]
fn command_ack(
    command: Command,
    health: Health,
    generation: u64,
    endpoints: &[mackes_midi_engine::EndpointInfo],
    route_generation: Option<u64>,
    routes: &[mackes_midi_engine::Route],
) -> String {
    match command {
        Command::Health => {
            let label = match health {
                Health::Starting => "starting",
                Health::Ready => "ready",
                Health::Degraded => "degraded",
                Health::Stopping => "stopping",
            };
            format!("{{\"ok\":true,\"generation\":{generation},\"health\":\"{label}\"}}\n")
        }
        Command::Panic => format!("{{\"ok\":true,\"generation\":{generation},\"panic\":true}}\n"),
        Command::Hello => format!("{{\"ok\":true,\"generation\":{generation},\"protocol\":1}}\n"),
        Command::Snapshot => {
            format!("{{\"ok\":true,\"generation\":{generation},\"snapshot\":true}}\n")
        }
        Command::Subscribe => {
            format!("{{\"ok\":true,\"generation\":{generation},\"subscribed\":true}}\n")
        }
        Command::Scenes => format!("{{\"ok\":true,\"generation\":{generation},\"scenes\":[]}}\n"),
        Command::Routes => {
            let payload = routes
                .iter()
                .map(|route| {
                    serde_json::json!({
                        "source": route.source.get(), "destination": route.destination.get(),
                        "destination_parameter": route.destination_parameter,
                        "channel": route.channel.map(mackes_domain::MidiChannel::one_based),
                        "class": route.class.map(|class| format!("{class:?}")),
                        "allow_cycle": route.allow_cycle,
                        "enabled": route.enabled,
                        "priority": route.priority,
                        "curve": match route.curve {
                            mackes_midi_engine::Curve::Linear => "linear",
                            mackes_midi_engine::Curve::Square => "square",
                            mackes_midi_engine::Curve::SquareRoot => "square_root",
                        },
                        "predicates": route.predicates,
                    })
                })
                .collect::<Vec<_>>();
            let encoded = serde_json::to_string(&payload).unwrap_or_else(|_| "[]".to_owned());
            format!("{{\"ok\":true,\"generation\":{generation},\"routes\":{encoded},\"route_generation\":{}}}\n", route_generation.unwrap_or(0))
        }
        Command::Learn => {
            format!("{{\"ok\":true,\"generation\":{generation},\"learn\":true}}\n")
        }
        Command::DeviceQuery => {
            let devices = endpoints
                .iter()
                .map(|endpoint| {
                    serde_json::json!({
                        "id": endpoint.id,
                        "name": endpoint.name,
                        "direction": format!("{:?}", endpoint.direction).to_lowercase(),
                    })
                })
                .collect::<Vec<_>>();
            format!(
                "{{\"ok\":true,\"generation\":{generation},\"devices\":{},\"physical_devices\":{}}}\n",
                serde_json::to_string(&devices).unwrap_or_else(|_| "[]".into()),
                physical_devices_json(endpoints)
            )
        }
        Command::DeviceControl => {
            format!("{{\"ok\":true,\"generation\":{generation},\"device_control\":true}}\n")
        }
        Command::Monitor => format!("{{\"ok\":true,\"generation\":{generation},\"monitor\":[]}}\n"),
        Command::Backups => format!("{{\"ok\":true,\"generation\":{generation},\"backups\":[]}}\n"),
        Command::Configuration => {
            format!("{{\"ok\":true,\"generation\":{generation},\"configuration\":true}}\n")
        }
        Command::Endpoints => {
            let payload = endpoints
                .iter()
                .map(|endpoint| {
                    serde_json::json!({
                        "id": endpoint.id,
                        "name": endpoint.name,
                        "direction": format!("{:?}", endpoint.direction).to_lowercase(),
                    })
                })
                .collect::<Vec<_>>();
            let encoded = serde_json::to_string(&payload).unwrap_or_else(|_| "[]".to_owned());
            let physical_encoded = physical_devices_json(endpoints);
            format!("{{\"ok\":true,\"generation\":{generation},\"endpoints\":{encoded},\"physical_devices\":{physical_encoded}}}\n")
        }
        Command::Validate => {
            format!("{{\"ok\":true,\"generation\":{generation},\"valid\":true}}\n")
        }
        Command::Sysex => {
            format!("{{\"ok\":true,\"generation\":{generation},\"sysex\":true,\"unsafe_required\":true}}\n")
        }
        Command::UnsafeMode => {
            format!("{{\"ok\":true,\"generation\":{generation},\"unsafe_mode\":\"disarmed\"}}\n")
        }
        other @ Command::Shutdown => format!(
            "{{\"ok\":true,\"generation\":{generation},\"accepted\":true,\"command\":\"{}\"}}\n",
            other.tag()
        ),
    }
}

#[cfg(target_os = "linux")]
impl Daemon {
    /// Binds the daemon control socket.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the socket cannot be created.
    pub fn bind(control_path: impl AsRef<Path>) -> io::Result<Self> {
        let (virtual_ingress_tx, virtual_ingress) = mpsc::channel();
        Ok(Self {
            server: LocalServer::bind(control_path)?,
            health: Health::Starting,
            generation: 0,
            active_scene: None,
            scene_ids: Vec::new(),
            catalog: serde_json::json!({"projects": [], "setlists": []}),
            physical_devices: serde_json::json!([]),
            config_path: None,
            router: mackes_midi_engine::RouterStore::new(Vec::new(), 0, 8)
                .map_err(io::Error::other)?,
            rtp_peer: mackes_midi_engine::RtpMidiPeer::new(0, 32).map_err(io::Error::other)?,
            outputs: mackes_midi_engine::OutputRegistry::new(64),
            inputs: mackes_midi_engine::InputRegistry::new(64),
            virtual_ports: None,
            virtual_ingress,
            virtual_ingress_tx,
            virtual_sequence: 0,
            received_events: 0,
            sent_events: 0,
            dropped_events: 0,
            activity: mackes_midi_engine::ActivityCoalescer::new(128)
                .ok_or_else(|| io::Error::other("activity capacity must be positive"))?,
            last_activity: None,
            last_activity_publish: Instant::now(),
            activation_result: None,
            state_sequence: 0,
            state_events: VecDeque::with_capacity(256),
        })
    }

    /// Enables or disables nonblocking accepts for the daemon-owned control socket.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the listener cannot change mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.server.set_nonblocking(nonblocking)
    }

    /// Creates and owns the standard ALSA virtual MIDI pair until daemon shutdown.
    ///
    /// # Errors
    ///
    /// Returns the backend error when ALSA cannot create either virtual port.
    pub fn enable_virtual_ports(&mut self) -> Result<(), String> {
        if self.virtual_ports.is_some() {
            return Ok(());
        }
        let sender = self.virtual_ingress_tx.clone();
        let ports = mackes_midi_engine::create_virtual_ports(move |timestamp, bytes, ()| {
            let _ = sender.send((timestamp, bytes.to_vec()));
        })?;
        self.virtual_ports = Some(ports);
        Ok(())
    }

    /// Drains bounded virtual-port input into the normal router path.
    pub fn drain_virtual_input(&mut self) -> usize {
        let mut accepted = 0;
        while let Ok((timestamp, bytes)) = self.virtual_ingress.try_recv() {
            let Some(message) = mackes_domain::MidiMessage::from_wire(&bytes).ok() else {
                continue;
            };
            self.virtual_sequence = self.virtual_sequence.saturating_add(1);
            let Some(endpoint) = mackes_domain::EndpointId::new(1) else {
                continue;
            };
            let event = mackes_domain::MidiEvent {
                timestamp: mackes_domain::TimestampNanos::new(timestamp),
                sequence: self.virtual_sequence,
                endpoint,
                message,
            };
            let _ = self.outputs.dispatch(&self.router, &event);
            accepted += 1;
        }
        accepted
    }

    /// Captures one bounded poll from the selected daemon-owned input for MIDI Learn.
    ///
    /// Capture is observational: it never routes or transmits the collected events. Events from
    /// other endpoints are ignored, and candidate inference remains the shared engine contract.
    #[must_use]
    pub fn capture_learn_candidates(
        &mut self,
        endpoint: mackes_domain::EndpointId,
        limit: usize,
    ) -> Vec<mackes_midi_engine::MidiLearnCandidate> {
        if limit == 0 {
            return Vec::new();
        }
        let events = self
            .inputs
            .poll_once()
            .into_iter()
            .filter(|event| event.endpoint == endpoint)
            .take(limit.min(128))
            .collect::<Vec<_>>();
        mackes_midi_engine::infer_midi_candidates(&events)
    }

    /// Atomically installs a validated route generation for subsequent events.
    ///
    /// # Errors
    ///
    /// Returns a validation error for an invalid hop limit, self-loop, or
    /// poisoned route store.
    pub fn replace_routes(
        &self,
        routes: Vec<mackes_midi_engine::Route>,
        generation: u64,
        hop_limit: u8,
    ) -> Result<(), &'static str> {
        if self.route_generation().is_some_and(|current| generation <= current) {
            return Err("route generation is stale");
        }
        self.router.swap(routes, generation, hop_limit)
    }

    /// Replaces routes from a bounded JSON array using the daemon's validated route contract.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed JSON, missing fields, invalid channels,
    /// unknown message classes, or invalid route topology.
    pub fn replace_routes_json(
        &self,
        payload: &[u8],
        generation: u64,
        hop_limit: u8,
    ) -> Result<(), &'static str> {
        let values: Vec<serde_json::Value> =
            serde_json::from_slice(payload).map_err(|_| "invalid route JSON")?;
        if values.len() > 1024 {
            return Err("route list exceeds bound");
        }
        let mut routes = Vec::with_capacity(values.len());
        for value in values {
            let object = value.as_object().ok_or("route must be an object")?;
            let source = mackes_domain::EndpointId::new(
                object
                    .get("source")
                    .and_then(serde_json::Value::as_u64)
                    .ok_or("missing route source")?,
            )
            .ok_or("invalid route source")?;
            let destination = mackes_domain::EndpointId::new(
                object
                    .get("destination")
                    .and_then(serde_json::Value::as_u64)
                    .ok_or("missing route destination")?,
            )
            .ok_or("invalid route destination")?;
            let channel = object
                .get("channel")
                .and_then(serde_json::Value::as_u64)
                .map(|value| {
                    mackes_domain::MidiChannel::new(
                        u8::try_from(value).map_err(|_| "invalid route channel")?,
                    )
                    .ok_or("invalid route channel")
                })
                .transpose()?;
            let class = match object.get("class").and_then(serde_json::Value::as_str) {
                None => None,
                Some("Note") => Some(mackes_midi_engine::MessageClass::Note),
                Some("ControlChange") => Some(mackes_midi_engine::MessageClass::ControlChange),
                Some("ProgramChange") => Some(mackes_midi_engine::MessageClass::ProgramChange),
                Some("PitchBend") => Some(mackes_midi_engine::MessageClass::PitchBend),
                Some("SysEx") => Some(mackes_midi_engine::MessageClass::SysEx),
                Some("Other") => Some(mackes_midi_engine::MessageClass::Other),
                Some(_) => return Err("unknown route class"),
            };
            routes.push(mackes_midi_engine::Route {
                source,
                destination,
                destination_parameter: object
                    .get("destination_parameter")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                channel,
                class,
                enabled: object.get("enabled").and_then(serde_json::Value::as_bool).unwrap_or(true),
                priority: object
                    .get("priority")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|value| u16::try_from(value).ok())
                    .unwrap_or(0),
                curve: match object.get("curve").and_then(serde_json::Value::as_str) {
                    None | Some("linear") => mackes_midi_engine::Curve::Linear,
                    Some("square") => mackes_midi_engine::Curve::Square,
                    Some("square_root") => mackes_midi_engine::Curve::SquareRoot,
                    Some(_) => return Err("unknown route curve"),
                },
                predicates: object
                    .get("predicates")
                    .map(|value| {
                        serde_json::from_value(value.clone())
                            .map_err(|_| "invalid route predicates")
                    })
                    .transpose()?
                    .unwrap_or_default(),
                allow_cycle: object
                    .get("allow_cycle")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            });
        }
        self.replace_routes(routes, generation, hop_limit)
    }

    /// Evaluates one ingress event against the current route generation.
    #[must_use]
    pub fn route_event(
        &self,
        event: &mackes_domain::MidiEvent,
    ) -> Vec<mackes_midi_engine::RoutedEvent> {
        self.router.route(event)
    }

    /// Routes and dispatches one ingress event to supplied output adapters.
    ///
    /// The caller owns adapter lifetimes and therefore controls whether the
    /// outputs are virtual, physical, or a test double.
    #[must_use]
    pub fn dispatch_event(
        &self,
        event: &mackes_domain::MidiEvent,
        outputs: &mut [&mut dyn mackes_midi_engine::MidiOutputAdapter],
    ) -> (usize, usize) {
        mackes_midi_engine::dispatch_routed_event(&self.router, event, outputs)
    }

    /// Pumps one captured physical input event through routing and outputs.
    ///
    /// # Errors
    ///
    /// Returns the MIDI decoder error when the queued input is malformed.
    pub fn pump_input(
        &self,
        input: &mut mackes_midi_engine::MidirInputCapture,
        timestamp: mackes_domain::TimestampNanos,
        sequence: u64,
        outputs: &mut [&mut dyn mackes_midi_engine::MidiOutputAdapter],
    ) -> Result<Option<(usize, usize)>, &'static str> {
        let Some(event) = input.receive_event(timestamp, sequence)? else { return Ok(None) };
        Ok(Some(self.dispatch_event(&event, outputs)))
    }

    /// Registers one explicitly opened output adapter with the daemon.
    ///
    /// # Errors
    ///
    /// Returns an error when the registry is full or the endpoint ID is duplicated.
    pub fn register_output(
        &mut self,
        output: Box<dyn mackes_midi_engine::MidiOutputAdapter>,
    ) -> Result<(), &'static str> {
        self.outputs.insert(output)
    }

    /// Opens and registers a physical ALSA output by stable endpoint ID.
    ///
    /// # Errors
    ///
    /// Returns an adapter/backend or registry validation error.
    pub fn provision_output(&mut self, endpoint_id: &str) -> Result<(), String> {
        let output = mackes_midi_engine::MidirOutputAdapter::open_id(endpoint_id)?;
        self.register_output(Box::new(output)).map_err(str::to_owned)
    }

    /// Dispatches one event through the daemon-owned output registry.
    #[must_use]
    pub fn dispatch_registered(&mut self, event: &mackes_domain::MidiEvent) -> (usize, usize) {
        let _ = self.activity.push(event);
        let stable_endpoint = self.inputs.stable_id_for_endpoint(event.endpoint);
        let routed = self.route_event(event);
        let (sent, unmatched) = self.outputs.dispatch(&self.router, event);
        self.received_events = self.received_events.saturating_add(1);
        self.sent_events = self.sent_events.saturating_add(sent as u64);
        self.dropped_events = self.dropped_events.saturating_add(unmatched as u64);
        let first_activity = self.last_activity.is_none();
        self.last_activity = Some(midi_activity_json(event, &routed, stable_endpoint.as_deref()));
        // Publish the post-dispatch counters so subscribed dashboards receive live activity
        // without needing a second command to trigger a journal append.
        if first_activity || self.last_activity_publish.elapsed() >= Duration::from_millis(33) {
            self.last_activity_publish = Instant::now();
            self.record_state_event(Command::Monitor);
        }
        (sent, unmatched)
    }

    /// Returns bounded aggregate MIDI activity counters.
    #[must_use]
    pub const fn activity_counters(&self) -> (u64, u64, u64) {
        (self.received_events, self.sent_events, self.dropped_events)
    }

    /// Records a validated startup scene for dashboard snapshots and events.
    pub fn set_active_scene(&mut self, scene: Option<String>) {
        self.active_scene = scene;
        self.record_state_event(Command::Scenes);
    }

    /// Returns the currently selected scene projected by the daemon.
    #[must_use]
    pub fn active_scene(&self) -> Option<&str> {
        self.active_scene.as_deref()
    }

    /// Selects the next or previous scene in the daemon-owned catalog.
    pub fn navigate_scene(&mut self, next: bool) -> Option<String> {
        if self.scene_ids.is_empty() {
            return self.active_scene.clone();
        }
        let current = self
            .active_scene
            .as_deref()
            .and_then(|scene| self.scene_ids.iter().position(|id| id == scene))
            .unwrap_or(0);
        let index = if next {
            (current + 1) % self.scene_ids.len()
        } else {
            current.checked_sub(1).unwrap_or(self.scene_ids.len() - 1)
        };
        let scene = self.scene_ids[index].clone();
        self.set_active_scene(Some(scene.clone()));
        Some(scene)
    }

    /// Selects an exact scene from the daemon-owned catalog and persists it.
    ///
    /// # Errors
    ///
    /// Returns an error when the scene is absent or persistence fails.
    pub fn select_scene(&mut self, scene: &str) -> Result<String, &'static str> {
        if !self.scene_ids.iter().any(|candidate| candidate == scene) {
            return Err("scene is not present in the active catalog");
        }
        let selected = scene.to_owned();
        self.set_active_scene(Some(selected.clone()));
        if let Some(path) = self.config_path.as_deref() {
            persist_active_scene(path, Some(scene)).map_err(|_| "scene persistence failed")?;
            let document = mackes_config::load(path).map_err(|_| "scene load failed")?;
            let active_project = document
                .settings
                .active_project
                .as_deref()
                .and_then(|id| document.projects.iter().find(|project| project.id == id))
                .ok_or("active project is unavailable")?;
            let scene_ref = active_project
                .scenes
                .iter()
                .find(|candidate| candidate.id == scene)
                .ok_or("scene is not present in the active project")?;
            let plan = compile_scene_actions(scene_ref).map_err(|_| "scene plan is invalid")?;
            let outputs = &mut self.outputs;
            let results = plan.execute_with(false, false, |action| {
                match (&action.destination, &action.message) {
                    (Some(destination), Some(message)) => outputs
                        .send_direct(destination, message)
                        .map_or(mackes_scene_engine::ActionResult::Failed, |()| {
                            mackes_scene_engine::ActionResult::Succeeded
                        }),
                    _ => mackes_scene_engine::ActionResult::Succeeded,
                }
            });
            self.publish_activation_result(&results);
            if results
                .iter()
                .any(|(_, result)| matches!(result, mackes_scene_engine::ActionResult::Failed))
            {
                return Err("scene operation failed");
            }
        }
        Ok(selected)
    }

    /// Registers one explicitly opened input adapter with the daemon.
    ///
    /// # Errors
    ///
    /// Returns an error when the registry is full or the endpoint ID is duplicated.
    pub fn register_input(
        &mut self,
        input: Box<dyn mackes_midi_engine::MidiInputAdapter>,
    ) -> Result<(), &'static str> {
        self.inputs.insert(input)
    }

    /// Opens and registers a physical ALSA input by exact backend name.
    ///
    /// # Errors
    ///
    /// Returns an adapter/backend or registry validation error.
    pub fn provision_input(&mut self, name: &str) -> Result<(), String> {
        let input = mackes_midi_engine::MidirInputCapture::open_named(name)?;
        self.register_input(Box::new(input)).map_err(str::to_owned)
    }

    /// Polls each daemon-owned input once in stable registration order.
    #[must_use]
    pub fn poll_inputs(&mut self) -> Vec<mackes_domain::MidiEvent> {
        self.inputs.poll_once()
    }

    /// Polls registered MIDI inputs and routes a bounded batch through the normal path.
    ///
    /// The bound prevents a busy physical controller from starving IPC and scene work.
    pub fn poll_and_dispatch_inputs(&mut self, limit: usize) -> (usize, usize, usize) {
        let events = self.poll_inputs();
        let mut processed = 0;
        let mut sent = 0;
        let mut unmatched = 0;
        for event in events.into_iter().take(limit.min(128)) {
            let (event_sent, event_unmatched) = self.dispatch_registered(&event);
            processed += 1;
            sent += event_sent;
            unmatched += event_unmatched;
        }
        (processed, sent, unmatched)
    }

    /// Polls registered inputs and resolves persisted dashboard bindings.
    ///
    /// Only the three non-destructive dashboard commands are returned. The
    /// bound prevents a busy input from starving daemon work; unmatched and
    /// ambiguous triggers are ignored fail-closed.
    #[must_use]
    pub fn poll_dashboard_commands(
        &mut self,
        bindings: &[mackes_config::DashboardMidiBinding],
        limit: usize,
    ) -> Vec<Command> {
        self.poll_dashboard_actions(bindings, limit)
            .into_iter()
            .map(|(command, _)| command)
            .collect()
    }

    /// Polls registered inputs and retains dashboard scene direction.
    #[must_use]
    fn poll_dashboard_actions(
        &mut self,
        bindings: &[mackes_config::DashboardMidiBinding],
        limit: usize,
    ) -> Vec<(Command, bool)> {
        let events = self.inputs.poll_once();
        let mut commands = Vec::with_capacity(limit.min(32));
        for event in events.into_iter().take(limit.min(128)) {
            let mut matched: Option<(Command, bool)> = None;
            for binding in bindings {
                let trigger_matches = match (&binding.trigger, &event.message) {
                    (
                        mackes_config::DashboardMidiTrigger::NoteOn { channel, note },
                        mackes_domain::MidiMessage::NoteOn {
                            channel: actual_channel,
                            note: actual_note,
                            ..
                        },
                    ) => {
                        *channel >= 1
                            && *channel <= 16
                            && *note <= 127
                            && actual_channel.one_based() == *channel
                            && actual_note.as_u8() == *note
                    }
                    (
                        mackes_config::DashboardMidiTrigger::ControlChange {
                            channel,
                            controller,
                            value,
                        },
                        mackes_domain::MidiMessage::ControlChange {
                            channel: actual_channel,
                            controller: actual_controller,
                            value: actual_value,
                        },
                    ) => {
                        *channel >= 1
                            && *channel <= 16
                            && *controller <= 127
                            && value.is_none_or(|expected| {
                                expected <= 127 && expected == actual_value.as_u8()
                            })
                            && actual_channel.one_based() == *channel
                            && actual_controller.as_u8() == *controller
                    }
                    _ => false,
                };
                if trigger_matches {
                    if matched.is_some() {
                        matched = None;
                        break;
                    }
                    matched = match binding.command.as_str() {
                        "panic" => Some((Command::Panic, true)),
                        "next_scene" => Some((Command::Scenes, true)),
                        "previous_scene" => Some((Command::Scenes, false)),
                        _ => None,
                    };
                }
            }
            if let Some(command) = matched {
                commands.push(command);
            }
        }
        commands
    }

    /// Polls and records mapped dashboard commands for the daemon loop.
    #[must_use]
    pub fn process_dashboard_commands(
        &mut self,
        bindings: &[mackes_config::DashboardMidiBinding],
        limit: usize,
    ) -> Vec<Command> {
        let actions = self.poll_dashboard_actions(bindings, limit);
        for (command, next) in &actions {
            if *command == Command::Scenes {
                self.navigate_scene(*next);
                self.record_state_event(*command);
            } else {
                let _ = self.handle_dashboard_command(*command);
            }
        }
        actions.into_iter().map(|(command, _)| command).collect()
    }

    /// Handles one daemon-owned dashboard command using the same bounded
    /// acknowledgment and journal path as a local IPC command.
    #[must_use]
    pub fn handle_dashboard_command(&mut self, command: Command) -> String {
        if !matches!(command, Command::Panic | Command::Scenes) {
            return "{\"ok\":false,\"error\":\"dashboard command is not allowed\"}\n".to_owned();
        }
        if command == Command::Scenes {
            self.navigate_scene(true);
        } else if command == Command::Panic {
            let _ = self.send_panic_controls();
            self.record_state_event(command);
        } else {
            self.record_state_event(command);
        }
        command_ack(command, self.health, self.generation, &[], self.route_generation(), &[])
    }

    /// Sends bounded All Notes Off and All Sound Off controls on every channel
    /// to every currently registered output.
    fn send_panic_controls(&mut self) -> (usize, usize) {
        let destinations = self.outputs.output_ids();
        let mut sent = 0;
        let mut failed = 0;
        for destination in destinations {
            for channel in 0_u8..16 {
                for controller in [120_u8, 123] {
                    let payload = [0xB0 | channel, controller, 0];
                    if self.outputs.send_direct(&destination, &payload).is_ok() {
                        sent += 1;
                        self.sent_events = self.sent_events.saturating_add(1);
                    } else {
                        failed += 1;
                        self.dropped_events = self.dropped_events.saturating_add(1);
                    }
                }
            }
        }
        (sent, failed)
    }

    /// Polls owned inputs and dispatches all decoded events through owned outputs.
    #[must_use]
    pub fn pump_registered_inputs(&mut self) -> Vec<(usize, usize)> {
        let events = self.inputs.poll_once();
        events.iter().map(|event| self.outputs.dispatch(&self.router, event)).collect()
    }

    /// Returns the active route generation, if the store lock is healthy.
    #[must_use]
    pub fn route_generation(&self) -> Option<u64> {
        self.router.generation()
    }

    /// Establishes the configured RTP-MIDI peer identity and resets sequence state.
    ///
    /// # Errors
    ///
    /// Returns an error when the supplied identity does not match the pending invitation.
    pub fn establish_rtp_peer(&mut self, token: u32, ssrc: u32) -> Result<(), &'static str> {
        self.rtp_peer.establish(token, ssrc)
    }

    /// Validates one RTP-MIDI packet against the configured peer and allowlist.
    ///
    /// # Errors
    ///
    /// Returns a session, peer-allowlist, framing, or sequence-policy error.
    pub fn receive_rtp_packet(
        &mut self,
        packet: &[u8],
        peer: std::net::SocketAddr,
        allowed_peers: &[std::net::SocketAddr],
        token: u32,
        ssrc: u32,
    ) -> Result<mackes_midi_engine::SequenceDisposition, &'static str> {
        self.rtp_peer
            .receive_packet(packet, peer, allowed_peers, token, ssrc)
            .map(|(_, disposition)| disposition)
    }

    /// Receives and validates one datagram from a configured RTP-MIDI transport.
    ///
    /// # Errors
    ///
    /// Returns socket, allowlist, session, framing, or sequence-policy errors.
    pub fn receive_rtp_from_transport(
        &mut self,
        transport: &mackes_midi_engine::UdpMidiTransport,
        allowlist: &mackes_midi_engine::PeerAllowlist,
        token: u32,
        ssrc: u32,
    ) -> std::io::Result<Option<(Vec<u8>, mackes_midi_engine::SequenceDisposition)>> {
        self.rtp_peer.receive_from_transport(transport, allowlist, token, ssrc)
    }

    /// Drains at most `limit` authorized RTP-MIDI datagrams from a transport.
    ///
    /// The bound makes one daemon loop iteration predictable and prevents a
    /// busy peer from starving local MIDI inputs and IPC work.
    ///
    /// # Errors
    ///
    /// Returns an invalid-input error for a zero limit, or the underlying
    /// transport/session error while receiving a datagram.
    pub fn poll_rtp_transport(
        &mut self,
        transport: &mackes_midi_engine::UdpMidiTransport,
        allowlist: &mackes_midi_engine::PeerAllowlist,
        token: u32,
        ssrc: u32,
        limit: usize,
    ) -> std::io::Result<Vec<(Vec<u8>, mackes_midi_engine::SequenceDisposition)>> {
        if limit == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "RTP poll limit must be nonzero",
            ));
        }
        let mut packets = Vec::with_capacity(limit.min(64));
        while packets.len() < limit {
            match self.receive_rtp_from_transport(transport, allowlist, token, ssrc)? {
                Some(packet) => packets.push(packet),
                None => break,
            }
        }
        Ok(packets)
    }

    /// Receives one authorized RTP-MIDI packet, decodes channel-voice events,
    /// and dispatches them through the daemon-owned router/output registry.
    /// System-common/realtime and `SysEx` packets use the dedicated sibling
    /// pumps below so callers cannot accidentally process a packet twice.
    ///
    /// # Errors
    ///
    /// Returns transport, framing, endpoint, or channel-message errors.
    pub fn pump_rtp_channel_transport(
        &mut self,
        transport: &mackes_midi_engine::UdpMidiTransport,
        allowlist: &mackes_midi_engine::PeerAllowlist,
        token: u32,
        ssrc: u32,
        endpoint: u64,
        sequence_start: u64,
    ) -> std::io::Result<Option<(usize, usize, usize)>> {
        let Some((packet, _disposition)) =
            self.receive_rtp_from_transport(transport, allowlist, token, ssrc)?
        else {
            return Ok(None);
        };
        let parsed =
            mackes_midi_engine::parse_rtp_midi_packet(&packet).map_err(std::io::Error::other)?;
        let events = mackes_midi_engine::rtp_channel_events(
            parsed.midi.commands,
            endpoint,
            u64::from(parsed.rtp.timestamp),
            sequence_start,
        )
        .map_err(std::io::Error::other)?;
        let mut sent = 0;
        let mut unmatched = 0;
        for event in &events {
            let (event_sent, event_unmatched) = self.dispatch_registered(event);
            sent += event_sent;
            unmatched += event_unmatched;
        }
        Ok(Some((events.len(), sent, unmatched)))
    }

    /// Receives one authorized RTP-MIDI packet and dispatches its system
    /// common/realtime messages. Channel-voice messages are rejected here so
    /// callers cannot accidentally process a packet twice.
    ///
    /// # Errors
    ///
    /// Returns transport, framing, endpoint, or system-message errors.
    pub fn pump_rtp_system_transport(
        &mut self,
        transport: &mackes_midi_engine::UdpMidiTransport,
        allowlist: &mackes_midi_engine::PeerAllowlist,
        token: u32,
        ssrc: u32,
        endpoint: u64,
        sequence_start: u64,
    ) -> std::io::Result<Option<(usize, usize, usize)>> {
        let Some((packet, _)) =
            self.receive_rtp_from_transport(transport, allowlist, token, ssrc)?
        else {
            return Ok(None);
        };
        let parsed =
            mackes_midi_engine::parse_rtp_midi_packet(&packet).map_err(std::io::Error::other)?;
        let messages = mackes_midi_engine::decode_rtp_midi_system(parsed.midi.commands)
            .map_err(std::io::Error::other)?;
        let events = mackes_midi_engine::rtp_system_events(
            &messages,
            endpoint,
            u64::from(parsed.rtp.timestamp),
            sequence_start,
        )
        .map_err(std::io::Error::other)?;
        let mut sent = 0;
        let mut unmatched = 0;
        for event in &events {
            let (event_sent, event_unmatched) = self.dispatch_registered(event);
            sent += event_sent;
            unmatched += event_unmatched;
        }
        Ok(Some((events.len(), sent, unmatched)))
    }

    /// Receives one authorized RTP-MIDI packet containing one complete `SysEx`
    /// command and dispatches it through the daemon-owned router.
    ///
    /// # Errors
    ///
    /// Returns transport, framing, endpoint, or `SysEx` validation errors.
    pub fn pump_rtp_sysex_transport(
        &mut self,
        transport: &mackes_midi_engine::UdpMidiTransport,
        allowlist: &mackes_midi_engine::PeerAllowlist,
        token: u32,
        ssrc: u32,
        endpoint: u64,
        sequence: u64,
    ) -> std::io::Result<Option<(usize, usize, usize)>> {
        let Some((packet, _)) =
            self.receive_rtp_from_transport(transport, allowlist, token, ssrc)?
        else {
            return Ok(None);
        };
        let parsed =
            mackes_midi_engine::parse_rtp_midi_packet(&packet).map_err(std::io::Error::other)?;
        let event = mackes_midi_engine::rtp_sysex_event(
            parsed.midi.commands,
            endpoint,
            u64::from(parsed.rtp.timestamp),
            sequence,
        )
        .map_err(std::io::Error::other)?;
        let (sent, unmatched) = self.dispatch_registered(&event);
        Ok(Some((1, sent, unmatched)))
    }

    /// Ends the established RTP-MIDI session and clears sequence history.
    ///
    /// # Errors
    ///
    /// Returns an error when the supplied identity does not match the active peer.
    pub fn end_rtp_peer(&mut self, token: u32, ssrc: u32) -> Result<(), &'static str> {
        self.rtp_peer.end_session(token, ssrc)
    }

    /// Executes a compiled scene plan through the scene engine's deterministic
    /// dependency and safety policy. This boundary never transmits hardware
    /// bytes by itself; device actions are supplied by a future output adapter.
    #[must_use]
    pub fn plan_scene(
        &self,
        plan: &mackes_scene_engine::ActivationPlan,
        unsafe_armed: bool,
        cancelled: bool,
    ) -> Vec<(String, mackes_scene_engine::ActionResult)> {
        plan.execute(unsafe_armed, cancelled)
    }

    /// Executes a scene plan with a caller-provided device action executor.
    #[must_use]
    pub fn execute_scene_with<F>(
        &mut self,
        plan: &mackes_scene_engine::ActivationPlan,
        unsafe_armed: bool,
        cancelled: bool,
        execute_action: F,
    ) -> Vec<(String, mackes_scene_engine::ActionResult)>
    where
        F: FnMut(&mackes_scene_engine::ActivationAction) -> mackes_scene_engine::ActionResult,
    {
        let results = plan.execute_with(unsafe_armed, cancelled, execute_action);
        self.publish_activation_result(&results);
        results
    }

    /// Executes startup restore through the ordinary planner with unsafe mode disarmed.
    #[must_use]
    pub fn execute_startup_restore<F>(
        &mut self,
        plan: &mackes_scene_engine::ActivationPlan,
        execute_action: F,
    ) -> Vec<(String, mackes_scene_engine::ActionResult)>
    where
        F: FnMut(&mackes_scene_engine::ActivationAction) -> mackes_scene_engine::ActionResult,
    {
        self.execute_scene_with(plan, false, false, execute_action)
    }

    /// Executes a scene through the daemon boundary with a monotonic deadline.
    #[must_use]
    pub fn execute_scene_with_deadline<F>(
        &mut self,
        plan: &mackes_scene_engine::ActivationPlan,
        unsafe_armed: bool,
        cancelled: bool,
        now: u64,
        deadline: u64,
        execute_action: F,
    ) -> Vec<(String, mackes_scene_engine::ActionResult)>
    where
        F: FnMut(&mackes_scene_engine::ActivationAction) -> mackes_scene_engine::ActionResult,
    {
        let results =
            plan.execute_with_deadline(unsafe_armed, cancelled, now, deadline, execute_action);
        self.publish_activation_result(&results);
        results
    }

    fn publish_activation_result(
        &mut self,
        results: &[(String, mackes_scene_engine::ActionResult)],
    ) {
        let summary = mackes_scene_engine::ActivationSummary::from_results(results);
        self.activation_result = Some(format!(
            "total={} succeeded={} failed={} skipped={} cancelled={} unverified={}",
            summary.total(),
            summary.succeeded,
            summary.failed,
            summary.skipped,
            summary.cancelled,
            summary.sent_unverified
        ));
        self.record_state_event(Command::Scenes);
    }

    /// Discovers ALSA MIDI endpoints without opening a device or transmitting MIDI.
    ///
    /// # Errors
    ///
    /// Returns the backend error when ALSA cannot be initialized or a port name
    /// cannot be read.
    pub fn discover_endpoints(&self) -> Result<Vec<mackes_midi_engine::EndpointInfo>, String> {
        mackes_midi_engine::enumerate_midir_ports()
    }

    fn record_state_event(&mut self, command: Command) {
        self.state_sequence = self.state_sequence.saturating_add(1);
        let payload = serde_json::to_vec(&serde_json::json!({
            "command": command.tag(),
            "generation": self.generation,
            "route_generation": self.route_generation(),
            "active_scene": self.active_scene.as_deref(),
            "received": self.received_events,
            "sent": self.sent_events,
            "dropped": self.dropped_events,
            "last_activity": self.last_activity,
            "activation_result": self.activation_result.as_deref(),
            "catalog": self.catalog,
            "physical_devices": self.physical_devices,
            "health": match self.health {
                Health::Starting => "starting",
                Health::Ready => "ready",
                Health::Degraded => "degraded",
                Health::Stopping => "stopping",
            },
        }))
        .unwrap_or_else(|_| b"{}".to_vec());
        if self.state_events.len() == 256 {
            self.state_events.pop_front();
        }
        self.state_events
            .push_back(mackes_ipc::StateEvent { sequence: self.state_sequence, payload });
    }

    fn snapshot_response(&self) -> String {
        serde_json::json!({
            "ok": true,
            "generation": self.generation,
            "route_generation": self.route_generation(),
            "active_scene": self.active_scene.as_deref(),
            "received": self.received_events,
            "sent": self.sent_events,
            "dropped": self.dropped_events,
            "last_activity": self.last_activity,
            "activation_result": self.activation_result.as_deref(),
            "last_sequence": self.state_sequence,
            "catalog": self.catalog,
            "physical_devices": self.physical_devices,
            "health": match self.health {
                Health::Starting => "starting",
                Health::Ready => "ready",
                Health::Degraded => "degraded",
                Health::Stopping => "stopping",
            },
        })
        .to_string()
            + "\n"
    }

    fn subscribe_response(&self, request: &[u8]) -> String {
        let after = serde_json::from_slice::<serde_json::Value>(request)
            .ok()
            .and_then(|value| {
                value
                    .get("payload")
                    .and_then(|payload| payload.get("after_sequence"))
                    .or_else(|| value.get("after_sequence"))
                    .and_then(serde_json::Value::as_u64)
            })
            .unwrap_or(0);
        let oldest = self
            .state_events
            .front()
            .map_or_else(|| self.state_sequence.saturating_add(1), |event| event.sequence);
        if after.saturating_add(1) < oldest {
            return serde_json::json!({
                "ok": false,
                "error": "event_gap",
                "snapshot_required": true,
                "oldest_sequence": oldest,
                "last_sequence": self.state_sequence,
            })
            .to_string()
                + "\n";
        }
        let events = self
            .state_events
            .iter()
            .filter(|event| event.sequence > after)
            .map(|event| {
                serde_json::json!({
                    "sequence": event.sequence,
                    "payload": serde_json::from_slice::<serde_json::Value>(&event.payload)
                        .unwrap_or(serde_json::Value::Null),
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!({"ok": true, "events": events, "last_sequence": self.state_sequence})
            .to_string()
            + "\n"
    }

    /// Handles one local request and remains usable for subsequent clients.
    ///
    /// # Errors
    ///
    /// Returns an I/O error for accept/read/write failures.
    #[allow(clippy::too_many_lines)]
    pub fn serve_once(&mut self, policy: AccessPolicy) -> io::Result<()> {
        let _ = self.drain_virtual_input();
        let (mut stream, identity) = self.server.accept_authorized(policy)?;
        stream.set_read_timeout(Some(std::time::Duration::from_millis(100)))?;
        let mut request = Vec::new();
        let mut byte = [0_u8; 1];
        loop {
            let count = stream.read(&mut byte)?;
            if count == 0 {
                return Ok(());
            }
            request.push(byte[0]);
            if byte[0] == b'\n' || request.len() >= mackes_ipc::MAX_FRAME_BYTES {
                break;
            }
        }
        let actor = if identity.uid == 0 {
            mackes_ipc::ActorClass::LocalCli
        } else {
            mackes_ipc::ActorClass::LocalTui
        };
        let command = classify_command(&request);
        let response = if command
            .is_some_and(|command| authorize(command, actor) == Authorization::Allowed)
        {
            self.health = health_after_authorized_command(self.health, command);
            self.generation = self.generation.saturating_add(1);
            if command == Some(Command::Configuration) {
                let value =
                    serde_json::from_slice::<serde_json::Value>(&request).unwrap_or_default();
                if value.get("setlists").is_some() || value.get("learned_mappings").is_some() {
                    let Some(path) = self.config_path.clone() else {
                        return stream.write_all(
                            b"{\"ok\":false,\"error\":\"configuration path is unavailable\"}\n",
                        );
                    };
                    let Ok(mut document) = mackes_config::load(&path) else {
                        return stream.write_all(
                            b"{\"ok\":false,\"error\":\"configuration load failed\"}\n",
                        );
                    };
                    if let Some(values) = value.get("setlists") {
                        let Ok(setlists) =
                            serde_json::from_value::<Vec<mackes_config::Setlist>>(values.clone())
                        else {
                            return stream
                                .write_all(b"{\"ok\":false,\"error\":\"invalid setlists\"}\n");
                        };
                        document.setlists = setlists;
                    }
                    if let Some(values) = value.get("learned_mappings") {
                        let Ok(mappings) = serde_json::from_value::<
                            Vec<mackes_config::LearnedMapping>,
                        >(values.clone()) else {
                            return stream.write_all(
                                b"{\"ok\":false,\"error\":\"invalid learned mappings\"}\n",
                            );
                        };
                        for mapping in mappings {
                            let updated = mackes_config::add_learned_mapping(&document, mapping)
                                .map_err(io::Error::other);
                            let Ok(updated) = updated else {
                                return stream.write_all(
                                    b"{\"ok\":false,\"error\":\"invalid learned mapping\"}\n",
                                );
                            };
                            document = updated;
                        }
                    }
                    if let Err(error) = mackes_config::validate(&document) {
                        return stream.write_all(
                            format!("{{\"ok\":false,\"error\":\"{error}\"}}\n").as_bytes(),
                        );
                    }
                    if let Err(error) = mackes_config::save(&path, &document, 5) {
                        return stream.write_all(
                            format!("{{\"ok\":false,\"error\":\"{error}\"}}\n").as_bytes(),
                        );
                    }
                    self.catalog = serde_json::json!({"projects": document.projects.iter().map(|project| serde_json::json!({"id": project.id, "scenes": project.scenes.iter().map(|scene| scene.id.clone()).collect::<Vec<_>>() })).collect::<Vec<_>>(), "setlists": document.setlists, "learned_mappings": document.learned_mappings});
                }
            }
            if command == Some(Command::Routes) {
                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&request) {
                    if let Some(route_values) = value.get("routes") {
                        let generation = value
                            .get("route_generation")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(self.generation);
                        let hop_limit = value
                            .get("hop_limit")
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|value| u8::try_from(value).ok())
                            .unwrap_or(8);
                        let encoded = serde_json::to_vec(route_values).map_err(io::Error::other)?;
                        if let Err(error) =
                            self.replace_routes_json(&encoded, generation, hop_limit)
                        {
                            return stream.write_all(
                                format!("{{\"ok\":false,\"error\":\"{error}\"}}\n").as_bytes(),
                            );
                        }
                        if let Some(path) = self.config_path.as_deref() {
                            if let Err(error) =
                                persist_routes(path, &serde_json::json!({"routes": route_values}))
                            {
                                return stream.write_all(
                                    format!("{{\"ok\":false,\"error\":\"route persistence failed: {error}\"}}\n").as_bytes(),
                                );
                            }
                        }
                    }
                }
            }
            if command == Some(Command::Learn) {
                let value =
                    serde_json::from_slice::<serde_json::Value>(&request).unwrap_or_default();
                let endpoint = value
                    .get("endpoint")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(mackes_domain::EndpointId::new)
                    .or_else(|| {
                        value
                            .get("endpoint_id")
                            .and_then(serde_json::Value::as_str)
                            .and_then(mackes_midi_engine::numeric_endpoint_id)
                    });
                let limit = value
                    .get("limit")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|value| usize::try_from(value).ok())
                    .unwrap_or(128);
                if let Some(endpoint) = endpoint {
                    let candidates = self.capture_learn_candidates(endpoint, limit);
                    let encoded = candidates
                        .iter()
                        .map(|candidate| {
                            serde_json::json!({
                                "kind": format!("{:?}", candidate.kind).to_lowercase(),
                                "channel": candidate.channel,
                                "number": candidate.number,
                                "observations": candidate.observations,
                                "minimum": candidate.minimum,
                                "maximum": candidate.maximum,
                                "raw": candidate.raw,
                            })
                        })
                        .collect::<Vec<_>>();
                    return stream.write_all(
                        format!(
                            "{{\"ok\":true,\"generation\":{},\"candidates\":{}}}\n",
                            self.generation,
                            serde_json::to_string(&encoded).unwrap_or_else(|_| "[]".into())
                        )
                        .as_bytes(),
                    );
                }
                return stream
                    .write_all(b"{\"ok\":false,\"error\":\"learn endpoint is required\"}\n");
            }
            if command == Some(Command::Scenes) {
                let value =
                    serde_json::from_slice::<serde_json::Value>(&request).unwrap_or_default();
                if let Some(scene) = value.get("scene").and_then(serde_json::Value::as_str) {
                    if let Err(error) = self.select_scene(scene) {
                        return stream.write_all(
                            format!("{{\"ok\":false,\"error\":\"{error}\"}}\n").as_bytes(),
                        );
                    }
                } else {
                    let next = value.get("direction").and_then(serde_json::Value::as_str)
                        != Some("previous");
                    self.navigate_scene(next);
                }
            }
            if command == Some(Command::DeviceQuery) {
                let value =
                    serde_json::from_slice::<serde_json::Value>(&request).unwrap_or_default();
                if let Some(profile_id) =
                    value.get("profile_id").and_then(serde_json::Value::as_str)
                {
                    let Some(profile) = mackes_profiles::builtin_profile(profile_id) else {
                        return stream
                            .write_all(b"{\"ok\":false,\"error\":\"unknown device profile\"}\n");
                    };
                    if let Some(query_id) =
                        value.get("query_id").and_then(serde_json::Value::as_str)
                    {
                        let parameters = match value.get("parameters") {
                            None => Vec::new(),
                            Some(value) => {
                                let Some(bytes) = value.as_array() else {
                                    return stream.write_all(b"{\"ok\":false,\"error\":\"query parameters are invalid\"}\n");
                                };
                                let Some(parameters) = bytes
                                    .iter()
                                    .map(|byte| {
                                        byte.as_u64().and_then(|byte| u8::try_from(byte).ok())
                                    })
                                    .collect::<Option<Vec<_>>>()
                                else {
                                    return stream.write_all(b"{\"ok\":false,\"error\":\"query parameters are invalid\"}\n");
                                };
                                parameters
                            }
                        };
                        if parameters.len() > 128 {
                            return stream.write_all(
                                b"{\"ok\":false,\"error\":\"query parameters exceed bound\"}\n",
                            );
                        }
                        let Ok(request) = profile.render_query_request(query_id, &parameters)
                        else {
                            return stream.write_all(
                                b"{\"ok\":false,\"error\":\"query cannot be rendered\"}\n",
                            );
                        };
                        let Some(query) = profile.queries.iter().find(|query| query.id == query_id)
                        else {
                            return stream
                                .write_all(b"{\"ok\":false,\"error\":\"query not found\"}\n");
                        };
                        let Some(reply) =
                            profile.replies.iter().find(|reply| reply.id == query.reply_id)
                        else {
                            return stream.write_all(
                                b"{\"ok\":false,\"error\":\"query reply not found\"}\n",
                            );
                        };
                        let response = serde_json::json!({
                            "ok": true,
                            "generation": self.generation,
                            "profile_id": profile.id,
                            "query_id": query.id,
                            "reply_id": reply.id,
                            "request": request,
                            "reply_value": reply.value,
                            "reply_mask": reply.mask,
                        });
                        return stream.write_all(format!("{response}\n").as_bytes());
                    }
                    let controls = profile
                        .controls
                        .iter()
                        .take(256)
                        .map(|control| {
                            serde_json::json!({
                                "label": control.label,
                                "cc": control.cc,
                                "program": control.program,
                                "range": [control.range.0, control.range.1],
                                "operation": control.operation,
                            })
                        })
                        .collect::<Vec<_>>();
                    let response = serde_json::json!({
                        "ok": true,
                        "generation": self.generation,
                        "profile": {
                            "id": profile.id,
                            "name": profile.name,
                            "capabilities": profile.capabilities.iter().map(|capability| &capability.id).collect::<Vec<_>>(),
                            "controls": controls,
                            "query_count": profile.queries.len(),
                            "queries": profile
                                .queries
                                .iter()
                                .take(64)
                                .map(|query| serde_json::json!({
                                    "id": query.id,
                                    "reply_id": query.reply_id,
                                    "request_bytes": query.request.len(),
                                }))
                                .collect::<Vec<_>>(),
                            "documented_features": profile.documented_features.iter().take(64).collect::<Vec<_>>(),
                        },
                    });
                    return stream.write_all(format!("{response}\n").as_bytes());
                }
            }
            if command == Some(Command::DeviceControl) {
                let value =
                    serde_json::from_slice::<serde_json::Value>(&request).unwrap_or_default();
                if value.get("confirm").and_then(serde_json::Value::as_bool) != Some(true) {
                    return stream.write_all(
                        b"{\"ok\":false,\"error\":\"device control requires confirmation\"}\n",
                    );
                }
                let profile_id = value
                    .get("profile_id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| io::Error::other("profile_id is required"));
                let control = value
                    .get("control")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| io::Error::other("control is required"));
                let channel = value
                    .get("channel")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|number| u8::try_from(number).ok())
                    .ok_or_else(|| io::Error::other("channel is required"));
                let control_value = value
                    .get("value")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|number| u16::try_from(number).ok())
                    .ok_or_else(|| io::Error::other("value is required"));
                let destination = value
                    .get("destination")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| io::Error::other("destination is required"));
                let (Ok(profile_id), Ok(control), Ok(channel), Ok(control_value), Ok(destination)) =
                    (profile_id, control, channel, control_value, destination)
                else {
                    return stream.write_all(
                        b"{\"ok\":false,\"error\":\"device control fields are required\"}\n",
                    );
                };
                let Some(profile) = mackes_profiles::builtin_profile(profile_id) else {
                    return stream
                        .write_all(b"{\"ok\":false,\"error\":\"unknown device profile\"}\n");
                };
                if profile_id == "m-vave.ir-box" {
                    let payload = control
                        .strip_prefix("Preset ")
                        .and_then(|value| value.parse::<u8>().ok())
                        .map_or_else(
                            || {
                                (control == "IR" || control == "EQ").then(|| {
                                    mackes_profiles::mvave_ir_box_module_sysex(
                                        if control == "IR" {
                                            mackes_profiles::MvaveIrBoxModule::Ir
                                        } else {
                                            mackes_profiles::MvaveIrBoxModule::Eq
                                        },
                                        control_value != 0,
                                    )
                                })
                            },
                            |preset| mackes_profiles::mvave_ir_box_preset_sysex(preset).ok(),
                        );
                    let Some(payload) = payload else {
                        return stream
                            .write_all(b"{\"ok\":false,\"error\":\"invalid M-VAVE control\"}\n");
                    };
                    if let Err(error) = self.outputs.send_direct(destination, &payload) {
                        return stream.write_all(
                            format!("{{\"ok\":false,\"error\":\"{error}\"}}\n").as_bytes(),
                        );
                    }
                    return stream.write_all(
                        format!(
                            "{{\"ok\":true,\"generation\":{},\"bytes\":{}}}\n",
                            self.generation,
                            serde_json::to_string(&payload).unwrap_or_else(|_| "[]".into())
                        )
                        .as_bytes(),
                    );
                }
                let Ok(payload) = profile.render_control_message(control, channel, control_value)
                else {
                    return stream
                        .write_all(b"{\"ok\":false,\"error\":\"invalid device control\"}\n");
                };
                if let Err(error) = self.outputs.send_direct(destination, &payload) {
                    return stream
                        .write_all(format!("{{\"ok\":false,\"error\":\"{error}\"}}\n").as_bytes());
                }
                return stream.write_all(
                    format!(
                        "{{\"ok\":true,\"generation\":{},\"bytes\":{}}}\n",
                        self.generation,
                        serde_json::to_string(&payload).unwrap_or_else(|_| "[]".into())
                    )
                    .as_bytes(),
                );
            }
            if command == Some(Command::Sysex) {
                let value =
                    serde_json::from_slice::<serde_json::Value>(&request).unwrap_or_default();
                if value.get("confirm").and_then(serde_json::Value::as_bool) != Some(true) {
                    return stream
                        .write_all(b"{\"ok\":false,\"error\":\"SysEx requires confirmation\"}\n");
                }
                let destination = value.get("destination").and_then(serde_json::Value::as_str);
                let bytes = value.get("bytes").and_then(serde_json::Value::as_array);
                let (Some(destination), Some(bytes)) = (destination, bytes) else {
                    return stream.write_all(
                        b"{\"ok\":false,\"error\":\"SysEx destination and bytes are required\"}\n",
                    );
                };
                if bytes.is_empty() || bytes.len() > 1024 {
                    return stream.write_all(
                        b"{\"ok\":false,\"error\":\"SysEx payload must contain 1..=1024 bytes\"}\n",
                    );
                }
                let payload = bytes
                    .iter()
                    .map(|byte| byte.as_u64().and_then(|byte| u8::try_from(byte).ok()))
                    .collect::<Option<Vec<_>>>();
                let Some(payload) = payload else {
                    return stream
                        .write_all(b"{\"ok\":false,\"error\":\"SysEx bytes are invalid\"}\n");
                };
                if payload.first() != Some(&0xF0) || payload.last() != Some(&0xF7) {
                    return stream.write_all(
                        b"{\"ok\":false,\"error\":\"SysEx payload must be framed F0..F7\"}\n",
                    );
                }
                if let Err(error) = self.outputs.send_direct(destination, &payload) {
                    return stream
                        .write_all(format!("{{\"ok\":false,\"error\":\"{error}\"}}\n").as_bytes());
                }
                return stream.write_all(
                    format!(
                        "{{\"ok\":true,\"generation\":{},\"bytes_sent\":{}}}\n",
                        self.generation,
                        payload.len()
                    )
                    .as_bytes(),
                );
            }
            if let Some(command) = command {
                if !matches!(
                    command,
                    Command::Hello | Command::Health | Command::Snapshot | Command::Subscribe
                ) {
                    if command == Command::Panic {
                        let _ = self.send_panic_controls();
                    }
                    self.record_state_event(command);
                }
            }
            match command {
                Some(Command::Snapshot) => self.snapshot_response(),
                Some(Command::Subscribe) => self.subscribe_response(&request),
                Some(Command::Scenes) => self.scenes_response(),
                Some(command) => {
                    let endpoints = if command == Command::Endpoints {
                        self.discover_endpoints().unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    let routes =
                        if command == Command::Routes { self.router.routes() } else { Vec::new() };
                    command_ack(
                        command,
                        self.health,
                        self.generation,
                        &endpoints,
                        self.route_generation(),
                        &routes,
                    )
                }
                None => "{\"ok\":false,\"error\":\"unknown command\"}\n".to_owned(),
            }
        } else if command.is_none() {
            "{\"ok\":false,\"error\":\"unknown command\"}\n".to_owned()
        } else {
            "{\"ok\":false,\"error\":\"unauthorized\"}\n".to_owned()
        };
        let shutdown = command == Some(Command::Shutdown) && response.contains("\"ok\":true");
        stream.write_all(response.as_bytes())?;
        if shutdown {
            self.health = Health::Stopping;
        }
        Ok(())
    }

    /// Requests graceful shutdown without interrupting an in-flight operation.
    ///
    /// Service and signal adapters call this boundary; the main loop observes
    /// [`Health::Stopping`] and stops accepting new requests.
    pub const fn request_shutdown(&mut self) {
        self.health = Health::Stopping;
    }

    /// Marks the daemon degraded while keeping routing and diagnostics available.
    pub const fn mark_degraded(&mut self) {
        if !matches!(self.health, Health::Stopping) {
            self.health = Health::Degraded;
        }
    }

    /// Returns current health.
    #[must_use]
    pub const fn health(&self) -> Health {
        self.health
    }

    /// Installs the validated active-project scene catalog for read-only IPC queries.
    pub fn set_scene_ids(&mut self, scene_ids: Vec<String>) {
        self.scene_ids = scene_ids;
    }

    /// Installs the validated project/setlist catalog for read-only UI queries.
    pub fn set_catalog(&mut self, catalog: serde_json::Value) {
        self.catalog = catalog;
    }

    /// Installs the daemon-owned physical-device inventory for snapshots.
    pub fn set_physical_devices(&mut self, endpoints: &[mackes_midi_engine::EndpointInfo]) {
        self.physical_devices = physical_devices_value(endpoints);
    }

    /// Sets the daemon-owned configuration path for authorized persistence.
    pub fn set_config_path(&mut self, path: impl Into<std::path::PathBuf>) {
        let path = path.into();
        if let Ok(bytes) = std::fs::read(routes_path(&path)) {
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                if let Some(routes) = value.get("routes") {
                    if let Ok(encoded) = serde_json::to_vec(routes) {
                        let _ = self.replace_routes_json(&encoded, 1, 8);
                    }
                }
            }
        }
        self.config_path = Some(path);
    }

    fn scenes_response(&self) -> String {
        serde_json::json!({
            "ok": true,
            "generation": self.generation,
            "scenes": self.scene_ids,
            "active_scene": self.active_scene,
            "catalog": self.catalog,
            "physical_devices": self.physical_devices,
        })
        .to_string()
            + "\n"
    }
}

/// Loads persisted state and computes automatic startup restore behavior.
///
/// # Errors
///
/// Returns a configuration error when persisted state cannot be loaded or validated.
pub fn startup_restore(path: &Path) -> Result<RestoreResult, ConfigError> {
    let document: ConfigDocument = load(path)?;
    let active_project = document.settings.active_project.clone();
    let active_scene = document.settings.active_scene.clone();
    let scenes = match active_project.as_deref() {
        None => {
            if active_scene.is_some() {
                return Err(ConfigError::Semantic {
                    path: path.to_owned(),
                    message: "active scene requires an active project".into(),
                });
            }
            Vec::new()
        }
        Some(project_id) => match document.projects.iter().find(|project| project.id == project_id)
        {
            Some(project) => {
                if let Some(scene) = active_scene.as_deref() {
                    if !project.scenes.iter().any(|entry| entry.id == scene) {
                        return Err(ConfigError::Semantic {
                            path: path.to_owned(),
                            message: format!(
                                "active scene '{scene}' is not present in the active project"
                            ),
                        });
                    }
                }
                project.scenes.iter().map(|scene| scene.id.clone()).collect()
            }
            None => {
                return Err(ConfigError::Semantic {
                    path: path.to_owned(),
                    message: format!(
                        "active project '{project_id}' is not present in the configuration"
                    ),
                })
            }
        },
    };
    let should_activate = active_project.is_some() && !scenes.is_empty();
    Ok(RestoreResult {
        active_project,
        active_scene,
        scenes,
        should_activate,
        // Startup may recover the selected project, but every hardware-mutating
        // action remains held until the operator explicitly arms unsafe mode.
        unsafe_actions_blocked: usize::from(should_activate),
    })
}

/// Compiles one persisted scene's actions into the ordinary activation planner.
///
/// # Errors
///
/// Returns the planner validation error when the action graph is invalid.
#[cfg(target_os = "linux")]
pub fn compile_scene_actions(
    scene: &mackes_config::SceneRef,
) -> Result<mackes_scene_engine::ActivationPlan, &'static str> {
    mackes_scene_engine::ActivationPlan::compile(
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
}

/// Persists an accepted active-scene selection through validated atomic config saving.
///
/// # Errors
///
/// Returns the load, validation, or atomic-save error without modifying the in-memory daemon.
#[cfg(target_os = "linux")]
pub fn persist_active_scene(path: &Path, scene: Option<&str>) -> Result<(), String> {
    let document = mackes_config::load(path).map_err(|error| error.to_string())?;
    let updated = mackes_config::set_active_scene(&document, scene)?;
    mackes_config::save(path, &updated, 10).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
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
            health_after_authorized_command(Health::Starting, Some(Command::Health)),
            Health::Starting
        );
        assert_eq!(
            health_after_authorized_command(Health::Starting, Some(Command::Snapshot)),
            Health::Ready
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
            settle_required_endpoints(
                EndpointSettlePolicy::default(),
                0,
                1,
                &["missing"],
                &endpoints
            ),
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
        let readiness = startup_restore_readiness(
            fixture,
            EndpointSettlePolicy::default(),
            0,
            1,
            &[],
            &endpoints,
        )
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
        let path =
            std::env::temp_dir().join(format!("mackes-shutdown-{}.sock", std::process::id()));
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
        let path =
            std::env::temp_dir().join(format!("mackes-nonblocking-{}.sock", std::process::id()));
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
        let path =
            std::env::temp_dir().join(format!("mackes-degraded-{}.sock", std::process::id()));
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
        let replay: serde_json::Value =
            serde_json::from_str(&daemon.subscribe_response(br#"{"after_sequence":0}"#))
                .expect("replay");
        assert_eq!(replay["events"].as_array().expect("events").len(), 1);
        let enveloped_replay: serde_json::Value = serde_json::from_str(
            &daemon.subscribe_response(br#"{"payload":{"after_sequence":1}}"#),
        )
        .expect("enveloped replay");
        assert!(enveloped_replay["events"].as_array().expect("events").is_empty());
        for _ in 0..256 {
            daemon.record_state_event(Command::Monitor);
        }
        let gap: serde_json::Value =
            serde_json::from_str(&daemon.subscribe_response(br#"{"after_sequence":0}"#))
                .expect("gap");
        assert_eq!(gap["snapshot_required"], true);
        assert_eq!(daemon.state_events.len(), 256);
        let _ = fs::remove_file(path);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn registered_dispatch_updates_activity_and_publishes_live_event() {
        let path =
            std::env::temp_dir().join(format!("mackes-activity-{}.sock", std::process::id()));
        let mut daemon = Daemon::bind(&path).expect("daemon");
        let event = mackes_domain::MidiEvent {
            timestamp: mackes_domain::TimestampNanos::new(1),
            sequence: 1,
            endpoint: mackes_domain::EndpointId::new(1).expect("endpoint"),
            message: mackes_domain::MidiMessage::ControlChange {
                channel: mackes_domain::MidiChannel::new(1).expect("channel"),
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
        let mut burst_event = event.clone();
        burst_event.sequence = 2;
        assert_eq!(daemon.dispatch_registered(&burst_event), (0, 0));
        assert_eq!(daemon.state_events.len(), 1, "activity journal must be rate-limited");
        let _ = fs::remove_file(path);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn active_scene_is_published_to_snapshot_and_journal() {
        let path =
            std::env::temp_dir().join(format!("mackes-scene-state-{}.sock", std::process::id()));
        let mut daemon = Daemon::bind(&path).expect("daemon");
        daemon.set_active_scene(Some("intro".to_owned()));
        let snapshot = serde_json::from_str::<serde_json::Value>(&daemon.snapshot_response())
            .expect("snapshot");
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
        let path = std::env::temp_dir()
            .join(format!("mackes-dashboard-input-{}.sock", std::process::id()));
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
        let plan = mackes_scene_engine::ActivationPlan::compile(vec![
            mackes_scene_engine::ActivationAction {
                id: "write".into(),
                description: "write".into(),
                unsafe_action: false,
                depends_on: None,
                destination: None,
                message: None,
            },
        ])
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
        let result = result
            .or_else(|_| startup_restore(std::path::Path::new("fixtures/config-valid.json5")));
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
        let path = std::env::temp_dir()
            .join(format!("mackes-startup-{}-missing.json5", std::process::id()));
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
}
