//! RTP-MIDI and `AppleMIDI` transport parsing and session helpers.

use mackes_domain::{
    EndpointId, FourteenBit, MidiChannel, MidiEvent, MidiMessage, RealtimeMessage, SevenBit,
    SystemCommonMessage, TimestampNanos,
};
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
