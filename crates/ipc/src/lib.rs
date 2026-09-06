//! Versioned local daemon/client framing boundary.

use mackes_config::{ControlMapping, ControlMappingDraft, MappingBehavior};
use serde::{Deserialize, Serialize};

use std::{
    io::{self, Read, Write},
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::{
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
};

#[cfg(target_os = "linux")]
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};

/// Major version of the local IPC envelope.
pub const PROTOCOL_MAJOR: u16 = 1;
/// Minor version of the local IPC envelope.
pub const PROTOCOL_MINOR: u16 = 0;
/// Maximum encoded envelope size accepted by default.
pub const MAX_FRAME_BYTES: usize = 1_048_576;
/// Maximum live-test request identifier length.
pub const MAX_LIVE_TEST_REQUEST_ID_BYTES: usize = 64;
/// Maximum operator-safe live-test reason length.
pub const MAX_LIVE_TEST_REASON_BYTES: usize = 256;
/// Maximum stable endpoint identity length in a live-test request.
pub const MAX_LIVE_TEST_ENDPOINT_ID_BYTES: usize = 128;

/// Terminal status returned by a daemon-owned MIDI Learn live test.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiveTestStatus {
    /// The daemon observed the expected test result.
    Passed,
    /// The test completed with a negative result.
    Failed,
    /// The bounded test deadline elapsed.
    TimedOut,
    /// Safety policy refused the operation.
    Denied,
    /// The selected profile cannot perform or verify the test.
    Unavailable,
}

impl LiveTestStatus {
    /// Returns the stable wire label.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::TimedOut => "timed_out",
            Self::Denied => "denied",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Validated daemon-owned MIDI Learn live-test request metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveTestRequest {
    /// Idempotency/correlation identifier.
    pub request_id: String,
    /// Stable source endpoint identity.
    pub source_endpoint_id: String,
    /// Stable destination identity.
    pub destination_id: String,
    /// Captured MIDI message family.
    pub candidate_kind: String,
    /// Captured controller/note/program number, when applicable.
    pub candidate_number: Option<u8>,
    /// Captured MIDI channel, when applicable.
    pub candidate_channel: Option<u8>,
    /// Optimistic routing/configuration generation.
    pub generation: u64,
}

impl LiveTestRequest {
    /// Decodes and validates a strict JSON request payload.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed JSON, missing fields, unknown fields, or invalid bounds.
    pub fn from_json(bytes: &[u8]) -> Result<Self, &'static str> {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| "invalid live-test request JSON")?;
        let object = value.as_object().ok_or("live-test request must be an object")?;
        if object.keys().any(|key| {
            !matches!(
                key.as_str(),
                "request_id"
                    | "source_endpoint_id"
                    | "destination_id"
                    | "candidate_kind"
                    | "candidate_number"
                    | "candidate_channel"
                    | "generation"
            )
        }) {
            return Err("unknown live-test request field");
        }
        Self {
            request_id: object
                .get("request_id")
                .and_then(serde_json::Value::as_str)
                .ok_or("live-test request_id is required")?
                .to_owned(),
            source_endpoint_id: object
                .get("source_endpoint_id")
                .and_then(serde_json::Value::as_str)
                .ok_or("live-test source endpoint is required")?
                .to_owned(),
            destination_id: object
                .get("destination_id")
                .and_then(serde_json::Value::as_str)
                .ok_or("live-test destination is required")?
                .to_owned(),
            candidate_kind: object
                .get("candidate_kind")
                .and_then(serde_json::Value::as_str)
                .ok_or("live-test candidate kind is required")?
                .to_owned(),
            candidate_number: object
                .get("candidate_number")
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| u8::try_from(value).ok()),
            candidate_channel: object
                .get("candidate_channel")
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| u8::try_from(value).ok()),
            generation: object
                .get("generation")
                .and_then(serde_json::Value::as_u64)
                .ok_or("live-test generation is required")?,
        }
        .validate()
    }

    /// Validates bounded non-empty request identity fields.
    ///
    /// # Errors
    ///
    /// Returns an error when an identity field is empty or exceeds its bound.
    pub fn validate(self) -> Result<Self, &'static str> {
        if self.request_id.is_empty() || self.request_id.len() > MAX_LIVE_TEST_REQUEST_ID_BYTES {
            return Err("live-test request identifier is empty or oversized");
        }
        if self.source_endpoint_id.is_empty()
            || self.source_endpoint_id.len() > MAX_LIVE_TEST_ENDPOINT_ID_BYTES
        {
            return Err("live-test source endpoint is empty or oversized");
        }
        if self.destination_id.is_empty()
            || self.destination_id.len() > MAX_LIVE_TEST_ENDPOINT_ID_BYTES
        {
            return Err("live-test destination is empty or oversized");
        }
        if self.candidate_kind.is_empty() || self.candidate_kind.len() > 32 {
            return Err("live-test candidate kind is empty or oversized");
        }
        if self.candidate_channel.is_some_and(|channel| channel > 15) {
            return Err("live-test candidate channel is out of range");
        }
        if self.candidate_number.is_some_and(|number| number > 127) {
            return Err("live-test candidate number is out of range");
        }
        Ok(self)
    }
}

/// Terminal daemon result for a MIDI Learn live test.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveTestResult {
    /// Correlates the result to the request.
    pub request_id: String,
    /// Terminal operation status.
    pub status: LiveTestStatus,
    /// Bounded operator-safe explanation.
    pub reason: String,
    /// Redacted audit reference, when a decision was recorded.
    pub audit_reference: Option<String>,
}

impl LiveTestResult {
    /// Decodes and validates a strict JSON result payload.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed JSON, missing fields, unknown fields, or invalid bounds.
    pub fn from_json(bytes: &[u8]) -> Result<Self, &'static str> {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| "invalid live-test result JSON")?;
        let object = value.as_object().ok_or("live-test result must be an object")?;
        if object.keys().any(|key| {
            !matches!(key.as_str(), "request_id" | "status" | "reason" | "audit_reference")
        }) {
            return Err("unknown live-test result field");
        }
        let status = match object
            .get("status")
            .and_then(serde_json::Value::as_str)
            .ok_or("live-test status is required")?
        {
            "passed" => LiveTestStatus::Passed,
            "failed" => LiveTestStatus::Failed,
            "timed_out" => LiveTestStatus::TimedOut,
            "denied" => LiveTestStatus::Denied,
            "unavailable" => LiveTestStatus::Unavailable,
            _ => return Err("unknown live-test status"),
        };
        Self {
            request_id: object
                .get("request_id")
                .and_then(serde_json::Value::as_str)
                .ok_or("live-test result request_id is required")?
                .to_owned(),
            status,
            reason: object
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .ok_or("live-test result reason is required")?
                .to_owned(),
            audit_reference: object
                .get("audit_reference")
                .and_then(|value| value.as_str().map(str::to_owned)),
        }
        .validate()
    }

    /// Validates the bounded result metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when an identifier, reason, or audit reference exceeds its bound.
    pub fn validate(self) -> Result<Self, &'static str> {
        if self.request_id.is_empty() || self.request_id.len() > MAX_LIVE_TEST_REQUEST_ID_BYTES {
            return Err("live-test result identifier is empty or oversized");
        }
        if self.reason.len() > MAX_LIVE_TEST_REASON_BYTES {
            return Err("live-test result reason is oversized");
        }
        if self.audit_reference.as_ref().is_some_and(|value| value.len() > 128) {
            return Err("live-test audit reference is oversized");
        }
        Ok(self)
    }
}

/// Bounded token-bucket limiter for administrative IPC actions.
#[derive(Debug)]
pub struct RateLimiter {
    capacity: u32,
    tokens: f64,
    refill_per_second: f64,
    last: Instant,
}

impl RateLimiter {
    /// Creates a limiter; zero capacity or refill is rejected.
    #[must_use]
    pub fn new(capacity: u32, refill_per_second: u32) -> Option<Self> {
        if capacity == 0 || refill_per_second == 0 {
            return None;
        }
        Some(Self {
            capacity,
            tokens: f64::from(capacity),
            refill_per_second: f64::from(refill_per_second),
            last: Instant::now(),
        })
    }

    /// Attempts to consume one action token, returning whether it is allowed.
    pub fn allow(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_secs_f64();
        self.tokens =
            elapsed.mul_add(self.refill_per_second, self.tokens).min(f64::from(self.capacity));
        self.last = now;
        if self.tokens < 1.0 {
            return false;
        }
        self.tokens -= 1.0;
        true
    }

    /// Returns the duration until one token is expected to be available.
    #[must_use]
    pub fn retry_after(&self) -> Duration {
        if self.tokens >= 1.0 {
            Duration::ZERO
        } else {
            Duration::from_secs_f64((1.0 - self.tokens) / self.refill_per_second)
        }
    }
}

/// Commands available through local IPC. Network MIDI never carries these values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command {
    /// Exchange protocol version and capabilities.
    Hello,
    /// Retrieve a complete state snapshot.
    Snapshot,
    /// Subscribe to sequenced state events.
    Subscribe,
    /// Validate a configuration path.
    Validate,
    /// Load or save configuration.
    Configuration,
    /// Plan or apply verified endpoint migration.
    Migrate,
    /// Request an immediate bounded native endpoint rescan.
    Rescan,
    /// Inspect endpoint inventory.
    Endpoints,
    /// Inspect or mutate routes.
    Routes,
    /// Capture bounded observational MIDI Learn candidates.
    Learn,
    /// Inspect or activate scenes.
    Scenes,
    /// Query a profile-backed device.
    DeviceQuery,
    /// Send one profile-validated device control message.
    DeviceControl,
    /// Perform a `SysEx` operation.
    Sysex,
    /// Inspect or restore backups.
    Backups,
    /// Monitor events.
    Monitor,
    /// Retrieve health state.
    Health,
    /// Issue the emergency panic action.
    Panic,
    /// Arm or disarm temporary unsafe mode.
    UnsafeMode,
    /// Inspect or mutate durable hardware-first mappings.
    Mappings,
    /// Inspect or mutate the daemon-owned `PiPedal` connector.
    PiPedal,
    /// Drive the daemon-owned controller assignment session.
    Assignment,
    /// Request bounded daemon shutdown.
    Shutdown,
}

impl Command {
    /// Returns the stable wire tag.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Hello => "hello",
            Self::Snapshot => "snapshot",
            Self::Subscribe => "subscribe",
            Self::Validate => "validate",
            Self::Configuration => "configuration",
            Self::Migrate => "migrate",
            Self::Rescan => "rescan",
            Self::Endpoints => "endpoints",
            Self::Routes => "routes",
            Self::Learn => "learn",
            Self::Scenes => "scenes",
            Self::DeviceQuery => "device_query",
            Self::DeviceControl => "device_control",
            Self::Sysex => "sysex",
            Self::Backups => "backups",
            Self::Monitor => "monitor",
            Self::Health => "health",
            Self::Panic => "panic",
            Self::UnsafeMode => "unsafe_mode",
            Self::Mappings => "mappings",
            Self::PiPedal => "pipedal",
            Self::Assignment => "assignment",
            Self::Shutdown => "shutdown",
        }
    }
}

/// Typed `PiPedal` IPC operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiPedalOperation {
    /// Read health, catalog, and mapping resolution.
    Snapshot,
    /// Apply one explicitly confirmed control mutation.
    Apply,
    /// Request an explicitly confirmed restore of the last mutation.
    Undo,
}

/// Strict `PiPedal` IPC request envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PiPedalRequest {
    /// Requested operation.
    pub operation: PiPedalOperation,
    /// Expected connector session generation.
    pub generation: u64,
    /// Required for mutation operations.
    #[serde(default)]
    pub confirm: bool,
    /// Stable mapping identity for apply requests.
    #[serde(default)]
    pub mapping: Option<PiPedalMappingTarget>,
    /// Fresh runtime plugin instance ID for apply requests.
    #[serde(default)]
    pub instance_id: Option<u64>,
    /// Client identity used by `PiPedal`.
    #[serde(default)]
    pub client_id: Option<String>,
    /// Requested normalized/control-domain value.
    #[serde(default)]
    pub value: Option<f32>,
}

/// Stable mapping target carried by a `PiPedal` apply request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PiPedalMappingTarget {
    /// Physical control identity.
    pub physical_control_id: String,
    /// Plugin URI.
    pub plugin_uri: String,
    /// Plugin parameter symbol.
    pub symbol: String,
    /// Optional instance-selection scope.
    #[serde(default)]
    pub scope: Option<String>,
}

/// Actor class attached to every mutation for policy and audit decisions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorClass {
    /// Interactive TUI actor.
    LocalTui,
    /// Interactive CLI actor.
    LocalCli,
    /// Automatic daemon startup restore.
    StartupRestore,
    /// MIDI mapping actor.
    MidiMapping,
    /// RTP-MIDI input actor.
    RtpMidi,
}

/// Result of centralized IPC command authorization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Authorization {
    /// Command may be dispatched.
    Allowed,
    /// Actor may not invoke administrative IPC.
    Denied,
}

/// Applies the local-only administrative boundary before command dispatch.
#[must_use]
pub const fn authorize(command: Command, actor: ActorClass) -> Authorization {
    match actor {
        ActorClass::RtpMidi => Authorization::Denied,
        ActorClass::MidiMapping => match command {
            Command::Hello | Command::Snapshot | Command::Subscribe | Command::Health => {
                Authorization::Allowed
            }
            _ => Authorization::Denied,
        },
        ActorClass::StartupRestore => match command {
            Command::Scenes | Command::Health | Command::DeviceQuery => Authorization::Allowed,
            _ => Authorization::Denied,
        },
        ActorClass::LocalTui | ActorClass::LocalCli => Authorization::Allowed,
    }
}

impl ActorClass {
    /// Returns the stable audit tag.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::LocalTui => "local_tui",
            Self::LocalCli => "local_cli",
            Self::StartupRestore => "startup_restore",
            Self::MidiMapping => "midi_mapping",
            Self::RtpMidi => "rtp_midi",
        }
    }
}

/// Negotiated capability flags exposed by `hello`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Capabilities {
    /// Whether unsafe mode is currently armed.
    pub unsafe_mode_status: bool,
    /// Whether this actor may request local unsafe-mode arming.
    pub may_arm_unsafe_mode: bool,
    /// Whether this connection may receive state events.
    pub may_subscribe: bool,
}

/// `PiPedal` session phases exposed to local status consumers.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiPedalPhase {
    /// No transport connection exists.
    Disconnected,
    /// Transport is open and the hello exchange is pending.
    Connected,
    /// The server accepted the client identity.
    Identified,
    /// Startup catalog/state requests are in progress.
    LoadingCatalog,
    /// Required startup state is available.
    Ready,
}

/// Read-only health projection for one daemon-owned `PiPedal` session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PiPedalStatus {
    /// Current qualified protocol phase.
    pub phase: PiPedalPhase,
    /// Session generation used to reject stale work.
    pub generation: u64,
    /// Number of requests waiting for transport service.
    pub pending_requests: u16,
    /// Number of transport timeouts in this session.
    pub timeouts: u64,
    /// Number of non-timeout transport/protocol failures in this session.
    pub transport_failures: u64,
    /// Number of complete protocol reads accepted in this session.
    pub successful_reads: u64,
}

/// A bounded, newline-delimited IPC envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Envelope {
    /// Protocol version.
    pub version: ProtocolVersion,
    /// Correlation identifier.
    pub request_id: RequestId,
    /// Command tag.
    pub command: Command,
    /// UTF-8 JSON payload bytes, without a newline.
    pub payload: Vec<u8>,
}

/// Typed mapping operation carried by the Mappings command.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MappingOperation {
    /// Read active mappings and drafts.
    Snapshot,
    /// Start or update an inactive draft.
    Draft,
    /// Activate a complete mapping.
    Activate,
    /// Explicitly replace an existing mapping.
    Replace,
    /// Update behavior.
    Behavior,
    /// Enable or disable a mapping.
    Enabled,
    /// Delete a mapping.
    Delete,
    /// Undo the latest mutation.
    Undo,
}

/// Generation-checked mapping IPC request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingRequest {
    /// Requested operation.
    pub operation: MappingOperation,
    /// Authoritative generation expected by the client.
    pub generation: u64,
    /// Typed mapping operation payload.
    #[serde(default)]
    pub payload: Option<MappingPayload>,
}

/// Typed payloads for mapping mutations; no opaque JSON crosses the IPC boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum MappingPayload {
    /// Complete mapping for activation or replacement.
    Mapping {
        /// Complete mapping record.
        mapping: ControlMapping,
    },
    /// Inactive resumable wizard draft.
    Draft {
        /// Inactive draft record.
        draft: ControlMappingDraft,
    },
    /// Behavior update for an active mapping.
    Behavior {
        /// Active mapping identifier.
        mapping_id: String,
        /// New validated behavior.
        behavior: MappingBehavior,
    },
    /// Enable/disable update for an active mapping.
    Enabled {
        /// Active mapping identifier.
        mapping_id: String,
        /// Desired enabled state.
        enabled: bool,
    },
    /// Delete an active mapping.
    Delete {
        /// Active mapping identifier.
        mapping_id: String,
    },
}

/// Stable result for a mapping mutation or snapshot request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingResult {
    /// Generation after the operation, unchanged on failure.
    pub generation: u64,
    /// Whether an Undo operation is currently available.
    pub undo_available: bool,
    /// Active mapping projection, when returned by the operation.
    #[serde(default)]
    pub active: Option<Vec<ControlMapping>>,
    /// Inactive draft projection, when returned by the operation.
    #[serde(default)]
    pub draft: Option<Vec<ControlMappingDraft>>,
    /// Stable terminal outcome.
    pub outcome: MappingOutcome,
}

/// Stable mapping operation outcome vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MappingOutcome {
    /// Operation succeeded and state is authoritative.
    Applied,
    /// Request generation was stale; no mutation occurred.
    GenerationConflict,
    /// Mapping conflicts with an occupied source or destination.
    Conflict,
    /// Persistence failed before runtime commit.
    PersistenceFailed,
    /// Mapping or draft was incomplete/invalid.
    Invalid,
    /// Nothing was available to undo.
    NothingToUndo,
}

/// Authoritative controller-assignment workflow phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AssignmentPhase {
    /// No assignment is active.
    Idle,
    /// Waiting for one eligible physical control.
    AwaitControl,
    /// Choosing a connected compatible device.
    ChooseDevice,
    /// Choosing a profile-owned preset when the device provides one.
    ChoosePreset,
    /// Choosing an effect block.
    ChooseEffect,
    /// Choosing a parameter type within the effect.
    ChooseType,
    /// Choosing a destination parameter.
    ChooseParameter,
    /// Confirming an occupied destination replacement.
    ConfirmReplace,
    /// Persisting and applying the complete mapping.
    Committing,
    /// Mapping committed successfully.
    Succeeded,
    /// Mapping failed and may be retried.
    Failed,
    /// Connection interruption requires explicit resume/discard.
    Interrupted,
}

/// Typed assignment-session action shared by hardware and keyboard input.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AssignmentAction {
    /// Begin from the prior TUI location.
    Start,
    /// Accept a unique physical control.
    ControlCaptured,
    /// Move selection upward.
    Up,
    /// Move selection downward.
    Down,
    /// Enter the selected level.
    Enter,
    /// Return one level.
    Back,
    /// Confirm replacement.
    ConfirmReplace,
    /// Commit a complete destination payload atomically.
    Commit,
    /// Cancel the session.
    Cancel,
    /// Retry a failed commit.
    Retry,
    /// Resume an interrupted draft.
    Resume,
    /// Mark the active session interrupted by disconnect/reconnect.
    Interrupt,
    /// Mark a commit successful.
    Succeed,
    /// Mark a commit failed.
    Fail,
    /// Discard an interrupted draft.
    Discard,
}

/// Generation-checked command for the daemon-owned assignment session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssignmentRequest {
    /// Expected assignment-session generation.
    pub generation: u64,
    /// Typed session action.
    pub action: AssignmentAction,
    /// Optional physical-control identity captured by the controller.
    #[serde(default)]
    pub physical_control_id: Option<String>,
    /// Selected profile/device identity once the chooser reaches a destination.
    #[serde(default)]
    pub destination_profile: Option<String>,
    /// Selected effect-block identity, when applicable.
    #[serde(default)]
    pub destination_effect: Option<String>,
    /// Selected parameter identity for a commit request.
    #[serde(default)]
    pub destination_parameter: Option<String>,
}

/// Stable assignment-session mutation result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssignmentResult {
    /// Authoritative generation after processing.
    pub generation: u64,
    /// Current session projection.
    pub session: AssignmentSession,
    /// Whether the action was applied.
    pub applied: bool,
    /// Bounded reason for rejection or failure.
    #[serde(default)]
    pub reason: Option<String>,
}

/// Bounded authoritative assignment-session state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssignmentSession {
    /// Current workflow phase.
    pub phase: AssignmentPhase,
    /// Screen to return to after completion/cancel.
    pub prior_screen: String,
    /// Current candidate index.
    pub index: u16,
    /// Candidate count.
    pub total: u16,
    /// Whether the current session has a resumable draft.
    pub has_draft: bool,
    /// Phase to restore when an interrupted draft is explicitly resumed.
    #[serde(default)]
    pub interrupted_phase: Option<AssignmentPhase>,
    /// Per-level cursors, persisted in the daemon snapshot so views cannot leak selection.
    #[serde(default)]
    pub cursors: AssignmentCursors,
    /// Authoritative Learn catalog reconstructed by any renderer from this snapshot alone.
    #[serde(default)]
    pub catalog: AssignmentCatalog,
}

/// One bounded Learn catalog row.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssignmentCatalogEntry {
    /// Stable identity used by commits and filtering.
    pub id: String,
    /// Operator-visible label.
    pub label: String,
    /// Optional parent identity (effect id for parameters).
    #[serde(default)]
    pub group: Option<String>,
}

/// Daemon-owned Learn catalog and captured source/destination identities.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssignmentCatalog {
    /// Visible breadcrumb for the current level.
    #[serde(default)]
    pub breadcrumb: String,
    /// Connected/enabled device profiles.
    #[serde(default)]
    pub devices: Vec<AssignmentCatalogEntry>,
    /// Preset rows, including an explicit NONE choice when the profile has none.
    #[serde(default)]
    pub presets: Vec<AssignmentCatalogEntry>,
    /// Effect blocks for the selected device.
    #[serde(default)]
    pub effects: Vec<AssignmentCatalogEntry>,
    /// Parameter types/groups for the selected effect.
    #[serde(default)]
    pub types: Vec<AssignmentCatalogEntry>,
    /// Role-filtered parameters for the selected type.
    #[serde(default)]
    pub parameters: Vec<AssignmentCatalogEntry>,
    /// Selected device profile id.
    #[serde(default)]
    pub selected_device: Option<String>,
    /// Selected preset id, or `NONE`.
    #[serde(default)]
    pub selected_preset: Option<String>,
    /// Selected effect id.
    #[serde(default)]
    pub selected_effect: Option<String>,
    /// Selected type/group label.
    #[serde(default)]
    pub selected_type: Option<String>,
    /// Selected parameter id.
    #[serde(default)]
    pub selected_parameter: Option<String>,
    /// Captured assignable physical control.
    #[serde(default)]
    pub captured_control_id: Option<String>,
    /// Captured physical role (`Knob`, `Fader`, `ChannelButton`).
    #[serde(default)]
    pub captured_role: Option<String>,
    /// Stable source endpoint identity, never a wildcard.
    #[serde(default)]
    pub source_endpoint: Option<String>,
    /// Zero-based captured MIDI channel.
    #[serde(default)]
    pub source_channel: Option<u8>,
    /// Captured controller or note number.
    #[serde(default)]
    pub source_number: Option<u8>,
    /// Stable destination output endpoint identity.
    #[serde(default)]
    pub destination_endpoint: Option<String>,
    /// Last assignment action that was accepted or rejected.
    #[serde(default)]
    pub pending_action: Option<String>,
    /// Terminal result (`Succeeded`/`Failed`) when persistence completes.
    #[serde(default)]
    pub last_result: Option<String>,
    /// Persistent visible rejection or failure reason.
    #[serde(default)]
    pub last_error: Option<String>,
}

/// Bounded cursor positions for each assignment catalog level.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssignmentCursors {
    /// Device catalog cursor.
    pub device: u16,
    /// Preset catalog cursor.
    pub preset: u16,
    /// Effect catalog cursor.
    pub effect: u16,
    /// Type catalog cursor.
    pub kind: u16,
    /// Parameter catalog cursor.
    pub parameter: u16,
}

/// Device-button gesture classification used by the assignment session.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DeviceGesture {
    /// Short press commits or advances the current workflow.
    ShortPress,
    /// Hold cancels the active workflow.
    HoldCancel,
}

/// Classifies a measured Device-button duration without wall-clock behavior.
#[must_use]
pub const fn classify_device_gesture(duration_ms: u64) -> DeviceGesture {
    if duration_ms >= 750 {
        DeviceGesture::HoldCancel
    } else {
        DeviceGesture::ShortPress
    }
}

/// Result of collecting physical-control candidates during the uniqueness window.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CandidateCapture {
    /// No eligible control was observed.
    None,
    /// One unique control was observed.
    Unique,
    /// More than one distinct control was observed.
    Ambiguous,
}

/// Maximum elapsed capture window for physical-control disambiguation.
pub const ASSIGNMENT_CANDIDATE_WINDOW_MS: u64 = 250;

/// Classifies bounded candidate input after de-duplicating repeated controls.
#[must_use]
pub fn classify_candidates(control_ids: &[&str]) -> CandidateCapture {
    let mut unique: Vec<&str> = Vec::new();
    for id in control_ids.iter().copied().filter(|id| !id.is_empty()) {
        if !unique.contains(&id) {
            unique.push(id);
            if unique.len() > 1 {
                return CandidateCapture::Ambiguous;
            }
        }
    }
    match unique.len() {
        0 => CandidateCapture::None,
        _ => CandidateCapture::Unique,
    }
}

impl AssignmentSession {
    /// Creates an idle session with bounded empty context.
    #[must_use]
    pub fn new(prior_screen: impl Into<String>) -> Self {
        Self {
            phase: AssignmentPhase::Idle,
            prior_screen: prior_screen.into(),
            index: 0,
            total: 0,
            has_draft: false,
            interrupted_phase: None,
            cursors: AssignmentCursors::default(),
            catalog: AssignmentCatalog::default(),
        }
    }

    /// Sets the bounded candidate count and clamps the current position.
    pub fn set_total(&mut self, total: u16) {
        self.total = total;
        self.index = self.active_cursor().min(total.saturating_sub(1));
        self.set_active_cursor(self.index);
    }

    /// Returns the authoritative cursor for the current catalog level.
    #[must_use]
    pub const fn active_cursor(&self) -> u16 {
        match self.phase {
            AssignmentPhase::ChooseDevice => self.cursors.device,
            AssignmentPhase::ChoosePreset => self.cursors.preset,
            AssignmentPhase::ChooseEffect => self.cursors.effect,
            AssignmentPhase::ChooseType => self.cursors.kind,
            AssignmentPhase::ChooseParameter => self.cursors.parameter,
            _ => self.index,
        }
    }

    const fn set_active_cursor(&mut self, value: u16) {
        match self.phase {
            AssignmentPhase::ChooseDevice => self.cursors.device = value,
            AssignmentPhase::ChoosePreset => self.cursors.preset = value,
            AssignmentPhase::ChooseEffect => self.cursors.effect = value,
            AssignmentPhase::ChooseType => self.cursors.kind = value,
            AssignmentPhase::ChooseParameter => self.cursors.parameter = value,
            _ => {}
        }
        self.index = value;
    }

    /// Applies one typed action and returns whether the phase changed.
    pub fn apply(&mut self, action: AssignmentAction) -> bool {
        let before = (self.phase, self.index);
        self.phase = match (self.phase, action) {
            (AssignmentPhase::Idle, AssignmentAction::Start) => AssignmentPhase::AwaitControl,
            (AssignmentPhase::AwaitControl, AssignmentAction::ControlCaptured)
            | (AssignmentPhase::ChoosePreset, AssignmentAction::Back) => {
                AssignmentPhase::ChooseDevice
            }
            (AssignmentPhase::ChooseEffect, AssignmentAction::Back)
            | (AssignmentPhase::ChooseDevice, AssignmentAction::Enter) => {
                AssignmentPhase::ChoosePreset
            }
            (AssignmentPhase::ChoosePreset, AssignmentAction::Enter) => {
                AssignmentPhase::ChooseEffect
            }
            (AssignmentPhase::ChooseType, AssignmentAction::Back) => AssignmentPhase::ChooseEffect,
            (AssignmentPhase::ChooseParameter, AssignmentAction::Back) => {
                AssignmentPhase::ChooseType
            }
            (AssignmentPhase::ConfirmReplace, AssignmentAction::Back) => {
                AssignmentPhase::ChooseParameter
            }
            (AssignmentPhase::ChooseDevice, AssignmentAction::Back) => {
                AssignmentPhase::AwaitControl
            }
            (AssignmentPhase::ChooseEffect, AssignmentAction::Enter) => AssignmentPhase::ChooseType,
            (AssignmentPhase::ChooseType, AssignmentAction::Enter)
            | (AssignmentPhase::Interrupted, AssignmentAction::Resume) => {
                self.interrupted_phase.take().unwrap_or(AssignmentPhase::ChooseParameter)
            }
            (
                AssignmentPhase::ChooseParameter | AssignmentPhase::ChoosePreset,
                AssignmentAction::Commit,
            )
            | (AssignmentPhase::ConfirmReplace, AssignmentAction::ConfirmReplace) => {
                AssignmentPhase::Committing
            }
            (AssignmentPhase::Failed, AssignmentAction::Retry) => AssignmentPhase::Committing,
            (AssignmentPhase::Committing, AssignmentAction::Succeed) => AssignmentPhase::Succeeded,
            (AssignmentPhase::Committing, AssignmentAction::Fail) => AssignmentPhase::Failed,
            (AssignmentPhase::Interrupted, AssignmentAction::Discard) => {
                self.interrupted_phase = None;
                self.has_draft = false;
                AssignmentPhase::Idle
            }
            (phase, AssignmentAction::Up) if self.active_cursor() > 0 => {
                self.set_active_cursor(self.active_cursor() - 1);
                phase
            }
            (phase, AssignmentAction::Down)
                if self.total > 0 && self.active_cursor() + 1 < self.total =>
            {
                self.set_active_cursor(self.active_cursor() + 1);
                phase
            }
            (phase, AssignmentAction::Cancel) if phase != AssignmentPhase::Idle => {
                AssignmentPhase::Idle
            }
            (phase, AssignmentAction::Interrupt) if phase != AssignmentPhase::Idle => {
                self.interrupted_phase = Some(phase);
                self.has_draft = true;
                AssignmentPhase::Interrupted
            }
            (phase, _) => phase,
        };
        before != (self.phase, self.index)
    }
}

impl AssignmentRequest {
    /// Returns whether this request carries a complete destination commit payload.
    #[must_use]
    pub const fn has_complete_destination(&self) -> bool {
        self.physical_control_id.is_some()
            && self.destination_profile.is_some()
            && self.destination_effect.is_some()
            && self.destination_parameter.is_some()
    }

    /// Validates bounded assignment input before daemon dispatch.
    ///
    /// # Errors
    ///
    /// Returns an error for missing/oversized captured identities.
    pub fn validate(self) -> Result<Self, &'static str> {
        if self
            .physical_control_id
            .as_ref()
            .is_some_and(|id| id.is_empty() || id.len() > 64 || id != id.trim())
        {
            return Err("assignment physical-control identity is invalid");
        }
        for (value, label) in [
            (&self.destination_profile, "profile"),
            (&self.destination_effect, "effect"),
            (&self.destination_parameter, "parameter"),
        ] {
            if value.as_ref().is_some_and(|id| id.is_empty() || id.len() > 96 || id != id.trim()) {
                return Err(match label {
                    "profile" => "assignment destination profile is invalid",
                    "effect" => "assignment destination effect is invalid",
                    _ => "assignment destination parameter is invalid",
                });
            }
        }
        let destination_fields = [
            self.destination_profile.is_some(),
            self.destination_effect.is_some(),
            self.destination_parameter.is_some(),
        ];
        if destination_fields.iter().filter(|present| **present).count() != 0
            && destination_fields.iter().any(|present| !present)
        {
            return Err("assignment destination payload is incomplete");
        }
        if matches!(self.action, AssignmentAction::ControlCaptured)
            && self.physical_control_id.is_none()
        {
            return Err("control capture requires a physical-control identity");
        }
        Ok(self)
    }
}

impl MappingRequest {
    /// Validates bounded request payloads.
    ///
    /// # Errors
    ///
    /// Returns an error when a mutation lacks its payload or exceeds frame bounds.
    pub fn validate(self) -> Result<Self, &'static str> {
        let needs_payload =
            !matches!(self.operation, MappingOperation::Snapshot | MappingOperation::Undo);
        if needs_payload && self.payload.is_none() {
            return Err("mapping operation requires a payload");
        }
        if serde_json::to_vec(&self.payload).map_or(true, |payload| payload.len() > 16 * 1024) {
            return Err("mapping payload is oversized");
        }
        Ok(self)
    }
}

/// Sequenced daemon event retained after a snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateEvent {
    /// Strictly increasing daemon sequence.
    pub sequence: u64,
    /// Event payload bytes.
    pub payload: Vec<u8>,
}

/// Reconnect snapshot sufficient to rebuild client state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateSnapshot {
    /// Last event included in the snapshot.
    pub last_sequence: u64,
    /// Complete state payload.
    pub payload: Vec<u8>,
}

impl StateEvent {
    /// Encodes a sequenced event as one bounded JSON line.
    ///
    /// # Errors
    ///
    /// Returns an error for zero sequence numbers, invalid JSON payloads, or oversized output.
    pub fn encode_line(&self) -> Result<Vec<u8>, String> {
        if self.sequence == 0 || serde_json::from_slice::<serde_json::Value>(&self.payload).is_err()
        {
            return Err("event sequence or JSON payload is invalid".into());
        }
        let mut line = serde_json::to_vec(&serde_json::json!({
            "sequence": self.sequence,
            "payload": serde_json::from_slice::<serde_json::Value>(&self.payload)
                .map_err(|_| "event payload is invalid")?,
        }))
        .map_err(|error| error.to_string())?;
        line.push(b'\n');
        if line.len() > MAX_FRAME_BYTES {
            return Err("event exceeds maximum frame size".into());
        }
        Ok(line)
    }

    /// Decodes one complete event line and validates its bounded sequence/payload.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed JSON, missing fields, zero sequences, or invalid payloads.
    pub fn decode_line(line: &[u8]) -> Result<Self, String> {
        let value: serde_json::Value =
            serde_json::from_slice(line).map_err(|_| "event line is invalid")?;
        let sequence = value
            .get("sequence")
            .and_then(serde_json::Value::as_u64)
            .ok_or("event sequence is missing")?;
        if sequence == 0 {
            return Err("event sequence must be nonzero".into());
        }
        let payload = value.get("payload").ok_or("event payload is missing")?;
        let payload = serde_json::to_vec(payload).map_err(|error| error.to_string())?;
        Ok(Self { sequence, payload })
    }
}

/// Bounded subscriber queue; a slow client is evicted instead of consuming unbounded memory.
#[derive(Debug)]
pub struct SubscriberQueue {
    events: Vec<StateEvent>,
    capacity: usize,
}

impl SubscriberQueue {
    /// Creates a queue with a positive capacity.
    #[must_use]
    pub const fn new(capacity: usize) -> Option<Self> {
        if capacity == 0 {
            None
        } else {
            Some(Self { events: Vec::new(), capacity })
        }
    }

    /// Enqueues an event, returning `false` when the subscriber must be evicted.
    pub fn push(&mut self, event: StateEvent) -> bool {
        if self.events.len() >= self.capacity {
            return false;
        }
        self.events.push(event);
        true
    }

    /// Drains queued events in sequence order.
    pub fn drain(&mut self) -> Vec<StateEvent> {
        std::mem::take(&mut self.events)
    }
}

/// Verifies that a reconnect event stream can be applied after a snapshot.
///
/// # Errors
///
/// Returns an error for duplicate, skipped, or stale event sequences.
pub fn validate_reconnect(snapshot: &StateSnapshot, events: &[StateEvent]) -> Result<(), String> {
    let mut expected = snapshot.last_sequence.saturating_add(1);
    for event in events {
        if event.sequence != expected {
            return Err(format!("event sequence {}, expected {expected}", event.sequence));
        }
        expected = expected.saturating_add(1);
    }
    Ok(())
}

/// Bounded reconnect policy shared by interactive clients.
///
/// The policy is deliberately pure: callers perform the actual sleeping and
/// socket I/O, while this type guarantees a finite number of attempts and a
/// capped exponential delay. This keeps TUI event loops testable and prevents
/// an unavailable daemon from causing an unbounded busy loop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReconnectPolicy {
    attempts: u8,
    initial_delay: Duration,
    maximum_delay: Duration,
}

impl ReconnectPolicy {
    /// Creates a policy; zero attempts or delays are rejected.
    #[must_use]
    pub const fn new(
        attempts: u8,
        initial_delay: Duration,
        maximum_delay: Duration,
    ) -> Option<Self> {
        if attempts == 0 || initial_delay.is_zero() || maximum_delay.is_zero() {
            return None;
        }
        Some(Self { attempts, initial_delay, maximum_delay })
    }

    /// Number of connection attempts, including the first attempt.
    #[must_use]
    pub const fn attempts(self) -> u8 {
        self.attempts
    }

    /// Returns the delay before the one-based retry number.
    #[must_use]
    pub fn delay_before_retry(self, retry: u8) -> Duration {
        if retry == 0 {
            return Duration::ZERO;
        }
        let shift = u32::from(retry.saturating_sub(1)).min(31);
        let multiplier = 1_u32.checked_shl(shift).unwrap_or(u32::MAX);
        self.initial_delay.saturating_mul(multiplier).min(self.maximum_delay)
    }

    /// Returns whether another attempt may be made after a failed attempt.
    #[must_use]
    pub const fn permits_retry(self, failed_attempt: u8) -> bool {
        failed_attempt < self.attempts
    }
}

impl Envelope {
    /// Validates and encodes the envelope with its terminating newline.
    ///
    /// # Errors
    ///
    /// Returns an error for incompatible versions, invalid UTF-8/newlines, or oversized data.
    pub fn encode_line(&self) -> Result<Vec<u8>, String> {
        if !self.version.compatible() {
            return Err("incompatible IPC major version".to_owned());
        }
        if self.payload.contains(&b'\n') || std::str::from_utf8(&self.payload).is_err() {
            return Err("payload must be UTF-8 without newline".to_owned());
        }
        let mut encoded = format!(
            "{{\"protocol_major\":{},\"protocol_minor\":{},\"request_id\":{},\"command\":\"{}\",\"payload\":",
            self.version.major, self.version.minor, self.request_id.get(), self.command.tag()
        ).into_bytes();
        encoded.extend_from_slice(&self.payload);
        encoded.extend_from_slice(b"}\n");
        if encoded.len() > MAX_FRAME_BYTES {
            return Err("IPC envelope exceeds maximum".to_owned());
        }
        Ok(encoded)
    }
}

/// Linux peer credentials captured by the daemon acceptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PeerIdentity {
    /// Operating-system user ID.
    pub uid: u32,
    /// Operating-system group ID.
    pub gid: u32,
    /// Operating-system process ID.
    pub pid: u32,
}

/// Reads Linux `SO_PEERCRED` from an accepted Unix stream.
#[cfg(target_os = "linux")]
///
/// # Errors
///
/// Returns an operating-system error when peer credentials cannot be read.
pub fn peer_identity(stream: &std::os::unix::net::UnixStream) -> io::Result<PeerIdentity> {
    let credentials = getsockopt(stream, PeerCredentials).map_err(io::Error::other)?;
    let pid =
        u32::try_from(credentials.pid()).map_err(|_| io::Error::other("peer PID out of range"))?;
    Ok(PeerIdentity { uid: credentials.uid(), gid: credentials.gid(), pid })
}

/// Group-based local control policy. The daemon remains the only authority that can arm unsafe mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccessPolicy {
    /// Required supplemental/control group ID.
    pub control_gid: u32,
    /// Daemon service user ID, which is always accepted.
    pub daemon_uid: u32,
}

impl AccessPolicy {
    /// Returns whether the captured peer may access the control socket.
    #[must_use]
    pub const fn allows(self, identity: PeerIdentity) -> bool {
        identity.uid == 0 || identity.uid == self.daemon_uid || identity.gid == self.control_gid
    }
}

#[cfg(target_os = "linux")]
fn peer_has_supplementary_group(identity: PeerIdentity, group: u32) -> bool {
    let Ok(status) = std::fs::read_to_string(format!("/proc/{}/status", identity.pid)) else {
        return false;
    };
    let uid_matches = status
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .and_then(|values| values.split_whitespace().next())
        .and_then(|value| value.parse::<u32>().ok())
        == Some(identity.uid);
    uid_matches
        && status.lines().find_map(|line| line.strip_prefix("Groups:")).is_some_and(|values| {
            values
                .split_whitespace()
                .filter_map(|value| value.parse::<u32>().ok())
                .any(|candidate| candidate == group)
        })
}

/// Bound local Unix socket server. The daemon owns command dispatch after accepting a stream.
#[cfg(unix)]
#[derive(Debug)]
pub struct LocalServer {
    listener: UnixListener,
    path: PathBuf,
}

#[cfg(unix)]
impl LocalServer {
    /// Binds a control socket and applies the required `0660` filesystem mode.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the path cannot be bound or permissions cannot be applied.
    pub fn bind(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_owned();
        let listener = UnixListener::bind(&path)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o660))?;
        Ok(Self { listener, path })
    }

    /// Accepts one client stream and leaves command authorization to the daemon.
    ///
    /// # Errors
    ///
    /// Returns the listener's accept error.
    pub fn accept(&self) -> io::Result<UnixStream> {
        self.listener.accept().map(|(stream, _)| stream)
    }

    /// Configures whether accept should return immediately when no client is ready.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the listener cannot change its blocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.listener.set_nonblocking(nonblocking)
    }

    /// Accepts one client only when its kernel credentials satisfy `policy`.
    ///
    /// # Errors
    ///
    /// Returns an I/O or authorization error. Unauthorized streams are closed immediately.
    #[cfg(target_os = "linux")]
    pub fn accept_authorized(
        &self,
        policy: AccessPolicy,
    ) -> io::Result<(UnixStream, PeerIdentity)> {
        let stream = self.accept()?;
        let identity = peer_identity(&stream)?;
        if !policy.allows(identity) && !peer_has_supplementary_group(identity, policy.control_gid) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "IPC peer is not in mackes-control",
            ));
        }
        Ok((stream, identity))
    }

    /// Returns the socket path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(unix)]
impl Drop for LocalServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Blocking local client used by the TUI/CLI transport adapter.
#[cfg(unix)]
#[derive(Debug)]
pub struct LocalClient {
    stream: UnixStream,
}

#[cfg(unix)]
impl LocalClient {
    /// Connects to a daemon control socket.
    ///
    /// # Errors
    ///
    /// Returns the operating-system connection error.
    pub fn connect(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self { stream: UnixStream::connect(path)? })
    }

    /// Connects with the bounded retry policy, sleeping only between failed attempts.
    ///
    /// # Errors
    ///
    /// Returns the final operating-system connection error after the finite attempt budget.
    pub fn connect_with_policy(
        path: impl AsRef<Path>,
        policy: ReconnectPolicy,
    ) -> io::Result<(Self, u8)> {
        let path = path.as_ref();
        let mut last_error = None;
        for attempt in 1..=policy.attempts() {
            match Self::connect(path) {
                Ok(client) => return Ok((client, attempt)),
                Err(error) => {
                    last_error = Some(error);
                    if policy.permits_retry(attempt) {
                        std::thread::sleep(policy.delay_before_retry(attempt));
                    }
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "reconnect policy has no attempts")
        }))
    }

    /// Sends one already-validated newline-delimited envelope.
    ///
    /// # Errors
    ///
    /// Returns an error if the stream cannot write all bytes.
    pub fn send(&mut self, envelope: &Envelope) -> Result<(), String> {
        let bytes = envelope.encode_line()?;
        self.stream.write_all(&bytes).map_err(|error| error.to_string())
    }

    /// Sends one envelope and receives its complete response line.
    ///
    /// # Errors
    ///
    /// Returns an encoding, write, framing, or peer-closure error.
    pub fn request(&mut self, envelope: &Envelope) -> Result<Vec<u8>, String> {
        self.send(envelope)?;
        self.receive()
    }

    /// Connects with a bounded policy and performs one request/response exchange.
    ///
    /// # Errors
    ///
    /// Returns the final connection, encoding, write, framing, or peer-closure error.
    pub fn request_with_policy(
        path: impl AsRef<Path>,
        policy: ReconnectPolicy,
        envelope: &Envelope,
    ) -> Result<(Vec<u8>, u8), String> {
        let (mut client, attempts) =
            Self::connect_with_policy(path, policy).map_err(|error| error.to_string())?;
        client.request(envelope).map(|response| (response, attempts))
    }

    /// Reads one complete response line using the shared size bound.
    ///
    /// # Errors
    ///
    /// Returns framing or stream errors.
    pub fn receive(&mut self) -> Result<Vec<u8>, String> {
        let mut decoder = LineDecoder::default();
        let mut byte = [0_u8; 1];
        loop {
            let count = self.stream.read(&mut byte).map_err(|error| error.to_string())?;
            if count == 0 {
                return Err("IPC peer closed the stream".to_owned());
            }
            let mut lines = decoder.feed(&byte[..count])?;
            if let Some(line) = lines.pop() {
                return Ok(line);
            }
        }
    }
}

/// A validated protocol version.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolVersion {
    /// Major version.
    pub major: u16,
    /// Minor version.
    pub minor: u16,
}

impl ProtocolVersion {
    /// Returns the current version.
    #[must_use]
    pub const fn current() -> Self {
        Self { major: PROTOCOL_MAJOR, minor: PROTOCOL_MINOR }
    }

    /// Checks compatibility with the current major version.
    #[must_use]
    pub const fn compatible(self) -> bool {
        self.major == PROTOCOL_MAJOR
    }
}

/// Request identifier used to correlate responses and audit records.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestId(u64);

impl RequestId {
    /// Creates a nonzero request identifier.
    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        if value == 0 {
            None
        } else {
            Some(Self(value))
        }
    }

    /// Returns the numeric identifier.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Incremental newline-delimited envelope decoder.
#[derive(Debug)]
pub struct LineDecoder {
    buffer: Vec<u8>,
    maximum: usize,
}

impl Default for LineDecoder {
    fn default() -> Self {
        Self::new(MAX_FRAME_BYTES)
    }
}

impl LineDecoder {
    /// Creates a decoder with a bounded envelope size.
    #[must_use]
    pub const fn new(maximum: usize) -> Self {
        Self { buffer: Vec::new(), maximum }
    }

    /// Feeds bytes and returns complete, newline-stripped envelopes.
    ///
    /// # Errors
    ///
    /// Returns an error when a partial or complete envelope exceeds the configured bound.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, String> {
        let mut result = Vec::new();
        // Inspect only newly received bytes. Rescanning the accumulated frame
        // on every byte makes large activity snapshots quadratic to decode.
        for part in bytes.split_inclusive(|byte| *byte == b'\n') {
            let complete = part.last() == Some(&b'\n');
            let payload = if complete { &part[..part.len() - 1] } else { part };
            if payload.len() > self.maximum.saturating_sub(self.buffer.len()) {
                return Err(format!("IPC envelope exceeds {} bytes", self.maximum));
            }
            self.buffer.extend_from_slice(payload);
            if complete {
                result.push(std::mem::take(&mut self.buffer));
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipedal_status_round_trips_and_rejects_unknown_fields() {
        let status = PiPedalStatus {
            phase: PiPedalPhase::LoadingCatalog,
            generation: 4,
            pending_requests: 7,
            timeouts: 2,
            transport_failures: 1,
            successful_reads: 3,
        };
        let encoded = serde_json::to_vec(&status).expect("encode");
        assert_eq!(serde_json::from_slice::<PiPedalStatus>(&encoded).expect("decode"), status);
        assert!(serde_json::from_slice::<PiPedalStatus>(
            br#"{"phase":"ready","generation":1,"pending_requests":0,"timeouts":0,"transport_failures":0,"extra":true}"#
        )
        .is_err());
    }

    #[test]
    fn maximum_response_decodes_bytewise_and_retains_following_frames() {
        let mut decoder = LineDecoder::default();
        for _ in 0..MAX_FRAME_BYTES {
            assert!(decoder.feed(b"x").expect("partial frame").is_empty());
        }
        let lines = decoder.feed(b"\nnext\npartial").expect("complete frames");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], vec![b'x'; MAX_FRAME_BYTES]);
        assert_eq!(lines[1], b"next");
        assert_eq!(decoder.feed(b"\n").expect("retained tail"), vec![b"partial".to_vec()]);
        let mut oversized = LineDecoder::new(2);
        assert!(oversized.feed(b"abc\n").is_err());
    }

    #[test]
    fn live_test_status_tags_are_stable_and_terminal() {
        let statuses = [
            LiveTestStatus::Passed,
            LiveTestStatus::Failed,
            LiveTestStatus::TimedOut,
            LiveTestStatus::Denied,
            LiveTestStatus::Unavailable,
        ];
        assert_eq!(
            statuses.map(LiveTestStatus::tag),
            ["passed", "failed", "timed_out", "denied", "unavailable",]
        );
    }

    #[test]
    fn live_test_contract_rejects_unbounded_metadata() {
        let request = LiveTestRequest {
            request_id: "x".repeat(MAX_LIVE_TEST_REQUEST_ID_BYTES + 1),
            source_endpoint_id: "source".into(),
            destination_id: "destination".into(),
            candidate_kind: "control_change".into(),
            candidate_number: Some(7),
            candidate_channel: Some(1),
            generation: 7,
        };
        assert_eq!(request.validate(), Err("live-test request identifier is empty or oversized"));
        let result = LiveTestResult {
            request_id: "request-1".into(),
            status: LiveTestStatus::Failed,
            reason: "x".repeat(MAX_LIVE_TEST_REASON_BYTES + 1),
            audit_reference: None,
        };
        assert_eq!(result.validate(), Err("live-test result reason is oversized"));
        let invalid_channel = LiveTestRequest {
            request_id: "request-1".into(),
            source_endpoint_id: "source".into(),
            destination_id: "destination".into(),
            candidate_kind: "control_change".into(),
            candidate_number: Some(7),
            candidate_channel: Some(16),
            generation: 7,
        };
        assert_eq!(
            invalid_channel.clone().validate(),
            Err("live-test candidate channel is out of range")
        );
        let invalid_number = LiveTestRequest {
            candidate_channel: Some(1),
            candidate_number: Some(128),
            ..invalid_channel
        };
        assert_eq!(invalid_number.validate(), Err("live-test candidate number is out of range"));
    }

    #[test]
    fn live_test_json_contract_round_trips_known_fields() {
        let request = LiveTestRequest::from_json(
            br#"{"request_id":"learn-7","source_endpoint_id":"input:1","destination_id":"pedal.mix","candidate_kind":"control_change","candidate_number":7,"candidate_channel":1,"generation":9}"#,
        )
        .expect("request");
        assert_eq!(request.generation, 9);
        let result = LiveTestResult::from_json(
            br#"{"request_id":"learn-7","status":"passed","reason":"observed","audit_reference":"audit-7"}"#,
        )
        .expect("result");
        assert_eq!(result.status, LiveTestStatus::Passed);
        assert_eq!(result.audit_reference.as_deref(), Some("audit-7"));
        assert_eq!(
            LiveTestRequest::from_json(br#"{"request_id":"x","unknown":true}"#),
            Err("unknown live-test request field")
        );
    }

    #[test]
    fn rate_limiter_rejects_saturation_and_reports_retry() {
        let mut limiter = RateLimiter::new(1, 1).expect("valid limiter");
        assert!(limiter.allow());
        assert!(!limiter.allow());
        assert!(limiter.retry_after() > Duration::ZERO);
        assert!(RateLimiter::new(0, 1).is_none());
    }

    #[test]
    fn decodes_fragmented_and_coalesced_lines() {
        let mut decoder = LineDecoder::new(32);
        assert!(decoder.feed(b"{\"a\"").expect("fragment").is_empty());
        assert_eq!(decoder.feed(b":1}\n{\"b\":2}\n").expect("lines").len(), 2);
    }

    #[test]
    fn rejects_oversized_partial_line() {
        let mut decoder = LineDecoder::new(3);
        assert!(decoder.feed(b"1234").is_err());
    }

    #[test]
    fn rejects_major_version_mismatch() {
        assert!(!ProtocolVersion { major: 2, minor: 0 }.compatible());
        assert!(ProtocolVersion::current().compatible());
    }

    #[test]
    fn encodes_golden_envelope_and_rejects_unsafe_payloads() {
        let envelope = Envelope {
            version: ProtocolVersion::current(),
            request_id: RequestId::new(7).expect("nonzero"),
            command: Command::Hello,
            payload: b"{}".to_vec(),
        };
        assert_eq!(String::from_utf8(envelope.encode_line().expect("valid")).expect("utf8"),
            "{\"protocol_major\":1,\"protocol_minor\":0,\"request_id\":7,\"command\":\"hello\",\"payload\":{}}\n");
        let bad = Envelope { payload: b"{\n}".to_vec(), ..envelope };
        assert!(bad.encode_line().is_err());
    }

    #[test]
    fn reconnect_requires_contiguous_post_snapshot_events() {
        let snapshot = StateSnapshot { last_sequence: 10, payload: b"state".to_vec() };
        let events = vec![
            StateEvent { sequence: 11, payload: b"a".to_vec() },
            StateEvent { sequence: 12, payload: b"b".to_vec() },
        ];
        assert!(validate_reconnect(&snapshot, &events).is_ok());
        let gap = vec![StateEvent { sequence: 13, payload: b"gap".to_vec() }];
        assert!(validate_reconnect(&snapshot, &gap).is_err());
    }

    #[test]
    fn state_event_line_round_trip_is_bounded_and_strict() {
        let event = StateEvent { sequence: 4, payload: br#"{"health":"ready"}"#.to_vec() };
        let line = event.encode_line().expect("encode");
        assert_eq!(StateEvent::decode_line(&line).expect("decode"), event);
        assert!(StateEvent { sequence: 0, payload: b"{}".to_vec() }.encode_line().is_err());
        assert!(StateEvent::decode_line(br#"{"sequence":0,"payload":{}}"#).is_err());
        assert!(StateEvent::decode_line(br#"{"sequence":1,"payload":x}"#).is_err());
    }

    #[test]
    fn reconnect_policy_is_bounded_and_exponential() {
        let policy = ReconnectPolicy::new(4, Duration::from_millis(10), Duration::from_millis(25))
            .expect("valid policy");
        assert_eq!(policy.attempts(), 4);
        assert_eq!(policy.delay_before_retry(0), Duration::ZERO);
        assert_eq!(policy.delay_before_retry(1), Duration::from_millis(10));
        assert_eq!(policy.delay_before_retry(2), Duration::from_millis(20));
        assert_eq!(policy.delay_before_retry(3), Duration::from_millis(25));
        assert!(policy.permits_retry(1));
        assert!(!policy.permits_retry(4));
    }

    #[cfg(unix)]
    #[test]
    fn local_client_retries_only_within_policy_budget() {
        let path = std::env::temp_dir().join(format!("mackes-no-socket-{}", std::process::id()));
        let policy = ReconnectPolicy::new(2, Duration::from_millis(1), Duration::from_millis(1))
            .expect("policy");
        let error = LocalClient::connect_with_policy(&path, policy).expect_err("missing socket");
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn reconnect_policy_rejects_unusable_values() {
        assert!(
            ReconnectPolicy::new(0, Duration::from_millis(1), Duration::from_millis(1)).is_none()
        );
        assert!(ReconnectPolicy::new(1, Duration::ZERO, Duration::from_millis(1)).is_none());
        assert!(ReconnectPolicy::new(1, Duration::from_millis(1), Duration::ZERO).is_none());
    }

    #[test]
    fn slow_subscriber_is_bounded_and_evicted() {
        let mut queue = SubscriberQueue::new(1).expect("positive capacity");
        assert!(queue.push(StateEvent { sequence: 1, payload: vec![] }));
        assert!(!queue.push(StateEvent { sequence: 2, payload: vec![] }));
        assert_eq!(queue.drain().len(), 1);
    }

    #[test]
    fn network_and_mapping_actors_cannot_dispatch_administrative_commands() {
        assert_eq!(authorize(Command::UnsafeMode, ActorClass::RtpMidi), Authorization::Denied);
        assert_eq!(
            authorize(Command::Configuration, ActorClass::MidiMapping),
            Authorization::Denied
        );
        assert_eq!(authorize(Command::Health, ActorClass::RtpMidi), Authorization::Denied);
        assert_eq!(authorize(Command::UnsafeMode, ActorClass::LocalTui), Authorization::Allowed);
    }

    #[test]
    fn migration_command_is_local_only_and_wire_stable() {
        assert_eq!(Command::Migrate.tag(), "migrate");
        assert_eq!(authorize(Command::Migrate, ActorClass::LocalCli), Authorization::Allowed);
        assert_eq!(authorize(Command::Migrate, ActorClass::MidiMapping), Authorization::Denied);
        assert_eq!(authorize(Command::Migrate, ActorClass::RtpMidi), Authorization::Denied);
    }

    #[test]
    fn rescan_command_is_local_only_and_wire_stable() {
        assert_eq!(Command::Rescan.tag(), "rescan");
        assert_eq!(authorize(Command::Rescan, ActorClass::LocalCli), Authorization::Allowed);
        assert_eq!(authorize(Command::Rescan, ActorClass::MidiMapping), Authorization::Denied);
        assert_eq!(authorize(Command::Rescan, ActorClass::RtpMidi), Authorization::Denied);
    }

    #[test]
    fn local_access_policy_accepts_root_daemon_and_primary_control_group() {
        let policy = AccessPolicy { control_gid: 987, daemon_uid: 991 };
        assert!(policy.allows(PeerIdentity { uid: 0, gid: 0, pid: 1 }));
        assert!(policy.allows(PeerIdentity { uid: 991, gid: 991, pid: 1 }));
        assert!(policy.allows(PeerIdentity { uid: 1000, gid: 987, pid: 1 }));
        assert!(!policy.allows(PeerIdentity { uid: 1000, gid: 1000, pid: 1 }));
    }

    #[cfg(unix)]
    #[test]
    fn unix_loopback_uses_shared_envelope_and_socket_mode() {
        use std::{fs, os::unix::fs::MetadataExt, thread};

        let path = std::env::temp_dir().join(format!("mackes-ipc-{}.sock", std::process::id()));
        let server = LocalServer::bind(&path).expect("bind socket");
        assert_eq!(fs::metadata(&path).expect("metadata").mode() & 0o777, 0o660);
        let worker = thread::spawn(move || {
            let policy = AccessPolicy {
                control_gid: nix::unistd::getgid().as_raw(),
                daemon_uid: nix::unistd::getuid().as_raw(),
            };
            let (mut stream, identity) =
                server.accept_authorized(policy).expect("authorized accept");
            assert_eq!(identity.uid, nix::unistd::getuid().as_raw());
            let mut bytes = Vec::new();
            let mut byte = [0_u8; 1];
            loop {
                stream.read_exact(&mut byte).expect("read");
                bytes.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            assert!(bytes.ends_with(b"\n"));
            stream.write_all(b"{\"ok\":true}\n").expect("reply");
        });
        let envelope = Envelope {
            version: ProtocolVersion::current(),
            request_id: RequestId::new(1).expect("id"),
            command: Command::Hello,
            payload: b"{}".to_vec(),
        };
        let policy = ReconnectPolicy::new(1, Duration::from_millis(1), Duration::from_millis(1))
            .expect("policy");
        let (response, attempts) =
            LocalClient::request_with_policy(&path, policy, &envelope).expect("request");
        assert_eq!(response, b"{\"ok\":true}");
        assert_eq!(attempts, 1);
        worker.join().expect("worker");
    }

    #[test]
    fn mapping_contract_round_trips_and_rejects_missing_mutation_payload() {
        let request = MappingRequest {
            operation: MappingOperation::Activate,
            generation: 7,
            payload: Some(MappingPayload::Delete { mapping_id: "map-1".into() }),
        };
        let encoded = serde_json::to_vec(&request).expect("encode");
        let decoded: MappingRequest = serde_json::from_slice(&encoded).expect("decode");
        assert_eq!(decoded.validate().expect("valid"), request);
        assert!(MappingRequest {
            operation: MappingOperation::Activate,
            generation: 7,
            payload: None
        }
        .validate()
        .is_err());
        let result = MappingResult {
            generation: 8,
            undo_available: true,
            active: None,
            draft: None,
            outcome: MappingOutcome::Applied,
        };
        assert_eq!(
            serde_json::from_slice::<MappingResult>(
                &serde_json::to_vec(&result).expect("result encode")
            )
            .expect("result decode"),
            result
        );
    }

    #[test]
    fn assignment_session_has_one_bounded_hardware_keyboard_path() {
        let mut session = AssignmentSession::new("live");
        assert_eq!(session.phase, AssignmentPhase::Idle);
        session.set_total(3);
        assert!(session.apply(AssignmentAction::Down));
        assert!(session.apply(AssignmentAction::Down));
        assert_eq!(session.index, 2);
        assert!(!session.apply(AssignmentAction::Down));
        assert!(session.apply(AssignmentAction::Up));
        assert_eq!(session.index, 1);
        assert!(session.apply(AssignmentAction::Start));
        assert!(session.apply(AssignmentAction::ControlCaptured));
        assert!(session.apply(AssignmentAction::Enter));
        assert_eq!(session.phase, AssignmentPhase::ChoosePreset);
        assert!(session.apply(AssignmentAction::Enter));
        assert_eq!(session.phase, AssignmentPhase::ChooseEffect);
        assert!(session.apply(AssignmentAction::Back));
        assert_eq!(session.phase, AssignmentPhase::ChoosePreset);
        assert!(session.apply(AssignmentAction::Back));
        assert_eq!(session.phase, AssignmentPhase::ChooseDevice);
        assert!(session.apply(AssignmentAction::Enter));
        assert!(session.apply(AssignmentAction::Enter));
        assert_eq!(session.phase, AssignmentPhase::ChooseEffect);
        assert!(session.apply(AssignmentAction::Enter));
        assert_eq!(session.phase, AssignmentPhase::ChooseType);
        assert!(session.apply(AssignmentAction::Enter));
        assert_eq!(session.phase, AssignmentPhase::ChooseParameter);
        assert!(session.apply(AssignmentAction::Back));
        assert_eq!(session.phase, AssignmentPhase::ChooseType);
        assert!(session.apply(AssignmentAction::Enter));
        assert_eq!(session.phase, AssignmentPhase::ChooseParameter);
        assert!(session.apply(AssignmentAction::Commit));
        assert_eq!(session.phase, AssignmentPhase::Committing);
        assert!(session.apply(AssignmentAction::Cancel));
        assert_eq!(session.phase, AssignmentPhase::Idle);
        assert_eq!(session.phase, AssignmentPhase::Idle);
    }

    #[test]
    fn replacement_back_returns_to_parameter_selection() {
        let mut session = AssignmentSession::new("live");
        session.phase = AssignmentPhase::ConfirmReplace;
        session.index = 3;
        assert!(session.apply(AssignmentAction::Back));
        assert_eq!(session.phase, AssignmentPhase::ChooseParameter);
        assert_eq!(session.index, 3);
    }

    #[test]
    fn device_gesture_uses_exact_750_millisecond_hold_boundary() {
        assert_eq!(classify_device_gesture(749), DeviceGesture::ShortPress);
        assert_eq!(classify_device_gesture(750), DeviceGesture::HoldCancel);
        assert_eq!(classify_device_gesture(2_000), DeviceGesture::HoldCancel);
    }

    #[test]
    fn candidate_capture_deduplicates_repeats_and_fails_closed_on_two_controls() {
        assert_eq!(ASSIGNMENT_CANDIDATE_WINDOW_MS, 250);
        assert_eq!(classify_candidates(&[]), CandidateCapture::None);
        assert_eq!(classify_candidates(&["knob-r1-c1", "knob-r1-c1"]), CandidateCapture::Unique);
        assert_eq!(
            classify_candidates(&["knob-r1-c1", "button-r1-c1"]),
            CandidateCapture::Ambiguous
        );
    }

    #[test]
    fn assignment_session_exposes_terminal_and_interruption_recovery() {
        let mut session = AssignmentSession::new("live");
        session.apply(AssignmentAction::Start);
        assert!(session.apply(AssignmentAction::Interrupt));
        assert_eq!(session.phase, AssignmentPhase::Interrupted);
        assert!(session.has_draft);
        assert!(session.apply(AssignmentAction::Resume));
        assert_eq!(session.phase, AssignmentPhase::AwaitControl);
        assert!(session.apply(AssignmentAction::Interrupt));
        assert!(session.apply(AssignmentAction::Discard));
        assert_eq!(session.phase, AssignmentPhase::Idle);
        assert!(!session.has_draft);
        session.apply(AssignmentAction::Start);
        session.apply(AssignmentAction::ControlCaptured);
        session.apply(AssignmentAction::Enter);
        session.apply(AssignmentAction::Enter);
        session.apply(AssignmentAction::Enter);
        session.apply(AssignmentAction::Enter);
        assert_eq!(session.phase, AssignmentPhase::ChooseParameter);
        assert!(session.apply(AssignmentAction::Interrupt));
        assert!(session.apply(AssignmentAction::Resume));
        assert_eq!(session.phase, AssignmentPhase::ChooseParameter);
        assert!(session.apply(AssignmentAction::Commit));
        assert!(session.apply(AssignmentAction::Succeed));
        assert_eq!(session.phase, AssignmentPhase::Succeeded);
        assert!(session.apply(AssignmentAction::Cancel));
        assert_eq!(session.phase, AssignmentPhase::Idle);
    }

    #[test]
    fn assignment_session_keeps_independent_catalog_cursors() {
        let mut session = AssignmentSession::new("map");
        session.phase = AssignmentPhase::ChooseDevice;
        session.set_total(4);
        session.apply(AssignmentAction::Down);
        session.apply(AssignmentAction::Down);
        assert_eq!(session.active_cursor(), 2);
        session.phase = AssignmentPhase::ChooseEffect;
        session.set_total(3);
        session.apply(AssignmentAction::Down);
        assert_eq!(session.active_cursor(), 1);
        session.phase = AssignmentPhase::ChooseDevice;
        assert_eq!(session.active_cursor(), 2);
        let encoded = serde_json::to_string(&session).expect("cursor state serializes");
        let restored: AssignmentSession =
            serde_json::from_str(&encoded).expect("cursor state restores");
        assert_eq!(restored.cursors.device, 2);
        assert_eq!(restored.cursors.effect, 1);
        assert!(restored.catalog.devices.is_empty());
    }

    #[test]
    fn assignment_request_is_typed_bounded_and_round_trips() {
        let request = AssignmentRequest {
            generation: 4,
            action: AssignmentAction::ControlCaptured,
            physical_control_id: Some("knob-r1-c1".into()),
            destination_profile: None,
            destination_effect: None,
            destination_parameter: None,
        };
        let decoded: AssignmentRequest =
            serde_json::from_slice(&serde_json::to_vec(&request).expect("encode")).expect("decode");
        assert_eq!(decoded.validate().expect("valid"), request);
        assert!(AssignmentRequest { physical_control_id: None, ..request.clone() }
            .validate()
            .is_err());
        assert!(AssignmentRequest {
            destination_parameter: Some(" bad ".into()),
            action: AssignmentAction::Enter,
            physical_control_id: None,
            ..request.clone()
        }
        .validate()
        .is_err());
        assert!(AssignmentRequest {
            destination_parameter: Some("reflex.mix".into()),
            action: AssignmentAction::Enter,
            physical_control_id: None,
            ..request
        }
        .validate()
        .is_err());
    }
}
