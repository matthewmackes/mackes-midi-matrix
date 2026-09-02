//! Bounded native ALSA MIDI 1.0 reader and decoder.
//!
//! The reader never routes. It drains a configured batch, projects timestamps and a
//! monotonic sequence, and counts malformed, unsupported, overflow, and spoofed events.

use crate::{numeric_endpoint_id, AlsaSequencerAddress, EndpointDirection, SysexReassembler};
use mackes_domain::{MidiEvent, MidiMessage, TimestampNanos};
use std::collections::{HashMap, VecDeque};

/// Default number of pending records decoded in one drain.
pub const DEFAULT_NATIVE_BATCH_LIMIT: usize = 32;
/// Maximum complete `SysEx` payload accepted from native ingress.
pub const MAX_NATIVE_SYSEX_BYTES: usize = 8192;
/// Maximum undecoded records retained before overflow counting begins.
pub const MAX_NATIVE_PENDING: usize = 256;

/// Mk2 Factory Template 1 Device press on zero-based channel 8.
pub const MK2_DEVICE_PRESS: &[u8] = &[0x98, 105, 127];
/// Mk2 Device release retained as note-on velocity zero.
pub const MK2_DEVICE_RELEASE: &[u8] = &[0x98, 105, 0];
/// Mk2 Up arrow CC104 press.
pub const MK2_ARROW_UP: &[u8] = &[0xB8, 104, 127];
/// Mk2 Down arrow CC105 press.
pub const MK2_ARROW_DOWN: &[u8] = &[0xB8, 105, 127];
/// Mk2 Left arrow CC106 press.
pub const MK2_ARROW_LEFT: &[u8] = &[0xB8, 106, 127];
/// Mk2 Right arrow CC107 press.
pub const MK2_ARROW_RIGHT: &[u8] = &[0xB8, 107, 127];
/// Mk2 knob row-1 column-1 CC13.
pub const MK2_KNOB_R1_C1: &[u8] = &[0xB8, 13, 64];
/// Mk2 fader 1 CC77.
pub const MK2_FADER_1: &[u8] = &[0xB8, 77, 0];

/// One undecoded native sequencer record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeAlsaPending {
    /// Volatile ALSA source address.
    pub source: AlsaSequencerAddress,
    /// Capture timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Complete or fragmented MIDI bytes from the sequencer.
    pub bytes: Vec<u8>,
}

/// Decoder counters frozen before daemon exposure.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativeAlsaCounters {
    /// Successfully decoded MIDI events.
    pub received: u64,
    /// Truncated, out-of-range, or malformed framing.
    pub malformed: u64,
    /// Recognized but unsupported sequencer/MIDI types.
    pub unsupported: u64,
    /// Records dropped because the pending queue was saturated.
    pub overflow: u64,
    /// Records whose source was not an explicit subscription.
    pub spoofed: u64,
}

/// Nonblocking native ALSA event reader with explicit subscriptions.
#[derive(Debug)]
pub struct NativeAlsaReader {
    batch_limit: usize,
    next_sequence: u64,
    subscriptions: HashMap<AlsaSequencerAddress, String>,
    pending: VecDeque<NativeAlsaPending>,
    sysex: HashMap<AlsaSequencerAddress, SysexReassembler>,
    counters: NativeAlsaCounters,
}

impl NativeAlsaReader {
    /// Creates a reader that drains at most `batch_limit` records per call.
    ///
    /// # Errors
    ///
    /// Returns an error when `batch_limit` is zero.
    pub fn new(batch_limit: usize) -> Result<Self, &'static str> {
        if batch_limit == 0 {
            return Err("native ALSA batch limit must be nonzero");
        }
        Ok(Self {
            batch_limit,
            next_sequence: 0,
            subscriptions: HashMap::new(),
            pending: VecDeque::new(),
            sysex: HashMap::new(),
            counters: NativeAlsaCounters::default(),
        })
    }

    /// Binds a volatile source address to a W083 stable endpoint identity.
    pub fn subscribe(&mut self, source: AlsaSequencerAddress, stable_id: impl Into<String>) {
        self.subscriptions.insert(source, stable_id.into());
        if let std::collections::hash_map::Entry::Vacant(entry) = self.sysex.entry(source) {
            if let Ok(reassembler) = SysexReassembler::new(MAX_NATIVE_SYSEX_BYTES) {
                entry.insert(reassembler);
            }
        }
    }

    /// Returns the configured drain batch limit.
    #[must_use]
    pub const fn batch_limit(&self) -> usize {
        self.batch_limit
    }

    /// Returns decoder counters.
    #[must_use]
    pub const fn counters(&self) -> NativeAlsaCounters {
        self.counters
    }

    /// Queues one nonblocking sequencer record without routing.
    pub fn ingest(&mut self, record: NativeAlsaPending) {
        if self.pending.len() >= MAX_NATIVE_PENDING {
            self.counters.overflow = self.counters.overflow.saturating_add(1);
            return;
        }
        self.pending.push_back(record);
    }

    /// Decodes at most the configured batch from the pending queue.
    pub fn drain(&mut self) -> Vec<MidiEvent> {
        let mut events = Vec::new();
        for _ in 0..self.batch_limit {
            let Some(record) = self.pending.pop_front() else { break };
            match self.decode_record(&record) {
                Ok(Some(event)) => {
                    self.counters.received = self.counters.received.saturating_add(1);
                    events.push(event);
                }
                Ok(None) => {}
                Err(NativeDecodeClass::Malformed) => {
                    self.counters.malformed = self.counters.malformed.saturating_add(1);
                }
                Err(NativeDecodeClass::Unsupported) => {
                    self.counters.unsupported = self.counters.unsupported.saturating_add(1);
                }
                Err(NativeDecodeClass::Spoofed) => {
                    self.counters.spoofed = self.counters.spoofed.saturating_add(1);
                }
            }
        }
        events
    }

    fn decode_record(
        &mut self,
        record: &NativeAlsaPending,
    ) -> Result<Option<MidiEvent>, NativeDecodeClass> {
        let stable_id =
            self.subscriptions.get(&record.source).cloned().ok_or(NativeDecodeClass::Spoofed)?;
        if record.bytes.is_empty() {
            return Err(NativeDecodeClass::Unsupported);
        }
        let message = self.decode_bytes(record.source, &record.bytes)?;
        let Some(message) = message else { return Ok(None) };
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        let endpoint = numeric_endpoint_id(&stable_id).ok_or(NativeDecodeClass::Malformed)?;
        Ok(Some(MidiEvent {
            timestamp: TimestampNanos::new(record.timestamp_ns),
            sequence,
            endpoint,
            message,
        }))
    }

    fn decode_bytes(
        &mut self,
        source: AlsaSequencerAddress,
        bytes: &[u8],
    ) -> Result<Option<MidiMessage>, NativeDecodeClass> {
        if looks_like_sysex_fragment(bytes) {
            return self.push_sysex_fragment(source, bytes);
        }
        classify_wire(MidiMessage::from_wire(bytes)).map(Some)
    }

    fn push_sysex_fragment(
        &mut self,
        source: AlsaSequencerAddress,
        bytes: &[u8],
    ) -> Result<Option<MidiMessage>, NativeDecodeClass> {
        let start = bytes.first() == Some(&0xF0);
        let end = bytes.last() == Some(&0xF7);
        let payload = match (start, end, bytes.len()) {
            (true, true, _) => return classify_wire(MidiMessage::from_wire(bytes)).map(Some),
            (true, false, _) => bytes.get(1..).unwrap_or(&[]),
            (false, true, _) => bytes.get(..bytes.len().saturating_sub(1)).unwrap_or(&[]),
            (false, false, _) => bytes,
        };
        let reassembler = self.sysex.get_mut(&source).ok_or(NativeDecodeClass::Spoofed)?;
        match reassembler.push(payload, start, end) {
            Ok(Some(complete)) => {
                let mut framed = Vec::with_capacity(complete.len() + 2);
                framed.push(0xF0);
                framed.extend_from_slice(&complete);
                framed.push(0xF7);
                classify_wire(MidiMessage::from_wire(&framed)).map(Some)
            }
            Ok(None) => Ok(None),
            Err(_) => Err(NativeDecodeClass::Malformed),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeDecodeClass {
    Malformed,
    Unsupported,
    Spoofed,
}

fn looks_like_sysex_fragment(bytes: &[u8]) -> bool {
    let start = bytes.first() == Some(&0xF0);
    let end = bytes.last() == Some(&0xF7);
    match (start, end) {
        (true, true) => false,
        (true, false) | (false, true) => true,
        (false, false) => !bytes.is_empty() && bytes.iter().all(|byte| *byte < 0x80),
    }
}

fn classify_wire(
    result: Result<MidiMessage, &'static str>,
) -> Result<MidiMessage, NativeDecodeClass> {
    match result {
        Ok(message) => Ok(message),
        Err("unsupported system message" | "unsupported MIDI status") => {
            Err(NativeDecodeClass::Unsupported)
        }
        Err(_) => Err(NativeDecodeClass::Malformed),
    }
}

/// Stable identity used by golden native-reader fixtures.
#[must_use]
pub fn mk2_fixture_stable_id() -> String {
    crate::stable_endpoint_id("Launch Control XL MK2 MIDI", EndpointDirection::Input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mackes_domain::{MidiChannel, SevenBit};

    fn source() -> AlsaSequencerAddress {
        AlsaSequencerAddress::new(28, 0)
    }

    fn reader() -> NativeAlsaReader {
        let mut reader = NativeAlsaReader::new(DEFAULT_NATIVE_BATCH_LIMIT).expect("reader");
        reader.subscribe(source(), mk2_fixture_stable_id());
        reader
    }

    fn ingest(reader: &mut NativeAlsaReader, timestamp: u64, bytes: &[u8]) {
        reader.ingest(NativeAlsaPending {
            source: source(),
            timestamp_ns: timestamp,
            bytes: bytes.to_vec(),
        });
    }

    fn note_on(bytes: &[u8]) -> (u8, u8, u8) {
        match MidiMessage::from_wire(bytes).expect("note") {
            MidiMessage::NoteOn { channel, note, velocity } => {
                (channel.one_based(), note.as_u8(), velocity.as_u8())
            }
            other => panic!("expected note-on, got {other:?}"),
        }
    }

    fn cc(bytes: &[u8]) -> (u8, u8, u8) {
        match MidiMessage::from_wire(bytes).expect("cc") {
            MidiMessage::ControlChange { channel, controller, value } => {
                (channel.one_based(), controller.as_u8(), value.as_u8())
            }
            other => panic!("expected CC, got {other:?}"),
        }
    }

    #[test]
    fn mk2_device_note_105_arrives_once_per_press_and_release() {
        let mut reader = reader();
        ingest(&mut reader, 10, MK2_DEVICE_PRESS);
        ingest(&mut reader, 11, MK2_DEVICE_RELEASE);
        let events = reader.drain();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence, 0);
        assert_eq!(events[1].sequence, 1);
        assert_eq!(events[0].timestamp, TimestampNanos::new(10));
        assert_eq!(note_on(&events[0].message.wire_bytes()), (9, 105, 127));
        assert_eq!(note_on(&events[1].message.wire_bytes()), (9, 105, 0));
        assert_eq!(reader.counters().received, 2);
        assert_eq!(reader.counters().malformed, 0);
    }

    #[test]
    fn mk2_arrows_and_continuous_controls_decode() {
        let mut reader = reader();
        for (stamp, bytes) in [
            (1, MK2_ARROW_UP),
            (2, MK2_ARROW_DOWN),
            (3, MK2_ARROW_LEFT),
            (4, MK2_ARROW_RIGHT),
            (5, MK2_KNOB_R1_C1),
            (6, MK2_FADER_1),
        ] {
            ingest(&mut reader, stamp, bytes);
        }
        let events = reader.drain();
        assert_eq!(events.len(), 6);
        assert_eq!(cc(&events[0].message.wire_bytes()), (9, 104, 127));
        assert_eq!(cc(&events[1].message.wire_bytes()), (9, 105, 127));
        assert_eq!(cc(&events[2].message.wire_bytes()), (9, 106, 127));
        assert_eq!(cc(&events[3].message.wire_bytes()), (9, 107, 127));
        assert_eq!(cc(&events[4].message.wire_bytes()), (9, 13, 64));
        assert_eq!(cc(&events[5].message.wire_bytes()), (9, 77, 0));
    }

    #[test]
    fn velocity_zero_remains_note_on() {
        let mut reader = reader();
        ingest(&mut reader, 1, MK2_DEVICE_RELEASE);
        let event = &reader.drain()[0];
        assert!(matches!(
            event.message,
            MidiMessage::NoteOn { velocity, .. } if velocity == SevenBit::new(0).expect("zero")
        ));
    }

    #[test]
    fn channel_and_range_bounds_fail_closed() {
        let mut reader = reader();
        ingest(&mut reader, 1, &[0x90, 60]);
        ingest(&mut reader, 2, &[0x9F, 127, 127]);
        ingest(&mut reader, 3, &[0xB8, 13, 128]);
        let events = reader.drain();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].message,
            MidiMessage::NoteOn {
                channel: MidiChannel::new(16).expect("ch"),
                note: SevenBit::new(127).expect("note"),
                velocity: SevenBit::new(127).expect("vel"),
            }
        );
        assert_eq!(reader.counters().malformed, 2);
    }

    #[test]
    fn ordered_mixed_events_preserve_timestamp_and_sequence() {
        let mut reader = reader();
        ingest(&mut reader, 100, MK2_DEVICE_PRESS);
        ingest(&mut reader, 101, MK2_KNOB_R1_C1);
        ingest(&mut reader, 102, &[0xC8, 3]);
        ingest(&mut reader, 103, &[0xA8, 60, 10]);
        ingest(&mut reader, 104, &[0xD8, 20]);
        ingest(&mut reader, 105, &[0xE8, 0x00, 0x40]);
        ingest(&mut reader, 106, &[0xF8]);
        let events = reader.drain();
        assert_eq!(events.len(), 7);
        let sequences: Vec<_> = events.iter().map(|event| event.sequence).collect();
        assert_eq!(sequences, vec![0, 1, 2, 3, 4, 5, 6]);
        let stamps: Vec<_> = events.iter().map(|event| event.timestamp.get()).collect();
        assert_eq!(stamps, vec![100, 101, 102, 103, 104, 105, 106]);
    }

    #[test]
    fn fragmented_maximum_and_oversized_sysex_are_bounded() {
        let mut reader = reader();
        ingest(&mut reader, 1, &[0xF0, 0x01]);
        ingest(&mut reader, 2, &[0x02, 0xF7]);
        let complete = reader.drain();
        assert_eq!(complete.len(), 1);
        assert_eq!(complete[0].message.wire_bytes(), vec![0xF0, 0x01, 0x02, 0xF7]);

        let mut maximum = vec![0xF0];
        maximum.extend(std::iter::repeat_n(0x01, MAX_NATIVE_SYSEX_BYTES));
        maximum.push(0xF7);
        ingest(&mut reader, 3, &maximum);
        assert_eq!(reader.drain().len(), 1);

        let mut oversized = vec![0xF0];
        oversized.extend(std::iter::repeat_n(0x01, MAX_NATIVE_SYSEX_BYTES + 1));
        ingest(&mut reader, 4, &oversized);
        assert!(reader.drain().is_empty());
        assert_eq!(reader.counters().malformed, 1);
    }

    #[test]
    fn unsupported_event_and_source_spoof_are_rejected() {
        let mut reader = reader();
        ingest(&mut reader, 1, &[0xF4]);
        reader.ingest(NativeAlsaPending {
            source: AlsaSequencerAddress::new(99, 1),
            timestamp_ns: 2,
            bytes: MK2_DEVICE_PRESS.to_vec(),
        });
        ingest(&mut reader, 3, &[]);
        assert!(reader.drain().is_empty());
        assert_eq!(reader.counters().unsupported, 2);
        assert_eq!(reader.counters().spoofed, 1);
        assert_eq!(reader.counters().received, 0);
    }

    #[test]
    fn saturation_counts_overflow_and_recovers_after_drain() {
        let mut reader = NativeAlsaReader::new(2).expect("reader");
        reader.subscribe(source(), mk2_fixture_stable_id());
        for index in 0..MAX_NATIVE_PENDING {
            ingest(&mut reader, index as u64, MK2_DEVICE_PRESS);
        }
        ingest(&mut reader, 9_000, MK2_DEVICE_PRESS);
        assert_eq!(reader.counters().overflow, 1);
        let first = reader.drain();
        assert_eq!(first.len(), 2);
        ingest(&mut reader, 9_001, MK2_ARROW_UP);
        let recovered = reader.drain();
        assert_eq!(recovered.len(), 2);
        assert_eq!(reader.counters().overflow, 1);
        assert!(reader.counters().received >= 4);
    }

    #[test]
    fn drain_never_exceeds_batch_limit() {
        let mut reader = NativeAlsaReader::new(3).expect("reader");
        reader.subscribe(source(), mk2_fixture_stable_id());
        for stamp in 0..10 {
            ingest(&mut reader, stamp, MK2_DEVICE_PRESS);
        }
        assert_eq!(reader.drain().len(), 3);
        assert_eq!(reader.drain().len(), 3);
        assert_eq!(reader.drain().len(), 3);
        assert_eq!(reader.drain().len(), 1);
    }
}

#[cfg(all(test, feature = "alsa-seq-backend"))]
mod live_tests {
    use super::{NativeAlsaPending, NativeAlsaReader, MK2_DEVICE_PRESS};
    use crate::AlsaSequencerClient;
    use alsa::seq::{EvNote, Event, EventType};
    use std::ffi::CString;

    #[test]
    #[ignore = "requires snd-seq-dummy kernel module and explicit ports"]
    fn snd_seq_dummy_ingress_delivers_device_note() {
        let mut client = AlsaSequencerClient::open("MACKES dummy listener").expect("open listener");
        let ports = client.discover_ports();
        let dummy = ports
            .iter()
            .find(|port| {
                port.readable
                    && port.writable
                    && (port.client_name.to_ascii_lowercase().contains("dummy")
                        || port.client_name.to_ascii_lowercase().contains("midi through"))
            })
            .expect("snd-seq-dummy or Midi Through port");
        client.subscribe_input(dummy.address).expect("subscribe dummy");
        let mut reader = NativeAlsaReader::new(8).expect("reader");
        reader.subscribe(dummy.address, "alsa-dummy-in");
        let seq = alsa::seq::Seq::open(None, None, true).expect("injector");
        seq.set_client_name(&CString::new("MACKES dummy injector").expect("name"))
            .expect("name injector");
        let port = seq
            .create_simple_port(
                &CString::new("inject").expect("port"),
                alsa::seq::PortCap::READ | alsa::seq::PortCap::SUBS_READ,
                alsa::seq::PortType::MIDI_GENERIC | alsa::seq::PortType::APPLICATION,
            )
            .expect("inject port");
        let mut event = Event::new(
            EventType::Noteon,
            &EvNote { channel: 8, note: 105, velocity: 127, off_velocity: 0, duration: 0 },
        );
        event.set_source(port);
        event.set_dest(alsa::seq::Addr {
            client: i32::from(dummy.address.client),
            port: i32::from(dummy.address.port),
        });
        event.set_direct();
        seq.event_output_direct(&mut event).expect("send");
        let observed = client.read_wire_events(8).expect("read");
        for (address, bytes) in observed {
            reader.ingest(NativeAlsaPending { source: address, timestamp_ns: 1, bytes });
        }
        let decoded = reader.drain();
        assert!(
            decoded.iter().any(|event| event.message.wire_bytes() == MK2_DEVICE_PRESS),
            "dummy ingress did not deliver Device note 105"
        );
    }
}
