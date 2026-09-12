//! Shared bounded contracts for the port-8081 web adapter.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

/// Maximum operation/request identity length accepted at the web boundary.
pub const MAX_ID_LENGTH: usize = 96;
/// Maximum operation name length accepted at the web boundary.
pub const MAX_OPERATION_LENGTH: usize = 96;
/// Maximum structured error-code length accepted at the web boundary.
pub const MAX_ERROR_CODE_LENGTH: usize = 64;
/// Maximum state-event kind length accepted at the web boundary.
pub const MAX_EVENT_KIND_LENGTH: usize = 64;
/// Maximum structured error text length accepted at the web boundary.
pub const MAX_ERROR_LENGTH: usize = 512;
/// Maximum number of queued state events for one web client.
pub const MAX_EVENTS_PER_CLIENT: usize = 256;
/// Maximum number of feature capabilities advertised by one device.
pub const MAX_DEVICE_FEATURES: usize = 512;
/// Maximum number of devices in one Studio capability snapshot.
pub const MAX_STUDIO_DEVICES: usize = 128;
/// Maximum HTTP request header bytes accepted by the web boundary.
pub const MAX_HTTP_HEADER_BYTES: usize = 16 * 1024;
/// Maximum HTTP request body bytes accepted by the web boundary.
pub const MAX_HTTP_BODY_BYTES: usize = 1_048_576;

/// A bounded, already-framed HTTP request for the local web adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpRequest {
    /// Request method, such as `GET` or `POST`.
    pub method: String,
    /// Origin-form request target.
    pub path: String,
    /// Lowercase header names and their values.
    pub headers: Vec<(String, String)>,
    /// Request body bytes.
    pub body: Vec<u8>,
}

/// A bounded HTTP response assembled by the web adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpResponse {
    /// Numeric HTTP status code.
    pub status: u16,
    /// Response body bytes.
    pub body: Vec<u8>,
    /// Whether the body is JSON rather than bundled text/HTML.
    pub json: bool,
    content_type: Option<&'static str>,
}

impl HttpResponse {
    /// Creates a response with a bounded body.
    ///
    /// # Errors
    ///
    /// Returns an error when the status is not valid or the body exceeds the web boundary limit.
    pub fn new(status: u16, body: Vec<u8>, json: bool) -> Result<Self, &'static str> {
        if !(100..=599).contains(&status) {
            return Err("HTTP status is invalid");
        }
        if body.len() > MAX_HTTP_BODY_BYTES {
            return Err("HTTP response body is oversized");
        }
        Ok(Self { status, body, json, content_type: None })
    }

    /// Creates a response for a bundled asset with an explicit media type.
    ///
    /// # Errors
    ///
    /// Returns an error when the status is invalid or the body is oversized.
    pub fn asset(
        status: u16,
        body: Vec<u8>,
        content_type: &'static str,
    ) -> Result<Self, &'static str> {
        let mut response = Self::new(status, body, false)?;
        response.content_type = Some(content_type);
        Ok(response)
    }

    /// Encodes a complete HTTP/1.1 response with restrictive browser headers.
    ///
    /// # Errors
    ///
    /// Returns an error when the status code has no supported reason phrase.
    pub fn encode(&self) -> Result<Vec<u8>, &'static str> {
        let reason = match self.status {
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            204 => "No Content",
            400 => "Bad Request",
            404 => "Not Found",
            409 => "Conflict",
            413 => "Payload Too Large",
            415 => "Unsupported Media Type",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            500 => "Internal Server Error",
            503 => "Service Unavailable",
            _ => return Err("unsupported HTTP status"),
        };
        let content_type = self.content_type.unwrap_or(if self.json {
            "application/json"
        } else {
            "text/html; charset=utf-8"
        });
        let header = format!(
            "HTTP/1.1 {} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; connect-src 'self'\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
            self.status,
            self.body.len()
        );
        Ok(header.into_bytes().into_iter().chain(self.body.iter().copied()).collect())
    }
}

impl HttpRequest {
    /// Parses one complete HTTP/1.1 request with strict size and framing limits.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed request lines, unsupported framing, oversized headers or
    /// bodies, duplicate content lengths, or missing browser origin checks on mutations.
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > MAX_HTTP_HEADER_BYTES + MAX_HTTP_BODY_BYTES {
            return Err("HTTP request is oversized");
        }
        let separator = bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or("HTTP headers are incomplete")?;
        if separator + 4 > MAX_HTTP_HEADER_BYTES {
            return Err("HTTP headers are oversized");
        }
        let header_text =
            std::str::from_utf8(&bytes[..separator]).map_err(|_| "HTTP headers are not UTF-8")?;
        let mut lines = header_text.split("\r\n");
        let request_line = lines.next().ok_or("HTTP request line is missing")?;
        let mut parts = request_line.split_ascii_whitespace();
        let method = parts.next().ok_or("HTTP method is missing")?;
        let path = parts.next().ok_or("HTTP path is missing")?;
        if parts.next() != Some("HTTP/1.1") || parts.next().is_some() {
            return Err("unsupported HTTP request line");
        }
        if !matches!(method, "GET" | "POST" | "OPTIONS")
            || path.is_empty()
            || !path.starts_with('/')
        {
            return Err("unsupported HTTP method or path");
        }
        let mut headers = Vec::new();
        for line in lines {
            let (name, value) = line.split_once(':').ok_or("malformed HTTP header")?;
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim().to_owned();
            if name.is_empty() || name.chars().any(|character| character.is_ascii_whitespace()) {
                return Err("malformed HTTP header name");
            }
            if headers.iter().any(|(existing, _)| existing == &name) {
                return Err("duplicate HTTP header");
            }
            headers.push((name, value));
        }
        let content_length = headers
            .iter()
            .find(|(name, _)| name == "content-length")
            .map(|(_, value)| value.parse::<usize>().map_err(|_| "invalid HTTP content length"))
            .transpose()?
            .unwrap_or(0);
        if headers.iter().any(|(name, _)| name == "transfer-encoding") {
            return Err("transfer encoding is unsupported");
        }
        if content_length > MAX_HTTP_BODY_BYTES || bytes.len() - separator - 4 != content_length {
            return Err("HTTP body length is invalid");
        }
        if method == "POST" && headers.iter().all(|(name, _)| name != "origin") {
            return Err("browser mutation requires an origin header");
        }
        Ok(Self {
            method: method.to_owned(),
            path: path.to_owned(),
            headers,
            body: bytes[separator + 4..].to_vec(),
        })
    }

    /// Validates the host and, for mutations, the browser origin against one configured origin.
    ///
    /// # Errors
    ///
    /// Returns an error when the host is absent/mismatched or a mutation origin is absent or
    /// mismatched. The expected value should be an origin such as `http://host:8081`.
    pub fn validate_same_origin(
        &self,
        expected_host: &str,
        expected_origin: &str,
    ) -> Result<(), &'static str> {
        let host = self
            .headers
            .iter()
            .find(|(name, _)| name == "host")
            .map(|(_, value)| value.as_str())
            .ok_or("HTTP Host header is missing")?;
        if host != expected_host {
            return Err("HTTP Host header does not match configured host");
        }
        if self.method == "POST" {
            let origin = self
                .headers
                .iter()
                .find(|(name, _)| name == "origin")
                .map(|(_, value)| value.as_str())
                .ok_or("browser mutation requires an origin header")?;
            if origin != expected_origin {
                return Err("browser Origin does not match configured origin");
            }
        }
        Ok(())
    }
}

/// Typed mutation envelope shared by HTTP and future generated clients.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[serde(deny_unknown_fields)]
pub struct OperationRequest {
    /// Client-supplied correlation identity.
    pub request_id: String,
    /// Stable canonical operation name.
    pub operation: String,
    /// Daemon generation the client observed.
    pub generation: u64,
    /// Explicit confirmation for elevated operations.
    #[serde(default)]
    pub confirm: bool,
    /// Typed operation payload, interpreted by the daemon contract.
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
}

impl OperationRequest {
    /// Validates bounded identity fields and rejects scalar payloads.
    ///
    /// # Errors
    ///
    /// Returns an error when an identity is empty/oversized or the payload is not an object.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.request_id.is_empty() || self.request_id.len() > MAX_ID_LENGTH {
            return Err("request_id is empty or oversized");
        }
        if self.operation.is_empty() || self.operation.len() > MAX_OPERATION_LENGTH {
            return Err("operation is empty or oversized");
        }
        if self.payload.as_ref().is_some_and(|payload| !payload.is_object()) {
            return Err("payload must be an object");
        }
        Ok(())
    }
}

/// Result returned after an operation is accepted by the daemon.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationResponse {
    /// Durable/ongoing operation identity.
    pub operation_id: String,
    /// Current daemon generation.
    pub generation: u64,
    /// Whether the daemon accepted the operation.
    pub accepted: bool,
}

impl OperationResponse {
    /// Validates the bounded operation identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the operation identity is empty or oversized.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.operation_id.is_empty() || self.operation_id.len() > MAX_ID_LENGTH {
            return Err("operation_id is empty or oversized");
        }
        Ok(())
    }
}

/// Structured error returned by the web adapter.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorResponse {
    /// Stable machine-readable error code.
    pub code: String,
    /// Bounded human-readable message.
    pub message: String,
    /// Optional invalid field.
    pub field: Option<String>,
    /// Owning work item when the operation is not implemented.
    pub work_item: Option<String>,
}

impl ErrorResponse {
    /// Validates bounded structured error fields.
    ///
    /// # Errors
    ///
    /// Returns an error when the code, message, field, or work-item values exceed their limits.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.code.is_empty() || self.code.len() > MAX_ERROR_CODE_LENGTH {
            return Err("error code is empty or oversized");
        }
        if self.message.is_empty() || self.message.len() > MAX_ERROR_LENGTH {
            return Err("error message is empty or oversized");
        }
        if self.field.as_ref().is_some_and(|field| field.len() > MAX_ID_LENGTH) {
            return Err("error field is oversized");
        }
        if self.work_item.as_ref().is_some_and(|work_item| work_item.len() > MAX_ID_LENGTH) {
            return Err("work item is oversized");
        }
        Ok(())
    }
}

/// Sequenced event delivered by the bounded state stream.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[serde(deny_unknown_fields)]
pub struct StateEvent {
    /// Monotonic event sequence.
    pub sequence: u64,
    /// Daemon generation associated with this event.
    pub generation: u64,
    /// Stable event kind.
    pub kind: String,
    /// Structured event payload.
    pub payload: serde_json::Value,
}

impl StateEvent {
    /// Validates event identity and object payload shape.
    ///
    /// # Errors
    ///
    /// Returns an error when the event kind is empty/oversized or its payload is not an object.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.kind.is_empty() || self.kind.len() > MAX_EVENT_KIND_LENGTH {
            return Err("event kind is empty or oversized");
        }
        if !self.payload.is_object() {
            return Err("event payload must be an object");
        }
        Ok(())
    }
}

/// Bounded FIFO used by a web client so a slow consumer cannot grow daemon memory.
#[derive(Debug, Default)]
pub struct BoundedEventQueue {
    events: VecDeque<StateEvent>,
}

impl BoundedEventQueue {
    /// Creates an empty client queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueues an event, returning it to the caller when the client is too slow.
    ///
    /// # Errors
    ///
    /// Returns the event unchanged when the queue has reached its per-client bound.
    pub fn push(&mut self, event: StateEvent) -> Result<(), StateEvent> {
        if self.events.len() >= MAX_EVENTS_PER_CLIENT {
            return Err(event);
        }
        self.events.push_back(event);
        Ok(())
    }

    /// Removes and returns the oldest queued event.
    pub fn pop(&mut self) -> Option<StateEvent> {
        self.events.pop_front()
    }

    /// Returns the number of queued events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns whether no events are queued.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Truthful lifecycle state for a discovered device.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceLifecycle {
    /// The system is identifying the device or probing its capabilities.
    Detecting,
    /// The device and its advertised capabilities are usable.
    Ready,
    /// The device is present but one or more capabilities are unavailable.
    Limited,
    /// The previously identified device is not currently reachable.
    Disconnected,
    /// More than one candidate prevents safe binding.
    Ambiguous,
    /// Discovery or transport failed with an actionable error.
    Error,
}

/// Truth state for a value shown by the Studio UI.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FeedbackTruth {
    /// The value was observed from the device or authoritative service.
    #[serde(rename = "observed")]
    Observed,
    /// The authoritative service accepted the requested operation.
    #[serde(rename = "acknowledged")]
    Acknowledged,
    /// The value was sent, but the destination cannot confirm it.
    #[serde(rename = "sent-unverified")]
    SentUnverified,
    /// A previously observed value retained after its freshness deadline.
    #[serde(rename = "last-known")]
    LastKnown,
    /// The value's freshness deadline expired.
    #[serde(rename = "stale")]
    Stale,
    /// The feature cannot currently provide a value.
    #[serde(rename = "unavailable")]
    Unavailable,
}

/// Capability flags for one named device feature.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)]
pub struct DeviceFeatureCapability {
    /// Stable internal feature key; the normal UI uses `label`.
    pub key: String,
    /// Musician-facing feature name.
    pub label: String,
    /// Whether an authoritative current value can be observed.
    pub readable: bool,
    /// Whether a validated mutation can be sent.
    pub writable: bool,
    /// Whether the device can be queried when no subscription exists.
    pub queryable: bool,
    /// Whether updates can arrive through the event stream.
    pub subscribable: bool,
    /// Whether the feature represents a continuously updating meter.
    pub meter: bool,
    /// Whether hardware feedback can be projected for this feature.
    pub led_feedback: bool,
    /// Qualification level for the advertised behavior.
    pub qualification: String,
    /// Plain-language reason when a capability is limited or unavailable.
    pub unavailable_reason: Option<String>,
}

impl DeviceFeatureCapability {
    /// Validates bounded identity, labels, and capability relationships.
    ///
    /// # Errors
    ///
    /// Returns an error when a field is empty/oversized or the capability flags contradict one
    /// another.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.key.is_empty() || self.key.len() > MAX_ID_LENGTH {
            return Err("device feature key is empty or oversized");
        }
        if self.label.is_empty() || self.label.len() > 128 {
            return Err("device feature label is empty or oversized");
        }
        if self.qualification.is_empty() || self.qualification.len() > MAX_ID_LENGTH {
            return Err("device feature qualification is empty or oversized");
        }
        if self.unavailable_reason.as_ref().is_some_and(|reason| reason.len() > MAX_ERROR_LENGTH) {
            return Err("device feature unavailable reason is oversized");
        }
        if self.writable && !self.readable && self.unavailable_reason.is_none() {
            return Err("write-only device feature needs an explicit feedback reason");
        }
        if self.meter && self.led_feedback {
            return Err("meter capability cannot claim controller LED feedback");
        }
        Ok(())
    }
}

/// One value observation retained independently from saved assignment state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[serde(deny_unknown_fields)]
pub struct DeviceFeedbackObservation {
    /// Feature key owning this value.
    pub feature_key: String,
    /// Truth classification for the displayed value.
    pub truth: FeedbackTruth,
    /// Device or service value, if one is available.
    pub value: Option<serde_json::Value>,
    /// Monotonic observation timestamp supplied by the authority.
    pub observed_at_ms: Option<u64>,
    /// Source label such as `device`, `daemon`, or `browser`.
    pub source: String,
}

impl DeviceFeedbackObservation {
    /// Validates bounded source and feature identity and object value shape.
    ///
    /// # Errors
    ///
    /// Returns an error when identity/source bounds fail or the value has an unsupported shape.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.feature_key.is_empty() || self.feature_key.len() > MAX_ID_LENGTH {
            return Err("feedback feature key is empty or oversized");
        }
        if self.source.is_empty() || self.source.len() > MAX_ID_LENGTH {
            return Err("feedback source is empty or oversized");
        }
        if self.value.as_ref().is_some_and(serde_json::Value::is_array) {
            return Err("feedback value cannot be an array");
        }
        Ok(())
    }
}

/// Versioned capability and observed-state projection for one device.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[serde(deny_unknown_fields)]
pub struct StudioDeviceCapability {
    /// Stable device identity used for reconnect and mapping joins.
    pub stable_id: String,
    /// Musician-facing device name.
    pub label: String,
    /// Renderer family, such as `novation.launch-control-xl` or `generic.endpoint`.
    pub renderer: String,
    /// Current discovery/transport lifecycle.
    pub lifecycle: DeviceLifecycle,
    /// Generation that produced this projection.
    pub generation: u64,
    /// Features available on this exact device instance.
    pub features: Vec<DeviceFeatureCapability>,
    /// Last-known or current feature values.
    pub observations: Vec<DeviceFeedbackObservation>,
}

impl StudioDeviceCapability {
    /// Validates all bounded fields and enforces unique feature/observation keys.
    ///
    /// # Errors
    ///
    /// Returns an error when device fields or child projections are invalid, oversized, or
    /// duplicated.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.stable_id.is_empty() || self.stable_id.len() > MAX_ID_LENGTH {
            return Err("device stable identity is empty or oversized");
        }
        if self.label.is_empty() || self.label.len() > 128 {
            return Err("device label is empty or oversized");
        }
        if self.renderer.is_empty() || self.renderer.len() > MAX_ID_LENGTH {
            return Err("device renderer is empty or oversized");
        }
        if self.features.len() > MAX_DEVICE_FEATURES
            || self.observations.len() > MAX_DEVICE_FEATURES
        {
            return Err("device capability projection is oversized");
        }
        let mut feature_keys = BTreeSet::new();
        for feature in &self.features {
            feature.validate()?;
            if !feature_keys.insert(&feature.key) {
                return Err("device feature keys are duplicated");
            }
        }
        let mut observation_keys = BTreeSet::new();
        for observation in &self.observations {
            observation.validate()?;
            if !observation_keys.insert(&observation.feature_key) {
                return Err("device feedback feature keys are duplicated");
            }
        }
        Ok(())
    }
}

/// Versioned bounded Studio capability snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[serde(deny_unknown_fields)]
pub struct StudioCapabilitySnapshot {
    /// Independent contract version.
    pub schema_version: u16,
    /// Authoritative daemon generation.
    pub generation: u64,
    /// Last event included in this snapshot.
    pub event_sequence: u64,
    /// Exact devices represented by this snapshot.
    pub devices: Vec<StudioDeviceCapability>,
}

impl StudioCapabilitySnapshot {
    /// Validates the version and every device projection.
    ///
    /// # Errors
    ///
    /// Returns an error when the schema version, size bounds, device projection, or stable identity
    /// uniqueness requirement fails.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1 {
            return Err("unsupported Studio capability schema version");
        }
        if self.devices.len() > MAX_STUDIO_DEVICES {
            return Err("Studio capability snapshot is oversized");
        }
        let mut identities = BTreeSet::new();
        for device in &self.devices {
            device.validate()?;
            if !identities.insert(&device.stable_id) {
                return Err("Studio device identities are duplicated");
            }
        }
        Ok(())
    }
}

/// Typed event families consumed by the live Studio surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudioEventKind {
    /// A device was discovered, changed, connected, or disconnected.
    DeviceLifecycle,
    /// A physical or software control produced an observed value.
    ControlObservation,
    /// A device or service changed its active preset.
    PresetChanged,
    /// A device changed its active algorithm or operating mode.
    ModeChanged,
    /// A continuously updating level or activity measurement.
    Meter,
    /// A saved mapping or mapping lifecycle state changed.
    MappingChanged,
    /// The active scene changed or was recalled.
    SceneChanged,
    /// The effective controller layer changed.
    LayerChanged,
    /// The daemon projected desired controller LED intent.
    LedIntent,
    /// The controller backend reported LED delivery progress.
    LedDelivery,
    /// The client must obtain a fresh authoritative snapshot.
    SnapshotRequired,
}

/// Bounded, sequence-aware Studio event envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[serde(deny_unknown_fields)]
pub struct StudioEvent {
    /// Monotonic event sequence used for reconnect and gap detection.
    pub sequence: u64,
    /// Daemon generation associated with the event.
    pub generation: u64,
    /// Typed event family.
    pub kind: StudioEventKind,
    /// Stable device identity, when the event belongs to one device.
    pub device_id: Option<String>,
    /// Stable feature/control identity, when the event belongs to one feature.
    pub feature_key: Option<String>,
    /// Structured event details.
    pub payload: serde_json::Value,
}

impl StudioEvent {
    /// Validates sequence, identities, and bounded object payload.
    ///
    /// # Errors
    ///
    /// Returns an error when the sequence is zero, an identity is oversized, or the payload is not
    /// a JSON object.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.sequence == 0 {
            return Err("Studio event sequence must be nonzero");
        }
        for identity in [self.device_id.as_ref(), self.feature_key.as_ref()].into_iter().flatten() {
            if identity.is_empty() || identity.len() > MAX_ID_LENGTH {
                return Err("Studio event identity is empty or oversized");
            }
        }
        if !self.payload.is_object() {
            return Err("Studio event payload must be an object");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_request_is_bounded_and_strict() {
        let request = OperationRequest {
            request_id: "web-1".into(),
            operation: "mappings.snapshot".into(),
            generation: 4,
            confirm: false,
            payload: Some(serde_json::json!({})),
        };
        request.validate().expect("valid request");
        assert!(serde_json::from_str::<OperationRequest>(
            r#"{"request_id":"x","operation":"y","generation":0,"extra":true}"#
        )
        .is_err());
        assert!(OperationRequest { payload: Some(serde_json::json!(true)), ..request }
            .validate()
            .is_err());
    }

    #[test]
    fn response_error_and_event_round_trip() {
        let response =
            OperationResponse { operation_id: "op-1".into(), generation: 2, accepted: true };
        let error = ErrorResponse {
            code: "conflict".into(),
            message: "stale".into(),
            field: None,
            work_item: None,
        };
        response.validate().expect("valid response");
        error.validate().expect("valid error");
        let event = StateEvent {
            sequence: 3,
            generation: 2,
            kind: "snapshot".into(),
            payload: serde_json::json!({"ok": true}),
        };
        event.validate().expect("valid event");
        assert_eq!(
            serde_json::from_str::<OperationResponse>(
                &serde_json::to_string(&response).expect("encode")
            )
            .expect("decode"),
            response
        );
        assert_eq!(
            serde_json::from_str::<ErrorResponse>(&serde_json::to_string(&error).expect("encode"))
                .expect("decode"),
            error
        );
        assert_eq!(
            serde_json::from_str::<StateEvent>(&serde_json::to_string(&event).expect("encode"))
                .expect("decode"),
            event
        );
    }

    #[test]
    fn event_queue_rejects_slow_consumers_at_bound() {
        let mut queue = BoundedEventQueue::new();
        for sequence in 0..MAX_EVENTS_PER_CLIENT as u64 {
            queue
                .push(StateEvent {
                    sequence,
                    generation: 1,
                    kind: "state".into(),
                    payload: serde_json::json!({}),
                })
                .expect("queue has capacity");
        }
        let rejected = queue
            .push(StateEvent {
                sequence: MAX_EVENTS_PER_CLIENT as u64,
                generation: 1,
                kind: "state".into(),
                payload: serde_json::json!({}),
            })
            .expect_err("full queue rejects event");
        assert_eq!(rejected.sequence, MAX_EVENTS_PER_CLIENT as u64);
        assert_eq!(queue.len(), MAX_EVENTS_PER_CLIENT);
        assert_eq!(queue.pop().expect("oldest event").sequence, 0);
    }

    #[test]
    fn studio_capability_snapshot_round_trips_and_validates() {
        let snapshot = StudioCapabilitySnapshot {
            schema_version: 1,
            generation: 9,
            event_sequence: 44,
            devices: vec![StudioDeviceCapability {
                stable_id: "novation:1235:0061".into(),
                label: "Launch Control XL".into(),
                renderer: "novation.launch-control-xl".into(),
                lifecycle: DeviceLifecycle::Ready,
                generation: 9,
                features: vec![DeviceFeatureCapability {
                    key: "knob-1".into(),
                    label: "Drive".into(),
                    readable: true,
                    writable: true,
                    queryable: false,
                    subscribable: true,
                    meter: false,
                    led_feedback: true,
                    qualification: "qualified".into(),
                    unavailable_reason: None,
                }],
                observations: vec![DeviceFeedbackObservation {
                    feature_key: "knob-1".into(),
                    truth: FeedbackTruth::Observed,
                    value: Some(serde_json::json!(0.5)),
                    observed_at_ms: Some(1200),
                    source: "device".into(),
                }],
            }],
        };
        snapshot.validate().expect("valid capability snapshot");
        let encoded = serde_json::to_string(&snapshot).expect("encode snapshot");
        let decoded: StudioCapabilitySnapshot =
            serde_json::from_str(&encoded).expect("decode snapshot");
        assert_eq!(decoded, snapshot);
        assert_eq!(
            serde_json::to_string(&FeedbackTruth::SentUnverified).expect("encode truth"),
            "\"sent-unverified\""
        );
    }

    #[test]
    fn studio_capability_validation_rejects_duplicates_and_unsafe_claims() {
        let feature = DeviceFeatureCapability {
            key: "meter".into(),
            label: "Level".into(),
            readable: true,
            writable: true,
            queryable: false,
            subscribable: true,
            meter: true,
            led_feedback: true,
            qualification: "qualified".into(),
            unavailable_reason: None,
        };
        assert_eq!(
            feature.validate(),
            Err("meter capability cannot claim controller LED feedback")
        );
        let valid_feature = DeviceFeatureCapability { led_feedback: false, ..feature };
        let device = StudioDeviceCapability {
            stable_id: "device".into(),
            label: "Device".into(),
            renderer: "generic.endpoint".into(),
            lifecycle: DeviceLifecycle::Limited,
            generation: 0,
            features: vec![valid_feature.clone(), valid_feature],
            observations: Vec::new(),
        };
        assert_eq!(device.validate(), Err("device feature keys are duplicated"));
    }

    #[test]
    fn studio_event_is_typed_sequence_aware_and_bounded() {
        let event = StudioEvent {
            sequence: 7,
            generation: 3,
            kind: StudioEventKind::ControlObservation,
            device_id: Some("novation:1235:0061".into()),
            feature_key: Some("knob-1".into()),
            payload: serde_json::json!({"value": 0.75}),
        };
        event.validate().expect("valid Studio event");
        let encoded = serde_json::to_string(&event).expect("encode event");
        assert!(encoded.contains("control_observation"));
        assert_eq!(serde_json::from_str::<StudioEvent>(&encoded).expect("decode event"), event);
        assert_eq!(
            (StudioEvent { sequence: 0, ..event.clone() }).validate(),
            Err("Studio event sequence must be nonzero")
        );
        assert_eq!(
            (StudioEvent { payload: serde_json::json!(true), ..event }).validate(),
            Err("Studio event payload must be an object")
        );
    }

    #[test]
    fn http_parser_bounds_mutations_and_body_framing() {
        let request = HttpRequest::parse(
            b"POST /api/v1/operations HTTP/1.1\r\nHost: localhost:8081\r\nOrigin: http://localhost:8081\r\nContent-Length: 2\r\n\r\n{}",
        )
        .expect("valid request");
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/api/v1/operations");
        assert_eq!(request.body, b"{}");
        request
            .validate_same_origin("localhost:8081", "http://localhost:8081")
            .expect("same origin");
        assert!(request.validate_same_origin("evil.example", "http://localhost:8081").is_err());
        assert!(HttpRequest::parse(
            b"POST /api/v1/operations HTTP/1.1\r\nHost: localhost\r\nContent-Length: 2\r\n\r\n{}"
        )
        .is_err());
        assert!(
            HttpRequest::parse(b"GET /api/v1/state HTTP/1.1\r\nHost: localhost\r\n\r\n").is_ok()
        );
        assert!(HttpRequest::parse(
            b"POST /api/v1/operations HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\nTransfer-Encoding: chunked\r\n\r\n"
        )
        .is_err());
    }

    #[test]
    fn http_response_has_safe_framing_and_security_headers() {
        let response = HttpResponse::new(200, br#"{"ok":true}"#.to_vec(), true).expect("response");
        let encoded = String::from_utf8(response.encode().expect("encode")).expect("UTF-8");
        assert!(encoded.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(encoded.contains("Content-Type: application/json\r\n"));
        assert!(encoded.contains("X-Content-Type-Options: nosniff\r\n"));
        assert!(
            encoded.contains("Content-Security-Policy: default-src 'self'; connect-src 'self'\r\n")
        );
        assert!(encoded.ends_with("\r\n\r\n{\"ok\":true}"));
        assert!(HttpResponse::new(200, vec![0; MAX_HTTP_BODY_BYTES + 1], false).is_err());
    }

    #[test]
    fn http_response_encodes_statuses_used_by_web_handlers() {
        for (status, reason) in [
            (202, "Accepted"),
            (415, "Unsupported Media Type"),
            (501, "Not Implemented"),
            (502, "Bad Gateway"),
        ] {
            let response = HttpResponse::new(status, Vec::new(), true).expect("response");
            let encoded = String::from_utf8(response.encode().expect("encode")).expect("UTF-8");
            assert!(encoded.starts_with(&format!("HTTP/1.1 {status} {reason}\r\n")));
        }
    }
}
