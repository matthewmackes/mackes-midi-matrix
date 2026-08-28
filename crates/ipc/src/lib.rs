//! Versioned local daemon/client framing boundary.

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
            Self::Endpoints => "endpoints",
            Self::Routes => "routes",
            Self::Learn => "learn",
            Self::Scenes => "scenes",
            Self::DeviceQuery => "device_query",
            Self::Sysex => "sysex",
            Self::Backups => "backups",
            Self::Monitor => "monitor",
            Self::Health => "health",
            Self::Panic => "panic",
            Self::UnsafeMode => "unsafe_mode",
            Self::Shutdown => "shutdown",
        }
    }
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
        identity.uid == self.daemon_uid || identity.gid == self.control_gid
    }
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
        if !policy.allows(identity) {
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
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() > self.maximum && !self.buffer.contains(&b'\n') {
            return Err(format!("IPC envelope exceeds {} bytes", self.maximum));
        }
        let mut result = Vec::new();
        while let Some(index) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.buffer.drain(..=index).collect();
            let line = &line[..line.len() - 1];
            if line.len() > self.maximum {
                return Err(format!("IPC envelope exceeds {} bytes", self.maximum));
            }
            result.push(line.to_vec());
        }
        if self.buffer.len() > self.maximum {
            return Err(format!("IPC envelope exceeds {} bytes", self.maximum));
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
