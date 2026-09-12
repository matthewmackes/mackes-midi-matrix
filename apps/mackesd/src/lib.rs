//! Persistent daemon lifecycle and local command boundary.
#![recursion_limit = "256"]
use mackes_config::ConfigError;
use mackes_ipc::{authorize, AccessPolicy, Authorization, Command, LocalServer};
use std::{
    collections::VecDeque,
    fmt::Write as FmtWrite,
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};
#[cfg(target_os = "linux")]
mod activity_projection;
mod configuration_response;
mod mapping_layers_runtime;
mod mapping_runtime;
mod pipedal_dispatch;
mod scene_runtime;
#[cfg(target_os = "linux")]
use activity_projection::midi_activity_json;
#[cfg(target_os = "linux")]
mod route_projection;
#[cfg(target_os = "linux")]
use route_projection::{persist_routes, routes_path, routes_undo_path};
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
    pipedal_worker: mackes_pipedal_adapter::Worker,
    pipedal_transport: Option<mackes_pipedal_adapter::WebSocketTransport>,
    pipedal_retry_at: Instant,
    generation: u64,
    active_scene: Option<String>,
    scene_ids: Vec<String>,
    catalog: serde_json::Value,
    physical_devices: serde_json::Value,
    profile_bindings: Vec<(String, String)>,
    binding_generation: u64,
    config_path: Option<std::path::PathBuf>,
    operation_journal: Option<mackes_config::OperationJournal>,
    novation_device_policy: Option<mackes_config::NovationDeviceConfig>,
    novation_rebind_previous: Option<mackes_config::NovationDeviceConfig>,
    /// Cached `PiPedal` physical control IDs used by the real-time LED composer.
    /// This must be refreshed at configuration boundaries, never from the LED tick.
    pipedal_controls: Vec<String>,
    /// Cached validated `PiPedal` identities used by snapshots and dispatch.
    pipedal_mappings: Vec<mackes_pipedal_adapter::MappingIdentity>,
    mapping_store: mackes_config::ControlMappingStore,
    /// Validated daemon-owned v2 layer projection loaded with configuration.
    mapping_layers_v2: Option<mackes_config::MappingLayersV2>,
    button_toggle_state: std::collections::HashMap<String, (bool, bool)>,
    lexicon_active_algorithm: Option<u8>,
    lexicon_readback_error: Option<String>,
    assignment_generation: u64,
    assignment_session: mackes_ipc::AssignmentSession,
    led: led_surface::LedSurface,
    assignment_previous_store: Option<mackes_config::ControlMappingStore>,
    assignment_pending_mapping: Option<mackes_config::ControlMapping>,
    assignment_control_id: Option<String>,
    assignment_capture_at: Option<Instant>,
    assignment_device_down_at: Option<Instant>,
    router: mackes_midi_engine::RouterStore,
    rtp_peer: mackes_midi_engine::RtpMidiPeer,
    outputs: mackes_midi_engine::OutputRegistry,
    inputs: mackes_midi_engine::InputRegistry,
    deferred_inputs: VecDeque<mackes_domain::MidiEvent>,
    alsa_supervisor: mackes_midi_engine::NativeAlsaSupervisor,
    last_native_failure: Option<&'static str>,
    native_led_resync: bool,
    native_rescan_at: Option<Instant>,
    #[cfg(feature = "alsa-seq-backend")]
    xl_midi_output_address: Option<mackes_midi_engine::AlsaSequencerAddress>,
    #[cfg(feature = "alsa-seq-backend")]
    alsa_input_client:
        Option<std::sync::Arc<std::sync::Mutex<mackes_midi_engine::AlsaSequencerClient>>>,
    virtual_ports: Option<mackes_midi_engine::VirtualMidiPorts>,
    virtual_ingress: Receiver<(u64, Vec<u8>)>,
    virtual_ingress_tx: Sender<(u64, Vec<u8>)>,
    virtual_sequence: u64,
    received_events: u64,
    sent_events: u64,
    dropped_events: u64,
    activity: mackes_midi_engine::ActivityCoalescer,
    last_activity: Option<serde_json::Value>,
    last_mapping_activity: Option<serde_json::Value>,
    last_activity_publish: Instant,
    route_undo: Option<serde_json::Value>,
    safety: mackes_scene_engine::SafetyController,
    audit: mackes_scene_engine::AuditLog,
    activation_result: Option<String>,
    activation_outcomes: Option<Vec<serde_json::Value>>,
    state_sequence: u64,
    state_events: VecDeque<mackes_ipc::StateEvent>,
    safety_clock: Instant,
}
/// Classifies a bounded JSON command tag without deserializing untrusted payloads.
#[cfg(target_os = "linux")]
#[must_use]
pub fn classify_command(request: &[u8]) -> Option<Command> {
    const COMMANDS: &[(Command, &[u8])] = &[
        (Command::Hello, b"hello"),
        (Command::Snapshot, b"snapshot"),
        (Command::NovationSnapshot, b"novation_snapshot"),
        (Command::Subscribe, b"subscribe"),
        (Command::Validate, b"validate"),
        (Command::Configuration, b"configuration"),
        (Command::Migrate, b"migrate"),
        (Command::Rescan, b"rescan"),
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
        (Command::Mappings, b"mappings"),
        (Command::PiPedal, b"pipedal"),
        (Command::Assignment, b"assignment"),
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
        .take(MAX_PHYSICAL_DEVICE_RECORDS)
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
const MAX_PHYSICAL_DEVICE_RECORDS: usize = 32;
/// Maximum interval between bounded native endpoint discovery passes.
pub(crate) const NATIVE_RESCAN_INTERVAL_MS: u64 = 250;
/// Produces a stable acknowledgment for a recognized command.
#[cfg(target_os = "linux")]
#[allow(clippy::too_many_lines)]
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
        Command::NovationSnapshot => {
            format!("{{\"ok\":true,\"generation\":{generation},\"novation_snapshot\":true}}\n")
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
            let endpoint_encoded = persistence_projection::endpoint_catalog(endpoints).to_string();
            format!("{{\"ok\":true,\"generation\":{generation},\"routes\":{encoded},\"endpoint_catalog\":{endpoint_encoded},\"route_generation\":{}}}\n", route_generation.unwrap_or(0))
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
        Command::Migrate => {
            format!("{{\"ok\":true,\"generation\":{generation},\"migrate\":true}}\n")
        }
        Command::Rescan => {
            format!("{{\"ok\":true,\"generation\":{generation},\"rescan\":\"scheduled\"}}\n")
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
        Command::Mappings => {
            format!("{{\"ok\":true,\"generation\":{generation},\"mappings\":[],\"drafts\":[],\"undo_available\":false}}\n")
        }
        Command::PiPedal => {
            format!("{{\"ok\":true,\"generation\":{generation},\"pipedal\":true}}\n")
        }
        Command::Assignment => {
            format!("{{\"ok\":true,\"generation\":{generation},\"phase\":\"Idle\"}}\n")
        }
        other @ Command::Shutdown => format!(
            "{{\"ok\":true,\"generation\":{generation},\"accepted\":true,\"command\":\"{}\"}}\n",
            other.tag()
        ),
    }
}
#[cfg(target_os = "linux")]
impl Daemon {
    fn cache_pipedal_mappings(&mut self, document: &mackes_config::ConfigDocument) {
        self.mapping_layers_v2.clone_from(&document.mapping_layers_v2);
        self.pipedal_controls = document
            .settings
            .pipedal_mappings
            .mappings
            .iter()
            .map(|mapping| mapping.physical_control_id.clone())
            .collect();
        self.pipedal_mappings = document
            .settings
            .pipedal_mappings
            .mappings
            .iter()
            .map(|mapping| mackes_pipedal_adapter::MappingIdentity {
                physical_control_id: mapping.physical_control_id.clone(),
                plugin_uri: mapping.plugin_uri.clone(),
                symbol: mapping.symbol.clone(),
                scope: mapping.scope.clone(),
            })
            .collect();
    }
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
            pipedal_worker: mackes_pipedal_adapter::Worker::default(),
            pipedal_transport: None,
            pipedal_retry_at: Instant::now(),
            generation: 0,
            active_scene: None,
            scene_ids: Vec::new(),
            catalog: serde_json::json!({"projects": [], "setlists": []}),
            physical_devices: serde_json::json!([]),
            profile_bindings: Vec::new(),
            binding_generation: 0,
            config_path: None,
            operation_journal: None,
            novation_device_policy: None,
            novation_rebind_previous: None,
            pipedal_controls: Vec::new(),
            pipedal_mappings: Vec::new(),
            mapping_store: mackes_config::ControlMappingStore::default(),
            mapping_layers_v2: None,
            button_toggle_state: std::collections::HashMap::new(),
            lexicon_active_algorithm: None,
            lexicon_readback_error: None,
            assignment_generation: 0,
            assignment_session: mackes_ipc::AssignmentSession::new("live"),
            led: led_surface::LedSurface::default(),
            assignment_previous_store: None,
            assignment_pending_mapping: None,
            assignment_control_id: None,
            assignment_capture_at: None,
            assignment_device_down_at: None,
            router: mackes_midi_engine::RouterStore::new(Vec::new(), 0, 8)
                .map_err(io::Error::other)?,
            rtp_peer: mackes_midi_engine::RtpMidiPeer::new(0, 32).map_err(io::Error::other)?,
            outputs: mackes_midi_engine::OutputRegistry::new(64),
            inputs: mackes_midi_engine::InputRegistry::new(64),
            deferred_inputs: VecDeque::with_capacity(256),
            alsa_supervisor: mackes_midi_engine::NativeAlsaSupervisor::new(),
            last_native_failure: None,
            native_led_resync: false,
            native_rescan_at: None,
            #[cfg(feature = "alsa-seq-backend")]
            xl_midi_output_address: None,
            #[cfg(feature = "alsa-seq-backend")]
            alsa_input_client: None,
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
            last_mapping_activity: None,
            last_activity_publish: Instant::now(),
            route_undo: None,
            safety: mackes_scene_engine::SafetyController::default(),
            audit: mackes_scene_engine::AuditLog::new(128).map_err(io::Error::other)?,
            activation_result: None,
            activation_outcomes: None,
            state_sequence: 0,
            state_events: VecDeque::with_capacity(256),
            safety_clock: Instant::now(),
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
    #[allow(clippy::missing_errors_doc)]
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
    /// This path exists only for the feature-gated `midir` rollback adapter. Production
    /// Linux input uses native ALSA polling through [`Self::poll_and_dispatch_inputs`].
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
    /// Removes a previously registered output by stable identity.
    pub fn unregister_output(&mut self, id: &str) -> bool {
        self.outputs.remove(id)
    }
    /// Rebuilds controller LEDs from persisted mappings onto the unique Mk2 MIDI output.
    pub fn replay_controller_leds(&mut self) {
        let now_ms = u64::try_from(self.safety_clock.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.led.start_reconnect_show(now_ms);
        self.led.request_full_resync();
        self.flush_controller_leds();
    }
    /// Advances the LED scheduler on the daemon monotonic clock and emits due frames.
    pub fn flush_controller_leds(&mut self) {
        let now_ms = u64::try_from(self.safety_clock.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.flush_controller_leds_at(now_ms);
    }
    fn drop_launch_control_midi_outputs(&mut self) {
        let Some(target) = self
            .profile_bindings
            .iter()
            .find(|(profile, _)| profile == "launch-control-xl-mk2")
            .map(|(_, endpoint)| endpoint.clone())
        else {
            return;
        };
        let ids: Vec<String> = self
            .outputs
            .output_infos()
            .into_iter()
            .filter(|info| info.id == target)
            .map(|info| info.id)
            .collect();
        for id in ids {
            self.outputs.remove(&id);
        }
    }
    fn reopen_launch_control_midi_output(&mut self) -> Result<(), String> {
        self.drop_launch_control_midi_outputs();
        let target = self
            .profile_bindings
            .iter()
            .find(|(profile, _)| profile == "launch-control-xl-mk2")
            .map(|(_, endpoint)| endpoint.clone())
            .ok_or_else(|| "Launch Control output binding is unresolved".to_owned())?;
        let ports = mackes_midi_engine::enumerate_midir_ports()?;
        let selected = ports
            .into_iter()
            .find(|port| {
                port.direction == mackes_midi_engine::EndpointDirection::Output && port.id == target
            })
            .ok_or_else(|| "resolved Launch Control output is missing".to_owned())?;
        self.provision_output(&selected.id)
    }
    #[cfg(feature = "alsa-seq-backend")]
    fn restore_launch_control_midi_output(
        &mut self,
        ports: &[mackes_midi_engine::AlsaSequencerPort],
    ) {
        let Some(binding) = self
            .profile_bindings
            .iter()
            .find(|(profile, _)| profile == "launch-control-xl-mk2")
            .map(|(_, endpoint)| endpoint.as_str())
        else {
            self.xl_midi_output_address = None;
            return;
        };
        let current = led_surface::mk2_midi_writable_address_for_binding(ports, binding);
        if current == self.xl_midi_output_address {
            return;
        }
        if current.is_none() {
            self.xl_midi_output_address = None;
            self.drop_launch_control_midi_outputs();
            return;
        }
        if self.reopen_launch_control_midi_output().is_ok() {
            self.xl_midi_output_address = current;
            self.native_led_resync = true;
            self.replay_controller_leds();
            self.request_lexicon_active_setup();
        }
    }
    /// Reopens physical outputs that appeared after daemon startup or a USB reconnect.
    #[cfg(feature = "alsa-seq-backend")]
    fn restore_late_physical_outputs(&mut self) {
        let Ok(endpoints) = mackes_midi_engine::enumerate_midir_ports() else { return };
        let physical_endpoints = endpoints
            .iter()
            .filter(|endpoint| {
                !endpoint.name.starts_with("MACKES ") && !endpoint.name.starts_with("Midi Through")
            })
            .cloned()
            .collect::<Vec<_>>();
        self.set_physical_devices(&physical_endpoints);
        let known = self.outputs.output_ids();
        for endpoint in endpoints.iter().filter(|endpoint| {
            endpoint.direction == mackes_midi_engine::EndpointDirection::Output
                && !endpoint.name.starts_with("MACKES ")
                && !endpoint.name.starts_with("Midi Through")
                && !known.iter().any(|id| id == &endpoint.id)
        }) {
            let _ = self.provision_output(&endpoint.id);
        }
    }
    /// Requests the Lexicon's authoritative active setup after its output is available.
    fn request_lexicon_active_setup(&mut self) {
        let Some(destination) = profile_bindings::stable_destination(
            &self.outputs,
            &self.profile_bindings,
            "lexicon.reflex",
        ) else {
            return;
        };
        let _ = self.outputs.send_direct(&destination, &[0xF0, 0x06, 0x02, 0x30, 0x60, 0x00, 0xF7]);
    }
    fn confirm_lexicon_algorithm(&mut self, algorithm: u8) {
        self.lexicon_active_algorithm = Some(algorithm);
        self.lexicon_readback_error = None;
        self.led.set_active_lexicon_algorithm(algorithm);
    }
    fn reject_lexicon_readback(&mut self, error: impl Into<String>) {
        let error = error.into();
        self.lexicon_readback_error = Some(error.clone());
        self.led.set_lexicon_readback_error(error);
    }
    /// Advances LED overlays at an explicit fake-clock instant.
    pub fn flush_controller_leds_at(&mut self, now_ms: u64) {
        self.flush_controller_leds_at_generation(now_ms, self.binding_generation);
    }
    fn flush_controller_leds_at_generation(&mut self, now_ms: u64, generation: u64) {
        if generation != self.binding_generation {
            self.led.set_lexicon_readback_error(
                "LED write rejected: stale endpoint binding generation",
            );
            return;
        }
        if self.novation_device_policy.as_ref().is_some_and(|policy| !policy.feedback_enabled) {
            return;
        }
        let target = self
            .profile_bindings
            .iter()
            .find(|(profile, _)| profile == "launch-control-xl-mk2")
            .map(|(_, endpoint)| endpoint.clone());
        self.led.set_target_binding(target);
        if let Some(policy) = self.novation_device_policy.as_ref() {
            self.led.set_template(policy.template);
        }
        self.led.set_pipedal_controls(self.pipedal_controls.clone());
        let mut effective_store = self.mapping_store.clone();
        effective_store.active = mapping_layers_runtime::effective_mappings(
            &self.mapping_store,
            self.mapping_layers_v2.as_ref(),
        );
        self.led.flush(
            now_ms,
            &effective_store,
            &self.assignment_session,
            self.assignment_control_id.as_deref(),
            &mut self.outputs,
            self.safety.performance_locked(),
        );
    }
    /// Sends one already-validated event to a registered endpoint.
    pub fn send_event_to_endpoint(
        &mut self,
        endpoint: mackes_domain::EndpointId,
        event: mackes_domain::MidiEvent,
    ) {
        let _ = self.outputs.send_to_endpoint(endpoint, event);
    }
    /// Arms experimental parameter mappings for the bounded fifteen-minute window.
    pub fn arm_experimental_mappings(&mut self) {
        let now = u64::try_from(self.safety_clock.elapsed().as_nanos().min(u128::from(u64::MAX)))
            .unwrap_or(u64::MAX);
        self.safety.arm_unsafe(now.saturating_add(15 * 60 * 1_000_000_000));
    }
    /// Applies one generation-checked assignment action in the daemon-owned session.
    ///
    /// # Panics
    ///
    /// Internal invariant checks panic only if a previously validated request loses
    /// required fields during this method.
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn apply_assignment_request(
        &mut self,
        request: mackes_ipc::AssignmentRequest,
    ) -> mackes_ipc::AssignmentResult {
        let mut applied = false;
        let mut reason = None;
        let mut request = request;
        if request.action == mackes_ipc::AssignmentAction::Snapshot {
            return mackes_ipc::AssignmentResult {
                generation: self.assignment_generation,
                session: self.assignment_session.clone(),
                applied: true,
                reason: None,
            };
        }
        if request.action == mackes_ipc::AssignmentAction::ControlCaptured {
            if let Some(control) = request.physical_control_id.clone() {
                self.assignment_session.catalog.captured_control_id = Some(control);
            }
        }
        if request.action == mackes_ipc::AssignmentAction::Enter {
            assignment_catalog::lock_active_selection(&mut self.assignment_session);
            // Eventide exposes direct effect parameters; its profile has no
            // preset/type selection in the controller assignment workflow.
            // Keep this transition daemon-owned so keyboard and controller
            // navigation produce the same authoritative trace.
            if self.assignment_session.catalog.selected_device.as_deref()
                == Some("eventide.micropitch")
            {
                match self.assignment_session.phase {
                    mackes_ipc::AssignmentPhase::ChooseDevice => {
                        self.assignment_session.phase = mackes_ipc::AssignmentPhase::ChoosePreset;
                    }
                    mackes_ipc::AssignmentPhase::ChooseEffect => {
                        self.assignment_session.phase = mackes_ipc::AssignmentPhase::ChooseType;
                    }
                    _ => {}
                }
            }
            if assignment_catalog::continuous_preset_forbidden(&self.assignment_session) {
                reason = Some("preset destinations require a channel button".into());
            } else if assignment_catalog::button_preset_ready(&self.assignment_session)
                || self.assignment_session.phase == mackes_ipc::AssignmentPhase::ChooseParameter
            {
                if let Some((control, profile, effect, parameter)) =
                    assignment_catalog::commit_from_catalog(&self.assignment_session)
                {
                    request.action = mackes_ipc::AssignmentAction::Commit;
                    request.physical_control_id = Some(control);
                    request.destination_profile = Some(profile);
                    request.destination_effect = Some(effect);
                    request.destination_parameter = Some(parameter);
                }
            }
        }
        if request.action == mackes_ipc::AssignmentAction::Commit
            && !request.has_complete_destination()
        {
            if let Some((control, profile, effect, parameter)) =
                assignment_catalog::commit_from_catalog(&self.assignment_session)
            {
                request.physical_control_id = Some(control);
                request.destination_profile = Some(profile);
                request.destination_effect = Some(effect);
                request.destination_parameter = Some(parameter);
            }
        }
        if reason.is_some() {
        } else if request.generation != self.assignment_generation {
            reason = Some("assignment generation conflict".into());
        } else if let Err(error) = request.clone().validate() {
            reason = Some(error.into());
        } else if request.physical_control_id.as_deref().is_some_and(|control| {
            mackes_profiles::PhysicalControlId::new(control).is_err()
                || mackes_profiles::launch_control_physical_catalog()
                    .iter()
                    .find(|item| item.id.as_str() == control)
                    .is_none_or(|item| item.role == mackes_profiles::PhysicalControlRole::Utility)
        }) {
            reason = Some("assignment control is reserved or unknown".into());
        } else if request.action == mackes_ipc::AssignmentAction::Commit
            && !request.has_complete_destination()
        {
            reason = Some("assignment commit requires a complete destination".into());
        } else {
            self.sync_assignment_catalog();
            if request.action == mackes_ipc::AssignmentAction::ConfirmReplace {
                let previous_store = self.mapping_store.clone();
                if let Some(mapping) = self.assignment_pending_mapping.take() {
                    if !assignment_commit::mapping_role_compatible(&mapping) {
                        reason = Some("preset destinations require a channel button".into());
                    } else if let Some(existing) =
                        self.mapping_store.active.iter().find(|existing| {
                            existing.physical_control_id == mapping.physical_control_id
                                || (existing.destination_profile == mapping.destination_profile
                                    && existing.destination_effect == mapping.destination_effect
                                    && existing.destination_parameter
                                        == mapping.destination_parameter)
                        })
                    {
                        let mut replacement = mapping;
                        replacement.id.clone_from(&existing.id);
                        if let Err(error) =
                            self.mapping_store.replace(self.mapping_store.generation, replacement)
                        {
                            reason = Some(format!("assignment replacement failed: {error}"));
                        } else if let Some(path) = self.config_path.as_deref() {
                            if mackes_config::save_control_mapping_store(
                                path,
                                &self.mapping_store,
                                1,
                            )
                            .is_err()
                            {
                                self.mapping_store = previous_store;
                                reason = Some("assignment persistence failed".into());
                            } else {
                                self.assignment_previous_store = Some(previous_store);
                            }
                        } else {
                            self.assignment_previous_store = Some(previous_store);
                        }
                    } else {
                        reason = Some("assignment replacement target disappeared".into());
                    }
                } else {
                    reason = Some("assignment replacement confirmation expired".into());
                }
            }
            if request.action == mackes_ipc::AssignmentAction::Commit {
                let previous_store = self.mapping_store.clone();
                match assignment_commit::mapping_from_request(
                    &request,
                    &self.assignment_session.catalog,
                ) {
                    Err(error) => reason = Some(error),
                    Ok(mapping) => {
                        let is_replace =
                            request.action == mackes_ipc::AssignmentAction::ConfirmReplace;
                        let mapping_for_rollback = mapping.clone();
                        let mut mapping = mapping;
                        if is_replace {
                            if let Some(existing) =
                                self.mapping_store.active.iter().find(|existing| {
                                    existing.physical_control_id == mapping.physical_control_id
                                        || (existing.destination_profile
                                            == mapping.destination_profile
                                            && existing.destination_effect
                                                == mapping.destination_effect
                                            && existing.destination_parameter
                                                == mapping.destination_parameter)
                                })
                            {
                                mapping.id.clone_from(&existing.id);
                            }
                        }
                        let mutation = if is_replace {
                            self.mapping_store.replace(self.mapping_store.generation, mapping)
                        } else {
                            self.mapping_store.activate(self.mapping_store.generation, mapping)
                        };
                        if let Err(error) = mutation {
                            if !is_replace && error.contains("occupied") {
                                self.assignment_pending_mapping = Some(mapping_for_rollback);
                                self.assignment_session.phase =
                                    mackes_ipc::AssignmentPhase::ConfirmReplace;
                                self.assignment_generation =
                                    self.assignment_generation.saturating_add(1);
                                return mackes_ipc::AssignmentResult {
                                    generation: self.assignment_generation,
                                    session: self.assignment_session.clone(),
                                    applied: true,
                                    reason: Some(
                                        "existing mapping found; confirm replacement".into(),
                                    ),
                                };
                            }
                            reason = Some(format!("assignment activation failed: {error}"));
                        } else if let Some(path) = self.config_path.as_deref() {
                            if mackes_config::save_control_mapping_store(
                                path,
                                &self.mapping_store,
                                1,
                            )
                            .is_err()
                            {
                                self.mapping_store = previous_store;
                                reason = Some("assignment persistence failed".into());
                            } else {
                                self.assignment_previous_store = Some(previous_store);
                            }
                        } else {
                            self.assignment_previous_store = Some(previous_store);
                        }
                    }
                }
            }
            if reason.is_some() {
                self.assignment_session.catalog.last_error.clone_from(&reason);
                self.assignment_session.catalog.pending_action =
                    Some(format!("{:?}", request.action));
                return mackes_ipc::AssignmentResult {
                    generation: self.assignment_generation,
                    session: self.assignment_session.clone(),
                    applied: false,
                    reason,
                };
            }
            if let Some(control) = request.physical_control_id {
                self.assignment_control_id = Some(control.clone());
                self.assignment_session.has_draft =
                    self.assignment_session.has_draft || !control.is_empty();
            }
            applied = self.assignment_session.apply(request.action);
            if applied {
                if request.action == mackes_ipc::AssignmentAction::Back {
                    self.assignment_pending_mapping = None;
                }
                if request.action == mackes_ipc::AssignmentAction::Interrupt {
                    if let (Some(path), Some(control)) =
                        (self.config_path.as_deref(), self.assignment_control_id.as_deref())
                    {
                        let draft = mackes_config::ControlMappingDraft {
                            id: "assignment-interrupted".into(),
                            step: format!(
                                "{:?}",
                                self.assignment_session
                                    .interrupted_phase
                                    .unwrap_or(mackes_ipc::AssignmentPhase::AwaitControl)
                            ),
                            physical_control_id: Some(control.into()),
                            destination: None,
                        };
                        let _ = self
                            .mapping_store
                            .save_draft(self.mapping_store.generation, draft)
                            .and_then(|()| {
                                mackes_config::save_control_mapping_store(
                                    path,
                                    &self.mapping_store,
                                    1,
                                )
                                .map_err(|_| "assignment draft persistence failed")
                            });
                    }
                }
                if matches!(
                    request.action,
                    mackes_ipc::AssignmentAction::Fail | mackes_ipc::AssignmentAction::Cancel
                ) {
                    self.assignment_pending_mapping = None;
                    if let Some(previous) = self.assignment_previous_store.take() {
                        self.mapping_store = previous;
                        if let Some(path) = self.config_path.as_deref() {
                            if let Err(error) = mackes_config::save_control_mapping_store(
                                path,
                                &self.mapping_store,
                                1,
                            ) {
                                reason = Some(format!(
                                    "assignment rollback persistence failed: {error}"
                                ));
                            }
                        }
                    }
                } else if request.action == mackes_ipc::AssignmentAction::Succeed {
                    self.assignment_pending_mapping = None;
                    self.assignment_previous_store = None;
                }
                self.assignment_generation = self.assignment_generation.saturating_add(1);
                if request.action == mackes_ipc::AssignmentAction::Commit
                    || request.action == mackes_ipc::AssignmentAction::ConfirmReplace
                    || request.action == mackes_ipc::AssignmentAction::Succeed
                    || request.action == mackes_ipc::AssignmentAction::Fail
                    || request.action == mackes_ipc::AssignmentAction::Cancel
                {
                    self.led.request_full_resync();
                }
                self.flush_controller_leds();
                self.record_state_event(Command::Assignment);
                if applied
                    && self.assignment_session.phase == mackes_ipc::AssignmentPhase::Committing
                    && reason.is_none()
                {
                    return self.apply_assignment_request(mackes_ipc::AssignmentRequest {
                        generation: self.assignment_generation,
                        action: mackes_ipc::AssignmentAction::Succeed,
                        physical_control_id: None,
                        destination_profile: None,
                        destination_effect: None,
                        destination_parameter: None,
                    });
                }
            }
        }
        self.sync_assignment_catalog();
        self.assignment_session.catalog.pending_action = Some(format!("{:?}", request.action));
        self.assignment_session.catalog.last_error.clone_from(&reason);
        if applied {
            self.assignment_session.catalog.last_result =
                Some(format!("{:?}", self.assignment_session.phase));
        }
        mackes_ipc::AssignmentResult {
            generation: self.assignment_generation,
            session: self.assignment_session.clone(),
            applied,
            reason,
        }
    }
    fn project_reflex_preset_to_controller(&mut self, preset_id: &str) {
        let Ok(values) =
            mackes_profiles::lexicon_reflex::pcm70_translation_controller_values(preset_id)
        else {
            return;
        };
        let values = values.into_iter().collect::<std::collections::BTreeMap<_, _>>();
        let infos = self.outputs.output_infos();
        let Ok((endpoint, _)) = led_surface::unique_output_from_infos(&infos) else {
            self.flush_controller_leds();
            return;
        };
        let mappings = self
            .mapping_store
            .active
            .iter()
            .filter(|mapping| mapping.enabled && mapping.destination_profile == "lexicon.reflex")
            .cloned()
            .collect::<Vec<_>>();
        for mapping in mappings {
            let Some(parameter) = mapping
                .destination_parameter
                .strip_prefix("reflex.parameter-")
                .and_then(|value| value.parse::<u8>().ok())
            else {
                continue;
            };
            let Some(value) = values.get(&parameter).copied() else { continue };
            if self
                .outputs
                .send_to_endpoint(
                    endpoint,
                    mackes_domain::MidiEvent {
                        timestamp: mackes_domain::TimestampNanos::new(0),
                        sequence: 0,
                        endpoint,
                        message: mackes_domain::MidiMessage::ControlChange {
                            channel: mackes_domain::MidiChannel::new(mapping.source_channel)
                                .expect("validated mapping channel"),
                            controller: mackes_domain::SevenBit::new(u16::from(
                                mapping.source_number,
                            ))
                            .expect("validated mapping controller"),
                            value: mackes_domain::SevenBit::new(u16::from(value))
                                .expect("normalized controller value"),
                        },
                    },
                )
                .is_err()
            {
                return;
            }
        }
        self.flush_controller_leds();
    }
    /// Resolves a profile-owned destination to one unique registered output.
    fn output_endpoint_for_profile(&self, profile_id: &str) -> Option<mackes_domain::EndpointId> {
        profile_bindings::output_endpoint(&self.outputs, &self.profile_bindings, profile_id)
    }
    fn launch_control_any_channel_id(&self, event: &mackes_domain::MidiEvent) -> Option<String> {
        let name = self.inputs.name_for_endpoint(event.endpoint)?;
        if !name.to_ascii_lowercase().contains("launch control xl") {
            return None;
        }
        let (kind, number) = match event.message {
            mackes_domain::MidiMessage::ControlChange { controller, .. } => {
                (mackes_profiles::LaunchControlSourceKind::ControlChange, controller.as_u8())
            }
            mackes_domain::MidiMessage::NoteOn { note, .. } => {
                (mackes_profiles::LaunchControlSourceKind::Note, note.as_u8())
            }
            _ => return None,
        };
        let mut matches = mackes_profiles::launch_control_mk2_factory1_layout()
            .into_iter()
            .filter(|control| control.source_kind == kind && control.source_number == number);
        let control = matches.next()?;
        matches.next().is_none().then_some(control.physical_control_id)
    }
    /// Resolves a persisted source alias to the currently registered runtime input.
    fn source_alias_matches_runtime(&self, alias: &str, runtime_id: Option<&str>) -> bool {
        let Some(runtime_id) = runtime_id else { return false };
        let Some(path) = self.config_path.as_deref() else { return false };
        mackes_config::load(path)
            .ok()
            .and_then(|document| {
                document.endpoints.into_iter().find(|endpoint| endpoint.id == alias)
            })
            .and_then(|endpoint| endpoint.stable_id)
            .is_some_and(|stable_id| stable_id == runtime_id)
    }
    /// Dispatches one event through the daemon-owned output registry.
    /// Returns bounded aggregate MIDI activity counters.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn dispatch_registered(&mut self, event: &mackes_domain::MidiEvent) -> (usize, usize) {
        let _ = self.activity.push(event);
        if matches!(event.message, mackes_domain::MidiMessage::SysEx(_)) {
            let frame = event.message.wire_bytes();
            match mackes_profiles::lexicon_reflex::decode_message(&frame) {
                Ok(mackes_profiles::lexicon_reflex::DecodedMessage::ActiveSetup {
                    setup, ..
                }) => {
                    match mackes_profiles::lexicon_reflex::ReflexSetup::new(&setup)
                        .ok()
                        .and_then(|setup| setup.algorithm())
                    {
                        Some(algorithm) => self.confirm_lexicon_algorithm(algorithm),
                        None => self.reject_lexicon_readback("active setup has invalid algorithm"),
                    }
                }
                Ok(_) => {}
                Err(error) => self.reject_lexicon_readback(error),
            }
            if self.lexicon_active_algorithm.is_none()
                && frame.len() == 65
                && frame.get(0..5) == Some(&[0xF0, 0x06, 0x02, 0x00, 0x38])
                && frame[64] == 0xF7
                && mackes_profiles::lexicon_reflex::checksum(&frame[5..61]) == frame[63]
            {
                self.confirm_lexicon_algorithm(frame[6] | ((frame[5] & 1) << 7));
            }
        }
        let stable_endpoint = self.inputs.stable_id_for_endpoint(event.endpoint);
        let routed = self.route_event(event);
        let mut sent = 0;
        let mut unmatched = 0;
        let active_mappings = mapping_layers_runtime::effective_mappings(
            &self.mapping_store,
            self.mapping_layers_v2.as_ref(),
        )
        .into_iter()
        .filter(|mapping| mapping.enabled)
        .collect::<Vec<_>>();
        for mapping in active_mappings {
            let now =
                u64::try_from(self.safety_clock.elapsed().as_nanos().min(u128::from(u64::MAX)))
                    .unwrap_or(u64::MAX);
            if mapping_runtime::is_experimental(&mapping) && !self.safety.unsafe_armed(now) {
                self.last_mapping_activity = Some(serde_json::json!({
                    "mapping_id": mapping.id,
                    "destination": mapping.destination_parameter,
                    "outcome": "blocked",
                    "reason": "experimental_mapping_disarmed"
                }));
                continue;
            }
            let source_endpoint = if stable_endpoint.as_deref()
                == Some(mapping.source_endpoint.as_str())
                || self.source_alias_matches_runtime(
                    &mapping.source_endpoint,
                    stable_endpoint.as_deref(),
                ) {
                event.endpoint
            } else if let Some(source_endpoint) = binding_generation::legacy_source_endpoint(
                &mapping.source_endpoint,
                stable_endpoint.as_deref(),
                event.endpoint,
            ) {
                source_endpoint
            } else {
                continue;
            };
            let configured_destination =
                mackes_midi_engine::numeric_endpoint_id(&mapping.destination_endpoint);
            let destination_endpoint = configured_destination
                .filter(|endpoint| {
                    self.outputs
                        .output_ids()
                        .iter()
                        .any(|id| mackes_midi_engine::numeric_endpoint_id(id) == Some(*endpoint))
                })
                .or_else(|| self.output_endpoint_for_profile(&mapping.destination_profile))
                // Preserve the configured endpoint for the normal disconnected-output
                // accounting path below when no profile-owned output is available.
                .or(configured_destination);
            let Some(destination_endpoint) = destination_endpoint else {
                continue;
            };
            let class = match mapping.source_kind.as_str() {
                "cc" => mackes_midi_engine::MessageClass::ControlChange,
                "pc" => mackes_midi_engine::MessageClass::ProgramChange,
                "note" => mackes_midi_engine::MessageClass::Note,
                _ => continue,
            };
            let parameter = mackes_midi_engine::ParameterMapping {
                source_endpoint,
                destination_endpoint,
                class,
                number: mapping.source_number,
                channel: Some(mapping.source_channel),
                source_range: mapping.behavior.source_range,
                destination_range: mapping.behavior.destination_range,
                invert: mapping.behavior.invert,
                curve: match mapping.behavior.curve.as_str() {
                    "square" => mackes_midi_engine::Curve::Square,
                    "square_root" => mackes_midi_engine::Curve::SquareRoot,
                    _ => mackes_midi_engine::Curve::Linear,
                },
            };
            if let Some((mut mapped, input_value)) = parameter.evaluate_with_value(event) {
                let Some(value) = mapping_runtime::destination_value(
                    &mut self.button_toggle_state,
                    &mapping,
                    input_value,
                ) else {
                    continue;
                };
                let Some(profile) = mackes_profiles::builtin_profile(&mapping.destination_profile)
                else {
                    continue;
                };
                let mut destination_value = value;
                if mapping.destination_profile == "lexicon.reflex" {
                    if let Some(parameter) = mapping
                        .destination_parameter
                        .strip_prefix("reflex.parameter-")
                        .and_then(|number| number.parse::<u8>().ok())
                    {
                        let Some(algorithm) = self.lexicon_active_algorithm else {
                            self.last_mapping_activity = Some(serde_json::json!({
                                "mapping_id": mapping.id,
                                "destination": mapping.destination_parameter,
                                "outcome": "blocked",
                                "reason": "lexicon_algorithm_unknown"
                            }));
                            continue;
                        };
                        let Ok(normalized) = u8::try_from(value.min(127)) else {
                            continue;
                        };
                        let Some(scaled) = mackes_profiles::lexicon_reflex::normalize_parameter(
                            algorithm, parameter, normalized,
                        ) else {
                            self.last_mapping_activity = Some(serde_json::json!({
                                "mapping_id": mapping.id,
                                "destination": mapping.destination_parameter,
                                "outcome": "blocked",
                                "reason": "parameter_not_supported_by_active_algorithm",
                                "active_algorithm": algorithm,
                                "parameter": parameter
                            }));
                            continue;
                        };
                        destination_value = scaled;
                    }
                }
                let channel =
                    mapping.destination_channel.map_or(1, |channel| channel.saturating_add(1));
                let Ok(bytes) = profile.render_parameter_message(
                    &mapping.destination_parameter,
                    channel,
                    destination_value,
                ) else {
                    continue;
                };
                let Ok(message) = mackes_domain::MidiMessage::from_wire(&bytes) else {
                    continue;
                };
                mapped.message = message;
                let confirms = mapping_runtime::needs_confirmation(&mapping);
                if confirms {
                    let now_ms =
                        u64::try_from(self.safety_clock.elapsed().as_millis()).unwrap_or(u64::MAX);
                    self.led.begin_backend_confirmation(
                        &mapping.physical_control_id,
                        value >= 64,
                        now_ms,
                    );
                    self.flush_controller_leds_at(now_ms);
                }
                let delivered = self.outputs.send_to_endpoint(destination_endpoint, mapped).is_ok();
                if confirms {
                    self.led.finish_backend_confirmation(delivered);
                }
                if delivered {
                    if mapping.destination_profile == "lexicon.reflex" {
                        if let Some(algorithm) = mapping
                            .destination_parameter
                            .strip_prefix("reflex.algorithm-")
                            .and_then(|value| value.parse::<u8>().ok())
                        {
                            self.confirm_lexicon_algorithm(algorithm);
                        }
                    }
                    if let Some(preset_id) =
                        mapping.destination_parameter.strip_prefix("pcm70_reflex:")
                    {
                        self.project_reflex_preset_to_controller(preset_id);
                    }
                    sent += 1;
                    self.last_mapping_activity = Some(serde_json::json!({
                        "mapping_id": mapping.id,
                        "destination": mapping.destination_parameter,
                        "outcome": "sent",
                        "source_value": value,
                        "wire_bytes": bytes,
                        "destination_channel": channel
                    }));
                } else {
                    unmatched += 1;
                    self.last_mapping_activity = Some(serde_json::json!({
                        "mapping_id": mapping.id,
                        "destination": mapping.destination_parameter,
                        "outcome": "blocked",
                        "reason": "destination_disconnected",
                        "source_value": value,
                        "wire_bytes": bytes,
                        "destination_channel": channel
                    }));
                }
            }
        }
        let (route_sent, route_unmatched) = self.outputs.dispatch(&self.router, event);
        sent += route_sent;
        unmatched += route_unmatched;
        self.received_events = self.received_events.saturating_add(1);
        self.sent_events = self.sent_events.saturating_add(sent as u64);
        self.dropped_events = self.dropped_events.saturating_add(unmatched as u64);
        let first_activity = self.last_activity.is_none();
        let mut activity = midi_activity_json(event, &routed, stable_endpoint.as_deref());
        // Preserve the controller's stable physical identity in the activity projection. The
        // browser cannot safely infer a knob/button from raw MIDI channel/number details, and
        // normal Studio feedback must remain protocol-free. This is observation metadata only;
        // it does not create or mutate a mapping.
        let physical_control_id = Self::launch_control_factory1_control_id(event)
            .or_else(|| self.launch_control_any_channel_id(event));
        if let Some(control_id) = physical_control_id {
            if let Some(object) = activity.as_object_mut() {
                object.insert("physical_control_id".into(), serde_json::Value::String(control_id));
                if let Some(value) = object.get("value").cloned() {
                    object.insert("observed_value".into(), value);
                }
            }
        }
        self.last_activity = Some(activity);
        // Publish the post-dispatch counters so subscribed dashboards receive live activity
        // without needing a second command to trigger a journal append.
        if first_activity || self.last_activity_publish.elapsed() >= Duration::from_millis(33) {
            self.last_activity_publish = Instant::now();
            self.record_state_event(Command::Monitor);
        }
        (sent, unmatched)
    }
    /// Applies a confirmed daemon-owned device control to a registered output.
    #[allow(clippy::missing_errors_doc)]
    pub fn apply_device_control(&mut self, request: &serde_json::Value) -> Result<Vec<u8>, String> {
        if request.get("confirm").and_then(serde_json::Value::as_bool) != Some(true) {
            return Err("device control requires confirmation".into());
        }
        let profile_id = request
            .get("profile_id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "device control fields are required".to_owned())?;
        let control = request
            .get("control")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "device control fields are required".to_owned())?;
        let destination = request
            .get("destination")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "device control fields are required".to_owned())?;
        if profile_id == "lexicon.reflex" && control == "system-reset" {
            let payload = vec![0xFF];
            self.outputs.send_direct(destination, &payload)?;
            self.record_physical_send(destination, "device-control:lexicon.reflex:system-reset");
            return Ok(payload);
        }
        let channel = request
            .get("channel")
            .and_then(serde_json::Value::as_u64)
            .and_then(|number| u8::try_from(number).ok())
            .ok_or_else(|| "device control fields are required".to_owned())?;
        let control_value = request
            .get("value")
            .and_then(serde_json::Value::as_u64)
            .and_then(|number| u16::try_from(number).ok())
            .ok_or_else(|| "device control fields are required".to_owned())?;
        let profile =
            mackes_profiles::builtin_profile(profile_id).ok_or("unknown device profile")?;
        let payload = profile
            .render_control_message(control, channel, control_value)
            .map_err(|_| "invalid device control".to_owned())?;
        self.outputs.send_direct(destination, &payload)?;
        self.record_physical_send(destination, format!("device-control:{profile_id}:{control}"));
        if profile_id == "eventide.micropitch" && control.eq_ignore_ascii_case("ACTIVE/BYPASS") {
            let active = control_value >= 64;
            let mapped: Vec<(String, String)> = self
                .mapping_store
                .active
                .iter()
                .filter(|mapping| {
                    mapping.destination_profile == profile_id
                        && mapping.destination_parameter == "control-2"
                })
                .map(|mapping| (mapping.id.clone(), mapping.physical_control_id.clone()))
                .collect();
            let now_ms = u64::try_from(self.safety_clock.elapsed().as_millis()).unwrap_or(u64::MAX);
            for (mapping_id, control_id) in mapped {
                self.button_toggle_state.insert(mapping_id, (false, active));
                self.led.begin_backend_confirmation(&control_id, active, now_ms);
                self.led.finish_backend_confirmation(true);
            }
        }
        Ok(payload)
    }
    /// Returns bounded aggregate MIDI activity counters.
    #[must_use]
    pub const fn activity_counters(&self) -> (u64, u64, u64) {
        (self.received_events, self.sent_events, self.dropped_events)
    }
    /// Records a validated startup scene for dashboard snapshots and events.
    pub fn set_active_scene(&mut self, scene: Option<String>) {
        self.active_scene = scene;
        if let Some(layers) = self.mapping_layers_v2.as_mut() {
            if layers.active_layer.take().is_some() {
                if let Some(path) = self.config_path.as_deref() {
                    let _ = mackes_config::save_mapping_layers(path, layers, 1);
                }
            }
        }
        self.record_state_event(Command::Scenes);
    }
    /// Returns the currently selected scene projected by the daemon.
    #[must_use]
    pub fn active_scene(&self) -> Option<&str> {
        self.active_scene.as_deref()
    }
    /// Selects the next or previous scene in the daemon-owned catalog.
    pub fn navigate_scene(&mut self, next: bool) -> Option<String> {
        self.try_navigate_scene(next).ok().flatten().or_else(|| self.active_scene.clone())
    }
    /// Selects the next or previous scene and persists it before reporting success.
    ///
    /// # Errors
    ///
    /// Returns an error when the selected scene cannot be persisted.
    pub fn try_navigate_scene(&mut self, next: bool) -> Result<Option<String>, &'static str> {
        if self.scene_ids.is_empty() {
            return Ok(self.active_scene.clone());
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
        self.select_scene_only(&scene).map(Some)
    }
    /// Selects an exact scene from the daemon-owned catalog and persists it.
    ///
    /// # Errors
    ///
    /// Returns an error when the scene is absent or persistence fails.
    pub fn select_scene(&mut self, scene: &str) -> Result<String, &'static str> {
        let selected = self.select_scene_only(scene)?;
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
    /// Selects and persists a scene without executing its actions.
    ///
    /// # Errors
    ///
    /// Returns an error when the scene is absent or persistence fails.
    pub fn select_scene_only(&mut self, scene: &str) -> Result<String, &'static str> {
        if !self.scene_ids.iter().any(|candidate| candidate == scene) {
            return Err("scene is not present in the active catalog");
        }
        let selected = scene.to_owned();
        if let Some(path) = self.config_path.as_deref() {
            persist_active_scene(path, Some(scene)).map_err(|_| "scene persistence failed")?;
        }
        self.set_active_scene(Some(selected.clone()));
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
        #[cfg(feature = "alsa-seq-backend")]
        {
            let client = if let Some(client) = self.alsa_input_client.clone() {
                client
            } else {
                let client = std::sync::Arc::new(std::sync::Mutex::new(
                    mackes_midi_engine::AlsaSequencerClient::open("MACKES input")?,
                ));
                self.alsa_input_client = Some(client.clone());
                client
            };
            let input =
                mackes_midi_engine::AlsaInputCapture::open_named_with_client(name, &client)?;
            self.alsa_supervisor.remember(
                mackes_midi_engine::MidiInputAdapter::info(&input).id.clone(),
                mackes_midi_engine::NativeHardwareIdentity::new(
                    name,
                    name,
                    0,
                    mackes_midi_engine::EndpointDirection::Input,
                ),
            );
            self.register_input(Box::new(input)).map_err(str::to_owned)
        }
        #[cfg(not(feature = "alsa-seq-backend"))]
        {
            let input = mackes_midi_engine::MidirInputCapture::open_named(name)?;
            self.register_input(Box::new(input)).map_err(str::to_owned)
        }
    }
    /// Polls each daemon-owned input once in stable registration order.
    #[must_use]
    pub fn poll_inputs(&mut self) -> Vec<mackes_domain::MidiEvent> {
        #[cfg(feature = "alsa-seq-backend")]
        self.poll_native_alsa_lifecycle();
        let mut events = self.deferred_inputs.drain(..).collect::<Vec<_>>();
        events.extend(self.inputs.poll_once());
        events
    }
    #[cfg(feature = "alsa-seq-backend")]
    fn poll_native_alsa_lifecycle(&mut self) {
        let Some(client) = self.alsa_input_client.clone() else {
            return;
        };
        let Ok(mut client) = client.lock() else {
            return;
        };
        let should_rescan = self
            .native_rescan_at
            .is_none_or(|last| last.elapsed() >= Duration::from_millis(NATIVE_RESCAN_INTERVAL_MS));
        let ports = if should_rescan { client.discover_ports() } else { Vec::new() };
        if should_rescan {
            self.native_rescan_at = Some(Instant::now());
        }
        let announcements = client
            .read_lifecycle_events(32)
            .into_iter()
            .map(|(lifecycle, address)| {
                let identity = ports
                    .iter()
                    .find(|port| port.address == address)
                    .map(mackes_midi_engine::NativeHardwareIdentity::from_alsa_port);
                mackes_midi_engine::NativePortAnnouncement {
                    lifecycle,
                    address,
                    identity,
                    permission_denied: false,
                }
            })
            .collect::<Vec<_>>();
        let _ = client.reconcile_input_subscriptions();
        drop(client);
        if should_rescan {
            self.restore_launch_control_midi_output(&ports);
            self.restore_late_physical_outputs();
        }
        for announcement in announcements {
            self.alsa_supervisor.ingest(announcement);
        }
        let transitions = self.alsa_supervisor.reconcile();
        self.apply_native_transitions(transitions);
    }
    /// Polls registered MIDI inputs and routes a bounded batch through the normal path.
    ///
    /// The bound prevents a busy physical controller from starving IPC and scene work.
    #[allow(clippy::too_many_lines)]
    pub fn poll_and_dispatch_inputs(&mut self, limit: usize) -> (usize, usize, usize) {
        let mut events = self.poll_inputs();
        let budget = limit.min(128);
        if events.len() > budget {
            let deferred = events.drain(budget..);
            for event in deferred {
                if self.deferred_inputs.len() < 256 {
                    self.deferred_inputs.push_back(event);
                } else {
                    self.dropped_events = self.dropped_events.saturating_add(1);
                }
            }
        }
        let mut processed = 0;
        let mut sent = 0;
        let mut unmatched = 0;
        for event in events {
            if matches!(event.message, mackes_domain::MidiMessage::SysEx(_)) {
                let frame = event.message.wire_bytes();
                if let Ok(mackes_profiles::lexicon_reflex::DecodedMessage::ActiveSetup {
                    setup,
                    ..
                }) = mackes_profiles::lexicon_reflex::decode_message(&frame)
                {
                    if let Ok(setup) = mackes_profiles::lexicon_reflex::ReflexSetup::new(&setup) {
                        if let Some(algorithm) = setup.algorithm() {
                            self.confirm_lexicon_algorithm(algorithm);
                        }
                    }
                }
                if self.lexicon_active_algorithm.is_none()
                    && frame.len() == 65
                    && frame.get(0..5) == Some(&[0xF0, 0x06, 0x02, 0x00, 0x38])
                    && frame[64] == 0xF7
                    && mackes_profiles::lexicon_reflex::checksum(&frame[5..61]) == frame[63]
                {
                    self.confirm_lexicon_algorithm(frame[6] | ((frame[5] & 1) << 7));
                }
            }
            if Self::launch_control_factory1_layout_id(&event).is_some() {
                let now_ms =
                    u64::try_from(self.safety_clock.elapsed().as_millis()).unwrap_or(u64::MAX);
                self.led.record_touch(now_ms);
            }
            if let Some(control_id) = Self::launch_control_factory1_layout_id(&event) {
                let now_ms =
                    u64::try_from(self.safety_clock.elapsed().as_millis()).unwrap_or(u64::MAX);
                let pressed = match event.message {
                    mackes_domain::MidiMessage::ControlChange { value, .. } => value.as_u8() != 0,
                    mackes_domain::MidiMessage::NoteOn { velocity, .. } => velocity.as_u8() != 0,
                    _ => false,
                };
                self.led.record_arrow(&control_id, pressed, now_ms);
                if control_id.starts_with("knob-")
                    && matches!(event.message, mackes_domain::MidiMessage::ControlChange { .. })
                {
                    self.led.record_knob_activity(&control_id, now_ms);
                }
            }
            if let Some(control_id) = Self::launch_control_factory1_control_id(&event) {
                if control_id == "utility-1" {
                    self.assignment_device_down_at = Some(Instant::now());
                    if self.assignment_session.phase == mackes_ipc::AssignmentPhase::Idle {
                        let _ = self.apply_assignment_request(mackes_ipc::AssignmentRequest {
                            generation: self.assignment_generation,
                            action: mackes_ipc::AssignmentAction::Start,
                            physical_control_id: None,
                            destination_profile: None,
                            destination_effect: None,
                            destination_parameter: None,
                        });
                    }
                    processed += 1;
                    continue;
                }
            } else if Self::launch_control_factory1_layout_id(&event).as_deref()
                == Some("utility-1")
            {
                if let Some(started) = self.assignment_device_down_at.take() {
                    if started.elapsed() >= Duration::from_millis(750)
                        && self.assignment_session.phase != mackes_ipc::AssignmentPhase::Idle
                    {
                        let _ = self.apply_assignment_request(mackes_ipc::AssignmentRequest {
                            generation: self.assignment_generation,
                            action: mackes_ipc::AssignmentAction::Cancel,
                            physical_control_id: None,
                            destination_profile: None,
                            destination_effect: None,
                            destination_parameter: None,
                        });
                    }
                }
                processed += 1;
                continue;
            }
            // A delayed or dropped Device release must never turn the next
            // unrelated controller event into a cancellation. Hold-to-cancel
            // is decided only by the matching physical Device release above.
            if self.assignment_session.phase != mackes_ipc::AssignmentPhase::Idle
                && self.assignment_session.phase != mackes_ipc::AssignmentPhase::AwaitControl
            {
                if let Some(control_id) = Self::launch_control_factory1_control_id(&event) {
                    let captured = self.assignment_control_id.as_deref();
                    let assignable =
                        mackes_profiles::launch_control_physical_catalog().iter().any(|control| {
                            control.id.as_str() == control_id
                                && control.role != mackes_profiles::PhysicalControlRole::Utility
                        });
                    if assignable
                        && captured.is_some_and(|id| id != control_id)
                        && self.assignment_capture_at.is_some_and(|started| {
                            started.elapsed() < Duration::from_millis(NATIVE_RESCAN_INTERVAL_MS)
                        })
                    {
                        self.assignment_session.catalog.last_error =
                            Some("ambiguous capture within 250 ms".into());
                        processed += 1;
                        continue;
                    }
                }
            }
            if self.assignment_session.phase == mackes_ipc::AssignmentPhase::AwaitControl {
                if let Some(control_id) = Self::launch_control_factory1_control_id(&event) {
                    let Some(stable_source) = self.inputs.stable_id_for_endpoint(event.endpoint)
                    else {
                        self.assignment_session.catalog.last_error =
                            Some("input identity is unresolved; repair the device binding".into());
                        processed += 1;
                        continue;
                    };
                    let result = self.apply_assignment_request(mackes_ipc::AssignmentRequest {
                        generation: self.assignment_generation,
                        action: mackes_ipc::AssignmentAction::ControlCaptured,
                        physical_control_id: Some(control_id.clone()),
                        destination_profile: None,
                        destination_effect: None,
                        destination_parameter: None,
                    });
                    if result.applied {
                        self.assignment_capture_at = Some(Instant::now());
                        self.assignment_session.catalog.source_endpoint = Some(stable_source);
                        if let mackes_domain::MidiMessage::ControlChange {
                            channel,
                            controller,
                            ..
                        }
                        | mackes_domain::MidiMessage::NoteOn {
                            channel,
                            note: controller,
                            ..
                        } = event.message
                        {
                            self.assignment_session.catalog.source_channel = Some(channel.wire());
                            self.assignment_session.catalog.source_number =
                                Some(controller.as_u8());
                        }
                        processed += 1;
                        continue;
                    }
                }
            }
            if let Some(action) = Self::launch_control_factory1_navigation(&event) {
                if self.assignment_session.phase != mackes_ipc::AssignmentPhase::Idle {
                    let assignment_action = match action {
                        "up" => mackes_ipc::AssignmentAction::Up,
                        "down" => mackes_ipc::AssignmentAction::Down,
                        "left" => mackes_ipc::AssignmentAction::Back,
                        "right"
                            if self.assignment_session.phase
                                == mackes_ipc::AssignmentPhase::ConfirmReplace =>
                        {
                            mackes_ipc::AssignmentAction::ConfirmReplace
                        }
                        _ => mackes_ipc::AssignmentAction::Enter,
                    };
                    let _ = self.apply_assignment_request(mackes_ipc::AssignmentRequest {
                        generation: self.assignment_generation,
                        action: assignment_action,
                        physical_control_id: None,
                        destination_profile: None,
                        destination_effect: None,
                        destination_parameter: None,
                    });
                    processed += 1;
                    continue;
                }
                self.record_navigation_event(action);
                processed += 1;
                continue;
            }
            if let Some(control_id) = Self::launch_control_factory1_control_id(&event) {
                if let mackes_domain::MidiMessage::ControlChange { value, .. } = &event.message {
                    let mappings = self
                        .pipedal_mappings
                        .iter()
                        .filter(|mapping| mapping.physical_control_id == control_id)
                        .cloned()
                        .collect::<Vec<_>>();
                    let generation = self.pipedal_worker.health().generation;
                    for mapping in mappings {
                        let _ = self.pipedal_worker.apply_physical_control(
                            generation,
                            &mapping,
                            value.as_u8(),
                            0.01,
                        );
                    }
                }
            }
            let (event_sent, event_unmatched) = self.dispatch_registered(&event);
            processed += 1;
            sent += event_sent;
            unmatched += event_unmatched;
        }
        self.flush_controller_leds();
        (processed, sent, unmatched)
    }
    #[cfg(test)]
    fn is_launch_control_factory1_device_press(event: &mackes_domain::MidiEvent) -> bool {
        matches!(Self::launch_control_factory1_control_id(event).as_deref(), Some("utility-1"))
    }
    fn launch_control_factory1_navigation(
        event: &mackes_domain::MidiEvent,
    ) -> Option<&'static str> {
        let mackes_domain::MidiMessage::ControlChange { value, .. } = event.message else {
            return None;
        };
        if value.as_u8() == 0 {
            return None;
        }
        match Self::launch_control_factory1_control_id(event).as_deref() {
            Some("utility-5") => Some("up"),
            Some("utility-6") => Some("down"),
            Some("utility-7") => Some("left"),
            Some("utility-8") => Some("right"),
            _ => None,
        }
    }
    fn launch_control_factory1_layout_id(event: &mackes_domain::MidiEvent) -> Option<String> {
        let (kind, number, channel) = match event.message {
            mackes_domain::MidiMessage::ControlChange { channel, controller, .. } => (
                mackes_profiles::LaunchControlSourceKind::ControlChange,
                controller.as_u8(),
                channel.wire(),
            ),
            mackes_domain::MidiMessage::NoteOn { channel, note, .. } => {
                (mackes_profiles::LaunchControlSourceKind::Note, note.as_u8(), channel.wire())
            }
            _ => return None,
        };
        mackes_profiles::launch_control_mk2_factory1_layout().into_iter().find_map(|control| {
            (control.channel == channel
                && control.source_kind == kind
                && control.source_number == number)
                .then_some(control.physical_control_id)
        })
    }
    fn launch_control_factory1_control_id(event: &mackes_domain::MidiEvent) -> Option<String> {
        let (kind, number, channel, value) = match event.message {
            mackes_domain::MidiMessage::ControlChange { channel, controller, value } => (
                mackes_profiles::LaunchControlSourceKind::ControlChange,
                controller.as_u8(),
                channel.wire(),
                value.as_u8(),
            ),
            mackes_domain::MidiMessage::NoteOn { channel, note, velocity } => (
                mackes_profiles::LaunchControlSourceKind::Note,
                note.as_u8(),
                channel.wire(),
                velocity.as_u8(),
            ),
            _ => return None,
        };
        mackes_profiles::resolve_launch_control_mk2_factory1_input(channel, kind, number, value)
    }
    fn record_navigation_event(&mut self, action: &'static str) {
        self.record_state_event(Command::Monitor);
        if let Some(event) = self.state_events.back_mut() {
            if let Ok(mut payload) = serde_json::from_slice::<serde_json::Value>(&event.payload) {
                payload["ui_navigation"] = serde_json::Value::String(action.to_owned());
                if let Ok(encoded) = serde_json::to_vec(&payload) {
                    event.payload = encoded;
                }
            }
        }
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
            let mut consumed_by_binding = false;
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
                    consumed_by_binding = true;
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
            } else if !consumed_by_binding && self.deferred_inputs.len() < 256 {
                self.deferred_inputs.push_back(event);
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
        events.iter().map(|event| self.dispatch_registered(event)).collect()
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
        self.activation_outcomes = Some(
            results
                .iter()
                .take(128)
                .map(|(id, result)| serde_json::json!({"id": id, "outcome": format!("{result:?}")}))
                .collect(),
        );
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
        // Control movement is the hottest event path. Keep these journal entries compact: copying
        // the full configuration, catalogs, capabilities, and device inventory for every MIDI tick
        // makes the bounded replay journal consume hundreds of megabytes and can saturate the local
        // IPC accept queue. Clients obtain that authoritative state from the snapshot endpoints.
        if command == Command::Monitor {
            let payload = serde_json::to_vec(&serde_json::json!({
                "command": command.tag(),
                "generation": self.generation,
                "received": self.received_events,
                "sent": self.sent_events,
                "dropped": self.dropped_events,
                "last_activity": self.last_activity,
                "last_mapping_activity": self.last_mapping_activity,
                "health": match self.health {
                    Health::Starting => "starting",
                    Health::Ready => "ready",
                    Health::Degraded => "degraded",
                    Health::Stopping => "stopping",
                },
            }))
            .unwrap_or_default();
            if self.state_events.len() == 256 {
                self.state_events.pop_front();
            }
            self.state_events
                .push_back(mackes_ipc::StateEvent { sequence: self.state_sequence, payload });
            return;
        }
        let payload = serde_json::to_vec(&serde_json::json!({
            "command": command.tag(),
            "generation": self.generation,
            "mapping_undo_available": self.mapping_store.undo_available(),
            "route_generation": self.route_generation(),
            "route_undo_available": self.route_undo.is_some(),
            "audit_count": self.audit.newest_first().count(),
            "audit": self.audit_projection(),
            "active_scene": self.active_scene.as_deref(),
            "received": self.received_events,
            "sent": self.sent_events,
            "dropped": self.dropped_events,
            "registered_inputs": self.inputs.len(),
            "last_activity": self.last_activity,
            "last_mapping_activity": self.last_mapping_activity,
            "control_mappings": self.mapping_store.active,
            "mapping_registry": self.resolved_mapping_registry(),
            "mapping_layers_v2": self.mapping_layers_v2,
            "rtp_midi": {"state": format!("{:?}", self.rtp_peer.state()), "remote_ssrc": self.rtp_peer.remote_ssrc(), "remote_name": self.rtp_peer.remote_name()},
            "activation_result": self.activation_result.as_deref(),
            "activation_outcomes": self.activation_outcomes,
            "activation_outcomes": self.activation_outcomes,
            "assignment_session": self.assignment_session,
            "catalog": self.catalog,
            "physical_devices": self.physical_devices,
            "novation_capabilities": mackes_profiles::launch_control_capability_descriptor(),
            "novation_device": self.novation_device_snapshot(),
            "config_persistence": persistence_projection::config_persistence(
                self.config_path.as_deref(),
            ),
            "pipedal": self.pipedal_worker.ipc_status(),
            "pipedal_catalog": self.pipedal_worker.catalog(),
            "pipedal_system_midi_bindings": self.pipedal_worker.system_midi_bindings(),
            "pipedal_version": self.pipedal_worker.version(),
            "pipedal_governor_settings": self.pipedal_worker.governor_settings(),
            "pipedal_show_status_monitor": self.pipedal_worker.show_status_monitor(),
            "pipedal_wifi_regulatory_domains": self.pipedal_worker.wifi_regulatory_domains(),
            "pipedal_operations": mackes_pipedal_adapter::supported_operations(),
            "pipedal_mapping_resolution": self.pipedal_mapping_resolution(),
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
    fn authorize_route_mutation(&mut self, action: &str) -> Result<(), &'static str> {
        let decision = self.safety.authorize_and_record(
            self.state_sequence,
            "local-ipc",
            mackes_scene_engine::AuditSource::LocalCli,
            mackes_scene_engine::GovernedOperation::ConfigurationEdit,
            action,
            "routing",
            mackes_scene_engine::RiskClass::Normal,
            mackes_scene_engine::ConfirmationClass::Normal,
            false,
            "route mutation requested",
            false,
            &mut self.audit,
        );
        (decision == mackes_scene_engine::PolicyDecision::Allow)
            .then_some(())
            .ok_or("route mutation denied by performance lock")
    }
    fn audit_projection(&self) -> Vec<serde_json::Value> {
        self.audit
            .newest_first()
            .take(32)
            .map(|record| {
                serde_json::json!({
                    "timestamp": record.timestamp,
                    "actor": record.actor,
                    "source": format!("{:?}", record.source),
                    "action": record.action_id,
                    "target": record.target_alias,
                    "risk": format!("{:?}", record.risk),
                    "allowed": record.allowed,
                    "result": record.result,
                })
            })
            .collect()
    }
    fn record_physical_send(&mut self, destination: &str, action: impl Into<String>) {
        self.sent_events = self.sent_events.saturating_add(1);
        self.audit.append(mackes_scene_engine::AuditRecord {
            timestamp: self.state_sequence,
            actor: "local-ipc".into(),
            source: mackes_scene_engine::AuditSource::LocalCli,
            action_id: action.into(),
            target_alias: destination.into(),
            risk: mackes_scene_engine::RiskClass::Normal,
            allowed: true,
            result: "sent".into(),
        });
        self.record_state_event(Command::Monitor);
    }
    fn novation_device_snapshot(&self) -> mackes_profiles::LaunchControlDeviceSnapshot {
        novation_snapshot::device_snapshot(
            &self.physical_devices,
            &self.profile_bindings,
            self.binding_generation,
            self.novation_device_policy.as_ref(),
        )
    }
    fn snapshot_response(&self) -> String {
        serde_json::json!({
            "ok": true,
            "generation": self.generation,
            "route_generation": self.route_generation(),
            "route_undo_available": self.route_undo.is_some(),
            "audit_count": self.audit.newest_first().count(),
            "audit": self.audit_projection(),
            "active_scene": self.active_scene.as_deref(),
            "received": self.received_events,
            "sent": self.sent_events,
            "dropped": self.dropped_events,
            "registered_inputs": self.inputs.len(),
            "last_activity": self.last_activity,
            "last_mapping_activity": self.last_mapping_activity,
            "control_mappings": self.mapping_store.active,
            "mapping_registry": self.resolved_mapping_registry(),
            "mapping_layers_v2": self.mapping_layers_v2,
            "activation_result": self.activation_result.as_deref(),
            "assignment_session": self.assignment_session,
            "last_sequence": self.state_sequence,
            "catalog": self.catalog,
            "physical_devices": self.physical_devices,
            "novation_capabilities": mackes_profiles::launch_control_capability_descriptor(),
            "novation_device": self.novation_device_snapshot(),
            "endpoint_bindings": self.endpoint_binding_projection(),
            "binding_generation": self.binding_generation,
            "native_backend": if cfg!(feature = "alsa-seq-backend") {
                "alsa-seq"
            } else {
                "midir-rollback"
            },
            "native_led_resync": self.native_led_resync,
            "native_rescan_interval_ms": NATIVE_RESCAN_INTERVAL_MS,
            "native_failure": self.last_native_failure,
            "config_persistence": persistence_projection::config_persistence(
                self.config_path.as_deref(),
            ),
            "led": {
                "phase": self.led.diagnostics().phase,
                "attempted": self.led.diagnostics().attempted,
                "sent": self.led.diagnostics().sent,
                "coalesced": self.led.diagnostics().coalesced,
                "failed": self.led.diagnostics().failed,
                "retries": self.led.diagnostics().retries,
                "last_error": self.led.diagnostics().last_error,
                "target_id": self.led.diagnostics().target_id,
                "template": self.led.diagnostics().template,
                "pending_deadline_ms": self.led.diagnostics().pending_deadline_ms,
                "desired_indices": self.led.diagnostics().desired_indices,
                "pending_indices": self.led.diagnostics().pending_indices,
                "backend_confirmation": self.led.diagnostics().backend_confirmation,
                "backend_state": self.led.diagnostics().backend_state,
                "active_lexicon_algorithm": self.lexicon_active_algorithm,
                "lexicon_readback_error": self.lexicon_readback_error,
            },
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
    fn monitor_response(&self) -> String {
        serde_json::json!({
            "ok": true,
            "generation": self.generation,
            "health": match self.health {
                Health::Starting => "starting",
                Health::Ready => "ready",
                Health::Degraded => "degraded",
                Health::Stopping => "stopping",
            },
            "received": self.received_events,
            "sent": self.sent_events,
            "dropped": self.dropped_events,
            "last_activity": self.last_activity,
            "last_mapping_activity": self.last_mapping_activity,
        })
        .to_string()
            + "\n"
    }
    fn endpoint_binding_projection(&self) -> serde_json::Value {
        let Some(path) = self.config_path.as_deref() else { return serde_json::json!([]) };
        let Ok(document) = mackes_config::load(path) else { return serde_json::json!([]) };
        let inputs = self.inputs.input_ids();
        let outputs = self.outputs.output_ids();
        serde_json::Value::Array(
            document
                .endpoints
                .into_iter()
                .map(|alias| {
                    let stable_id = alias.stable_id.clone();
                    let input_match = stable_id
                        .as_deref()
                        .is_some_and(|id| inputs.iter().any(|candidate| candidate == id));
                    let output_match = stable_id
                        .as_deref()
                        .is_some_and(|id| outputs.iter().any(|candidate| candidate == id));
                    let (state, action) = if stable_id.is_none() {
                        ("missing", "bind this alias to a verified stable device identity")
                    } else if alias.direction.is_none() && input_match && output_match {
                        ("ambiguous", "choose input or output direction for this alias")
                    } else if match alias.direction.as_deref() {
                        Some("input") => input_match,
                        Some("output") => output_match,
                        _ => input_match || output_match,
                    } {
                        ("connected", "none")
                    } else {
                        (
                            "missing",
                            "rescan and rebind this alias; display-name matching is disabled",
                        )
                    };
                    serde_json::json!({
                        "alias": alias.id,
                        "stable_id": stable_id,
                        "direction": alias.direction,
                        "role": alias.role,
                        "state": state,
                        "action": action,
                    })
                })
                .collect(),
        )
    }
    fn pipedal_mapping_resolution(&self) -> serde_json::Value {
        serde_json::to_value(self.pipedal_worker.resolve_mappings(&self.pipedal_mappings))
            .unwrap_or_else(|_| serde_json::json!([]))
    }
    fn resolved_mapping_registry(&self) -> Vec<serde_json::Value> {
        let physical = mackes_profiles::launch_control_physical_catalog();
        self.mapping_store
            .active
            .iter()
            .map(|mapping| {
                let control = physical
                    .iter()
                    .find(|item| item.id.as_str() == mapping.physical_control_id);
                let parameter = mackes_profiles::builtin_profile(&mapping.destination_profile)
                    .and_then(|profile| {
                        mackes_profiles::destination_parameters(&profile)
                            .into_iter()
                            .find(|item| item.id == mapping.destination_parameter)
                    });
                let channel = mapping.destination_channel.map_or(1, |value| value + 1);
                let wire_example = mackes_profiles::builtin_profile(&mapping.destination_profile)
                    .and_then(|profile| {
                        profile
                            .render_parameter_message(&mapping.destination_parameter, channel, 127)
                            .ok()
                    });
                let algorithm_labels = mapping
                    .destination_parameter
                    .strip_prefix("reflex.parameter-")
                    .and_then(|value| value.parse::<u8>().ok())
                    .map(|number| {
                        mackes_profiles::lexicon_reflex::algorithms()
                            .iter()
                            .filter_map(|algorithm| {
                                mackes_profiles::lexicon_reflex::parameters(algorithm.number)
                                    .iter()
                                    .find(|parameter| parameter.number == number)
                                    .map(|parameter| {
                                        format!("{}: {}", algorithm.name, parameter.mrc_name)
                                    })
                            })
                            .collect::<Vec<_>>()
                    });
                serde_json::json!({
                    "id": mapping.id,
                    "enabled": mapping.enabled,
                    "controller_profile": mapping.controller_profile, "physical_control_id": mapping.physical_control_id, "source_endpoint": mapping.source_endpoint, "source_kind": mapping.source_kind, "source_channel": mapping.source_channel, "destination_channel": mapping.destination_channel, "source_number": mapping.source_number, "destination_endpoint": mapping.destination_endpoint, "destination_profile": mapping.destination_profile, "destination_effect": mapping.destination_effect, "destination_parameter": mapping.destination_parameter, "behavior": mapping.behavior, "profile_version": mapping.profile_version,
                    "physical_control": mapping.physical_control_id,
                    "physical_role": control.map(|item| format!("{:?}", item.role)),
                    "input": {"endpoint": mapping.source_endpoint, "kind": mapping.source_kind, "channel": mapping.source_channel + 1, "number": mapping.source_number},
                    "device": {"profile": mapping.destination_profile, "parameter": mapping.destination_parameter, "label": parameter.map(|item| item.label), "algorithm_labels": algorithm_labels, "channel": channel, "endpoint": mapping.destination_endpoint, "wire_example": wire_example},
                    "led": if mapping.source_kind == "note" { "state: green active / red bypass" } else if control.is_some_and(|item| format!("{:?}", item.role) == "Knob") { "blink green while moving" } else { "normal assigned state" }
                })
            })
            .collect()
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
    #[allow(clippy::too_many_lines, missing_docs, clippy::missing_errors_doc)]
    pub fn serve_once(&mut self, policy: AccessPolicy) -> io::Result<()> {
        let _ = self.drain_virtual_input();
        let (mut stream, identity) = self.server.accept_authorized(policy)?;
        stream.set_read_timeout(Some(std::time::Duration::from_millis(100)))?;
        stream.set_write_timeout(Some(std::time::Duration::from_millis(100)))?;
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
        let request_payload = serde_json::from_slice::<serde_json::Value>(&request)
            .ok()
            .and_then(|envelope| envelope.get("payload").cloned())
            .and_then(|payload| serde_json::to_vec(&payload).ok())
            .unwrap_or_else(|| request.clone());
        let response = if command
            .is_some_and(|command| authorize(command, actor) == Authorization::Allowed)
        {
            self.health = mapping_runtime::health_after_authorized_command(self.health, command);
            self.generation = self.generation.saturating_add(1);
            if command == Some(Command::Migrate) {
                return stream.write_all(self.migration_response(&request_payload).as_bytes());
            }
            if command == Some(Command::Configuration) {
                let value = serde_json::from_slice::<serde_json::Value>(&request_payload)
                    .unwrap_or_default();
                if ["configuration", "setlists", "learned_mappings"]
                    .iter()
                    .any(|key| value.get(*key).is_some())
                {
                    let request_id = value.get("request_id").and_then(serde_json::Value::as_str);
                    let operation_id = value
                        .get("operation_id")
                        .and_then(serde_json::Value::as_str)
                        .or(request_id);
                    let fingerprint = request_id.map(|_| {
                        let mut encoded = String::with_capacity(request_payload.len() * 2);
                        for byte in &request_payload {
                            let _ = write!(&mut encoded, "{byte:02x}");
                        }
                        encoded
                    });
                    if let (Some(request_id), Some(fingerprint), Some(journal)) =
                        (request_id, fingerprint.as_deref(), self.operation_journal.as_ref())
                    {
                        match journal.lookup(request_id, fingerprint) {
                            Ok(Some(record)) => {
                                return stream
                                    .write_all((record.outcome.to_string() + "\n").as_bytes());
                            }
                            Err(error) => {
                                return stream
                                    .write_all(configuration_response::error(&error).as_bytes());
                            }
                            Ok(None) => {}
                        }
                    }
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
                    let expected_revision = configuration_response::revision(&value);
                    if let Err(error) =
                        configuration_response::apply_payload(&value, &path, &mut document)
                    {
                        return stream.write_all(configuration_response::error(&error).as_bytes());
                    }
                    let mut prepared_store =
                        match mackes_config::ControlMappingStore::from_document(&document) {
                            Ok(store) => store,
                            Err(message) => {
                                let error = mackes_config::ConfigError::Semantic {
                                    path: path.clone(),
                                    message,
                                };
                                return stream
                                    .write_all(configuration_response::error(&error).as_bytes());
                            }
                        };
                    prepared_store.active.retain(assignment_commit::mapping_role_compatible);
                    if let Err(error) = mackes_config::save_with_optional_revision(
                        &path,
                        &document,
                        5,
                        expected_revision.as_deref(),
                    ) {
                        return stream.write_all(configuration_response::error(&error).as_bytes());
                    }
                    self.novation_device_policy.clone_from(&document.settings.novation_device);
                    self.cache_pipedal_mappings(&document);
                    self.mapping_store = prepared_store;
                    self.assignment_session.has_draft = !self.mapping_store.drafts.is_empty();
                    self.sync_assignment_catalog();
                    self.catalog = serde_json::json!({"configuration": document, "configuration_revision": mackes_config::document_revision(&document).unwrap_or_default(), "projects": document.projects.iter().map(|project| serde_json::json!({"id": project.id, "scenes": project.scenes.iter().map(|scene| scene.id.clone()).collect::<Vec<_>>() })).collect::<Vec<_>>(), "setlists": document.setlists, "learned_mappings": document.learned_mappings});
                    if let (
                        Some(request_id),
                        Some(operation_id),
                        Some(fingerprint),
                        Some(journal),
                    ) = (request_id, operation_id, fingerprint, self.operation_journal.as_mut())
                    {
                        let outcome = serde_json::json!({"ok": true, "operation_id": operation_id});
                        let _ = journal.record(mackes_config::OperationRecord {
                            request_id: request_id.to_owned(),
                            fingerprint,
                            operation_id: operation_id.to_owned(),
                            outcome,
                        });
                    }
                }
            }
            if command == Some(Command::Routes) {
                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&request_payload) {
                    if value.get("action").and_then(serde_json::Value::as_str)
                        == Some("rtp_establish")
                    {
                        let token = value
                            .get("token")
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|v| u32::try_from(v).ok());
                        let ssrc = value
                            .get("ssrc")
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|v| u32::try_from(v).ok());
                        let result = token
                            .zip(ssrc)
                            .ok_or("RTP token and SSRC are required")
                            .and_then(|(token, ssrc)| self.establish_rtp_peer(token, ssrc));
                        return stream.write_all(
                            match result {
                                Ok(()) => format!(
                                    "{{\"ok\":true,\"generation\":{},\"rtp\":\"established\"}}\n",
                                    self.generation
                                ),
                                Err(error) => format!(
                                    "{{\"ok\":false,\"generation\":{},\"error\":\"{error}\"}}\n",
                                    self.generation
                                ),
                            }
                            .as_bytes(),
                        );
                    }
                    if value.get("action").and_then(serde_json::Value::as_str) == Some("rtp_end") {
                        let token = value
                            .get("token")
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|v| u32::try_from(v).ok());
                        let ssrc = value
                            .get("ssrc")
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|v| u32::try_from(v).ok());
                        let result = token
                            .zip(ssrc)
                            .ok_or("RTP token and SSRC are required")
                            .and_then(|(token, ssrc)| self.end_rtp_peer(token, ssrc));
                        return stream.write_all(
                            match result {
                                Ok(()) => format!(
                                    "{{\"ok\":true,\"generation\":{},\"rtp\":\"ended\"}}\n",
                                    self.generation
                                ),
                                Err(error) => format!(
                                    "{{\"ok\":false,\"generation\":{},\"error\":\"{error}\"}}\n",
                                    self.generation
                                ),
                            }
                            .as_bytes(),
                        );
                    }
                    if value.get("action").and_then(serde_json::Value::as_str) == Some("undo") {
                        if let Some(expected) =
                            value.get("route_generation").and_then(serde_json::Value::as_u64)
                        {
                            if self.route_generation() != Some(expected) {
                                return stream.write_all(
                                    b"{\"ok\":false,\"error\":\"mapping generation changed concurrently\"}\n",
                                );
                            }
                        }
                        if let Err(error) = self.authorize_route_mutation("route_undo") {
                            return stream.write_all(
                                format!("{{\"ok\":false,\"error\":\"{error}\"}}\n").as_bytes(),
                            );
                        }
                        let Some(previous) = self.route_undo.clone() else {
                            return stream.write_all(
                                b"{\"ok\":false,\"error\":\"route undo history is empty\"}\n",
                            );
                        };
                        let generation = self.route_generation().unwrap_or(0).saturating_add(1);
                        let encoded = serde_json::to_vec(&previous).map_err(io::Error::other)?;
                        if let Some(path) = self.config_path.as_deref() {
                            if let Err(error) =
                                persist_routes(path, &serde_json::json!({"routes": previous}))
                            {
                                return stream.write_all(
                                    format!("{{\"ok\":false,\"error\":\"route undo persistence failed: {error}\"}}\n").as_bytes(),
                                );
                            }
                        }
                        if let Err(error) = self.replace_routes_json(&encoded, generation, 8) {
                            if let Some(path) = self.config_path.as_deref() {
                                let current =
                                    serde_json::from_str::<serde_json::Value>(&command_ack(
                                        Command::Routes,
                                        self.health,
                                        self.generation,
                                        &[],
                                        self.route_generation(),
                                        &self.router.routes(),
                                    ))
                                    .ok()
                                    .and_then(|value| value.get("routes").cloned())
                                    .unwrap_or_else(|| serde_json::json!([]));
                                let _ =
                                    persist_routes(path, &serde_json::json!({"routes": current}));
                            }
                            return stream.write_all(
                                format!("{{\"ok\":false,\"error\":\"{error}\"}}\n").as_bytes(),
                            );
                        }
                        if let Some(path) = self.config_path.as_deref() {
                            let _ = fs::remove_file(routes_undo_path(path));
                        }
                        self.route_undo = None;
                        return stream.write_all(
                            format!(
                                "{{\"ok\":true,\"undo\":true,\"route_generation\":{generation}}}\n"
                            )
                            .as_bytes(),
                        );
                    }
                    if let Some(route_values) = value.get("routes") {
                        if let Err(error) = self.authorize_route_mutation("route_replace") {
                            return stream.write_all(
                                format!("{{\"ok\":false,\"error\":\"{error}\"}}\n").as_bytes(),
                            );
                        }
                        let generation = value
                            .get("route_generation")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(self.generation);
                        let hop_limit = value
                            .get("hop_limit")
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|value| u8::try_from(value).ok())
                            .unwrap_or(8);
                        let mut current_routes =
                            serde_json::from_str::<serde_json::Value>(&command_ack(
                                Command::Routes,
                                self.health,
                                self.generation,
                                &[],
                                self.route_generation(),
                                &self.router.routes(),
                            ))
                            .ok()
                            .and_then(|value| value.get("routes").cloned());
                        let encoded = serde_json::to_vec(route_values).map_err(io::Error::other)?;
                        if let Some(path) = self.config_path.as_deref() {
                            if let Some(previous) = current_routes.clone() {
                                if let Err(error) = persistence_projection::persist_json_pair_atomic(
                                    &routes_path(path),
                                    &serde_json::json!({"routes": route_values}),
                                    &routes_undo_path(path),
                                    &previous,
                                    &path.with_extension("routes.commit.json"),
                                ) {
                                    return stream.write_all(
                                        format!("{{\"ok\":false,\"error\":\"route pair persistence failed: {error}\"}}\n").as_bytes(),
                                    );
                                }
                            } else if let Err(error) =
                                persist_routes(path, &serde_json::json!({"routes": route_values}))
                            {
                                return stream.write_all(
                                    format!("{{\"ok\":false,\"error\":\"route persistence failed: {error}\"}}\n").as_bytes(),
                                );
                            }
                        }
                        if let Err(error) =
                            self.replace_routes_json(&encoded, generation, hop_limit)
                        {
                            if let (Some(path), Some(previous)) =
                                (self.config_path.as_deref(), current_routes.take())
                            {
                                let _ =
                                    persist_routes(path, &serde_json::json!({"routes": previous}));
                                let _ = fs::remove_file(routes_undo_path(path));
                            }
                            return stream.write_all(
                                format!("{{\"ok\":false,\"error\":\"{error}\"}}\n").as_bytes(),
                            );
                        }
                        self.route_undo = current_routes;
                    }
                }
            }
            if command == Some(Command::Learn) {
                let value = serde_json::from_slice::<serde_json::Value>(&request_payload)
                    .unwrap_or_default();
                if value.get("action").and_then(serde_json::Value::as_str) == Some("live_test") {
                    if value.as_object().is_none_or(|object| {
                        object.keys().any(|key| {
                            !matches!(
                                key.as_str(),
                                "action"
                                    | "request_id"
                                    | "source_endpoint_id"
                                    | "destination_id"
                                    | "generation"
                            )
                        })
                    }) {
                        return stream.write_all(
                            b"{\"ok\":false,\"error\":\"unknown live-test request field\"}\n",
                        );
                    }
                    let parsed = mackes_ipc::LiveTestRequest {
                        request_id: value
                            .get("request_id")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        source_endpoint_id: value
                            .get("source_endpoint_id")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        destination_id: value
                            .get("destination_id")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        candidate_kind: value
                            .get("candidate_kind")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        candidate_number: value
                            .get("candidate_number")
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|value| u8::try_from(value).ok()),
                        candidate_channel: value
                            .get("candidate_channel")
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|value| u8::try_from(value).ok()),
                        generation: value
                            .get("generation")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or_default(),
                    };
                    let response = match parsed.validate() {
                        Ok(request) => serde_json::json!({
                            "ok": true,
                            "generation": self.generation,
                            "live_test": {
                                "request_id": request.request_id,
                                "status": "unavailable",
                                "reason": "no profile-backed live-test probe is available",
                            }
                        }),
                        Err(error) => serde_json::json!({"ok": false, "error": error}),
                    };
                    return stream.write_all(format!("{response}\n").as_bytes());
                }
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
                let value = serde_json::from_slice::<serde_json::Value>(&request_payload)
                    .unwrap_or_default();
                if let Some(value) = value.get("setlist_create") {
                    let Some(path) = self.config_path.as_deref() else {
                        return stream.write_all(
                            b"{\"ok\":false,\"error\":\"configuration path is unavailable\"}\n",
                        );
                    };
                    return stream.write_all(
                        match scene_runtime::create_setlist(path, value) {
                            Ok(id) => format!(
                                "{{\"ok\":true,\"generation\":{},\"setlist\":{}}}\n",
                                self.generation,
                                serde_json::to_string(&id).unwrap_or_default()
                            )
                            .into_bytes(),
                            Err(error) => format!(
                                "{{\"ok\":false,\"error\":{}}}\n",
                                serde_json::to_string(&error).unwrap_or_default()
                            )
                            .into_bytes(),
                        }
                        .as_slice(),
                    );
                }
                if let Some(id) = value.get("setlist_delete").and_then(serde_json::Value::as_str) {
                    let Some(path) = self.config_path.as_deref() else {
                        return stream.write_all(
                            b"{\"ok\":false,\"error\":\"configuration path is unavailable\"}\n",
                        );
                    };
                    return stream.write_all(
                        match scene_runtime::delete_setlist(path, id) {
                            Ok(id) => format!(
                                "{{\"ok\":true,\"generation\":{},\"setlist\":{}}}\n",
                                self.generation,
                                serde_json::to_string(&id).unwrap_or_default()
                            )
                            .into_bytes(),
                            Err(error) => format!(
                                "{{\"ok\":false,\"error\":{}}}\n",
                                serde_json::to_string(&error).unwrap_or_default()
                            )
                            .into_bytes(),
                        }
                        .as_slice(),
                    );
                }
                if let Some(copy) = value.get("project_copy") {
                    let Some(path) = self.config_path.as_deref() else {
                        return stream.write_all(
                            b"{\"ok\":false,\"error\":\"configuration path is unavailable\"}\n",
                        );
                    };
                    return stream.write_all(
                        match scene_runtime::copy_project(path, copy) {
                            Ok(id) => format!(
                                "{{\"ok\":true,\"generation\":{},\"project\":{}}}\n",
                                self.generation,
                                serde_json::to_string(&id).unwrap_or_default()
                            )
                            .into_bytes(),
                            Err(error) => format!(
                                "{{\"ok\":false,\"error\":{}}}\n",
                                serde_json::to_string(&error).unwrap_or_default()
                            )
                            .into_bytes(),
                        }
                        .as_slice(),
                    );
                }
                if let Some(copy) = value.get("setlist_copy") {
                    let Some(path) = self.config_path.as_deref() else {
                        return stream.write_all(
                            b"{\"ok\":false,\"error\":\"configuration path is unavailable\"}\n",
                        );
                    };
                    return stream.write_all(
                        match scene_runtime::copy_setlist(path, copy) {
                            Ok(id) => format!(
                                "{{\"ok\":true,\"generation\":{},\"setlist\":{}}}\n",
                                self.generation,
                                serde_json::to_string(&id).unwrap_or_default()
                            )
                            .into_bytes(),
                            Err(error) => format!(
                                "{{\"ok\":false,\"error\":{}}}\n",
                                serde_json::to_string(&error).unwrap_or_default()
                            )
                            .into_bytes(),
                        }
                        .as_slice(),
                    );
                }
                if let Some(scene_id) =
                    value.get("preview_scene").and_then(serde_json::Value::as_str)
                {
                    let Some(path) = self.config_path.as_deref() else {
                        return stream.write_all(
                            b"{\"ok\":false,\"error\":\"configuration path is unavailable\"}\n",
                        );
                    };
                    return stream.write_all(
                        match scene_runtime::preview_scene(path, scene_id) {
                            Ok(preview) => format!(
                                "{{\"ok\":true,\"generation\":{},\"preview\":{}}}\n",
                                self.generation, preview
                            )
                            .into_bytes(),
                            Err(error) => format!(
                                "{{\"ok\":false,\"error\":{}}}\n",
                                serde_json::to_string(&error).unwrap_or_default()
                            )
                            .into_bytes(),
                        }
                        .as_slice(),
                    );
                }
                if let Some(setlist_id) =
                    value.get("preview_setlist").and_then(serde_json::Value::as_str)
                {
                    let Some(path) = self.config_path.as_deref() else {
                        return stream.write_all(
                            b"{\"ok\":false,\"error\":\"configuration path is unavailable\"}\n",
                        );
                    };
                    return stream.write_all(
                        match scene_runtime::preview_setlist(path, setlist_id) {
                            Ok(preview) => format!(
                                "{{\"ok\":true,\"generation\":{},\"preview\":{}}}\n",
                                self.generation, preview
                            )
                            .into_bytes(),
                            Err(error) => format!(
                                "{{\"ok\":false,\"error\":{}}}\n",
                                serde_json::to_string(&error).unwrap_or_default()
                            )
                            .into_bytes(),
                        }
                        .as_slice(),
                    );
                }
                if let Some(scene) = value.get("execute_scene").and_then(serde_json::Value::as_str)
                {
                    if let Err(error) = self.select_scene(scene) {
                        return stream.write_all(
                            format!("{{\"ok\":false,\"error\":\"{error}\"}}\n").as_bytes(),
                        );
                    }
                    return stream.write_all(serde_json::json!({"ok": true, "generation": self.generation, "executed_scene": scene, "activation_result": self.activation_result, "activation_outcomes": self.activation_outcomes}).to_string().as_bytes()).and_then(|()| stream.write_all(b"\n"));
                }
                if let Some(setlist_value) = value.get("setlist") {
                    let Some(path) = self.config_path.as_deref() else {
                        return stream.write_all(
                            b"{\"ok\":false,\"error\":\"configuration path is unavailable\"}\n",
                        );
                    };
                    return stream.write_all(
                        match scene_runtime::replace_setlist(path, setlist_value) {
                            Ok(setlist_id) => format!(
                                "{{\"ok\":true,\"generation\":{},\"setlist\":{}}}\n",
                                self.generation,
                                serde_json::to_string(&setlist_id).unwrap_or_default()
                            )
                            .into_bytes(),
                            Err(error) => format!(
                                "{{\"ok\":false,\"error\":{}}}\n",
                                serde_json::to_string(&error).unwrap_or_default()
                            )
                            .into_bytes(),
                        }
                        .as_slice(),
                    );
                }
                if let Some(project_value) = value.get("project") {
                    let Some(path) = self.config_path.as_deref() else {
                        return stream.write_all(
                            b"{\"ok\":false,\"error\":\"configuration path is unavailable\"}\n",
                        );
                    };
                    return stream.write_all(
                        match scene_runtime::replace_project(path, project_value) {
                            Ok(project_id) => format!(
                                "{{\"ok\":true,\"generation\":{},\"project\":{}}}\n",
                                self.generation,
                                serde_json::to_string(&project_id).unwrap_or_default()
                            )
                            .into_bytes(),
                            Err(error) => format!(
                                "{{\"ok\":false,\"error\":{}}}\n",
                                serde_json::to_string(&error).unwrap_or_default()
                            )
                            .into_bytes(),
                        }
                        .as_slice(),
                    );
                }
                if let (Some(scene_id), Some(actions_value)) =
                    (value.get("scene").and_then(serde_json::Value::as_str), value.get("actions"))
                {
                    let Some(path) = self.config_path.as_deref() else {
                        return stream.write_all(
                            b"{\"ok\":false,\"error\":\"configuration path is unavailable\"}\n",
                        );
                    };
                    let result = scene_runtime::replace_actions(path, scene_id, actions_value);
                    return stream.write_all(
                        match result {
                            Ok(()) => format!(
                                "{{\"ok\":true,\"generation\":{},\"scene\":\"{}\"}}\n",
                                self.generation, scene_id
                            )
                            .into_bytes(),
                            Err(error) => format!(
                                "{{\"ok\":false,\"error\":\"{}\"}}\n",
                                error.replace('"', "'")
                            )
                            .into_bytes(),
                        }
                        .as_slice(),
                    );
                }
                if let Some(scene) = value.get("scene").and_then(serde_json::Value::as_str) {
                    if let Err(error) = self.select_scene_only(scene) {
                        return stream.write_all(
                            format!("{{\"ok\":false,\"error\":\"{error}\"}}\n").as_bytes(),
                        );
                    }
                } else {
                    let next = value.get("direction").and_then(serde_json::Value::as_str)
                        != Some("previous");
                    if let Err(error) = self.try_navigate_scene(next) {
                        return stream.write_all(
                            format!("{{\"ok\":false,\"error\":\"{error}\"}}\n").as_bytes(),
                        );
                    }
                }
            }
            if command == Some(Command::UnsafeMode) {
                let value = serde_json::from_slice::<serde_json::Value>(&request_payload)
                    .unwrap_or_default();
                if value.get("confirm").and_then(serde_json::Value::as_bool) != Some(true) {
                    return stream.write_all(
                        b"{\"ok\":false,\"error\":\"unsafe mode requires confirmation\"}\n",
                    );
                }
                self.arm_experimental_mappings();
                return stream.write_all(
                    format!(
                        "{{\"ok\":true,\"generation\":{},\"unsafe_mode\":\"armed\",\"window_seconds\":900}}\n",
                        self.generation
                    )
                    .as_bytes(),
                );
            }
            if command == Some(Command::DeviceQuery) {
                let value = serde_json::from_slice::<serde_json::Value>(&request_payload)
                    .unwrap_or_default();
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
                let value = serde_json::from_slice::<serde_json::Value>(&request_payload)
                    .unwrap_or_default();
                return match self.apply_device_control(&value) {
                    Ok(payload) => stream.write_all(
                        format!(
                            "{{\"ok\":true,\"generation\":{},\"bytes\":{}}}\n",
                            self.generation,
                            serde_json::to_string(&payload).unwrap_or_else(|_| "[]".into())
                        )
                        .as_bytes(),
                    ),
                    Err(error) => stream
                        .write_all(format!("{{\"ok\":false,\"error\":\"{error}\"}}\n").as_bytes()),
                };
            }
            if command == Some(Command::Sysex) {
                let value = serde_json::from_slice::<serde_json::Value>(&request_payload)
                    .unwrap_or_default();
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
                self.record_physical_send(destination, "sysex");
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
                    Command::Hello
                        | Command::Health
                        | Command::Snapshot
                        | Command::NovationSnapshot
                        | Command::Subscribe
                ) {
                    if command == Command::Panic {
                        let _ = self.send_panic_controls();
                    }
                    self.record_state_event(command);
                }
            }
            match command {
                Some(Command::Snapshot) => self.snapshot_response(),
                Some(Command::NovationSnapshot) => self.novation_snapshot_response(),
                Some(Command::Rescan) => {
                    if request_payload
                        .get(..)
                        .and_then(|payload| {
                            serde_json::from_slice::<serde_json::Value>(payload).ok()
                        })
                        .and_then(|value| {
                            value
                                .get("action")
                                .and_then(serde_json::Value::as_str)
                                .map(str::to_owned)
                        })
                        .as_deref()
                        .is_some_and(|action| action == "rebind" || action == "rebind_undo")
                    {
                        let value = serde_json::from_slice::<serde_json::Value>(&request_payload)
                            .unwrap_or_default();
                        if value.get("action").and_then(serde_json::Value::as_str)
                            == Some("rebind_undo")
                        {
                            let Some(previous) = self.novation_rebind_previous.take() else {
                                return stream.write_all(format!("{{\"ok\":false,\"generation\":{},\"error\":\"no rebind undo is available\"}}\n", self.generation).as_bytes());
                            };
                            let Some(path) = self.config_path.as_deref() else {
                                return stream.write_all(format!("{{\"ok\":false,\"generation\":{},\"error\":\"rebind undo requires a persisted configuration\"}}\n", self.generation).as_bytes());
                            };
                            let result = mackes_config::load(path)
                                .map_err(|error| error.to_string())
                                .and_then(|mut document| {
                                    document
                                        .settings
                                        .novation_device
                                        .clone_from(&Some(previous.clone()));
                                    mackes_config::validate(&document)?;
                                    mackes_config::save(path, &document, 1)
                                        .map_err(|error| error.to_string())
                                });
                            return match result {
                                Ok(()) => {
                                    self.novation_device_policy = Some(previous);
                                    self.native_rescan_at = None;
                                    stream.write_all(format!("{{\"ok\":true,\"generation\":{},\"rebind_undo\":\"applied\"}}\n", self.generation).as_bytes())
                                }
                                Err(error) => stream.write_all(
                                    format!(
                                        "{{\"ok\":false,\"generation\":{},\"error\":{}}}\n",
                                        self.generation,
                                        serde_json::to_string(&error)
                                            .unwrap_or_else(|_| "\"rebind undo failed\"".into())
                                    )
                                    .as_bytes(),
                                ),
                            };
                        }
                        let stable_id = value
                            .get("stable_id")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("");
                        let template = value
                            .get("template")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or_else(|| {
                                u64::from(mackes_profiles::LAUNCH_CONTROL_MK2_FACTORY1_SLOT)
                            });
                        if stable_id.trim().is_empty()
                            || stable_id.len() > 128
                            || stable_id != stable_id.trim()
                            || template >= 16
                        {
                            return stream.write_all(format!("{{\"ok\":false,\"generation\":{},\"error\":\"invalid rebind identity or template\"}}\n", self.generation).as_bytes());
                        }
                        let Ok(template) = u8::try_from(template) else {
                            return stream.write_all(format!("{{\"ok\":false,\"generation\":{},\"error\":\"invalid template\"}}\n", self.generation).as_bytes());
                        };
                        let Some(path) = self.config_path.as_deref() else {
                            return stream.write_all(format!("{{\"ok\":false,\"generation\":{},\"error\":\"rebind requires a persisted configuration\"}}\n", self.generation).as_bytes());
                        };
                        match mackes_config::load(path).map_err(|error| error.to_string()).and_then(
                            |mut document| {
                                if let Some(previous) = document.settings.novation_device.clone() {
                                    self.novation_rebind_previous = Some(previous);
                                }
                                document.settings.novation_device =
                                    Some(mackes_config::NovationDeviceConfig {
                                        version: 1,
                                        stable_id: Some(stable_id.to_owned()),
                                        template,
                                        auto_reapply: true,
                                        feedback_enabled: true,
                                    });
                                mackes_config::validate(&document)?;
                                mackes_config::save(path, &document, 1)
                                    .map_err(|error| error.to_string())
                            },
                        ) {
                            Ok(()) => {
                                self.novation_device_policy =
                                    Some(mackes_config::NovationDeviceConfig {
                                        version: 1,
                                        stable_id: Some(stable_id.to_owned()),
                                        template,
                                        auto_reapply: true,
                                        feedback_enabled: true,
                                    });
                                self.native_rescan_at = None;
                                return stream.write_all(format!("{{\"ok\":true,\"generation\":{},\"rebind\":\"applied\"}}\n", self.generation).as_bytes());
                            }
                            Err(error) => {
                                return stream.write_all(
                                    format!(
                                        "{{\"ok\":false,\"generation\":{},\"error\":{}}}\n",
                                        self.generation,
                                        serde_json::to_string(&error)
                                            .unwrap_or_else(|_| "\"rebind failed\"".into())
                                    )
                                    .as_bytes(),
                                )
                            }
                        }
                    }
                    self.native_rescan_at = None;
                    format!(
                        "{{\"ok\":true,\"generation\":{},\"rescan\":\"scheduled\"}}\n",
                        self.generation
                    )
                }
                Some(Command::Subscribe) => self.subscribe_response(&request),
                Some(Command::Scenes) => self.scenes_response(),
                Some(Command::Backups) => persistence_projection::backup_response(
                    self.generation,
                    self.config_path.as_deref(),
                    Some(&request_payload),
                ),
                Some(Command::PiPedal) => {
                    let parsed =
                        serde_json::from_slice::<mackes_ipc::PiPedalRequest>(&request_payload).ok();
                    match parsed {
                        Some(request)
                            if matches!(
                                request.operation,
                                mackes_ipc::PiPedalOperation::Snapshot
                            ) =>
                        {
                            serde_json::json!({
                                "ok": true,
                                "generation": self.generation,
                            "pipedal": self.pipedal_worker.ipc_status(),
                                "catalog": self.pipedal_worker.catalog(),
                                "generation": self.pipedal_worker.health().generation,
                                "favorites": self.pipedal_worker.favorites(),
                                "system_midi_bindings": self.pipedal_worker.system_midi_bindings(),
                                "version": self.pipedal_worker.version(),
                                "governor_settings": self.pipedal_worker.governor_settings(),
                                "show_status_monitor": self.pipedal_worker.show_status_monitor(),
                                "wifi_regulatory_domains": self.pipedal_worker.wifi_regulatory_domains(),
                                "supported_operations": mackes_pipedal_adapter::supported_operations(),
                                "mapping_resolution": self.pipedal_mapping_resolution(),
                            })
                            .to_string()
                                + "\n"
                        }
                        Some(request)
                            if matches!(request.operation, mackes_ipc::PiPedalOperation::Repair) =>
                        {
                            if !request.confirm {
                                return stream.write_all(b"{\"ok\":false,\"error\":\"PiPedal repair requires confirmation\"}\n");
                            }
                            if request.generation != self.pipedal_worker.health().generation {
                                return stream.write_all(serde_json::json!({"ok": false, "error": "PiPedal generation conflict", "generation": self.pipedal_worker.health().generation}).to_string().as_bytes()).and_then(|()| stream.write_all(b"\n"));
                            }
                            let (Some(control_id), Some(target)) =
                                (request.physical_control_id, request.mapping)
                            else {
                                return stream.write_all(b"{\"ok\":false,\"error\":\"PiPedal repair requires physical_control_id and mapping\"}\n");
                            };
                            if control_id.len() > 128 || target.plugin_uri.len() > 512 || target.symbol.len() > 256 || target.scope.as_ref().is_some_and(|scope| scope.len() > 256) {
                                return stream.write_all(b"{\"ok\":false,\"error\":\"PiPedal repair fields are too long\"}\n");
                            }
                            let Some(path) = self.config_path.clone() else {
                                return stream.write_all(b"{\"ok\":false,\"error\":\"configuration path is unavailable\"}\n");
                            };
                            let Ok(mut document) = mackes_config::load(&path) else {
                                return stream.write_all(b"{\"ok\":false,\"error\":\"configuration load failed\"}\n");
                            };
                            let Some(existing) = document.settings.pipedal_mappings.mappings.iter_mut().find(|mapping| mapping.physical_control_id == control_id) else {
                                return stream.write_all(b"{\"ok\":false,\"error\":\"persisted PiPedal mapping was not found\"}\n");
                            };
                            existing.plugin_uri = target.plugin_uri;
                            existing.symbol = target.symbol;
                            existing.scope = target.scope;
                            let expected_revision = mackes_config::document_revision(&document).ok();
                            let Some(expected_revision) = expected_revision else {
                                return stream.write_all(b"{\"ok\":false,\"error\":\"configuration revision unavailable\"}\n");
                            };
                            if let Err(error) = mackes_config::save_if_revision(&path, &document, 5, &expected_revision) {
                                return stream.write_all(serde_json::json!({"ok": false, "error": error.to_string()}).to_string().as_bytes()).and_then(|()| stream.write_all(b"\n"));
                            }
                            self.cache_pipedal_mappings(&document);
                            serde_json::json!({"ok": true, "repaired": true, "generation": self.pipedal_worker.health().generation, "physical_control_id": control_id}).to_string() + "\n"
                        }
                        Some(request)
                            if matches!(request.operation, mackes_ipc::PiPedalOperation::Apply) =>
                        {
                            if let Some(body) = pipedal_dispatch::apply_readback_or_preset(&mut self.pipedal_worker, &request) {
                                return stream.write_all(body.to_string().as_bytes()).and_then(|()| stream.write_all(b"\n"));
                            }
                            if let (Some(instance_id), Some(enabled)) = (request.instance_id, request.enabled) {
                                if !request.confirm { return stream.write_all(b"{\"ok\":false,\"error\":\"PiPedal bypass requires confirmation\"}\n"); }
                                let Some(client_id) = self.pipedal_worker.pipedal_client_id() else { return stream.write_all(b"{\"ok\":false,\"error\":\"PiPedal client identity is unavailable\"}\n"); };
                                let reply_to = self.pipedal_worker.allocate_reply_id();
                                let result = self.pipedal_worker.apply_set_pedalboard_item_enable(request.generation, client_id, instance_id, enabled, Some(reply_to), true);
                                let body = match result { Ok(()) => serde_json::json!({"ok": true, "applied": true, "bypass": !enabled, "generation": request.generation}), Err(error) => serde_json::json!({"ok": false, "error": error}) };
                                return stream.write_all(body.to_string().as_bytes()).and_then(|()| stream.write_all(b"\n"));
                            }
                            if let (Some(instance_id), Some(use_mod_ui)) = (request.instance_id, request.use_mod_ui) {
                                let Some(client_id) = self.pipedal_worker.pipedal_client_id() else { return stream.write_all(b"{\"ok\":false,\"error\":\"PiPedal client identity is unavailable\"}\n"); };
                                let reply_to = self.pipedal_worker.allocate_reply_id();
                                let result = self.pipedal_worker.apply_set_pedalboard_item_use_mod_ui(request.generation, client_id, instance_id, use_mod_ui, Some(reply_to), request.confirm);
                                let body = match result { Ok(()) => serde_json::json!({"ok": true, "applied": true, "use_mod_ui": use_mod_ui, "generation": request.generation}), Err(error) => serde_json::json!({"ok": false, "error": error}) };
                                return stream.write_all(body.to_string().as_bytes()).and_then(|()| stream.write_all(b"\n"));
                            }
                            if let (Some(instance_id), Some(title), Some(color_key)) = (request.instance_id, request.title, request.color_key) {
                                let reply_to = self.pipedal_worker.allocate_reply_id();
                                let result = self.pipedal_worker.apply_set_pedalboard_item_title(request.generation, instance_id, title, color_key, Some(reply_to), request.confirm);
                                let body = match result { Ok(()) => serde_json::json!({"ok": true, "applied": true, "title": true, "generation": request.generation}), Err(error) => serde_json::json!({"ok": false, "error": error}) };
                                return stream.write_all(body.to_string().as_bytes()).and_then(|()| stream.write_all(b"\n"));
                            }
                            if let (Some(volume_db), Some(input)) = (request.volume_db, request.preview_input) {
                                let reply_to = self.pipedal_worker.allocate_reply_id();
                                let result = self.pipedal_worker.apply_preview_volume(request.generation, input, volume_db, Some(reply_to));
                                let body = match result { Ok(()) => serde_json::json!({"ok": true, "previewed": true, "input": input, "volume_db": volume_db, "generation": request.generation}), Err(error) => serde_json::json!({"ok": false, "error": error}) };
                                return stream.write_all(body.to_string().as_bytes()).and_then(|()| stream.write_all(b"\n"));
                            }
                            if let (Some(handle), Some(cancel)) = (request.midi_listener_handle, request.cancel_midi_listener) {
                                let reply_to = self.pipedal_worker.allocate_reply_id();
                                let result = if cancel { self.pipedal_worker.apply_cancel_listen_for_midi_event(request.generation, handle, Some(reply_to)) } else { self.pipedal_worker.apply_listen_for_midi_event(request.generation, handle, Some(reply_to)) };
                                let body = match result { Ok(()) => serde_json::json!({"ok": true, "listening": !cancel, "cancelled": cancel, "handle": handle, "generation": request.generation}), Err(error) => serde_json::json!({"ok": false, "error": error}) };
                                return stream.write_all(body.to_string().as_bytes()).and_then(|()| stream.write_all(b"\n"));
                            }
                            if let (Some(handle), Some(cancel)) = (request.monitor_client_handle, request.cancel_patch_monitor) {
                                let reply_to = self.pipedal_worker.allocate_reply_id();
                                let result = if cancel {
                                    self.pipedal_worker.apply_cancel_monitor_patch_property(request.generation, handle, Some(reply_to))
                                } else {
                                    let Some(instance_id) = request.instance_id else { return stream.write_all(b"{\"ok\":false,\"error\":\"PiPedal patch monitor instance is required\"}\n"); };
                                    let Some(property_uri) = request.property_uri else { return stream.write_all(b"{\"ok\":false,\"error\":\"PiPedal patch monitor property is required\"}\n"); };
                                    self.pipedal_worker.apply_monitor_patch_property(request.generation, instance_id, handle, property_uri, Some(reply_to))
                                };
                                let body = match result { Ok(()) => serde_json::json!({"ok": true, "monitoring": !cancel, "cancelled": cancel, "handle": handle, "generation": request.generation}), Err(error) => serde_json::json!({"ok": false, "error": error}) };
                                return stream.write_all(body.to_string().as_bytes()).and_then(|()| stream.write_all(b"\n"));
                            }
                            let target = match (request.mapping, request.physical_control_id) {
                                (Some(target), None) => target,
                                (None, Some(control_id)) => {
                                    let matches = self.pipedal_mappings.iter().filter(|mapping| mapping.physical_control_id == control_id).collect::<Vec<_>>();
                                    if matches.len() != 1 { let error = if matches.is_empty() { "Persisted PiPedal mapping was not found" } else { "Persisted PiPedal mapping is ambiguous" }; return stream.write_all(serde_json::json!({"ok": false, "error": error}).to_string().as_bytes()).and_then(|()| stream.write_all(b"\n")); }
                                    mackes_ipc::PiPedalMappingTarget { physical_control_id: matches[0].physical_control_id.clone(), plugin_uri: matches[0].plugin_uri.clone(), symbol: matches[0].symbol.clone(), scope: matches[0].scope.clone() }
                                }
                                (Some(_), Some(_)) => return stream.write_all(b"{\"ok\":false,\"error\":\"Choose either mapping or physical_control_id\"}\n"),
                                (None, None) => return stream.write_all(b"{\"ok\":false,\"error\":\"PiPedal mapping or physical_control_id is required\"}\n"),
                            }; let (Some(instance_id), Some(value), Some(client_id)) = (
                                request.instance_id,
                                request.value,
                                self.pipedal_worker.pipedal_client_id(),
                            ) else {
                                return stream.write_all(b"{\"ok\":false,\"error\":\"PiPedal apply target fields are required\"}\n");
                            }; let mapping = mackes_pipedal_adapter::MappingIdentity {
                                physical_control_id: target.physical_control_id,
                                plugin_uri: target.plugin_uri,
                                symbol: target.symbol,
                                scope: target.scope,
                            };
                            let reply_to = self.pipedal_worker.allocate_reply_id();
                            match self.pipedal_worker.apply_set_control_with_record(
                                request.generation,
                                &mapping,
                                instance_id,
                                client_id,
                                Some(reply_to),
                                value,
                                request.confirm,
                            ) {
                                Ok(()) => {
                                    if let Some(path) = self.config_path.as_deref() {
                                        if let Err(error) = persistence_projection::persist_pipedal_undo(path, self.pipedal_worker.apply_record()) {
                                            return stream.write_all(serde_json::json!({"ok": false, "error": format!("PiPedal undo journal persistence failed: {error}")}).to_string().as_bytes()).and_then(|()| stream.write_all(b"\n"));
                                        }
                                    }
                                    serde_json::json!({"ok": true, "applied": true, "generation": request.generation}).to_string() + "\n"
                                }
                                Err(error) => {
                                    serde_json::json!({"ok": false, "error": error}).to_string()
                                        + "\n"
                                }
                            }
                        }
                        Some(request)
                            if matches!(request.operation, mackes_ipc::PiPedalOperation::Undo) =>
                        {
                            let Some(client_id) = self.pipedal_worker.pipedal_client_id() else {
                                return stream.write_all(b"{\"ok\":false,\"error\":\"PiPedal client identity is unavailable\"}\n");
                            };
                            let reply_to = self.pipedal_worker.allocate_reply_id();
                            match self.pipedal_worker.restore_last_apply(
                                request.generation,
                                client_id,
                                Some(reply_to),
                                request.confirm,
                            ) {
                                Ok(()) => {
                                    if let Some(path) = self.config_path.as_deref() {
                                        if let Err(error) = persistence_projection::persist_pipedal_undo(path, None) {
                                            return stream.write_all(serde_json::json!({"ok": false, "error": format!("PiPedal undo journal cleanup failed: {error}")}).to_string().as_bytes()).and_then(|()| stream.write_all(b"\n"));
                                        }
                                    }
                                    serde_json::json!({"ok": true, "undo": true, "generation": request.generation}).to_string() + "\n"
                                }
                                Err(error) => {
                                    serde_json::json!({"ok": false, "error": error}).to_string()
                                        + "\n"
                                }
                            }
                        }
                        _ => {
                            "{\"ok\":false,\"error\":\"unsupported PiPedal operation\",\"code\":\"unsupported_operation\",\"work_item\":\"W113\"}\n".to_owned()
                        }
                    }
                }
                Some(Command::Assignment) => {
                    let parsed =
                        serde_json::from_slice::<mackes_ipc::AssignmentRequest>(&request_payload)
                            .ok()
                            .or_else(|| {
                                let mut value =
                                    serde_json::from_slice::<serde_json::Value>(&request).ok()?;
                                value.as_object_mut()?.remove("command");
                                serde_json::from_value(value).ok()
                            })
                            .and_then(|request| request.validate().ok());
                    let result = match parsed {
                        None => mackes_ipc::AssignmentResult {
                            generation: self.assignment_generation,
                            session: self.assignment_session.clone(),
                            applied: false,
                            reason: Some("invalid assignment request".into()),
                        },
                        Some(request) => self.apply_assignment_request(request),
                    };
                    serde_json::to_string(&result).map_or_else(
                        |_| {
                            "{\"ok\":false,\"error\":\"assignment result encoding failed\"}\n"
                                .to_owned()
                        },
                        |body| format!("{body}\n"),
                    )
                }
                Some(Command::Mappings) => {
                    let parsed =
                        serde_json::from_slice::<mackes_ipc::MappingRequest>(&request_payload)
                            .ok()
                            .or_else(|| {
                                let mut value =
                                    serde_json::from_slice::<serde_json::Value>(&request).ok()?;
                                value.as_object_mut()?.remove("command");
                                serde_json::from_value(value).ok()
                            })
                            .and_then(|mapping| mapping.validate().ok());
                    let mut outcome = mackes_ipc::MappingOutcome::Invalid;
                    if let Some(mapping) = parsed {
                        if matches!(mapping.operation, mackes_ipc::MappingOperation::Snapshot) {
                            outcome = mackes_ipc::MappingOutcome::Applied;
                        } else {
                            let mut candidate = self.mapping_store.clone();
                            let mutation = match (mapping.operation, mapping.payload) {
                                (
                                    mackes_ipc::MappingOperation::Draft,
                                    Some(mackes_ipc::MappingPayload::Draft { draft }),
                                ) => candidate
                                    .save_draft(mapping.generation, draft)
                                    .map_err(str::to_owned),
                                (
                                    mackes_ipc::MappingOperation::Activate,
                                    Some(mackes_ipc::MappingPayload::Mapping { mapping: record }),
                                ) => candidate
                                    .activate(mapping.generation, record)
                                    .map_err(str::to_owned),
                                (
                                    mackes_ipc::MappingOperation::Activate,
                                    Some(mackes_ipc::MappingPayload::Batch { mappings }),
                                ) => candidate
                                    .activate_batch(mapping.generation, mappings)
                                    .map_err(str::to_owned),
                                (
                                    mackes_ipc::MappingOperation::Replace,
                                    Some(mackes_ipc::MappingPayload::Mapping { mapping: record }),
                                ) => candidate.replace_with_runtime(
                                    mapping.generation,
                                    record,
                                    |replacement| {
                                        if mackes_profiles::builtin_profile(
                                            &replacement.destination_profile,
                                        )
                                        .is_none()
                                        {
                                            return Err("destination profile is unavailable");
                                        }
                                        if mackes_midi_engine::numeric_endpoint_id(
                                            &replacement.destination_endpoint,
                                        )
                                        .is_none()
                                        {
                                            return Err("destination endpoint is invalid");
                                        }
                                        Ok(())
                                    },
                                ),
                                (
                                    mackes_ipc::MappingOperation::Behavior,
                                    Some(mackes_ipc::MappingPayload::Behavior {
                                        mapping_id,
                                        behavior,
                                    }),
                                ) => candidate
                                    .update_behavior(mapping.generation, &mapping_id, behavior)
                                    .map_err(str::to_owned),
                                (
                                    mackes_ipc::MappingOperation::Enabled,
                                    Some(mackes_ipc::MappingPayload::Enabled {
                                        mapping_id,
                                        enabled,
                                    }),
                                ) => candidate
                                    .set_enabled(mapping.generation, &mapping_id, enabled)
                                    .map_err(str::to_owned),
                                (
                                    mackes_ipc::MappingOperation::Delete,
                                    Some(mackes_ipc::MappingPayload::Delete { mapping_id }),
                                ) => candidate
                                    .delete(mapping.generation, &mapping_id)
                                    .map_err(str::to_owned),
                                (
                                    mackes_ipc::MappingOperation::SelectLayer,
                                    Some(mackes_ipc::MappingPayload::Layer { active_layer }),
                                ) => {
                                    match mapping_layers_runtime::select_layer(
                                        self.mapping_layers_v2.as_ref(),
                                        mapping.generation,
                                        candidate.generation,
                                        active_layer,
                                    ) {
                                        Ok((next, generation)) => {
                                            self.mapping_layers_v2 = Some(next);
                                            candidate.generation = generation;
                                            Ok(())
                                        }
                                        Err(error) => Err(error),
                                    }
                                }
                                (mackes_ipc::MappingOperation::Undo, None) => {
                                    candidate.undo(mapping.generation).map_err(str::to_owned)
                                }
                                _ => Err("mapping operation payload is invalid".to_owned()),
                            };
                            outcome = match mutation {
                                Ok(()) => {
                                    let persisted = mapping_layers_runtime::persist(
                                        self.config_path.as_deref(),
                                        mapping.operation,
                                        self.mapping_layers_v2.as_ref(),
                                        &candidate,
                                    );
                                    if persisted {
                                        self.mapping_store = candidate;
                                        mackes_ipc::MappingOutcome::Applied
                                    } else {
                                        mackes_ipc::MappingOutcome::PersistenceFailed
                                    }
                                }
                                Err(error) if error.contains("generation") => {
                                    mackes_ipc::MappingOutcome::GenerationConflict
                                }
                                Err(error) if error.contains("occupied") => {
                                    mackes_ipc::MappingOutcome::Conflict
                                }
                                Err(_) => mackes_ipc::MappingOutcome::Invalid,
                            };
                        }
                    }
                    let generation = self.mapping_store.generation;
                    let (active, drafts) = self.mapping_store.snapshot();
                    serde_json::to_string(&mackes_ipc::MappingResult {
                        generation,
                        undo_available: self.mapping_store.undo_available(),
                        active: Some(active.to_vec()),
                        draft: Some(drafts.to_vec()),
                        mapping_layers_v2: self.mapping_layers_v2.clone(),
                        outcome,
                    })
                    .map_or_else(
                        |_| {
                            "{\"ok\":false,\"error\":\"mapping result encoding failed\"}\n"
                                .to_owned()
                        },
                        |body| format!("{body}\n"),
                    )
                }
                Some(command) => {
                    if command == Command::Monitor {
                        return stream.write_all(self.monitor_response().as_bytes());
                    }
                    if command == Command::Configuration {
                        let persisted_revision = self
                            .config_path
                            .as_deref()
                            .and_then(|path| mackes_config::load(path).ok())
                            .and_then(|document| mackes_config::document_revision(&document).ok());
                        let catalog = configuration_response::with_runtime_status(
                            self.catalog.clone(),
                            persisted_revision.as_deref(),
                        );
                        return configuration_response::write(
                            &mut stream,
                            &catalog,
                            self.generation,
                        );
                    }
                    let endpoints = if matches!(
                        command,
                        Command::DeviceQuery | Command::Endpoints | Command::Routes
                    ) {
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
    /// Services the bounded `PiPedal` worker without running network I/O on MIDI dispatch.
    pub fn poll_pipedal(&mut self) {
        let now = Instant::now();
        if self.pipedal_transport.is_none() {
            if now < self.pipedal_retry_at {
                return;
            }
            if let Ok(transport) = mackes_pipedal_adapter::WebSocketTransport::connect(
                mackes_pipedal_adapter::default_endpoint(),
            ) {
                self.pipedal_transport = Some(transport);
                let _ = self.pipedal_worker.enqueue(mackes_pipedal_adapter::Command::Start);
            } else {
                self.pipedal_retry_at = now + Duration::from_secs(10);
                return;
            }
        }
        let result = if let Some(transport) = self.pipedal_transport.as_mut() {
            self.pipedal_worker.process(transport, 1);
            self.pipedal_worker.pump(transport, 8, 8)
        } else {
            return;
        };
        if let Ok(frames) = result {
            for frame in frames {
                if self.pipedal_worker.accept_frame(&frame).is_err() {
                    self.pipedal_transport = None;
                    self.pipedal_retry_at = Instant::now() + Duration::from_secs(10);
                    break;
                }
                self.pipedal_worker.reconcile_pickup_targets(&self.pipedal_mappings);
            }
        } else {
            self.pipedal_transport = None;
            self.pipedal_retry_at = Instant::now() + Duration::from_secs(10);
        }
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
        let discovered = physical_devices_value(endpoints);
        let Some(current) = self.physical_devices.as_array() else {
            self.physical_devices = discovered;
            return;
        };
        let Some(next) = discovered.as_array() else {
            return;
        };
        // Retain the last bounded record for a disconnected device so mappings
        // keep their stable identity across an ALSA refresh. Reconnected
        // devices replace the retained record with fresh endpoint state.
        let mut merged = next.clone();
        for previous in current {
            let Some(id) = previous.get("id").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if merged
                .iter()
                .any(|device| device.get("id").and_then(serde_json::Value::as_str) == Some(id))
            {
                continue;
            }
            let mut offline = previous.clone();
            if let Some(object) = offline.as_object_mut() {
                object.insert("state".to_owned(), serde_json::Value::String("offline".to_owned()));
            }
            merged.push(offline);
        }
        merged.sort_by(|left, right| {
            let left_offline =
                left.get("state").and_then(serde_json::Value::as_str) == Some("offline");
            let right_offline =
                right.get("state").and_then(serde_json::Value::as_str) == Some("offline");
            left_offline.cmp(&right_offline).then_with(|| {
                left.get("id")
                    .and_then(serde_json::Value::as_str)
                    .cmp(&right.get("id").and_then(serde_json::Value::as_str))
            })
        });
        merged.truncate(MAX_PHYSICAL_DEVICE_RECORDS);
        self.physical_devices = serde_json::Value::Array(merged);
    }
    /// Installs validated profile-to-output bindings from persistent configuration.
    ///
    /// # Errors
    /// Returns an error for an unknown profile, unavailable endpoint, or duplicate profile.
    pub fn set_profile_bindings(&mut self, bindings: Vec<(String, String)>) -> Result<(), String> {
        for (index, (profile, endpoint)) in bindings.iter().enumerate() {
            if mackes_profiles::builtin_profile(profile).is_none()
                || !self.outputs.output_infos().iter().any(|output| output.id == *endpoint)
                || bindings[..index].iter().any(|(prior, _)| prior == profile)
            {
                return Err("profile output binding is invalid, unavailable, or duplicated".into());
            }
        }
        if self.profile_bindings != bindings {
            self.binding_generation = self.binding_generation.saturating_add(1);
        }
        self.profile_bindings = bindings;
        self.sync_assignment_catalog();
        self.request_lexicon_active_setup();
        Ok(())
    }
    /// Sets the daemon-owned configuration path for authorized persistence.
    pub fn set_config_path(&mut self, path: impl Into<std::path::PathBuf>) {
        let path = path.into();
        self.operation_journal =
            match mackes_config::OperationJournal::open(path.with_extension("operations.json")) {
                Ok(journal) => Some(journal),
                Err(error) => {
                    eprint!(
                        "{}",
                        structured_log_line("error", "operation_journal_open_failed", &error)
                    );
                    None
                }
            };
        if let Err(error) =
            persistence_projection::recover_json_pair(&path.with_extension("routes.commit.json"))
        {
            eprint!(
                "{}",
                structured_log_line("error", "route_pair_recovery_failed", &error.to_string())
            );
        }
        match mackes_config::load(&path) {
            Ok(document) => {
                self.novation_device_policy.clone_from(&document.settings.novation_device);
                self.cache_pipedal_mappings(&document);
                match mackes_config::ControlMappingStore::from_document(&document) {
                    Ok(store) => {
                        self.mapping_store = store;
                        self.mapping_store
                            .active
                            .retain(assignment_commit::mapping_role_compatible);
                        self.assignment_session.has_draft = !self.mapping_store.drafts.is_empty();
                    }
                    Err(error) => eprint!(
                        "{}",
                        structured_log_line("error", "control_mapping_restore_rejected", &error)
                    ),
                }
            }
            Err(error) => eprint!(
                "{}",
                structured_log_line("error", "control_mapping_restore_failed", &error.to_string())
            ),
        }
        if let Ok(bytes) = std::fs::read(routes_undo_path(&path)) {
            self.route_undo = serde_json::from_slice(&bytes).ok();
        }
        if let Ok(bytes) = std::fs::read(persistence_projection::pipedal_undo_path(&path)) {
            match serde_json::from_slice::<mackes_pipedal_adapter::ApplyRecord>(&bytes)
                .ok()
                .and_then(|record| self.pipedal_worker.restore_apply_record(record).ok())
            {
                Some(()) => {}
                None => {
                    let _ = persistence_projection::persist_pipedal_undo(&path, None);
                }
            }
        }
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
        if !self.outputs.is_empty()
            && self.novation_device_policy.as_ref().is_none_or(|policy| policy.auto_reapply)
        {
            self.replay_controller_leds();
        }
    }
    fn sync_assignment_catalog(&mut self) {
        let profiles =
            profile_bindings::catalog_ids(&self.physical_devices, &self.profile_bindings);
        assignment_catalog::refresh(&mut self.assignment_session, &profiles);
        if self.assignment_session.catalog.source_endpoint.is_none() {
            self.assignment_session.catalog.source_endpoint = Some("launch-control-xl-mk2".into());
        }
        let profile =
            self.assignment_session.catalog.selected_device.as_deref().unwrap_or("lexicon.reflex");
        self.assignment_session.catalog.destination_endpoint =
            profile_bindings::stable_destination(&self.outputs, &self.profile_bindings, profile)
                .or_else(|| Some(profile.to_owned()));
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
mod startup_restore;
pub use startup_restore::startup_restore;
#[cfg(target_os = "linux")]
pub use startup_restore::{compile_scene_actions, persist_active_scene};
mod assignment_catalog;
mod assignment_commit;
mod binding_generation;
mod led_surface;
#[cfg(all(test, target_os = "linux"))]
mod native_cutover;
mod novation_snapshot;
mod novation_snapshot_response;
mod persistence_projection;
mod profile_bindings;
#[cfg(test)]
mod tests;
