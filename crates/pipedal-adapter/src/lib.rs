//! Bounded daemon-boundary orchestration for a `PiPedal` connector session.
//!
//! This crate deliberately contains no socket or MIDI implementation. The daemon boundary
//! supplies a transport, while this layer owns admission, lifecycle generation, and health
//! projection so network work cannot run on the MIDI dispatch path.

use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

use mackes_pipedal_connector::{
    decode_server_frame, encode_client_text, encode_request, startup_requests, Request, Session,
    SessionPhase, TextAssembler, Transport, TransportError, MAX_FRAME_BYTES,
};

/// Maximum number of commands admitted before the worker must make progress.
pub const MAX_PENDING_COMMANDS: usize = 128;
/// Maximum mapping outcomes retained in one resolution report.
pub const MAX_RESOLUTION_OUTCOMES: usize = 128;

/// Returns the default qualified `PiPedal` control endpoint.
#[must_use]
pub const fn default_endpoint() -> SocketAddr {
    SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::LOCALHOST), 8080)
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
            .write_all(b"GET /pipedal HTTP/1.1\r\nHost: 127.0.0.1:8080\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: bWFja2VzLXBpcGVkYWw=\r\n\r\n")
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
        self.expected_replies.push(reply_to);
        let request =
            Request { message: (*message).to_owned(), reply_to: Some(reply_to), body: None::<()> };
        match encode_request(&request) {
            Ok(frame) => {
                let generation = self.session.generation();
                if self.session.enqueue(generation, frame).is_ok() {
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
        if header.message == "ehlo" {
            self.pipedal_client_id = body.as_ref().and_then(serde_json::Value::as_u64);
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

    /// Returns the numeric client identifier assigned by `PiPedal` during `hello`.
    #[must_use]
    pub const fn pipedal_client_id(&self) -> Option<u64> {
        self.pipedal_client_id
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
                let (state, detail) = match self.catalog.resolve_mapping(&connector_mapping) {
                    Ok(_) => {
                        (ResolutionState::Resolved, "target is available and writable".to_owned())
                    }
                    Err(error) if error.contains("ambiguous") => {
                        (ResolutionState::Ambiguous, error)
                    }
                    Err(error) if error.contains("read-only") => (ResolutionState::ReadOnly, error),
                    Err(error) => (ResolutionState::Unavailable, error),
                };
                ResolutionOutcome {
                    physical_control_id: mapping.physical_control_id.clone(),
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

    /// Allocates a bounded reply identifier for a future correlated mutation.
    pub fn allocate_reply_id(&mut self) -> u64 {
        let id = self.next_reply_id;
        self.next_reply_id = self.next_reply_id.saturating_add(1).max(1);
        id
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
        let frame = self.prepare_set_control(
            intent.generation,
            &intent.mapping,
            intent.instance_id,
            client_id,
            reply_to,
            intent.value,
        )?;
        self.session.enqueue_control(intent.generation, frame)
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
    let items = value
        .get("items")
        .and_then(serde_json::Value::as_array)
        .ok_or("PiPedal pedalboard items are missing")?;
    runtime_targets.clear();
    for item in items {
        let object = item.as_object().ok_or("PiPedal pedalboard item is not an object")?;
        let uri = object
            .get("uri")
            .and_then(serde_json::Value::as_str)
            .ok_or("PiPedal pedalboard URI is missing")?;
        let instance_id = object
            .get("instanceId")
            .and_then(serde_json::Value::as_u64)
            .ok_or("PiPedal pedalboard instance ID is missing")?;
        if runtime_targets.len() < mackes_pipedal_connector::MAX_CATALOG_CONTROLS {
            runtime_targets.push((instance_id, uri.to_owned()));
        }
        let values = object
            .get("controlValues")
            .and_then(serde_json::Value::as_array)
            .map_or(&[][..], |values| values.as_slice());
        for value in values {
            let value_object = value.as_object().ok_or("PiPedal control value is not an object")?;
            let symbol = value_object
                .get("key")
                .and_then(serde_json::Value::as_str)
                .ok_or("PiPedal control value key is missing")?;
            let value = value_object
                .get("value")
                .and_then(serde_json::Value::as_f64)
                .map(|v| f64_to_f32(v, "PiPedal control value is not finite"))
                .transpose()?
                .ok_or("PiPedal control value is missing")?;
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
    fn start_queues_the_bounded_qualified_handshake() {
        let mut worker = Worker::default();
        worker.enqueue(Command::Start).expect("capacity");
        worker.process(&mut NoopTransport, 1);
        assert_eq!(worker.health().phase, SessionPhase::Connected);
        assert_eq!(worker.health().pending_commands, 0);
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
        worker.arm_pickup(&mapping).expect("arm pickup");
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
