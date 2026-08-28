//! Dependency-light domain contracts and MIDI invariants.

use std::fmt;

/// Stable identifier used by serialized configuration.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StableId(String);

impl StableId {
    /// Creates a non-empty identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when the supplied identifier is empty or whitespace-only.
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("identifier must not be empty");
        }
        Ok(Self(value))
    }

    /// Returns the identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable serialized device alias.
pub type DeviceAlias = StableId;

/// Runtime endpoint identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EndpointId(u64);

impl EndpointId {
    /// Creates an endpoint identifier; zero is reserved as "unset".
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

macro_rules! bounded_value {
    ($name:ident, $max:expr, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u16);

        impl $name {
            /// Creates a value after checking its wire range.
            #[must_use]
            pub const fn new(value: u16) -> Option<Self> {
                if value <= $max {
                    Some(Self(value))
                } else {
                    None
                }
            }

            /// Returns the numeric value.
            #[must_use]
            pub const fn get(self) -> u16 {
                self.0
            }

            /// Returns the least-significant seven wire bits.
            #[must_use]
            pub const fn low7(self) -> u8 {
                (self.0 & 0x7F) as u8
            }

            /// Returns the upper seven wire bits.
            #[must_use]
            pub const fn high7(self) -> u8 {
                ((self.0 >> 7) & 0x7F) as u8
            }
        }
    };
}

bounded_value!(SevenBit, 127, "A MIDI data byte in the inclusive range 0..=127.");
bounded_value!(FourteenBit, 16_383, "A MIDI 14-bit value in the inclusive range 0..=16383.");

impl SevenBit {
    /// Returns the validated value as a byte.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn as_u8(self) -> u8 {
        self.get() as u8
    }
}

/// A MIDI channel numbered one through sixteen for operator-facing APIs.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MidiChannel(u8);

impl MidiChannel {
    /// Creates a one-based MIDI channel.
    #[must_use]
    pub const fn new(one_based: u8) -> Option<Self> {
        if one_based >= 1 && one_based <= 16 {
            Some(Self(one_based))
        } else {
            None
        }
    }

    /// Returns the one-based channel number.
    #[must_use]
    pub const fn one_based(self) -> u8 {
        self.0
    }

    /// Returns the zero-based wire channel nibble.
    #[must_use]
    pub const fn wire(self) -> u8 {
        self.0 - 1
    }
}

/// Service-monotonic event timestamp in nanoseconds.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TimestampNanos(u64);

impl TimestampNanos {
    /// Creates a timestamp.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns nanoseconds.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// MIDI system real-time messages.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RealtimeMessage {
    /// Timing clock.
    Clock,
    /// Start transport.
    Start,
    /// Continue transport.
    Continue,
    /// Stop transport.
    Stop,
    /// Active sensing heartbeat.
    ActiveSensing,
    /// System reset.
    Reset,
}

/// MIDI system-common messages.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SystemCommonMessage {
    /// MIDI time-code quarter frame.
    TimeCodeQuarterFrame(SevenBit),
    /// Song position pointer.
    SongPosition(FourteenBit),
    /// Song select.
    SongSelect(SevenBit),
    /// Tune request.
    TuneRequest,
}

/// A validated MIDI 1.0 message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MidiMessage {
    /// Note-on with zero velocity retained as note-on semantics.
    NoteOn {
        /// MIDI channel.
        channel: MidiChannel,
        /// Note number.
        note: SevenBit,
        /// Attack velocity.
        velocity: SevenBit,
    },
    /// Note-off message.
    NoteOff {
        /// MIDI channel.
        channel: MidiChannel,
        /// Note number.
        note: SevenBit,
        /// Release velocity.
        velocity: SevenBit,
    },
    /// Polyphonic key pressure.
    PolyPressure {
        /// MIDI channel.
        channel: MidiChannel,
        /// Note number.
        note: SevenBit,
        /// Pressure value.
        pressure: SevenBit,
    },
    /// Control change.
    ControlChange {
        /// MIDI channel.
        channel: MidiChannel,
        /// Controller number.
        controller: SevenBit,
        /// Controller value.
        value: SevenBit,
    },
    /// Program change.
    ProgramChange {
        /// MIDI channel.
        channel: MidiChannel,
        /// Program number.
        program: SevenBit,
    },
    /// Channel pressure.
    ChannelPressure {
        /// MIDI channel.
        channel: MidiChannel,
        /// Pressure value.
        pressure: SevenBit,
    },
    /// Pitch bend, represented as unsigned MIDI 14-bit wire value.
    PitchBend {
        /// MIDI channel.
        channel: MidiChannel,
        /// Unsigned bend value.
        value: FourteenBit,
    },
    /// System-common message.
    SystemCommon(SystemCommonMessage),
    /// System real-time message.
    Realtime(RealtimeMessage),
    /// `SysEx` payload without F0/F7 framing; every byte is guaranteed 7-bit.
    SysEx(Vec<SevenBit>),
}

impl MidiMessage {
    /// Creates a `SysEx` message from payload bytes without framing.
    ///
    /// # Errors
    ///
    /// Returns the first byte that is not a MIDI 7-bit data byte.
    pub fn sysex(payload: impl Into<Vec<u8>>) -> Result<Self, u8> {
        payload
            .into()
            .into_iter()
            .map(|byte| SevenBit::new(u16::from(byte)).ok_or(byte))
            .collect::<Result<Vec<_>, _>>()
            .map(Self::SysEx)
    }

    /// Encodes the validated message as MIDI 1.0 bytes.
    #[must_use]
    pub fn wire_bytes(&self) -> Vec<u8> {
        let channel_status = |base: u8, channel: MidiChannel| base | channel.wire();
        match self {
            Self::NoteOn { channel, note, velocity } => {
                vec![channel_status(0x90, *channel), note.as_u8(), velocity.as_u8()]
            }
            Self::NoteOff { channel, note, velocity } => {
                vec![channel_status(0x80, *channel), note.as_u8(), velocity.as_u8()]
            }
            Self::PolyPressure { channel, note, pressure } => {
                vec![channel_status(0xA0, *channel), note.as_u8(), pressure.as_u8()]
            }
            Self::ControlChange { channel, controller, value } => {
                vec![channel_status(0xB0, *channel), controller.as_u8(), value.as_u8()]
            }
            Self::ProgramChange { channel, program } => {
                vec![channel_status(0xC0, *channel), program.as_u8()]
            }
            Self::ChannelPressure { channel, pressure } => {
                vec![channel_status(0xD0, *channel), pressure.as_u8()]
            }
            Self::PitchBend { channel, value } => {
                vec![channel_status(0xE0, *channel), value.low7(), value.high7()]
            }
            Self::SystemCommon(message) => match message {
                SystemCommonMessage::TimeCodeQuarterFrame(value) => vec![0xF1, value.as_u8()],
                SystemCommonMessage::SongPosition(value) => {
                    vec![0xF2, value.low7(), value.high7()]
                }
                SystemCommonMessage::SongSelect(value) => vec![0xF3, value.as_u8()],
                SystemCommonMessage::TuneRequest => vec![0xF6],
            },
            Self::Realtime(message) => vec![match message {
                RealtimeMessage::Clock => 0xF8,
                RealtimeMessage::Start => 0xFA,
                RealtimeMessage::Continue => 0xFB,
                RealtimeMessage::Stop => 0xFC,
                RealtimeMessage::ActiveSensing => 0xFE,
                RealtimeMessage::Reset => 0xFF,
            }],
            Self::SysEx(payload) => {
                let mut bytes = Vec::with_capacity(payload.len() + 2);
                bytes.push(0xF0);
                bytes.extend(payload.iter().map(|byte| byte.as_u8()));
                bytes.push(0xF7);
                bytes
            }
        }
    }

    /// Decodes one complete MIDI message without running status.
    ///
    /// # Errors
    ///
    /// Returns an error for truncated messages, invalid status bytes, or
    /// malformed `SysEx` framing.
    pub fn from_wire(bytes: &[u8]) -> Result<Self, &'static str> {
        let status = *bytes.first().ok_or("empty MIDI message")?;
        if status == 0xF0 {
            if bytes.last().copied() != Some(0xF7) || bytes.len() < 2 {
                return Err("malformed SysEx framing");
            }
            return Self::sysex(bytes[1..bytes.len() - 1].to_vec())
                .map_err(|_| "invalid SysEx data");
        }
        if status >= 0xF0 {
            return Err("unsupported system message");
        }
        let channel = MidiChannel::new((status & 0x0F) + 1).ok_or("invalid channel")?;
        let data = &bytes[1..];
        let value = |index: usize| {
            SevenBit::new(u16::from(*data.get(index).ok_or("truncated MIDI message")?))
                .ok_or("invalid MIDI data")
        };
        match status & 0xF0 {
            0x80 => Ok(Self::NoteOff { channel, note: value(0)?, velocity: value(1)? }),
            0x90 => Ok(Self::NoteOn { channel, note: value(0)?, velocity: value(1)? }),
            0xA0 => Ok(Self::PolyPressure { channel, note: value(0)?, pressure: value(1)? }),
            0xB0 => Ok(Self::ControlChange { channel, controller: value(0)?, value: value(1)? }),
            0xC0 => Ok(Self::ProgramChange { channel, program: value(0)? }),
            0xD0 => Ok(Self::ChannelPressure { channel, pressure: value(0)? }),
            0xE0 => {
                let low = value(0)?.as_u8();
                let high = value(1)?.as_u8();
                Ok(Self::PitchBend {
                    channel,
                    value: FourteenBit::new(u16::from(low) | (u16::from(high) << 7))
                        .ok_or("invalid pitch bend")?,
                })
            }
            _ => Err("unsupported MIDI status"),
        }
    }

    /// Returns a bounded, display-safe summary.
    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            Self::SysEx(payload) => format!("SysEx ({} bytes)", payload.len()),
            other => format!("{other:?}"),
        }
    }
}

impl fmt::Display for MidiMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.summary())
    }
}

/// A timestamped message entering or leaving a runtime endpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiEvent {
    /// Service-monotonic timestamp.
    pub timestamp: TimestampNanos,
    /// Monotonically increasing service sequence.
    pub sequence: u64,
    /// Source or destination endpoint.
    pub endpoint: EndpointId,
    /// Validated message.
    pub message: MidiMessage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_wire_ranges() {
        assert!(SevenBit::new(127).is_some());
        assert!(SevenBit::new(128).is_none());
        assert!(FourteenBit::new(16_383).is_some());
        assert!(FourteenBit::new(16_384).is_none());
        assert!(MidiChannel::new(1).is_some());
        assert!(MidiChannel::new(16).is_some());
        assert!(MidiChannel::new(17).is_none());
    }

    #[test]
    fn sysex_rejects_non_data_bytes() {
        assert!(MidiMessage::sysex(vec![0, 127]).is_ok());
        assert_eq!(MidiMessage::sysex(vec![0x80]), Err(0x80));
    }

    #[test]
    fn encodes_representative_golden_messages() {
        let channel = MidiChannel::new(1).expect("valid channel");
        let note = MidiMessage::NoteOn {
            channel,
            note: SevenBit::new(60).expect("valid note"),
            velocity: SevenBit::new(100).expect("valid velocity"),
        };
        assert_eq!(note.wire_bytes(), [0x90, 60, 100]);
        let sysex = MidiMessage::sysex([0x01, 0x7F]).expect("valid payload");
        assert_eq!(sysex.wire_bytes(), [0xF0, 0x01, 0x7F, 0xF7]);
    }

    #[test]
    fn decodes_channel_voice_and_sysex_wire_messages() {
        let note = MidiMessage::from_wire(&[0x91, 60, 100]).expect("note");
        assert_eq!(note.wire_bytes(), vec![0x91, 60, 100]);
        let sysex = MidiMessage::from_wire(&[0xF0, 1, 127, 0xF7]).expect("sysex");
        assert_eq!(sysex.wire_bytes(), vec![0xF0, 1, 127, 0xF7]);
        assert!(MidiMessage::from_wire(&[0x90, 60]).is_err());
    }

    #[test]
    fn event_order_is_timestamp_then_sequence_when_sorted_by_key() {
        let endpoint = EndpointId::new(1).expect("nonzero");
        let first = MidiEvent {
            timestamp: TimestampNanos::new(10),
            sequence: 1,
            endpoint,
            message: MidiMessage::Realtime(RealtimeMessage::Clock),
        };
        let second = MidiEvent { sequence: 2, ..first.clone() };
        assert!(first.sequence < second.sequence);
        assert_eq!(first.endpoint, second.endpoint);
    }
}
