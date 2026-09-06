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
pub const MAX_FRAME_BYTES: usize = 1_048_576;
/// Maximum reusable PiPedal mappings in one connector configuration.
pub const MAX_MAPPINGS: usize = 128;
/// Maximum discovered plugin controls in one catalog snapshot.
pub const MAX_CATALOG_CONTROLS: usize = 2_048;
/// Maximum system MIDI bindings accepted in one PiPedal update.
pub const MAX_SYSTEM_MIDI_BINDINGS: usize = 128;
/// Maximum requests waiting for the PiPedal transport worker.
pub const MAX_PENDING_REQUESTS: usize = 64;

/// Bounded handoff queue between MIDI/control callers and the PiPedal transport worker.
#[derive(Debug, Default)]
pub struct RequestQueue {
    pending: VecDeque<Vec<u8>>,
}

/// Connector-owned session state used to reject stale work after reconnect.
#[derive(Debug)]
pub struct Session {
    phase: SessionPhase,
    generation: u64,
    queue: RequestQueue,
}

impl Default for Session {
    fn default() -> Self {
        Self { phase: SessionPhase::Disconnected, generation: 0, queue: RequestQueue::default() }
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
}

impl RequestQueue {
    /// Enqueue an encoded request, rejecting oversized or saturated queues.
    pub fn push(&mut self, request: Vec<u8>) -> Result<(), String> {
        if request.len() > MAX_FRAME_BYTES {
            return Err("PiPedal request exceeds configured frame limit".into());
        }
        if self.pending.len() >= MAX_PENDING_REQUESTS {
            return Err("PiPedal request queue is full".into());
        }
        self.pending.push_back(request);
        Ok(())
    }

    /// Remove the oldest request for transport processing.
    pub fn pop(&mut self) -> Option<Vec<u8>> {
        self.pending.pop_front()
    }

    /// Number of requests awaiting transport.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Whether no requests await transport processing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
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
    /// Load a saved preset.
    LoadPreset,
    /// Save the current preset.
    SaveCurrentPreset,
    /// Query ALSA devices.
    GetAlsaDevices,
    /// Query JACK status.
    GetJackStatus,
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
            Self::LoadPreset,
            Self::SaveCurrentPreset,
            Self::GetAlsaDevices,
            Self::GetJackStatus,
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
            Self::LoadPreset => "loadPreset",
            Self::SaveCurrentPreset => "saveCurrentPreset",
            Self::GetAlsaDevices => "getAlsaDevices",
            Self::GetJackStatus => "getJackStatus",
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
                | Self::LoadPreset
                | Self::SaveCurrentPreset
                | Self::Restart
                | Self::Shutdown
        )
    }

    /// Whether this operation only queries PiPedal state.
    #[must_use]
    pub const fn is_read_only(self) -> bool {
        matches!(self, Self::GetAlsaDevices | Self::GetJackStatus)
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
            Self::LoadPreset | Self::SaveCurrentPreset => "presets",
            Self::GetAlsaDevices | Self::GetJackStatus => "diagnostics",
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
    #[serde(rename = "replyTo")]
    pub reply_to: Option<u64>,
}

/// PiPedal error reply body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorBody {
    /// Human-readable server error.
    pub message: String,
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
pub const fn startup_requests() -> [&'static str; 9] {
    [
        "hello",
        "version",
        "imageList",
        "plugins",
        "currentPedalboard",
        "pluginClasses",
        "getPresets",
        "getBankIndex",
        "getSystemMidiBindings",
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
                | "getFavorites" | "imageList",
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
    pub client_id: String,
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
        if self.client_id.is_empty() || self.client_id.len() > 128 || self.symbol.is_empty() {
            return Err("PiPedal setControl identity is invalid".into());
        }
        if !self.value.is_finite() {
            return Err("PiPedal setControl value is not finite".into());
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
#[derive(Clone, Debug, Default, PartialEq)]
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
        let control = self
            .find_control(&mapping.plugin_uri, &mapping.symbol)
            .ok_or_else(|| "PiPedal mapping control is unavailable".to_string())?;
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
    let header = serde_json::json!({
        "message": request.message,
        "replyTo": request.reply_to,
    });
    match &request.body {
        Some(body) => serde_json::to_vec(&serde_json::json!([header, body])),
        None => serde_json::to_vec(&serde_json::json!([header])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_control_uses_pipedal_wire_names() {
        let request = Request {
            message: "setControl".into(),
            reply_to: Some(7),
            body: Some(SetControl {
                client_id: "mackes".into(),
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
        assert_eq!(queue.pop(), Some(b"one".to_vec()));
        assert_eq!(queue.pop(), Some(b"two".to_vec()));
        assert_eq!(queue.pop(), None);
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
    }

    #[test]
    fn session_connect_starts_handshake_once() {
        let mut session = Session::default();
        session.connect().expect("connect");
        assert_eq!(session.phase(), SessionPhase::Connected);
        assert!(session.connect().is_err());
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
    fn session_requires_hello_and_version_before_catalog_ready() {
        assert_eq!(SessionPhase::Connected.accept("ehlo").expect("ehlo"), SessionPhase::Identified);
        assert!(SessionPhase::Connected.accept("hello").is_err());
        assert!(SessionPhase::Connected.accept("getSystemMidiBindings").is_err());
        let phase = SessionPhase::Identified.accept("version").expect("version");
        let phase = phase.accept("plugins").expect("plugins");
        assert_eq!(phase.accept("getSystemMidiBindings").expect("bindings"), SessionPhase::Ready);
        assert_eq!(phase.reset(), SessionPhase::Disconnected);
    }

    #[test]
    fn startup_plan_is_ordered_and_bounded() {
        let requests = startup_requests();
        assert_eq!(requests[0], "hello");
        assert_eq!(requests[1], "version");
        assert_eq!(requests.last(), Some(&"getSystemMidiBindings"));
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
        assert_eq!(Operation::all().len(), 18);
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
    }
}
