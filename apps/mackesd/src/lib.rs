//! Persistent daemon lifecycle and local command boundary.

use mackes_config::{load, ConfigDocument, ConfigError};
use mackes_ipc::{authorize, AccessPolicy, Authorization, Command, LocalServer};
use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
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
        (Command::Scenes, b"scenes"),
        (Command::DeviceQuery, b"device_query"),
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
                        "channel": route.channel.map(mackes_domain::MidiChannel::one_based),
                        "class": route.class.map(|class| format!("{class:?}")),
                    })
                })
                .collect::<Vec<_>>();
            let encoded = serde_json::to_string(&payload).unwrap_or_else(|_| "[]".to_owned());
            format!("{{\"ok\":true,\"generation\":{generation},\"routes\":{encoded},\"route_generation\":{}}}\n", route_generation.unwrap_or(0))
        }
        Command::DeviceQuery => {
            format!("{{\"ok\":true,\"generation\":{generation},\"devices\":[]}}\n")
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
            format!("{{\"ok\":true,\"generation\":{generation},\"endpoints\":{encoded}}}\n")
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
            state_sequence: 0,
            state_events: VecDeque::with_capacity(256),
        })
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
                channel,
                class,
                predicates: Vec::new(),
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
        let (sent, unmatched) = self.outputs.dispatch(&self.router, event);
        self.received_events = self.received_events.saturating_add(1);
        self.sent_events = self.sent_events.saturating_add(sent as u64);
        self.dropped_events = self.dropped_events.saturating_add(unmatched as u64);
        (sent, unmatched)
    }

    /// Returns bounded aggregate MIDI activity counters.
    #[must_use]
    pub const fn activity_counters(&self) -> (u64, u64, u64) {
        (self.received_events, self.sent_events, self.dropped_events)
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

    /// Receives one authorized RTP-MIDI packet, decodes channel voice events,
    /// and dispatches them through the daemon-owned router/output registry.
    /// System-common and `SysEx` sections remain explicitly unsupported here.
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
        &self,
        plan: &mackes_scene_engine::ActivationPlan,
        unsafe_armed: bool,
        cancelled: bool,
        execute_action: F,
    ) -> Vec<(String, mackes_scene_engine::ActionResult)>
    where
        F: FnMut(&mackes_scene_engine::ActivationAction) -> mackes_scene_engine::ActionResult,
    {
        plan.execute_with(unsafe_armed, cancelled, execute_action)
    }

    /// Executes startup restore through the ordinary planner with unsafe mode disarmed.
    #[must_use]
    pub fn execute_startup_restore<F>(
        &self,
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
        &self,
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
        plan.execute_with_deadline(unsafe_armed, cancelled, now, deadline, execute_action)
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
            "received": self.received_events,
            "sent": self.sent_events,
            "dropped": self.dropped_events,
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
            "received": self.received_events,
            "sent": self.sent_events,
            "dropped": self.dropped_events,
            "last_sequence": self.state_sequence,
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
    pub fn serve_once(&mut self, policy: AccessPolicy) -> io::Result<()> {
        let _ = self.drain_virtual_input();
        let (mut stream, identity) = self.server.accept_authorized(policy)?;
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
                    }
                }
            }
            if let Some(command) = command {
                if !matches!(
                    command,
                    Command::Hello | Command::Health | Command::Snapshot | Command::Subscribe
                ) {
                    self.record_state_event(command);
                }
            }
            match command {
                Some(Command::Snapshot) => self.snapshot_response(),
                Some(Command::Subscribe) => self.subscribe_response(&request),
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
        let daemon = Daemon::bind(&path).expect("daemon");
        let plan = mackes_scene_engine::ActivationPlan::compile(vec![
            mackes_scene_engine::ActivationAction {
                id: "write".into(),
                description: "write".into(),
                unsafe_action: false,
                depends_on: None,
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
        let daemon = Daemon::bind(&path).expect("daemon");
        let plan = mackes_scene_engine::ActivationPlan::compile(vec![
            mackes_scene_engine::ActivationAction {
                id: "safe".into(),
                description: "safe".into(),
                unsafe_action: false,
                depends_on: None,
            },
            mackes_scene_engine::ActivationAction {
                id: "unsafe".into(),
                description: "unsafe".into(),
                unsafe_action: true,
                depends_on: Some("safe".into()),
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
            ("scenes", Command::Scenes),
            ("device_query", Command::DeviceQuery),
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
            command_ack(Command::Scenes, Health::Ready, 7, &[], None, &[]),
            "{\"ok\":true,\"generation\":7,\"scenes\":[]}\n"
        );
        assert_eq!(
            command_ack(Command::DeviceQuery, Health::Ready, 8, &[], None, &[]),
            "{\"ok\":true,\"generation\":8,\"devices\":[]}\n"
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
                br#"[{"source":1,"destination":2,"channel":1,"class":"ControlChange"}]"#,
                4,
                8,
            )
            .expect("routes");
        assert_eq!(daemon.route_generation(), Some(4));
        assert_eq!(daemon.router.routes().len(), 1);
        assert!(daemon.replace_routes_json(br#"[{"source":1,"destination":1}]"#, 5, 8).is_err());
        assert_eq!(daemon.route_generation(), Some(4));
        drop(daemon);
        let _ = fs::remove_file(path);
    }
}
