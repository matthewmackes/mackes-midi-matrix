#![allow(clippy::derive_partial_eq_without_eq)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::match_same_arms)]
#![allow(missing_docs)]

//! Typed, transport-independent `PiPedal` WebSocket protocol contracts.
//!
//! This crate deliberately does not open sockets. The daemon transport can use these
//! bounded message types without placing network I/O on the MIDI dispatch path.

use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

/// `PiPedal`'s array-framed WebSocket request envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Request<T> {
    /// Message name.
    pub message: String,
    /// Correlation identifier, when a response is expected.
    #[serde(rename = "replyTo", skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<u64>,
    /// Optional request body.
    pub body: Option<T>,
}

/// Maximum encoded PiPedal control frame accepted by the connector.
pub const MAX_FRAME_BYTES: usize = 4 * 1_048_576;
/// Maximum reusable PiPedal mappings in one connector configuration.
pub const MAX_MAPPINGS: usize = 128;
/// Maximum pickup states retained during reconciliation.
pub const MAX_RECONCILIATION_STATES: usize = MAX_MAPPINGS;
/// Maximum discovered plugin controls in one catalog snapshot.
pub const MAX_CATALOG_CONTROLS: usize = 4_096;
/// Maximum preset entries accepted from one PiPedal catalog response.
pub const MAX_PRESET_ENTRIES: usize = 256;
/// Maximum bank entries accepted from one PiPedal catalog response.
pub const MAX_BANK_ENTRIES: usize = 128;
/// Maximum current-pedalboard items accepted from one PiPedal state response.
pub const MAX_PEDALBOARD_ITEMS: usize = 256;
/// Maximum plugin entries accepted from one PiPedal `plugins` response.
pub const MAX_PLUGIN_ENTRIES: usize = 256;
/// Maximum child classes retained in one plugin-class node.
pub const MAX_PLUGIN_CLASS_CHILDREN: usize = 256;
/// Maximum favorite identities accepted from one PiPedal response.
pub const MAX_FAVORITES: usize = 512;
/// Maximum length of a PiPedal version string accepted at handshake.
pub const MAX_VERSION_TEXT: usize = 256;
/// Maximum CPU-governor identifier accepted from PiPedal.
pub const MAX_GOVERNOR_TEXT: usize = 64;
/// Maximum Wi-Fi regulatory-domain entries accepted from PiPedal.
pub const MAX_WIFI_REGULATORY_DOMAINS: usize = 256;
/// Maximum known Wi-Fi network names retained from one PiPedal response.
pub const MAX_KNOWN_WIFI_NETWORKS: usize = 256;
/// Maximum Wi-Fi channel selectors accepted in one response.
pub const MAX_WIFI_CHANNELS: usize = 256;
/// Maximum ALSA devices accepted in one PiPedal response.
pub const MAX_ALSA_DEVICES: usize = 256;
/// Maximum JACK status error/governor text length.
pub const MAX_JACK_STATUS_TEXT: usize = 256;
/// Maximum device-name text length in JACK server settings.
pub const MAX_JACK_DEVICE_TEXT: usize = 256;
/// Maximum system MIDI bindings accepted in one PiPedal update.
pub const MAX_SYSTEM_MIDI_BINDINGS: usize = 128;
/// Maximum requests waiting for the PiPedal transport worker.
pub const MAX_PENDING_REQUESTS: usize = 64;
/// Maximum encoded request bytes waiting for the PiPedal transport worker.
pub const MAX_PENDING_REQUEST_BYTES: usize = 4 * MAX_FRAME_BYTES;

/// Transport-facing error categories kept independent of socket libraries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransportError {
    /// The peer closed or the connection was otherwise lost.
    Disconnected,
    /// The transport exceeded its bounded deadline.
    Timeout,
    /// A frame or protocol contract was invalid.
    Protocol,
}

/// Minimal transport boundary for the daemon-owned PiPedal worker.
pub trait Transport {
    /// Send one already-encoded client frame.
    fn send(&mut self, frame: &[u8]) -> Result<(), TransportError>;
    /// Receive one complete server frame, if available without blocking MIDI dispatch.
    fn receive(&mut self) -> Result<Option<Vec<u8>>, TransportError>;
}

/// Bounded handoff queue between MIDI/control callers and the PiPedal transport worker.
#[derive(Debug, Default)]
pub struct RequestQueue {
    pending: VecDeque<Vec<u8>>,
    pending_bytes: usize,
    rejected: u64,
}

/// Connector-owned session state used to reject stale work after reconnect.
#[derive(Debug)]
pub struct Session {
    phase: SessionPhase,
    generation: u64,
    queue: RequestQueue,
    timeouts: u64,
    transport_failures: u64,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            phase: SessionPhase::Disconnected,
            generation: 0,
            queue: RequestQueue::default(),
            timeouts: 0,
            transport_failures: 0,
        }
    }
}

impl Session {
    /// Mark a newly opened WebSocket as ready for the hello request.
    pub fn connect(&mut self) -> Result<(), String> {
        if self.phase != SessionPhase::Disconnected {
            return Err("PiPedal session is already connected".into());
        }
        self.phase = SessionPhase::Connected;
        Ok(())
    }
    /// Current handshake phase.
    #[must_use]
    pub const fn phase(&self) -> SessionPhase {
        self.phase
    }
    /// Current reconnect generation.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }
    /// Whether the PiPedal catalog and system bindings are ready for delivery.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self.phase, SessionPhase::Ready)
    }
    /// Advance the handshake state.
    pub fn accept(&mut self, message: &str) -> Result<SessionPhase, String> {
        self.phase = self.phase.accept(message)?;
        Ok(self.phase)
    }
    /// Reset state and invalidate queued work after socket loss.
    pub fn reset(&mut self) {
        self.phase = self.phase.reset();
        self.generation = self.generation.wrapping_add(1);
        self.queue = RequestQueue::default();
        self.timeouts = 0;
        self.transport_failures = 0;
    }
    /// Queue one encoded request for the current generation.
    pub fn enqueue(&mut self, generation: u64, request: Vec<u8>) -> Result<(), String> {
        if self.phase == SessionPhase::Disconnected {
            return Err("PiPedal session is disconnected".into());
        }
        if generation != self.generation {
            return Err("PiPedal request belongs to an old session generation".into());
        }
        self.queue.push(request)
    }
    /// Queue a platform control request only after PiPedal is fully ready.
    pub fn enqueue_control(&mut self, generation: u64, request: Vec<u8>) -> Result<(), String> {
        if !self.is_ready() {
            return Err("PiPedal platform is not ready for control delivery".into());
        }
        self.enqueue(generation, request)
    }
    /// Pop the next request for transport processing.
    pub fn pop(&mut self) -> Option<Vec<u8>> {
        self.queue.pop()
    }

    /// Send at most `budget` queued frames through a nonblocking transport boundary.
    ///
    /// The caller owns scheduling and socket readiness; this method only drains already
    /// encoded work and never waits for the peer. A failed frame is not silently retried.
    pub fn send_pending<T: Transport>(
        &mut self,
        transport: &mut T,
        budget: usize,
    ) -> Result<usize, TransportError> {
        let mut sent = 0;
        while sent < budget {
            let Some(frame) = self.pop() else { break };
            if let Err(error) = transport.send(&frame) {
                self.record_transport_error(error);
                return Err(error);
            }
            sent += 1;
        }
        Ok(sent)
    }

    /// Poll at most `budget` available server frames without waiting for the peer.
    pub fn receive_available<T: Transport>(
        &mut self,
        transport: &mut T,
        budget: usize,
    ) -> Result<Vec<Vec<u8>>, TransportError> {
        let mut received = Vec::new();
        while received.len() < budget {
            match transport.receive() {
                Ok(Some(frame)) => received.push(frame),
                Ok(None) => break,
                Err(error) => {
                    self.record_transport_error(error);
                    return Err(error);
                }
            }
        }
        Ok(received)
    }

    /// Poll and decode at most `budget` complete PiPedal protocol messages.
    pub fn receive_messages<T: Transport>(
        &mut self,
        transport: &mut T,
        budget: usize,
    ) -> Result<Vec<(MessageHeader, Option<serde_json::Value>)>, TransportError> {
        let frames = self.receive_available(transport, budget)?;
        frames
            .into_iter()
            .map(|frame| decode_message(&frame).map_err(|_| TransportError::Protocol))
            .collect()
    }

    /// Number of requests currently waiting for transport processing.
    #[must_use]
    pub fn pending_requests(&self) -> usize {
        self.queue.len()
    }

    /// Number of encoded request bytes currently waiting for transport processing.
    #[must_use]
    pub const fn pending_request_bytes(&self) -> usize {
        self.queue.pending_bytes()
    }

    /// Number of requests rejected since this session was created or reset.
    #[must_use]
    pub const fn rejected_requests(&self) -> u64 {
        self.queue.rejected_count()
    }

    /// Records a bounded transport outcome for diagnostics and recovery policy.
    pub const fn record_transport_error(&mut self, error: TransportError) {
        match error {
            TransportError::Timeout => self.timeouts = self.timeouts.saturating_add(1),
            TransportError::Disconnected | TransportError::Protocol => {
                self.transport_failures = self.transport_failures.saturating_add(1);
            }
        }
    }

    /// Number of transport timeouts recorded for this session.
    #[must_use]
    pub const fn timeouts(&self) -> u64 {
        self.timeouts
    }

    /// Number of non-timeout transport failures recorded for this session.
    #[must_use]
    pub const fn transport_failures(&self) -> u64 {
        self.transport_failures
    }
}

impl RequestQueue {
    /// Enqueue an encoded request, rejecting oversized or saturated queues.
    pub fn push(&mut self, request: Vec<u8>) -> Result<(), String> {
        if request.len() > MAX_FRAME_BYTES {
            self.rejected = self.rejected.saturating_add(1);
            return Err("PiPedal request exceeds configured frame limit".into());
        }
        if self.pending.len() >= MAX_PENDING_REQUESTS {
            self.rejected = self.rejected.saturating_add(1);
            return Err("PiPedal request queue is full".into());
        }
        if self.pending_bytes.saturating_add(request.len()) > MAX_PENDING_REQUEST_BYTES {
            self.rejected = self.rejected.saturating_add(1);
            return Err("PiPedal request queue byte budget is full".into());
        }
        self.pending_bytes = self.pending_bytes.saturating_add(request.len());
        self.pending.push_back(request);
        Ok(())
    }

    /// Remove the oldest request for transport processing.
    pub fn pop(&mut self) -> Option<Vec<u8>> {
        let request = self.pending.pop_front()?;
        self.pending_bytes = self.pending_bytes.saturating_sub(request.len());
        Some(request)
    }

    /// Number of requests awaiting transport.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Number of encoded request bytes awaiting transport processing.
    #[must_use]
    pub const fn pending_bytes(&self) -> usize {
        self.pending_bytes
    }

    /// Whether no requests await transport processing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Number of requests rejected by this queue.
    #[must_use]
    pub const fn rejected_count(&self) -> u64 {
        self.rejected
    }
}

/// PiPedal operations that the connector may expose after capability discovery.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Operation {
    /// Set one plugin parameter.
    SetControl,
    /// Preview a parameter without committing it.
    PreviewControl,
    /// Replace the current pedalboard graph.
    UpdateCurrentPedalboard,
    /// Select a pedalboard plugin for UI/editing context.
    SetSelectedPedalboardPlugin,
    /// Enable or bypass one pedalboard item.
    SetPedalboardItemEnable,
    /// Toggle whether a pedalboard item uses its plugin UI.
    SetPedalboardItemUseModUi,
    /// Rename a pedalboard item.
    SetPedalboardItemTitle,
    /// Select or write a snapshot.
    SetSnapshot,
    /// Replace snapshot definitions.
    SetSnapshots,
    /// Write system MIDI bindings.
    SetSystemMidiBindings,
    /// Set the PiPedal input mixer level.
    SetInputVolume,
    /// Set the PiPedal output mixer level.
    SetOutputVolume,
    /// Preview the PiPedal input mixer level without persisting it.
    PreviewInputVolume,
    /// Preview the PiPedal output mixer level without persisting it.
    PreviewOutputVolume,
    /// Start listening for one PiPedal MIDI event handle.
    ListenForMidiEvent,
    /// Cancel one PiPedal MIDI event listener.
    CancelListenForMidiEvent,
    /// Monitor one PiPedal patch property.
    MonitorPatchProperty,
    /// Cancel one PiPedal patch-property monitor.
    CancelMonitorPatchProperty,
    /// Query the current system MIDI bindings.
    GetSystemMidiBindings,
    /// Load a saved preset.
    LoadPreset,
    /// Save the current preset.
    SaveCurrentPreset,
    /// Save the current pedalboard as a new preset.
    SaveCurrentPresetAs,
    /// Save a plugin state as a new plugin preset.
    SavePluginPresetAs,
    /// Query ALSA devices.
    GetAlsaDevices,
    /// Query JACK status.
    GetJackStatus,
    /// Query host update status.
    GetUpdateStatus,
    /// Query whether Wi-Fi hardware is available.
    GetHasWifi,
    /// Query the URI-to-favorite flags used by the PiPedal browser.
    GetFavorites,
    /// Replace the URI-to-favorite flags used by the PiPedal browser.
    SetFavorites,
    /// Query Wi-Fi channels for a country code.
    GetWifiChannels,
    /// Query presets for a plugin URI.
    GetPluginPresets,
    /// Query the current preset index.
    GetPresets,
    /// Query the current bank index.
    GetBankIndex,
    /// Query known Wi-Fi network names.
    GetKnownWifiNetworks,
    /// Load a plugin preset into a runtime instance.
    LoadPluginPreset,
    /// Query JACK server settings.
    GetJackServerSettings,
    /// Set JACK server settings.
    SetJackServerSettings,
    /// Apply an update from a release URL.
    UpdateNow,
    /// Set the CPU governor policy.
    SetGovernorSettings,
    /// Query the CPU governor policy.
    GetGovernorSettings,
    /// Query whether PiPedal's status monitor is shown.
    GetShowStatusMonitor,
    /// Set whether PiPedal's status monitor is shown.
    SetShowStatusMonitor,
    /// Query Wi-Fi regulatory-domain labels.
    GetWifiRegulatoryDomains,
    /// Restart the PiPedal engine.
    Restart,
    /// Shut down the PiPedal host.
    Shutdown,
}

impl Operation {
    /// Return every operation currently qualified by the connector boundary.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::SetControl,
            Self::PreviewControl,
            Self::UpdateCurrentPedalboard,
            Self::SetSelectedPedalboardPlugin,
            Self::SetPedalboardItemEnable,
            Self::SetPedalboardItemUseModUi,
            Self::SetPedalboardItemTitle,
            Self::SetSnapshot,
            Self::SetSnapshots,
            Self::SetSystemMidiBindings,
            Self::SetInputVolume,
            Self::SetOutputVolume,
            Self::PreviewInputVolume,
            Self::PreviewOutputVolume,
            Self::ListenForMidiEvent,
            Self::CancelListenForMidiEvent,
            Self::MonitorPatchProperty,
            Self::CancelMonitorPatchProperty,
            Self::GetSystemMidiBindings,
            Self::LoadPreset,
            Self::SaveCurrentPreset,
            Self::SaveCurrentPresetAs,
            Self::SavePluginPresetAs,
            Self::GetAlsaDevices,
            Self::GetJackStatus,
            Self::GetUpdateStatus,
            Self::GetHasWifi,
            Self::GetFavorites,
            Self::SetFavorites,
            Self::GetWifiChannels,
            Self::GetPluginPresets,
            Self::GetPresets,
            Self::GetBankIndex,
            Self::GetKnownWifiNetworks,
            Self::LoadPluginPreset,
            Self::GetJackServerSettings,
            Self::SetJackServerSettings,
            Self::UpdateNow,
            Self::SetGovernorSettings,
            Self::GetGovernorSettings,
            Self::GetShowStatusMonitor,
            Self::SetShowStatusMonitor,
            Self::GetWifiRegulatoryDomains,
            Self::Restart,
            Self::Shutdown,
        ]
    }

    /// Wire operation name registered by PiPedal.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::SetControl => "setControl",
            Self::PreviewControl => "previewControl",
            Self::UpdateCurrentPedalboard => "updateCurrentPedalboard",
            Self::SetSelectedPedalboardPlugin => "setSelectedPedalboardPlugin",
            Self::SetPedalboardItemEnable => "setPedalboardItemEnable",
            Self::SetPedalboardItemUseModUi => "setPedalboardItemUseModUi",
            Self::SetPedalboardItemTitle => "setPedalboardItemTitle",
            Self::SetSnapshot => "setSnapshot",
            Self::SetSnapshots => "setSnapshots",
            Self::SetSystemMidiBindings => "setSystemMidiBindings",
            Self::SetInputVolume => "setInputVolume",
            Self::SetOutputVolume => "setOutputVolume",
            Self::PreviewInputVolume => "previewInputVolume",
            Self::PreviewOutputVolume => "previewOutputVolume",
            Self::ListenForMidiEvent => "listenForMidiEvent",
            Self::CancelListenForMidiEvent => "cancelListenForMidiEvent",
            Self::MonitorPatchProperty => "monitorPatchProperty",
            Self::CancelMonitorPatchProperty => "cancelMonitorPatchProperty",
            Self::GetSystemMidiBindings => "getSystemMidiBindings",
            Self::LoadPreset => "loadPreset",
            Self::SaveCurrentPreset => "saveCurrentPreset",
            Self::SaveCurrentPresetAs => "saveCurrentPresetAs",
            Self::SavePluginPresetAs => "savePluginPresetAs",
            Self::GetAlsaDevices => "getAlsaDevices",
            Self::GetJackStatus => "getJackStatus",
            Self::GetUpdateStatus => "getUpdateStatus",
            Self::GetHasWifi => "getHasWifi",
            Self::GetFavorites => "getFavorites",
            Self::SetFavorites => "setFavorites",
            Self::GetWifiChannels => "getWifiChannels",
            Self::GetPluginPresets => "getPluginPresets",
            Self::GetPresets => "getPresets",
            Self::GetBankIndex => "getBankIndex",
            Self::GetKnownWifiNetworks => "getKnownWifiNetworks",
            Self::LoadPluginPreset => "loadPluginPreset",
            Self::GetJackServerSettings => "getJackServerSettings",
            Self::SetJackServerSettings => "setJackServerSettings",
            Self::UpdateNow => "updateNow",
            Self::SetGovernorSettings => "setGovernorSettings",
            Self::GetGovernorSettings => "getGovernorSettings",
            Self::GetShowStatusMonitor => "getShowStatusMonitor",
            Self::SetShowStatusMonitor => "setShowStatusMonitor",
            Self::GetWifiRegulatoryDomains => "getWifiRegulatoryDomains",
            Self::Restart => "restart",
            Self::Shutdown => "shutdown",
        }
    }

    /// Whether this operation changes persistent or host-wide state.
    #[must_use]
    pub const fn requires_confirmation(self) -> bool {
        matches!(
            self,
            Self::UpdateCurrentPedalboard
                | Self::SetSystemMidiBindings
                | Self::SetInputVolume
                | Self::SetOutputVolume
                | Self::PreviewInputVolume
                | Self::PreviewOutputVolume
                | Self::ListenForMidiEvent
                | Self::CancelListenForMidiEvent
                | Self::MonitorPatchProperty
                | Self::CancelMonitorPatchProperty
                | Self::GetSystemMidiBindings
                | Self::LoadPreset
                | Self::SaveCurrentPreset
                | Self::SaveCurrentPresetAs
                | Self::SavePluginPresetAs
                | Self::SetFavorites
                | Self::LoadPluginPreset
                | Self::SetGovernorSettings
                | Self::SetJackServerSettings
                | Self::UpdateNow
                | Self::SetShowStatusMonitor
                | Self::Restart
                | Self::Shutdown
        )
    }

    /// Whether this operation only queries PiPedal state.
    #[must_use]
    pub const fn is_read_only(self) -> bool {
        matches!(
            self,
            Self::GetAlsaDevices
                | Self::GetJackStatus
                | Self::GetUpdateStatus
                | Self::GetHasWifi
                | Self::GetFavorites
                | Self::GetWifiChannels
                | Self::GetPluginPresets
                | Self::GetPresets
                | Self::GetBankIndex
                | Self::GetKnownWifiNetworks
                | Self::GetJackServerSettings
                | Self::GetGovernorSettings
                | Self::GetShowStatusMonitor
                | Self::GetWifiRegulatoryDomains
                | Self::GetSystemMidiBindings
        )
    }

    /// Whether the operation is safe to expose as a physical scalar/toggle mapping.
    #[must_use]
    pub const fn is_mapping_eligible(self) -> bool {
        matches!(self, Self::SetControl | Self::SetPedalboardItemEnable | Self::SetSnapshot)
    }

    /// Stable UI/diagnostic family for this operation.
    #[must_use]
    pub const fn family(self) -> &'static str {
        match self {
            Self::SetControl | Self::PreviewControl => "controls",
            Self::UpdateCurrentPedalboard
            | Self::SetSelectedPedalboardPlugin
            | Self::SetPedalboardItemEnable => "pedalboard",
            Self::SetPedalboardItemUseModUi => "pedalboard",
            Self::SetPedalboardItemTitle => "pedalboard",
            Self::SetSnapshot | Self::SetSnapshots => "snapshots",
            Self::SetSystemMidiBindings => "midi",
            Self::SetInputVolume | Self::SetOutputVolume => "audio",
            Self::PreviewInputVolume | Self::PreviewOutputVolume => "audio",
            Self::ListenForMidiEvent
            | Self::CancelListenForMidiEvent
            | Self::MonitorPatchProperty
            | Self::CancelMonitorPatchProperty => "midi",
            Self::GetSystemMidiBindings => "midi",
            Self::LoadPreset
            | Self::SaveCurrentPreset
            | Self::SaveCurrentPresetAs
            | Self::SavePluginPresetAs => "presets",
            Self::GetAlsaDevices
            | Self::GetJackStatus
            | Self::GetUpdateStatus
            | Self::GetHasWifi => "diagnostics",
            Self::GetFavorites | Self::SetFavorites => "preferences",
            Self::GetWifiChannels => "diagnostics",
            Self::GetPluginPresets => "presets",
            Self::GetPresets | Self::GetBankIndex => "presets",
            Self::GetKnownWifiNetworks => "diagnostics",
            Self::LoadPluginPreset => "presets",
            Self::GetJackServerSettings | Self::GetGovernorSettings => "diagnostics",
            Self::GetShowStatusMonitor => "monitoring",
            Self::GetWifiRegulatoryDomains => "diagnostics",
            Self::SetGovernorSettings => "host",
            Self::SetJackServerSettings => "host",
            Self::UpdateNow => "host",
            Self::SetShowStatusMonitor => "monitoring",
            Self::Restart | Self::Shutdown => "host",
        }
    }
}

/// Bounded accumulator for fragmented WebSocket text messages.
#[derive(Debug, Default)]
pub struct TextAssembler {
    pending: Vec<u8>,
}

/// A decoded server-to-client WebSocket frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerFrame {
    /// FIN bit indicating the final fragment.
    pub final_fragment: bool,
    /// WebSocket opcode (1=text, 0=continuation, 8=close, 9=ping, 10=pong).
    pub opcode: u8,
    /// Unmasked payload bytes.
    pub payload: Vec<u8>,
}

/// Encode a masked client text frame for PiPedal.
pub fn encode_client_text(payload: &[u8], mask: [u8; 4]) -> Result<Vec<u8>, String> {
    if payload.len() > MAX_FRAME_BYTES {
        return Err("PiPedal client payload exceeds configured limit".into());
    }
    let mut output = Vec::with_capacity(payload.len() + 14);
    output.push(0x81);
    match payload.len() {
        0..=125 => output.push(
            0x80 | u8::try_from(payload.len()).map_err(|_| "invalid short payload".to_string())?,
        ),
        126..=65_535 => {
            output.push(0xFE);
            output.extend_from_slice(
                &u16::try_from(payload.len())
                    .map_err(|_| "invalid extended payload".to_string())?
                    .to_be_bytes(),
            );
        }
        _ => {
            output.push(0xFF);
            output.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
    }
    output.extend_from_slice(&mask);
    output.extend(payload.iter().enumerate().map(|(i, byte)| byte ^ mask[i % 4]));
    Ok(output)
}

/// Decode one complete unmasked server WebSocket frame.
pub fn decode_server_frame(input: &[u8]) -> Result<ServerFrame, String> {
    if input.len() < 2 {
        return Err("PiPedal frame header is truncated".into());
    }
    let final_fragment = input[0] & 0x80 != 0;
    let opcode = input[0] & 0x0f;
    if input[1] & 0x80 != 0 {
        return Err("server WebSocket frame must not be masked".into());
    }
    let (length, offset) = match input[1] & 0x7f {
        n @ 0..=125 => (n as usize, 2),
        126 if input.len() >= 4 => (u16::from_be_bytes([input[2], input[3]]) as usize, 4),
        127 if input.len() >= 10 => {
            let mut bytes = [0_u8; 8];
            bytes.copy_from_slice(&input[2..10]);
            let length = usize::try_from(u64::from_be_bytes(bytes))
                .map_err(|_| "PiPedal frame length overflows platform size".to_string())?;
            (length, 10)
        }
        _ => return Err("PiPedal frame length is truncated".into()),
    };
    if length > MAX_FRAME_BYTES || input.len() != offset + length {
        return Err("PiPedal frame length is invalid or exceeds limit".into());
    }
    Ok(ServerFrame { final_fragment, opcode, payload: input[offset..].to_vec() })
}

impl TextAssembler {
    /// Add one WebSocket payload fragment, returning a complete message at `final_fragment`.
    pub fn push(
        &mut self,
        fragment: &[u8],
        final_fragment: bool,
    ) -> Result<Option<Vec<u8>>, String> {
        if self.pending.len().saturating_add(fragment.len()) > MAX_FRAME_BYTES {
            self.pending.clear();
            return Err("PiPedal fragmented message exceeds configured limit".into());
        }
        self.pending.extend_from_slice(fragment);
        if final_fragment {
            Ok(Some(std::mem::take(&mut self.pending)))
        } else {
            Ok(None)
        }
    }
}

/// Header returned by PiPedal for replies and asynchronous events.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MessageHeader {
    /// Message or event name.
    pub message: String,
    /// Correlation identifier for a request response.
    #[serde(rename = "reply")]
    pub reply_to: Option<u64>,
}

/// PiPedal error reply body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorBody {
    /// Human-readable server error.
    pub message: String,
}

/// One entry returned by PiPedal's `getPresets` response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresetIndexEntry {
    #[serde(rename = "instanceId")]
    pub instance_id: i64,
    pub name: String,
}

/// Bounded readback returned by PiPedal's `getPresets` operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresetIndex {
    #[serde(rename = "selectedInstanceId")]
    pub selected_instance_id: i64,
    #[serde(rename = "presetChanged")]
    pub preset_changed: bool,
    pub presets: Vec<PresetIndexEntry>,
}

/// Body accepted by PiPedal's `saveCurrentPresetAs` operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveCurrentPresetAs {
    #[serde(rename = "bankInstanceId")]
    pub bank_instance_id: i64,
    pub name: String,
    #[serde(rename = "saveAfterInstanceId")]
    pub save_after_instance_id: i64,
}

impl SaveCurrentPresetAs {
    /// Validate the source-backed save-as payload before transport.
    pub fn validate(&self) -> Result<(), String> {
        if self.bank_instance_id < 0
            || self.save_after_instance_id < -1
            || self.name.trim().is_empty()
            || self.name.len() > 256
        {
            return Err("PiPedal preset save-as fields are invalid".into());
        }
        Ok(())
    }
}

/// Body accepted by PiPedal's `savePluginPresetAs` operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavePluginPresetAs {
    #[serde(rename = "instanceId")]
    pub instance_id: i64,
    pub name: String,
}

impl SavePluginPresetAs {
    /// Validate the source-backed plugin-preset save-as payload.
    pub fn validate(&self) -> Result<(), String> {
        if self.instance_id <= 0 || self.name.trim().is_empty() || self.name.len() > 256 {
            return Err("PiPedal plugin-preset save-as fields are invalid".into());
        }
        Ok(())
    }
}

impl PresetIndex {
    /// Validate the installed PiPedal preset-index shape and bounds.
    pub fn validate(&self) -> Result<(), String> {
        if self.presets.len() > MAX_PRESET_ENTRIES {
            return Err("PiPedal preset catalog exceeds configured entry limit".into());
        }
        let mut ids = HashSet::with_capacity(self.presets.len());
        for preset in &self.presets {
            if preset.instance_id < 0 || preset.name.is_empty() || !ids.insert(preset.instance_id) {
                return Err("PiPedal preset catalog has invalid or duplicate entry".into());
            }
        }
        Ok(())
    }
}

/// Decode and validate a `getPresets` response body from the installed PiPedal schema.
pub fn decode_preset_index(body: Option<serde_json::Value>) -> Result<PresetIndex, String> {
    let index: PresetIndex = decode_body(body)?;
    index.validate()?;
    Ok(index)
}

/// One entry returned by PiPedal's `getBankIndex` response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BankIndexEntry {
    #[serde(rename = "instanceId")]
    pub instance_id: i64,
    pub name: String,
}

/// Bounded readback returned by PiPedal's `getBankIndex` operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BankIndex {
    #[serde(rename = "selectedBank")]
    pub selected_bank: i64,
    pub entries: Vec<BankIndexEntry>,
}

impl BankIndex {
    /// Validate the installed PiPedal bank-index shape and bounds.
    pub fn validate(&self) -> Result<(), String> {
        if self.entries.len() > MAX_BANK_ENTRIES {
            return Err("PiPedal bank catalog exceeds configured entry limit".into());
        }
        let mut ids = HashSet::with_capacity(self.entries.len());
        for bank in &self.entries {
            if bank.instance_id < 0 || bank.name.is_empty() || !ids.insert(bank.instance_id) {
                return Err("PiPedal bank catalog has invalid or duplicate entry".into());
            }
        }
        Ok(())
    }
}

/// Decode and validate a `getBankIndex` response body from the installed PiPedal schema.
pub fn decode_bank_index(body: Option<serde_json::Value>) -> Result<BankIndex, String> {
    let index: BankIndex = decode_body(body)?;
    index.validate()?;
    Ok(index)
}

/// One bounded runtime item in PiPedal's `currentPedalboard` response.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurrentPedalboardItem {
    #[serde(rename = "instanceId")]
    pub instance_id: u64,
    pub uri: String,
    #[serde(rename = "controlValues", default)]
    pub control_values: Vec<CurrentControlValue>,
}

/// Current value readback for one pedalboard control.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurrentControlValue {
    pub key: String,
    pub value: f64,
}

/// Bounded readback returned by PiPedal's `currentPedalboard` operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurrentPedalboard {
    pub items: Vec<CurrentPedalboardItem>,
}

impl CurrentPedalboard {
    /// Validate runtime identities and finite control values without inventing plugin metadata.
    pub fn validate(&self) -> Result<(), String> {
        if self.items.len() > MAX_PEDALBOARD_ITEMS {
            return Err("PiPedal pedalboard exceeds configured item limit".into());
        }
        let mut ids = HashSet::with_capacity(self.items.len());
        for item in &self.items {
            if item.instance_id == 0 || item.uri.is_empty() || !ids.insert(item.instance_id) {
                return Err("PiPedal pedalboard has invalid or duplicate runtime item".into());
            }
            for control in &item.control_values {
                if control.key.is_empty() || !control.value.is_finite() {
                    return Err("PiPedal pedalboard has invalid control readback".into());
                }
            }
        }
        Ok(())
    }
}

/// Decode and validate a `currentPedalboard` response body.
pub fn decode_current_pedalboard(
    body: Option<serde_json::Value>,
) -> Result<CurrentPedalboard, String> {
    let pedalboard: CurrentPedalboard = decode_body(body)?;
    pedalboard.validate()?;
    Ok(pedalboard)
}

/// Minimal source-backed identity envelope for one `plugins` response entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginCatalogEntry {
    pub uri: String,
    #[serde(rename = "instanceId", default)]
    pub instance_id: Option<u64>,
    pub name: String,
    #[serde(default)]
    pub controls: Vec<serde_json::Value>,
}

/// Typed control metadata used by the adapter's plugin catalog projection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginControlMetadata {
    pub symbol: String,
    #[serde(rename = "minValue", alias = "min_value")]
    pub min_value: f64,
    #[serde(rename = "maxValue", alias = "max_value")]
    pub max_value: f64,
    #[serde(default, rename = "value", alias = "default_value")]
    pub value: Option<f64>,
    #[serde(default, alias = "is_input")]
    pub writable: bool,
    #[serde(default)]
    pub label: Option<String>,
}

/// Decode bounded control metadata from one plugin catalog entry.
pub fn decode_control_metadata(
    body: Option<serde_json::Value>,
) -> Result<Vec<PluginControlMetadata>, String> {
    let controls: Vec<PluginControlMetadata> = decode_body(body)?;
    if controls.len() > MAX_CATALOG_CONTROLS {
        return Err("PiPedal control metadata exceeds configured limit".into());
    }
    let mut symbols = HashSet::with_capacity(controls.len());
    for control in &controls {
        if control.symbol.is_empty()
            || !symbols.insert(&control.symbol)
            || !control.min_value.is_finite()
            || !control.max_value.is_finite()
            || control.min_value > control.max_value
            || control.value.is_some_and(|value| !value.is_finite())
        {
            return Err("PiPedal control metadata is invalid or duplicated".into());
        }
    }
    Ok(controls)
}

/// Source-backed plugin class node used for bounded catalog readback.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginClassNode {
    pub uri: String,
    pub display_name: String,
    pub parent_uri: String,
    pub plugin_type: String,
    #[serde(default)]
    pub children: Vec<Self>,
}

fn validate_plugin_class_node(node: &PluginClassNode) -> Result<(), String> {
    if node.uri.is_empty()
        || node.display_name.is_empty()
        || node.children.len() > MAX_PLUGIN_CLASS_CHILDREN
    {
        return Err("PiPedal plugin class node is invalid or exceeds child limit".into());
    }
    for child in &node.children {
        validate_plugin_class_node(child)?;
    }
    Ok(())
}

/// Decode the `pluginClasses` tree without enabling class-driven writes.
pub fn decode_plugin_classes(body: Option<serde_json::Value>) -> Result<PluginClassNode, String> {
    let root: PluginClassNode = decode_body(body)?;
    validate_plugin_class_node(&root)?;
    Ok(root)
}

/// Decode PiPedal's source-backed `getFavorites` URI-to-flag map.
pub fn decode_favorites(
    body: Option<serde_json::Value>,
) -> Result<std::collections::BTreeMap<String, bool>, String> {
    let favorites: std::collections::BTreeMap<String, bool> = decode_body(body)?;
    if favorites.len() > MAX_FAVORITES || favorites.keys().any(String::is_empty) {
        return Err("PiPedal favorites contain invalid or excessive identities".into());
    }
    Ok(favorites)
}

/// Decode PiPedal's scalar `getGovernorSettings` response.
pub fn decode_governor_settings(body: Option<serde_json::Value>) -> Result<String, String> {
    let governor: String = decode_body(body)?;
    if governor.trim().is_empty() || governor.len() > MAX_GOVERNOR_TEXT {
        return Err("PiPedal governor setting is empty or excessive".into());
    }
    Ok(governor)
}

/// Decode PiPedal's scalar `getShowStatusMonitor` response.
pub fn decode_show_status_monitor(body: Option<serde_json::Value>) -> Result<bool, String> {
    decode_body(body)
}

/// Decode PiPedal's source-backed Wi-Fi availability flag.
pub fn decode_has_wifi(body: Option<serde_json::Value>) -> Result<bool, String> {
    decode_body(body)
}

/// Decode PiPedal's bounded Wi-Fi regulatory-domain label map.
pub fn decode_wifi_regulatory_domains(
    body: Option<serde_json::Value>,
) -> Result<std::collections::BTreeMap<String, String>, String> {
    let domains: std::collections::BTreeMap<String, String> = decode_body(body)?;
    if domains.len() > MAX_WIFI_REGULATORY_DOMAINS
        || domains.iter().any(|(key, value)| {
            key.is_empty() || key.len() > 16 || value.trim().is_empty() || value.len() > 128
        })
    {
        return Err("PiPedal Wi-Fi regulatory domains are invalid or excessive".into());
    }
    Ok(domains)
}

/// Decode PiPedal's source-backed known-network string array.
pub fn decode_known_wifi_networks(body: Option<serde_json::Value>) -> Result<Vec<String>, String> {
    let networks: Vec<String> = decode_body(body)?;
    if networks.len() > MAX_KNOWN_WIFI_NETWORKS
        || networks.iter().any(|network| network.is_empty() || network.len() > 128)
    {
        return Err("PiPedal known Wi-Fi networks are invalid or excessive".into());
    }
    Ok(networks)
}

/// One source-backed Wi-Fi channel selector.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiChannel {
    pub channel_id: String,
    pub channel_name: String,
}

/// One source-backed plugin UI preset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginUiPreset {
    pub instance_id: i64,
    pub label: String,
}

/// Source-backed plugin UI preset catalog.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginUiPresets {
    pub plugin_uri: String,
    pub presets: Vec<PluginUiPreset>,
}

/// Decode and bound a plugin-specific preset catalog.
pub fn decode_plugin_presets(body: Option<serde_json::Value>) -> Result<PluginUiPresets, String> {
    let catalog: PluginUiPresets = decode_body(body)?;
    if catalog.plugin_uri.is_empty()
        || catalog.plugin_uri.len() > 512
        || catalog.presets.len() > MAX_PRESET_ENTRIES
        || catalog.presets.iter().any(|preset| {
            preset.instance_id < 0 || preset.label.is_empty() || preset.label.len() > 256
        })
    {
        return Err("PiPedal plugin preset catalog is invalid or excessive".into());
    }
    Ok(catalog)
}

/// Decode PiPedal's bounded Wi-Fi channel selector array.
pub fn decode_wifi_channels(body: Option<serde_json::Value>) -> Result<Vec<WifiChannel>, String> {
    let channels: Vec<WifiChannel> = decode_body(body)?;
    if channels.len() > MAX_WIFI_CHANNELS
        || channels.iter().any(|channel| {
            channel.channel_id.is_empty()
                || channel.channel_id.len() > 16
                || channel.channel_name.is_empty()
                || channel.channel_name.len() > 128
        })
    {
        return Err("PiPedal Wi-Fi channels are invalid or excessive".into());
    }
    Ok(channels)
}

/// One source-backed ALSA audio-device description.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct AlsaDeviceInfo {
    pub card_id: i32,
    pub id: String,
    pub name: String,
    pub long_name: String,
    pub sample_rates: Vec<u32>,
    pub min_buffer_size: u32,
    pub max_buffer_size: u32,
    pub supports_capture: bool,
    pub supports_playback: bool,
    pub capture_busy: bool,
    pub playback_busy: bool,
}

/// Decode PiPedal's bounded ALSA audio-device array.
pub fn decode_alsa_devices(body: Option<serde_json::Value>) -> Result<Vec<AlsaDeviceInfo>, String> {
    let devices: Vec<AlsaDeviceInfo> = decode_body(body)?;
    if devices.len() > MAX_ALSA_DEVICES
        || devices.iter().any(|device| {
            device.id.is_empty()
                || device.id.len() > MAX_VERSION_TEXT
                || device.name.len() > MAX_VERSION_TEXT
                || device.long_name.len() > MAX_VERSION_TEXT
                || device.sample_rates.len() > 64
                || device.min_buffer_size > device.max_buffer_size
        })
    {
        return Err("PiPedal ALSA device inventory is invalid or excessive".into());
    }
    Ok(devices)
}

/// Source-backed JACK/audio-host status readback.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JackHostStatus {
    pub active: bool,
    pub error_message: String,
    pub restarting: bool,
    pub underruns: u64,
    pub cpu_usage: f32,
    pub ms_since_last_underrun: u64,
    pub temperaturem_c: i32,
    pub cpu_freq_max: u64,
    pub cpu_freq_min: u64,
    pub has_cpu_governor: bool,
    pub governor: String,
}

/// Decode and validate JACK/audio-host status.
pub fn decode_jack_status(body: Option<serde_json::Value>) -> Result<JackHostStatus, String> {
    let status: JackHostStatus = decode_body(body)?;
    if !status.cpu_usage.is_finite()
        || status.cpu_usage < 0.0
        || status.cpu_usage > 100.0
        || status.cpu_freq_min > status.cpu_freq_max
        || status.error_message.len() > MAX_JACK_STATUS_TEXT
        || status.governor.len() > MAX_JACK_STATUS_TEXT
    {
        return Err("PiPedal JACK status is invalid or excessive".into());
    }
    Ok(status)
}

/// Source-backed JACK server configuration readback.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct JackServerSettings {
    pub valid: bool,
    pub is_onboarding: bool,
    pub reboot_required: bool,
    pub is_jack_audio: bool,
    #[serde(default)]
    pub alsa_device: String,
    #[serde(default)]
    pub alsa_input_device: String,
    #[serde(default)]
    pub alsa_output_device: String,
    #[serde(default)]
    pub alsa_input_device_name: String,
    #[serde(default)]
    pub alsa_output_device_name: String,
    pub sample_rate: u64,
    pub buffer_size: u32,
    pub number_of_buffers: u32,
}

/// Decode and validate JACK server settings.
pub fn decode_jack_server_settings(
    body: Option<serde_json::Value>,
) -> Result<JackServerSettings, String> {
    let settings: JackServerSettings = decode_body(body)?;
    let names = [
        &settings.alsa_device,
        &settings.alsa_input_device,
        &settings.alsa_output_device,
        &settings.alsa_input_device_name,
        &settings.alsa_output_device_name,
    ];
    if (settings.valid
        && (settings.sample_rate == 0
            || settings.buffer_size == 0
            || settings.number_of_buffers == 0))
        || names.iter().any(|name| name.len() > MAX_JACK_DEVICE_TEXT)
    {
        return Err("PiPedal JACK server settings are invalid or excessive".into());
    }
    Ok(settings)
}

/// Source-backed JACK channel-selection readback.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JackChannelSelection {
    pub input_audio_ports: Vec<String>,
    pub output_audio_ports: Vec<String>,
    pub input_midi_devices: Vec<JackMidiDeviceInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JackMidiDeviceInfo {
    pub name: String,
    pub description: String,
}

/// Decode and bound PiPedal's JACK channel-selection response.
pub fn decode_jack_settings(
    body: Option<serde_json::Value>,
) -> Result<JackChannelSelection, String> {
    let selection: JackChannelSelection = decode_body(body)?;
    let valid_text = |value: &String| !value.is_empty() && value.len() <= MAX_JACK_DEVICE_TEXT;
    if selection.input_audio_ports.len() > 256
        || selection.output_audio_ports.len() > 256
        || selection.input_midi_devices.len() > 256
        || selection.input_audio_ports.iter().any(|value| !valid_text(value))
        || selection.output_audio_ports.iter().any(|value| !valid_text(value))
        || selection
            .input_midi_devices
            .iter()
            .any(|device| !valid_text(&device.name) || !valid_text(&device.description))
    {
        return Err("PiPedal JACK channel settings are invalid or excessive".into());
    }
    Ok(selection)
}

/// Source-backed JACK runtime configuration readback.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct JackConfiguration {
    pub is_valid: bool,
    pub is_onboarding: bool,
    pub is_restarting: bool,
    pub error_status: String,
    pub sample_rate: u32,
    pub block_length: u64,
    pub midi_buffer_size: u64,
    pub max_allowed_midi_delta: f64,
    pub input_audio_ports: Vec<String>,
    pub output_audio_ports: Vec<String>,
    pub input_midi_devices: Vec<JackMidiDeviceInfo>,
}

/// Decode and bound PiPedal's JACK runtime configuration response.
pub fn decode_jack_configuration(
    body: Option<serde_json::Value>,
) -> Result<JackConfiguration, String> {
    let configuration: JackConfiguration = decode_body(body)?;
    let valid_text = |value: &String| value.len() <= MAX_JACK_DEVICE_TEXT;
    if !configuration.max_allowed_midi_delta.is_finite()
        || configuration.input_audio_ports.len() > 256
        || configuration.output_audio_ports.len() > 256
        || configuration.input_midi_devices.len() > 256
        || configuration.error_status.len() > MAX_JACK_DEVICE_TEXT
        || configuration.input_audio_ports.iter().any(|value| !valid_text(value))
        || configuration.output_audio_ports.iter().any(|value| !valid_text(value))
        || configuration
            .input_midi_devices
            .iter()
            .any(|device| !valid_text(&device.name) || !valid_text(&device.description))
    {
        return Err("PiPedal JACK configuration is invalid or excessive".into());
    }
    Ok(configuration)
}

/// Validate a URI-to-favorite map before sending PiPedal's `setFavorites`.
pub fn validate_favorites(
    favorites: &std::collections::BTreeMap<String, bool>,
) -> Result<(), String> {
    if favorites.len() > MAX_FAVORITES || favorites.keys().any(String::is_empty) {
        return Err("PiPedal favorites contain invalid or excessive identities".into());
    }
    Ok(())
}

/// Read-only version metadata returned by PiPedal's `version` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PiPedalVersion {
    #[serde(default)]
    pub server: String,
    pub server_version: String,
    #[serde(default)]
    pub operating_system: String,
    #[serde(default)]
    pub os_version: String,
    #[serde(default)]
    pub debug: bool,
}

/// Decode and bound the version handshake response.
pub fn decode_version(body: Option<serde_json::Value>) -> Result<PiPedalVersion, String> {
    let version: PiPedalVersion = decode_body(body)?;
    if version.server_version.is_empty()
        || version.server.len() > MAX_VERSION_TEXT
        || version.server_version.len() > MAX_VERSION_TEXT
        || version.operating_system.len() > MAX_VERSION_TEXT
        || version.os_version.len() > MAX_VERSION_TEXT
    {
        return Err("PiPedal version metadata is invalid or oversized".into());
    }
    Ok(version)
}

/// One bounded release candidate in PiPedal's update-status response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRelease {
    pub update_available: bool,
    #[serde(default)]
    pub upgrade_version: String,
    #[serde(default)]
    pub upgrade_version_display_name: String,
    #[serde(default)]
    pub asset_name: String,
    #[serde(default)]
    pub update_url: String,
    #[serde(default)]
    pub gpg_signature_url: String,
}

/// Bounded source-backed `getUpdateStatus` readback.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    #[serde(default)]
    pub last_update_time: i64,
    pub is_valid: bool,
    #[serde(default)]
    pub error_message: String,
    pub update_policy: i32,
    pub is_online: bool,
    #[serde(default)]
    pub current_version: String,
    #[serde(default)]
    pub current_version_display_name: String,
    pub release_only_release: UpdateRelease,
    pub release_or_beta_release: UpdateRelease,
    pub dev_release: UpdateRelease,
}

impl UpdateStatus {
    /// Validate bounded text and the source-defined update-policy ordinal.
    pub fn validate(&self) -> Result<(), String> {
        if !(0..=3).contains(&self.update_policy) {
            return Err("PiPedal update policy is outside the source-defined range".into());
        }
        if self.error_message.len() > MAX_VERSION_TEXT
            || self.current_version.len() > MAX_VERSION_TEXT
            || self.current_version_display_name.len() > MAX_VERSION_TEXT
        {
            return Err("PiPedal update status contains oversized text".into());
        }
        for release in
            [&self.release_only_release, &self.release_or_beta_release, &self.dev_release]
        {
            if release.upgrade_version.len() > MAX_VERSION_TEXT
                || release.upgrade_version_display_name.len() > MAX_VERSION_TEXT
                || release.asset_name.len() > MAX_VERSION_TEXT
                || release.update_url.len() > MAX_VERSION_TEXT
                || release.gpg_signature_url.len() > MAX_VERSION_TEXT
            {
                return Err("PiPedal update release contains oversized text".into());
            }
        }
        Ok(())
    }
}

/// Decode and validate PiPedal's nested update-status response.
pub fn decode_update_status(body: Option<serde_json::Value>) -> Result<UpdateStatus, String> {
    let status: UpdateStatus = decode_body(body)?;
    status.validate()?;
    Ok(status)
}

/// Decode the qualified plugin catalog envelope while leaving version-specific control metadata
/// to the adapter's existing bounded projection.
pub fn decode_plugin_catalog(
    body: Option<serde_json::Value>,
) -> Result<Vec<PluginCatalogEntry>, String> {
    let entries: Vec<PluginCatalogEntry> = decode_body(body)?;
    if entries.is_empty() || entries.len() > MAX_PLUGIN_ENTRIES {
        return Err("PiPedal plugin catalog is empty or exceeds configured entry limit".into());
    }
    let mut uris = HashSet::with_capacity(entries.len());
    for entry in &entries {
        if entry.uri.is_empty() || entry.name.is_empty() || !uris.insert(&entry.uri) {
            return Err("PiPedal plugin catalog has invalid or duplicate URI".into());
        }
        decode_control_metadata(Some(serde_json::Value::Array(entry.controls.clone())))?;
    }
    Ok(entries)
}

/// Ordered phases of a PiPedal control session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionPhase {
    /// No WebSocket handshake has completed.
    Disconnected,
    /// WebSocket is open and `hello` is next.
    Connected,
    /// Client identity was accepted; `version` is next.
    Identified,
    /// Server version was read; catalog loading is in progress.
    LoadingCatalog,
    /// Required startup catalog/state has been loaded.
    Ready,
}

/// The bounded read-only requests used to populate a fresh PiPedal session.
#[must_use]
pub const fn startup_requests() -> [&'static str; 12] {
    [
        "hello",
        "version",
        "plugins",
        "currentPedalboard",
        "getSystemMidiBindings",
        "getFavorites",
        "getGovernorSettings",
        "getShowStatusMonitor",
        "getWifiRegulatoryDomains",
        "getHasWifi",
        "getUpdateStatus",
        "getKnownWifiNetworks",
    ]
}

impl SessionPhase {
    /// Return to the unauthenticated state after socket loss.
    #[must_use]
    pub const fn reset(self) -> Self {
        let _ = self;
        Self::Disconnected
    }

    /// Advance the session after a successful response.
    pub fn accept(self, message: &str) -> Result<Self, String> {
        match (self, message) {
            (Self::Connected, "ehlo") => Ok(Self::Identified),
            (Self::Identified, "version") => Ok(Self::LoadingCatalog),
            (Self::LoadingCatalog, "getSystemMidiBindings") => Ok(Self::Ready),
            (
                Self::LoadingCatalog,
                "plugins" | "currentPedalboard" | "pluginClasses" | "getPresets" | "getBankIndex"
                | "imageList",
            ) => Ok(Self::LoadingCatalog),
            (Self::Ready, _) => Ok(Self::Ready),
            (phase, message) => {
                Err(format!("unexpected PiPedal message {message} during {phase:?}"))
            }
        }
    }
}

/// Decode a bounded PiPedal array-framed message.
pub fn decode_message(input: &[u8]) -> Result<(MessageHeader, Option<serde_json::Value>), String> {
    if input.len() > MAX_FRAME_BYTES {
        return Err("PiPedal frame exceeds configured limit".into());
    }
    let value: serde_json::Value = serde_json::from_slice(input).map_err(|e| e.to_string())?;
    let array = value.as_array().ok_or_else(|| "PiPedal frame is not an array".to_string())?;
    if !(1..=2).contains(&array.len()) {
        return Err("PiPedal frame must contain one header and at most one body".into());
    }
    let header: MessageHeader =
        serde_json::from_value(array[0].clone()).map_err(|e| e.to_string())?;
    Ok((header, array.get(1).cloned()))
}

/// Decode a message body into a caller-selected typed value.
pub fn decode_body<T: for<'de> Deserialize<'de>>(
    body: Option<serde_json::Value>,
) -> Result<T, String> {
    body.ok_or_else(|| "PiPedal message has no body".to_string())
        .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()))
}

/// Body accepted by `PiPedal`'s `setControl` operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetControl {
    /// Client/session identifier.
    #[serde(rename = "clientId")]
    pub client_id: u64,
    /// Runtime plugin instance identifier.
    #[serde(rename = "instanceId")]
    pub instance_id: u64,
    /// Plugin control symbol.
    pub symbol: String,
    /// Numeric control value.
    pub value: f32,
}

impl SetControl {
    /// Validate the identity and numeric value before encoding a write.
    pub fn validate(&self) -> Result<(), String> {
        if self.client_id == 0 || self.symbol.is_empty() {
            return Err("PiPedal setControl identity is invalid".into());
        }
        if !self.value.is_finite() {
            return Err("PiPedal setControl value is not finite".into());
        }
        Ok(())
    }
}

/// Body accepted by PiPedal's pedalboard-item enable/bypass operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetPedalboardItemEnable {
    /// PiPedal client identity that owns the session.
    #[serde(rename = "clientId")]
    pub client_id: u64,
    /// Runtime pedalboard instance identifier.
    #[serde(rename = "instanceId")]
    pub instance_id: u64,
    /// Whether the item should be enabled (false means bypassed).
    pub enabled: bool,
}

impl SetPedalboardItemEnable {
    /// Validate the runtime identity before encoding a write.
    pub fn validate(&self) -> Result<(), String> {
        if self.instance_id == 0 {
            return Err("PiPedal pedalboard item identity is invalid".into());
        }
        Ok(())
    }
}

/// Body accepted by PiPedal's pedalboard-item plugin-UI mode operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetPedalboardItemUseModUi {
    /// PiPedal client identity that owns the session.
    #[serde(rename = "clientId")]
    pub client_id: u64,
    /// Runtime pedalboard instance identifier.
    #[serde(rename = "instanceId")]
    pub instance_id: u64,
    /// Whether the item should use its plugin-provided UI.
    #[serde(rename = "useModUi")]
    pub use_mod_ui: bool,
}

impl SetPedalboardItemUseModUi {
    /// Validate the runtime identity before encoding.
    pub fn validate(&self) -> Result<(), String> {
        if self.instance_id == 0 {
            return Err("PiPedal pedalboard item identity is invalid".into());
        }
        Ok(())
    }
}

/// Body accepted by PiPedal's pedalboard-item title operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetPedalboardItemTitle {
    /// Runtime pedalboard instance identifier.
    #[serde(rename = "instanceId")]
    pub instance_id: u64,
    /// User-visible pedalboard item title.
    pub title: String,
    /// PiPedal icon color key.
    #[serde(rename = "colorKey")]
    pub color_key: String,
}

impl SetPedalboardItemTitle {
    /// Validate the runtime identity and title fields before encoding.
    pub fn validate(&self) -> Result<(), String> {
        if self.instance_id == 0 || self.title.trim().is_empty() || self.color_key.trim().is_empty()
        {
            return Err("PiPedal pedalboard item title fields are invalid".into());
        }
        Ok(())
    }
}

/// Body accepted by PiPedal's MIDI listener operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ListenForMidiEvent {
    /// Client-generated listener handle.
    pub handle: u64,
}

/// Body accepted by PiPedal's patch-property monitor operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MonitorPatchProperty {
    /// Runtime pedalboard instance identifier.
    #[serde(rename = "instanceId")]
    pub instance_id: u64,
    /// Client-generated monitor handle.
    #[serde(rename = "clientHandle")]
    pub client_handle: u64,
    /// Patch property URI to monitor.
    #[serde(rename = "propertyUri")]
    pub property_uri: String,
}

/// Body accepted by PiPedal's plugin-preset load operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoadPluginPreset {
    /// Runtime plugin instance identifier.
    #[serde(rename = "pluginInstanceId")]
    pub plugin_instance_id: u64,
    /// Preset instance identifier returned by PiPedal.
    #[serde(rename = "presetInstanceId")]
    pub preset_instance_id: u64,
}

/// Preset instance identifier accepted by PiPedal's current-preset load operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LoadPreset(pub i64);

impl LoadPreset {
    /// Validate the preset identity before encoding.
    pub fn validate(&self) -> Result<(), String> {
        if self.0 <= 0 {
            return Err("PiPedal preset instance identity is invalid".into());
        }
        Ok(())
    }
}

impl LoadPluginPreset {
    /// Validate both runtime identities before encoding.
    pub fn validate(&self) -> Result<(), String> {
        if self.plugin_instance_id == 0 || self.preset_instance_id == 0 {
            return Err("PiPedal plugin-preset identities are invalid".into());
        }
        Ok(())
    }
}

impl MonitorPatchProperty {
    /// Validate identities and the property URI before encoding.
    pub fn validate(&self) -> Result<(), String> {
        if self.instance_id == 0 || self.client_handle == 0 || self.property_uri.trim().is_empty() {
            return Err("PiPedal patch-property monitor fields are invalid".into());
        }
        Ok(())
    }
}

impl ListenForMidiEvent {
    /// Validate the listener handle before encoding.
    pub fn validate(&self) -> Result<(), String> {
        if self.handle == 0 {
            return Err("PiPedal MIDI listener handle is invalid".into());
        }
        Ok(())
    }
}

/// `PiPedal` system or plugin MIDI binding metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiBinding {
    pub symbol: String,
    pub channel: i32,
    pub binding_type: i32,
    pub note: i32,
    pub control: i32,
    pub min_control_value: i32,
    pub max_control_value: i32,
    pub min_value: f32,
    pub max_value: f32,
    pub rotary_scale: f32,
    pub linear_control_type: i32,
    pub switch_control_type: i32,
}

/// Bounded body for `setSystemMidiBindings`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SystemMidiBindings {
    /// Complete replacement binding set.
    pub bindings: Vec<MidiBinding>,
}

impl SystemMidiBindings {
    /// Validate the replacement set size and numeric ranges.
    pub fn validate(&self) -> Result<(), String> {
        if self.bindings.len() > MAX_SYSTEM_MIDI_BINDINGS {
            return Err("PiPedal MIDI binding set exceeds configured limit".into());
        }
        for binding in &self.bindings {
            if binding.channel < -1
                || binding.channel > 15
                || binding.control < 0
                || binding.control > 127
            {
                return Err("PiPedal MIDI binding address is invalid".into());
            }
            if !binding.min_value.is_finite()
                || !binding.max_value.is_finite()
                || binding.min_value > binding.max_value
            {
                return Err("PiPedal MIDI binding range is invalid".into());
            }
        }
        Ok(())
    }
}

/// Decode the read-only array returned by `getSystemMidiBindings`.
pub fn decode_system_midi_bindings(
    body: Option<serde_json::Value>,
) -> Result<Vec<MidiBinding>, String> {
    let bindings: Vec<MidiBinding> = decode_body(body)?;
    SystemMidiBindings { bindings: bindings.clone() }.validate()?;
    Ok(bindings)
}

/// Stable identity for a discovered PiPedal plugin instance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginTarget {
    /// Plugin LV2/VST URI.
    pub uri: String,
    /// Runtime instance ID, never used as reusable identity.
    #[serde(rename = "instanceId")]
    pub instance_id: u64,
    /// Human-readable plugin name.
    pub name: String,
}

/// Metadata for one discovered, controllable plugin parameter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlDescriptor {
    /// Plugin URI and symbol together identify the reusable control.
    pub plugin_uri: String,
    /// LV2/plugin control symbol.
    pub symbol: String,
    /// Display label.
    pub label: String,
    /// Minimum value.
    pub min_value: f32,
    /// Maximum value.
    pub max_value: f32,
    /// Current value, if known.
    pub value: Option<f32>,
    /// Whether PiPedal accepts writes for this control.
    pub writable: bool,
}

/// Reusable physical-to-PiPedal mapping identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlMapping {
    /// Launch Control physical ID, for example `knob-r3-c4`.
    pub physical_control_id: String,
    /// Stable plugin URI.
    pub plugin_uri: String,
    /// Stable plugin parameter symbol.
    pub symbol: String,
    /// Optional preset or snapshot scope.
    pub scope: Option<String>,
}

/// A bounded, validated snapshot of the PiPedal plugin catalog.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PluginCatalog {
    /// Discovered plugin instances.
    pub targets: Vec<PluginTarget>,
    /// Discovered controls across those instances.
    pub controls: Vec<ControlDescriptor>,
}

impl PluginCatalog {
    /// Find a control by stable plugin URI and parameter symbol.
    #[must_use]
    pub fn find_control(&self, plugin_uri: &str, symbol: &str) -> Option<&ControlDescriptor> {
        self.controls
            .iter()
            .find(|control| control.plugin_uri == plugin_uri && control.symbol == symbol)
    }

    /// Resolve a reusable mapping against this snapshot.
    pub fn resolve_mapping(&self, mapping: &ControlMapping) -> Result<&ControlDescriptor, String> {
        mapping.validate()?;
        let matches = self.targets.iter().filter(|target| target.uri == mapping.plugin_uri).count();
        if matches == 0 {
            return Err("PiPedal mapping plugin is unavailable".into());
        }
        if matches > 1 && mapping.scope.is_none() {
            return Err("PiPedal mapping plugin is ambiguous".into());
        }
        let mut controls = self.controls.iter().filter(|control| {
            control.plugin_uri == mapping.plugin_uri && control.symbol == mapping.symbol
        });
        let control =
            controls.next().ok_or_else(|| "PiPedal mapping control is unavailable".to_string())?;
        if controls.next().is_some() {
            return Err("PiPedal mapping control is ambiguous".into());
        }
        if !control.writable {
            return Err("PiPedal mapping control is read-only".into());
        }
        Ok(control)
    }

    /// Validate bounds, instance identity, and every control descriptor.
    pub fn validate(&self) -> Result<(), String> {
        if self.controls.len() > MAX_CATALOG_CONTROLS {
            return Err("PiPedal catalog exceeds configured control limit".into());
        }
        let mut instances = HashSet::with_capacity(self.targets.len());
        for target in &self.targets {
            if target.uri.is_empty()
                || target.name.is_empty()
                || !instances.insert(target.instance_id)
            {
                return Err("PiPedal catalog has invalid or duplicate plugin instance".into());
            }
        }
        for control in &self.controls {
            control.validate()?;
        }
        Ok(())
    }
}

impl ControlMapping {
    /// Validate a reusable mapping before persistence or runtime resolution.
    pub fn validate(&self) -> Result<(), String> {
        if self.physical_control_id.is_empty()
            || self.plugin_uri.is_empty()
            || self.symbol.is_empty()
        {
            return Err("PiPedal mapping identity is incomplete".into());
        }
        if self.scope.as_ref().is_some_and(String::is_empty) {
            return Err("PiPedal mapping scope is empty".into());
        }
        Ok(())
    }
}

/// Validate a mapping set for identity and physical-control collisions.
pub fn validate_mappings(mappings: &[ControlMapping]) -> Result<(), String> {
    if mappings.len() > MAX_MAPPINGS {
        return Err("PiPedal mapping set exceeds configured limit".into());
    }
    let mut physical = HashSet::with_capacity(mappings.len());
    let mut targets = HashSet::with_capacity(mappings.len());
    for mapping in mappings {
        mapping.validate()?;
        if !physical.insert(&mapping.physical_control_id) {
            return Err(format!(
                "duplicate PiPedal physical control {}",
                mapping.physical_control_id
            ));
        }
        let target = (&mapping.plugin_uri, &mapping.symbol, &mapping.scope);
        if !targets.insert(target) {
            return Err(format!(
                "duplicate PiPedal target {}:{}",
                mapping.plugin_uri, mapping.symbol
            ));
        }
    }
    Ok(())
}

/// Bounded pickup state for one physical control after discovery or reconnect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickupState {
    /// Session generation for which pickup was armed.
    pub generation: u64,
    /// External value that the physical control must cross before writes resume.
    pub target: f32,
    /// Whether the physical control has picked up the external value.
    pub acquired: bool,
}

impl PickupState {
    /// Arms pickup against a freshly reconciled external value.
    pub fn arm(generation: u64, target: f32) -> Result<Self, String> {
        if !target.is_finite() {
            return Err("PiPedal pickup target must be finite".into());
        }
        Ok(Self { generation, target, acquired: false })
    }

    /// Re-arms pickup only for the current session generation.
    pub fn rearm(&mut self, generation: u64, target: f32) -> Result<(), String> {
        *self = Self::arm(generation, target)?;
        Ok(())
    }

    /// Records a physical value and acquires pickup when it is within tolerance.
    pub fn observe(&mut self, generation: u64, value: f32, tolerance: f32) -> Result<bool, String> {
        if !value.is_finite() || !tolerance.is_finite() || tolerance < 0.0 {
            return Err("PiPedal pickup observation is invalid".into());
        }
        if generation != self.generation {
            return Ok(false);
        }
        if (value - self.target).abs() <= tolerance {
            self.acquired = true;
        }
        Ok(self.acquired)
    }

    /// Returns whether a control write may be emitted for this generation.
    #[must_use]
    pub const fn permits_write(&self, generation: u64) -> bool {
        self.acquired && generation == self.generation
    }
}

/// Bounded pickup ledger keyed by stable physical-control identity.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReconciliationLedger {
    entries: Vec<(String, PickupState)>,
}

impl ReconciliationLedger {
    /// Arms or replaces one physical control's pickup state.
    pub fn arm(&mut self, physical_control_id: &str, state: PickupState) -> Result<(), String> {
        if physical_control_id.is_empty() {
            return Err("PiPedal reconciliation control identity is empty".into());
        }
        if let Some((_, existing)) =
            self.entries.iter_mut().find(|(identity, _)| identity == physical_control_id)
        {
            *existing = state;
            return Ok(());
        }
        if self.entries.len() >= MAX_RECONCILIATION_STATES {
            return Err("PiPedal reconciliation ledger is full".into());
        }
        self.entries.push((physical_control_id.to_owned(), state));
        Ok(())
    }

    /// Observes one physical value, returning whether pickup is acquired.
    pub fn observe(
        &mut self,
        physical_control_id: &str,
        generation: u64,
        value: f32,
        tolerance: f32,
    ) -> Result<bool, String> {
        self.entries
            .iter_mut()
            .find(|(identity, _)| identity == physical_control_id)
            .ok_or_else(|| "PiPedal reconciliation control is not armed".to_owned())?
            .1
            .observe(generation, value, tolerance)
    }

    /// Returns whether writes may proceed for a control in the current generation.
    #[must_use]
    pub fn permits_write(&self, physical_control_id: &str, generation: u64) -> bool {
        self.entries
            .iter()
            .find(|(identity, _)| identity == physical_control_id)
            .is_some_and(|(_, state)| state.permits_write(generation))
    }

    /// Invalidates every state after a reconnect; callers must re-arm from fresh external data.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Number of currently armed controls.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no controls are currently armed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl ControlDescriptor {
    /// Validate identity, bounds, and an optional current value.
    pub fn validate(&self) -> Result<(), String> {
        if self.plugin_uri.is_empty() || self.symbol.is_empty() {
            return Err("PiPedal control identity is empty".into());
        }
        if !self.min_value.is_finite()
            || !self.max_value.is_finite()
            || self.min_value > self.max_value
        {
            return Err("PiPedal control range is invalid".into());
        }
        if let Some(value) = self.value {
            if !value.is_finite() || value < self.min_value || value > self.max_value {
                return Err("PiPedal control value is outside its range".into());
            }
        }
        Ok(())
    }
}

/// Encode a `PiPedal` request as its documented two-element JSON array.
pub fn encode_request<T: Serialize>(request: &Request<T>) -> serde_json::Result<Vec<u8>> {
    let mut header = serde_json::Map::new();
    header.insert("message".into(), serde_json::Value::String(request.message.clone()));
    if let Some(reply_to) = request.reply_to {
        header.insert("replyTo".into(), serde_json::json!(reply_to));
    }
    match &request.body {
        Some(body) => serde_json::to_vec(&serde_json::json!([header, body])),
        None => serde_json::to_vec(&serde_json::json!([header])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_server_reply_preserves_request_correlation() {
        let (header, body) = decode_message(br#"[{"reply":1,"message":"ehlo"},1]"#)
            .expect("installed hello response");
        assert_eq!(header.reply_to, Some(1));
        assert_eq!(body, Some(serde_json::json!(1)));
        let (event, _) = decode_message(br#"[{"message":"onPedalboardChanged"},{}]"#)
            .expect("unsolicited event");
        assert_eq!(event.reply_to, None);
    }

    struct MockTransport {
        sent: Vec<Vec<u8>>,
        received: Option<Vec<u8>>,
        send_error: Option<TransportError>,
        receive_error: Option<TransportError>,
    }
    impl Transport for MockTransport {
        fn send(&mut self, frame: &[u8]) -> Result<(), TransportError> {
            if let Some(error) = self.send_error {
                return Err(error);
            }
            self.sent.push(frame.to_vec());
            Ok(())
        }
        fn receive(&mut self) -> Result<Option<Vec<u8>>, TransportError> {
            if let Some(error) = self.receive_error {
                return Err(error);
            }
            Ok(self.received.take())
        }
    }

    #[test]
    fn set_control_uses_pipedal_wire_names() {
        let request = Request {
            message: "setControl".into(),
            reply_to: Some(7),
            body: Some(SetControl {
                client_id: 1,
                instance_id: 127,
                symbol: "lfLevel".into(),
                value: -3.5,
            }),
        };
        let value: serde_json::Value =
            serde_json::from_slice(&encode_request(&request).expect("encode")).expect("json");
        assert_eq!(value[0]["message"], "setControl");
        assert_eq!(value[0]["replyTo"], 7);
        assert_eq!(value[1]["instanceId"], 127);
        assert_eq!(value[1]["symbol"], "lfLevel");
        assert!(request.body.as_ref().expect("body").validate().is_ok());
    }

    #[test]
    fn midi_binding_round_trips_all_fields() {
        let binding = MidiBinding {
            symbol: "lfLevel".into(),
            channel: 0,
            binding_type: 1,
            note: 0,
            control: 74,
            min_control_value: 0,
            max_control_value: 127,
            min_value: -12.0,
            max_value: 12.0,
            rotary_scale: 1.0,
            linear_control_type: 0,
            switch_control_type: 0,
        };
        let encoded = serde_json::to_vec(&binding).expect("encode");
        assert_eq!(serde_json::from_slice::<MidiBinding>(&encoded).expect("decode"), binding);
    }

    #[test]
    fn system_midi_binding_payload_is_bounded_and_validated() {
        let binding = MidiBinding {
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
        assert!(SystemMidiBindings { bindings: vec![binding] }.validate().is_ok());
    }

    #[test]
    fn request_queue_is_fifo_and_bounded() {
        let mut queue = RequestQueue::default();
        queue.push(b"one".to_vec()).expect("push");
        queue.push(b"two".to_vec()).expect("push");
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pending_bytes(), 6);
        assert_eq!(queue.pop(), Some(b"one".to_vec()));
        assert_eq!(queue.pending_bytes(), 3);
        assert_eq!(queue.pop(), Some(b"two".to_vec()));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn pickup_requires_current_generation_and_external_value_match() {
        let mut pickup = PickupState::arm(4, 0.5).expect("arm");
        assert!(!pickup.permits_write(4));
        assert!(!pickup.observe(3, 0.5, 0.01).expect("stale observation"));
        assert!(!pickup.observe(4, 0.8, 0.01).expect("miss"));
        assert!(pickup.observe(4, 0.505, 0.01).expect("acquire"));
        assert!(pickup.permits_write(4));
        assert!(!pickup.permits_write(5));
        pickup.rearm(5, -1.0).expect("rearm");
        assert!(!pickup.permits_write(5));
    }

    #[test]
    fn pickup_rejects_non_finite_inputs() {
        assert!(PickupState::arm(1, f32::NAN).is_err());
        let mut pickup = PickupState::arm(1, 0.0).expect("arm");
        assert!(pickup.observe(1, f32::INFINITY, 0.1).is_err());
        assert!(pickup.observe(1, 0.0, -0.1).is_err());
    }

    #[test]
    fn reconciliation_ledger_is_bounded_and_clears_on_reconnect() {
        let mut ledger = ReconciliationLedger::default();
        ledger.arm("knob-r3-c4", PickupState::arm(8, 0.25).expect("arm")).expect("insert");
        assert!(!ledger.permits_write("knob-r3-c4", 8));
        assert!(ledger.observe("knob-r3-c4", 8, 0.25, 0.001).expect("observe"));
        assert!(ledger.permits_write("knob-r3-c4", 8));
        assert!(ledger.observe("missing", 8, 0.0, 0.1).is_err());
        ledger.clear();
        assert!(ledger.is_empty());
        assert_eq!(ledger.len(), 0);
    }

    #[test]
    fn reconciliation_ledger_rejects_state_beyond_mapping_bound() {
        let mut ledger = ReconciliationLedger::default();
        for index in 0..MAX_RECONCILIATION_STATES {
            ledger
                .arm(&format!("knob-{index}"), PickupState::arm(1, 0.0).expect("finite target"))
                .expect("within bound");
        }
        assert_eq!(ledger.len(), MAX_RECONCILIATION_STATES);
        assert!(ledger.arm("overflow", PickupState::arm(1, 0.0).expect("finite target")).is_err());
    }

    #[test]
    fn request_queue_reports_bounded_rejections() {
        let mut queue = RequestQueue::default();
        let oversized = vec![b'x'; MAX_FRAME_BYTES + 1];
        assert!(queue.push(oversized).is_err());
        for _ in 0..MAX_PENDING_REQUESTS {
            queue.push(vec![b'x']).expect("capacity");
        }
        assert!(queue.push(vec![b'x']).is_err());
        assert_eq!(queue.len(), MAX_PENDING_REQUESTS);
        assert_eq!(queue.rejected_count(), 2);
    }

    #[test]
    fn request_queue_enforces_total_byte_budget() {
        let mut queue = RequestQueue::default();
        for _ in 0..4 {
            queue.push(vec![b'x'; MAX_FRAME_BYTES]).expect("frame budget");
        }
        assert!(queue.push(vec![b'x']).is_err());
        assert_eq!(queue.pending_bytes(), MAX_PENDING_REQUEST_BYTES);
        assert_eq!(queue.pop().expect("frame").len(), MAX_FRAME_BYTES);
        assert_eq!(queue.pending_bytes(), MAX_PENDING_REQUEST_BYTES - MAX_FRAME_BYTES);
        queue.push(vec![b'x']).expect("released byte budget");
    }

    #[test]
    fn session_accounts_transport_errors_without_changing_generation() {
        let mut session = Session::default();
        let generation = session.generation();
        session.record_transport_error(TransportError::Timeout);
        session.record_transport_error(TransportError::Disconnected);
        session.record_transport_error(TransportError::Protocol);
        assert_eq!(session.timeouts(), 1);
        assert_eq!(session.transport_failures(), 2);
        assert_eq!(session.generation(), generation);
        session.reset();
        assert_eq!(session.timeouts(), 0);
        assert_eq!(session.transport_failures(), 0);
    }

    #[test]
    fn session_generation_invalidates_queued_work_on_reset() {
        let mut session = Session::default();
        let generation = session.generation();
        session.connect().expect("connect");
        assert!(!session.is_ready());
        session.enqueue(generation, b"ok".to_vec()).expect("enqueue");
        assert!(session.enqueue_control(generation, b"control".to_vec()).is_err());
        session.reset();
        assert!(session.enqueue(generation, b"stale".to_vec()).is_err());
        assert!(session.pop().is_none());
        assert_eq!(session.generation(), generation + 1);
        assert!(Session::default().enqueue(0, b"blocked".to_vec()).is_err());
        assert_eq!(session.pending_requests(), 0);
        assert_eq!(session.pending_request_bytes(), 0);
        assert_eq!(session.rejected_requests(), 0);
    }

    #[test]
    fn session_connect_starts_handshake_once() {
        let mut session = Session::default();
        session.connect().expect("connect");
        assert_eq!(session.phase(), SessionPhase::Connected);
        assert!(session.connect().is_err());
    }

    #[test]
    fn session_allows_control_only_after_catalog_handshake() {
        let mut session = Session::default();
        session.connect().expect("connect");
        let generation = session.generation();
        for message in [
            "ehlo",
            "version",
            "plugins",
            "currentPedalboard",
            "pluginClasses",
            "getPresets",
            "getBankIndex",
            "getSystemMidiBindings",
            "getFavorites",
        ] {
            session.accept(message).expect("handshake");
        }
        assert!(session.is_ready());
        session.enqueue_control(generation, b"control".to_vec()).expect("control");
        assert_eq!(session.pop(), Some(b"control".to_vec()));
    }

    #[test]
    fn transport_boundary_supports_nonblocking_mock_exchange() {
        let mut transport = MockTransport {
            sent: Vec::new(),
            received: Some(b"reply".to_vec()),
            send_error: None,
            receive_error: None,
        };
        transport.send(b"request").expect("send");
        assert_eq!(transport.sent, vec![b"request".to_vec()]);
        assert_eq!(transport.receive().expect("receive"), Some(b"reply".to_vec()));
    }

    #[test]
    fn session_sends_only_within_worker_budget_and_accounts_failure() {
        let mut session = Session::default();
        session.connect().expect("connect");
        let generation = session.generation();
        session.enqueue(generation, b"one".to_vec()).expect("enqueue");
        session.enqueue(generation, b"two".to_vec()).expect("enqueue");
        let mut transport = MockTransport {
            sent: Vec::new(),
            received: None,
            send_error: None,
            receive_error: None,
        };
        assert_eq!(session.send_pending(&mut transport, 1).expect("send"), 1);
        assert_eq!(session.pending_requests(), 1);
        assert_eq!(transport.sent, vec![b"one".to_vec()]);

        transport.send_error = Some(TransportError::Timeout);
        assert_eq!(session.send_pending(&mut transport, 1), Err(TransportError::Timeout));
        assert_eq!(session.timeouts(), 1);
        assert_eq!(session.pending_requests(), 0);

        transport.send_error = None;
        transport.received = Some(b"event".to_vec());
        assert_eq!(
            session.receive_available(&mut transport, 1).expect("receive"),
            vec![b"event".to_vec()]
        );
        transport.receive_error = Some(TransportError::Disconnected);
        assert_eq!(session.receive_available(&mut transport, 1), Err(TransportError::Disconnected));
        assert_eq!(session.transport_failures(), 1);
        transport.receive_error = None;
        transport.received = Some(br#"[{"message":"onPedalboardChanged"},{}]"#.to_vec());
        assert_eq!(
            session.receive_messages(&mut transport, 1).expect("decode")[0].0.message,
            "onPedalboardChanged"
        );
    }

    #[test]
    fn decoder_rejects_invalid_shape_and_accepts_event_body() {
        assert!(decode_message(br"{}").is_err());
        assert!(decode_message(br#"[{"message":"x"},{},{}]"#).is_err());
        let (header, body) =
            decode_message(br#"[{"message":"onPedalboardChanged"},{"generation":3}]"#)
                .expect("decode");
        assert_eq!(header.message, "onPedalboardChanged");
        assert_eq!(body.expect("body")["generation"], 3);
    }

    #[test]
    fn decoder_rejects_oversized_frames() {
        let input = vec![b' '; MAX_FRAME_BYTES + 1];
        assert!(decode_message(&input).is_err());
    }

    #[test]
    fn typed_body_decode_reports_missing_body_and_errors() {
        assert!(decode_body::<ErrorBody>(None).is_err());
        let body = serde_json::json!({"message":"invalid control"});
        assert_eq!(decode_body::<ErrorBody>(Some(body)).expect("body").message, "invalid control");
    }

    #[test]
    fn preset_readback_decoder_accepts_qualified_source_shape() {
        let body = serde_json::json!({
            "selectedInstanceId": 7,
            "presetChanged": true,
            "presets": [
                {"instanceId": 7, "name": "Clean"},
                {"instanceId": 8, "name": "Drive"}
            ]
        });
        let index = decode_preset_index(Some(body)).expect("preset index");
        assert_eq!(index.selected_instance_id, 7);
        assert!(index.preset_changed);
        assert_eq!(index.presets[1].name, "Drive");
    }

    #[test]
    fn preset_readback_decoder_rejects_duplicates_and_oversized_catalogs() {
        let duplicate = serde_json::json!({
            "selectedInstanceId": -1,
            "presetChanged": false,
            "presets": [{"instanceId": 1, "name": "A"}, {"instanceId": 1, "name": "B"}]
        });
        assert!(decode_preset_index(Some(duplicate)).is_err());
        let oversized = serde_json::json!({
            "selectedInstanceId": -1,
            "presetChanged": false,
            "presets": (0..=MAX_PRESET_ENTRIES).map(|id| serde_json::json!({"instanceId": id, "name": id.to_string()})).collect::<Vec<_>>()
        });
        assert!(decode_preset_index(Some(oversized)).is_err());
    }

    #[test]
    fn bank_readback_decoder_accepts_qualified_source_shape() {
        let body = serde_json::json!({
            "selectedBank": 11,
            "entries": [
                {"instanceId": 11, "name": "Factory"},
                {"instanceId": 12, "name": "User"}
            ]
        });
        let index = decode_bank_index(Some(body)).expect("bank index");
        assert_eq!(index.selected_bank, 11);
        assert_eq!(index.entries[1].name, "User");
    }

    #[test]
    fn bank_readback_decoder_rejects_duplicates_and_oversized_catalogs() {
        let duplicate = serde_json::json!({
            "selectedBank": -1,
            "entries": [{"instanceId": 1, "name": "A"}, {"instanceId": 1, "name": "B"}]
        });
        assert!(decode_bank_index(Some(duplicate)).is_err());
        let oversized = serde_json::json!({
            "selectedBank": -1,
            "entries": (0..=MAX_BANK_ENTRIES).map(|id| serde_json::json!({"instanceId": id, "name": id.to_string()})).collect::<Vec<_>>()
        });
        assert!(decode_bank_index(Some(oversized)).is_err());
    }

    #[test]
    fn current_pedalboard_decoder_accepts_qualified_source_shape() {
        let body = serde_json::json!({
            "items": [{"instanceId": 7, "uri": "urn:eq", "controlValues": [{"key": "gain", "value": 0.5}]}]
        });
        let state = decode_current_pedalboard(Some(body)).expect("pedalboard");
        assert_eq!(state.items[0].instance_id, 7);
        assert_eq!(state.items[0].control_values[0].key, "gain");
    }

    #[test]
    fn current_pedalboard_decoder_rejects_duplicates_nonfinite_and_oversized_state() {
        let duplicate = serde_json::json!({
            "items": [{"instanceId": 1, "uri": "urn:a"}, {"instanceId": 1, "uri": "urn:b"}]
        });
        assert!(decode_current_pedalboard(Some(duplicate)).is_err());
        let nonfinite = serde_json::json!({
            "items": [{"instanceId": 1, "uri": "urn:a", "controlValues": [{"key": "x", "value": "NaN"}]}]
        });
        assert!(decode_current_pedalboard(Some(nonfinite)).is_err());
        let oversized = serde_json::json!({
            "items": (1..=MAX_PEDALBOARD_ITEMS + 1).map(|id| serde_json::json!({"instanceId": id, "uri": id.to_string()})).collect::<Vec<_>>()
        });
        assert!(decode_current_pedalboard(Some(oversized)).is_err());
    }

    #[test]
    fn plugin_catalog_decoder_bounds_identity_envelope() {
        let body = serde_json::json!([{"uri": "urn:eq", "name": "EQ", "controls": []}]);
        let entries = decode_plugin_catalog(Some(body)).expect("plugin catalog");
        assert_eq!(entries[0].uri, "urn:eq");
        let duplicate = serde_json::json!([
            {"uri": "urn:eq", "name": "EQ"}, {"uri": "urn:eq", "name": "EQ 2"}
        ]);
        assert!(decode_plugin_catalog(Some(duplicate)).is_err());
    }

    #[test]
    fn control_metadata_decoder_validates_ranges_and_aliases() {
        let body = serde_json::json!([{
            "symbol": "gain", "minValue": -12, "maxValue": 12,
            "value": 0, "writable": true, "label": "Gain"
        }]);
        let controls = decode_control_metadata(Some(body)).expect("control metadata");
        assert_eq!(controls[0].symbol, "gain");
        assert_eq!(controls[0].value, Some(0.0));
        let invalid = serde_json::json!([{"symbol": "gain", "minValue": 2, "maxValue": 1}]);
        assert!(decode_control_metadata(Some(invalid)).is_err());
    }

    #[test]
    fn plugin_class_decoder_bounds_source_tree() {
        let body = serde_json::json!({
            "uri": "urn:root", "display_name": "Root", "parent_uri": "", "plugin_type": "None",
            "children": [{"uri": "urn:fx", "display_name": "Effects", "parent_uri": "urn:root", "plugin_type": "Effect"}]
        });
        let root = decode_plugin_classes(Some(body)).expect("plugin classes");
        assert_eq!(root.children[0].uri, "urn:fx");
        let invalid = serde_json::json!({"uri":"", "display_name":"Root", "parent_uri":"", "plugin_type":"None"});
        assert!(decode_plugin_classes(Some(invalid)).is_err());
    }

    #[test]
    fn favorites_decoder_accepts_source_map_and_rejects_empty_key() {
        let value = decode_favorites(Some(serde_json::json!({"urn:eq": true}))).expect("favorites");
        assert!(value.get("urn:eq").copied().unwrap_or(false));
        assert!(decode_favorites(Some(serde_json::json!({"": true}))).is_err());
    }

    #[test]
    fn version_decoder_accepts_source_shape_and_bounds_text() {
        let body = serde_json::json!({"server":"PiPedal","serverVersion":"2.0","operatingSystem":"Linux","osVersion":"6","debug":false});
        assert_eq!(decode_version(Some(body)).expect("version").server_version, "2.0");
        let invalid = serde_json::json!({"server":"PiPedal","serverVersion":"","operatingSystem":"Linux","osVersion":"6","debug":false});
        assert!(decode_version(Some(invalid)).is_err());
    }

    #[test]
    fn has_wifi_decoder_accepts_only_boolean_source_shape() {
        assert!(decode_has_wifi(Some(serde_json::json!(true))).expect("Wi-Fi flag"));
        assert!(decode_has_wifi(Some(serde_json::json!("true"))).is_err());
        assert!(decode_has_wifi(None).is_err());
    }

    #[test]
    fn known_wifi_network_decoder_is_bounded_and_rejects_empty_names() {
        assert_eq!(
            decode_known_wifi_networks(Some(serde_json::json!(["studio", "backup"])))
                .expect("known networks"),
            vec!["studio", "backup"]
        );
        assert!(decode_known_wifi_networks(Some(serde_json::json!([""]))).is_err());
    }

    #[test]
    fn system_midi_readback_decoder_accepts_array_and_enforces_limits() {
        let body = serde_json::json!([{
            "symbol":"gain", "channel":1, "bindingType":0, "note":0, "control":7,
            "minControlValue":0, "maxControlValue":127, "minValue":0.0, "maxValue":1.0,
            "rotaryScale":1.0, "linearControlType":0, "switchControlType":0
        }]);
        assert_eq!(decode_system_midi_bindings(Some(body)).expect("bindings").len(), 1);
        let invalid = serde_json::json!([{"symbol":"gain","channel":99,"control":7,"minValue":0,"maxValue":1,"minControlValue":0,"maxControlValue":127,"bindingType":0,"note":0,"rotaryScale":1,"linearControlType":0,"switchControlType":0}]);
        assert!(decode_system_midi_bindings(Some(invalid)).is_err());
    }

    #[test]
    fn session_requires_hello_and_version_before_catalog_ready() {
        assert_eq!(SessionPhase::Connected.accept("ehlo").expect("ehlo"), SessionPhase::Identified);
        assert!(SessionPhase::Connected.accept("hello").is_err());
        assert!(SessionPhase::Connected.accept("getSystemMidiBindings").is_err());
        let phase = SessionPhase::Identified.accept("version").expect("version");
        let phase = phase.accept("plugins").expect("plugins");
        let phase = phase.accept("getSystemMidiBindings").expect("bindings");
        assert_eq!(phase.accept("getFavorites").expect("favorites"), SessionPhase::Ready);
        assert_eq!(phase.reset(), SessionPhase::Disconnected);
    }

    #[test]
    fn startup_plan_is_ordered_and_bounded() {
        let requests = startup_requests();
        assert_eq!(requests[0], "hello");
        assert_eq!(requests[1], "version");
        assert_eq!(requests[4], "getSystemMidiBindings");
        assert_eq!(requests.last(), Some(&"getKnownWifiNetworks"));
        assert!(requests.len() <= 16);
    }

    #[test]
    fn operation_catalog_preserves_wire_names_and_confirmation_policy() {
        assert_eq!(Operation::SetControl.wire_name(), "setControl");
        assert!(!Operation::SetControl.requires_confirmation());
        assert!(Operation::Shutdown.requires_confirmation());
        assert!(Operation::GetJackStatus.is_read_only());
        assert!(!Operation::SetOutputVolume.is_read_only());
        assert!(Operation::SetControl.is_mapping_eligible());
        assert!(!Operation::Restart.is_mapping_eligible());
        assert_eq!(Operation::SetControl.family(), "controls");
        assert_eq!(Operation::Shutdown.family(), "host");
        for operation in Operation::all() {
            if operation.is_mapping_eligible() {
                assert!(!operation.requires_confirmation());
                assert!(!operation.is_read_only());
            }
        }
        assert_eq!(Operation::all().len(), 45);
        assert!(Operation::all().iter().all(|operation| !operation.wire_name().is_empty()));
        assert_eq!(serde_json::to_string(&Operation::SetControl).expect("json"), "\"setControl\"");
    }

    #[test]
    fn text_assembler_reassembles_fragments_and_bounds_growth() {
        let mut assembler = TextAssembler::default();
        assert_eq!(assembler.push(br"[{", false).expect("fragment"), None);
        assert_eq!(assembler.push(br"}]", true).expect("complete"), Some(b"[{}]".to_vec()));
        let mut assembler = TextAssembler::default();
        assert!(assembler.push(&vec![b'x'; MAX_FRAME_BYTES], false).is_ok());
        assert!(assembler.push(b"x", true).is_err());
    }

    #[test]
    fn server_frame_decoder_validates_header_and_payload() {
        let frame = decode_server_frame(&[0x81, 3, b'o', b'k', b'!']).expect("frame");
        assert_eq!(
            frame,
            ServerFrame { final_fragment: true, opcode: 1, payload: b"ok!".to_vec() }
        );
        assert!(decode_server_frame(&[0x81, 0x80]).is_err());
        assert!(decode_server_frame(&[0x81, 4, b'o']).is_err());
    }

    #[test]
    fn client_encoder_masks_text_and_supports_extended_lengths() {
        let frame = encode_client_text(b"ok", [1, 2, 3, 4]).expect("frame");
        assert_eq!(&frame[..6], &[0x81, 0x82, 1, 2, 3, 4]);
        assert_eq!(&frame[6..], &[110, 105]);
        assert!(encode_client_text(&vec![0; MAX_FRAME_BYTES + 1], [0; 4]).is_err());
    }

    #[test]
    fn control_descriptor_rejects_invalid_identity_range_and_value() {
        let mut descriptor = ControlDescriptor {
            plugin_uri: "urn:eq".into(),
            symbol: "gain".into(),
            label: "Gain".into(),
            min_value: -12.0,
            max_value: 12.0,
            value: Some(0.0),
            writable: true,
        };
        assert!(descriptor.validate().is_ok());
        descriptor.value = Some(13.0);
        assert!(descriptor.validate().is_err());
    }

    #[test]
    fn control_mapping_requires_stable_identity() {
        let mapping = ControlMapping {
            physical_control_id: "knob-r3-c4".into(),
            plugin_uri: "http://two-play.com/plugins/toob-parametric-eq".into(),
            symbol: "lfLevel".into(),
            scope: Some("preset:10".into()),
        };
        assert!(mapping.validate().is_ok());
        assert!(ControlMapping { symbol: String::new(), ..mapping }.validate().is_err());
    }

    #[test]
    fn mapping_set_rejects_physical_and_target_collisions() {
        let first = ControlMapping {
            physical_control_id: "knob-r3-c4".into(),
            plugin_uri: "urn:eq".into(),
            symbol: "lfLevel".into(),
            scope: None,
        };
        let second = ControlMapping {
            physical_control_id: "knob-r3-c4".into(),
            plugin_uri: "urn:eq".into(),
            symbol: "hfLevel".into(),
            scope: None,
        };
        assert!(validate_mappings(&[first.clone(), second]).is_err());
        let duplicate_target = ControlMapping { physical_control_id: "knob-r3-c5".into(), ..first };
        assert!(validate_mappings(&[duplicate_target.clone(), duplicate_target]).is_err());
        let many = (0..=MAX_MAPPINGS)
            .map(|i| ControlMapping {
                physical_control_id: format!("knob-{i}"),
                plugin_uri: "urn:eq".into(),
                symbol: format!("band-{i}"),
                scope: None,
            })
            .collect::<Vec<_>>();
        assert!(validate_mappings(&many).is_err());
    }

    #[test]
    fn catalog_rejects_duplicate_instances_and_oversized_control_sets() {
        let target = PluginTarget { uri: "urn:eq".into(), instance_id: 1, name: "EQ".into() };
        let catalog = PluginCatalog { targets: vec![target.clone(), target], controls: Vec::new() };
        assert!(catalog.validate().is_err());
        let controls = (0..=MAX_CATALOG_CONTROLS)
            .map(|i| ControlDescriptor {
                plugin_uri: "urn:eq".into(),
                symbol: format!("c{i}"),
                label: "Control".into(),
                min_value: 0.0,
                max_value: 1.0,
                value: Some(0.0),
                writable: true,
            })
            .collect();
        assert!(PluginCatalog { targets: Vec::new(), controls }.validate().is_err());
    }

    #[test]
    fn catalog_lookup_uses_uri_and_symbol() {
        let control = ControlDescriptor {
            plugin_uri: "urn:eq".into(),
            symbol: "lfLevel".into(),
            label: "Low".into(),
            min_value: -12.0,
            max_value: 12.0,
            value: Some(0.0),
            writable: true,
        };
        let catalog = PluginCatalog {
            targets: vec![PluginTarget { uri: "urn:eq".into(), instance_id: 1, name: "EQ".into() }],
            controls: vec![control],
        };
        assert_eq!(
            catalog.find_control("urn:eq", "lfLevel").map(|value| value.label.as_str()),
            Some("Low")
        );
        assert!(catalog.find_control("urn:eq", "missing").is_none());
        let mapping = ControlMapping {
            physical_control_id: "knob-r3-c4".into(),
            plugin_uri: "urn:eq".into(),
            symbol: "lfLevel".into(),
            scope: None,
        };
        assert!(catalog.resolve_mapping(&mapping).is_ok());
        let ambiguous = PluginCatalog {
            targets: vec![
                PluginTarget { uri: "urn:eq".into(), instance_id: 1, name: "EQ 1".into() },
                PluginTarget { uri: "urn:eq".into(), instance_id: 2, name: "EQ 2".into() },
            ],
            controls: catalog.controls,
        };
        assert!(ambiguous.resolve_mapping(&mapping).is_err());
        let duplicate_control = PluginCatalog {
            targets: vec![PluginTarget { uri: "urn:eq".into(), instance_id: 1, name: "EQ".into() }],
            controls: vec![
                ControlDescriptor {
                    plugin_uri: "urn:eq".into(),
                    symbol: "lfLevel".into(),
                    label: "Low A".into(),
                    min_value: -12.0,
                    max_value: 12.0,
                    value: Some(0.0),
                    writable: true,
                },
                ControlDescriptor {
                    plugin_uri: "urn:eq".into(),
                    symbol: "lfLevel".into(),
                    label: "Low B".into(),
                    min_value: -12.0,
                    max_value: 12.0,
                    value: Some(0.0),
                    writable: true,
                },
            ],
        };
        assert_eq!(
            duplicate_control.resolve_mapping(&mapping),
            Err("PiPedal mapping control is ambiguous".into())
        );
    }

    #[test]
    fn monitoring_payloads_match_source_names_and_reject_zero_handles() {
        let listen = ListenForMidiEvent { handle: 4 };
        assert_eq!(
            serde_json::to_value(&listen).expect("listen payload"),
            serde_json::json!({"handle": 4})
        );
        assert!(ListenForMidiEvent { handle: 0 }.validate().is_err());
        let monitor = MonitorPatchProperty {
            instance_id: 7,
            client_handle: 4,
            property_uri: "urn:property".into(),
        };
        assert_eq!(
            serde_json::to_value(&monitor).expect("monitor payload"),
            serde_json::json!({"instanceId": 7, "clientHandle": 4, "propertyUri": "urn:property"})
        );
        assert!(MonitorPatchProperty { property_uri: String::new(), ..monitor }
            .validate()
            .is_err());
    }

    #[test]
    fn jack_server_settings_decode_source_shape_and_invalid_active_audio() {
        let body = serde_json::json!({
            "valid": true,
            "isOnboarding": false,
            "rebootRequired": false,
            "isJackAudio": true,
            "alsaDevice": "",
            "alsaInputDevice": "hw:1",
            "alsaOutputDevice": "hw:1",
            "alsaInputDeviceName": "USB Audio",
            "alsaOutputDeviceName": "USB Audio",
            "sampleRate": 48000,
            "bufferSize": 64,
            "numberOfBuffers": 3
        });
        let settings = decode_jack_server_settings(Some(body)).expect("settings response");
        assert_eq!(settings.sample_rate, 48_000);
        assert_eq!(settings.alsa_input_device, "hw:1");

        let invalid = serde_json::json!({
            "valid": true,
            "isOnboarding": false,
            "rebootRequired": false,
            "isJackAudio": true,
            "sampleRate": 0,
            "bufferSize": 64,
            "numberOfBuffers": 3
        });
        assert!(decode_jack_server_settings(Some(invalid)).is_err());
    }

    #[test]
    fn jack_settings_decode_bounds_source_channel_selection() {
        let selection = decode_jack_settings(Some(serde_json::json!({
            "inputAudioPorts": ["system:capture_1"],
            "outputAudioPorts": ["system:playback_1"],
            "inputMidiDevices": [{"name": "midi:0", "description": "Controller"}]
        })))
        .expect("channel selection response");
        assert_eq!(selection.output_audio_ports[0], "system:playback_1");
        assert!(decode_jack_settings(Some(serde_json::json!({
            "inputAudioPorts": [""], "outputAudioPorts": [], "inputMidiDevices": []
        })))
        .is_err());
    }
}
