//! MIDI endpoint, routing, scheduling, and mapping boundary.

use mackes_domain::{
    EndpointId, FourteenBit, MidiChannel, MidiEvent, MidiMessage, RealtimeMessage, SevenBit,
    SystemCommonMessage, TimestampNanos,
};
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "alsa-seq-backend")]
use std::ffi::CString;
#[cfg(feature = "alsa-seq-backend")]
use std::sync::Mutex;
use std::sync::{Arc, RwLock};

/// Direction of a MIDI endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointDirection {
    /// Messages received from a device or client.
    Input,
    /// Messages sent to a device or client.
    Output,
}

/// Stable endpoint metadata exposed to the rest of the application.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointInfo {
    /// Stable application identifier.
    pub id: String,
    /// Human-readable backend name.
    pub name: String,
    /// Endpoint direction.
    pub direction: EndpointDirection,
}

/// Volatile ALSA Sequencer client/port address for a live subscription.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct AlsaSequencerAddress {
    /// Runtime ALSA client number.
    pub client: u8,
    /// Runtime ALSA port number.
    pub port: u8,
}

/// Descriptive native ALSA Sequencer port record used before subscription.
#[cfg(feature = "alsa-seq-backend")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlsaSequencerPort {
    /// Runtime client/port address.
    pub address: AlsaSequencerAddress,
    /// ALSA client display name.
    pub client_name: String,
    /// ALSA port display name.
    pub port_name: String,
    /// Whether the port can be read by an application.
    pub readable: bool,
    /// Whether the port can be written by an application.
    pub writable: bool,
}

impl AlsaSequencerAddress {
    /// Creates an address, rejecting values outside ALSA's MIDI address range.
    #[must_use]
    pub const fn new(client: u8, port: u8) -> Self {
        Self { client, port }
    }
}

/// Lifecycle notifications needed to reconcile native ALSA subscriptions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlsaSequencerLifecycle {
    /// A client or port became visible.
    Started,
    /// Metadata for a client or port changed.
    Changed,
    /// A client or port disappeared.
    Exited,
    /// A subscription was established.
    Subscribed,
    /// A subscription was removed.
    Unsubscribed,
}

/// Connection state of a physical MIDI device projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicalDeviceState {
    /// The device has at least one currently visible MIDI port.
    Connected,
    /// The device identity is present in persisted state but no port is visible.
    Offline,
    /// More than one visible device could satisfy the same persisted identity.
    Ambiguous,
    /// No profile identity could be established from the endpoint metadata.
    Unknown,
}

/// Deterministic physical-device projection grouped from MIDI endpoints.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalDevice {
    /// Stable grouping key derived from the endpoint name.
    pub id: String,
    /// Display name shared by the grouped ports.
    pub name: String,
    /// Input endpoint IDs belonging to this device.
    pub inputs: Vec<String>,
    /// Output endpoint IDs belonging to this device.
    pub outputs: Vec<String>,
    /// Current identity state.
    pub state: PhysicalDeviceState,
}

/// Groups endpoint metadata without guessing across distinct names.
#[must_use]
pub fn group_physical_devices(endpoints: &[EndpointInfo]) -> Vec<PhysicalDevice> {
    let mut devices: Vec<PhysicalDevice> = Vec::new();
    for endpoint in endpoints {
        let device_name = endpoint
            .name
            .split_once(':')
            .map_or(endpoint.name.as_str(), |(device, _)| device)
            .trim();
        let id = device_name.to_ascii_lowercase();
        if id.is_empty() {
            continue;
        }
        let index = devices.iter().position(|device| device.id == id).unwrap_or_else(|| {
            devices.push(PhysicalDevice {
                id: id.clone(),
                name: device_name.to_owned(),
                inputs: Vec::new(),
                outputs: Vec::new(),
                state: PhysicalDeviceState::Unknown,
            });
            devices.len() - 1
        });
        let device = &mut devices[index];
        match endpoint.direction {
            EndpointDirection::Input => device.inputs.push(endpoint.id.clone()),
            EndpointDirection::Output => device.outputs.push(endpoint.id.clone()),
        }
        device.state = PhysicalDeviceState::Connected;
    }
    devices.sort_by(|left, right| left.id.cmp(&right.id));
    devices
}

/// Canonical virtual MIDI input port name exposed by MACKES.
pub const VIRTUAL_INPUT_NAME: &str = "MACKES DAW In";
/// Canonical virtual MIDI output port name exposed by MACKES.
pub const VIRTUAL_OUTPUT_NAME: &str = "MACKES DAW Out";

/// `AppleMIDI` control command parsed before any session state mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppleMidiCommand {
    /// Invitation request.
    Invitation,
    /// Invitation acceptance.
    Acceptance,
    /// Invitation rejection.
    Rejection,
    /// Synchronization packet.
    Synchronization,
    /// Receiver feedback packet.
    ReceiverFeedback,
    /// End-session packet.
    EndSession,
}

/// Lifecycle of one configured `AppleMIDI` session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionState {
    /// No peer is currently associated.
    Disconnected,
    /// An invitation has been sent or received.
    Invited,
    /// Token and remote SSRC have been established.
    Established,
}

/// Auth-free `AppleMIDI` session identity; peer allowlisting remains external policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppleMidiSession {
    /// Current lifecycle state.
    pub state: SessionState,
    /// Negotiated invitation token.
    pub token: u32,
    /// Remote synchronization source.
    pub remote_ssrc: Option<u32>,
    /// Remote advertised name.
    pub remote_name: Option<String>,
}

/// Explicitly bound UDP socket for configured RTP-MIDI traffic.
#[derive(Debug)]
pub struct UdpMidiTransport {
    socket: std::net::UdpSocket,
    max_datagram: usize,
}

/// Observable counters for one MIDI transport instance.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TransportStats {
    /// Datagrams received successfully.
    pub received: u64,
    /// Datagrams sent successfully.
    pub sent: u64,
    /// Malformed datagrams rejected by parsing.
    pub malformed: u64,
    /// Packets dropped by policy or sequence handling.
    pub dropped: u64,
    /// Late packets observed inside the reorder window.
    pub late: u64,
    /// Queue or buffer overflow events.
    pub overflow: u64,
}

/// Bounded explicit RTP-MIDI peer allowlist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerAllowlist {
    peers: Vec<std::net::SocketAddr>,
}

impl PeerAllowlist {
    /// Maximum number of configured peers.
    pub const MAX_PEERS: usize = 64;

    /// Creates an allowlist, rejecting duplicates and oversized configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when the list is empty, duplicated, or exceeds the bound.
    pub fn new(peers: Vec<std::net::SocketAddr>) -> Result<Self, &'static str> {
        if peers.is_empty() || peers.len() > Self::MAX_PEERS {
            return Err("RTP peer allowlist size is out of bounds");
        }
        let mut unique = peers.clone();
        unique.sort_unstable();
        unique.dedup();
        if unique.len() != peers.len() {
            return Err("RTP peer allowlist contains duplicates");
        }
        Ok(Self { peers })
    }

    /// Tests whether a peer is explicitly configured.
    #[must_use]
    pub fn contains(&self, peer: &std::net::SocketAddr) -> bool {
        self.peers.contains(peer)
    }

    /// Returns the configured peers in declaration order.
    #[must_use]
    pub fn peers(&self) -> &[std::net::SocketAddr] {
        &self.peers
    }
}

impl TransportStats {
    /// Records one packet outcome without allowing counters to wrap unexpectedly.
    pub const fn record_received(&mut self) {
        self.received = self.received.saturating_add(1);
    }
    /// Records one successful send.
    pub const fn record_sent(&mut self) {
        self.sent = self.sent.saturating_add(1);
    }
    /// Records one malformed packet.
    pub const fn record_malformed(&mut self) {
        self.malformed = self.malformed.saturating_add(1);
    }
    /// Records one policy/sequence drop.
    pub const fn record_dropped(&mut self) {
        self.dropped = self.dropped.saturating_add(1);
    }
    /// Records one late packet.
    pub const fn record_late(&mut self) {
        self.late = self.late.saturating_add(1);
    }
    /// Records one bounded-buffer overflow.
    pub const fn record_overflow(&mut self) {
        self.overflow = self.overflow.saturating_add(1);
    }
}

impl UdpMidiTransport {
    /// Binds one configured address in nonblocking mode.
    ///
    /// # Errors
    ///
    /// Returns an error when the address cannot be bound or `max_datagram` is zero.
    pub fn bind(address: std::net::SocketAddr, max_datagram: usize) -> std::io::Result<Self> {
        if max_datagram == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "UDP datagram limit must be nonzero",
            ));
        }
        let socket = std::net::UdpSocket::bind(address)?;
        socket.set_nonblocking(true)?;
        Ok(Self { socket, max_datagram })
    }

    /// Returns the local address selected by the operating system.
    ///
    /// # Errors
    ///
    /// Returns the underlying socket address error.
    pub fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.socket.local_addr()
    }

    /// Receives one bounded datagram, returning `None` when no packet is ready.
    ///
    /// # Errors
    ///
    /// Returns the underlying UDP receive error.
    pub fn receive(&self) -> std::io::Result<Option<(Vec<u8>, std::net::SocketAddr)>> {
        let mut buffer = vec![0; self.max_datagram];
        match self.socket.recv_from(&mut buffer) {
            Ok((length, peer)) => {
                buffer.truncate(length);
                Ok(Some((buffer, peer)))
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Receives one datagram only from an explicit peer allowlist.
    ///
    /// # Errors
    ///
    /// Returns the underlying UDP receive error.
    pub fn receive_from_allowed(
        &self,
        allowed_peers: &[std::net::SocketAddr],
    ) -> std::io::Result<Option<(Vec<u8>, std::net::SocketAddr)>> {
        match self.receive()? {
            Some((payload, peer)) if allowed_peers.contains(&peer) => Ok(Some((payload, peer))),
            Some(_) | None => Ok(None),
        }
    }

    /// Receives from a peer validated by [`PeerAllowlist`].
    ///
    /// # Errors
    ///
    /// Returns the underlying UDP receive error.
    pub fn receive_from_peer(
        &self,
        allowlist: &PeerAllowlist,
    ) -> std::io::Result<Option<(Vec<u8>, std::net::SocketAddr)>> {
        self.receive_from_allowed(allowlist.peers())
    }

    /// Sends one bounded datagram to an explicitly selected peer.
    ///
    /// # Errors
    ///
    /// Returns an error when the payload exceeds the configured limit or the socket write fails.
    pub fn send_to(&self, payload: &[u8], peer: std::net::SocketAddr) -> std::io::Result<usize> {
        if payload.len() > self.max_datagram {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "UDP datagram exceeds configured limit",
            ));
        }
        self.socket.send_to(payload, peer)
    }

    /// Sends only when the destination appears in the explicit peer allowlist.
    ///
    /// # Errors
    ///
    /// Returns `PermissionDenied` for an unconfigured peer, or the underlying
    /// socket error for an allowed peer.
    pub fn send_to_allowed(
        &self,
        payload: &[u8],
        peer: std::net::SocketAddr,
        allowed_peers: &[std::net::SocketAddr],
    ) -> std::io::Result<usize> {
        if !allowed_peers.contains(&peer) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "RTP-MIDI peer is not allowlisted",
            ));
        }
        self.send_to(payload, peer)
    }

    /// Sends to a peer validated by [`PeerAllowlist`].
    ///
    /// # Errors
    ///
    /// Returns an allowlist or underlying socket error.
    pub fn send_to_peer(
        &self,
        payload: &[u8],
        peer: std::net::SocketAddr,
        allowlist: &PeerAllowlist,
    ) -> std::io::Result<usize> {
        self.send_to_allowed(payload, peer, allowlist.peers())
    }
}

impl AppleMidiSession {
    /// Creates a disconnected session with a caller-provided token.
    #[must_use]
    pub const fn new(token: u32) -> Self {
        Self { state: SessionState::Disconnected, token, remote_ssrc: None, remote_name: None }
    }

    /// Records an invitation and peer name.
    pub fn invite(&mut self, name: impl Into<String>) {
        let name = name.into();
        if name.is_empty() || self.state == SessionState::Established {
            return;
        }
        self.remote_name = Some(name);
        self.state = SessionState::Invited;
    }

    /// Accepts a peer only when token and SSRC are consistent.
    ///
    /// # Errors
    ///
    /// Returns an error when the token is wrong, SSRC is zero, or the session is not invited.
    pub fn establish(&mut self, token: u32, ssrc: u32) -> Result<(), &'static str> {
        if self.state != SessionState::Invited {
            return Err("AppleMIDI session is not invited");
        }
        if token != self.token {
            return Err("AppleMIDI session token mismatch");
        }
        if ssrc == 0 {
            return Err("AppleMIDI SSRC must be nonzero");
        }
        self.remote_ssrc = Some(ssrc);
        self.state = SessionState::Established;
        Ok(())
    }

    /// Verifies an established packet identity.
    #[must_use]
    pub fn accepts(&self, token: u32, ssrc: u32) -> bool {
        self.state == SessionState::Established
            && token == self.token
            && self.remote_ssrc == Some(ssrc)
    }

    /// Ends an established session after verifying its peer identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the identity does not match an established peer.
    pub fn end_session(&mut self, token: u32, ssrc: u32) -> Result<(), &'static str> {
        if !self.accepts(token, ssrc) {
            return Err("AppleMIDI end-session identity mismatch");
        }
        self.disconnect();
        Ok(())
    }

    /// Drops peer state so reconnect starts a fresh invitation.
    pub fn disconnect(&mut self) {
        self.state = SessionState::Disconnected;
        self.remote_ssrc = None;
        self.remote_name = None;
    }
}

/// Coordinates peer identity and RTP sequence state across reconnects.
#[derive(Debug)]
pub struct RtpMidiPeer {
    session: AppleMidiSession,
    sequence: SequenceTracker,
}

impl RtpMidiPeer {
    /// Creates a disconnected peer with a bounded reorder window.
    ///
    /// # Errors
    ///
    /// Returns an error when the reorder window is outside the supported bound.
    pub fn new(token: u32, reorder_window: u16) -> Result<Self, &'static str> {
        Ok(Self {
            session: AppleMidiSession::new(token),
            sequence: SequenceTracker::new(reorder_window)?,
        })
    }

    /// Records an invitation for a named peer.
    pub fn invite(&mut self, name: impl Into<String>) {
        self.session.invite(name);
    }

    /// Returns the current `AppleMIDI` lifecycle state.
    #[must_use]
    pub const fn state(&self) -> SessionState {
        self.session.state
    }

    /// Returns the negotiated remote SSRC, if established.
    #[must_use]
    pub const fn remote_ssrc(&self) -> Option<u32> {
        self.session.remote_ssrc
    }

    /// Returns the advertised remote name, if an invitation was observed.
    #[must_use]
    pub fn remote_name(&self) -> Option<&str> {
        self.session.remote_name.as_deref()
    }

    /// Establishes the peer and resets stale sequence history.
    ///
    /// # Errors
    ///
    /// Returns an error when the invitation identity is invalid.
    pub fn establish(&mut self, token: u32, ssrc: u32) -> Result<(), &'static str> {
        self.session.establish(token, ssrc)?;
        self.sequence.reset();
        Ok(())
    }

    /// Accepts a packet only when peer identity and sequence policy both allow it.
    pub fn observe(&mut self, token: u32, ssrc: u32, sequence: u16) -> Option<SequenceDisposition> {
        self.session.accepts(token, ssrc).then(|| self.sequence.observe(sequence))
    }

    /// Validates and observes one authorized RTP-MIDI datagram.
    ///
    /// # Errors
    ///
    /// Returns a framing or allowlist error when the datagram is not acceptable.
    pub fn receive_packet<'a>(
        &mut self,
        packet: &'a [u8],
        peer: std::net::SocketAddr,
        allowed_peers: &[std::net::SocketAddr],
        token: u32,
        ssrc: u32,
    ) -> Result<(ParsedRtpMidi<'a>, SequenceDisposition), &'static str> {
        if !self.session.accepts(token, ssrc) {
            return Err("RTP-MIDI session identity is not established");
        }
        let parsed = validate_inbound_rtp_midi(packet, peer, allowed_peers)?;
        let disposition = self.sequence.observe(parsed.rtp.sequence);
        Ok((parsed, disposition))
    }

    /// Receives one datagram from a configured UDP transport and validates it.
    ///
    /// # Errors
    ///
    /// Returns an I/O, allowlist, session, or framing error. An empty socket
    /// queue is reported as `WouldBlock` by the transport.
    pub fn receive_from_transport(
        &mut self,
        transport: &UdpMidiTransport,
        allowlist: &PeerAllowlist,
        token: u32,
        ssrc: u32,
    ) -> std::io::Result<Option<(Vec<u8>, SequenceDisposition)>> {
        let Some((packet, peer)) = transport.receive_from_peer(allowlist)? else {
            return Ok(None);
        };
        let disposition = self
            .receive_packet(&packet, peer, allowlist.peers(), token, ssrc)
            .map(|(_, disposition)| disposition)
            .map_err(std::io::Error::other)?;
        Ok(Some((packet, disposition)))
    }

    /// Ends the peer session after identity validation and clears sequence state.
    ///
    /// # Errors
    ///
    /// Returns an error when the supplied identity does not match the established peer.
    pub fn end_session(&mut self, token: u32, ssrc: u32) -> Result<(), &'static str> {
        self.session.end_session(token, ssrc)?;
        self.sequence.reset();
        Ok(())
    }

    /// Drops the current session and sequence history after transport loss.
    pub fn disconnect(&mut self) {
        self.session.disconnect();
        self.sequence.reset();
    }
}

/// Validated RTP header metadata for an RTP-MIDI data packet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RtpHeader<'a> {
    /// RTP sequence number.
    pub sequence: u16,
    /// RTP timestamp.
    pub timestamp: u32,
    /// Synchronization source identifier.
    pub ssrc: u32,
    /// Payload after the complete RTP header.
    pub payload: &'a [u8],
}

/// Validated RTP-MIDI command-section framing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RtpMidiPayload<'a> {
    /// Begin/continue flag from the RTP-MIDI header.
    pub begin: bool,
    /// Dropped/invalid flag from the RTP-MIDI header.
    pub dropped: bool,
    /// Command section bytes, excluding the two-byte RTP-MIDI header.
    pub commands: &'a [u8],
}

/// Parsed RTP and RTP-MIDI framing for one datagram.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParsedRtpMidi<'a> {
    /// Validated RTP header metadata.
    pub rtp: RtpHeader<'a>,
    /// Validated RTP-MIDI command framing.
    pub midi: RtpMidiPayload<'a>,
}

/// Validates RTP and RTP-MIDI framing as one operation.
///
/// # Errors
///
/// Returns the first RTP or RTP-MIDI framing error encountered.
pub fn parse_rtp_midi_packet(packet: &[u8]) -> Result<ParsedRtpMidi<'_>, &'static str> {
    let rtp = parse_rtp_header(packet)?;
    let midi = parse_rtp_midi_payload(rtp.payload)?;
    Ok(ParsedRtpMidi { rtp, midi })
}

/// Validates an inbound RTP-MIDI datagram against an explicit peer allowlist.
///
/// # Errors
///
/// Returns a permission error for an unconfigured peer, or the framing error
/// for an authorized but malformed datagram.
pub fn validate_inbound_rtp_midi<'a>(
    packet: &'a [u8],
    peer: std::net::SocketAddr,
    allowed_peers: &[std::net::SocketAddr],
) -> Result<ParsedRtpMidi<'a>, &'static str> {
    if !allowed_peers.contains(&peer) {
        return Err("RTP-MIDI peer is not allowlisted");
    }
    parse_rtp_midi_packet(packet)
}

/// One decoded RTP-MIDI channel-voice command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RtpMidiCommand {
    /// Full status byte, including channel.
    pub status: u8,
    /// Data bytes in wire order (one or two bytes).
    pub data: [u8; 2],
    /// Number of valid data bytes.
    pub data_len: u8,
}

/// Converts one validated RTP-MIDI channel-voice command to a domain message.
///
/// # Errors
///
/// Returns an error for malformed status/data lengths or an unsupported status.
pub fn rtp_command_to_message(command: RtpMidiCommand) -> Result<MidiMessage, &'static str> {
    let channel = MidiChannel::new((command.status & 0x0f) + 1).ok_or("invalid MIDI channel")?;
    let first = SevenBit::new(u16::from(command.data[0])).ok_or("invalid MIDI data")?;
    let second = SevenBit::new(u16::from(command.data[1])).ok_or("invalid MIDI data")?;
    match command.status & 0xf0 {
        0x80 if command.data_len == 2 => {
            Ok(MidiMessage::NoteOff { channel, note: first, velocity: second })
        }
        0x90 if command.data_len == 2 => {
            Ok(MidiMessage::NoteOn { channel, note: first, velocity: second })
        }
        0xa0 if command.data_len == 2 => {
            Ok(MidiMessage::PolyPressure { channel, note: first, pressure: second })
        }
        0xb0 if command.data_len == 2 => {
            Ok(MidiMessage::ControlChange { channel, controller: first, value: second })
        }
        0xc0 if command.data_len == 1 => Ok(MidiMessage::ProgramChange { channel, program: first }),
        0xd0 if command.data_len == 1 => {
            Ok(MidiMessage::ChannelPressure { channel, pressure: first })
        }
        0xe0 if command.data_len == 2 => Ok(MidiMessage::PitchBend {
            channel,
            value: FourteenBit::new(u16::from(command.data[0]) | (u16::from(command.data[1]) << 7))
                .ok_or("invalid pitch bend")?,
        }),
        _ => Err("unsupported or malformed RTP-MIDI command"),
    }
}

/// Decodes the channel-voice command section of one RTP-MIDI payload.
///
/// # Errors
///
/// Returns the first framing or command validation error.
pub fn decode_rtp_channel_messages(bytes: &[u8]) -> Result<Vec<MidiMessage>, &'static str> {
    decode_rtp_midi_channel_voice(bytes)?.into_iter().map(rtp_command_to_message).collect()
}

/// Builds ordered domain events from an RTP command section.
///
/// `sequence_start` is assigned to the first message and saturates between
/// messages. The caller supplies the endpoint identity explicitly.
///
/// # Errors
///
/// Returns the first RTP command decoding error or an invalid endpoint error.
pub fn rtp_channel_events(
    bytes: &[u8],
    endpoint: u64,
    timestamp: u64,
    sequence_start: u64,
) -> Result<Vec<MidiEvent>, &'static str> {
    let endpoint = EndpointId::new(endpoint).ok_or("RTP endpoint ID must be nonzero")?;
    Ok(decode_rtp_channel_messages(bytes)?
        .into_iter()
        .enumerate()
        .map(|(index, message)| MidiEvent {
            timestamp: TimestampNanos::new(timestamp),
            sequence: sequence_start.saturating_add(index as u64),
            endpoint,
            message,
        })
        .collect())
}

/// Converts one validated RTP-MIDI system message to a domain message.
///
/// # Errors
///
/// Returns an error for invalid data lengths, out-of-range values, or
/// unsupported system statuses.
pub fn rtp_system_to_message(message: RtpMidiSystemMessage) -> Result<MidiMessage, &'static str> {
    let first = SevenBit::new(u16::from(message.data[0])).ok_or("invalid system data")?;
    match (message.status, message.data_len) {
        (0xf1, 1) => {
            Ok(MidiMessage::SystemCommon(SystemCommonMessage::TimeCodeQuarterFrame(first)))
        }
        (0xf2, 2) => Ok(MidiMessage::SystemCommon(SystemCommonMessage::SongPosition(
            FourteenBit::new(u16::from(message.data[0]) | (u16::from(message.data[1]) << 7))
                .ok_or("invalid song position")?,
        ))),
        (0xf3, 1) => Ok(MidiMessage::SystemCommon(SystemCommonMessage::SongSelect(first))),
        (0xf6, 0) => Ok(MidiMessage::SystemCommon(SystemCommonMessage::TuneRequest)),
        (0xf8, 0) => Ok(MidiMessage::Realtime(RealtimeMessage::Clock)),
        (0xfa, 0) => Ok(MidiMessage::Realtime(RealtimeMessage::Start)),
        (0xfb, 0) => Ok(MidiMessage::Realtime(RealtimeMessage::Continue)),
        (0xfc, 0) => Ok(MidiMessage::Realtime(RealtimeMessage::Stop)),
        (0xfe, 0) => Ok(MidiMessage::Realtime(RealtimeMessage::ActiveSensing)),
        (0xff, 0) => Ok(MidiMessage::Realtime(RealtimeMessage::Reset)),
        _ => Err("unsupported or malformed RTP-MIDI system message"),
    }
}

/// Builds ordered domain events from decoded RTP system messages.
///
/// # Errors
///
/// Returns the first system-message or endpoint validation error.
pub fn rtp_system_events(
    messages: &[RtpMidiSystemMessage],
    endpoint: u64,
    timestamp: u64,
    sequence_start: u64,
) -> Result<Vec<MidiEvent>, &'static str> {
    let endpoint = EndpointId::new(endpoint).ok_or("RTP endpoint ID must be nonzero")?;
    messages
        .iter()
        .enumerate()
        .map(|(index, message)| {
            Ok(MidiEvent {
                timestamp: TimestampNanos::new(timestamp),
                sequence: sequence_start.saturating_add(index as u64),
                endpoint,
                message: rtp_system_to_message(*message)?,
            })
        })
        .collect()
}

/// Converts a complete RTP-MIDI `SysEx` command into a validated domain message.
///
/// Fragmented `SysEx` is intentionally rejected; callers must use the bounded
/// `SysexReassembler` before invoking this function.
///
/// # Errors
///
/// Returns an error for missing framing, empty payloads, or non-7-bit data.
pub fn rtp_sysex_to_message(bytes: &[u8]) -> Result<MidiMessage, &'static str> {
    const MAX_RTP_SYSEX_BYTES: usize = 4096;
    if bytes.len() > MAX_RTP_SYSEX_BYTES {
        return Err("RTP SysEx exceeds bounded frame size");
    }
    if bytes.len() < 2 || bytes.first() != Some(&0xf0) || bytes.last() != Some(&0xf7) {
        return Err("RTP SysEx requires complete F0/F7 framing");
    }
    MidiMessage::sysex(bytes[1..bytes.len() - 1].to_vec())
        .map_err(|_| "RTP SysEx contains non-MIDI data")
}

/// Builds one timestamped domain event from a complete RTP `SysEx` frame.
///
/// # Errors
///
/// Returns an error for invalid endpoint identity or malformed `SysEx` framing.
pub fn rtp_sysex_event(
    bytes: &[u8],
    endpoint: u64,
    timestamp: u64,
    sequence: u64,
) -> Result<MidiEvent, &'static str> {
    Ok(MidiEvent {
        timestamp: TimestampNanos::new(timestamp),
        sequence,
        endpoint: EndpointId::new(endpoint).ok_or("RTP endpoint ID must be nonzero")?,
        message: rtp_sysex_to_message(bytes)?,
    })
}

/// One validated MIDI system message found in an RTP-MIDI command section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RtpMidiSystemMessage {
    /// Status byte.
    pub status: u8,
    /// Number of following data bytes.
    pub data_len: u8,
    /// Data bytes in wire order.
    pub data: [u8; 2],
}

/// Bounded `SysEx` fragment reassembler for RTP-MIDI sessions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SysexReassembler {
    max_bytes: usize,
    buffer: Vec<u8>,
    active: bool,
}

/// Classification of an incoming RTP sequence number.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceDisposition {
    /// Exactly the expected next packet.
    InOrder,
    /// A forward jump; the field counts missing packets.
    ForwardGap {
        /// Number of packets absent between the expected and observed sequence.
        missing: u16,
    },
    /// A packet already observed.
    Duplicate,
    /// An older packet inside the reorder window.
    Late,
}

/// Bounded sequence tracker for one RTP-MIDI session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SequenceTracker {
    next: Option<u16>,
    window: u16,
}

/// One timestamped packet held by the bounded jitter buffer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JitterPacket<T> {
    /// RTP timestamp used for presentation ordering.
    pub timestamp: u32,
    /// RTP sequence used as deterministic tie-breaker.
    pub sequence: u16,
    /// Decoded packet payload.
    pub payload: T,
}

/// Bounded deterministic jitter buffer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JitterBuffer<T> {
    capacity: usize,
    packets: Vec<JitterPacket<T>>,
}

impl<T> JitterBuffer<T> {
    /// Creates a buffer with a nonzero capacity.
    ///
    /// # Errors
    ///
    /// Returns an error when capacity is zero.
    pub const fn new(capacity: usize) -> Result<Self, &'static str> {
        if capacity == 0 {
            return Err("jitter buffer capacity must be nonzero");
        }
        Ok(Self { capacity, packets: Vec::new() })
    }

    /// Inserts a packet in timestamp/sequence order; returns overflow when full.
    ///
    /// # Errors
    ///
    /// Returns the packet unchanged when the capacity is full.
    pub fn push(&mut self, packet: JitterPacket<T>) -> Result<(), JitterPacket<T>> {
        if self.packets.len() >= self.capacity {
            return Err(packet);
        }
        let position = self
            .packets
            .binary_search_by_key(&(packet.timestamp, packet.sequence), |item| {
                (item.timestamp, item.sequence)
            })
            .unwrap_or_else(|index| index);
        self.packets.insert(position, packet);
        Ok(())
    }

    /// Removes the earliest packet, if available.
    pub fn pop(&mut self) -> Option<JitterPacket<T>> {
        if self.packets.is_empty() {
            None
        } else {
            Some(self.packets.remove(0))
        }
    }

    /// Drains all packets whose timestamp is at or before `timestamp`.
    pub fn drain_until(&mut self, timestamp: u32) -> Vec<JitterPacket<T>> {
        let count = self.packets.partition_point(|packet| packet.timestamp <= timestamp);
        self.packets.drain(..count).collect()
    }
}

impl SequenceTracker {
    /// Creates a tracker with a reorder window from 1 through 1024 packets.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested window is zero or exceeds 1024.
    pub const fn new(window: u16) -> Result<Self, &'static str> {
        if window == 0 || window > 1024 {
            return Err("RTP reorder window is out of bounds");
        }
        Ok(Self { next: None, window })
    }

    /// Clears sequence history for a newly established session.
    pub const fn reset(&mut self) {
        self.next = None;
    }

    /// Classifies and advances the expected sequence number when appropriate.
    pub const fn observe(&mut self, sequence: u16) -> SequenceDisposition {
        let Some(next) = self.next else {
            self.next = Some(sequence.wrapping_add(1));
            return SequenceDisposition::InOrder;
        };
        let distance = sequence.wrapping_sub(next);
        if distance == 0 {
            self.next = Some(next.wrapping_add(1));
            SequenceDisposition::InOrder
        } else if distance < 0x8000 {
            self.next = Some(sequence.wrapping_add(1));
            SequenceDisposition::ForwardGap { missing: distance }
        } else if next.wrapping_sub(sequence) <= self.window {
            SequenceDisposition::Late
        } else {
            SequenceDisposition::Duplicate
        }
    }
}

impl SysexReassembler {
    /// Creates a reassembler with a nonzero maximum payload size.
    ///
    /// # Errors
    ///
    /// Returns an error when `max_bytes` is zero.
    pub const fn new(max_bytes: usize) -> Result<Self, &'static str> {
        if max_bytes == 0 {
            return Err("SysEx maximum must be nonzero");
        }
        Ok(Self { max_bytes, buffer: Vec::new(), active: false })
    }

    /// Adds one fragment, returning a message only when `end` completes it.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid framing, non-MIDI data, or overflow; state is cleared.
    pub fn push(
        &mut self,
        fragment: &[u8],
        start: bool,
        end: bool,
    ) -> Result<Option<Vec<u8>>, &'static str> {
        if fragment.iter().any(|byte| *byte & 0x80 != 0) {
            self.clear();
            return Err("SysEx fragment contains non-data byte");
        }
        if start {
            if self.active {
                self.clear();
                return Err("SysEx fragment started before prior message ended");
            }
            self.buffer.clear();
            self.active = true;
        } else if !self.active {
            return Err("SysEx continuation has no active message");
        }
        if self.buffer.len().saturating_add(fragment.len()) > self.max_bytes {
            self.clear();
            return Err("SysEx reassembly limit exceeded");
        }
        self.buffer.extend_from_slice(fragment);
        if end {
            self.active = false;
            return Ok(Some(std::mem::take(&mut self.buffer)));
        }
        Ok(None)
    }

    /// Discards an incomplete message.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.active = false;
    }
}

/// Decodes realtime and system-common MIDI messages (no `SysEx`) from a section.
///
/// # Errors
///
/// Returns an error for unsupported system status, non-data bytes, or truncation.
pub fn decode_rtp_midi_system(bytes: &[u8]) -> Result<Vec<RtpMidiSystemMessage>, &'static str> {
    let mut result = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let status = bytes[index];
        if status < 0xF0 {
            return Err("RTP-MIDI system decoder received channel status");
        }
        let length = match status {
            0xF1 | 0xF3 => 1,
            0xF2 => 2,
            0xF6 | 0xF8..=0xFF => 0,
            _ => return Err("unsupported RTP-MIDI system status"),
        };
        if bytes.len() - index - 1 < length {
            return Err("RTP-MIDI system message is truncated");
        }
        let first = if length > 0 { bytes[index + 1] } else { 0 };
        if first & 0x80 != 0 {
            return Err("RTP-MIDI system data is not MIDI data");
        }
        let second = if length > 1 {
            let value = bytes[index + 2];
            if value & 0x80 != 0 {
                return Err("RTP-MIDI system data is not MIDI data");
            }
            value
        } else {
            0
        };
        result.push(RtpMidiSystemMessage {
            status,
            data_len: if length == 0 {
                0
            } else if length == 1 {
                1
            } else {
                2
            },
            data: [first, second],
        });
        index += 1 + length;
    }
    Ok(result)
}

/// Decodes channel-voice commands with MIDI running status.
///
/// # Errors
///
/// Returns an error for system bytes, status changes in the middle of a command,
/// non-data bytes, or incomplete commands.
pub fn decode_rtp_midi_channel_voice(bytes: &[u8]) -> Result<Vec<RtpMidiCommand>, &'static str> {
    let mut result = Vec::new();
    let mut index = 0;
    let mut running = None;
    while index < bytes.len() {
        let status = if bytes[index] & 0x80 != 0 {
            let value = bytes[index];
            if value >= 0xF0 {
                return Err("RTP-MIDI system command is not channel voice");
            }
            running = Some(value);
            index += 1;
            value
        } else {
            running.ok_or("RTP-MIDI data has no running status")?
        };
        let length = if status & 0xE0 == 0xC0 { 1 } else { 2 };
        if bytes.len() - index < length {
            return Err("RTP-MIDI command is truncated");
        }
        let first = bytes[index];
        if first & 0x80 != 0 {
            return Err("RTP-MIDI command data is not MIDI data");
        }
        let second = if length == 2 {
            let value = bytes[index + 1];
            if value & 0x80 != 0 {
                return Err("RTP-MIDI command data is not MIDI data");
            }
            value
        } else {
            0
        };
        result.push(RtpMidiCommand {
            status,
            data: [first, second],
            data_len: if length == 1 { 1 } else { 2 },
        });
        index += length;
    }
    Ok(result)
}

/// Parses the RFC 6295 RTP-MIDI command-section header.
///
/// # Errors
///
/// Returns an error for a missing header or a command-section length mismatch.
pub fn parse_rtp_midi_payload(payload: &[u8]) -> Result<RtpMidiPayload<'_>, &'static str> {
    if payload.len() < 2 {
        return Err("RTP-MIDI payload is truncated");
    }
    let declared = usize::from(u16::from(payload[0] & 0x0f) << 8 | u16::from(payload[1]));
    if declared != payload.len() - 2 {
        return Err("RTP-MIDI command length mismatch");
    }
    Ok(RtpMidiPayload {
        begin: payload[0] & 0x80 != 0,
        dropped: payload[0] & 0x40 != 0,
        commands: &payload[2..],
    })
}

/// Parses an RTP v2 header and returns the unmodified payload.
///
/// # Errors
///
/// Returns an error for unsupported versions, truncated CSRC/extension fields,
/// invalid padding, or packets without a payload.
pub fn parse_rtp_header(packet: &[u8]) -> Result<RtpHeader<'_>, &'static str> {
    if packet.len() < 12 || packet[0] >> 6 != 2 {
        return Err("invalid RTP header");
    }
    let csrc_count = usize::from(packet[0] & 0x0f);
    let extension = packet[0] & 0x10 != 0;
    let padding = packet[0] & 0x20 != 0;
    let mut offset = 12usize
        .checked_add(csrc_count.checked_mul(4).ok_or("RTP CSRC overflow")?)
        .ok_or("RTP header overflow")?;
    if packet.len() < offset {
        return Err("truncated RTP CSRC list");
    }
    if extension {
        if packet.len() < offset + 4 {
            return Err("truncated RTP extension header");
        }
        let words = usize::from(u16::from_be_bytes([packet[offset + 2], packet[offset + 3]]));
        offset = offset
            .checked_add(4 + words.checked_mul(4).ok_or("RTP extension overflow")?)
            .ok_or("RTP header overflow")?;
        if packet.len() < offset {
            return Err("truncated RTP extension");
        }
    }
    let end = if padding {
        let count = usize::from(*packet.last().ok_or("missing RTP padding")?);
        if count == 0 || count > packet.len() - offset {
            return Err("invalid RTP padding");
        }
        packet.len() - count
    } else {
        packet.len()
    };
    if end <= offset {
        return Err("RTP packet has no payload");
    }
    Ok(RtpHeader {
        sequence: u16::from_be_bytes([packet[2], packet[3]]),
        timestamp: u32::from_be_bytes([packet[4], packet[5], packet[6], packet[7]]),
        ssrc: u32::from_be_bytes([packet[8], packet[9], packet[10], packet[11]]),
        payload: &packet[offset..end],
    })
}

/// Builds an RTP v2 packet carrying one already-framed RTP-MIDI payload.
///
/// # Errors
///
/// Returns an error when the payload is empty or exceeds the RTP length bound.
pub fn build_rtp_packet(
    sequence: u16,
    timestamp: u32,
    ssrc: u32,
    payload: &[u8],
) -> Result<Vec<u8>, &'static str> {
    if payload.is_empty() {
        return Err("RTP payload must not be empty");
    }
    if payload.len() > usize::from(u16::MAX) {
        return Err("RTP payload exceeds length bound");
    }
    let mut packet = Vec::with_capacity(12 + payload.len());
    packet.extend_from_slice(&[0x80, 0x61]);
    packet.extend_from_slice(&sequence.to_be_bytes());
    packet.extend_from_slice(&timestamp.to_be_bytes());
    packet.extend_from_slice(&ssrc.to_be_bytes());
    packet.extend_from_slice(payload);
    Ok(packet)
}

/// Parses an `AppleMIDI` control header and enforces command-specific minimum sizes.
///
/// # Errors
///
/// Returns an error for a non-control RTP packet, unknown command, or truncated packet.
pub fn parse_apple_midi_command(packet: &[u8]) -> Result<AppleMidiCommand, &'static str> {
    if packet.len() < 4 || packet[0..2] != [0xff, 0xff] {
        return Err("not an AppleMIDI control packet");
    }
    let command = &packet[2..4];
    let (parsed, minimum) = match command {
        b"IN" => (AppleMidiCommand::Invitation, 16),
        b"OK" => (AppleMidiCommand::Acceptance, 16),
        b"NO" => (AppleMidiCommand::Rejection, 16),
        b"CK" => (AppleMidiCommand::Synchronization, 36),
        b"RS" => (AppleMidiCommand::ReceiverFeedback, 20),
        b"BY" => (AppleMidiCommand::EndSession, 12),
        _ => return Err("unknown AppleMIDI command"),
    };
    if packet.len() < minimum {
        return Err("truncated AppleMIDI command");
    }
    Ok(parsed)
}

/// Creates a deterministic endpoint identifier from backend name and direction.
/// Port enumeration indexes are intentionally excluded because ALSA renumbers ports.
#[must_use]
pub fn stable_endpoint_id(name: &str, direction: EndpointDirection) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.bytes().chain([match direction {
        EndpointDirection::Input => 0,
        EndpointDirection::Output => 1,
    }]) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!(
        "midir-{}-{hash:016x}",
        match direction {
            EndpointDirection::Input => "in",
            EndpointDirection::Output => "out",
        }
    )
}

/// Converts a stable endpoint identifier into the numeric domain endpoint ID.
/// The conversion is deterministic and shared by adapters and IPC capture paths.
#[must_use]
pub fn numeric_endpoint_id(stable_id: &str) -> Option<mackes_domain::EndpointId> {
    let value = stable_id
        .bytes()
        .fold(0_u64, |hash, byte| hash.wrapping_mul(257).wrapping_add(u64::from(byte)))
        .max(1);
    mackes_domain::EndpointId::new(value)
}

/// Native ALSA Sequencer client with one owned application ingress and egress port.
#[cfg(feature = "alsa-seq-backend")]
pub struct AlsaSequencerClient {
    seq: alsa::seq::Seq,
    input_port: i32,
    output_port: i32,
    pending_wire: VecDeque<(AlsaSequencerAddress, Vec<u8>)>,
    announcements_subscribed: bool,
}

#[cfg(feature = "alsa-seq-backend")]
impl std::fmt::Debug for AlsaSequencerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AlsaSequencerClient")
            .field("input_port", &self.input_port)
            .field("output_port", &self.output_port)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "alsa-seq-backend")]
impl AlsaSequencerClient {
    /// Opens a nonblocking ALSA Sequencer client and creates owned application ports.
    ///
    /// # Errors
    ///
    /// Returns the ALSA error when the client or either port cannot be created.
    ///
    /// # Errors
    ///
    /// Returns an error if the name contains NUL or ALSA rejects client/port creation.
    pub fn open(client_name: &str) -> Result<Self, String> {
        let seq = alsa::seq::Seq::open(None, None, true).map_err(|error| error.to_string())?;
        let client_name = CString::new(client_name).map_err(|_| "client name contains NUL")?;
        seq.set_client_name(&client_name).map_err(|error| error.to_string())?;
        let input_name = CString::new("MACKES Input").map_err(|_| "input name contains NUL")?;
        let output_name = CString::new("MACKES Output").map_err(|_| "output name contains NUL")?;
        let midi = alsa::seq::PortType::MIDI_GENERIC | alsa::seq::PortType::APPLICATION;
        let input_port = seq
            .create_simple_port(
                &input_name,
                alsa::seq::PortCap::WRITE | alsa::seq::PortCap::SUBS_WRITE,
                midi,
            )
            .map_err(|error| error.to_string())?;
        let output_port = seq
            .create_simple_port(
                &output_name,
                alsa::seq::PortCap::READ | alsa::seq::PortCap::SUBS_READ,
                midi,
            )
            .map_err(|error| error.to_string())?;
        Ok(Self {
            seq,
            input_port,
            output_port,
            pending_wire: VecDeque::new(),
            announcements_subscribed: false,
        })
    }

    /// Returns the volatile ALSA client number allocated to this process.
    ///
    /// # Errors
    ///
    /// Returns an ALSA query or range-conversion error.
    pub fn client_id(&self) -> Result<u8, String> {
        self.seq.client_id().map_err(|error| error.to_string()).and_then(|id| {
            u8::try_from(id).map_err(|_| "ALSA client number outside MIDI range".to_owned())
        })
    }

    /// Returns the owned ingress and egress port addresses.
    ///
    /// # Errors
    ///
    /// Returns an error if ALSA reports an invalid client or port number.
    pub fn application_ports(
        &self,
    ) -> Result<(AlsaSequencerAddress, AlsaSequencerAddress), String> {
        let client = self.client_id()?;
        Ok((
            AlsaSequencerAddress::new(
                client,
                u8::try_from(self.input_port).map_err(|_| "invalid input port")?,
            ),
            AlsaSequencerAddress::new(
                client,
                u8::try_from(self.output_port).map_err(|_| "invalid output port")?,
            ),
        ))
    }

    /// Explicitly subscribes an external source to the owned ingress port.
    ///
    /// # Errors
    ///
    /// Returns the ALSA subscription error.
    pub fn subscribe_input(&self, source: AlsaSequencerAddress) -> Result<(), String> {
        let client = self.client_id()?;
        let destination = alsa::seq::Addr { client: client.into(), port: self.input_port };
        let sender =
            alsa::seq::Addr { client: i32::from(source.client), port: i32::from(source.port) };
        if alsa::seq::PortSubscribeIter::new(&self.seq, sender, alsa::seq::QuerySubsType::READ)
            .any(|subscription| subscription.get_dest() == destination)
        {
            return Ok(());
        }
        let subscription = alsa::seq::PortSubscribe::empty().map_err(|error| error.to_string())?;
        subscription.set_sender(sender);
        subscription.set_dest(destination);
        self.seq.subscribe_port(&subscription).map_err(|error| error.to_string())
    }

    /// Subscribes the owned input port to ALSA's system announcement port.
    ///
    /// # Errors
    ///
    /// Returns the ALSA subscription error.
    pub fn subscribe_announcements(&mut self) -> Result<(), String> {
        if self.announcements_subscribed {
            return Ok(());
        }
        let client = self.client_id()?;
        let subscription = alsa::seq::PortSubscribe::empty().map_err(|error| error.to_string())?;
        subscription.set_sender(alsa::seq::Addr::system_announce());
        subscription.set_dest(alsa::seq::Addr { client: client.into(), port: self.input_port });
        self.seq.subscribe_port(&subscription).map_err(|error| error.to_string())?;
        self.announcements_subscribed = true;
        Ok(())
    }

    /// Enumerates external ALSA MIDI ports with stable runtime descriptors.
    #[must_use]
    pub fn discover_ports(&self) -> Vec<AlsaSequencerPort> {
        let mut ports = Vec::new();
        for client in alsa::seq::ClientIter::new(&self.seq) {
            let Ok(client_name) = client.get_name() else { continue };
            for port in alsa::seq::PortIter::new(&self.seq, client.get_client()) {
                let Ok(port_name) = port.get_name() else { continue };
                let address = AlsaSequencerAddress {
                    client: u8::try_from(port.get_client()).unwrap_or_default(),
                    port: u8::try_from(port.get_port()).unwrap_or_default(),
                };
                let capabilities = port.get_capability();
                ports.push(AlsaSequencerPort {
                    address,
                    client_name: client_name.to_owned(),
                    port_name: port_name.to_owned(),
                    readable: capabilities.contains(alsa::seq::PortCap::READ),
                    writable: capabilities.contains(alsa::seq::PortCap::WRITE),
                });
            }
        }
        ports
    }

    /// Reads the number of currently pending native ALSA events without blocking.
    ///
    /// # Errors
    ///
    /// Returns the ALSA pending-event query error.
    pub fn pending_events(&self) -> Result<u32, String> {
        self.seq.input().event_input_pending(true).map_err(|error| error.to_string())
    }

    /// Reads at most `limit` pending events as validated MIDI wire messages.
    ///
    /// # Errors
    ///
    /// Returns an ALSA read error or a malformed MIDI conversion error.
    pub fn read_wire_events(
        &self,
        limit: usize,
    ) -> Result<Vec<(AlsaSequencerAddress, Vec<u8>)>, String> {
        let mut input = self.seq.input();
        let mut events = Vec::new();
        for _ in 0..limit {
            if input.event_input_pending(true).map_err(|error| error.to_string())? == 0 {
                break;
            }
            let event = input.event_input().map_err(|error| error.to_string())?;
            let Some(bytes) = alsa_event_to_wire(&event)? else { continue };
            let source = event.get_source();
            let client = u8::try_from(source.client).map_err(|_| "invalid ALSA source client")?;
            let port = u8::try_from(source.port).map_err(|_| "invalid ALSA source port")?;
            events.push((AlsaSequencerAddress::new(client, port), bytes));
        }
        Ok(events)
    }

    /// Reads the next event belonging to one explicitly subscribed source.
    ///
    /// # Errors
    ///
    /// Returns an ALSA read or source-address conversion error.
    pub fn read_wire_event_for(
        &mut self,
        source: AlsaSequencerAddress,
    ) -> Result<Option<Vec<u8>>, String> {
        if let Some(index) = self.pending_wire.iter().position(|(address, _)| *address == source) {
            return Ok(self.pending_wire.remove(index).map(|(_, bytes)| bytes));
        }
        let mut input = self.seq.input();
        for _ in 0..32 {
            if input.event_input_pending(true).map_err(|error| error.to_string())? == 0 {
                break;
            }
            let event = input.event_input().map_err(|error| error.to_string())?;
            eprintln!(
                "native filtered reader saw {:?} from {}:{}",
                event.get_type(),
                event.get_source().client,
                event.get_source().port
            );
            let Some(bytes) = alsa_event_to_wire(&event)? else { continue };
            let source_address = event.get_source();
            let address = AlsaSequencerAddress::new(
                u8::try_from(source_address.client).map_err(|_| "invalid ALSA source client")?,
                u8::try_from(source_address.port).map_err(|_| "invalid ALSA source port")?,
            );
            self.pending_wire.push_back((address, bytes));
            if address == source {
                return Ok(self.pending_wire.pop_back().map(|(_, bytes)| bytes));
            }
        }
        Ok(None)
    }

    /// Reads bounded ALSA client/port lifecycle notifications.
    #[must_use]
    pub fn read_lifecycle_events(
        &self,
        limit: usize,
    ) -> Vec<(AlsaSequencerLifecycle, AlsaSequencerAddress)> {
        let mut input = self.seq.input();
        let mut events = Vec::new();
        for _ in 0..limit {
            if input.event_input_pending(true).unwrap_or(0) == 0 {
                break;
            }
            let Ok(event) = input.event_input() else { break };
            let lifecycle = match event.get_type() {
                alsa::seq::EventType::ClientStart | alsa::seq::EventType::PortStart => {
                    AlsaSequencerLifecycle::Started
                }
                alsa::seq::EventType::ClientChange | alsa::seq::EventType::PortChange => {
                    AlsaSequencerLifecycle::Changed
                }
                alsa::seq::EventType::ClientExit | alsa::seq::EventType::PortExit => {
                    AlsaSequencerLifecycle::Exited
                }
                alsa::seq::EventType::PortSubscribed => AlsaSequencerLifecycle::Subscribed,
                alsa::seq::EventType::PortUnsubscribed => AlsaSequencerLifecycle::Unsubscribed,
                _ => continue,
            };
            let address = event.get_data::<alsa::seq::Addr>().unwrap_or_else(|| event.get_source());
            let (Ok(client), Ok(port)) = (u8::try_from(address.client), u8::try_from(address.port))
            else {
                continue;
            };
            events.push((lifecycle, AlsaSequencerAddress::new(client, port)));
        }
        events
    }
}

/// Native ALSA input adapter with one explicit source subscription.
#[cfg(feature = "alsa-seq-backend")]
pub struct AlsaInputCapture {
    info: EndpointInfo,
    client: Arc<Mutex<AlsaSequencerClient>>,
    source: AlsaSequencerAddress,
    next_sequence: u64,
}

#[cfg(feature = "alsa-seq-backend")]
impl std::fmt::Debug for AlsaInputCapture {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("AlsaInputCapture").field("info", &self.info).finish_non_exhaustive()
    }
}

#[cfg(feature = "alsa-seq-backend")]
impl AlsaInputCapture {
    /// Opens a named ALSA source, creates an application client, and subscribes it explicitly.
    ///
    /// # Errors
    ///
    /// Returns an error when ALSA cannot open, discover, or subscribe the requested source.
    pub fn open_named(name: &str) -> Result<Self, String> {
        let client = Arc::new(Mutex::new(AlsaSequencerClient::open("MACKES input")?));
        Self::open_named_with_client(name, &client)
    }

    /// Opens a named source on an already-owned daemon client.
    ///
    /// # Errors
    ///
    /// Returns an error when the client lock, discovery, or subscription fails.
    pub fn open_named_with_client(
        name: &str,
        client: &Arc<Mutex<AlsaSequencerClient>>,
    ) -> Result<Self, String> {
        let requested_address =
            name.split_whitespace().last().and_then(|address| address.split_once(':')).and_then(
                |(client, port)| {
                    Some(AlsaSequencerAddress::new(client.parse().ok()?, port.parse().ok()?))
                },
            );
        let mut client_guard = client.lock().map_err(|_| "ALSA client lock poisoned".to_owned())?;
        let source = client_guard
            .discover_ports()
            .into_iter()
            .find(|port| {
                port.readable && (port.port_name == name || requested_address == Some(port.address))
            })
            .ok_or_else(|| format!("ALSA MIDI input not found: {name}"))?;
        client_guard.subscribe_input(source.address)?;
        client_guard.subscribe_announcements()?;
        drop(client_guard);
        Ok(Self {
            info: EndpointInfo {
                id: stable_endpoint_id(name, EndpointDirection::Input),
                name: name.to_owned(),
                direction: EndpointDirection::Input,
            },
            client: Arc::clone(client),
            source: source.address,
            next_sequence: 0,
        })
    }
}

#[cfg(feature = "alsa-seq-backend")]
impl MidiInputAdapter for AlsaInputCapture {
    fn info(&self) -> &EndpointInfo {
        &self.info
    }

    fn receive(&mut self) -> Option<MidiEvent> {
        let mut client = self.client.lock().ok()?;
        let bytes = client.read_wire_event_for(self.source).ok()??;
        drop(client);
        let message = MidiMessage::from_wire(&bytes).ok()?;
        if bytes.as_slice() == [0x98, 105, 127] {
            eprintln!("native projection Device endpoint={}", self.info.id);
        }
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        Some(MidiEvent {
            timestamp: TimestampNanos::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .and_then(|duration| u64::try_from(duration.as_nanos()).ok())?,
            ),
            sequence,
            endpoint: numeric_endpoint_id(&self.info.id)?,
            message,
        })
    }
}

#[cfg(feature = "alsa-seq-backend")]
fn alsa_event_to_wire(event: &alsa::seq::Event<'_>) -> Result<Option<Vec<u8>>, String> {
    use alsa::seq::{EvCtrl, EvNote, EventType};
    let channel_status = |base: u8, channel: u8| {
        base.checked_add(channel).ok_or_else(|| "invalid MIDI channel".to_owned())
    };
    let seven = |value: i32| {
        u8::try_from(value)
            .ok()
            .filter(|value| *value < 128)
            .ok_or_else(|| "MIDI value out of range".to_owned())
    };
    let bytes = match event.get_type() {
        EventType::Noteon => {
            let data = event.get_data::<EvNote>().ok_or("missing note data")?;
            Some(vec![channel_status(0x90, data.channel)?, data.note, data.velocity])
        }
        EventType::Noteoff => {
            let data = event.get_data::<EvNote>().ok_or("missing note data")?;
            Some(vec![channel_status(0x80, data.channel)?, data.note, data.velocity])
        }
        EventType::Keypress => {
            let data = event.get_data::<EvNote>().ok_or("missing pressure data")?;
            Some(vec![channel_status(0xa0, data.channel)?, data.note, data.velocity])
        }
        EventType::Controller => {
            let data = event.get_data::<EvCtrl>().ok_or("missing controller data")?;
            Some(vec![
                channel_status(0xb0, data.channel)?,
                u8::try_from(data.param)
                    .ok()
                    .filter(|value| *value < 128)
                    .ok_or_else(|| "MIDI controller out of range".to_owned())?,
                seven(data.value)?,
            ])
        }
        EventType::Pgmchange => {
            let data = event.get_data::<EvCtrl>().ok_or("missing program data")?;
            Some(vec![channel_status(0xc0, data.channel)?, seven(data.value)?])
        }
        EventType::Chanpress => {
            let data = event.get_data::<EvCtrl>().ok_or("missing channel pressure data")?;
            Some(vec![channel_status(0xd0, data.channel)?, seven(data.value)?])
        }
        EventType::Pitchbend => {
            let data = event.get_data::<EvCtrl>().ok_or("missing pitch data")?;
            let value = u16::try_from(data.value.saturating_add(8192))
                .map_err(|_| "pitch value out of range")?;
            if value > 16_383 {
                return Err("pitch value out of range".into());
            }
            Some(vec![
                channel_status(0xe0, data.channel)?,
                (value & 0x7f) as u8,
                u8::try_from(value >> 7).map_err(|_| "pitch value out of range")?,
            ])
        }
        EventType::Sysex => event.get_ext().map(<[u8]>::to_vec),
        _ => None,
    };
    Ok(bytes)
}

/// Enumerates physical MIDI ports through the Fedora `midir` backend.
///
/// The returned data is descriptive only; opening a port is deliberately a
/// separate operation so discovery cannot trigger device I/O.
///
/// # Errors
///
/// Returns a backend or port-name error when ALSA cannot be initialized or a
/// port name cannot be read.
#[cfg(feature = "midir-backend")]
pub fn enumerate_midir_ports() -> Result<Vec<EndpointInfo>, String> {
    let input = midir::MidiInput::new("MACKES discovery").map_err(|error| error.to_string())?;
    let output = midir::MidiOutput::new("MACKES discovery").map_err(|error| error.to_string())?;
    let mut ports = Vec::new();
    for port in input.ports() {
        let name = input.port_name(&port).map_err(|error| error.to_string())?;
        ports.push(EndpointInfo {
            id: stable_endpoint_id(&name, EndpointDirection::Input),
            name,
            direction: EndpointDirection::Input,
        });
    }
    for port in output.ports() {
        let name = output.port_name(&port).map_err(|error| error.to_string())?;
        ports.push(EndpointInfo {
            id: stable_endpoint_id(&name, EndpointDirection::Output),
            name,
            direction: EndpointDirection::Output,
        });
    }
    Ok(ports)
}

/// Owned ALSA virtual-port connections created by an explicit runtime request.
#[cfg(feature = "midir-backend")]
pub struct VirtualMidiPorts {
    input: midir::MidiInputConnection<()>,
    output: midir::MidiOutputConnection,
}

#[cfg(feature = "midir-backend")]
impl std::fmt::Debug for VirtualMidiPorts {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("VirtualMidiPorts").finish_non_exhaustive()
    }
}

#[cfg(feature = "midir-backend")]
impl VirtualMidiPorts {
    /// Borrows the input connection for lifecycle/control operations.
    pub const fn input_connection(&mut self) -> &mut midir::MidiInputConnection<()> {
        &mut self.input
    }

    /// Sends raw MIDI bytes through the virtual output port.
    ///
    /// # Errors
    ///
    /// Returns the backend error when the virtual output is unavailable.
    pub fn send(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.output.send(bytes).map_err(|error| error.to_string())
    }
}

/// Creates the standard MACKES DAW virtual input/output pair.
///
/// The callback receives raw bytes from the virtual input and must be wired to
/// the daemon ingress by the caller. Dropping this value removes both ports.
///
/// # Errors
///
/// Returns an error when ALSA cannot create either virtual port.
#[cfg(feature = "midir-backend")]
pub fn create_virtual_ports<F>(callback: F) -> Result<VirtualMidiPorts, String>
where
    F: FnMut(u64, &[u8], &mut ()) + Send + 'static,
{
    use midir::os::unix::{VirtualInput, VirtualOutput};
    let input = midir::MidiInput::new("MACKES virtual input").map_err(|error| error.to_string())?;
    let output =
        midir::MidiOutput::new("MACKES virtual output").map_err(|error| error.to_string())?;
    let input = input
        .create_virtual(VIRTUAL_INPUT_NAME, callback, ())
        .map_err(|error| error.to_string())?;
    let output = output.create_virtual(VIRTUAL_OUTPUT_NAME).map_err(|error| error.to_string())?;
    Ok(VirtualMidiPorts { input, output })
}

/// Input adapter boundary; backend-specific types must not cross it.
pub trait MidiInputAdapter {
    /// Returns immutable endpoint metadata.
    fn info(&self) -> &EndpointInfo;
    /// Receives the next event, if one is available.
    fn receive(&mut self) -> Option<MidiEvent>;
}

/// Bounded owner for heterogeneous MIDI input adapters.
#[derive(Default)]
pub struct InputRegistry {
    inputs: Vec<Box<dyn MidiInputAdapter>>,
    capacity: usize,
}

impl std::fmt::Debug for InputRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InputRegistry")
            .field("count", &self.inputs.len())
            .field("capacity", &self.capacity)
            .finish()
    }
}

impl InputRegistry {
    /// Creates an empty registry with a bounded adapter count.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self { inputs: Vec::new(), capacity: capacity.max(1) }
    }

    /// Returns the number of registered input adapters.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inputs.len()
    }

    /// Returns whether no input adapters are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }

    /// Adds an input unless full or its stable ID is duplicated.
    ///
    /// # Errors
    ///
    /// Returns an error when capacity is reached or the endpoint ID is duplicated.
    pub fn insert(&mut self, input: Box<dyn MidiInputAdapter>) -> Result<(), &'static str> {
        if self.inputs.len() >= self.capacity {
            return Err("input registry capacity reached");
        }
        if self.inputs.iter().any(|item| item.info().id == input.info().id) {
            return Err("duplicate input endpoint");
        }
        self.inputs.push(input);
        Ok(())
    }

    /// Removes an input by stable endpoint ID.
    pub fn remove(&mut self, id: &str) -> bool {
        let Some(index) = self.inputs.iter().position(|input| input.info().id == id) else {
            return false;
        };
        self.inputs.remove(index);
        true
    }

    /// Returns the stable endpoint key for a numeric event endpoint.
    #[must_use]
    pub fn stable_id_for_endpoint(&self, endpoint: mackes_domain::EndpointId) -> Option<String> {
        self.inputs.iter().find_map(|input| {
            (numeric_endpoint_id(&input.info().id) == Some(endpoint))
                .then(|| input.info().id.clone())
        })
    }

    /// Polls each input once in stable registration order.
    #[must_use]
    pub fn poll_once(&mut self) -> Vec<MidiEvent> {
        self.inputs.iter_mut().filter_map(|input| input.receive()).collect()
    }
}

/// Output adapter boundary; backend-specific types must not cross it.
pub trait MidiOutputAdapter {
    /// Returns immutable endpoint metadata.
    fn info(&self) -> &EndpointInfo;
    /// Sends one event while preserving caller order.
    fn send(&mut self, event: MidiEvent);
}

/// Bounded owner for heterogeneous MIDI output adapters.
#[derive(Default)]
pub struct OutputRegistry {
    outputs: Vec<Box<dyn MidiOutputAdapter>>,
    capacity: usize,
}

impl std::fmt::Debug for OutputRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OutputRegistry")
            .field("count", &self.outputs.len())
            .field("capacity", &self.capacity)
            .finish()
    }
}

impl OutputRegistry {
    /// Creates an empty registry with a bounded adapter count.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self { outputs: Vec::new(), capacity: capacity.max(1) }
    }

    /// Adds an adapter unless the registry is full or its ID is duplicated.
    ///
    /// # Errors
    ///
    /// Returns an error when capacity is reached or the endpoint ID is duplicated.
    pub fn insert(&mut self, output: Box<dyn MidiOutputAdapter>) -> Result<(), &'static str> {
        if self.outputs.len() >= self.capacity {
            return Err("output registry capacity reached");
        }
        if self.outputs.iter().any(|item| item.info().id == output.info().id) {
            return Err("duplicate output endpoint");
        }
        self.outputs.push(output);
        Ok(())
    }

    /// Removes an adapter by stable endpoint ID and reports whether it existed.
    pub fn remove(&mut self, id: &str) -> bool {
        let Some(index) = self.outputs.iter().position(|output| output.info().id == id) else {
            return false;
        };
        self.outputs.remove(index);
        true
    }

    /// Sends one already-validated MIDI message directly to a named output.
    ///
    /// # Errors
    ///
    /// Returns an error when the destination is not registered or the message is malformed.
    pub fn send_direct(&mut self, destination: &str, payload: &[u8]) -> Result<(), String> {
        let message = mackes_domain::MidiMessage::from_wire(payload).map_err(str::to_owned)?;
        let endpoint = mackes_domain::EndpointId::new(1).ok_or("invalid direct endpoint")?;
        let event = mackes_domain::MidiEvent {
            timestamp: mackes_domain::TimestampNanos::new(0),
            sequence: 0,
            endpoint,
            message,
        };
        let output = self
            .outputs
            .iter_mut()
            .find(|output| output.info().id == destination)
            .ok_or("destination output is not registered")?;
        output.send(event);
        Ok(())
    }

    /// Sends an already-constructed event to exactly one numeric endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error when the endpoint is not registered.
    pub fn send_to_endpoint(
        &mut self,
        endpoint: EndpointId,
        event: MidiEvent,
    ) -> Result<(), String> {
        let output = self
            .outputs
            .iter_mut()
            .find(|output| numeric_endpoint_id(&output.info().id) == Some(endpoint))
            .ok_or_else(|| "destination output is not registered".to_owned())?;
        output.send(event);
        Ok(())
    }

    /// Returns registered output IDs in stable registration order.
    #[must_use]
    pub fn output_ids(&self) -> Vec<String> {
        self.outputs.iter().map(|output| output.info().id.clone()).collect()
    }

    /// Returns numeric IDs for outputs whose backend name contains `needle`.
    #[must_use]
    pub fn endpoint_ids_named(&self, needle: &str) -> Vec<EndpointId> {
        self.outputs
            .iter()
            .filter(|output| output.info().name.contains(needle))
            .filter_map(|output| numeric_endpoint_id(&output.info().id))
            .collect()
    }

    /// Routes and dispatches one event through all registered outputs.
    #[must_use]
    pub fn dispatch(&mut self, router: &RouterStore, event: &MidiEvent) -> (usize, usize) {
        let mut outputs = self
            .outputs
            .iter_mut()
            .map(|output| output.as_mut() as &mut dyn MidiOutputAdapter)
            .collect::<Vec<_>>();
        dispatch_routed_event(router, event, &mut outputs)
    }

    /// Returns the number of registered adapters.
    #[must_use]
    pub fn len(&self) -> usize {
        self.outputs.len()
    }

    /// Returns whether no adapters are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }
}

/// Routes one ingress event and sends each matching result to its named output.
///
/// Outputs are searched in declaration order; unmatched destinations are
/// counted as drops instead of being silently redirected.
#[must_use]
pub fn dispatch_routed_event(
    router: &RouterStore,
    event: &MidiEvent,
    outputs: &mut [&mut dyn MidiOutputAdapter],
) -> (usize, usize) {
    let mut sent = 0;
    let mut unmatched = 0;
    for routed in router.route(event) {
        let destination = routed.event.endpoint.get().to_string();
        if let Some(output) = outputs.iter_mut().find(|output| output.info().id == destination) {
            output.send(routed.event);
            sent += 1;
        } else {
            unmatched += 1;
        }
    }
    (sent, unmatched)
}

/// Operational counters exposed by endpoint adapters.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EndpointStats {
    /// Events accepted for transmission.
    pub sent: u64,
    /// Events delivered to the router.
    pub received: u64,
    /// Events dropped because the bounded queue was full.
    pub dropped: u64,
}

/// Coarse MIDI message class used by route filters.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MessageClass {
    /// Note on/off or poly pressure.
    Note,
    /// Control change.
    ControlChange,
    /// Program change.
    ProgramChange,
    /// Pitch bend.
    PitchBend,
    /// System exclusive.
    SysEx,
    /// Other message class.
    Other,
}

impl MessageClass {
    const fn of(message: &MidiMessage) -> Self {
        match message {
            MidiMessage::NoteOn { .. }
            | MidiMessage::NoteOff { .. }
            | MidiMessage::PolyPressure { .. } => Self::Note,
            MidiMessage::ControlChange { .. } => Self::ControlChange,
            MidiMessage::ProgramChange { .. } => Self::ProgramChange,
            MidiMessage::PitchBend { .. } => Self::PitchBend,
            MidiMessage::SysEx(_) => Self::SysEx,
            _ => Self::Other,
        }
    }
}

/// Stable identity for one physical MIDI control stream.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ActivityKey {
    /// Source endpoint identity.
    pub endpoint: EndpointId,
    /// MIDI message family.
    pub class: MessageClass,
    /// Note/controller/program number when the message has one.
    pub number: Option<u8>,
}

/// Latest observed value for one activity key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivitySample {
    /// Stable control identity.
    pub key: ActivityKey,
    /// Latest source event sequence.
    pub sequence: u64,
    /// Latest source timestamp.
    pub timestamp: mackes_domain::TimestampNanos,
    /// Latest value, when the message family carries one.
    pub value: Option<u16>,
}

/// Bounded coalescer retaining only the newest sample per physical control.
#[derive(Clone, Debug)]
pub struct ActivityCoalescer {
    capacity: usize,
    samples: HashMap<ActivityKey, ActivitySample>,
}

impl ActivityCoalescer {
    /// Creates a coalescer with a positive maximum number of controls.
    #[must_use]
    pub fn new(capacity: usize) -> Option<Self> {
        (capacity > 0).then(|| Self { capacity, samples: HashMap::new() })
    }

    /// Inserts a sample, replacing only an older sample for the same control.
    ///
    /// Returns `false` when the sample is stale or the bounded store is full.
    pub fn push(&mut self, event: &mackes_domain::MidiEvent) -> bool {
        let key = ActivityKey {
            endpoint: event.endpoint,
            class: MessageClass::of(&event.message),
            number: message_number(&event.message),
        };
        let sample = ActivitySample {
            key,
            sequence: event.sequence,
            timestamp: event.timestamp,
            value: message_value(&event.message),
        };
        if self.samples.get(&key).is_some_and(|old| old.sequence >= sample.sequence) {
            return false;
        }
        if self.samples.len() >= self.capacity && !self.samples.contains_key(&key) {
            return false;
        }
        self.samples.insert(key, sample);
        true
    }

    /// Returns newest samples in deterministic endpoint/class/number order and clears them.
    pub fn drain(&mut self) -> Vec<ActivitySample> {
        let mut samples: Vec<_> = self.samples.drain().map(|(_, sample)| sample).collect();
        samples
            .sort_by_key(|sample| (sample.key.endpoint.get(), sample.key.class, sample.key.number));
        samples
    }

    /// Returns the number of currently coalesced controls.
    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Returns whether no coalesced activity samples are pending.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

/// Immutable validated route definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Route {
    /// Source endpoint.
    pub source: mackes_domain::EndpointId,
    /// Destination endpoint.
    pub destination: mackes_domain::EndpointId,
    /// Optional profile-owned destination parameter metadata.
    pub destination_parameter: Option<String>,
    /// Optional one-based channel filter.
    pub channel: Option<mackes_domain::MidiChannel>,
    /// Optional message class filter.
    pub class: Option<MessageClass>,
    /// Whether this route participates in evaluation.
    pub enabled: bool,
    /// Lower values execute first.
    pub priority: u16,
    /// Value shaping applied to continuous controller values.
    pub curve: Curve,
    /// Additional predicates, all of which must match.
    pub predicates: Vec<RoutePredicate>,
    /// Explicitly authorizes this edge to participate in a bounded cycle.
    pub allow_cycle: bool,
}

/// Fine-grained deterministic predicate applied after coarse route filters.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum RoutePredicate {
    /// Note, controller, or program number in an inclusive MIDI range.
    NumberRange {
        /// Inclusive lower bound.
        minimum: u8,
        /// Inclusive upper bound.
        maximum: u8,
    },
    /// Velocity, pressure, controller value, or pitch-bend value in an inclusive range.
    ValueRange {
        /// Inclusive lower bound.
        minimum: u16,
        /// Inclusive upper bound.
        maximum: u16,
    },
    /// Exact system real-time message.
    Realtime(RealtimeMessage),
    /// Masked `SysEx` payload match; framing bytes are excluded.
    SysExMask {
        /// Expected 7-bit payload bytes.
        pattern: Vec<u8>,
        /// Per-byte comparison mask.
        mask: Vec<u8>,
    },
}

impl RoutePredicate {
    fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::NumberRange { minimum, maximum } if minimum <= maximum && *maximum <= 127 => {
                Ok(())
            }
            Self::ValueRange { minimum, maximum } if minimum <= maximum && *maximum <= 16_383 => {
                Ok(())
            }
            Self::Realtime(_) => Ok(()),
            Self::SysExMask { pattern, mask }
                if !pattern.is_empty()
                    && pattern.len() == mask.len()
                    && pattern.len() <= 1024
                    && pattern.iter().all(|byte| *byte <= 127)
                    && mask.iter().all(|byte| *byte <= 127) =>
            {
                Ok(())
            }
            Self::NumberRange { .. } => Err("invalid MIDI number range"),
            Self::ValueRange { .. } => Err("invalid MIDI value range"),
            Self::SysExMask { .. } => Err("invalid SysEx mask predicate"),
        }
    }

    fn matches(&self, message: &MidiMessage) -> bool {
        match self {
            Self::NumberRange { minimum, maximum } => {
                message_number(message).is_some_and(|value| (*minimum..=*maximum).contains(&value))
            }
            Self::ValueRange { minimum, maximum } => {
                message_value(message).is_some_and(|value| (*minimum..=*maximum).contains(&value))
            }
            Self::Realtime(expected) => {
                matches!(message, MidiMessage::Realtime(actual) if actual == expected)
            }
            Self::SysExMask { pattern, mask } => match message {
                MidiMessage::SysEx(payload) if payload.len() == pattern.len() => payload
                    .iter()
                    .zip(pattern.iter().zip(mask))
                    .all(|(actual, (expected, mask))| actual.as_u8() & mask == expected & mask),
                _ => false,
            },
        }
    }
}

/// Validates a route predicate for editor and configuration consumers.
///
/// # Errors
///
/// Returns an error when bounds, payload size, or `SysEx` mask shape is invalid.
pub fn validate_route_predicate(predicate: &RoutePredicate) -> Result<(), &'static str> {
    predicate.validate()
}

const fn message_number(message: &MidiMessage) -> Option<u8> {
    match message {
        MidiMessage::NoteOn { note, .. }
        | MidiMessage::NoteOff { note, .. }
        | MidiMessage::PolyPressure { note, .. } => Some(note.as_u8()),
        MidiMessage::ControlChange { controller, .. } => Some(controller.as_u8()),
        MidiMessage::ProgramChange { program, .. } => Some(program.as_u8()),
        _ => None,
    }
}

fn message_value(message: &MidiMessage) -> Option<u16> {
    match message {
        MidiMessage::NoteOn { velocity, .. } | MidiMessage::NoteOff { velocity, .. } => {
            Some(u16::from(velocity.as_u8()))
        }
        MidiMessage::PolyPressure { pressure, .. }
        | MidiMessage::ChannelPressure { pressure, .. } => Some(u16::from(pressure.as_u8())),
        MidiMessage::ControlChange { value, .. } => Some(u16::from(value.as_u8())),
        MidiMessage::PitchBend { value, .. } => Some(value.get()),
        _ => None,
    }
}

/// Routed event carrying non-serialized provenance and hop count.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutedEvent {
    /// Event payload.
    pub event: MidiEvent,
    /// Route generation.
    pub generation: u64,
    /// Number of route hops.
    pub hops: u8,
}

/// Deterministic route evaluator; routes are evaluated in declaration order.
#[derive(Clone, Debug, Default)]
pub struct Router {
    routes: Vec<Route>,
    generation: u64,
    hop_limit: u8,
}

impl Router {
    /// Builds a router, rejecting zero or excessive hop limits.
    ///
    /// # Errors
    ///
    /// Returns an error for a hop limit outside 1..=16 or a self-loop.
    pub fn new(routes: Vec<Route>, generation: u64, hop_limit: u8) -> Result<Self, &'static str> {
        if !(1..=16).contains(&hop_limit) {
            return Err("hop limit must be 1..=16");
        }
        if routes.iter().any(|route| route.source == route.destination) {
            return Err("self-loop route");
        }
        if routes
            .iter()
            .flat_map(|route| &route.predicates)
            .any(|predicate| predicate.validate().is_err())
        {
            return Err("invalid route predicate");
        }
        if has_unauthorized_cycle(&routes) {
            return Err("route cycle requires explicit authorization on every edge");
        }
        Ok(Self { routes, generation, hop_limit })
    }

    /// Evaluates one event and returns matching destinations in stable order.
    #[must_use]
    pub fn route(&self, event: &MidiEvent) -> Vec<RoutedEvent> {
        self.route_with_hops(event, 0)
    }

    /// Evaluates an event entering this generation after `hops` prior hops.
    /// Events at the configured bound are dropped, preventing routing cycles
    /// from amplifying indefinitely.
    #[must_use]
    pub fn route_with_hops(&self, event: &MidiEvent, hops: u8) -> Vec<RoutedEvent> {
        if hops >= self.hop_limit {
            return Vec::new();
        }
        let mut routes = self
            .routes
            .iter()
            .filter(|route| route.enabled)
            .filter(|route| route.source == event.endpoint)
            .filter(|route| {
                route.channel.is_none_or(|channel| match &event.message {
                    MidiMessage::NoteOn { channel: actual, .. }
                    | MidiMessage::NoteOff { channel: actual, .. }
                    | MidiMessage::PolyPressure { channel: actual, .. }
                    | MidiMessage::ControlChange { channel: actual, .. }
                    | MidiMessage::ProgramChange { channel: actual, .. }
                    | MidiMessage::ChannelPressure { channel: actual, .. }
                    | MidiMessage::PitchBend { channel: actual, .. } => actual == &channel,
                    _ => false,
                })
            })
            .filter(|route| {
                route.class.is_none_or(|class| class == MessageClass::of(&event.message))
            })
            .filter(|route| {
                route.predicates.iter().all(|predicate| predicate.matches(&event.message))
            })
            .map(|route| {
                (
                    route.priority,
                    RoutedEvent {
                        event: MidiEvent {
                            endpoint: route.destination,
                            message: match event.message {
                                MidiMessage::ControlChange { channel, controller, value } => {
                                    MidiMessage::ControlChange {
                                        channel,
                                        controller,
                                        value: mackes_domain::SevenBit::new(u16::from(
                                            apply_curve(value.as_u8(), route.curve),
                                        ))
                                        .unwrap_or(value),
                                    }
                                }
                                _ => event.message.clone(),
                            },
                            ..event.clone()
                        },
                        generation: self.generation,
                        hops: hops.saturating_add(1),
                    },
                )
            })
            .collect::<Vec<_>>();
        routes.sort_by_key(|(priority, _)| *priority);
        routes.into_iter().map(|(_, route)| route).collect()
    }
}

fn has_unauthorized_cycle(routes: &[Route]) -> bool {
    routes.iter().any(|route| {
        let mut stack = vec![(route.destination, route.allow_cycle)];
        let mut visited = std::collections::HashSet::new();
        while let Some((node, authorized)) = stack.pop() {
            if node == route.source && !authorized {
                return true;
            }
            if !visited.insert((node, authorized)) {
                continue;
            }
            for next in routes.iter().filter(|next| next.source == node) {
                stack.push((next.destination, authorized && next.allow_cycle));
            }
        }
        false
    })
}

/// Atomically replaceable routing generation for concurrent readers.
#[derive(Clone, Debug)]
pub struct RouterStore(Arc<RwLock<Router>>);

/// Deterministic MIDI CC mapping rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CcMapping {
    /// Source controller number.
    pub source_controller: mackes_domain::SevenBit,
    /// Destination controller number.
    pub destination_controller: mackes_domain::SevenBit,
    /// Optional destination channel.
    pub destination_channel: Option<mackes_domain::MidiChannel>,
}

/// Message families supported by a typed number mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypedMappingKind {
    /// Note messages, preserving note-on/off or poly-pressure shape.
    Note,
    /// Control-change messages.
    ControlChange,
    /// Program-change messages.
    ProgramChange,
}

/// Deterministic mapping that changes a MIDI number without changing its message family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypedNumberMapping {
    /// Required source message family.
    pub source_kind: TypedMappingKind,
    /// Source note/controller/program number.
    pub source_number: SevenBit,
    /// Destination message family; must match `source_kind`.
    pub destination_kind: TypedMappingKind,
    /// Destination note/controller/program number.
    pub destination_number: SevenBit,
    /// Optional destination channel.
    pub destination_channel: Option<MidiChannel>,
}

/// A typed mapping guarded by an inclusive source-value condition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConditionalTypedMapping {
    /// Mapping applied when the condition matches.
    pub mapping: TypedNumberMapping,
    /// Inclusive lower source value.
    pub minimum: u8,
    /// Inclusive upper source value.
    pub maximum: u8,
}

/// Stateful absolute-control pickup gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PickupState {
    target: u16,
    tolerance: u16,
    acquired: bool,
}

/// Physical-control takeover behavior after a scene or page change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TakeoverMode {
    /// Apply the physical value immediately.
    Jump,
    /// Wait until the physical value reaches the target.
    Pickup {
        /// Destination-domain pickup tolerance.
        tolerance: u16,
    },
    /// Scale first, then wait until the scaled value reaches the target.
    ScaledPickup {
        /// Inclusive physical input range.
        input: (u16, u16),
        /// Inclusive destination range.
        output: (u16, u16),
        /// Destination-domain pickup tolerance.
        tolerance: u16,
    },
    /// Apply signed relative MIDI values, clamped to the destination range.
    Relative {
        /// Amount applied for each relative unit.
        step: u16,
        /// Inclusive destination range.
        range: (u16, u16),
    },
}

/// Stateful deterministic evaluator for one physical control.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TakeoverState {
    mode: TakeoverMode,
    target: u16,
    acquired: bool,
}

impl TakeoverState {
    /// Creates a takeover evaluator after validating all ranges.
    ///
    /// # Errors
    ///
    /// Rejects invalid ranges, a target outside its destination range, or a zero relative step.
    pub fn new(mode: TakeoverMode, target: u16) -> Result<Self, &'static str> {
        match mode {
            TakeoverMode::Jump | TakeoverMode::Pickup { .. } if target <= 16_383 => {}
            TakeoverMode::ScaledPickup { input, output, .. }
                if input.0 < input.1
                    && output.0 <= output.1
                    && output.1 <= 16_383
                    && (output.0..=output.1).contains(&target) => {}
            TakeoverMode::Relative { step, range }
                if step > 0
                    && range.0 <= range.1
                    && range.1 <= 16_383
                    && (range.0..=range.1).contains(&target) => {}
            _ => return Err("takeover mode or target is invalid"),
        }
        Ok(Self {
            mode,
            target,
            acquired: matches!(mode, TakeoverMode::Jump | TakeoverMode::Relative { .. }),
        })
    }

    /// Applies one MIDI 7-bit physical value, returning `None` while pickup is pending.
    ///
    /// Relative input uses the common binary-offset convention: 1–63 increase, 65–127
    /// decrease, and 0/64 are neutral.
    ///
    /// # Errors
    ///
    /// Returns an error when the physical input exceeds 127 or scaling is invalid.
    pub fn apply(&mut self, physical: u16) -> Result<Option<u16>, &'static str> {
        if physical > 127 {
            return Err("physical MIDI value exceeds 127");
        }
        match self.mode {
            TakeoverMode::Jump => {
                self.target = physical;
                Ok(Some(physical))
            }
            TakeoverMode::Pickup { tolerance } => {
                if !self.acquired && pickup_accept(physical, self.target, tolerance) {
                    self.acquired = true;
                }
                if self.acquired {
                    self.target = physical;
                    Ok(Some(physical))
                } else {
                    Ok(None)
                }
            }
            TakeoverMode::ScaledPickup { input, output, tolerance } => {
                let scaled = scale_value(physical, input, output, false)?;
                if !self.acquired && pickup_accept(scaled, self.target, tolerance) {
                    self.acquired = true;
                }
                if self.acquired {
                    self.target = scaled;
                    Ok(Some(scaled))
                } else {
                    Ok(None)
                }
            }
            TakeoverMode::Relative { step, range } => {
                let units = match physical {
                    1..=63 => i32::from(physical),
                    65..=127 => -i32::from(128 - physical),
                    0 | 64 => 0,
                    _ => unreachable!("validated MIDI 7-bit value"),
                };
                let candidate = i32::from(self.target)
                    .saturating_add(units.saturating_mul(i32::from(step)))
                    .clamp(i32::from(range.0), i32::from(range.1));
                self.target = u16::try_from(candidate).map_err(|_| "relative value overflow")?;
                Ok(Some(self.target))
            }
        }
    }

    /// Re-arms pickup modes at a new scene target.
    ///
    /// # Errors
    ///
    /// Returns an error if the new target violates the configured destination range.
    pub fn reset(&mut self, target: u16) -> Result<(), &'static str> {
        let replacement = Self::new(self.mode, target)?;
        *self = replacement;
        Ok(())
    }

    /// Returns the current destination target.
    #[must_use]
    pub const fn target(self) -> u16 {
        self.target
    }
}

/// Reset behavior for page-scoped mapping state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingReset {
    /// Retain the last value across scene changes.
    Preserve,
    /// Restore the mapping's declared scene default.
    SceneDefault,
    /// Force the mapping off.
    Off,
}

/// Stateful button behavior for one mapping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatefulMode {
    /// Alternate between zero and the declared on value on each press.
    Toggle {
        /// Value emitted for the on state.
        on_value: u8,
    },
    /// Emit the on value while pressed and zero on release.
    Latch {
        /// Value emitted while held.
        on_value: u8,
    },
    /// Select exactly one mapping in a named group.
    Radio {
        /// Stable mutual-exclusion group identifier.
        group: String,
        /// Value emitted for the selected member.
        on_value: u8,
    },
    /// Advance through a nonempty ordered value list on each press.
    Step {
        /// Ordered values visited on successive presses.
        values: Vec<u8>,
    },
}

/// One mapping value mutation produced by [`MappingStateStore`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappingChange {
    /// Stable mapping identifier.
    pub mapping_id: String,
    /// Active controller page.
    pub page: String,
    /// New MIDI 7-bit value.
    pub value: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MappingState {
    mode: StatefulMode,
    reset: MappingReset,
    scene_default: u8,
    value: u8,
    pressed: bool,
}

/// Page-isolated deterministic state store for button mappings.
#[derive(Clone, Debug, Default)]
pub struct MappingStateStore {
    states: HashMap<(String, String), MappingState>,
}

impl MappingStateStore {
    /// Registers one stable mapping on one page.
    ///
    /// # Errors
    ///
    /// Rejects empty identifiers, invalid MIDI values, duplicate keys, empty radio groups,
    /// or invalid/empty step tables.
    pub fn register(
        &mut self,
        mapping_id: impl Into<String>,
        page: impl Into<String>,
        mode: StatefulMode,
        reset: MappingReset,
        scene_default: u8,
    ) -> Result<(), &'static str> {
        let mapping_id = mapping_id.into();
        let page = page.into();
        if mapping_id.is_empty() || page.is_empty() || scene_default > 127 {
            return Err("mapping identity or default is invalid");
        }
        match &mode {
            StatefulMode::Toggle { on_value } | StatefulMode::Latch { on_value }
                if *on_value > 127 =>
            {
                return Err("mapping on value exceeds MIDI range")
            }
            StatefulMode::Radio { group, on_value } if group.is_empty() || *on_value > 127 => {
                return Err("radio mapping is invalid");
            }
            StatefulMode::Step { values }
                if values.is_empty()
                    || values.len() > 128
                    || values.iter().any(|value| *value > 127) =>
            {
                return Err("step mapping values are invalid");
            }
            _ => {}
        }
        let key = (mapping_id, page);
        if self.states.contains_key(&key) {
            return Err("duplicate page mapping");
        }
        self.states.insert(
            key,
            MappingState { mode, reset, scene_default, value: scene_default, pressed: false },
        );
        Ok(())
    }

    /// Applies one button value and returns every resulting mutation in stable identifier order.
    ///
    /// # Errors
    ///
    /// Returns an error when the mapping/page pair is unknown or the input exceeds 127.
    pub fn apply(
        &mut self,
        mapping_id: &str,
        page: &str,
        input: u8,
    ) -> Result<Vec<MappingChange>, &'static str> {
        if input > 127 {
            return Err("mapping input exceeds MIDI range");
        }
        let key = (mapping_id.to_owned(), page.to_owned());
        let state = self.states.get_mut(&key).ok_or("unknown page mapping")?;
        let now_pressed = input > 0;
        let rising = now_pressed && !state.pressed;
        state.pressed = now_pressed;
        let (next, radio_group) = match &state.mode {
            StatefulMode::Toggle { on_value } if rising => {
                (Some(if state.value == 0 { *on_value } else { 0 }), None)
            }
            StatefulMode::Latch { on_value } => {
                (Some(if now_pressed { *on_value } else { 0 }), None)
            }
            StatefulMode::Radio { group, on_value } if rising => {
                (Some(*on_value), Some(group.clone()))
            }
            StatefulMode::Step { values } if rising => {
                let index = values.iter().position(|value| *value == state.value);
                (Some(values[index.map_or(0, |value| (value + 1) % values.len())]), None)
            }
            _ => (None, None),
        };
        let mut changes = Vec::new();
        if let Some(group) = radio_group {
            for ((other_id, other_page), other) in &mut self.states {
                if other_page == page
                    && other_id != mapping_id
                    && matches!(&other.mode, StatefulMode::Radio { group: other_group, .. } if other_group == &group)
                    && other.value != 0
                {
                    other.value = 0;
                    changes.push(MappingChange {
                        mapping_id: other_id.clone(),
                        page: other_page.clone(),
                        value: 0,
                    });
                }
            }
        }
        if let Some(value) = next {
            let state = self.states.get_mut(&key).ok_or("unknown page mapping")?;
            if state.value != value {
                state.value = value;
                changes.push(MappingChange {
                    mapping_id: mapping_id.to_owned(),
                    page: page.to_owned(),
                    value,
                });
            }
        }
        changes.sort_by(|left, right| left.mapping_id.cmp(&right.mapping_id));
        Ok(changes)
    }

    /// Applies each mapping's declared scene-reset behavior on one page.
    #[must_use]
    pub fn reset_scene(&mut self, page: &str) -> Vec<MappingChange> {
        let mut changes = Vec::new();
        for ((mapping_id, mapping_page), state) in &mut self.states {
            if mapping_page != page || state.reset == MappingReset::Preserve {
                continue;
            }
            let value = if state.reset == MappingReset::Off { 0 } else { state.scene_default };
            state.pressed = false;
            if state.value != value {
                state.value = value;
                changes.push(MappingChange {
                    mapping_id: mapping_id.clone(),
                    page: mapping_page.clone(),
                    value,
                });
            }
        }
        changes.sort_by(|left, right| left.mapping_id.cmp(&right.mapping_id));
        changes
    }

    /// Returns a mapping's current page-scoped value.
    #[must_use]
    pub fn value(&self, mapping_id: &str, page: &str) -> Option<u8> {
        self.states.get(&(mapping_id.to_owned(), page.to_owned())).map(|state| state.value)
    }
}

/// Deterministic MIDI Learn candidate derived from observed CC traffic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LearnCandidate {
    /// Controller number observed.
    pub controller: mackes_domain::SevenBit,
    /// Number of observations for this controller.
    pub observations: u32,
}

/// MIDI message family retained by generalized Learn inference.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LearnMessageKind {
    /// Note-on message.
    NoteOn,
    /// Note-off message.
    NoteOff,
    /// Polyphonic key pressure.
    PolyPressure,
    /// Control change.
    ControlChange,
    /// Program change.
    ProgramChange,
    /// Channel pressure.
    ChannelPressure,
    /// Pitch bend.
    PitchBend,
    /// System-common message.
    SystemCommon,
    /// System real-time message.
    Realtime,
    /// Exact `SysEx` payload.
    SysEx,
}

/// One bounded generalized MIDI Learn candidate group.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiLearnCandidate {
    /// Message family.
    pub kind: LearnMessageKind,
    /// Exact one-based MIDI channel, when present.
    pub channel: Option<u8>,
    /// Controller, note, program, or system subtype when applicable.
    pub number: Option<u8>,
    /// Number of matching observations.
    pub observations: u32,
    /// Minimum observed continuous value.
    pub minimum: Option<u16>,
    /// Maximum observed continuous value.
    pub maximum: Option<u16>,
    /// Last complete wire message for decoded/raw review.
    pub raw: Vec<u8>,
}

/// Groups at most 128 events from one already-selected endpoint into deterministic candidates.
/// Exact channels are retained; callers may additionally offer an explicit any-channel policy.
#[must_use]
pub fn infer_midi_candidates(events: &[MidiEvent]) -> Vec<MidiLearnCandidate> {
    type Key = (LearnMessageKind, Option<u8>, Option<u8>, Vec<u8>);
    let mut groups: std::collections::BTreeMap<Key, MidiLearnCandidate> =
        std::collections::BTreeMap::new();
    for event in events.iter().take(128) {
        let (kind, channel, number, value, exact) = learn_signature(&event.message);
        let raw = event.message.wire_bytes();
        let key = (kind, channel, number, exact);
        let candidate = groups.entry(key).or_insert_with(|| MidiLearnCandidate {
            kind,
            channel,
            number,
            observations: 0,
            minimum: value,
            maximum: value,
            raw: raw.clone(),
        });
        candidate.observations = candidate.observations.saturating_add(1);
        if let Some(value) = value {
            candidate.minimum = Some(candidate.minimum.map_or(value, |minimum| minimum.min(value)));
            candidate.maximum = Some(candidate.maximum.map_or(value, |maximum| maximum.max(value)));
        }
        candidate.raw = raw;
    }
    groups.into_values().collect()
}

fn learn_signature(
    message: &MidiMessage,
) -> (LearnMessageKind, Option<u8>, Option<u8>, Option<u16>, Vec<u8>) {
    let channel = |channel: MidiChannel| Some(channel.one_based());
    match message {
        MidiMessage::NoteOn { channel: midi_channel, note, velocity } => (
            LearnMessageKind::NoteOn,
            channel(*midi_channel),
            Some(note.as_u8()),
            Some(u16::from(velocity.as_u8())),
            Vec::new(),
        ),
        MidiMessage::NoteOff { channel: midi_channel, note, velocity } => (
            LearnMessageKind::NoteOff,
            channel(*midi_channel),
            Some(note.as_u8()),
            Some(u16::from(velocity.as_u8())),
            Vec::new(),
        ),
        MidiMessage::PolyPressure { channel: midi_channel, note, pressure } => (
            LearnMessageKind::PolyPressure,
            channel(*midi_channel),
            Some(note.as_u8()),
            Some(u16::from(pressure.as_u8())),
            Vec::new(),
        ),
        MidiMessage::ControlChange { channel: midi_channel, controller, value } => (
            LearnMessageKind::ControlChange,
            channel(*midi_channel),
            Some(controller.as_u8()),
            Some(u16::from(value.as_u8())),
            Vec::new(),
        ),
        MidiMessage::ProgramChange { channel: midi_channel, program } => (
            LearnMessageKind::ProgramChange,
            channel(*midi_channel),
            Some(program.as_u8()),
            None,
            Vec::new(),
        ),
        MidiMessage::ChannelPressure { channel: midi_channel, pressure } => (
            LearnMessageKind::ChannelPressure,
            channel(*midi_channel),
            None,
            Some(u16::from(pressure.as_u8())),
            Vec::new(),
        ),
        MidiMessage::PitchBend { channel: midi_channel, value } => (
            LearnMessageKind::PitchBend,
            channel(*midi_channel),
            None,
            Some(value.get()),
            Vec::new(),
        ),
        MidiMessage::SystemCommon(_) => (
            LearnMessageKind::SystemCommon,
            None,
            message.wire_bytes().first().copied(),
            None,
            message.wire_bytes(),
        ),
        MidiMessage::Realtime(_) => (
            LearnMessageKind::Realtime,
            None,
            message.wire_bytes().first().copied(),
            None,
            message.wire_bytes(),
        ),
        MidiMessage::SysEx(payload) => (
            LearnMessageKind::SysEx,
            None,
            None,
            None,
            payload.iter().map(|byte| byte.as_u8()).collect(),
        ),
    }
}

impl LearnCandidate {
    /// Returns observation confidence in thousandths, bounded to 1000.
    #[must_use]
    pub fn confidence_milli(self, total_observations: u32) -> u16 {
        if total_observations == 0 {
            return 0;
        }
        let value = self.observations.saturating_mul(1000) / total_observations;
        u16::try_from(value.min(1000)).unwrap_or(1000)
    }
}

/// Summarizes at most 128 observations into stable controller candidates.
///
/// # Panics
///
/// This function cannot panic because controller indexes are bounded to 0..=127.
#[must_use]
pub fn infer_cc_candidates(events: &[MidiEvent]) -> Vec<LearnCandidate> {
    let mut counts = [0_u32; 128];
    for event in events.iter().take(128) {
        if let MidiMessage::ControlChange { controller, .. } = event.message {
            counts[usize::from(controller.get())] =
                counts[usize::from(controller.get())].saturating_add(1);
        }
    }
    counts
        .iter()
        .enumerate()
        .filter(|(_, observations)| **observations > 0)
        .map(|(controller, observations)| LearnCandidate {
            controller: mackes_domain::SevenBit::new(u16::try_from(controller).expect("bounded"))
                .expect("bounded"),
            observations: *observations,
        })
        .collect()
}

/// Selects the most frequently observed candidate, preferring the lower CC on ties.
#[must_use]
pub fn best_cc_candidate(candidates: &[LearnCandidate]) -> Option<LearnCandidate> {
    candidates.iter().copied().max_by_key(|candidate| {
        (
            candidate.observations,
            u8::MAX - u8::try_from(candidate.controller.get()).unwrap_or(u8::MAX),
        )
    })
}

impl PickupState {
    /// Creates a gate waiting for the physical control to reach the target.
    #[must_use]
    pub const fn new(target: u16, tolerance: u16) -> Self {
        Self { target, tolerance, acquired: false }
    }

    /// Accepts a value once it enters tolerance; subsequent values pass.
    pub const fn accept(&mut self, value: u16) -> bool {
        if !self.acquired && pickup_accept(value, self.target, self.tolerance) {
            self.acquired = true;
        }
        self.acquired
    }

    /// Resets pickup for a new scene target.
    pub const fn reset(&mut self, target: u16) {
        self.target = target;
        self.acquired = false;
    }
}

/// Scales a bounded MIDI value with optional inversion using integer arithmetic.
///
/// # Errors
///
/// Returns an error for invalid or zero-width ranges.
pub fn scale_value(
    value: u16,
    input: (u16, u16),
    output: (u16, u16),
    invert: bool,
) -> Result<u16, &'static str> {
    if input.0 >= input.1 || output.0 > output.1 || value < input.0 || value > input.1 {
        return Err("invalid scaling range or value");
    }
    let numerator = u32::from(value - input.0) * u32::from(output.1 - output.0);
    let span = u32::from(input.1 - input.0);
    let mut result = u16::try_from(u32::from(output.0) + (numerator / span))
        .map_err(|_| "scaled value overflow")?;
    if invert {
        result = output.1 - (result - output.0);
    }
    Ok(result)
}

/// Applies absolute-control pickup/takeover semantics.
#[must_use]
pub const fn pickup_accept(current: u16, target: u16, tolerance: u16) -> bool {
    current.abs_diff(target) <= tolerance
}

/// Approved deterministic mapping curve.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Curve {
    /// No shaping.
    Linear,
    /// Quadratic emphasis toward the low end.
    Square,
    /// Square-root emphasis toward the high end.
    SquareRoot,
}

/// Exact-match parameter mapping evaluated independently of ordinary routes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParameterMapping {
    /// Input endpoint identity.
    pub source_endpoint: EndpointId,
    /// Output endpoint identity.
    pub destination_endpoint: EndpointId,
    /// Required MIDI message family.
    pub class: MessageClass,
    /// Required MIDI number (controller, note, or program).
    pub number: u8,
    /// Optional zero-based channel restriction.
    pub channel: Option<u8>,
    /// Inclusive source value range.
    pub source_range: (u16, u16),
    /// Inclusive destination value range.
    pub destination_range: (u16, u16),
    /// Reverse the destination direction.
    pub invert: bool,
    /// Value curve.
    pub curve: Curve,
}

impl ParameterMapping {
    /// Evaluates one event, returning `None` for every non-exact source match.
    #[must_use]
    pub fn evaluate(&self, event: &mackes_domain::MidiEvent) -> Option<mackes_domain::MidiEvent> {
        self.evaluate_with_value(event).map(|(event, _)| event)
    }

    /// Evaluates one event and retains the full transformed destination value.
    #[must_use]
    pub fn evaluate_with_value(
        &self,
        event: &mackes_domain::MidiEvent,
    ) -> Option<(mackes_domain::MidiEvent, u16)> {
        if event.endpoint != self.source_endpoint
            || MessageClass::of(&event.message) != self.class
            || message_number(&event.message) != Some(self.number)
            || self.channel.is_some_and(|channel| {
                event_channel(&event.message).is_none_or(|actual| actual.one_based() != channel + 1)
            })
        {
            return None;
        }
        let output = if let Some(value) = message_value(&event.message) {
            let shaped = apply_curve(u8::try_from(value).ok()?, self.curve);
            scale_value(u16::from(shaped), self.source_range, self.destination_range, self.invert)
                .ok()?
        } else {
            self.destination_range.0
        };
        let value = mackes_domain::SevenBit::new(output.min(127))?;
        let message = match &event.message {
            mackes_domain::MidiMessage::ControlChange { channel, controller, .. } => {
                mackes_domain::MidiMessage::ControlChange {
                    channel: *channel,
                    controller: *controller,
                    value,
                }
            }
            mackes_domain::MidiMessage::ProgramChange { channel, .. } => {
                mackes_domain::MidiMessage::ProgramChange { channel: *channel, program: value }
            }
            _ => return None,
        };
        Some((
            mackes_domain::MidiEvent {
                endpoint: self.destination_endpoint,
                message,
                ..event.clone()
            },
            output,
        ))
    }
}

/// Applies a curve to a normalized 0..=127 MIDI value.
#[must_use]
pub fn apply_curve(value: u8, curve: Curve) -> u8 {
    if value == 0 {
        return 0;
    }
    match curve {
        Curve::Linear => value,
        Curve::Square => {
            u8::try_from((u16::from(value) * u16::from(value) + 127) / 127).unwrap_or(127).min(127)
        }
        Curve::SquareRoot => {
            let target = u16::from(value) * 127;
            (0..=127)
                .rev()
                .find(|candidate| u16::from(*candidate) * u16::from(*candidate) <= target)
                .unwrap_or(0)
        }
    }
}

/// Deterministic scheduled output item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledEvent {
    /// Monotonic due time in nanoseconds.
    pub due_at: u64,
    /// Stable insertion order.
    pub order: u64,
    /// MIDI event to emit.
    pub event: MidiEvent,
}

/// Fake-clock scheduler contract for ordered, cancellable output.
#[derive(Clone, Debug, Default)]
pub struct Scheduler {
    pending: Vec<ScheduledEvent>,
    next_order: u64,
}

/// Bounded pacing and retry policy for SysEx/device operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetryPolicy {
    /// Minimum interval between messages in nanoseconds.
    pub min_interval_ns: u64,
    /// Maximum retry attempts after the initial send.
    pub retries: u8,
    /// Base retry delay in nanoseconds.
    pub retry_delay_ns: u64,
}

impl RetryPolicy {
    /// Computes a capped exponential retry delay.
    #[must_use]
    pub const fn delay_for(self, attempt: u8) -> u64 {
        let shift = if attempt > 6 { 6 } else { attempt };
        self.retry_delay_ns.saturating_mul(1_u64 << shift)
    }
}

/// State for one bounded request/response exchange with a MIDI device.
/// Only one exchange should be active per device unless its protocol explicitly
/// documents multiplexing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PendingTransaction {
    /// Monotonic time at which the current attempt expires.
    pub deadline_ns: u64,
    /// Monotonic time at which another attempt may be sent.
    pub next_retry_ns: u64,
    /// Number of attempts already sent, including the initial attempt.
    pub attempts_sent: u8,
    /// Maximum retries permitted after the initial attempt.
    pub max_retries: u8,
    /// Whether a matching response has completed the exchange.
    pub completed: bool,
}

/// Exact-length masked matcher for a documented device response prefix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponseMatcher {
    /// Expected bytes.
    pub value: Vec<u8>,
    /// Mask bits; a one bit participates in comparison.
    pub mask: Vec<u8>,
}

/// One bounded captured MIDI message retained for query correlation and diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureRecord {
    /// Source endpoint.
    pub endpoint: EndpointId,
    /// Monotonic capture time.
    pub timestamp_ns: u64,
    /// Complete wire bytes.
    pub bytes: Vec<u8>,
    /// Whether a query claimed this record as its reply.
    pub matched: bool,
}

/// Bounded capture store that never discards unsolicited messages during correlation.
#[derive(Clone, Debug)]
pub struct CaptureStore {
    capacity: usize,
    records: VecDeque<CaptureRecord>,
}

impl CaptureStore {
    /// Creates a bounded store.
    ///
    /// # Errors
    ///
    /// Rejects zero or more than 65,536 retained records.
    pub const fn new(capacity: usize) -> Result<Self, &'static str> {
        if capacity == 0 || capacity > 65_536 {
            return Err("capture capacity is invalid");
        }
        Ok(Self { capacity, records: VecDeque::new() })
    }

    /// Retains one complete message and evicts the oldest record at capacity.
    ///
    /// # Errors
    ///
    /// Rejects empty messages or messages above the generic 8 KiB bound.
    pub fn push(&mut self, record: CaptureRecord) -> Result<(), &'static str> {
        if record.bytes.is_empty() || record.bytes.len() > ResponseMatcher::MAX_BYTES {
            return Err("capture message size is invalid");
        }
        if self.records.len() == self.capacity {
            self.records.pop_front();
        }
        self.records.push_back(record);
        Ok(())
    }

    /// Correlates the earliest unmatched reply from the exact endpoint and inclusive time window.
    /// Unmatched and unsolicited captures remain available.
    pub fn correlate(
        &mut self,
        endpoint: EndpointId,
        start_ns: u64,
        end_ns: u64,
        matcher: &ResponseMatcher,
    ) -> Option<&CaptureRecord> {
        if start_ns > end_ns {
            return None;
        }
        let index = self.records.iter().position(|record| {
            !record.matched
                && record.endpoint == endpoint
                && (start_ns..=end_ns).contains(&record.timestamp_ns)
                && matcher.matches(&record.bytes)
        })?;
        self.records[index].matched = true;
        self.records.get(index)
    }

    /// Returns retained unsolicited/unclaimed records in arrival order.
    pub fn unmatched(&self) -> impl Iterator<Item = &CaptureRecord> {
        self.records.iter().filter(|record| !record.matched)
    }

    /// Returns all retained records in arrival order.
    #[must_use]
    pub const fn records(&self) -> &VecDeque<CaptureRecord> {
        &self.records
    }
}

/// Named bounded field in a captured byte message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureField {
    /// Display label from profile metadata.
    pub name: String,
    /// Zero-based byte offset.
    pub offset: usize,
    /// Number of base-128 big-endian bytes (1–8).
    pub length: usize,
}

/// Decodes named 7-bit fields from one captured message.
///
/// # Errors
///
/// Rejects empty/duplicate names, invalid bounds, non-7-bit bytes, and numeric overflow.
pub fn decode_capture_fields(
    bytes: &[u8],
    fields: &[CaptureField],
) -> Result<Vec<(String, u64)>, &'static str> {
    let mut decoded = Vec::with_capacity(fields.len());
    for field in fields {
        if field.name.trim().is_empty()
            || field.length == 0
            || field.length > 8
            || field.offset.checked_add(field.length).is_none_or(|end| end > bytes.len())
            || decoded.iter().any(|(name, _)| name == &field.name)
        {
            return Err("capture field definition is invalid");
        }
        let mut value = 0_u64;
        for byte in &bytes[field.offset..field.offset + field.length] {
            if *byte > 127 {
                return Err("capture field contains non-MIDI data");
            }
            value = value
                .checked_mul(128)
                .and_then(|value| value.checked_add(u64::from(*byte)))
                .ok_or("capture field value overflow")?;
        }
        decoded.push((field.name.clone(), value));
    }
    Ok(decoded)
}

/// Returns differing byte positions, including positions present on only one side.
#[must_use]
pub fn diff_capture_bytes(left: &[u8], right: &[u8]) -> Vec<usize> {
    (0..left.len().max(right.len())).filter(|index| left.get(*index) != right.get(*index)).collect()
}

/// Validated outbound request for one device transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceRequest {
    /// MIDI/SysEx bytes to transmit.
    pub bytes: Vec<u8>,
    /// Expected response correlation pattern.
    pub response: ResponseMatcher,
    /// Pacing and retry behavior.
    pub policy: RetryPolicy,
    /// Response timeout in nanoseconds.
    pub timeout_ns: u64,
}

impl DeviceRequest {
    /// Maximum request size accepted by the generic executor boundary.
    pub const MAX_BYTES: usize = 8192;

    /// Creates a request after validating bounds and timeout.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty/oversized payload or zero timeout.
    pub fn new(
        bytes: Vec<u8>,
        response: ResponseMatcher,
        policy: RetryPolicy,
        timeout_ns: u64,
    ) -> Result<Self, &'static str> {
        if bytes.is_empty() || bytes.len() > Self::MAX_BYTES || timeout_ns == 0 {
            return Err("device request bounds are invalid");
        }
        Ok(Self { bytes, response, policy, timeout_ns })
    }

    /// Starts the bounded transaction at a monotonic timestamp.
    #[must_use]
    pub const fn begin(&self, now: u64) -> Option<PendingTransaction> {
        PendingTransaction::start(now, self.timeout_ns, self.policy)
    }

    /// Converts the request bytes into one validated output event.
    ///
    /// # Errors
    ///
    /// Returns an error when the bytes are not exactly one valid MIDI message
    /// or the endpoint identity is invalid.
    pub fn to_event(
        &self,
        endpoint: u64,
        timestamp: u64,
        sequence: u64,
    ) -> Result<MidiEvent, &'static str> {
        Ok(MidiEvent {
            timestamp: TimestampNanos::new(timestamp),
            sequence,
            endpoint: EndpointId::new(endpoint).ok_or("MIDI endpoint ID must be nonzero")?,
            message: MidiMessage::from_wire(&self.bytes)?,
        })
    }
}

impl ResponseMatcher {
    /// Maximum expected response pattern size.
    pub const MAX_BYTES: usize = 8192;

    /// Creates a matcher, rejecting empty or mismatched patterns.
    ///
    /// # Errors
    ///
    /// Returns an error when either pattern is empty or their lengths differ.
    pub fn new(value: Vec<u8>, mask: Vec<u8>) -> Result<Self, &'static str> {
        if value.is_empty() || value.len() != mask.len() || value.len() > Self::MAX_BYTES {
            return Err("response matcher length is invalid");
        }
        Ok(Self { value, mask })
    }

    /// Returns whether the complete response matches the documented pattern.
    #[must_use]
    pub fn matches(&self, response: &[u8]) -> bool {
        response.len() == self.value.len()
            && response
                .iter()
                .zip(self.value.iter().zip(self.mask.iter()))
                .all(|(actual, (expected, mask))| actual & mask == expected & mask)
    }
}

impl PendingTransaction {
    /// Starts a transaction at `now`, with a bounded response timeout.
    #[must_use]
    pub const fn start(now: u64, timeout_ns: u64, policy: RetryPolicy) -> Option<Self> {
        if timeout_ns == 0 {
            return None;
        }
        Some(Self {
            deadline_ns: now.saturating_add(timeout_ns),
            next_retry_ns: now.saturating_add(policy.min_interval_ns),
            attempts_sent: 1,
            max_retries: policy.retries,
            completed: false,
        })
    }

    /// Returns whether the response deadline has elapsed.
    #[must_use]
    pub const fn timed_out(self, now: u64) -> bool {
        now >= self.deadline_ns
    }

    /// Accepts a response only when the exchange is still within its deadline
    /// and the supplied documented matcher matches the complete payload.
    #[must_use]
    pub fn accepts_response(self, now: u64, matcher: &ResponseMatcher, response: &[u8]) -> bool {
        !self.completed && !self.timed_out(now) && matcher.matches(response)
    }

    /// Marks the exchange complete when a timely matching response arrives.
    pub fn complete_if_matches(
        &mut self,
        now: u64,
        matcher: &ResponseMatcher,
        response: &[u8],
    ) -> bool {
        if !self.accepts_response(now, matcher, response) {
            return false;
        }
        self.completed = true;
        true
    }

    /// Advances to a retry attempt when permitted and pacing allows it.
    pub const fn retry(&mut self, now: u64, timeout_ns: u64, policy: RetryPolicy) -> bool {
        if self.completed
            || self.attempts_sent.saturating_sub(1) >= self.max_retries
            || now < self.next_retry_ns
        {
            return false;
        }
        self.attempts_sent = self.attempts_sent.saturating_add(1);
        self.deadline_ns = now.saturating_add(timeout_ns);
        self.next_retry_ns = now.saturating_add(policy.min_interval_ns);
        true
    }
}

impl Scheduler {
    /// Queues one event after a relative delay.
    pub fn schedule(&mut self, now: u64, delay: u64, event: MidiEvent) {
        let order = self.next_order;
        self.next_order = self.next_order.saturating_add(1);
        self.pending.push(ScheduledEvent { due_at: now.saturating_add(delay), order, event });
    }
    /// Cancels all pending events matching an event sequence.
    pub fn cancel_sequence(&mut self, sequence: u64) {
        self.pending.retain(|item| item.event.sequence != sequence);
    }
    /// Cancels every pending event, returning the number removed.
    pub fn cancel_all(&mut self) -> usize {
        let count = self.pending.len();
        self.pending.clear();
        count
    }
    /// Removes all events due at or before the supplied fake-clock time.
    pub fn drain_due(&mut self, now: u64) -> Vec<MidiEvent> {
        self.pending.sort_by_key(|item| (item.due_at, item.order));
        let split = self.pending.partition_point(|item| item.due_at <= now);
        self.pending.drain(..split).map(|item| item.event).collect()
    }
}

impl CcMapping {
    /// Applies the mapping, returning `None` for non-CC events or other channels.
    #[must_use]
    pub fn apply(&self, event: &MidiEvent) -> Option<MidiEvent> {
        let MidiMessage::ControlChange { channel, controller: source, value } = &event.message
        else {
            return None;
        };
        if *source != self.source_controller {
            return None;
        }
        Some(MidiEvent {
            message: MidiMessage::ControlChange {
                channel: self.destination_channel.unwrap_or(*channel),
                controller: self.destination_controller,
                value: *value,
            },
            ..event.clone()
        })
    }
}

impl TypedNumberMapping {
    /// Applies a type-safe number mapping, rejecting family conversions.
    #[must_use]
    pub fn apply(&self, event: &MidiEvent) -> Option<MidiEvent> {
        if self.source_kind != self.destination_kind
            || message_number(&event.message) != Some(self.source_number.as_u8())
        {
            return None;
        }
        let channel = self.destination_channel;
        let message = match (&event.message, self.source_kind) {
            (MidiMessage::NoteOn { velocity, .. }, TypedMappingKind::Note) => MidiMessage::NoteOn {
                channel: channel.unwrap_or(event_channel(&event.message)?),
                note: self.destination_number,
                velocity: *velocity,
            },
            (MidiMessage::NoteOff { velocity, .. }, TypedMappingKind::Note) => {
                MidiMessage::NoteOff {
                    channel: channel.unwrap_or(event_channel(&event.message)?),
                    note: self.destination_number,
                    velocity: *velocity,
                }
            }
            (MidiMessage::PolyPressure { pressure, .. }, TypedMappingKind::Note) => {
                MidiMessage::PolyPressure {
                    channel: channel.unwrap_or(event_channel(&event.message)?),
                    note: self.destination_number,
                    pressure: *pressure,
                }
            }
            (MidiMessage::ControlChange { value, .. }, TypedMappingKind::ControlChange) => {
                MidiMessage::ControlChange {
                    channel: channel.unwrap_or(event_channel(&event.message)?),
                    controller: self.destination_number,
                    value: *value,
                }
            }
            (MidiMessage::ProgramChange { .. }, TypedMappingKind::ProgramChange) => {
                MidiMessage::ProgramChange {
                    channel: channel.unwrap_or(event_channel(&event.message)?),
                    program: self.destination_number,
                }
            }
            _ => return None,
        };
        Some(MidiEvent { message, ..event.clone() })
    }
}

impl ConditionalTypedMapping {
    /// Applies the mapping only when the source value is present and in range.
    #[must_use]
    pub fn apply(&self, event: &MidiEvent) -> Option<MidiEvent> {
        let value = message_value(&event.message)?;
        if value > 127 || !(u16::from(self.minimum)..=u16::from(self.maximum)).contains(&value) {
            return None;
        }
        self.mapping.apply(event)
    }
}

const fn event_channel(message: &MidiMessage) -> Option<MidiChannel> {
    match message {
        MidiMessage::NoteOn { channel, .. }
        | MidiMessage::NoteOff { channel, .. }
        | MidiMessage::PolyPressure { channel, .. }
        | MidiMessage::ControlChange { channel, .. }
        | MidiMessage::ProgramChange { channel, .. } => Some(*channel),
        _ => None,
    }
}

impl RouterStore {
    /// Creates a store after validating the initial route set.
    ///
    /// # Errors
    ///
    /// Returns the same validation errors as [`Router::new`].
    pub fn new(routes: Vec<Route>, generation: u64, hop_limit: u8) -> Result<Self, &'static str> {
        Ok(Self(Arc::new(RwLock::new(Router::new(routes, generation, hop_limit)?))))
    }

    /// Replaces the complete generation atomically after validation.
    ///
    /// # Errors
    ///
    /// Returns an error when the proposed route set is invalid or the lock is poisoned.
    pub fn swap(
        &self,
        routes: Vec<Route>,
        generation: u64,
        hop_limit: u8,
    ) -> Result<(), &'static str> {
        let replacement = Router::new(routes, generation, hop_limit)?;
        *self.0.write().map_err(|_| "router lock poisoned")? = replacement;
        Ok(())
    }

    /// Routes against one immutable generation snapshot.
    #[must_use]
    pub fn route(&self, event: &MidiEvent) -> Vec<RoutedEvent> {
        self.0.read().map_or_else(|_| Vec::new(), |router| router.route(event))
    }

    /// Routes an event while preserving an existing hop count across generations.
    #[must_use]
    pub fn route_with_hops(&self, event: &MidiEvent, hops: u8) -> Vec<RoutedEvent> {
        self.0.read().map_or_else(|_| Vec::new(), |router| router.route_with_hops(event, hops))
    }

    /// Returns the generation currently visible to new route evaluations.
    #[must_use]
    pub fn generation(&self) -> Option<u64> {
        self.0.read().ok().map(|router| router.generation)
    }

    /// Returns a stable snapshot of the currently installed routes.
    #[must_use]
    pub fn routes(&self) -> Vec<Route> {
        self.0.read().map_or_else(|_| Vec::new(), |router| router.routes.clone())
    }
}

/// Deterministic in-memory endpoint used by tests and virtual MIDI integration.
#[derive(Debug)]
pub struct VirtualEndpoint {
    info: EndpointInfo,
    queue: VecDeque<MidiEvent>,
    stats: EndpointStats,
    capacity: usize,
}

impl VirtualEndpoint {
    /// Creates a virtual endpoint with the requested direction.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        direction: EndpointDirection,
    ) -> Self {
        Self {
            info: EndpointInfo { id: id.into(), name: name.into(), direction },
            queue: VecDeque::new(),
            stats: EndpointStats::default(),
            capacity: 1024,
        }
    }

    /// Sets the bounded queue capacity; zero is clamped to one.
    #[must_use]
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity.max(1);
        self
    }

    /// Returns a snapshot of endpoint counters.
    #[must_use]
    pub const fn stats(&self) -> EndpointStats {
        self.stats
    }
}

impl MidiInputAdapter for VirtualEndpoint {
    fn info(&self) -> &EndpointInfo {
        &self.info
    }

    fn receive(&mut self) -> Option<MidiEvent> {
        let event = self.queue.pop_front();
        if event.is_some() {
            self.stats.received = self.stats.received.saturating_add(1);
        }
        event
    }
}

impl MidiOutputAdapter for VirtualEndpoint {
    fn info(&self) -> &EndpointInfo {
        &self.info
    }

    fn send(&mut self, event: MidiEvent) {
        if self.queue.len() >= self.capacity {
            self.stats.dropped = self.stats.dropped.saturating_add(1);
        } else {
            self.queue.push_back(event);
            self.stats.sent = self.stats.sent.saturating_add(1);
        }
    }
}

/// Explicitly opened Fedora/ALSA MIDI output backed by `midir`.
#[cfg(feature = "midir-backend")]
pub struct MidirOutputAdapter {
    info: EndpointInfo,
    connection: midir::MidiOutputConnection,
    sent: u64,
    failed: u64,
}

/// Explicitly opened Fedora/ALSA MIDI input capture with bounded raw messages.
#[cfg(feature = "midir-backend")]
pub struct MidirInputCapture {
    info: EndpointInfo,
    queue: std::sync::Arc<std::sync::Mutex<VecDeque<Vec<u8>>>>,
    _connection: midir::MidiInputConnection<()>,
    next_sequence: u64,
    received: u64,
    malformed: u64,
}

#[cfg(feature = "midir-backend")]
impl std::fmt::Debug for MidirInputCapture {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MidirInputCapture")
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "midir-backend")]
impl MidirInputCapture {
    /// Opens an input by its exact backend-reported name.
    ///
    /// # Errors
    ///
    /// Returns an error when ALSA cannot initialize, the name is unavailable,
    /// or the backend refuses the connection.
    pub fn open_named(name: &str) -> Result<Self, String> {
        use midir::Ignore;
        let mut midi = midir::MidiInput::new("MACKES input").map_err(|error| error.to_string())?;
        midi.ignore(Ignore::None);
        let port = midi
            .ports()
            .into_iter()
            .find(|port| midi.port_name(port).is_ok_and(|candidate| candidate == name))
            .ok_or_else(|| format!("MIDI input not found: {name}"))?;
        let queue = std::sync::Arc::new(std::sync::Mutex::new(VecDeque::new()));
        let callback_queue = std::sync::Arc::clone(&queue);
        let connection = midi
            .connect(
                &port,
                "MACKES input",
                move |_stamp, bytes, ()| {
                    if let Ok(mut queue) = callback_queue.lock() {
                        if queue.len() < 1024 {
                            queue.push_back(bytes.to_vec());
                        }
                    }
                },
                (),
            )
            .map_err(|error| error.to_string())?;
        Ok(Self {
            info: EndpointInfo {
                id: stable_endpoint_id(name, EndpointDirection::Input),
                name: name.to_owned(),
                direction: EndpointDirection::Input,
            },
            queue,
            _connection: connection,
            next_sequence: 0,
            received: 0,
            malformed: 0,
        })
    }

    /// Returns the next captured raw MIDI message, if available.
    pub fn receive_raw(&mut self) -> Option<Vec<u8>> {
        self.queue.lock().ok().and_then(|mut queue| queue.pop_front())
    }

    /// Decodes the next captured message into a domain event.
    ///
    /// # Errors
    ///
    /// Returns the domain decoder error when the captured bytes are malformed.
    pub fn receive_event(
        &mut self,
        timestamp: mackes_domain::TimestampNanos,
        sequence: u64,
    ) -> Result<Option<MidiEvent>, &'static str> {
        let Some(bytes) = self.receive_raw() else { return Ok(None) };
        let message = match mackes_domain::MidiMessage::from_wire(&bytes) {
            Ok(message) => {
                self.received = self.received.saturating_add(1);
                message
            }
            Err(error) => {
                self.malformed = self.malformed.saturating_add(1);
                return Err(error);
            }
        };
        let endpoint = numeric_endpoint_id(&self.info.id).ok_or("invalid input endpoint")?;
        Ok(Some(MidiEvent { timestamp, sequence, endpoint, message }))
    }

    /// Returns received and malformed-message counters.
    #[must_use]
    pub const fn counters(&self) -> (u64, u64) {
        (self.received, self.malformed)
    }
}

#[cfg(feature = "midir-backend")]
impl MidiInputAdapter for MidirInputCapture {
    fn info(&self) -> &EndpointInfo {
        &self.info
    }

    fn receive(&mut self) -> Option<MidiEvent> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|duration| u64::try_from(duration.as_nanos()).ok())
            .map(mackes_domain::TimestampNanos::new)?;
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.receive_event(timestamp, sequence).ok().flatten()
    }
}

#[cfg(feature = "midir-backend")]
impl std::fmt::Debug for MidirOutputAdapter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MidirOutputAdapter")
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "midir-backend")]
impl MidirOutputAdapter {
    /// Opens an output by its exact backend-reported name.
    ///
    /// # Errors
    ///
    /// Returns an error when ALSA cannot initialize, the name is unavailable,
    /// or the backend refuses the connection.
    pub fn open_named(name: &str) -> Result<Self, String> {
        let midi = midir::MidiOutput::new("MACKES output").map_err(|error| error.to_string())?;
        let port = midi
            .ports()
            .into_iter()
            .find(|port| midi.port_name(port).is_ok_and(|candidate| candidate == name))
            .ok_or_else(|| format!("MIDI output not found: {name}"))?;
        let connection = midi.connect(&port, "MACKES output").map_err(|error| error.to_string())?;
        Ok(Self {
            info: EndpointInfo {
                id: stable_endpoint_id(name, EndpointDirection::Output),
                name: name.to_owned(),
                direction: EndpointDirection::Output,
            },
            connection,
            sent: 0,
            failed: 0,
        })
    }

    /// Opens an output by a stable discovered endpoint ID.
    ///
    /// # Errors
    ///
    /// Returns an error when the ID is not an output endpoint, the endpoint
    /// disappeared, or the backend refuses the connection.
    pub fn open_id(id: &str) -> Result<Self, String> {
        let midi = midir::MidiOutput::new("MACKES output").map_err(|error| error.to_string())?;
        for port in midi.ports() {
            let name = midi.port_name(&port).map_err(|error| error.to_string())?;
            if stable_endpoint_id(&name, EndpointDirection::Output) == id {
                let connection =
                    midi.connect(&port, "MACKES output").map_err(|error| error.to_string())?;
                return Ok(Self {
                    info: EndpointInfo {
                        id: id.to_owned(),
                        name,
                        direction: EndpointDirection::Output,
                    },
                    connection,
                    sent: 0,
                    failed: 0,
                });
            }
        }
        Err(format!("MIDI output endpoint unavailable: {id}"))
    }

    /// Sends one validated MIDI event and returns a backend error if delivery fails.
    ///
    /// # Errors
    ///
    /// Returns the backend send error when the output connection is unavailable.
    pub fn send_checked(&mut self, event: &MidiEvent) -> Result<(), String> {
        match self.connection.send(&event.message.wire_bytes()) {
            Ok(()) => {
                self.sent = self.sent.saturating_add(1);
                Ok(())
            }
            Err(error) => {
                self.failed = self.failed.saturating_add(1);
                Err(error.to_string())
            }
        }
    }

    /// Returns successful and failed send counters.
    #[must_use]
    pub const fn counters(&self) -> (u64, u64) {
        (self.sent, self.failed)
    }
}

#[cfg(feature = "midir-backend")]
impl MidiOutputAdapter for MidirOutputAdapter {
    fn info(&self) -> &EndpointInfo {
        &self.info
    }

    fn send(&mut self, event: MidiEvent) {
        let _ = self.send_checked(&event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mackes_domain::{MidiChannel, MidiMessage, TimestampNanos};

    #[test]
    fn virtual_endpoint_preserves_output_order() {
        let mut endpoint =
            VirtualEndpoint::new("virtual-out", "MACKES DAW Out", EndpointDirection::Output);
        for note in [60, 61] {
            endpoint.send(MidiEvent {
                timestamp: TimestampNanos::new(u64::from(note)),
                sequence: u64::from(note),
                endpoint: mackes_domain::EndpointId::new(1).expect("endpoint"),
                message: MidiMessage::NoteOn {
                    channel: MidiChannel::new(1).expect("channel"),
                    note: mackes_domain::SevenBit::new(note).expect("note"),
                    velocity: mackes_domain::SevenBit::new(100).expect("velocity"),
                },
            });
        }
        assert_eq!(endpoint.receive().expect("first").sequence, 60);
        assert_eq!(endpoint.receive().expect("second").sequence, 61);
        assert!(endpoint.receive().is_none());
    }

    #[test]
    fn router_filters_compound_route_and_enforces_hop_limit() {
        let source = mackes_domain::EndpointId::new(1).expect("source");
        let destination = mackes_domain::EndpointId::new(2).expect("destination");
        let router = Router::new(
            vec![Route {
                source,
                destination,
                destination_parameter: None,
                channel: Some(MidiChannel::new(2).expect("channel")),
                class: Some(MessageClass::ControlChange),
                enabled: true,
                priority: 0,
                curve: Curve::Linear,
                predicates: Vec::new(),
                allow_cycle: false,
            }],
            7,
            2,
        )
        .expect("router");
        let event = MidiEvent {
            timestamp: TimestampNanos::new(1),
            sequence: 1,
            endpoint: source,
            message: MidiMessage::ControlChange {
                channel: MidiChannel::new(2).expect("channel"),
                controller: mackes_domain::SevenBit::new(10).expect("cc"),
                value: mackes_domain::SevenBit::new(64).expect("value"),
            },
        };
        assert_eq!(router.route(&event).len(), 1);
        assert_eq!(router.route_with_hops(&event, 2), Vec::new());
        let wrong_channel = MidiEvent {
            timestamp: event.timestamp,
            sequence: event.sequence,
            endpoint: event.endpoint,
            message: MidiMessage::ControlChange {
                channel: MidiChannel::new(1).expect("channel"),
                controller: mackes_domain::SevenBit::new(10).expect("cc"),
                value: mackes_domain::SevenBit::new(64).expect("value"),
            },
        };
        assert!(router.route(&wrong_channel).is_empty());
        let store = RouterStore::new(Vec::new(), 7, 2).expect("store");
        assert_eq!(store.generation(), Some(7));
        assert!(store.routes().is_empty());
        store.swap(Vec::new(), 8, 2).expect("swap");
        assert_eq!(store.generation(), Some(8));
    }

    #[test]
    fn router_enforces_enabled_priority_and_cc_curve() {
        let source = EndpointId::new(1).expect("source");
        let first = EndpointId::new(2).expect("first");
        let disabled = EndpointId::new(3).expect("disabled");
        let router = Router::new(
            vec![
                Route {
                    source,
                    destination: disabled,
                    destination_parameter: None,
                    channel: None,
                    class: Some(MessageClass::ControlChange),
                    enabled: false,
                    priority: 0,
                    curve: Curve::Linear,
                    predicates: Vec::new(),
                    allow_cycle: false,
                },
                Route {
                    source,
                    destination: first,
                    destination_parameter: None,
                    channel: None,
                    class: Some(MessageClass::ControlChange),
                    enabled: true,
                    priority: 7,
                    curve: Curve::Square,
                    predicates: Vec::new(),
                    allow_cycle: false,
                },
            ],
            1,
            4,
        )
        .expect("router");
        let event = MidiEvent {
            timestamp: TimestampNanos::new(1),
            sequence: 1,
            endpoint: source,
            message: MidiMessage::ControlChange {
                channel: MidiChannel::new(1).expect("channel"),
                controller: SevenBit::new(10).expect("controller"),
                value: SevenBit::new(64).expect("value"),
            },
        };
        let outputs = router.route(&event);
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].event.endpoint, first);
        assert_eq!(
            outputs[0].event.message,
            MidiMessage::ControlChange {
                channel: MidiChannel::new(1).expect("channel"),
                controller: SevenBit::new(10).expect("controller"),
                value: SevenBit::new(apply_curve(64, Curve::Square).into()).expect("curve"),
            }
        );
    }

    #[test]
    fn router_applies_number_value_realtime_and_masked_sysex_predicates() {
        let source = EndpointId::new(1).expect("source");
        let destination = EndpointId::new(2).expect("destination");
        let event = |message| MidiEvent {
            timestamp: TimestampNanos::new(1),
            sequence: 1,
            endpoint: source,
            message,
        };
        let route = |predicates| Route {
            source,
            destination,
            destination_parameter: None,
            channel: None,
            class: None,
            enabled: true,
            priority: 0,
            curve: Curve::Linear,
            predicates,
            allow_cycle: false,
        };
        let cc = event(MidiMessage::ControlChange {
            channel: MidiChannel::new(1).expect("channel"),
            controller: SevenBit::new(10).expect("controller"),
            value: SevenBit::new(64).expect("value"),
        });
        let router = Router::new(
            vec![route(vec![
                RoutePredicate::NumberRange { minimum: 10, maximum: 12 },
                RoutePredicate::ValueRange { minimum: 60, maximum: 70 },
            ])],
            1,
            4,
        )
        .expect("CC router");
        assert_eq!(router.route(&cc).len(), 1);

        let realtime =
            Router::new(vec![route(vec![RoutePredicate::Realtime(RealtimeMessage::Clock)])], 1, 4)
                .expect("real-time router");
        assert_eq!(realtime.route(&event(MidiMessage::Realtime(RealtimeMessage::Clock))).len(), 1);
        assert!(realtime.route(&event(MidiMessage::Realtime(RealtimeMessage::Stop))).is_empty());

        let sysex = Router::new(
            vec![route(vec![RoutePredicate::SysExMask {
                pattern: vec![0x06, 0x00, 0x10],
                mask: vec![0x7f, 0x7f, 0x70],
            }])],
            1,
            4,
        )
        .expect("SysEx router");
        assert_eq!(
            sysex.route(&event(MidiMessage::sysex([0x06, 0x00, 0x1f]).expect("SysEx"))).len(),
            1
        );
        assert!(sysex
            .route(&event(MidiMessage::sysex([0x06, 0x01, 0x1f]).expect("SysEx")))
            .is_empty());
        assert!(Router::new(
            vec![route(vec![RoutePredicate::NumberRange { minimum: 12, maximum: 10 }])],
            1,
            4,
        )
        .is_err());
    }

    #[test]
    fn router_rejects_accidental_cycles_and_bounds_explicit_cycles() {
        let one = EndpointId::new(1).expect("one");
        let two = EndpointId::new(2).expect("two");
        let edge = |source, destination, allow_cycle| Route {
            source,
            destination,
            destination_parameter: None,
            channel: None,
            class: None,
            enabled: true,
            priority: 0,
            curve: Curve::Linear,
            predicates: Vec::new(),
            allow_cycle,
        };
        assert!(Router::new(vec![edge(one, two, false), edge(two, one, false)], 1, 4).is_err());
        let router = Router::new(vec![edge(one, two, true), edge(two, one, true)], 1, 2)
            .expect("explicit bounded cycle");
        let event = MidiEvent {
            timestamp: TimestampNanos::new(1),
            sequence: 1,
            endpoint: one,
            message: MidiMessage::Realtime(RealtimeMessage::Clock),
        };
        let first = router.route(&event);
        assert_eq!(first.len(), 1);
        assert_eq!(router.route_with_hops(&first[0].event, first[0].hops).len(), 1);
        assert!(router.route_with_hops(&event, 2).is_empty());
    }

    #[test]
    fn dispatch_routes_to_matching_output_and_counts_unmatched() {
        let source = mackes_domain::EndpointId::new(1).expect("source");
        let destination = mackes_domain::EndpointId::new(2).expect("destination");
        let router = RouterStore::new(
            vec![Route {
                source,
                destination,
                destination_parameter: None,
                channel: None,
                class: None,
                enabled: true,
                priority: 0,
                curve: Curve::Linear,
                predicates: Vec::new(),
                allow_cycle: false,
            }],
            1,
            2,
        )
        .expect("router");
        let event = MidiEvent {
            timestamp: TimestampNanos::new(1),
            sequence: 9,
            endpoint: source,
            message: MidiMessage::NoteOn {
                channel: MidiChannel::new(1).expect("channel"),
                note: mackes_domain::SevenBit::new(60).expect("note"),
                velocity: mackes_domain::SevenBit::new(100).expect("velocity"),
            },
        };
        let mut output = VirtualEndpoint::new("2", "destination", EndpointDirection::Output);
        let mut outputs: Vec<&mut dyn MidiOutputAdapter> = vec![&mut output];
        assert_eq!(dispatch_routed_event(&router, &event, &mut outputs), (1, 0));
        assert_eq!(output.stats().sent, 1);
        let mut none: Vec<&mut dyn MidiOutputAdapter> = Vec::new();
        assert_eq!(dispatch_routed_event(&router, &event, &mut none), (0, 1));
    }

    #[test]
    fn output_registry_rejects_duplicates_and_capacity_overflow() {
        let mut registry = OutputRegistry::new(1);
        registry
            .insert(Box::new(VirtualEndpoint::new("1", "one", EndpointDirection::Output)))
            .expect("first output");
        assert_eq!(registry.len(), 1);
        assert!(registry
            .insert(Box::new(VirtualEndpoint::new("1", "duplicate", EndpointDirection::Output)))
            .is_err());
        assert!(registry
            .insert(Box::new(VirtualEndpoint::new("2", "full", EndpointDirection::Output)))
            .is_err());
        assert!(registry.remove("1"));
        assert!(!registry.remove("missing"));
        assert!(registry.is_empty());
    }

    #[test]
    fn direct_output_send_is_named_bounded_and_validated() {
        let mut registry = OutputRegistry::new(1);
        registry
            .insert(Box::new(VirtualEndpoint::new("out", "output", EndpointDirection::Output)))
            .expect("output");
        registry.send_direct("out", &[0xF0, 0x7D, 0x01, 0xF7]).expect("SysEx send");
        assert_eq!(
            registry.send_direct("missing", &[0xF0, 0x7D, 0xF7]),
            Err("destination output is not registered".into())
        );
        assert!(registry.send_direct("out", &[0xF0, 0x80, 0xF7]).is_err());
    }

    #[test]
    fn cc_mapping_remaps_controller_and_channel() {
        let event = MidiEvent {
            timestamp: TimestampNanos::new(1),
            sequence: 1,
            endpoint: mackes_domain::EndpointId::new(1).expect("endpoint"),
            message: MidiMessage::ControlChange {
                channel: MidiChannel::new(1).expect("channel"),
                controller: mackes_domain::SevenBit::new(10).expect("cc"),
                value: mackes_domain::SevenBit::new(64).expect("value"),
            },
        };
        let mapping = CcMapping {
            source_controller: mackes_domain::SevenBit::new(10).expect("cc"),
            destination_controller: mackes_domain::SevenBit::new(20).expect("cc"),
            destination_channel: Some(MidiChannel::new(2).expect("channel")),
        };
        assert!(
            matches!(mapping.apply(&event).expect("mapped").message, MidiMessage::ControlChange { channel, controller, .. } if channel == MidiChannel::new(2).expect("channel") && controller == mackes_domain::SevenBit::new(20).expect("cc"))
        );
        let candidates = infer_cc_candidates(&[event.clone(), event]);
        assert_eq!(
            candidates,
            vec![LearnCandidate {
                controller: mackes_domain::SevenBit::new(10).expect("cc"),
                observations: 2
            }]
        );
        assert_eq!(candidates[0].confidence_milli(4), 500);
        assert_eq!(candidates[0].confidence_milli(0), 0);
        assert_eq!(best_cc_candidate(&candidates), Some(candidates[0]));
    }

    #[test]
    fn typed_number_mapping_preserves_family_and_rejects_conversion() {
        let event = MidiEvent {
            timestamp: TimestampNanos::new(1),
            sequence: 1,
            endpoint: EndpointId::new(1).expect("endpoint"),
            message: MidiMessage::NoteOn {
                channel: MidiChannel::new(1).expect("channel"),
                note: SevenBit::new(60).expect("note"),
                velocity: SevenBit::new(100).expect("velocity"),
            },
        };
        let mapping = TypedNumberMapping {
            source_kind: TypedMappingKind::Note,
            source_number: SevenBit::new(60).expect("source"),
            destination_kind: TypedMappingKind::Note,
            destination_number: SevenBit::new(64).expect("destination"),
            destination_channel: Some(MidiChannel::new(2).expect("channel")),
        };
        assert!(matches!(mapping.apply(&event).expect("mapped").message,
            MidiMessage::NoteOn { channel, note, velocity }
                if channel == MidiChannel::new(2).expect("channel")
                    && note == SevenBit::new(64).expect("note")
                    && velocity == SevenBit::new(100).expect("velocity")));
        let invalid =
            TypedNumberMapping { destination_kind: TypedMappingKind::ControlChange, ..mapping };
        assert!(invalid.apply(&event).is_none());
    }

    #[test]
    fn conditional_typed_mapping_is_bounded_and_deterministic() {
        let event = MidiEvent {
            timestamp: TimestampNanos::new(1),
            sequence: 1,
            endpoint: EndpointId::new(1).expect("endpoint"),
            message: MidiMessage::ControlChange {
                channel: MidiChannel::new(1).expect("channel"),
                controller: SevenBit::new(10).expect("controller"),
                value: SevenBit::new(64).expect("value"),
            },
        };
        let mapping = ConditionalTypedMapping {
            mapping: TypedNumberMapping {
                source_kind: TypedMappingKind::ControlChange,
                source_number: SevenBit::new(10).expect("source"),
                destination_kind: TypedMappingKind::ControlChange,
                destination_number: SevenBit::new(20).expect("destination"),
                destination_channel: None,
            },
            minimum: 64,
            maximum: 127,
        };
        assert!(mapping.apply(&event).is_some());
        let below = MidiEvent {
            message: MidiMessage::ControlChange {
                channel: MidiChannel::new(1).expect("channel"),
                controller: SevenBit::new(10).expect("controller"),
                value: SevenBit::new(63).expect("value"),
            },
            ..event
        };
        assert!(mapping.apply(&below).is_none());
    }

    #[test]
    fn generalized_learn_groups_every_midi_family_with_raw_evidence() {
        let endpoint = EndpointId::new(1).expect("endpoint");
        let channel = MidiChannel::new(2).expect("channel");
        let event = |sequence, message| MidiEvent {
            timestamp: TimestampNanos::new(sequence),
            sequence,
            endpoint,
            message,
        };
        let messages = vec![
            event(
                1,
                MidiMessage::NoteOn {
                    channel,
                    note: SevenBit::new(60).expect("note"),
                    velocity: SevenBit::new(10).expect("velocity"),
                },
            ),
            event(
                2,
                MidiMessage::NoteOff {
                    channel,
                    note: SevenBit::new(60).expect("note"),
                    velocity: SevenBit::new(0).expect("velocity"),
                },
            ),
            event(
                3,
                MidiMessage::ControlChange {
                    channel,
                    controller: SevenBit::new(7).expect("CC"),
                    value: SevenBit::new(10).expect("value"),
                },
            ),
            event(
                4,
                MidiMessage::ControlChange {
                    channel,
                    controller: SevenBit::new(7).expect("CC"),
                    value: SevenBit::new(100).expect("value"),
                },
            ),
            event(
                5,
                MidiMessage::ProgramChange { channel, program: SevenBit::new(4).expect("program") },
            ),
            event(
                6,
                MidiMessage::ChannelPressure {
                    channel,
                    pressure: SevenBit::new(50).expect("pressure"),
                },
            ),
            event(
                7,
                MidiMessage::PitchBend { channel, value: FourteenBit::new(8192).expect("bend") },
            ),
            event(
                8,
                MidiMessage::SystemCommon(SystemCommonMessage::SongSelect(
                    SevenBit::new(3).expect("song"),
                )),
            ),
            event(9, MidiMessage::Realtime(RealtimeMessage::Clock)),
            event(10, MidiMessage::sysex([0x06, 0x00, 0x10]).expect("SysEx")),
        ];
        let candidates = infer_midi_candidates(&messages);
        assert_eq!(candidates.len(), 9);
        let cc = candidates
            .iter()
            .find(|candidate| candidate.kind == LearnMessageKind::ControlChange)
            .expect("CC candidate");
        assert_eq!((cc.channel, cc.number, cc.observations), (Some(2), Some(7), 2));
        assert_eq!((cc.minimum, cc.maximum), (Some(10), Some(100)));
        let sysex = candidates
            .iter()
            .find(|candidate| candidate.kind == LearnMessageKind::SysEx)
            .expect("SysEx candidate");
        assert_eq!(sysex.raw, vec![0xF0, 0x06, 0x00, 0x10, 0xF7]);
        assert!(candidates.iter().all(|candidate| !candidate.raw.is_empty()));
    }

    #[test]
    fn scaling_is_bounded_and_invertible_at_endpoints() {
        assert_eq!(scale_value(0, (0, 127), (0, 100), false), Ok(0));
        assert_eq!(scale_value(127, (0, 127), (0, 100), false), Ok(100));
        assert_eq!(scale_value(0, (0, 127), (0, 100), true), Ok(100));
        assert!(scale_value(128, (0, 127), (0, 100), false).is_err());
    }

    #[test]
    fn pickup_accepts_only_within_tolerance() {
        assert!(pickup_accept(64, 60, 4));
        assert!(!pickup_accept(65, 60, 4));
        let mut state = PickupState::new(60, 2);
        assert!(!state.accept(64));
        assert!(state.accept(62));
        assert!(state.accept(100));
        state.reset(100);
        assert!(!state.accept(60));
    }

    #[test]
    fn takeover_modes_prevent_scene_jumps_and_bound_relative_values() {
        let mut jump = TakeoverState::new(TakeoverMode::Jump, 100).expect("jump");
        assert_eq!(jump.apply(10), Ok(Some(10)));

        let mut pickup =
            TakeoverState::new(TakeoverMode::Pickup { tolerance: 2 }, 60).expect("pickup");
        assert_eq!(pickup.apply(20), Ok(None));
        assert_eq!(pickup.apply(58), Ok(Some(58)));
        assert_eq!(pickup.apply(100), Ok(Some(100)));
        pickup.reset(40).expect("new scene");
        assert_eq!(pickup.apply(100), Ok(None));

        let mut scaled = TakeoverState::new(
            TakeoverMode::ScaledPickup { input: (0, 127), output: (0, 1000), tolerance: 8 },
            500,
        )
        .expect("scaled pickup");
        assert_eq!(scaled.apply(10), Ok(None));
        assert_eq!(scaled.apply(64), Ok(Some(503)));

        let mut relative =
            TakeoverState::new(TakeoverMode::Relative { step: 2, range: (0, 100) }, 50)
                .expect("relative");
        assert_eq!(relative.apply(3), Ok(Some(56)));
        assert_eq!(relative.apply(126), Ok(Some(52)));
        assert_eq!(relative.apply(64), Ok(Some(52)));
        assert_eq!(relative.apply(63), Ok(Some(100)));
        assert_eq!(relative.apply(127), Ok(Some(98)));
        assert_eq!(relative.target(), 98);
        assert!(relative.apply(128).is_err());
    }

    #[test]
    fn mapping_state_store_is_page_isolated_and_edge_triggered() {
        let mut store = MappingStateStore::default();
        store
            .register(
                "effect",
                "practice",
                StatefulMode::Toggle { on_value: 127 },
                MappingReset::Off,
                0,
            )
            .expect("toggle");
        store
            .register(
                "effect",
                "device",
                StatefulMode::Latch { on_value: 100 },
                MappingReset::Preserve,
                0,
            )
            .expect("page-isolated latch");
        assert_eq!(store.apply("effect", "practice", 127).expect("press")[0].value, 127);
        assert!(store.apply("effect", "practice", 127).expect("held").is_empty());
        assert!(store.apply("effect", "practice", 0).expect("release").is_empty());
        assert_eq!(store.apply("effect", "practice", 127).expect("second press")[0].value, 0);
        assert_eq!(store.apply("effect", "device", 127).expect("latch press")[0].value, 100);
        assert_eq!(store.apply("effect", "device", 0).expect("latch release")[0].value, 0);
    }

    #[test]
    fn radio_step_and_scene_reset_are_deterministic() {
        let mut store = MappingStateStore::default();
        for id in ["scene-a", "scene-b"] {
            store
                .register(
                    id,
                    "practice",
                    StatefulMode::Radio { group: "scenes".into(), on_value: 127 },
                    MappingReset::Off,
                    0,
                )
                .expect("radio");
        }
        store
            .register(
                "division",
                "practice",
                StatefulMode::Step { values: vec![0, 32, 64, 127] },
                MappingReset::SceneDefault,
                32,
            )
            .expect("step");
        assert_eq!(store.apply("scene-a", "practice", 127).expect("radio a").len(), 1);
        store.apply("scene-a", "practice", 0).expect("release a");
        let changes = store.apply("scene-b", "practice", 127).expect("radio b");
        assert_eq!(
            changes.iter().map(|change| (&*change.mapping_id, change.value)).collect::<Vec<_>>(),
            vec![("scene-a", 0), ("scene-b", 127)]
        );
        assert_eq!(store.apply("division", "practice", 127).expect("step")[0].value, 64);
        assert_eq!(
            store
                .reset_scene("practice")
                .iter()
                .map(|change| (&*change.mapping_id, change.value))
                .collect::<Vec<_>>(),
            vec![("division", 32), ("scene-b", 0)]
        );
        assert_eq!(store.value("division", "practice"), Some(32));
    }

    #[test]
    fn curves_preserve_midi_endpoints() {
        for curve in [Curve::Linear, Curve::Square, Curve::SquareRoot] {
            assert_eq!(apply_curve(0, curve), 0);
            assert_eq!(apply_curve(127, curve), 127);
        }
        assert!(apply_curve(64, Curve::Square) < 64);
    }

    #[test]
    fn parameter_mapping_requires_exact_source_and_scales_cc_values() {
        let source = EndpointId::new(1).expect("source");
        let destination = EndpointId::new(2).expect("destination");
        let mapping = ParameterMapping {
            source_endpoint: source,
            destination_endpoint: destination,
            class: MessageClass::ControlChange,
            number: 21,
            channel: Some(0),
            source_range: (0, 127),
            destination_range: (10, 100),
            invert: false,
            curve: Curve::Linear,
        };
        let event = MidiEvent {
            timestamp: TimestampNanos::new(1),
            sequence: 2,
            endpoint: source,
            message: MidiMessage::ControlChange {
                channel: MidiChannel::new(1).expect("channel"),
                controller: SevenBit::new(21).expect("controller"),
                value: SevenBit::new(64).expect("value"),
            },
        };
        let mapped = mapping.evaluate(&event).expect("exact match");
        assert_eq!(mapped.endpoint, destination);
        assert!(
            matches!(mapped.message, MidiMessage::ControlChange { value, .. } if value.get() == 55)
        );
        assert!(mapping.evaluate(&MidiEvent { endpoint: source, ..event.clone() }).is_some());
        assert!(mapping.evaluate(&MidiEvent { endpoint: destination, ..event.clone() }).is_none());
        let wrong_channel = MidiEvent {
            message: MidiMessage::ControlChange {
                channel: MidiChannel::new(2).expect("channel"),
                controller: SevenBit::new(21).expect("controller"),
                value: SevenBit::new(64).expect("value"),
            },
            ..event
        };
        assert!(mapping.evaluate(&wrong_channel).is_none());
        let wide = ParameterMapping { destination_range: (100, 1000), ..mapping };
        assert_eq!(wide.evaluate_with_value(&event).expect("wide value").1, 553);
    }

    #[test]
    fn parameter_mapping_handles_exact_program_change_buttons() {
        let mapping = ParameterMapping {
            source_endpoint: EndpointId::new(1).expect("source"),
            destination_endpoint: EndpointId::new(2).expect("destination"),
            class: MessageClass::ProgramChange,
            number: 7,
            channel: Some(0),
            source_range: (0, 127),
            destination_range: (1, 1),
            invert: false,
            curve: Curve::Linear,
        };
        let event = MidiEvent {
            timestamp: TimestampNanos::new(1),
            sequence: 1,
            endpoint: EndpointId::new(1).expect("source"),
            message: MidiMessage::ProgramChange {
                channel: MidiChannel::new(1).expect("channel"),
                program: SevenBit::new(7).expect("program"),
            },
        };
        let mapped = mapping.evaluate(&event).expect("button match");
        assert_eq!(mapped.endpoint, EndpointId::new(2).expect("destination"));
        assert!(
            matches!(mapped.message, MidiMessage::ProgramChange { program, .. } if program.get() == 1)
        );
        let near_miss = MidiEvent {
            message: MidiMessage::ProgramChange {
                channel: MidiChannel::new(1).expect("channel"),
                program: SevenBit::new(8).expect("program"),
            },
            ..event
        };
        assert!(mapping.evaluate(&near_miss).is_none());
    }

    #[test]
    fn scheduler_orders_by_due_time_then_insertion_and_cancels() {
        let event = |sequence| MidiEvent {
            timestamp: TimestampNanos::new(1),
            sequence,
            endpoint: mackes_domain::EndpointId::new(1).expect("endpoint"),
            message: MidiMessage::Realtime(mackes_domain::RealtimeMessage::Clock),
        };
        let mut scheduler = Scheduler::default();
        scheduler.schedule(0, 20, event(1));
        scheduler.schedule(0, 10, event(2));
        scheduler.schedule(0, 10, event(3));
        scheduler.cancel_sequence(3);
        assert_eq!(
            scheduler.drain_due(10).iter().map(|item| item.sequence).collect::<Vec<_>>(),
            vec![2]
        );
        assert_eq!(
            scheduler.drain_due(20).iter().map(|item| item.sequence).collect::<Vec<_>>(),
            vec![1]
        );
    }

    #[test]
    fn scheduler_cancel_all_clears_pending_chain() {
        let endpoint = mackes_domain::EndpointId::new(1).expect("endpoint");
        let event = |sequence| MidiEvent {
            timestamp: TimestampNanos::new(1),
            sequence,
            endpoint,
            message: MidiMessage::Realtime(mackes_domain::RealtimeMessage::Clock),
        };
        let mut scheduler = Scheduler::default();
        scheduler.schedule(0, 1, event(1));
        scheduler.schedule(0, 2, event(2));
        assert_eq!(scheduler.cancel_all(), 2);
        assert!(scheduler.drain_due(10).is_empty());
        assert_eq!(scheduler.cancel_all(), 0);
    }

    #[test]
    fn retry_policy_is_capped_and_deterministic() {
        let policy = RetryPolicy { min_interval_ns: 100, retries: 3, retry_delay_ns: 50 };
        assert_eq!(policy.delay_for(0), 50);
        assert_eq!(policy.delay_for(3), 400);
        assert_eq!(policy.delay_for(255), 3_200);
    }

    #[test]
    fn pending_transaction_enforces_timeout_pacing_and_retry_bound() {
        let policy = RetryPolicy { min_interval_ns: 10, retries: 1, retry_delay_ns: 5 };
        let mut pending = PendingTransaction::start(100, 50, policy).expect("transaction");
        assert!(!pending.timed_out(149));
        assert!(!pending.retry(105, 50, policy));
        assert!(pending.retry(110, 50, policy));
        assert_eq!(pending.attempts_sent, 2);
        assert!(!pending.retry(120, 50, policy));
        let matcher = ResponseMatcher::new(vec![0xf0, 1], vec![0xff, 0xff]).expect("matcher");
        assert!(pending.complete_if_matches(115, &matcher, &[0xf0, 1]));
        assert!(pending.completed);
        assert!(!pending.accepts_response(115, &matcher, &[0xf0, 1]));
        assert!(!pending.accepts_response(200, &matcher, &[0xf0, 1]));
        assert!(!pending.retry(130, 50, policy));
        assert!(PendingTransaction::start(0, 0, policy).is_none());
    }

    #[test]
    fn response_matcher_is_exact_and_masked() {
        let matcher =
            ResponseMatcher::new(vec![0xf0, 0x01, 0x10], vec![0xff, 0xff, 0xf0]).expect("matcher");
        assert!(matcher.matches(&[0xf0, 0x01, 0x1f]));
        assert!(!matcher.matches(&[0xf0, 0x02, 0x1f]));
        assert!(!matcher.matches(&[0xf0, 0x01]));
        assert!(ResponseMatcher::new(Vec::new(), Vec::new()).is_err());
        assert!(ResponseMatcher::new(
            vec![0; ResponseMatcher::MAX_BYTES + 1],
            vec![0; ResponseMatcher::MAX_BYTES + 1]
        )
        .is_err());
    }

    #[test]
    fn capture_correlation_retains_unsolicited_and_decodes_diffs() {
        let endpoint = EndpointId::new(1).expect("endpoint");
        let other = EndpointId::new(2).expect("other");
        let mut store = CaptureStore::new(3).expect("store");
        for record in [
            CaptureRecord {
                endpoint: other,
                timestamp_ns: 10,
                bytes: vec![1, 2, 3],
                matched: false,
            },
            CaptureRecord { endpoint, timestamp_ns: 11, bytes: vec![1, 2, 4], matched: false },
            CaptureRecord { endpoint, timestamp_ns: 12, bytes: vec![1, 2, 3], matched: false },
        ] {
            store.push(record).expect("capture");
        }
        let matcher = ResponseMatcher::new(vec![1, 2, 3], vec![0x7f; 3]).expect("matcher");
        assert_eq!(
            store.correlate(endpoint, 10, 12, &matcher).map(|record| record.timestamp_ns),
            Some(12)
        );
        assert_eq!(store.unmatched().count(), 2);
        store
            .push(CaptureRecord { endpoint, timestamp_ns: 13, bytes: vec![7], matched: false })
            .expect("evict oldest");
        assert_eq!(store.records().front().map(|record| record.timestamp_ns), Some(11));
        assert!(store.correlate(endpoint, 20, 10, &matcher).is_none());

        let decoded = decode_capture_fields(
            &[0x01, 0x02, 0x03],
            &[
                CaptureField { name: "manufacturer".into(), offset: 0, length: 1 },
                CaptureField { name: "value".into(), offset: 1, length: 2 },
            ],
        )
        .expect("decode fields");
        assert_eq!(decoded, vec![("manufacturer".into(), 1), ("value".into(), 259)]);
        assert_eq!(diff_capture_bytes(&[1, 2, 3], &[1, 4]), vec![1, 2]);
        assert!(decode_capture_fields(
            &[0x80],
            &[CaptureField { name: "bad".into(), offset: 0, length: 1 }]
        )
        .is_err());
    }

    #[test]
    fn device_request_validates_and_starts_transaction() {
        let matcher = ResponseMatcher::new(vec![0xf0, 1], vec![0xff, 0xff]).expect("matcher");
        let request = DeviceRequest::new(
            vec![0xf0, 1, 0xf7],
            matcher,
            RetryPolicy { min_interval_ns: 1, retries: 2, retry_delay_ns: 1 },
            10,
        )
        .expect("request");
        assert_eq!(request.begin(5).expect("pending").deadline_ns, 15);
        let event = request.to_event(3, 20, 9).expect("event");
        assert_eq!(event.endpoint.get(), 3);
        assert_eq!(event.message.wire_bytes(), vec![0xf0, 1, 0xf7]);
        assert!(
            DeviceRequest::new(Vec::new(), request.response.clone(), request.policy, 1).is_err()
        );
    }

    #[test]
    fn endpoint_ids_are_stable_and_direction_scoped() {
        let input = stable_endpoint_id("Launch Control XL", EndpointDirection::Input);
        assert_eq!(input, stable_endpoint_id("Launch Control XL", EndpointDirection::Input));
        assert_ne!(input, stable_endpoint_id("Launch Control XL", EndpointDirection::Output));
        assert_ne!(input, stable_endpoint_id("Launch Control XL HUI", EndpointDirection::Input));
        assert!(input.starts_with("midir-in-"));
    }

    #[test]
    fn native_alsa_address_is_runtime_only_and_lifecycle_is_typed() {
        let address = AlsaSequencerAddress::new(24, 0);
        assert_eq!(address, AlsaSequencerAddress { client: 24, port: 0 });
        assert_ne!(AlsaSequencerLifecycle::Started, AlsaSequencerLifecycle::Exited);
        assert_eq!(AlsaSequencerLifecycle::Subscribed, AlsaSequencerLifecycle::Subscribed);
    }

    #[test]
    fn virtual_port_names_are_stable_product_contracts() {
        assert_eq!(VIRTUAL_INPUT_NAME, "MACKES DAW In");
        assert_eq!(VIRTUAL_OUTPUT_NAME, "MACKES DAW Out");
    }

    #[test]
    fn apple_midi_control_parser_rejects_truncation_and_unknown_commands() {
        let mut invitation = vec![0xff, 0xff, b'I', b'N'];
        invitation.resize(16, 0);
        assert_eq!(parse_apple_midi_command(&invitation), Ok(AppleMidiCommand::Invitation));
        assert_eq!(parse_apple_midi_command(&invitation[..8]), Err("truncated AppleMIDI command"));
        assert_eq!(
            parse_apple_midi_command(&[0xff, 0xff, b'X', b'X', 0, 0]),
            Err("unknown AppleMIDI command")
        );
        assert_eq!(
            parse_apple_midi_command(&[0x80, 0x60, 0x00, 0x01]),
            Err("not an AppleMIDI control packet")
        );
    }

    #[test]
    fn apple_midi_session_requires_invitation_and_matching_identity() {
        let mut session = AppleMidiSession::new(42);
        assert!(!session.accepts(42, 7));
        assert!(session.establish(42, 7).is_err());
        session.invite("peer");
        assert!(session.establish(41, 7).is_err());
        assert!(session.establish(42, 0).is_err());
        session.establish(42, 7).expect("establish");
        assert!(session.accepts(42, 7));
        assert!(!session.accepts(42, 8));
        session.invite("replacement");
        assert_eq!(session.remote_name.as_deref(), Some("peer"));
        assert!(session.end_session(42, 8).is_err());
        session.end_session(42, 7).expect("end");
        session.invite("");
        assert_eq!(session.state, SessionState::Disconnected);
        session.disconnect();
        assert_eq!(session.state, SessionState::Disconnected);
    }

    #[test]
    fn rtp_peer_binds_identity_to_sequence_and_resets_on_reconnect() {
        let mut peer = RtpMidiPeer::new(9, 4).expect("peer");
        assert_eq!(peer.state(), SessionState::Disconnected);
        peer.invite("peer");
        assert_eq!(peer.remote_name(), Some("peer"));
        peer.establish(9, 17).expect("establish");
        assert_eq!(peer.state(), SessionState::Established);
        assert_eq!(peer.remote_ssrc(), Some(17));
        assert_eq!(peer.observe(8, 17, 1), None);
        assert_eq!(peer.observe(9, 17, 1), Some(SequenceDisposition::InOrder));
        peer.disconnect();
        assert_eq!(peer.state(), SessionState::Disconnected);
        assert_eq!(peer.remote_ssrc(), None);
        peer.invite("peer");
        peer.establish(9, 17).expect("reconnect");
        assert_eq!(peer.observe(9, 17, 500), Some(SequenceDisposition::InOrder));
    }

    #[test]
    fn rtp_peer_packet_ingest_enforces_allowlist_and_framing() {
        let mut peer = RtpMidiPeer::new(9, 4).expect("peer");
        peer.invite("peer");
        peer.establish(9, 17).expect("establish");
        let packet = build_rtp_packet(1, 2, 17, &[0, 1, 0x90]).expect("packet");
        let allowed = ["127.0.0.1:9000".parse().expect("address")];
        let denied = "127.0.0.1:9001".parse().expect("address");
        assert!(peer.receive_packet(&packet, denied, &allowed, 9, 17).is_err());
        let (parsed, disposition) =
            peer.receive_packet(&packet, allowed[0], &allowed, 9, 17).expect("ingest");
        assert_eq!(parsed.rtp.sequence, 1);
        assert_eq!(disposition, SequenceDisposition::InOrder);
    }

    #[test]
    fn udp_transport_binds_nonblocking_and_reports_empty_queue() {
        let transport =
            UdpMidiTransport::bind("127.0.0.1:0".parse().expect("address"), 128).expect("bind");
        assert!(transport.local_addr().expect("local").port() > 0);
        assert!(transport.receive().expect("receive").is_none());
        assert!(UdpMidiTransport::bind("127.0.0.1:0".parse().expect("address"), 0).is_err());
        assert!(transport.send_to(&[0; 129], "127.0.0.1:9".parse().expect("peer")).is_err());
        let peer = "127.0.0.1:9".parse().expect("peer");
        assert_eq!(
            transport.send_to_allowed(&[1], peer, &[]).expect_err("deny").kind(),
            std::io::ErrorKind::PermissionDenied
        );
        let receiver = std::net::UdpSocket::bind("127.0.0.1:0").expect("receiver");
        receiver.set_read_timeout(Some(std::time::Duration::from_secs(1))).expect("timeout");
        let target = receiver.local_addr().expect("target");
        assert_eq!(transport.send_to(&[9, 8, 7], target).expect("send"), 3);
        let mut bytes = [0; 3];
        receiver.recv_from(&mut bytes).expect("datagram");
        assert_eq!(bytes, [9, 8, 7]);
        let sender = std::net::UdpSocket::bind("127.0.0.1:0").expect("sender");
        let sender_addr = sender.local_addr().expect("sender address");
        sender
            .send_to(&[4, 5], transport.local_addr().expect("transport address"))
            .expect("send inbound");
        assert!(transport.receive_from_allowed(&[]).expect("denied inbound").is_none());
        sender
            .send_to(&[4, 5], transport.local_addr().expect("transport address"))
            .expect("send inbound");
        assert_eq!(
            transport
                .receive_from_allowed(&[sender_addr])
                .expect("allowed inbound")
                .expect("packet")
                .0,
            vec![4, 5]
        );
        let mut peer_session = RtpMidiPeer::new(3, 4).expect("peer");
        peer_session.invite("loopback");
        peer_session.establish(3, 22).expect("establish");
        let packet = build_rtp_packet(8, 9, 22, &[0, 1, 0x90]).expect("packet");
        let local = transport.local_addr().expect("local");
        transport.send_to(&packet, local).expect("packet send");
        let allowlist = PeerAllowlist::new(vec![local]).expect("allowlist");
        let peer_packet = peer_session
            .receive_from_transport(&transport, &allowlist, 3, 22)
            .expect("peer receive")
            .expect("packet available");
        assert_eq!(peer_packet.1, SequenceDisposition::InOrder);
    }

    #[test]
    fn rtp_header_parser_handles_csrc_extension_and_padding() {
        let packet =
            [0xB1, 0x61, 0x12, 0x34, 0, 0, 0, 9, 0, 0, 0, 7, 1, 2, 3, 4, 0, 1, 0, 0, 0x55, 0, 2];
        let parsed = parse_rtp_header(&packet).expect("RTP header");
        assert_eq!((parsed.sequence, parsed.timestamp, parsed.ssrc), (0x1234, 9, 7));
        assert_eq!(parsed.payload, &[0x55]);
        assert_eq!(parse_rtp_header(&[0; 11]), Err("invalid RTP header"));
    }

    #[test]
    fn rtp_packet_builder_round_trips_header_parser() {
        let packet = build_rtp_packet(7, 99, 123, &[0x80, 0x01, 0x90]).expect("packet");
        let parsed = parse_rtp_header(&packet).expect("header");
        assert_eq!((parsed.sequence, parsed.timestamp, parsed.ssrc), (7, 99, 123));
        assert_eq!(parsed.payload, &[0x80, 0x01, 0x90]);
        assert_eq!(build_rtp_packet(0, 0, 0, &[]), Err("RTP payload must not be empty"));
        let combined = build_rtp_packet(8, 10, 12, &[0x80, 0x03, 0x90, 60, 127]).expect("packet");
        let parsed = parse_rtp_midi_packet(&combined).expect("combined");
        assert_eq!(parsed.rtp.sequence, 8);
        assert_eq!(parsed.midi.commands, &[0x90, 60, 127]);
        let peer = "127.0.0.1:5000".parse().expect("peer");
        assert!(validate_inbound_rtp_midi(&combined, peer, &[]).is_err());
        assert!(validate_inbound_rtp_midi(&combined, peer, &[peer]).is_ok());
    }

    #[test]
    fn rtp_midi_payload_parser_enforces_command_length() {
        let payload = [0x80, 0x03, 0x90, 60, 127];
        let parsed = parse_rtp_midi_payload(&payload).expect("RTP-MIDI payload");
        assert!(parsed.begin);
        assert!(!parsed.dropped);
        assert_eq!(parsed.commands, &[0x90, 60, 127]);
        assert_eq!(
            parse_rtp_midi_payload(&[0x80, 0x04, 0x90, 60, 127]),
            Err("RTP-MIDI command length mismatch")
        );
        assert_eq!(parse_rtp_midi_payload(&[0x80]), Err("RTP-MIDI payload is truncated"));
    }

    #[test]
    fn rtp_midi_channel_voice_decoder_supports_running_status() {
        let commands =
            decode_rtp_midi_channel_voice(&[0x90, 60, 127, 61, 0, 0xC0, 7]).expect("commands");
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[1].status, 0x90);
        assert_eq!(commands[1].data, [61, 0]);
        assert_eq!(commands[2].data_len, 1);
        assert!(decode_rtp_midi_channel_voice(&[60, 1]).is_err());
        assert!(decode_rtp_midi_channel_voice(&[0xF8]).is_err());
    }

    #[test]
    fn rtp_channel_command_converts_to_domain_message() {
        let message =
            rtp_command_to_message(RtpMidiCommand { status: 0x91, data: [60, 100], data_len: 2 })
                .expect("note-on");
        assert_eq!(message.wire_bytes(), vec![0x91, 60, 100]);
        assert!(rtp_command_to_message(RtpMidiCommand { status: 0xc0, data: [7, 0], data_len: 2 })
            .is_err());
    }

    #[test]
    fn rtp_channel_batch_decoder_preserves_order() {
        let messages = decode_rtp_channel_messages(&[0x90, 60, 100, 61, 0]).expect("batch");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].wire_bytes(), vec![0x90, 60, 100]);
        assert_eq!(messages[1].wire_bytes(), vec![0x90, 61, 0]);
    }

    #[test]
    fn rtp_channel_events_assign_identity_and_sequence() {
        let events = rtp_channel_events(&[0x90, 60, 100, 61, 0], 7, 42, 10).expect("events");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].endpoint.get(), 7);
        assert_eq!(events[0].timestamp.get(), 42);
        assert_eq!(events[0].sequence, 10);
        assert_eq!(events[1].sequence, 11);
    }

    #[test]
    fn rtp_system_events_assign_identity_and_sequence() {
        let messages = vec![
            RtpMidiSystemMessage { status: 0xf8, data: [0, 0], data_len: 0 },
            RtpMidiSystemMessage { status: 0xf6, data: [0, 0], data_len: 0 },
        ];
        let events = rtp_system_events(&messages, 9, 77, 20).expect("events");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].endpoint.get(), 9);
        assert_eq!(events[1].sequence, 21);
    }

    #[test]
    fn rtp_sysex_requires_complete_framing() {
        let message = rtp_sysex_to_message(&[0xf0, 0x01, 0x7f, 0xf7]).expect("sysex");
        assert_eq!(message.wire_bytes(), vec![0xf0, 0x01, 0x7f, 0xf7]);
        assert!(rtp_sysex_to_message(&[0xf0, 0x01]).is_err());
        assert!(rtp_sysex_to_message(&[0xf0, 0x80, 0xf7]).is_err());
        let mut oversized = vec![0x01; 4097];
        oversized[0] = 0xf0;
        oversized[4096] = 0xf7;
        assert!(rtp_sysex_to_message(&oversized).is_err());
    }

    #[test]
    fn rtp_sysex_event_assigns_metadata() {
        let event = rtp_sysex_event(&[0xf0, 1, 2, 0xf7], 4, 99, 8).expect("event");
        assert_eq!(event.endpoint.get(), 4);
        assert_eq!(event.timestamp.get(), 99);
        assert_eq!(event.sequence, 8);
    }

    #[test]
    fn rtp_midi_system_decoder_handles_common_and_realtime() {
        let messages = decode_rtp_midi_system(&[0xF1, 3, 0xF2, 1, 2, 0xF8, 0xF6]).expect("system");
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[1].data, [1, 2]);
        assert_eq!(messages[2].data_len, 0);
        assert!(decode_rtp_midi_system(&[0xF2, 1]).is_err());
        assert!(decode_rtp_midi_system(&[0xF4]).is_err());
    }

    #[test]
    fn sysex_reassembler_emits_only_complete_bounded_messages() {
        let mut reassembler = SysexReassembler::new(4).expect("limit");
        assert_eq!(reassembler.push(&[1, 2], true, false).expect("start"), None);
        assert_eq!(reassembler.push(&[3, 4], false, true).expect("end"), Some(vec![1, 2, 3, 4]));
        assert!(reassembler.push(&[1], false, true).is_err());
        assert!(reassembler.push(&[1, 2, 3, 4, 5], true, true).is_err());
        assert!(SysexReassembler::new(0).is_err());
        assert!(reassembler.push(&[0x80], true, true).is_err());
    }

    #[test]
    fn sequence_tracker_classifies_gaps_duplicates_late_and_wraparound() {
        let mut tracker = SequenceTracker::new(4).expect("window");
        assert_eq!(tracker.observe(u16::MAX), SequenceDisposition::InOrder);
        assert_eq!(tracker.observe(0), SequenceDisposition::InOrder);
        assert_eq!(tracker.observe(2), SequenceDisposition::ForwardGap { missing: 1 });
        assert_eq!(tracker.observe(2), SequenceDisposition::Late);
        assert_eq!(tracker.observe(1), SequenceDisposition::Late);
        tracker.reset();
        assert_eq!(tracker.observe(500), SequenceDisposition::InOrder);
        assert_eq!(SequenceTracker::new(0), Err("RTP reorder window is out of bounds"));
    }

    #[test]
    fn jitter_buffer_is_bounded_and_ordered() {
        let mut buffer = JitterBuffer::new(2).expect("capacity");
        buffer.push(JitterPacket { timestamp: 20, sequence: 2, payload: "late" }).expect("insert");
        buffer.push(JitterPacket { timestamp: 10, sequence: 1, payload: "first" }).expect("insert");
        assert_eq!(buffer.pop().expect("first").payload, "first");
        assert_eq!(buffer.pop().expect("second").payload, "late");
        assert!(buffer.push(JitterPacket { timestamp: 1, sequence: 1, payload: "x" }).is_ok());
        assert!(buffer.push(JitterPacket { timestamp: 2, sequence: 2, payload: "y" }).is_ok());
        assert!(buffer.push(JitterPacket { timestamp: 3, sequence: 3, payload: "z" }).is_err());
        assert_eq!(buffer.drain_until(1).len(), 1);
        assert!(buffer.pop().is_some());
    }

    #[test]
    fn transport_stats_are_saturating_and_explicit() {
        let mut stats = TransportStats::default();
        stats.record_received();
        stats.record_sent();
        stats.record_malformed();
        stats.record_dropped();
        stats.record_late();
        stats.record_overflow();
        assert_eq!(
            stats,
            TransportStats { received: 1, sent: 1, malformed: 1, dropped: 1, late: 1, overflow: 1 }
        );
        stats.received = u64::MAX;
        stats.record_received();
        assert_eq!(stats.received, u64::MAX);
    }

    #[test]
    fn physical_devices_group_matching_ports_deterministically() {
        let endpoints = vec![
            EndpointInfo {
                id: "out-b".into(),
                name: "Launch Control XL".into(),
                direction: EndpointDirection::Output,
            },
            EndpointInfo {
                id: "in-b".into(),
                name: "Launch Control XL".into(),
                direction: EndpointDirection::Input,
            },
            EndpointInfo {
                id: "in-a".into(),
                name: "MIDI Interface".into(),
                direction: EndpointDirection::Input,
            },
        ];
        let devices = group_physical_devices(&endpoints);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].name, "Launch Control XL");
        assert_eq!(devices[0].inputs, ["in-b"]);
        assert_eq!(devices[0].outputs, ["out-b"]);
        assert_eq!(devices[0].state, PhysicalDeviceState::Connected);
        assert_eq!(devices[1].name, "MIDI Interface");
        assert_eq!(devices[1].outputs, Vec::<String>::new());
    }

    #[test]
    fn physical_device_grouping_does_not_merge_distinct_names() {
        let endpoints = vec![
            EndpointInfo {
                id: "a".into(),
                name: "Controller A".into(),
                direction: EndpointDirection::Input,
            },
            EndpointInfo {
                id: "b".into(),
                name: "Controller B".into(),
                direction: EndpointDirection::Input,
            },
        ];
        let devices = group_physical_devices(&endpoints);
        assert_eq!(
            devices.iter().map(|device| device.name.as_str()).collect::<Vec<_>>(),
            ["Controller A", "Controller B"]
        );
    }

    #[test]
    fn physical_device_grouping_normalizes_alsa_device_prefixes() {
        let endpoints = vec![
            EndpointInfo {
                id: "in-a".into(),
                name: "Launch Control XL:Launch Control XL Launch Contro 28:0".into(),
                direction: EndpointDirection::Input,
            },
            EndpointInfo {
                id: "in-b".into(),
                name: "Launch Control XL:Launch Control XL HUI 28:1".into(),
                direction: EndpointDirection::Input,
            },
            EndpointInfo {
                id: "out-a".into(),
                name: "Launch Control XL:Launch Control XL Launch Contro 28:0".into(),
                direction: EndpointDirection::Output,
            },
        ];
        let devices = group_physical_devices(&endpoints);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "Launch Control XL");
        assert_eq!(devices[0].inputs, vec!["in-a", "in-b"]);
        assert_eq!(devices[0].outputs, vec!["out-a"]);
    }

    #[test]
    fn activity_coalescer_keeps_newest_value_per_control() {
        let endpoint = mackes_domain::EndpointId::new(4).expect("endpoint");
        let event = |sequence, value| mackes_domain::MidiEvent {
            timestamp: mackes_domain::TimestampNanos::new(sequence),
            sequence,
            endpoint,
            message: mackes_domain::MidiMessage::ControlChange {
                channel: mackes_domain::MidiChannel::new(1).expect("channel"),
                controller: mackes_domain::SevenBit::new(21).expect("controller"),
                value: mackes_domain::SevenBit::new(value).expect("value"),
            },
        };
        let mut coalescer = ActivityCoalescer::new(2).expect("capacity");
        assert!(coalescer.push(&event(1, 10)));
        assert!(coalescer.push(&event(3, 99)));
        assert!(!coalescer.push(&event(2, 50)));
        assert_eq!(coalescer.len(), 1);
        let samples = coalescer.drain();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].sequence, 3);
        assert_eq!(samples[0].value, Some(99));
        assert_eq!(coalescer.len(), 0);
    }

    #[test]
    fn activity_coalescer_bounds_distinct_controls() {
        let event = |endpoint: u64, number: u8| mackes_domain::MidiEvent {
            timestamp: mackes_domain::TimestampNanos::new(u64::from(number)),
            sequence: u64::from(number),
            endpoint: mackes_domain::EndpointId::new(endpoint).expect("endpoint"),
            message: mackes_domain::MidiMessage::ControlChange {
                channel: mackes_domain::MidiChannel::new(1).expect("channel"),
                controller: mackes_domain::SevenBit::new(number.into()).expect("controller"),
                value: mackes_domain::SevenBit::new(number.into()).expect("value"),
            },
        };
        let mut coalescer = ActivityCoalescer::new(1).expect("capacity");
        assert!(coalescer.push(&event(2, 2)));
        assert!(!coalescer.push(&event(1, 1)));
        assert_eq!(coalescer.drain()[0].key.endpoint.get(), 2);
        assert!(ActivityCoalescer::new(0).is_none());
    }

    #[test]
    fn peer_allowlist_is_bounded_unique_and_order_preserving() {
        let first = "127.0.0.1:5000".parse().expect("peer");
        let second = "127.0.0.1:5001".parse().expect("peer");
        let allowlist = PeerAllowlist::new(vec![first, second]).expect("allowlist");
        assert!(allowlist.contains(&first));
        assert_eq!(allowlist.peers(), &[first, second]);
        assert!(PeerAllowlist::new(vec![first, first]).is_err());
        assert!(PeerAllowlist::new(Vec::new()).is_err());
    }
}
