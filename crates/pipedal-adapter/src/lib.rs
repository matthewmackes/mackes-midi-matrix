//! Bounded daemon-boundary orchestration for a `PiPedal` connector session.
//!
//! This crate deliberately contains no socket or MIDI implementation. The daemon boundary
//! supplies a transport, while this layer owns admission, lifecycle generation, and health
//! projection so network work cannot run on the MIDI dispatch path.

use std::io::{self, Read, Write};
use std::net::{Ipv6Addr, SocketAddr, TcpStream};
use std::time::Duration;

use mackes_pipedal_connector::{
    decode_server_frame, encode_client_text, encode_request, startup_requests, Request, Session,
    SessionPhase, TextAssembler, Transport, TransportError, MAX_FRAME_BYTES,
};

/// Maximum number of commands admitted before the worker must make progress.
pub const MAX_PENDING_COMMANDS: usize = 128;
/// Maximum mapping outcomes retained in one resolution report.
pub const MAX_RESOLUTION_OUTCOMES: usize = 128;

/// Returns the connector operations that are qualified for discovery by API clients.
#[must_use]
pub const fn supported_operations() -> &'static [mackes_pipedal_connector::Operation] {
    mackes_pipedal_connector::Operation::all()
}

/// Returns the default qualified `PiPedal` control endpoint.
#[must_use]
pub const fn default_endpoint() -> SocketAddr {
    SocketAddr::new(std::net::IpAddr::V6(Ipv6Addr::LOCALHOST), 8080)
}

/// A small, nonblocking WebSocket transport for the qualified `PiPedal` endpoint.
#[derive(Debug)]
pub struct WebSocketTransport {
    stream: TcpStream,
    read_buffer: Vec<u8>,
    text_assembler: TextAssembler,
    mask_counter: u32,
}

impl WebSocketTransport {
    /// Connects and performs the `PiPedal` HTTP WebSocket upgrade.
    ///
    /// # Errors
    ///
    /// Returns `Disconnected` for TCP or upgrade failures and `Protocol` for an invalid response.
    pub fn connect(endpoint: SocketAddr) -> Result<Self, TransportError> {
        let mut stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2))
            .map_err(|_| TransportError::Disconnected)?;
        stream.set_nonblocking(false).map_err(|_| TransportError::Disconnected)?;
        stream
            .write_all(b"GET /pipedal HTTP/1.1\r\nHost: [::1]:8080\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: bWFja2VzLXBpcGVkYWw=\r\n\r\n")
            .map_err(|_| TransportError::Disconnected)?;
        let mut response = Vec::with_capacity(512);
        let mut chunk = [0_u8; 256];
        while response.len() < 4096 && !response.windows(4).any(|w| w == b"\r\n\r\n") {
            let count = stream.read(&mut chunk).map_err(|_| TransportError::Disconnected)?;
            if count == 0 {
                return Err(TransportError::Disconnected);
            }
            response.extend_from_slice(&chunk[..count]);
        }
        if !response.starts_with(b"HTTP/1.1 101") {
            return Err(TransportError::Protocol);
        }
        stream
            .set_read_timeout(Some(Duration::from_millis(1)))
            .map_err(|_| TransportError::Disconnected)?;
        stream.set_nonblocking(true).map_err(|_| TransportError::Disconnected)?;
        Ok(Self {
            stream,
            read_buffer: Vec::new(),
            text_assembler: TextAssembler::default(),
            mask_counter: 0,
        })
    }

    fn frame_size(buffer: &[u8]) -> Option<usize> {
        if buffer.len() < 2 {
            return None;
        }
        let length = match buffer[1] & 0x7f {
            n @ 0..=125 => usize::from(n),
            126 if buffer.len() >= 4 => usize::from(u16::from_be_bytes([buffer[2], buffer[3]])),
            127 if buffer.len() >= 10 => {
                usize::try_from(u64::from_be_bytes(buffer[2..10].try_into().ok()?)).ok()?
            }
            _ => return None,
        };
        let header: usize = if buffer[1] & 0x7f <= 125 {
            2
        } else if buffer[1] & 0x7f == 126 {
            4
        } else {
            10
        };
        Some(header.saturating_add(length))
    }
}

impl Transport for WebSocketTransport {
    fn send(&mut self, frame: &[u8]) -> Result<(), TransportError> {
        let mask = self.mask_counter.to_be_bytes();
        self.mask_counter = self.mask_counter.wrapping_add(1);
        let encoded = encode_client_text(frame, mask).map_err(|_| TransportError::Protocol)?;
        self.stream.write_all(&encoded).map_err(|error| {
            if error.kind() == io::ErrorKind::WouldBlock {
                TransportError::Timeout
            } else {
                TransportError::Disconnected
            }
        })
    }

    fn receive(&mut self) -> Result<Option<Vec<u8>>, TransportError> {
        let mut chunk = [0_u8; 4096];
        match self.stream.read(&mut chunk) {
            Ok(0) => return Err(TransportError::Disconnected),
            Ok(count) => self.read_buffer.extend_from_slice(&chunk[..count]),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(_) => return Err(TransportError::Disconnected),
        }
        let Some(size) = Self::frame_size(&self.read_buffer) else { return Ok(None) };
        if size > MAX_FRAME_BYTES.saturating_add(10) {
            return Err(TransportError::Protocol);
        }
        if self.read_buffer.len() < size {
            return Ok(None);
        }
        let frame = self.read_buffer.drain(..size).collect::<Vec<_>>();
        let decoded = decode_server_frame(&frame).map_err(|_| TransportError::Protocol)?;
        if decoded.opcode == 8 {
            return Err(TransportError::Disconnected);
        }
        if decoded.opcode == 9 {
            return Ok(None);
        }
        self.text_assembler
            .push(&decoded.payload, decoded.final_fragment)
            .map_err(|_| TransportError::Protocol)
    }
}

/// A bounded worker command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Ask the session to begin its qualified handshake.
    Start,
    /// Request a graceful session reset.
    Reconnect,
}

/// Validation result for one persisted mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionState {
    /// Exactly one writable target resolved.
    Resolved,
    /// No matching plugin/control exists.
    Unavailable,
    /// More than one target matches without an explicit scope.
    Ambiguous,
    /// The target exists but cannot accept writes.
    ReadOnly,
}

/// Bounded mapping-resolution report entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionOutcome {
    /// Stable physical control identity.
    pub physical_control_id: String,
    /// Stable plugin URI used to join this result with catalog metadata.
    pub plugin_uri: String,
    /// Stable plugin parameter symbol used to join this result with catalog metadata.
    pub symbol: String,
    /// Runtime instance identity when exactly one writable target resolves.
    pub instance_id: Option<u64>,
    /// Resolution state.
    pub state: ResolutionState,
    /// Short operator-facing detail.
    pub detail: String,
}

/// Stable persisted mapping identity accepted by the adapter boundary.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MappingIdentity {
    /// Stable physical control identity.
    pub physical_control_id: String,
    /// Stable plugin URI.
    pub plugin_uri: String,
    /// Stable parameter symbol.
    pub symbol: String,
    /// Optional instance-selection scope.
    pub scope: Option<String>,
}

/// One reversible scalar mutation retained by the adapter.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ApplyRecord {
    /// Mapping identity that was changed.
    pub mapping: MappingIdentity,
    /// Runtime plugin instance used for the mutation.
    pub instance_id: u64,
    /// Value observed before the mutation.
    pub previous_value: f32,
    /// Session generation in which the mutation was admitted.
    pub generation: u64,
}

/// Explicit restore intent returned by an undo request.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RestoreIntent {
    /// Mapping identity to restore.
    pub mapping: MappingIdentity,
    /// Runtime instance to address after fresh catalog validation.
    pub instance_id: u64,
    /// Previously observed value.
    pub value: f32,
    /// Generation that must still be current before preparation.
    pub generation: u64,
}

/// Public health projection for IPC or UI consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Health {
    /// Current protocol phase.
    pub phase: SessionPhase,
    /// Monotonic generation; changes invalidate stale work.
    pub generation: u64,
    /// Number of encoded requests waiting for transport service.
    pub pending_requests: usize,
    /// Number of lifecycle commands not yet processed.
    pub pending_commands: usize,
    /// Last transport failure, if any.
    pub last_error: Option<TransportError>,
}

/// Session lifecycle coordinator with bounded command admission.
#[derive(Debug)]
pub struct Worker {
    session: Session,
    catalog: mackes_pipedal_connector::PluginCatalog,
    favorites: std::collections::BTreeMap<String, bool>,
    governor_settings: String,
    show_status_monitor: Option<bool>,
    has_wifi: Option<bool>,
    wifi_config_settings: Option<mackes_pipedal_connector::WifiConfigSettings>,
    alsa_sequencer_configuration: Option<mackes_pipedal_connector::AlsaSequencerConfiguration>,
    alsa_sequencer_ports: Vec<mackes_pipedal_connector::AlsaSequencerConnection>,
    update_status: Option<mackes_pipedal_connector::UpdateStatus>,
    known_wifi_networks: Vec<String>,
    wifi_channels: Vec<mackes_pipedal_connector::WifiChannel>,
    alsa_devices: Vec<mackes_pipedal_connector::AlsaDeviceInfo>,
    jack_status: Option<mackes_pipedal_connector::JackHostStatus>,
    jack_server_settings: Option<mackes_pipedal_connector::JackServerSettings>,
    jack_settings: Option<mackes_pipedal_connector::JackChannelSelection>,
    jack_configuration: Option<mackes_pipedal_connector::JackConfiguration>,
    plugin_presets: std::collections::BTreeMap<String, mackes_pipedal_connector::PluginUiPresets>,
    wifi_regulatory_domains: std::collections::BTreeMap<String, String>,
    system_midi_bindings: Vec<mackes_pipedal_connector::MidiBinding>,
    version: Option<mackes_pipedal_connector::PiPedalVersion>,
    runtime_targets: Vec<(u64, String)>,
    pickup_ledger: mackes_pipedal_connector::ReconciliationLedger,
    pickup_targets: Vec<(String, String, String)>,
    apply_record: Option<ApplyRecord>,
    expected_replies: Vec<u64>,
    startup_next: usize,
    pipedal_client_id: Option<u64>,
    next_reply_id: u64,
    pending: Vec<Command>,
    last_error: Option<TransportError>,
    successful_reads: u64,
}

impl Default for Worker {
    fn default() -> Self {
        Self::new(Session::default())
    }
}

impl Worker {
    /// Creates an idle worker.
    #[must_use]
    pub fn new(session: Session) -> Self {
        Self {
            session,
            catalog: mackes_pipedal_connector::PluginCatalog::default(),
            favorites: std::collections::BTreeMap::new(),
            governor_settings: String::new(),
            show_status_monitor: None,
            has_wifi: None,
            wifi_config_settings: None,
            alsa_sequencer_configuration: None,
            alsa_sequencer_ports: Vec::new(),
            update_status: None,
            known_wifi_networks: Vec::new(),
            wifi_channels: Vec::new(),
            alsa_devices: Vec::new(),
            jack_status: None,
            jack_server_settings: None,
            jack_settings: None,
            jack_configuration: None,
            plugin_presets: std::collections::BTreeMap::new(),
            wifi_regulatory_domains: std::collections::BTreeMap::new(),
            system_midi_bindings: Vec::new(),
            version: None,
            runtime_targets: Vec::new(),
            pickup_ledger: mackes_pipedal_connector::ReconciliationLedger::default(),
            pickup_targets: Vec::new(),
            apply_record: None,
            expected_replies: Vec::with_capacity(9),
            startup_next: 0,
            pipedal_client_id: None,
            next_reply_id: 1,
            pending: Vec::with_capacity(MAX_PENDING_COMMANDS),
            last_error: None,
            successful_reads: 0,
        }
    }

    /// Admits one command without performing I/O.
    ///
    /// # Errors
    ///
    /// Returns the command when the bounded queue is full.
    pub fn enqueue(&mut self, command: Command) -> Result<(), Command> {
        if self.pending.len() >= MAX_PENDING_COMMANDS {
            return Err(command);
        }
        self.pending.push(command);
        Ok(())
    }

    /// Processes at most `budget` lifecycle commands; transport I/O is supplied by the caller.
    pub fn process<T: Transport>(&mut self, transport: &mut T, budget: usize) {
        let _ = transport;
        for _ in 0..budget {
            let Some(command) = self.pending.pop() else { break };
            match command {
                Command::Start => {
                    if let Err(error) = self.session.connect() {
                        self.last_error = Some(TransportError::Protocol);
                        let _ = error;
                    } else {
                        self.expected_replies.clear();
                        self.startup_next = 0;
                        self.queue_startup_request();
                        self.next_reply_id =
                            u64::try_from(startup_requests().len() + 1).unwrap_or(1);
                    }
                }
                Command::Reconnect => {
                    self.session.reset();
                    self.expected_replies.clear();
                    self.startup_next = 0;
                    self.runtime_targets.clear();
                    self.favorites.clear();
                    self.governor_settings.clear();
                    self.show_status_monitor = None;
                    self.has_wifi = None;
                    self.wifi_config_settings = None;
                    self.alsa_sequencer_configuration = None;
                    self.alsa_sequencer_ports.clear();
                    self.update_status = None;
                    self.known_wifi_networks.clear();
                    self.wifi_channels.clear();
                    self.alsa_devices.clear();
                    self.jack_status = None;
                    self.jack_server_settings = None;
                    self.jack_settings = None;
                    self.jack_configuration = None;
                    self.plugin_presets.clear();
                    self.wifi_regulatory_domains.clear();
                    self.system_midi_bindings.clear();
                    self.version = None;
                    self.pickup_ledger.clear();
                    self.pickup_targets.clear();
                    self.next_reply_id = 1;
                }
            }
        }
    }

    fn queue_startup_request(&mut self) {
        let requests = startup_requests();
        let Some(message) = requests.get(self.startup_next) else { return };
        let reply_to = u64::try_from(self.startup_next + 1).unwrap_or(u64::MAX);
        let request =
            Request { message: (*message).to_owned(), reply_to: Some(reply_to), body: None::<()> };
        match encode_request(&request) {
            Ok(frame) => {
                let generation = self.session.generation();
                if self.session.enqueue(generation, frame).is_ok() {
                    self.expected_replies.push(reply_to);
                    self.startup_next += 1;
                } else {
                    self.last_error = Some(TransportError::Protocol);
                }
            }
            Err(_) => self.last_error = Some(TransportError::Protocol),
        }
    }

    /// Services bounded outbound and inbound transport work for one worker tick.
    ///
    /// The returned frames are complete transport frames; protocol decoding and catalog
    /// projection remain explicit follow-up steps for the daemon adapter.
    ///
    /// # Errors
    ///
    /// Returns the connector's transport error and resets the session when the peer cannot be
    /// serviced. A failed outbound frame is not retried implicitly.
    pub fn pump<T: Transport>(
        &mut self,
        transport: &mut T,
        send_budget: usize,
        receive_budget: usize,
    ) -> Result<Vec<Vec<u8>>, TransportError> {
        if let Err(error) = self.session.send_pending(transport, send_budget) {
            self.last_error = Some(error);
            self.session.reset();
            self.expected_replies.clear();
            return Err(error);
        }
        match self.session.receive_available(transport, receive_budget) {
            Ok(frames) => Ok(frames),
            Err(error) => {
                self.last_error = Some(error);
                self.session.reset();
                self.expected_replies.clear();
                Err(error)
            }
        }
    }

    /// Feeds one complete `PiPedal` message into the session phase machine.
    ///
    /// # Errors
    ///
    /// Returns `Protocol` when the frame or phase transition is invalid.
    pub fn accept_frame(&mut self, frame: &[u8]) -> Result<SessionPhase, TransportError> {
        let (header, body) = mackes_pipedal_connector::decode_message(frame)
            .map_err(|_| TransportError::Protocol)?;
        if let Some(reply) = header.reply_to {
            let Some(position) =
                self.expected_replies.iter().position(|expected| *expected == reply)
            else {
                return Err(TransportError::Protocol);
            };
            self.expected_replies.remove(position);
            if self.startup_next < startup_requests().len() && self.expected_replies.is_empty() {
                self.queue_startup_request();
            }
        }
        if header.message == "plugins" {
            self.catalog = decode_catalog(body.as_ref().ok_or(TransportError::Protocol)?)
                .map_err(|_| TransportError::Protocol)?;
        }
        self.accept_auxiliary_readback(&header.message, body.clone())?;
        if header.message == "ehlo" {
            self.pipedal_client_id = body.as_ref().and_then(serde_json::Value::as_u64);
        }
        if header.message == "version" {
            self.version = Some(
                mackes_pipedal_connector::decode_version(body.clone())
                    .map_err(|_| TransportError::Protocol)?,
            );
        }
        if header.message == "currentPedalboard" {
            ingest_current_pedalboard(
                &mut self.catalog,
                &mut self.runtime_targets,
                body.as_ref().ok_or(TransportError::Protocol)?,
            )
            .map_err(|_| TransportError::Protocol)?;
        }
        if header.message == "onControlChanged" {
            let (uri, symbol, value) = ingest_control_changed(
                &mut self.catalog,
                &self.runtime_targets,
                body.as_ref().ok_or(TransportError::Protocol)?,
            )
            .map_err(|_| TransportError::Protocol)?;
            for (physical_control_id, target_uri, target_symbol) in &self.pickup_targets {
                if target_uri == &uri && target_symbol == &symbol {
                    self.pickup_ledger
                        .arm(
                            physical_control_id,
                            mackes_pipedal_connector::PickupState::arm(
                                self.session.generation(),
                                value,
                            )
                            .map_err(|_| TransportError::Protocol)?,
                        )
                        .map_err(|_| TransportError::Protocol)?;
                }
            }
        }
        let phase = self.session.accept(&header.message).map_err(|_| TransportError::Protocol)?;
        self.successful_reads = self.successful_reads.saturating_add(1);
        Ok(phase)
    }

    #[allow(clippy::too_many_lines)]
    fn accept_auxiliary_readback(
        &mut self,
        message: &str,
        body: Option<serde_json::Value>,
    ) -> Result<(), TransportError> {
        match message {
            "pluginClasses" => mackes_pipedal_connector::decode_plugin_classes(body)
                .map(|_| ())
                .map_err(|_| TransportError::Protocol),
            "getFavorites" => {
                self.favorites = mackes_pipedal_connector::decode_favorites(body)
                    .map_err(|_| TransportError::Protocol)?;
                Ok(())
            }
            "getSystemMidiBindings" => {
                self.system_midi_bindings =
                    mackes_pipedal_connector::decode_system_midi_bindings(body)
                        .map_err(|_| TransportError::Protocol)?;
                Ok(())
            }
            "getGovernorSettings" => {
                self.governor_settings = mackes_pipedal_connector::decode_governor_settings(body)
                    .map_err(|_| TransportError::Protocol)?;
                Ok(())
            }
            "getShowStatusMonitor" => {
                self.show_status_monitor = Some(
                    mackes_pipedal_connector::decode_show_status_monitor(body)
                        .map_err(|_| TransportError::Protocol)?,
                );
                Ok(())
            }
            "getHasWifi" => {
                self.has_wifi = Some(
                    mackes_pipedal_connector::decode_has_wifi(body)
                        .map_err(|_| TransportError::Protocol)?,
                );
                Ok(())
            }
            "getWifiConfigSettings" => {
                self.wifi_config_settings = Some(
                    mackes_pipedal_connector::decode_wifi_config_settings(body)
                        .map_err(|_| TransportError::Protocol)?,
                );
                Ok(())
            }
            "getAlsaSequencerConfiguration" => {
                self.alsa_sequencer_configuration = Some(
                    mackes_pipedal_connector::decode_alsa_sequencer_configuration(body)
                        .map_err(|_| TransportError::Protocol)?,
                );
                Ok(())
            }
            "getAlsaSequencerPorts" => {
                self.alsa_sequencer_ports =
                    mackes_pipedal_connector::decode_alsa_sequencer_ports(body)
                        .map_err(|_| TransportError::Protocol)?;
                Ok(())
            }
            "getUpdateStatus" => {
                self.update_status = Some(
                    mackes_pipedal_connector::decode_update_status(body)
                        .map_err(|_| TransportError::Protocol)?,
                );
                Ok(())
            }
            "getKnownWifiNetworks" => {
                self.known_wifi_networks =
                    mackes_pipedal_connector::decode_known_wifi_networks(body)
                        .map_err(|_| TransportError::Protocol)?;
                Ok(())
            }
            "getWifiChannels" => {
                self.wifi_channels = mackes_pipedal_connector::decode_wifi_channels(body)
                    .map_err(|_| TransportError::Protocol)?;
                Ok(())
            }
            "getAlsaDevices" => {
                self.alsa_devices = mackes_pipedal_connector::decode_alsa_devices(body)
                    .map_err(|_| TransportError::Protocol)?;
                Ok(())
            }
            "getJackStatus" => {
                self.jack_status = Some(
                    mackes_pipedal_connector::decode_jack_status(body)
                        .map_err(|_| TransportError::Protocol)?,
                );
                Ok(())
            }
            "getJackServerSettings" => {
                self.jack_server_settings = Some(
                    mackes_pipedal_connector::decode_jack_server_settings(body)
                        .map_err(|_| TransportError::Protocol)?,
                );
                Ok(())
            }
            "getJackSettings" => {
                self.jack_settings = Some(
                    mackes_pipedal_connector::decode_jack_settings(body)
                        .map_err(|_| TransportError::Protocol)?,
                );
                Ok(())
            }
            "getJackConfiguration" => {
                self.jack_configuration = Some(
                    mackes_pipedal_connector::decode_jack_configuration(body)
                        .map_err(|_| TransportError::Protocol)?,
                );
                Ok(())
            }
            "getPluginPresets" => {
                let catalog = mackes_pipedal_connector::decode_plugin_presets(body)
                    .map_err(|_| TransportError::Protocol)?;
                self.plugin_presets.insert(catalog.plugin_uri.clone(), catalog);
                Ok(())
            }
            "getWifiRegulatoryDomains" => {
                self.wifi_regulatory_domains =
                    mackes_pipedal_connector::decode_wifi_regulatory_domains(body)
                        .map_err(|_| TransportError::Protocol)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Returns the numeric client identifier assigned by `PiPedal` during `hello`.
    #[must_use]
    pub const fn pipedal_client_id(&self) -> Option<u64> {
        self.pipedal_client_id
    }

    /// Returns the latest validated `PiPedal` version metadata for this session.
    #[must_use]
    pub const fn version(&self) -> Option<&mackes_pipedal_connector::PiPedalVersion> {
        self.version.as_ref()
    }

    /// Arms pickup for one stable mapping from its freshest catalog value.
    ///
    /// # Errors
    ///
    /// Returns an error when the target has no fresh value or pickup capacity is exhausted.
    pub fn arm_pickup(&mut self, mapping: &MappingIdentity) -> Result<(), String> {
        let value = self
            .catalog
            .find_control(&mapping.plugin_uri, &mapping.symbol)
            .and_then(|control| control.value)
            .ok_or("PiPedal pickup target value is unavailable")?;
        self.pickup_ledger.arm(
            &mapping.physical_control_id,
            mackes_pipedal_connector::PickupState::arm(self.session.generation(), value)?,
        )?;
        if let Some(target) = self
            .pickup_targets
            .iter_mut()
            .find(|(physical, _, _)| physical == &mapping.physical_control_id)
        {
            *target = (
                mapping.physical_control_id.clone(),
                mapping.plugin_uri.clone(),
                mapping.symbol.clone(),
            );
        } else if self.pickup_targets.len() < mackes_pipedal_connector::MAX_MAPPINGS {
            self.pickup_targets.push((
                mapping.physical_control_id.clone(),
                mapping.plugin_uri.clone(),
                mapping.symbol.clone(),
            ));
        } else {
            return Err("PiPedal pickup target registry is full".into());
        }
        Ok(())
    }

    /// Arms newly discovered persisted mappings against the current catalog.
    ///
    /// Existing targets are left untouched so repeated daemon ticks cannot reset a physical
    /// control that has already moved toward pickup. Targets without a fresh catalog value are
    /// skipped and will be retried after the next catalog update.
    pub fn reconcile_pickup_targets(&mut self, mappings: &[MappingIdentity]) {
        for mapping in mappings.iter().take(mackes_pipedal_connector::MAX_MAPPINGS) {
            if self
                .pickup_targets
                .iter()
                .any(|(physical, _, _)| physical == &mapping.physical_control_id)
            {
                continue;
            }
            let _ = self.arm_pickup(mapping);
        }
    }

    /// Records a physical value and reports whether pickup permits delivery.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown control, stale/invalid observation, or tolerance.
    pub fn observe_pickup(
        &mut self,
        physical_control_id: &str,
        generation: u64,
        value: f32,
        tolerance: f32,
    ) -> Result<bool, String> {
        self.pickup_ledger.observe(physical_control_id, generation, value, tolerance)
    }

    /// Returns the latest validated plugin catalog.
    #[must_use]
    pub const fn catalog(&self) -> &mackes_pipedal_connector::PluginCatalog {
        &self.catalog
    }

    /// Returns the latest bounded favorite-URI projection.
    #[must_use]
    pub const fn favorites(&self) -> &std::collections::BTreeMap<String, bool> {
        &self.favorites
    }

    /// Returns the latest bounded system MIDI binding projection.
    #[must_use]
    pub fn system_midi_bindings(&self) -> &[mackes_pipedal_connector::MidiBinding] {
        &self.system_midi_bindings
    }

    /// Last validated CPU-governor readback.
    #[must_use]
    pub fn governor_settings(&self) -> &str {
        &self.governor_settings
    }

    /// Last validated status-monitor visibility readback.
    #[must_use]
    pub const fn show_status_monitor(&self) -> Option<bool> {
        self.show_status_monitor
    }

    /// Last validated Wi-Fi availability projection.
    #[must_use]
    pub const fn has_wifi(&self) -> Option<bool> {
        self.has_wifi
    }

    /// Last validated update-status readback.
    #[must_use]
    pub const fn update_status(&self) -> Option<&mackes_pipedal_connector::UpdateStatus> {
        self.update_status.as_ref()
    }

    /// Last validated known Wi-Fi network names.
    #[must_use]
    pub fn known_wifi_networks(&self) -> &[String] {
        &self.known_wifi_networks
    }

    /// Last validated Wi-Fi channel selectors.
    #[must_use]
    pub fn wifi_channels(&self) -> &[mackes_pipedal_connector::WifiChannel] {
        &self.wifi_channels
    }

    /// Last validated ALSA audio-device inventory.
    #[must_use]
    pub fn alsa_devices(&self) -> &[mackes_pipedal_connector::AlsaDeviceInfo] {
        &self.alsa_devices
    }

    /// Last validated JACK/audio-host status.
    #[must_use]
    pub const fn jack_status(&self) -> Option<&mackes_pipedal_connector::JackHostStatus> {
        self.jack_status.as_ref()
    }

    /// Last validated password-redacted Wi-Fi configuration.
    #[must_use]
    pub const fn wifi_config_settings(
        &self,
    ) -> Option<&mackes_pipedal_connector::WifiConfigSettings> {
        self.wifi_config_settings.as_ref()
    }

    /// Last validated ALSA sequencer configuration.
    #[must_use]
    pub const fn alsa_sequencer_configuration(
        &self,
    ) -> Option<&mackes_pipedal_connector::AlsaSequencerConfiguration> {
        self.alsa_sequencer_configuration.as_ref()
    }

    /// Last validated ALSA sequencer port inventory.
    #[must_use]
    pub fn alsa_sequencer_ports(&self) -> &[mackes_pipedal_connector::AlsaSequencerConnection] {
        &self.alsa_sequencer_ports
    }

    /// Last validated JACK server configuration.
    #[must_use]
    pub const fn jack_server_settings(
        &self,
    ) -> Option<&mackes_pipedal_connector::JackServerSettings> {
        self.jack_server_settings.as_ref()
    }

    /// Last validated JACK channel selection.
    #[must_use]
    pub const fn jack_settings(&self) -> Option<&mackes_pipedal_connector::JackChannelSelection> {
        self.jack_settings.as_ref()
    }

    /// Last validated JACK runtime configuration.
    #[must_use]
    pub const fn jack_configuration(&self) -> Option<&mackes_pipedal_connector::JackConfiguration> {
        self.jack_configuration.as_ref()
    }

    /// Last validated plugin-preset catalogs keyed by plugin URI.
    #[must_use]
    pub const fn plugin_presets(
        &self,
    ) -> &std::collections::BTreeMap<String, mackes_pipedal_connector::PluginUiPresets> {
        &self.plugin_presets
    }

    /// Last validated Wi-Fi regulatory-domain labels.
    #[must_use]
    pub const fn wifi_regulatory_domains(&self) -> &std::collections::BTreeMap<String, String> {
        &self.wifi_regulatory_domains
    }

    /// Resolves the current runtime instance for a persisted mapping.
    ///
    /// An explicit `instance:<id>` scope is honored; otherwise exactly one current
    /// instance for the plugin URI is required. Ambiguous or missing targets fail closed.
    ///
    /// # Errors
    ///
    /// Returns an error when the scope is malformed or the target is unavailable or ambiguous.
    pub fn resolve_instance_id(&self, mapping: &MappingIdentity) -> Result<u64, String> {
        let candidates = if self.runtime_targets.is_empty() {
            self.catalog
                .targets
                .iter()
                .map(|target| (target.instance_id, target.uri.as_str()))
                .collect::<Vec<_>>()
        } else {
            self.runtime_targets
                .iter()
                .map(|(instance_id, uri)| (*instance_id, uri.as_str()))
                .collect::<Vec<_>>()
        };
        if let Some(scope) = mapping.scope.as_deref() {
            let instance_id = scope
                .strip_prefix("instance:")
                .ok_or("PiPedal mapping scope must use instance:<id>")?
                .parse::<u64>()
                .map_err(|_| "PiPedal mapping instance scope is invalid")?;
            if instance_id == 0 {
                return Err("PiPedal mapping instance scope is invalid".into());
            }
            return candidates
                .iter()
                .find(|(candidate, uri)| *candidate == instance_id && *uri == mapping.plugin_uri)
                .map(|(candidate, _)| *candidate)
                .ok_or_else(|| "PiPedal mapping instance is unavailable".into());
        }
        let matches = candidates
            .iter()
            .filter(|(_, uri)| *uri == mapping.plugin_uri)
            .map(|(instance_id, _)| *instance_id)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [instance_id] => Ok(*instance_id),
            [] => Err("PiPedal mapping plugin is unavailable".into()),
            _ => Err("PiPedal mapping plugin is ambiguous; select an instance".into()),
        }
    }

    /// Converts one normalized controller value and admits its matching `PiPedal` write.
    ///
    /// Pickup is evaluated before queue admission, so a physical control cannot jump a
    /// parameter after discovery or reconnect. `Ok(false)` means pickup is still waiting.
    ///
    /// # Errors
    ///
    /// Returns an error when the mapping, pickup state, runtime instance, session, or queue is
    /// unavailable or invalid.
    pub fn apply_physical_control(
        &mut self,
        generation: u64,
        mapping: &MappingIdentity,
        normalized_value: u8,
        tolerance: f32,
    ) -> Result<bool, String> {
        let connector_mapping = mackes_pipedal_connector::ControlMapping {
            physical_control_id: mapping.physical_control_id.clone(),
            plugin_uri: mapping.plugin_uri.clone(),
            symbol: mapping.symbol.clone(),
            scope: mapping.scope.clone(),
        };
        let control = self.catalog.resolve_mapping(&connector_mapping)?;
        let value = (control.max_value - control.min_value)
            .mul_add(f32::from(normalized_value) / f32::from(u8::MAX), control.min_value);
        if !self.observe_pickup(&mapping.physical_control_id, generation, value, tolerance)? {
            return Ok(false);
        }
        let instance_id = self.resolve_instance_id(mapping)?;
        let client_id = self.pipedal_client_id.ok_or("PiPedal client identity is unavailable")?;
        let reply_to = self.allocate_reply_id();
        self.apply_set_control(
            generation,
            mapping,
            instance_id,
            client_id,
            Some(reply_to),
            value,
            true,
        )?;
        Ok(true)
    }

    /// Resolves persisted mappings against the current catalog without performing I/O.
    #[must_use]
    pub fn resolve_mappings(&self, mappings: &[MappingIdentity]) -> Vec<ResolutionOutcome> {
        mappings
            .iter()
            .take(MAX_RESOLUTION_OUTCOMES)
            .map(|mapping| {
                let connector_mapping = mackes_pipedal_connector::ControlMapping {
                    physical_control_id: mapping.physical_control_id.clone(),
                    plugin_uri: mapping.plugin_uri.clone(),
                    symbol: mapping.symbol.clone(),
                    scope: mapping.scope.clone(),
                };
                let (state, detail, instance_id) =
                    match self.catalog.resolve_mapping(&connector_mapping) {
                        Ok(_) => match self.resolve_instance_id(mapping) {
                            Ok(instance_id) => (
                                ResolutionState::Resolved,
                                "target is available and writable".to_owned(),
                                Some(instance_id),
                            ),
                            Err(error) if error.contains("ambiguous") => {
                                (ResolutionState::Ambiguous, error, None)
                            }
                            Err(error) => (ResolutionState::Unavailable, error, None),
                        },
                        Err(error) if error.contains("ambiguous") => {
                            (ResolutionState::Ambiguous, error, None)
                        }
                        Err(error) if error.contains("read-only") => {
                            (ResolutionState::ReadOnly, error, None)
                        }
                        Err(error) => (ResolutionState::Unavailable, error, None),
                    };
                ResolutionOutcome {
                    physical_control_id: mapping.physical_control_id.clone(),
                    plugin_uri: mapping.plugin_uri.clone(),
                    symbol: mapping.symbol.clone(),
                    instance_id,
                    state,
                    detail,
                }
            })
            .collect()
    }

    /// Prepares one generation-checked `setControl` request after catalog validation.
    ///
    /// # Errors
    ///
    /// Rejects stale generations, unavailable targets, and out-of-range values. The returned
    /// frame is not queued or sent.
    pub fn prepare_set_control(
        &self,
        generation: u64,
        mapping: &MappingIdentity,
        instance_id: u64,
        client_id: u64,
        reply_to: Option<u64>,
        value: f32,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal mapping belongs to an old session generation".into());
        }
        let connector_mapping = mackes_pipedal_connector::ControlMapping {
            physical_control_id: mapping.physical_control_id.clone(),
            plugin_uri: mapping.plugin_uri.clone(),
            symbol: mapping.symbol.clone(),
            scope: mapping.scope.clone(),
        };
        let control = self.catalog.resolve_mapping(&connector_mapping)?;
        if !value.is_finite() || value < control.min_value || value > control.max_value {
            return Err("PiPedal control value is outside its catalog range".into());
        }
        let body = mackes_pipedal_connector::SetControl {
            client_id: client_id.to_owned(),
            instance_id,
            symbol: mapping.symbol.clone(),
            value,
        };
        body.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "setControl".into(),
            reply_to,
            body: Some(body),
        })
        .map_err(|error| error.to_string())
    }

    /// Validates and queues one explicitly confirmed `setControl` request.
    ///
    /// # Errors
    ///
    /// Rejects missing confirmation, a non-ready session, stale generation, or invalid target
    /// metadata without changing the request queue.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_set_control(
        &mut self,
        generation: u64,
        mapping: &MappingIdentity,
        instance_id: u64,
        client_id: u64,
        reply_to: Option<u64>,
        value: f32,
        confirmed: bool,
    ) -> Result<(), String> {
        if !confirmed {
            return Err("PiPedal setControl requires explicit confirmation".into());
        }
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for control delivery".into());
        }
        if reply_to.is_some() && self.expected_replies.len() >= 128 {
            return Err("PiPedal reply tracking capacity is full".into());
        }
        let frame =
            self.prepare_set_control(generation, mapping, instance_id, client_id, reply_to, value)?;
        self.session.enqueue_control(generation, frame)?;
        if let Some(reply_to) = reply_to {
            self.expected_replies.push(reply_to);
        }
        Ok(())
    }

    /// Prepares a generation-checked complete replacement for `PiPedal` system MIDI bindings.
    ///
    /// The installed `PiPedal` protocol treats this as a replacement set, so callers must supply
    /// the full validated response rather than a partial patch.
    ///
    /// # Errors
    ///
    /// Returns an error when the generation is stale or the binding set is invalid.
    pub fn prepare_set_system_midi_bindings(
        &self,
        generation: u64,
        bindings: mackes_pipedal_connector::SystemMidiBindings,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal MIDI bindings belong to an old session generation".into());
        }
        bindings.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "setSystemMidiBindings".into(),
            reply_to,
            body: Some(bindings),
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a confirmed complete replacement for `PiPedal` system MIDI bindings.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation is absent, the session is not ready, the generation
    /// is stale, or the binding set is invalid.
    pub fn apply_set_system_midi_bindings(
        &mut self,
        generation: u64,
        bindings: mackes_pipedal_connector::SystemMidiBindings,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<(), String> {
        if !confirmed {
            return Err("PiPedal MIDI binding replacement requires explicit confirmation".into());
        }
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for MIDI binding delivery".into());
        }
        let frame = self.prepare_set_system_midi_bindings(generation, bindings, reply_to)?;
        self.session.enqueue_control(generation, frame)
    }

    /// Prepares a generation-checked pedalboard enable/bypass request.
    ///
    /// # Errors
    ///
    /// Returns an error when the session generation is stale or the instance identity is invalid.
    pub fn prepare_set_pedalboard_item_enable(
        &self,
        generation: u64,
        client_id: u64,
        instance_id: u64,
        enabled: bool,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal pedalboard item belongs to an old session generation".into());
        }
        let body =
            mackes_pipedal_connector::SetPedalboardItemEnable { client_id, instance_id, enabled };
        body.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "setPedalboardItemEnable".into(),
            reply_to,
            body: Some(body),
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a confirmed pedalboard enable/bypass request.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation is absent, the session is not ready, the generation is
    /// stale, or the instance identity is invalid.
    pub fn apply_set_pedalboard_item_enable(
        &mut self,
        generation: u64,
        client_id: u64,
        instance_id: u64,
        enabled: bool,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<(), String> {
        if !confirmed {
            return Err("PiPedal pedalboard enable requires explicit confirmation".into());
        }
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for pedalboard delivery".into());
        }
        let frame = self.prepare_set_pedalboard_item_enable(
            generation,
            client_id,
            instance_id,
            enabled,
            reply_to,
        )?;
        self.session.enqueue_control(generation, frame)
    }

    /// Prepare a generation-checked plugin-UI mode request.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale generation, invalid instance identity, or encoding failure.
    pub fn prepare_set_pedalboard_item_use_mod_ui(
        &self,
        generation: u64,
        client_id: u64,
        instance_id: u64,
        use_mod_ui: bool,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal pedalboard item belongs to an old session generation".into());
        }
        let body = mackes_pipedal_connector::SetPedalboardItemUseModUi {
            client_id,
            instance_id,
            use_mod_ui,
        };
        body.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "setPedalboardItemUseModUi".into(),
            reply_to,
            body: Some(body),
        })
        .map_err(|error| error.to_string())
    }

    /// Queue a confirmed plugin-UI mode request.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, readiness, generation, identity, or queue admission
    /// validation fails.
    pub fn apply_set_pedalboard_item_use_mod_ui(
        &mut self,
        generation: u64,
        client_id: u64,
        instance_id: u64,
        use_mod_ui: bool,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<(), String> {
        if !confirmed {
            return Err("PiPedal plugin UI mode requires explicit confirmation".into());
        }
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for plugin UI mode".into());
        }
        let frame = self.prepare_set_pedalboard_item_use_mod_ui(
            generation,
            client_id,
            instance_id,
            use_mod_ui,
            reply_to,
        )?;
        self.session.enqueue_control(generation, frame)
    }

    /// Prepare a generation-checked pedalboard item title request.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale generation, invalid title fields, or encoding failure.
    pub fn prepare_set_pedalboard_item_title(
        &self,
        generation: u64,
        instance_id: u64,
        title: String,
        color_key: String,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal pedalboard item belongs to an old session generation".into());
        }
        let body =
            mackes_pipedal_connector::SetPedalboardItemTitle { instance_id, title, color_key };
        body.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "setPedalboardItemTitle".into(),
            reply_to,
            body: Some(body),
        })
        .map_err(|error| error.to_string())
    }

    /// Queue a confirmed pedalboard item title request.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, readiness, generation, or title validation fails.
    pub fn apply_set_pedalboard_item_title(
        &mut self,
        generation: u64,
        instance_id: u64,
        title: String,
        color_key: String,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<(), String> {
        if !confirmed {
            return Err("PiPedal pedalboard item title requires explicit confirmation".into());
        }
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for title delivery".into());
        }
        let frame = self.prepare_set_pedalboard_item_title(
            generation,
            instance_id,
            title,
            color_key,
            reply_to,
        )?;
        self.session.enqueue_control(generation, frame)
    }

    /// Prepare a source-compatible scalar `PiPedal` preview-volume request.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale generation, non-finite volume, or encoding failure.
    pub fn prepare_preview_volume(
        &self,
        generation: u64,
        input: bool,
        volume_db: f32,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal volume preview belongs to an old session generation".into());
        }
        if !volume_db.is_finite() {
            return Err("PiPedal volume preview is not finite".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: if input { "previewInputVolume" } else { "previewOutputVolume" }.into(),
            reply_to,
            body: Some(volume_db),
        })
        .map_err(|error| error.to_string())
    }

    /// Queue a source-compatible scalar `PiPedal` preview-volume request.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready or validation fails.
    pub fn apply_preview_volume(
        &mut self,
        generation: u64,
        input: bool,
        volume_db: f32,
        reply_to: Option<u64>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for volume preview".into());
        }
        let frame = self.prepare_preview_volume(generation, input, volume_db, reply_to)?;
        self.session.enqueue_control(generation, frame)
    }

    /// Prepare a generation-checked `PiPedal` MIDI listener request.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale generation, invalid handle, or encoding failure.
    pub fn prepare_listen_for_midi_event(
        &self,
        generation: u64,
        handle: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal MIDI listener belongs to an old session generation".into());
        }
        let body = mackes_pipedal_connector::ListenForMidiEvent { handle };
        body.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "listenForMidiEvent".into(),
            reply_to,
            body: Some(body),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepare a generation-checked `PiPedal` MIDI listener cancellation request.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale generation, invalid handle, or encoding failure.
    pub fn prepare_cancel_listen_for_midi_event(
        &self,
        generation: u64,
        handle: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() || handle == 0 {
            return Err("PiPedal MIDI listener cancellation is invalid".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "cancelListenForMidiEvent".into(),
            reply_to,
            body: Some(handle),
        })
        .map_err(|error| error.to_string())
    }

    /// Queue a generation-checked `PiPedal` MIDI listener start request.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready or validation fails.
    pub fn apply_listen_for_midi_event(
        &mut self,
        generation: u64,
        handle: u64,
        reply_to: Option<u64>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for MIDI listening".into());
        }
        let frame = self.prepare_listen_for_midi_event(generation, handle, reply_to)?;
        self.session.enqueue_control(generation, frame)
    }

    /// Queue a generation-checked `PiPedal` MIDI listener cancellation request.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready or validation fails.
    pub fn apply_cancel_listen_for_midi_event(
        &mut self,
        generation: u64,
        handle: u64,
        reply_to: Option<u64>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for MIDI listening".into());
        }
        let frame = self.prepare_cancel_listen_for_midi_event(generation, handle, reply_to)?;
        self.session.enqueue_control(generation, frame)
    }

    /// Prepare a generation-checked `PiPedal` patch-property monitor request.
    ///
    /// # Errors
    ///
    /// Returns an error for stale generation, invalid identities/URI, or encoding failure.
    pub fn prepare_monitor_patch_property(
        &self,
        generation: u64,
        instance_id: u64,
        client_handle: u64,
        property_uri: String,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal patch monitor belongs to an old session generation".into());
        }
        let body = mackes_pipedal_connector::MonitorPatchProperty {
            instance_id,
            client_handle,
            property_uri,
        };
        body.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "monitorPatchProperty".into(),
            reply_to,
            body: Some(body),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepare a generation-checked `PiPedal` patch-property monitor cancellation request.
    ///
    /// # Errors
    ///
    /// Returns an error for stale generation, invalid handle, or encoding failure.
    pub fn prepare_cancel_monitor_patch_property(
        &self,
        generation: u64,
        client_handle: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() || client_handle == 0 {
            return Err("PiPedal patch monitor cancellation is invalid".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "cancelMonitorPatchProperty".into(),
            reply_to,
            body: Some(client_handle),
        })
        .map_err(|error| error.to_string())
    }

    /// Queue a generation-checked patch-property monitor request.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready or validation fails.
    pub fn apply_monitor_patch_property(
        &mut self,
        generation: u64,
        instance_id: u64,
        client_handle: u64,
        property_uri: String,
        reply_to: Option<u64>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for patch monitoring".into());
        }
        let frame = self.prepare_monitor_patch_property(
            generation,
            instance_id,
            client_handle,
            property_uri,
            reply_to,
        )?;
        self.session.enqueue_control(generation, frame)
    }

    /// Queue a generation-checked patch-property monitor cancellation request.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready or validation fails.
    pub fn apply_cancel_monitor_patch_property(
        &mut self,
        generation: u64,
        client_handle: u64,
        reply_to: Option<u64>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for patch monitoring".into());
        }
        let frame =
            self.prepare_cancel_monitor_patch_property(generation, client_handle, reply_to)?;
        self.session.enqueue_control(generation, frame)
    }

    /// Prepares a generation-checked, read-only system MIDI bindings query.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested generation is stale or encoding fails.
    pub fn prepare_get_system_midi_bindings(
        &self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal system MIDI query belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "getSystemMidiBindings".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a read-only system MIDI bindings query.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready, the generation is stale, or queue admission fails.
    pub fn query_system_midi_bindings(
        &mut self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for system MIDI queries".into());
        }
        let frame = self.prepare_get_system_midi_bindings(generation, reply_to)?;
        self.session.enqueue(generation, frame)
    }

    /// Prepares a read-only qualified preset catalog query for the current session.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested generation is stale or the request cannot be encoded.
    pub fn prepare_get_presets(
        &self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal preset query belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "getPresets".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a read-only qualified preset catalog query.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready, the generation is stale, or queue
    /// admission fails.
    pub fn query_presets(&mut self, generation: u64, reply_to: Option<u64>) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for preset queries".into());
        }
        let frame = self.prepare_get_presets(generation, reply_to)?;
        self.session.enqueue(generation, frame)
    }

    /// Prepares a generation-checked ALSA sequencer configuration query.
    ///
    /// # Errors
    ///
    /// Returns an error when the generation is stale or encoding fails.
    pub fn prepare_get_alsa_sequencer_configuration(
        &self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal ALSA sequencer query belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "getAlsaSequencerConfiguration".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a generation-checked ALSA sequencer ports query.
    ///
    /// # Errors
    ///
    /// Returns an error when the generation is stale or encoding fails.
    pub fn prepare_get_alsa_sequencer_ports(
        &self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err(
                "PiPedal ALSA sequencer ports query belongs to an old session generation".into()
            );
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "getAlsaSequencerPorts".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked preset-index update.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, index validation, or encoding fails.
    pub fn prepare_update_presets(
        &self,
        generation: u64,
        index: mackes_pipedal_connector::PresetIndex,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal preset updates require explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal preset update belongs to an old session generation".into());
        }
        let value = serde_json::to_value(&index).map_err(|error| error.to_string())?;
        mackes_pipedal_connector::decode_preset_index(Some(value))?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "updatePresets".into(),
            reply_to,
            body: Some(index),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked bank move.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, identity validation, or encoding fails.
    pub fn prepare_move_bank(
        &self,
        generation: u64,
        move_range: mackes_pipedal_connector::FromTo,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal bank moves require explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal bank move belongs to an old session generation".into());
        }
        move_range.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "moveBank".into(),
            reply_to,
            body: Some(move_range),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked bank-open command.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, identity validation, or encoding fails.
    pub fn prepare_open_bank(
        &self,
        generation: u64,
        bank_id: i64,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal bank opens require explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal bank open belongs to an old session generation".into());
        }
        if bank_id < 0 {
            return Err("PiPedal bank identity is invalid".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "openBank".into(),
            reply_to,
            body: Some(bank_id),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked save-bank-as command.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, payload validation, or encoding fails.
    pub fn prepare_save_bank_as(
        &self,
        generation: u64,
        request: mackes_pipedal_connector::SaveBankAs,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal save-bank-as requires explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal save-bank-as belongs to an old session generation".into());
        }
        request.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "saveBankAs".into(),
            reply_to,
            body: Some(request),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked plugin-preset copy.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, payload validation, or encoding fails.
    pub fn prepare_copy_plugin_preset(
        &self,
        generation: u64,
        request: mackes_pipedal_connector::CopyPluginPreset,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal plugin-preset copies require explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal plugin-preset copy belongs to an old session generation".into());
        }
        request.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "copyPluginPreset".into(),
            reply_to,
            body: Some(request),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked bank-item deletion.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, identity validation, or encoding fails.
    pub fn prepare_delete_bank_item(
        &self,
        generation: u64,
        instance_id: i64,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal bank-item deletion requires explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal bank-item deletion belongs to an old session generation".into());
        }
        if instance_id < 0 {
            return Err("PiPedal bank-item identity is invalid".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "deleteBankItem".into(),
            reply_to,
            body: Some(instance_id),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked preset-item deletion list.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, payload validation, or encoding fails.
    pub fn prepare_delete_preset_items(
        &self,
        generation: u64,
        request: mackes_pipedal_connector::DeletePresetItems,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal preset deletion requires explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal preset deletion belongs to an old session generation".into());
        }
        request.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "deletePresetItems".into(),
            reply_to,
            body: Some(request),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked ALSA sequencer configuration update.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, configuration validation, or encoding fails.
    pub fn prepare_set_alsa_sequencer_configuration(
        &self,
        generation: u64,
        configuration: mackes_pipedal_connector::AlsaSequencerConfiguration,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal ALSA sequencer changes require explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal ALSA sequencer change belongs to an old session generation".into());
        }
        let value = serde_json::to_value(&configuration).map_err(|error| error.to_string())?;
        mackes_pipedal_connector::decode_alsa_sequencer_configuration(Some(value))?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "setAlsaSequencerConfiguration".into(),
            reply_to,
            body: Some(configuration),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked onboarding-state change.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, or encoding fails.
    pub fn prepare_set_onboarding(
        &self,
        generation: u64,
        value: bool,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal onboarding changes require explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal onboarding change belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "setOnboarding".into(),
            reply_to,
            body: Some(value),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked preset rename.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, payload validation, or encoding fails.
    pub fn prepare_rename_preset_item(
        &self,
        generation: u64,
        request: mackes_pipedal_connector::RenamePresetItem,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal preset renames require explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal preset rename belongs to an old session generation".into());
        }
        request.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "renamePresetItem".into(),
            reply_to,
            body: Some(request),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked preset copy.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, payload validation, or encoding fails.
    pub fn prepare_copy_preset(
        &self,
        generation: u64,
        request: mackes_pipedal_connector::CopyPreset,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal preset copies require explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal preset copy belongs to an old session generation".into());
        }
        request.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "copyPreset".into(),
            reply_to,
            body: Some(request),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked next-bank command.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, or encoding fails.
    pub fn prepare_next_bank(
        &self,
        generation: u64,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal bank navigation requires explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal next-bank command belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "nextBank".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked previous-bank command.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, or encoding fails.
    pub fn prepare_previous_bank(
        &self,
        generation: u64,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal bank navigation requires explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal previous-bank command belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "previousBank".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked bank rename.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, payload validation, or encoding fails.
    pub fn prepare_rename_bank(
        &self,
        generation: u64,
        rename: mackes_pipedal_connector::RenameBank,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal bank renames require explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal bank rename belongs to an old session generation".into());
        }
        rename.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "renameBank".into(),
            reply_to,
            body: Some(rename),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked next-preset command.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, or encoding fails.
    pub fn prepare_next_preset(
        &self,
        generation: u64,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        self.prepare_preset_navigation(generation, reply_to, confirmed, "nextPreset")
    }

    /// Prepares a confirmed, generation-checked previous-preset command.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, or encoding fails.
    pub fn prepare_previous_preset(
        &self,
        generation: u64,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        self.prepare_preset_navigation(generation, reply_to, confirmed, "previousPreset")
    }

    fn prepare_preset_navigation(
        &self,
        generation: u64,
        reply_to: Option<u64>,
        confirmed: bool,
        message: &'static str,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal preset navigation requires explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal preset navigation belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: message.into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked current-preset load request.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, preset identity, or request encoding is invalid.
    pub fn prepare_load_preset(
        &self,
        generation: u64,
        preset_instance_id: i64,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal preset load requires explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal preset load belongs to an old session generation".into());
        }
        let body = mackes_pipedal_connector::LoadPreset(preset_instance_id);
        body.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "loadPreset".into(),
            reply_to,
            body: Some(body),
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a confirmed current-preset load request.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready, confirmation or generation is invalid,
    /// or queue admission fails.
    pub fn apply_load_preset(
        &mut self,
        generation: u64,
        preset_instance_id: i64,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for preset load".into());
        }
        let frame =
            self.prepare_load_preset(generation, preset_instance_id, reply_to, confirmed)?;
        self.session.enqueue_control(generation, frame)
    }

    /// Prepares a confirmed, generation-checked current-preset save-as request.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, or source-backed fields are invalid.
    pub fn prepare_save_current_preset_as(
        &self,
        generation: u64,
        bank_instance_id: i64,
        name: String,
        save_after_instance_id: i64,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal preset save-as requires explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal preset save-as belongs to an old session generation".into());
        }
        let body = mackes_pipedal_connector::SaveCurrentPresetAs {
            bank_instance_id,
            name,
            save_after_instance_id,
        };
        body.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "saveCurrentPresetAs".into(),
            reply_to,
            body: Some(body),
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a confirmed current-preset save-as request.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready, confirmation/generation validation fails,
    /// or queue admission fails.
    pub fn apply_save_current_preset_as(
        &mut self,
        generation: u64,
        bank_instance_id: i64,
        name: String,
        save_after_instance_id: i64,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for preset save-as".into());
        }
        let frame = self.prepare_save_current_preset_as(
            generation,
            bank_instance_id,
            name,
            save_after_instance_id,
            reply_to,
            confirmed,
        )?;
        self.session.enqueue_control(generation, frame)
    }

    /// Prepares a confirmed, generation-checked plugin-preset save-as request.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, or source-backed fields are invalid.
    pub fn prepare_save_plugin_preset_as(
        &self,
        generation: u64,
        instance_id: i64,
        name: String,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal plugin-preset save-as requires explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal plugin-preset save-as belongs to an old session generation".into());
        }
        let body = mackes_pipedal_connector::SavePluginPresetAs { instance_id, name };
        body.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "savePluginPresetAs".into(),
            reply_to,
            body: Some(body),
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a confirmed plugin-preset save-as request.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready, confirmation/generation validation fails,
    /// or queue admission fails.
    pub fn apply_save_plugin_preset_as(
        &mut self,
        generation: u64,
        instance_id: i64,
        name: String,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for plugin-preset save-as".into());
        }
        let frame =
            self.prepare_save_plugin_preset_as(generation, instance_id, name, reply_to, confirmed)?;
        self.session.enqueue_control(generation, frame)
    }

    /// Prepares a generation-checked, read-only favorites query.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested generation is stale or the request cannot be encoded.
    pub fn prepare_get_favorites(
        &self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal favorites query belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "getFavorites".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a read-only favorites query.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready, the generation is stale, or queue
    /// admission fails.
    pub fn query_favorites(
        &mut self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for favorites queries".into());
        }
        let frame = self.prepare_get_favorites(generation, reply_to)?;
        self.session.enqueue(generation, frame)
    }

    /// Prepares a generation-checked favorites replacement.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale generation, invalid URI map, or encoding failure.
    pub fn prepare_set_favorites(
        &self,
        generation: u64,
        favorites: std::collections::BTreeMap<String, bool>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal favorites write belongs to an old session generation".into());
        }
        mackes_pipedal_connector::validate_favorites(&favorites)?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "setFavorites".into(),
            reply_to: None,
            body: Some(favorites),
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a generation-checked favorites replacement.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready, the generation is stale, validation
    /// fails, or queue admission fails.
    pub fn apply_set_favorites(
        &mut self,
        generation: u64,
        favorites: std::collections::BTreeMap<String, bool>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for favorites writes".into());
        }
        let frame = self.prepare_set_favorites(generation, favorites)?;
        self.session.enqueue_control(generation, frame)
    }

    /// Prepares a generation-checked, read-only update-status query.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested generation is stale or the request cannot be encoded.
    pub fn prepare_get_update_status(
        &self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal update-status query belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "getUpdateStatus".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a read-only update-status query.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready, the generation is stale, or queue admission fails.
    pub fn query_update_status(
        &mut self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for update status".into());
        }
        let frame = self.prepare_get_update_status(generation, reply_to)?;
        self.session.enqueue(generation, frame)
    }

    /// Prepares a generation-checked, read-only Wi-Fi availability query.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested generation is stale or encoding fails.
    pub fn prepare_get_has_wifi(
        &self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal Wi-Fi query belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "getHasWifi".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a generation-checked Wi-Fi channel query with the source scalar country body.
    ///
    /// # Errors
    ///
    /// Returns an error when the generation or country code is invalid, or encoding fails.
    pub fn prepare_get_wifi_channels(
        &self,
        generation: u64,
        country: String,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal Wi-Fi channel query belongs to an old session generation".into());
        }
        if country.trim().is_empty() || country.len() > 16 {
            return Err("PiPedal Wi-Fi country code is invalid".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "getWifiChannels".into(),
            reply_to,
            body: Some(country),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a generation-checked plugin-preset query with a bounded URI body.
    ///
    /// # Errors
    ///
    /// Returns an error when the generation or URI is invalid, or encoding fails.
    pub fn prepare_get_plugin_presets(
        &self,
        generation: u64,
        plugin_uri: String,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal plugin-preset query belongs to an old session generation".into());
        }
        if plugin_uri.trim().is_empty() || plugin_uri.len() > 512 {
            return Err("PiPedal plugin URI is invalid".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "getPluginPresets".into(),
            reply_to,
            body: Some(plugin_uri),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a generation-checked, read-only known-Wi-Fi-networks query.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested generation is stale or encoding fails.
    pub fn prepare_get_known_wifi_networks(
        &self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal known-Wi-Fi query belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "getKnownWifiNetworks".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed plugin-preset load request.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, identities, or encoding is invalid.
    pub fn prepare_load_plugin_preset(
        &self,
        generation: u64,
        plugin_instance_id: u64,
        preset_instance_id: u64,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal plugin-preset load requires explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal plugin-preset load belongs to an old session generation".into());
        }
        let body =
            mackes_pipedal_connector::LoadPluginPreset { plugin_instance_id, preset_instance_id };
        body.validate()?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "loadPluginPreset".into(),
            reply_to,
            body: Some(body),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a generation-checked, read-only JACK-settings query.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested generation is stale or encoding fails.
    pub fn prepare_get_jack_server_settings(
        &self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal JACK-settings query belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "getJackServerSettings".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a read-only JACK-settings query for a ready session.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready, the generation is stale, or queue
    /// admission fails.
    pub fn query_jack_server_settings(
        &mut self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for JACK-settings queries".into());
        }
        let frame = self.prepare_get_jack_server_settings(generation, reply_to)?;
        self.session.enqueue(generation, frame)
    }

    /// Prepares a generation-checked, read-only JACK channel-selection query.
    ///
    /// # Errors
    ///
    /// Returns an error when the generation is stale or encoding fails.
    pub fn prepare_get_jack_settings(
        &self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal JACK-settings query belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "getJackSettings".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a read-only JACK channel-selection query for a ready session.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready, the generation is stale, or queue
    /// admission fails.
    pub fn query_jack_settings(
        &mut self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for JACK-settings queries".into());
        }
        let frame = self.prepare_get_jack_settings(generation, reply_to)?;
        self.session.enqueue(generation, frame)
    }

    /// Prepares a generation-checked, read-only JACK configuration query.
    ///
    /// # Errors
    ///
    /// Returns an error when the generation is stale or encoding fails.
    pub fn prepare_get_jack_configuration(
        &self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err(
                "PiPedal JACK configuration query belongs to an old session generation".into()
            );
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "getJackConfiguration".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a read-only JACK configuration query for a ready session.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready, the generation is stale, or queue
    /// admission fails.
    pub fn query_jack_configuration(
        &mut self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for JACK configuration queries".into());
        }
        let frame = self.prepare_get_jack_configuration(generation, reply_to)?;
        self.session.enqueue(generation, frame)
    }

    /// Prepares a confirmed, generation-checked JACK server settings mutation.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, validation, or encoding fails.
    pub fn prepare_set_jack_server_settings(
        &self,
        generation: u64,
        settings: mackes_pipedal_connector::JackServerSettings,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal JACK server settings require explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal JACK server settings belong to an old session generation".into());
        }
        let value = serde_json::to_value(&settings).map_err(|error| error.to_string())?;
        mackes_pipedal_connector::decode_jack_server_settings(Some(value))?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "setJackServerSettings".into(),
            reply_to,
            body: Some(settings),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked update request.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, URL bounds, or encoding fails.
    pub fn prepare_update_now(
        &self,
        generation: u64,
        update_url: String,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal updates require explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal update belongs to an old session generation".into());
        }
        if update_url.is_empty() || update_url.len() > 1024 {
            return Err("PiPedal update URL is invalid or excessive".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "updateNow".into(),
            reply_to,
            body: Some(update_url),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked status-monitor visibility change.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, or encoding fails.
    pub fn prepare_set_show_status_monitor(
        &self,
        generation: u64,
        show: bool,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal status-monitor changes require explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal status-monitor change belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "setShowStatusMonitor".into(),
            reply_to,
            body: Some(show),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed, generation-checked plugin-preset catalog update.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, catalog validation, or encoding fails.
    pub fn prepare_update_plugin_presets(
        &self,
        generation: u64,
        catalog: mackes_pipedal_connector::PluginUiPresets,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal plugin-preset updates require explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal plugin-preset update belongs to an old session generation".into());
        }
        let value = serde_json::to_value(&catalog).map_err(|error| error.to_string())?;
        mackes_pipedal_connector::decode_plugin_presets(Some(value))?;
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "updatePluginPresets".into(),
            reply_to,
            body: Some(catalog),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a confirmed CPU-governor settings request with a bounded scalar body.
    ///
    /// # Errors
    ///
    /// Returns an error when confirmation, generation, or governor input is invalid.
    pub fn prepare_set_governor_settings(
        &self,
        generation: u64,
        governor: String,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<Vec<u8>, String> {
        if !confirmed {
            return Err("PiPedal governor settings require explicit confirmation".into());
        }
        if generation != self.session.generation() {
            return Err("PiPedal governor settings belong to an old session generation".into());
        }
        if governor.trim().is_empty() || governor.len() > 64 {
            return Err("PiPedal governor setting is invalid".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request {
            message: "setGovernorSettings".into(),
            reply_to,
            body: Some(governor),
        })
        .map_err(|error| error.to_string())
    }

    /// Prepares a generation-checked, read-only CPU-governor query.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested generation is stale or encoding fails.
    pub fn prepare_get_governor_settings(
        &self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal governor query belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "getGovernorSettings".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a read-only CPU-governor query.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready, the generation is stale, or queue
    /// admission fails.
    pub fn query_governor_settings(
        &mut self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for governor queries".into());
        }
        let frame = self.prepare_get_governor_settings(generation, reply_to)?;
        self.session.enqueue(generation, frame)
    }

    /// Prepares a generation-checked, read-only status-monitor query.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested generation is stale or encoding fails.
    pub fn prepare_get_show_status_monitor(
        &self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal status-monitor query belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "getShowStatusMonitor".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a read-only status-monitor query.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready, the generation is stale, or queue
    /// admission fails.
    pub fn query_show_status_monitor(
        &mut self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for status-monitor queries".into());
        }
        let frame = self.prepare_get_show_status_monitor(generation, reply_to)?;
        self.session.enqueue(generation, frame)
    }

    /// Prepares a generation-checked, read-only Wi-Fi regulatory-domain query.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested generation is stale or encoding fails.
    pub fn prepare_get_wifi_regulatory_domains(
        &self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal Wi-Fi domain query belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "getWifiRegulatoryDomains".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a read-only Wi-Fi regulatory-domain query.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready, the generation is stale, or queue
    /// admission fails.
    pub fn query_wifi_regulatory_domains(
        &mut self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for Wi-Fi domain queries".into());
        }
        let frame = self.prepare_get_wifi_regulatory_domains(generation, reply_to)?;
        self.session.enqueue(generation, frame)
    }

    /// Prepares a generation-checked, read-only current-bank query.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested generation is stale or the request cannot be encoded.
    pub fn prepare_get_bank_index(
        &self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<Vec<u8>, String> {
        if generation != self.session.generation() {
            return Err("PiPedal bank query belongs to an old session generation".into());
        }
        mackes_pipedal_connector::encode_request(&mackes_pipedal_connector::Request::<()> {
            message: "getBankIndex".into(),
            reply_to,
            body: None,
        })
        .map_err(|error| error.to_string())
    }

    /// Queues a read-only current-bank query.
    ///
    /// # Errors
    ///
    /// Returns an error when the session is not ready, the generation is stale, or queue
    /// admission fails.
    pub fn query_bank_index(
        &mut self,
        generation: u64,
        reply_to: Option<u64>,
    ) -> Result<(), String> {
        if !self.session.is_ready() {
            return Err("PiPedal session is not ready for bank queries".into());
        }
        let frame = self.prepare_get_bank_index(generation, reply_to)?;
        self.session.enqueue(generation, frame)
    }

    /// Applies a control only when a fresh catalog value is available, retaining that value for
    /// an immediately safe undo. The journal is updated only after queue admission succeeds.
    ///
    /// # Errors
    ///
    /// Returns an error when the catalog value is stale or unavailable, confirmation is absent,
    /// the session is not ready, or queue admission fails.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_set_control_with_record(
        &mut self,
        generation: u64,
        mapping: &MappingIdentity,
        instance_id: u64,
        client_id: u64,
        reply_to: Option<u64>,
        value: f32,
        confirmed: bool,
    ) -> Result<(), String> {
        let previous_value = self
            .catalog
            .find_control(&mapping.plugin_uri, &mapping.symbol)
            .and_then(|control| control.value)
            .filter(|value| value.is_finite())
            .ok_or("PiPedal fresh prior value is unavailable")?;
        self.apply_set_control(
            generation,
            mapping,
            instance_id,
            client_id,
            reply_to,
            value,
            confirmed,
        )?;
        self.record_apply(ApplyRecord {
            mapping: mapping.clone(),
            instance_id,
            previous_value,
            generation,
        })
    }

    /// Allocates a bounded reply identifier for a future correlated mutation.
    pub fn allocate_reply_id(&mut self) -> u64 {
        let id = self.next_reply_id;
        self.next_reply_id = self.next_reply_id.saturating_add(1).max(1);
        id
    }

    /// Returns the last admitted scalar mutation for daemon-owned persistence.
    #[must_use]
    pub const fn apply_record(&self) -> Option<&ApplyRecord> {
        self.apply_record.as_ref()
    }

    /// Restores a persisted journal entry only when it belongs to this session.
    ///
    /// # Errors
    ///
    /// Returns an error when the record belongs to another generation or contains a
    /// non-finite prior value.
    pub fn restore_apply_record(&mut self, record: ApplyRecord) -> Result<(), String> {
        if record.generation != self.session.generation() || !record.previous_value.is_finite() {
            return Err("PiPedal persisted undo belongs to an old or invalid session".into());
        }
        self.apply_record = Some(record);
        Ok(())
    }

    /// Records the prior value after a confirmed apply has been admitted.
    ///
    /// # Errors
    ///
    /// Rejects an invalid value or stale session generation.
    pub fn record_apply(&mut self, record: ApplyRecord) -> Result<(), String> {
        if record.generation != self.session.generation() || !record.previous_value.is_finite() {
            return Err("PiPedal apply record belongs to a stale or invalid generation".into());
        }
        self.apply_record = Some(record);
        Ok(())
    }

    /// Takes the latest apply record as an explicit restore intent.
    ///
    /// # Errors
    ///
    /// Rejects stale generations or an empty undo journal.
    pub fn undo_apply(&mut self, generation: u64) -> Result<RestoreIntent, String> {
        if generation != self.session.generation() {
            return Err("PiPedal undo belongs to an old session generation".into());
        }
        let record = self.apply_record.take().ok_or("no PiPedal apply is available to undo")?;
        Ok(RestoreIntent {
            mapping: record.mapping,
            instance_id: record.instance_id,
            value: record.previous_value,
            generation: record.generation,
        })
    }

    /// Queues an explicit restore intent after fresh validation.
    ///
    /// # Errors
    ///
    /// Rejects missing confirmation, stale generation, a non-ready session, or an invalid target.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_restore_intent(
        &mut self,
        intent: &RestoreIntent,
        client_id: u64,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<(), String> {
        if !confirmed {
            return Err("PiPedal restore requires explicit confirmation".into());
        }
        if !self.session.is_ready() || intent.generation != self.session.generation() {
            return Err("PiPedal restore session is stale or not ready".into());
        }
        if reply_to.is_some() && self.expected_replies.len() >= 128 {
            return Err("PiPedal reply tracking capacity is full".into());
        }
        let frame = self.prepare_set_control(
            intent.generation,
            &intent.mapping,
            intent.instance_id,
            client_id,
            reply_to,
            intent.value,
        )?;
        self.session.enqueue_control(intent.generation, frame)?;
        if let Some(reply_to) = reply_to {
            self.expected_replies.push(reply_to);
        }
        Ok(())
    }

    /// Validates and queues restoration of the latest apply atomically.
    ///
    /// # Errors
    ///
    /// Returns an error without consuming the journal when restore validation or queue admission
    /// fails.
    pub fn restore_last_apply(
        &mut self,
        generation: u64,
        client_id: u64,
        reply_to: Option<u64>,
        confirmed: bool,
    ) -> Result<(), String> {
        if generation != self.session.generation() {
            return Err("PiPedal undo belongs to an old session generation".into());
        }
        let intent = self.apply_record.as_ref().ok_or("no PiPedal apply is available to undo")?;
        let intent = RestoreIntent {
            mapping: intent.mapping.clone(),
            instance_id: intent.instance_id,
            value: intent.previous_value,
            generation: intent.generation,
        };
        self.apply_restore_intent(&intent, client_id, reply_to, confirmed)?;
        self.apply_record = None;
        Ok(())
    }

    /// Returns a bounded status snapshot.
    #[must_use]
    pub fn health(&self) -> Health {
        Health {
            phase: self.session.phase(),
            generation: self.session.generation(),
            pending_requests: self.session.pending_requests(),
            pending_commands: self.pending.len(),
            last_error: self.last_error,
        }
    }

    /// Converts the worker health into the versioned local IPC projection.
    #[must_use]
    pub fn ipc_status(&self) -> mackes_ipc::PiPedalStatus {
        let health = self.health();
        mackes_ipc::PiPedalStatus {
            phase: match health.phase {
                SessionPhase::Disconnected => mackes_ipc::PiPedalPhase::Disconnected,
                SessionPhase::Connected => mackes_ipc::PiPedalPhase::Connected,
                SessionPhase::Identified => mackes_ipc::PiPedalPhase::Identified,
                SessionPhase::LoadingCatalog => mackes_ipc::PiPedalPhase::LoadingCatalog,
                SessionPhase::Ready => mackes_ipc::PiPedalPhase::Ready,
            },
            generation: health.generation,
            pending_requests: u16::try_from(health.pending_requests.min(u16::MAX as usize))
                .unwrap_or(u16::MAX),
            timeouts: 0,
            transport_failures: u64::from(health.last_error.is_some()),
            successful_reads: self.successful_reads,
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn decode_catalog(
    value: &serde_json::Value,
) -> Result<mackes_pipedal_connector::PluginCatalog, String> {
    mackes_pipedal_connector::decode_plugin_catalog(Some(value.clone()))?;
    let entries = value.as_array().ok_or("PiPedal plugins body is not an array")?;
    let mut catalog = mackes_pipedal_connector::PluginCatalog::default();
    for (entry_index, entry) in
        entries.iter().take(mackes_pipedal_connector::MAX_CATALOG_CONTROLS).enumerate()
    {
        let object = entry.as_object().ok_or("PiPedal plugin entry is not an object")?;
        let uri = object
            .get("uri")
            .or_else(|| object.get("pluginUri"))
            .and_then(serde_json::Value::as_str)
            .ok_or("PiPedal plugin URI is missing")?;
        let instance_id = object
            .get("instanceId")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(|| u64::try_from(entry_index + 1).unwrap_or(u64::MAX));
        let name = object
            .get("name")
            .or_else(|| object.get("label"))
            .and_then(serde_json::Value::as_str)
            .ok_or("PiPedal plugin name is missing")?;
        catalog.targets.push(mackes_pipedal_connector::PluginTarget {
            uri: uri.to_owned(),
            instance_id,
            name: name.to_owned(),
        });
        if let Some(controls) = object.get("controls").and_then(serde_json::Value::as_array) {
            for control in controls.iter().take(
                mackes_pipedal_connector::MAX_CATALOG_CONTROLS
                    .saturating_sub(catalog.controls.len()),
            ) {
                let control =
                    control.as_object().ok_or("PiPedal control entry is not an object")?;
                let symbol = control
                    .get("symbol")
                    .and_then(serde_json::Value::as_str)
                    .ok_or("PiPedal control symbol is missing")?;
                let min_value = control
                    .get("minValue")
                    .or_else(|| control.get("min_value"))
                    .and_then(serde_json::Value::as_f64)
                    .map(|value| f64_to_f32(value, "PiPedal minimum is not finite"))
                    .transpose()?
                    .ok_or("PiPedal control minimum is missing")?;
                let max_value = control
                    .get("maxValue")
                    .or_else(|| control.get("max_value"))
                    .and_then(serde_json::Value::as_f64)
                    .map(|value| f64_to_f32(value, "PiPedal maximum is not finite"))
                    .transpose()?
                    .ok_or("PiPedal control maximum is missing")?;
                let value = control
                    .get("value")
                    .or_else(|| control.get("default_value"))
                    .and_then(serde_json::Value::as_f64)
                    .map(|value| f64_to_f32(value, "PiPedal value is not finite"))
                    .transpose()?;
                catalog.controls.push(mackes_pipedal_connector::ControlDescriptor {
                    plugin_uri: uri.to_owned(),
                    symbol: symbol.to_owned(),
                    label: control
                        .get("label")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(symbol)
                        .to_owned(),
                    min_value,
                    max_value,
                    value,
                    writable: control
                        .get("writable")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or_else(|| {
                            control
                                .get("is_input")
                                .and_then(serde_json::Value::as_bool)
                                .unwrap_or(false)
                        }),
                });
            }
        }
    }
    if catalog.targets.is_empty() {
        return Err("PiPedal plugin catalog is empty".into());
    }
    Ok(catalog)
}

#[allow(clippy::cast_possible_truncation)]
fn f64_to_f32(value: f64, error: &str) -> Result<f32, String> {
    value.is_finite().then_some(value as f32).ok_or_else(|| error.to_owned())
}

fn ingest_current_pedalboard(
    catalog: &mut mackes_pipedal_connector::PluginCatalog,
    runtime_targets: &mut Vec<(u64, String)>,
    value: &serde_json::Value,
) -> Result<(), String> {
    let snapshot = mackes_pipedal_connector::decode_current_pedalboard(Some(value.clone()))?;
    runtime_targets.clear();
    for item in snapshot.items {
        let uri = item.uri;
        let instance_id = item.instance_id;
        if runtime_targets.len() < mackes_pipedal_connector::MAX_CATALOG_CONTROLS {
            runtime_targets.push((instance_id, uri.clone()));
        }
        for control_value in item.control_values {
            let symbol = control_value.key;
            let value = f64_to_f32(control_value.value, "PiPedal control value is not finite")?;
            if let Some(control) = catalog
                .controls
                .iter_mut()
                .find(|control| control.plugin_uri == uri && control.symbol == symbol)
            {
                control.value = Some(value);
            }
        }
    }
    Ok(())
}

fn ingest_control_changed(
    catalog: &mut mackes_pipedal_connector::PluginCatalog,
    runtime_targets: &[(u64, String)],
    value: &serde_json::Value,
) -> Result<(String, String, f32), String> {
    let instance_id = value
        .get("instanceId")
        .and_then(serde_json::Value::as_u64)
        .ok_or("PiPedal control event instance ID is missing")?;
    let symbol = value
        .get("symbol")
        .and_then(serde_json::Value::as_str)
        .ok_or("PiPedal control event symbol is missing")?;
    let new_value = value
        .get("value")
        .and_then(serde_json::Value::as_f64)
        .map(|v| f64_to_f32(v, "PiPedal control event value is not finite"))
        .transpose()?
        .ok_or("PiPedal control event value is missing")?;
    let uri = runtime_targets
        .iter()
        .find(|(candidate, _)| *candidate == instance_id)
        .map(|(_, uri)| uri)
        .ok_or("PiPedal control event instance is unknown")?;
    let control = catalog
        .controls
        .iter_mut()
        .find(|control| control.plugin_uri == *uri && control.symbol == symbol)
        .ok_or("PiPedal control event target is unknown")?;
    control.value = Some(new_value);
    Ok((uri.clone(), symbol.to_owned(), new_value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_is_bounded() {
        let mut worker = Worker::new(Session::default());
        for _ in 0..MAX_PENDING_COMMANDS {
            worker.enqueue(Command::Start).expect("capacity");
        }
        assert_eq!(worker.enqueue(Command::Start), Err(Command::Start));
        assert_eq!(worker.health().pending_commands, MAX_PENDING_COMMANDS);
    }

    #[test]
    fn reconnect_advances_generation_and_resets_phase() {
        let mut worker = Worker::new(Session::default());
        worker.enqueue(Command::Reconnect).expect("capacity");
        worker.process(&mut NoopTransport, 1);
        assert_eq!(worker.health().generation, 1);
        assert_eq!(worker.health().phase, SessionPhase::Disconnected);
    }

    #[test]
    fn favorites_projection_is_retained_and_cleared_on_reconnect() {
        let mut worker = Worker::default();
        worker.enqueue(Command::Start).expect("start queue");
        worker.process(&mut NoopTransport, 1);
        worker.accept_frame(br#"[{"message":"ehlo"},1]"#).expect("hello");
        worker
            .accept_frame(br#"[{"message":"version"},{"serverVersion":"PiPedal v2"}]"#)
            .expect("version");
        assert_eq!(worker.version().map(|value| value.server_version.as_str()), Some("PiPedal v2"));
        worker
            .accept_frame(br#"[{"message":"getSystemMidiBindings"},[{"symbol":"gain","channel":1,"bindingType":0,"note":0,"control":7,"minControlValue":0,"maxControlValue":127,"minValue":0.0,"maxValue":1.0,"rotaryScale":1.0,"linearControlType":0,"switchControlType":0}]]"#)
            .expect("system MIDI bindings");
        assert_eq!(worker.system_midi_bindings().len(), 1);
        worker
            .accept_frame(br#"[{"message":"getFavorites"},{"urn:eq":true,"urn:delay":false}]"#)
            .expect("favorites");
        assert_eq!(worker.favorites().get("urn:eq"), Some(&true));
        worker.enqueue(Command::Reconnect).expect("reconnect queue");
        worker.process(&mut NoopTransport, 1);
        assert!(worker.favorites().is_empty());
        assert!(worker.system_midi_bindings().is_empty());
        assert!(worker.version().is_none());
    }

    #[test]
    fn favorites_write_preserves_source_map_shape_and_generation() {
        let worker = Worker::default();
        let favorites = std::collections::BTreeMap::from([("urn:eq".to_owned(), true)]);
        let frame = worker.prepare_set_favorites(0, favorites).expect("favorites write");
        let text = String::from_utf8_lossy(&frame);
        assert!(text.contains("setFavorites"));
        assert!(text.contains("urn:eq"));
        assert!(worker.prepare_set_favorites(1, std::collections::BTreeMap::new()).is_err());
        let invalid = std::collections::BTreeMap::from([(String::new(), true)]);
        assert!(worker.prepare_set_favorites(0, invalid).is_err());
    }

    #[test]
    fn pump_returns_available_frames_without_waiting() {
        let mut worker = Worker::new(Session::default());
        let mut transport = QueuedTransport { frames: vec![vec![1, 2, 3]] };
        assert_eq!(worker.pump(&mut transport, 1, 1).expect("poll"), vec![vec![1, 2, 3]]);
        assert_eq!(worker.health().phase, SessionPhase::Disconnected);
    }

    #[test]
    fn ipc_status_projects_session_phase_and_generation() {
        let mut worker = Worker::new(Session::default());
        worker.enqueue(Command::Reconnect).expect("capacity");
        worker.process(&mut NoopTransport, 1);
        assert_eq!(worker.ipc_status().phase, mackes_ipc::PiPedalPhase::Disconnected);
        assert_eq!(worker.ipc_status().generation, 1);
    }

    #[test]
    fn runtime_instance_resolution_is_unique_or_explicit_and_fail_closed() {
        let mut worker = Worker::default();
        worker.catalog.targets = vec![
            mackes_pipedal_connector::PluginTarget {
                uri: "urn:eq".into(),
                instance_id: 7,
                name: "EQ one".into(),
            },
            mackes_pipedal_connector::PluginTarget {
                uri: "urn:reverb".into(),
                instance_id: 8,
                name: "Reverb".into(),
            },
        ];
        let mapping = MappingIdentity {
            physical_control_id: "knob-r3-c4".into(),
            plugin_uri: "urn:reverb".into(),
            symbol: "mix".into(),
            scope: None,
        };
        assert_eq!(worker.resolve_instance_id(&mapping), Ok(8));
        let explicit = MappingIdentity {
            scope: Some("instance:7".into()),
            plugin_uri: "urn:eq".into(),
            ..mapping.clone()
        };
        assert_eq!(worker.resolve_instance_id(&explicit), Ok(7));
        let ambiguous = MappingIdentity { plugin_uri: "urn:eq".into(), ..mapping };
        worker.catalog.targets.push(mackes_pipedal_connector::PluginTarget {
            uri: "urn:eq".into(),
            instance_id: 9,
            name: "EQ two".into(),
        });
        assert!(worker.resolve_instance_id(&ambiguous).is_err());
        let wrong_scope = MappingIdentity { scope: Some("instance:8".into()), ..explicit };
        assert!(worker.resolve_instance_id(&wrong_scope).is_err());
    }

    #[test]
    fn start_queues_the_bounded_qualified_handshake() {
        let mut worker = Worker::default();
        worker.enqueue(Command::Start).expect("capacity");
        worker.process(&mut NoopTransport, 1);
        assert_eq!(worker.health().phase, SessionPhase::Connected);
        assert_eq!(worker.health().pending_commands, 0);
    }

    #[test]
    fn system_midi_binding_replacement_is_generation_checked_and_typed() {
        let worker = Worker::default();
        let binding = mackes_pipedal_connector::MidiBinding {
            symbol: "gain".into(),
            channel: -1,
            binding_type: 0,
            note: 0,
            control: 74,
            min_control_value: 0,
            max_control_value: 127,
            min_value: -1.0,
            max_value: 1.0,
            rotary_scale: 1.0,
            linear_control_type: 0,
            switch_control_type: 0,
        };
        let body = mackes_pipedal_connector::SystemMidiBindings { bindings: vec![binding] };
        let frame = worker
            .prepare_set_system_midi_bindings(0, body, Some(41))
            .expect("typed binding replacement");
        assert!(String::from_utf8_lossy(&frame).contains("setSystemMidiBindings"));
        assert!(worker
            .prepare_set_system_midi_bindings(
                1,
                mackes_pipedal_connector::SystemMidiBindings { bindings: Vec::new() },
                None,
            )
            .is_err());
    }

    #[test]
    fn pedalboard_enable_request_is_typed_and_rejects_zero_identity() {
        let worker = Worker::default();
        let frame = worker
            .prepare_set_pedalboard_item_enable(0, 99, 7, false, Some(42))
            .expect("typed bypass request");
        assert!(String::from_utf8_lossy(&frame).contains("setPedalboardItemEnable"));
        assert!(worker.prepare_set_pedalboard_item_enable(0, 99, 0, true, None).is_err());
    }

    #[test]
    fn preview_volume_request_preserves_source_scalar_body_and_bounds_values() {
        let worker = Worker::default();
        let input = worker.prepare_preview_volume(0, true, -6.5, Some(43)).expect("input preview");
        let output = worker.prepare_preview_volume(0, false, 3.0, None).expect("output preview");
        assert!(String::from_utf8_lossy(&input).contains("previewInputVolume"));
        assert!(String::from_utf8_lossy(&input).contains("-6.5"));
        assert!(String::from_utf8_lossy(&output).contains("previewOutputVolume"));
        assert!(worker.prepare_preview_volume(0, true, f32::NAN, None).is_err());
        assert!(worker.prepare_preview_volume(1, true, 0.0, None).is_err());
    }

    #[test]
    fn patch_monitor_requests_preserve_source_object_and_scalar_shapes() {
        let worker = Worker::default();
        let start = worker
            .prepare_monitor_patch_property(0, 7, 4, "urn:property".into(), Some(44))
            .expect("monitor");
        let cancel = worker.prepare_cancel_monitor_patch_property(0, 4, None).expect("cancel");
        let start_text = String::from_utf8_lossy(&start);
        assert!(start_text.contains("monitorPatchProperty"));
        assert!(start_text.contains("clientHandle"));
        assert!(start_text.contains("propertyUri"));
        assert!(String::from_utf8_lossy(&cancel).contains("cancelMonitorPatchProperty"));
        assert!(worker.prepare_monitor_patch_property(0, 0, 4, "urn:x".into(), None).is_err());
        assert!(worker.prepare_cancel_monitor_patch_property(1, 4, None).is_err());
    }

    #[test]
    fn system_midi_query_is_read_only_and_generation_checked() {
        let worker = Worker::default();
        let frame = worker.prepare_get_system_midi_bindings(0, Some(45)).expect("MIDI query");
        assert!(String::from_utf8_lossy(&frame).contains("getSystemMidiBindings"));
        assert!(worker.prepare_get_system_midi_bindings(1, None).is_err());
    }

    #[test]
    fn preset_catalog_query_is_read_only_and_generation_checked() {
        let worker = Worker::default();
        let frame = worker.prepare_get_presets(0, Some(77)).expect("preset query");
        assert!(String::from_utf8_lossy(&frame).contains("getPresets"));
        assert!(worker.prepare_get_presets(1, None).is_err());
    }

    #[test]
    fn preset_save_as_preserves_source_payload_and_confirmation() {
        let worker = Worker::default();
        let frame = worker
            .prepare_save_current_preset_as(0, 7, "New preset".into(), 12, Some(78), true)
            .expect("preset save-as");
        let text = String::from_utf8_lossy(&frame);
        assert!(text.contains("saveCurrentPresetAs"));
        assert!(text.contains("bankInstanceId"));
        assert!(text.contains("saveAfterInstanceId"));
        assert!(worker
            .prepare_save_current_preset_as(0, 7, "New preset".into(), 12, None, false)
            .is_err());
        assert!(worker
            .prepare_save_current_preset_as(1, 7, "New preset".into(), 12, None, true)
            .is_err());
        assert!(worker
            .prepare_save_current_preset_as(0, 7, String::new(), 12, None, true)
            .is_err());
    }

    #[test]
    fn plugin_preset_save_as_preserves_source_payload_and_confirmation() {
        let worker = Worker::default();
        let frame = worker
            .prepare_save_plugin_preset_as(0, 137, "Warm gain".into(), Some(79), true)
            .expect("plugin preset save-as");
        let text = String::from_utf8_lossy(&frame);
        assert!(text.contains("savePluginPresetAs"));
        assert!(text.contains("instanceId"));
        assert!(text.contains("Warm gain"));
        assert!(worker
            .prepare_save_plugin_preset_as(0, 137, "Warm gain".into(), None, false)
            .is_err());
        assert!(worker
            .prepare_save_plugin_preset_as(0, 0, "Warm gain".into(), None, true)
            .is_err());
    }

    #[test]
    fn plugins_response_populates_validated_catalog() {
        let mut worker = Worker::default();
        worker.enqueue(Command::Start).expect("capacity");
        worker.process(&mut NoopTransport, 1);
        worker.accept_frame(br#"[{"message":"ehlo"},1]"#).expect("hello");
        worker
            .accept_frame(br#"[{"message":"version"},{"serverVersion":"PiPedal v2"}]"#)
            .expect("version");
        worker.accept_frame(br#"[{"message":"plugins"},[{"uri":"urn:eq","instanceId":7,"name":"EQ","controls":[{"symbol":"gain","minValue":-12,"maxValue":12,"value":0,"writable":true}]}]]"#).expect("catalog");
        assert_eq!(worker.catalog().targets.len(), 1);
        assert_eq!(worker.catalog().controls[0].symbol, "gain");
    }

    #[test]
    fn physical_control_bridge_enforces_pickup_and_queues_one_native_range_write() {
        let mut worker = Worker::default();
        worker.enqueue(Command::Start).expect("capacity");
        worker.process(&mut NoopTransport, 1);
        worker.accept_frame(br#"[{"message":"ehlo"},1]"#).expect("hello");
        worker
            .accept_frame(br#"[{"message":"version"},{"serverVersion":"PiPedal v2"}]"#)
            .expect("version");
        worker.accept_frame(br#"[{"message":"plugins"},[{"uri":"urn:eq","instanceId":7,"name":"EQ","controls":[{"symbol":"gain","minValue":-12,"maxValue":12,"value":0,"writable":true}]}]]"#).expect("catalog");
        worker.accept_frame(br#"[{"message":"currentPedalboard"},{"items":[{"instanceId":7,"uri":"urn:eq","controlValues":[{"key":"gain","value":0}]}]}]"#).expect("pedalboard");
        worker.accept_frame(br#"[{"message":"getSystemMidiBindings"},[]]"#).expect("midi bindings");
        while worker.session.pop().is_some() {}
        let mapping = MappingIdentity {
            physical_control_id: "knob-r3-c4".into(),
            plugin_uri: "urn:eq".into(),
            symbol: "gain".into(),
            scope: None,
        };
        worker.reconcile_pickup_targets(std::slice::from_ref(&mapping));
        let generation = worker.health().generation;
        assert_eq!(worker.apply_physical_control(generation, &mapping, 0, 0.01), Ok(false));
        assert_eq!(worker.apply_physical_control(generation, &mapping, 64, 100.0), Ok(true));
        let frame = worker.session.pop().expect("queued setControl");
        assert!(frame.len() > 14, "setControl frame should contain an encoded request");
        assert!(worker.session.pop().is_none());
    }

    #[test]
    fn pedalboard_snapshot_and_control_event_refresh_runtime_value() {
        let mut catalog = decode_catalog(&serde_json::json!([{
            "uri": "urn:eq",
            "name": "EQ",
            "controls": [{
                "symbol": "gain",
                "min_value": -12,
                "max_value": 12,
                "default_value": 0,
                "is_input": true
            }]
        }]))
        .expect("catalog");
        let mut targets = Vec::new();
        ingest_current_pedalboard(
            &mut catalog,
            &mut targets,
            &serde_json::json!({
                "items": [{
                    "instanceId": 137,
                    "uri": "urn:eq",
                    "controlValues": [{"key": "gain", "value": 0}]
                }]
            }),
        )
        .expect("pedalboard");
        ingest_control_changed(
            &mut catalog,
            &targets,
            &serde_json::json!({"instanceId": 137, "symbol": "gain", "value": 2}),
        )
        .expect("event");
        assert_eq!(catalog.find_control("urn:eq", "gain").and_then(|c| c.value), Some(2.0));

        ingest_current_pedalboard(&mut catalog, &mut targets, &serde_json::json!({"items": []}))
            .expect("replacement snapshot");
        assert!(ingest_control_changed(
            &mut catalog,
            &targets,
            &serde_json::json!({"instanceId": 137, "symbol": "gain", "value": 3}),
        )
        .is_err());
    }

    #[test]
    fn current_pedalboard_acceptance_uses_bounded_connector_projection() {
        let mut catalog = decode_catalog(&serde_json::json!([{
            "uri": "urn:eq", "name": "EQ", "controls": []
        }]))
        .expect("catalog");
        let mut targets = Vec::new();
        let malformed = serde_json::json!({
            "items": [
                {"instanceId": 4, "uri": "urn:eq"},
                {"instanceId": 4, "uri": "urn:eq"}
            ]
        });
        assert!(ingest_current_pedalboard(&mut catalog, &mut targets, &malformed).is_err());
        assert!(targets.is_empty());
    }

    #[test]
    fn control_event_burst_converges_without_feedback_and_reconnect_rejects_stale_instance() {
        let mut worker = Worker::default();
        worker.enqueue(Command::Start).expect("start");
        worker.process(&mut NoopTransport, 1);
        worker.accept_frame(br#"[{"message":"ehlo"},1]"#).expect("hello");
        worker
            .accept_frame(br#"[{"message":"version"},{"serverVersion":"PiPedal v2"}]"#)
            .expect("version");
        worker.accept_frame(br#"[{"message":"plugins"},[{"uri":"urn:eq","name":"EQ","controls":[{"symbol":"gain","min_value":-40,"max_value":30,"default_value":0,"is_input":true}]}]]"#).expect("catalog");
        worker.accept_frame(br#"[{"message":"currentPedalboard"},{"items":[{"instanceId":137,"uri":"urn:eq","controlValues":[{"key":"gain","value":0}]}]}]"#).expect("pedalboard");
        worker.accept_frame(br#"[{"message":"getSystemMidiBindings"},[]]"#).expect("ready");
        let mapping = MappingIdentity {
            physical_control_id: "knob-r3-c4".into(),
            plugin_uri: "urn:eq".into(),
            symbol: "gain".into(),
            scope: None,
        };
        worker.reconcile_pickup_targets(std::slice::from_ref(&mapping));
        assert!(worker.observe_pickup("knob-r3-c4", 0, 0.0, 0.01).expect("initial pickup"));
        let pending_before = worker.health().pending_requests;

        for value in 0..10_000_u32 {
            let frame = serde_json::to_vec(&serde_json::json!([
                {"message": "onControlChanged"},
                {"clientId": 1, "instanceId": 137, "symbol": "gain", "value": value % 31}
            ]))
            .expect("event");
            worker.accept_frame(&frame).expect("burst event");
        }
        assert_eq!(worker.health().pending_requests, pending_before);
        assert_eq!(
            worker.catalog().find_control("urn:eq", "gain").and_then(|control| control.value),
            Some(17.0)
        );
        assert!(!worker.observe_pickup("knob-r3-c4", 0, 0.0, 0.01).expect("rearmed miss"));
        assert!(worker.observe_pickup("knob-r3-c4", 0, 17.0, 0.01).expect("reacquire"));

        worker.enqueue(Command::Reconnect).expect("reconnect");
        worker.process(&mut NoopTransport, 1);
        assert!(worker.observe_pickup("knob-r3-c4", 1, 17.0, 0.01).is_err());
        assert!(worker
            .accept_frame(br#"[{"message":"onControlChanged"},{"clientId":1,"instanceId":137,"symbol":"gain","value":1}]"#)
            .is_err());
    }

    #[test]
    fn resolution_outcome_is_strictly_serializable() {
        let outcome = ResolutionOutcome {
            physical_control_id: "knob-r3-c4".into(),
            plugin_uri: "urn:eq".into(),
            symbol: "gain".into(),
            instance_id: None,
            state: ResolutionState::Unavailable,
            detail: "target is unavailable".into(),
        };
        let encoded = serde_json::to_vec(&outcome).expect("encode");
        assert_eq!(serde_json::from_slice::<ResolutionOutcome>(&encoded).expect("decode"), outcome);
        assert!(serde_json::from_slice::<ResolutionOutcome>(
            br#"{"physical_control_id":"x","state":"resolved","detail":"ok","extra":true}"#
        )
        .is_err());
    }

    #[test]
    fn unknown_correlated_reply_is_rejected() {
        let mut worker = Worker::default();
        worker.enqueue(Command::Start).expect("capacity");
        worker.process(&mut NoopTransport, 1);
        assert_eq!(
            worker.accept_frame(br#"[{"reply":99,"message":"ehlo"},1]"#),
            Err(TransportError::Protocol)
        );
    }

    #[test]
    fn default_endpoint_is_the_qualified_ipv6_loopback() {
        assert_eq!(default_endpoint(), "[::1]:8080".parse().expect("socket address"));
    }

    #[test]
    fn supported_operations_are_nonempty_and_unique() {
        let operations = supported_operations();
        assert!(!operations.is_empty());
        for (index, operation) in operations.iter().enumerate() {
            assert!(!operations[..index].contains(operation));
        }
    }

    #[test]
    fn apply_journal_returns_current_generation_restore_intent() {
        let mut worker = Worker::default();
        let mapping = MappingIdentity {
            physical_control_id: "knob-r3-c4".into(),
            plugin_uri: "urn:eq".into(),
            symbol: "gain".into(),
            scope: None,
        };
        worker
            .record_apply(ApplyRecord {
                mapping: mapping.clone(),
                instance_id: 7,
                previous_value: -3.0,
                generation: 0,
            })
            .expect("record");
        let intent = worker.undo_apply(0).expect("undo");
        assert_eq!(intent.mapping, mapping);
        assert!((intent.value + 3.0).abs() < f32::EPSILON);
        assert!(worker.undo_apply(0).is_err());
    }

    #[test]
    fn persisted_apply_record_is_generation_checked_and_round_trips() {
        let record = ApplyRecord {
            mapping: MappingIdentity {
                physical_control_id: "knob-r3-c4".into(),
                plugin_uri: "urn:eq".into(),
                symbol: "gain".into(),
                scope: Some("main".into()),
            },
            instance_id: 7,
            previous_value: -3.0,
            generation: 0,
        };
        let encoded = serde_json::to_vec(&record).expect("encode journal");
        let decoded: ApplyRecord = serde_json::from_slice(&encoded).expect("decode journal");
        let mut worker = Worker::default();
        worker.restore_apply_record(decoded).expect("restore journal");
        assert!(worker.apply_record().is_some());
        assert!(worker
            .restore_apply_record(ApplyRecord { previous_value: f32::NAN, generation: 0, ..record })
            .is_err());
    }

    #[test]
    fn apply_with_record_requires_a_fresh_catalog_value() {
        let mut worker = Worker::default();
        let mapping = MappingIdentity {
            physical_control_id: "knob-r3-c4".into(),
            plugin_uri: "urn:eq".into(),
            symbol: "gain".into(),
            scope: None,
        };
        let error = worker
            .apply_set_control_with_record(0, &mapping, 7, 2, None, 0.5, true)
            .expect_err("an empty catalog cannot provide an undo value");
        assert_eq!(error, "PiPedal fresh prior value is unavailable");
        assert!(worker.undo_apply(0).is_err());
    }

    #[test]
    fn restore_with_reply_id_tracks_the_correlated_response() {
        let mut worker = Worker::default();
        worker.session.connect().expect("connect");
        worker.session.accept("ehlo").expect("ehlo");
        worker.session.accept("version").expect("version");
        worker.session.accept("getSystemMidiBindings").expect("ready");
        worker.catalog.targets.push(mackes_pipedal_connector::PluginTarget {
            uri: "urn:eq".into(),
            instance_id: 7,
            name: "EQ".into(),
        });
        worker.catalog.controls.push(mackes_pipedal_connector::ControlDescriptor {
            plugin_uri: "urn:eq".into(),
            symbol: "gain".into(),
            label: "Gain".into(),
            min_value: -12.0,
            max_value: 12.0,
            value: Some(0.0),
            writable: true,
        });
        let intent = RestoreIntent {
            mapping: MappingIdentity {
                physical_control_id: "knob-r3-c4".into(),
                plugin_uri: "urn:eq".into(),
                symbol: "gain".into(),
                scope: None,
            },
            instance_id: 7,
            value: -3.0,
            generation: 0,
        };
        worker.apply_restore_intent(&intent, 2, Some(91), true).expect("restore queues");
        assert!(worker.accept_frame(br#"[{"reply":91,"message":"setControl"}]"#).is_ok());
    }

    #[test]
    fn governor_settings_request_is_confirmed_bounded_and_scalar() {
        let worker = Worker::default();
        let frame = worker
            .prepare_set_governor_settings(0, "performance".into(), Some(45), true)
            .expect("governor settings queues");
        let text = String::from_utf8_lossy(&frame);
        assert!(text.contains("setGovernorSettings"));
        assert!(text.contains("performance"));
        assert!(worker
            .prepare_set_governor_settings(0, "performance".into(), None, false)
            .is_err());
        assert!(worker.prepare_set_governor_settings(0, String::new(), None, true).is_err());
        assert!(worker.prepare_set_governor_settings(1, "performance".into(), None, true).is_err());
    }

    #[test]
    fn jack_server_settings_query_is_generation_checked_and_read_only() {
        let worker = Worker::default();
        let frame =
            worker.prepare_get_jack_server_settings(0, Some(46)).expect("JACK query encodes");
        let text = String::from_utf8_lossy(&frame);
        assert!(text.contains("getJackServerSettings"));
        assert!(worker.prepare_get_jack_server_settings(1, None).is_err());
    }

    #[test]
    fn status_monitor_query_is_generation_checked_and_boolean() {
        let worker = Worker::default();
        let frame =
            worker.prepare_get_show_status_monitor(0, Some(46)).expect("status monitor query");
        assert!(String::from_utf8_lossy(&frame).contains("getShowStatusMonitor"));
        assert!(worker.prepare_get_show_status_monitor(1, None).is_err());
    }

    #[test]
    fn wifi_regulatory_domain_query_is_generation_checked() {
        let worker = Worker::default();
        let frame =
            worker.prepare_get_wifi_regulatory_domains(0, Some(47)).expect("Wi-Fi domain query");
        assert!(String::from_utf8_lossy(&frame).contains("getWifiRegulatoryDomains"));
        assert!(worker.prepare_get_wifi_regulatory_domains(1, None).is_err());
    }

    #[test]
    fn failed_startup_queue_admission_does_not_reserve_reply_id() {
        let mut worker = Worker::default();
        worker.session.connect().expect("connect");
        for _ in 0..mackes_pipedal_connector::MAX_PENDING_REQUESTS {
            worker.session.enqueue(0, vec![0]).expect("fill queue");
        }
        worker.queue_startup_request();
        assert!(worker.expected_replies.is_empty());
        assert_eq!(worker.startup_next, 0);
        assert_eq!(worker.last_error, Some(TransportError::Protocol));
    }

    struct NoopTransport;
    impl Transport for NoopTransport {
        fn send(&mut self, _: &[u8]) -> Result<(), TransportError> {
            Ok(())
        }
        fn receive(&mut self) -> Result<Option<Vec<u8>>, TransportError> {
            Ok(None)
        }
    }

    struct QueuedTransport {
        frames: Vec<Vec<u8>>,
    }

    impl Transport for QueuedTransport {
        fn send(&mut self, _: &[u8]) -> Result<(), TransportError> {
            Ok(())
        }
        fn receive(&mut self) -> Result<Option<Vec<u8>>, TransportError> {
            Ok(self.frames.pop())
        }
    }
}
