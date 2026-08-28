//! Declarative and built-in device profile boundary.

use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

/// Hardcoded Lexicon Reflex Rev. 1 protocol constants.
pub mod lexicon_reflex {
    /// Manual-defined Reflex DSP algorithm metadata.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct AlgorithmMetadata {
        /// Wire algorithm number (1–8).
        pub number: u8,
        /// Exact MRC/manual display label.
        pub name: &'static str,
        /// Manual description.
        pub description: &'static str,
        /// Preset numbers associated with this algorithm in the manual.
        pub preset_numbers: &'static [u8],
    }

    const REVERB_PRESETS: [u8; 6] = [1, 2, 3, 4, 5, 6];
    const PLATE_PRESETS: [u8; 3] = [9, 10, 11];
    const SINGLE_PRESET_12: [u8; 1] = [12];
    const SINGLE_PRESET_15: [u8; 1] = [15];
    const SINGLE_PRESET_16: [u8; 1] = [16];
    const SINGLE_PRESET_7: [u8; 1] = [7];
    const SINGLE_PRESET_8: [u8; 1] = [8];
    const DELAY1_PRESETS: [u8; 2] = [13, 14];

    /// The eight algorithms in the exact Appendix A order.
    pub const ALGORITHMS: [AlgorithmMetadata; 8] = [
        AlgorithmMetadata {
            number: 1,
            name: "Reverb",
            description: "Rooms and Halls",
            preset_numbers: &REVERB_PRESETS,
        },
        AlgorithmMetadata {
            number: 2,
            name: "Plate",
            description: "Plate Emulation",
            preset_numbers: &PLATE_PRESETS,
        },
        AlgorithmMetadata {
            number: 3,
            name: "Chorus 1",
            description: "Flanger",
            preset_numbers: &SINGLE_PRESET_12,
        },
        AlgorithmMetadata {
            number: 4,
            name: "Delay 2",
            description: "Multi-Tap Delay",
            preset_numbers: &SINGLE_PRESET_15,
        },
        AlgorithmMetadata {
            number: 5,
            name: "Chorus 2",
            description: "Resonator",
            preset_numbers: &SINGLE_PRESET_16,
        },
        AlgorithmMetadata {
            number: 6,
            name: "Inverse",
            description: "Inverse Room",
            preset_numbers: &SINGLE_PRESET_7,
        },
        AlgorithmMetadata {
            number: 7,
            name: "Gate",
            description: "Gated Reverb",
            preset_numbers: &SINGLE_PRESET_8,
        },
        AlgorithmMetadata {
            number: 8,
            name: "Delay 1",
            description: "Chorus/Delays",
            preset_numbers: &DELAY1_PRESETS,
        },
    ];

    /// Returns the compiled algorithm table without permitting caller reordering.
    #[must_use]
    pub const fn algorithms() -> &'static [AlgorithmMetadata; 8] {
        &ALGORITHMS
    }

    /// Manual-defined audio parameter metadata used by the Reflex workspace.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct ParameterMetadata {
        /// Zero-based wire parameter number.
        pub number: u8,
        /// Exact manual description.
        pub description: &'static str,
        /// MRC-facing label where documented.
        pub mrc_name: &'static str,
        /// Bipolar or unipolar polarity.
        pub bipolar: bool,
        /// Effective number of usable steps.
        pub effective_steps: u16,
        /// Inclusive legal 16-bit value range.
        pub min: u16,
        /// Inclusive legal 16-bit value range.
        pub max: u16,
    }

    const U: u16 = 0x8000;
    const H: u16 = 0xBFC0;
    const B: u16 = 0x4000;
    const BH: u16 = 0xBF80;
    const BC: u16 = 0xBC00;

    const REVERB_PARAMETERS: [ParameterMetadata; 10] = [
        ParameterMetadata {
            number: 0,
            description: "Mid Reverb Decay",
            mrc_name: "RTIME",
            bipolar: false,
            effective_steps: 16,
            min: U,
            max: BC,
        },
        ParameterMetadata {
            number: 1,
            description: "Predelay",
            mrc_name: "PDLY",
            bipolar: false,
            effective_steps: 8192,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 2,
            description: "Effects Level",
            mrc_name: "FXLVL",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 3,
            description: "Bass Multiply",
            mrc_name: "BASS",
            bipolar: true,
            effective_steps: 32,
            min: B,
            max: 0xB800,
        },
        ParameterMetadata {
            number: 4,
            description: "High Freq Cutoff",
            mrc_name: "HICUT",
            bipolar: false,
            effective_steps: 16,
            min: U,
            max: BC,
        },
        ParameterMetadata {
            number: 5,
            description: "Size",
            mrc_name: "SIZE",
            bipolar: false,
            effective_steps: 64,
            min: U,
            max: 0xBF00,
        },
        ParameterMetadata {
            number: 6,
            description: "Predelay Feedback",
            mrc_name: "FDBK",
            bipolar: true,
            effective_steps: 512,
            min: B,
            max: BH,
        },
        ParameterMetadata {
            number: 7,
            description: "Diffusion",
            mrc_name: "DIFF",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 8,
            description: "Reflection Level",
            mrc_name: "",
            bipolar: false,
            effective_steps: 128,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 9,
            description: "Reflection Delay",
            mrc_name: "",
            bipolar: false,
            effective_steps: 128,
            min: U,
            max: H,
        },
    ];
    const PLATE_PARAMETERS: [ParameterMetadata; 8] = [
        REVERB_PARAMETERS[0],
        REVERB_PARAMETERS[1],
        REVERB_PARAMETERS[2],
        REVERB_PARAMETERS[3],
        REVERB_PARAMETERS[4],
        REVERB_PARAMETERS[5],
        REVERB_PARAMETERS[6],
        REVERB_PARAMETERS[7],
    ];
    const CHORUS1_PARAMETERS: [ParameterMetadata; 8] = [
        ParameterMetadata {
            number: 0,
            description: "Negative Feedback",
            mrc_name: "",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 1,
            description: "Flange Depth",
            mrc_name: "DEPTH",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 2,
            description: "Effects Level",
            mrc_name: "FXLVL",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 3,
            description: "Right Dly Feedback",
            mrc_name: "RFDBK",
            bipolar: true,
            effective_steps: 512,
            min: B,
            max: BH,
        },
        ParameterMetadata {
            number: 4,
            description: "Right Delay",
            mrc_name: "R-DLY",
            bipolar: false,
            effective_steps: 128,
            min: U,
            max: BH,
        },
        ParameterMetadata {
            number: 5,
            description: "Shape",
            mrc_name: "SHAPE",
            bipolar: false,
            effective_steps: 8,
            min: U,
            max: 0xB800,
        },
        ParameterMetadata {
            number: 6,
            description: "Left Dly Feedback",
            mrc_name: "LFDBK",
            bipolar: true,
            effective_steps: 512,
            min: B,
            max: BH,
        },
        ParameterMetadata {
            number: 7,
            description: "Left Delay",
            mrc_name: "L-DLY",
            bipolar: false,
            effective_steps: 128,
            min: U,
            max: BH,
        },
    ];
    const DELAY2_PARAMETERS: [ParameterMetadata; 8] = [
        ParameterMetadata {
            number: 1,
            description: "Group Delay",
            mrc_name: "GPDLY",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 2,
            description: "Effects Level",
            mrc_name: "FXLVL",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 3,
            description: "Feedback",
            mrc_name: "FDBK",
            bipolar: true,
            effective_steps: 512,
            min: B,
            max: BH,
        },
        ParameterMetadata {
            number: 4,
            description: "Left Delay",
            mrc_name: "L-DLY",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 5,
            description: "Right Delay",
            mrc_name: "R-DLY",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 7,
            description: "High Freq Cutoff",
            mrc_name: "HICUT",
            bipolar: false,
            effective_steps: 16,
            min: U,
            max: BC,
        },
        ParameterMetadata {
            number: 8,
            description: "Diffusion",
            mrc_name: "DIFF",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 9,
            description: "Echo Rhythm",
            mrc_name: "",
            bipolar: false,
            effective_steps: 14,
            min: U,
            max: 0xB400,
        },
    ];
    const CHORUS2_PARAMETERS: [ParameterMetadata; 8] = [
        ParameterMetadata {
            number: 2,
            description: "Effects Level",
            mrc_name: "FXLVL",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 3,
            description: "Predelay",
            mrc_name: "PDLY",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 4,
            description: "Low Freq Cutoff",
            mrc_name: "LOCUT",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 5,
            description: "Shimmer",
            mrc_name: "SHIMR",
            bipolar: false,
            effective_steps: 16,
            min: U,
            max: BC,
        },
        ParameterMetadata {
            number: 6,
            description: "Resonance Fdbk",
            mrc_name: "RESON",
            bipolar: true,
            effective_steps: 64,
            min: B,
            max: 0xBE00,
        },
        ParameterMetadata {
            number: 7,
            description: "Richness",
            mrc_name: "RICH",
            bipolar: false,
            effective_steps: 16,
            min: U,
            max: BC,
        },
        ParameterMetadata {
            number: 8,
            description: "Slope",
            mrc_name: "SLOPE",
            bipolar: false,
            effective_steps: 32,
            min: U,
            max: 0xBE00,
        },
        ParameterMetadata {
            number: 9,
            description: "Tuning",
            mrc_name: "TUNE",
            bipolar: true,
            effective_steps: 128,
            min: B,
            max: 0xBF00,
        },
    ];
    const INVERSE_PARAMETERS: [ParameterMetadata; 7] = [
        ParameterMetadata {
            number: 0,
            description: "Size",
            mrc_name: "SIZE",
            bipolar: false,
            effective_steps: 32,
            min: U,
            max: 0xBE00,
        },
        ParameterMetadata {
            number: 2,
            description: "Effects Level",
            mrc_name: "FXLVL",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 4,
            description: "High Freq Cutoff",
            mrc_name: "HICUT",
            bipolar: false,
            effective_steps: 16,
            min: U,
            max: BC,
        },
        ParameterMetadata {
            number: 5,
            description: "Slope",
            mrc_name: "SLOPE",
            bipolar: false,
            effective_steps: 32,
            min: U,
            max: 0xBE00,
        },
        ParameterMetadata {
            number: 6,
            description: "Predelay Feedback",
            mrc_name: "FDBK",
            bipolar: true,
            effective_steps: 512,
            min: B,
            max: BH,
        },
        ParameterMetadata {
            number: 7,
            description: "Diffusion",
            mrc_name: "DIFF",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 8,
            description: "Predelay",
            mrc_name: "PDLY",
            bipolar: false,
            effective_steps: 8192,
            min: U,
            max: H,
        },
    ];
    const GATE_PARAMETERS: [ParameterMetadata; 7] = [
        ParameterMetadata {
            number: 0,
            description: "Gate Time",
            mrc_name: "TIME",
            bipolar: false,
            effective_steps: 32,
            min: U,
            max: 0xBE00,
        },
        ParameterMetadata {
            number: 2,
            description: "Effects Level",
            mrc_name: "FXLVL",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 4,
            description: "High Freq Cutoff",
            mrc_name: "HICUT",
            bipolar: false,
            effective_steps: 16,
            min: U,
            max: BC,
        },
        ParameterMetadata {
            number: 5,
            description: "Slope",
            mrc_name: "SLOPE",
            bipolar: false,
            effective_steps: 16,
            min: U,
            max: BC,
        },
        ParameterMetadata {
            number: 6,
            description: "Predelay Feedback",
            mrc_name: "FDBK",
            bipolar: true,
            effective_steps: 512,
            min: B,
            max: BH,
        },
        ParameterMetadata {
            number: 7,
            description: "Diffusion",
            mrc_name: "DIFF",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 8,
            description: "Predelay",
            mrc_name: "PDLY",
            bipolar: false,
            effective_steps: 8192,
            min: U,
            max: H,
        },
    ];
    const DELAY1_PARAMETERS: [ParameterMetadata; 8] = [
        ParameterMetadata {
            number: 1,
            description: "Delay 1",
            mrc_name: "DELAY",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: 0xF700,
        },
        ParameterMetadata {
            number: 2,
            description: "Effects Level",
            mrc_name: "FXLVL",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 3,
            description: "High Freq Cutoff",
            mrc_name: "HICUT",
            bipolar: false,
            effective_steps: 16,
            min: U,
            max: BC,
        },
        ParameterMetadata {
            number: 4,
            description: "Delay 2 Spread",
            mrc_name: "DLY-2",
            bipolar: false,
            effective_steps: 128,
            min: U,
            max: BH,
        },
        ParameterMetadata {
            number: 5,
            description: "Delay 3 Spread",
            mrc_name: "DLY-3",
            bipolar: false,
            effective_steps: 128,
            min: U,
            max: BH,
        },
        ParameterMetadata {
            number: 6,
            description: "Feedback",
            mrc_name: "FDBK3",
            bipolar: true,
            effective_steps: 512,
            min: B,
            max: BH,
        },
        ParameterMetadata {
            number: 7,
            description: "Diffusion",
            mrc_name: "DIFF",
            bipolar: false,
            effective_steps: 256,
            min: U,
            max: H,
        },
        ParameterMetadata {
            number: 8,
            description: "Chorus Rate",
            mrc_name: "RATE",
            bipolar: false,
            effective_steps: 16,
            min: U,
            max: BC,
        },
    ];

    /// Returns only parameters documented as usable for the selected algorithm.
    #[must_use]
    pub const fn parameters(algorithm: u8) -> &'static [ParameterMetadata] {
        match algorithm {
            1 => &REVERB_PARAMETERS,
            2 => &PLATE_PARAMETERS,
            3 => &CHORUS1_PARAMETERS,
            4 => &DELAY2_PARAMETERS,
            5 => &CHORUS2_PARAMETERS,
            6 => &INVERSE_PARAMETERS,
            7 => &GATE_PARAMETERS,
            8 => &DELAY1_PARAMETERS,
            _ => &[],
        }
    }

    /// Musical Echo Rhythm values documented by the Reflex MIDI implementation.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct EchoRhythm {
        /// Wire value, one through fourteen.
        pub value: u8,
        /// Exact manual label.
        pub label: &'static str,
    }

    /// Complete Echo Rhythm lookup table in wire order.
    pub const ECHO_RHYTHMS: [EchoRhythm; 14] = [
        EchoRhythm { value: 1, label: "64th" },
        EchoRhythm { value: 2, label: "Thirty-second" },
        EchoRhythm { value: 3, label: "Sixteenth Triplet" },
        EchoRhythm { value: 4, label: "Sixteenth note" },
        EchoRhythm { value: 5, label: "Eight Triplet" },
        EchoRhythm { value: 6, label: "Dotted Sixteenth note" },
        EchoRhythm { value: 7, label: "Eighth note" },
        EchoRhythm { value: 8, label: "Quarter Triplet" },
        EchoRhythm { value: 9, label: "Dotted Eighth note" },
        EchoRhythm { value: 10, label: "Quarter note" },
        EchoRhythm { value: 11, label: "Half triplet" },
        EchoRhythm { value: 12, label: "Dotted Quarter note" },
        EchoRhythm { value: 13, label: "Half note" },
        EchoRhythm { value: 14, label: "Whole note" },
    ];

    /// Looks up a valid Echo Rhythm wire value.
    #[must_use]
    pub fn echo_rhythm(value: u8) -> Option<&'static EchoRhythm> {
        ECHO_RHYTHMS.iter().find(|rhythm| rhythm.value == value)
    }

    /// Lexicon manufacturer ID.
    pub const MANUFACTURER_ID: u8 = 0x06;
    /// LXP-1-compatible Reflex product ID.
    pub const PRODUCT_ID: u8 = 0x02;
    /// Active/stored setup payload byte count before packing.
    pub const SETUP_BYTES: usize = 56;
    /// Rev.1 raw setup bytes (49 bytes pack to 56 MIDI-safe bytes).
    pub const SETUP_RAW_BYTES: usize = 49;
    /// Rev.1 packed setup bytes.
    pub const SETUP_PACKED_BYTES: usize = 56;

    /// Validated raw Rev.1 setup payload.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct ReflexSetup(pub [u8; SETUP_RAW_BYTES]);

    /// Complete validated 128-register Reflex bank.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct ReflexRegisterBank(pub Vec<ReflexSetup>);

    impl ReflexRegisterBank {
        /// Constructs a bank only when exactly 128 setups are supplied.
        ///
        /// # Errors
        ///
        /// Returns an error when the register count is not exactly 128.
        pub fn new(registers: Vec<ReflexSetup>) -> Result<Self, &'static str> {
            if registers.len() != 128 {
                return Err("Reflex register bank must contain 128 setups");
            }
            Ok(Self(registers))
        }
        /// Constructs a bank from the exact raw all-register payload.
        ///
        /// # Errors
        ///
        /// Returns an error when the payload is not exactly 6,272 bytes.
        pub fn from_raw(raw: &[u8]) -> Result<Self, &'static str> {
            if raw.len() != ALL_REGISTERS_RAW_BYTES {
                return Err("Reflex raw register bank has invalid size");
            }
            let registers = raw
                .chunks_exact(SETUP_RAW_BYTES)
                .map(ReflexSetup::new)
                .collect::<Result<Vec<_>, _>>()?;
            Self::new(registers)
        }
        /// Decodes a complete type-4 frame into a typed register bank.
        ///
        /// # Errors
        ///
        /// Returns an error when framing, checksum, channel, or bank size is invalid.
        pub fn from_frame(frame: &[u8]) -> Result<(u8, Self), &'static str> {
            let (channel, raw) = decode_all_registers_frame(frame)?;
            Ok((channel, Self::from_raw(&raw)?))
        }
        /// Returns a register by zero-based index.
        #[must_use]
        pub fn get(&self, index: u8) -> Option<&ReflexSetup> {
            self.0.get(usize::from(index))
        }
        /// Returns a mutable register by zero-based index.
        pub fn get_mut(&mut self, index: u8) -> Option<&mut ReflexSetup> {
            self.0.get_mut(usize::from(index))
        }
        /// Replaces one register while preserving the fixed bank size.
        ///
        /// # Errors
        ///
        /// Returns an error when the register index is outside 0–127.
        pub fn set(&mut self, index: u8, setup: ReflexSetup) -> Result<(), &'static str> {
            let Some(slot) = self.0.get_mut(usize::from(index)) else {
                return Err("Reflex register is out of range");
            };
            *slot = setup;
            Ok(())
        }
        /// Flattens the bank into the raw all-register payload.
        #[must_use]
        pub fn raw_bytes(&self) -> Vec<u8> {
            self.0.iter().flat_map(|setup| setup.0).collect()
        }
        /// Encodes this validated bank as a documented type-4 frame.
        ///
        /// # Errors
        ///
        /// Returns an error when the MIDI channel is invalid.
        pub fn encode_frame(&self, channel: u8) -> Result<Vec<u8>, &'static str> {
            encode_all_registers_frame(channel, &self.raw_bytes())
        }
    }

    impl serde::Serialize for ReflexSetup {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_bytes(&self.0)
        }
    }

    impl<'de> serde::Deserialize<'de> for ReflexSetup {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let bytes = <Vec<u8>>::deserialize(deserializer)?;
            Self::new(&bytes).map_err(serde::de::Error::custom)
        }
    }

    /// Four-slot MIDI patch mapping stored in a Reflex setup.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct ReflexPatch {
        /// MIDI patch sources (0–127).
        pub sources: [u8; 4],
        /// Audio parameter destinations (0–9).
        pub destinations: [u8; 4],
        /// Signed two's-complement scale values.
        pub scales: [i8; 4],
    }

    impl ReflexPatch {
        /// Validates the documented patch ranges.
        ///
        /// # Errors
        ///
        /// Returns an error when a destination exceeds parameter 9.
        pub fn validate(self) -> Result<(), &'static str> {
            if self.destinations.iter().any(|destination| *destination > 9) {
                return Err("Reflex patch destination is out of range");
            }
            Ok(())
        }
        /// Encodes patch fields into setup offsets 37–48.
        ///
        /// # Errors
        ///
        /// Returns an error when a destination is invalid.
        pub fn encode(self, setup: &mut [u8]) -> Result<(), &'static str> {
            self.validate()?;
            if setup.len() != SETUP_RAW_BYTES {
                return Err("Reflex raw setup must contain 49 bytes");
            }
            setup[37..41].copy_from_slice(&self.sources);
            setup[41..45].copy_from_slice(&self.destinations);
            for (index, scale) in self.scales.iter().copied().enumerate() {
                setup[45 + index] = scale.to_ne_bytes()[0];
            }
            Ok(())
        }

        /// Decodes patch fields from setup offsets 37–48.
        ///
        /// # Errors
        ///
        /// Returns an error when the setup size or destination values are invalid.
        pub fn decode(setup: &[u8]) -> Result<Self, &'static str> {
            if setup.len() != SETUP_RAW_BYTES {
                return Err("Reflex raw setup must contain 49 bytes");
            }
            let mut scales = [0_i8; 4];
            for (index, byte) in setup[45..49].iter().copied().enumerate() {
                scales[index] = i8::from_ne_bytes([byte]);
            }
            let patch = Self {
                sources: setup[37..41].try_into().map_err(|_| "invalid Reflex patch sources")?,
                destinations: setup[41..45]
                    .try_into()
                    .map_err(|_| "invalid Reflex patch destinations")?,
                scales,
            };
            patch.validate()?;
            Ok(patch)
        }
    }

    impl ReflexSetup {
        /// Constructs a setup from exactly 49 raw bytes.
        ///
        /// # Errors
        ///
        /// Returns an error when the input is not exactly 49 bytes.
        pub fn new(bytes: &[u8]) -> Result<Self, &'static str> {
            let array: [u8; SETUP_RAW_BYTES] =
                bytes.try_into().map_err(|_| "Reflex raw setup must contain 49 bytes")?;
            Ok(Self(array))
        }
        /// Returns the raw setup bytes.
        #[must_use]
        pub const fn as_bytes(&self) -> &[u8; SETUP_RAW_BYTES] {
            &self.0
        }
        /// Packs this setup for a MIDI transfer.
        #[must_use]
        pub fn packed(&self) -> Vec<u8> {
            pack(&self.0)
        }
        /// Constructs a setup from exactly 56 packed MIDI-safe bytes.
        ///
        /// # Errors
        ///
        /// Returns an error when packed bytes are malformed or decode to the wrong size.
        pub fn from_packed(packed: &[u8]) -> Result<Self, &'static str> {
            if packed.len() != SETUP_PACKED_BYTES || packed.iter().any(|byte| *byte > 127) {
                return Err("Reflex packed setup is invalid");
            }
            let mut raw = Vec::with_capacity(SETUP_RAW_BYTES);
            for group in packed.chunks(8) {
                let msb = group[0];
                for (index, byte) in group[1..].iter().copied().enumerate() {
                    raw.push(byte | (((msb >> index) & 1) << 7));
                }
            }
            Self::new(&raw)
        }
        /// Returns the documented algorithm ID (1–8), if valid.
        #[must_use]
        pub fn algorithm(&self) -> Option<u8> {
            (1..=8).contains(&self.0[0]).then_some(self.0[0])
        }
        /// Returns the raw 16-byte setup name field.
        #[must_use]
        pub fn name_bytes(&self) -> &[u8] {
            &self.0[21..37]
        }
        /// Replaces the setup name and NUL-terminates unused bytes.
        ///
        /// # Errors
        ///
        /// Returns an error when the name exceeds 16 bytes or contains MIDI framing bytes.
        pub fn set_name(&mut self, name: &[u8]) -> Result<(), &'static str> {
            if name.len() > 16
                || name.iter().any(|byte| *byte >= 0x80 || *byte == 0xF0 || *byte == 0xF7)
            {
                return Err("Reflex setup name is invalid");
            }
            self.0[21..37].fill(0);
            self.0[21..21 + name.len()].copy_from_slice(name);
            Ok(())
        }
        /// Extracts and validates the four-slot patch matrix.
        ///
        /// # Errors
        ///
        /// Returns an error if the setup contains an invalid patch destination.
        pub fn patch(&self) -> Result<ReflexPatch, &'static str> {
            ReflexPatch::decode(&self.0)
        }
        /// Reads one documented 16-bit audio parameter (0–9).
        #[must_use]
        pub fn parameter(&self, index: u8) -> Option<u16> {
            if index > 9 {
                return None;
            }
            let offset = 1 + usize::from(index) * 2;
            Some(u16::from_le_bytes([self.0[offset], self.0[offset + 1]]))
        }
        /// Writes one documented 16-bit audio parameter (0–9).
        ///
        /// # Errors
        ///
        /// Returns an error when the parameter index is outside 0–9.
        pub fn set_parameter(&mut self, index: u8, value: u16) -> Result<(), &'static str> {
            if index > 9 {
                return Err("Reflex audio parameter is out of range");
            }
            let offset = 1 + usize::from(index) * 2;
            self.0[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
            Ok(())
        }
    }
    /// Number of unpacked bytes in the 128-register bank (128 × 49).
    pub const ALL_REGISTERS_RAW_BYTES: usize = 6_272;
    /// Number of packed bytes in the 128-register bank (128 × 56).
    pub const ALL_REGISTERS_PACKED_BYTES: usize = 7_168;
    /// Maximum device/channel selector (zero-based nibble).
    pub const MAX_CHANNEL: u8 = 15;
    /// Request active setup.
    pub const REQUEST_ACTIVE: u8 = 0x60;
    /// Request one stored register.
    pub const REQUEST_REGISTER: u8 = 0x61;
    /// Request packed parameter.
    pub const REQUEST_PACKED_PARAMETER: u8 = 0x62;
    /// Request all registers.
    pub const REQUEST_ALL_REGISTERS: u8 = 0x64;
    /// Request nibblized parameter.
    pub const REQUEST_NIBBLIZED_PARAMETER: u8 = 0x65;
    /// System task: store active setup.
    pub const TASK_STORE: u8 = 0x70;
    /// System task: recall register.
    pub const TASK_RECALL: u8 = 0x71;
    /// System task: bypass.
    pub const TASK_BYPASS: u8 = 0x72;

    /// Encodes a type-3 request frame.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid channel, request code, or non-7-bit argument.
    pub fn encode_request(channel: u8, request: u8, argument: u8) -> Result<[u8; 7], &'static str> {
        let header = header(3, channel)?;
        if !matches!(
            request,
            REQUEST_ACTIVE
                | REQUEST_REGISTER
                | REQUEST_PACKED_PARAMETER
                | REQUEST_ALL_REGISTERS
                | REQUEST_NIBBLIZED_PARAMETER
        ) {
            return Err("unsupported Reflex request");
        }
        if argument > 127 {
            return Err("request argument is not MIDI data");
        }
        Ok([header[0], header[1], header[2], header[3], request, argument, 0xF7])
    }

    /// Decodes a type-3 Reflex request frame and validates its request-specific argument.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed framing, unsupported request codes, or invalid data.
    pub fn decode_request(frame: &[u8]) -> Result<(u8, u8, u8), &'static str> {
        if frame.len() != 7
            || frame[0..3] != [0xF0, MANUFACTURER_ID, PRODUCT_ID]
            || frame[6] != 0xF7
        {
            return Err("invalid Reflex request frame");
        }
        let channel = frame[3] & 0x0F;
        let request = frame[4];
        let argument = frame[5];
        if frame[3] >> 4 != 3
            || frame[5] > 127
            || !matches!(
                request,
                REQUEST_ACTIVE
                    | REQUEST_REGISTER
                    | REQUEST_PACKED_PARAMETER
                    | REQUEST_ALL_REGISTERS
                    | REQUEST_NIBBLIZED_PARAMETER
            )
        {
            return Err("invalid Reflex request payload");
        }
        if matches!(request, REQUEST_REGISTER) && argument > 127 {
            return Err("Reflex register is out of range");
        }
        Ok((channel, request, argument))
    }

    /// Builds the common framed header for a message type and zero-based channel.
    ///
    /// # Errors
    ///
    /// Returns an error for a message type outside 0..=6 or channel above 15.
    pub const fn header(message_type: u8, channel: u8) -> Result<[u8; 4], &'static str> {
        if message_type > 6 {
            return Err("unsupported Reflex message type");
        }
        if channel > MAX_CHANNEL {
            return Err("Reflex channel out of range");
        }
        Ok([0xF0, MANUFACTURER_ID, PRODUCT_ID, (message_type << 4) | channel])
    }

    /// Encodes a preferred nibblized parameter-adjust frame (type 5).
    ///
    /// # Errors
    ///
    /// Returns an error when the message type or channel is invalid.
    pub fn encode_nibblized_parameter(
        channel: u8,
        parameter: u8,
        value: u16,
    ) -> Result<Vec<u8>, &'static str> {
        let mut frame = header(5, channel)?.to_vec();
        frame.push(parameter & 0x7F);
        frame.extend(nibblize(value));
        frame.push(0xF7);
        Ok(frame)
    }

    /// Decodes a documented type-5 nibblized parameter adjustment.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid framing, parameter bytes, or nibble values.
    pub fn decode_nibblized_parameter(frame: &[u8]) -> Result<(u8, u8, u16), &'static str> {
        if frame.len() != 10
            || frame[0..3] != [0xF0, MANUFACTURER_ID, PRODUCT_ID]
            || frame[9] != 0xF7
        {
            return Err("invalid Reflex nibblized parameter frame");
        }
        let channel = frame[3] & 0x0F;
        if frame[3] >> 4 != 5 || frame[4] > 127 || frame[5..9].iter().any(|byte| *byte > 0x0F) {
            return Err("invalid Reflex nibblized parameter payload");
        }
        let value = (u16::from(frame[5]) << 12)
            | (u16::from(frame[6]) << 8)
            | (u16::from(frame[7]) << 4)
            | u16::from(frame[8]);
        Ok((channel, frame[4], value))
    }

    /// Encodes a documented type-2 packed 16-bit parameter adjustment.
    ///
    /// # Errors
    ///
    /// Returns an error when channel or parameter is outside the MIDI range.
    pub fn encode_packed_parameter(
        channel: u8,
        parameter: u8,
        value: u16,
    ) -> Result<Vec<u8>, &'static str> {
        let header = header(2, channel)?;
        let bytes = value.to_le_bytes();
        let packed = pack(&bytes);
        let mut frame = header.to_vec();
        frame.push(parameter & 0x7F);
        frame.extend(packed);
        frame.push(0xF7);
        Ok(frame)
    }

    /// Decodes a documented type-2 packed parameter adjustment.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid framing, parameter payload size, or MIDI bytes.
    pub fn decode_packed_parameter(frame: &[u8]) -> Result<(u8, u8, u16), &'static str> {
        if frame.len() != 9
            || frame[0..3] != [0xF0, MANUFACTURER_ID, PRODUCT_ID]
            || frame[8] != 0xF7
        {
            return Err("invalid Reflex packed parameter frame");
        }
        let channel = frame[3] & 0x0F;
        if frame[3] >> 4 != 2 || frame[4] > 127 || frame[5..8].iter().any(|byte| *byte > 127) {
            return Err("invalid Reflex packed parameter payload");
        }
        let msb = frame[5];
        let low = frame[6] | ((msb & 1) << 7);
        let high = frame[7] | (((msb >> 1) & 1) << 7);
        Ok((channel, frame[4], u16::from_le_bytes([low, high])))
    }

    /// Encodes a system-task frame (type 6), enforcing bypass safety.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid channels, tasks, or bypass arguments.
    pub fn encode_task(channel: u8, task: u8, argument: u8) -> Result<[u8; 7], &'static str> {
        let header = header(6, channel)?;
        if !matches!(task, TASK_STORE | TASK_RECALL | TASK_BYPASS) {
            return Err("unsupported Reflex task");
        }
        if task == TASK_BYPASS && argument > 1 {
            return Err("invalid bypass argument");
        }
        Ok([header[0], header[1], header[2], header[3], task, argument & 0x7F, 0xF7])
    }

    /// Decodes and validates a type-6 system-task frame.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed framing, unsupported tasks, or unsafe arguments.
    pub fn decode_task(frame: &[u8]) -> Result<(u8, u8, u8), &'static str> {
        if frame.len() != 7
            || frame[0] != 0xF0
            || frame[1] != MANUFACTURER_ID
            || frame[2] != PRODUCT_ID
            || frame[6] != 0xF7
        {
            return Err("invalid Reflex task frame");
        }
        let channel = frame[3] & 0x0F;
        if frame[3] >> 4 != 6 || channel > MAX_CHANNEL {
            return Err("invalid Reflex task header");
        }
        let task = frame[4];
        let argument = frame[5];
        if !matches!(task, TASK_STORE | TASK_RECALL | TASK_BYPASS) {
            return Err("unsupported Reflex task");
        }
        if argument > 127 || (task == TASK_BYPASS && argument > 1) {
            return Err("invalid Reflex task argument");
        }
        Ok((channel, task, argument))
    }

    /// Packs up to seven source bytes into one MSB byte plus seven data bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when more than seven bytes are supplied or a source byte
    /// is outside the 8-bit range (the latter is impossible for `u8`, retained
    /// as a stable validation contract).
    pub fn pack_group(source: &[u8]) -> Result<Vec<u8>, &'static str> {
        if source.len() > 7 {
            return Err("pack group exceeds seven bytes");
        }
        let mut msb = 0_u8;
        let mut packed = Vec::with_capacity(source.len() + 1);
        for (index, byte) in source.iter().copied().enumerate() {
            msb |= ((byte >> 7) & 1) << index;
            packed.push(byte & 0x7F);
        }
        packed.insert(0, msb);
        Ok(packed)
    }

    /// Packs an arbitrary payload in groups of seven source bytes.
    ///
    /// # Panics
    ///
    /// This function cannot panic because each chunk is bounded to seven bytes.
    #[must_use]
    pub fn pack(source: &[u8]) -> Vec<u8> {
        source.chunks(7).flat_map(|group| pack_group(group).expect("bounded chunk")).collect()
    }

    /// Unpacks a base-128 payload produced by [`pack`].
    ///
    /// # Errors
    ///
    /// Returns an error for an impossible packed length, non-MIDI data, or a
    /// source length that does not match the encoded groups.
    pub fn unpack(packed: &[u8], source_len: usize) -> Result<Vec<u8>, &'static str> {
        if source_len == 0 || packed.is_empty() || source_len > 7 * packed.len() {
            return Err("packed payload length is invalid");
        }
        let groups = source_len.div_ceil(7);
        if packed.len() != source_len + groups {
            return Err("packed payload group length is invalid");
        }
        let mut output = Vec::with_capacity(source_len);
        let mut offset = 0;
        for group in 0..groups {
            let count = (source_len - group * 7).min(7);
            let msb = packed[offset];
            offset += 1;
            for index in 0..count {
                let value = packed[offset];
                if value > 127 {
                    return Err("packed payload contains non-MIDI data");
                }
                output.push(value | (((msb >> index) & 1) << 7));
                offset += 1;
            }
        }
        Ok(output)
    }

    /// Packs exactly one complete 56-byte Reflex setup and appends its checksum.
    ///
    /// # Errors
    ///
    /// Returns an error unless the source contains exactly 56 bytes.
    pub fn encode_setup_dump(source: &[u8]) -> Result<Vec<u8>, &'static str> {
        if source.len() != SETUP_BYTES {
            return Err("Reflex setup must contain exactly 56 bytes");
        }
        let packed = pack(source);
        let checksum = checksum(&packed);
        let mut frame = packed;
        frame.push(checksum);
        Ok(frame)
    }

    /// Wraps a validated setup dump in the Rev.1 type-4 `SysEx` transfer frame.
    ///
    /// # Errors
    ///
    /// Returns an error when the channel or setup size is invalid.
    /// The caller supplies the zero-based MIDI channel.
    pub fn encode_setup_frame(channel: u8, source: &[u8]) -> Result<Vec<u8>, &'static str> {
        let header = header(4, channel)?;
        let dump = encode_setup_dump(source)?;
        let mut frame = header.to_vec();
        frame.extend(dump);
        frame.push(0xF7);
        Ok(frame)
    }

    /// Encodes a Rev.1 type-0 active setup from its 49 raw bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when channel or raw setup size is invalid.
    pub fn encode_active_setup_frame(channel: u8, source: &[u8]) -> Result<Vec<u8>, &'static str> {
        if source.len() != SETUP_RAW_BYTES {
            return Err("Reflex raw setup must contain 49 bytes");
        }
        let mut frame = header(0, channel)?.to_vec();
        frame.push(0x38);
        frame.extend(pack(source));
        frame.push(checksum(&frame[5..]));
        frame.push(0xF7);
        Ok(frame)
    }

    /// Encodes a Rev.1 type-1 stored-register setup from its 49 raw bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when channel, register, or raw setup size is invalid.
    pub fn encode_register_setup_frame(
        channel: u8,
        register: u8,
        source: &[u8],
    ) -> Result<Vec<u8>, &'static str> {
        if register > 127 {
            return Err("Reflex register is out of range");
        }
        if source.len() != SETUP_RAW_BYTES {
            return Err("Reflex raw setup must contain 49 bytes");
        }
        let mut frame = header(1, channel)?.to_vec();
        frame.extend([register, 0x38]);
        let packed = pack(source);
        frame.extend(&packed);
        frame.push(checksum(&packed));
        frame.push(0xF7);
        Ok(frame)
    }

    /// Encodes a complete type-4 all-register transfer from 128 raw 49-byte setups.
    ///
    /// # Errors
    ///
    /// Returns an error when the channel or bank size is invalid.
    pub fn encode_all_registers_frame(channel: u8, source: &[u8]) -> Result<Vec<u8>, &'static str> {
        let header = header(4, channel)?;
        if source.len() != ALL_REGISTERS_RAW_BYTES {
            return Err("Reflex register bank has invalid size");
        }
        let packed = pack(source);
        if packed.len() != ALL_REGISTERS_PACKED_BYTES {
            return Err("Reflex register bank packing has invalid size");
        }
        let mut frame = header.to_vec();
        frame.extend([0x38, 0x00]);
        frame.extend(&packed);
        frame.push(checksum(&packed));
        frame.push(0xF7);
        Ok(frame)
    }

    /// Decodes the packed 56-byte setup payload from a validated dump.
    ///
    /// # Errors
    ///
    /// Returns an error when framing, checksum, or decoded size is invalid.
    pub fn decode_setup_dump(frame: &[u8]) -> Result<Vec<u8>, &'static str> {
        let packed = validate_setup_dump(frame)?;
        let mut source = Vec::with_capacity(SETUP_BYTES);
        for group in packed.chunks(8) {
            let msb = group[0];
            for (index, byte) in group[1..].iter().copied().enumerate() {
                source.push(byte | (((msb >> index) & 1) << 7));
            }
        }
        if source.len() != SETUP_BYTES {
            return Err("Reflex setup dump decoded to invalid length");
        }
        Ok(source)
    }

    /// Validates and decodes a complete type-4 setup transfer frame.
    ///
    /// # Errors
    ///
    /// Returns an error when framing, channel, checksum, or payload size is invalid.
    pub fn decode_setup_frame(frame: &[u8]) -> Result<(u8, Vec<u8>), &'static str> {
        if frame.len() != 70
            || frame[0] != 0xF0
            || frame[1] != MANUFACTURER_ID
            || frame[2] != PRODUCT_ID
            || frame[69] != 0xF7
        {
            return Err("invalid Reflex setup frame");
        }
        let channel = frame[3] & 0x0F;
        if frame[3] >> 4 != 4 {
            return Err("invalid Reflex setup frame header");
        }
        Ok((channel, decode_setup_dump(&frame[4..69])?))
    }

    /// Decodes a Rev.1 type-0 active setup frame into 49 raw bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when framing, checksum, or packed size is invalid.
    pub fn decode_active_setup_frame(frame: &[u8]) -> Result<(u8, Vec<u8>), &'static str> {
        if frame.len() != 63
            || frame[0..3] != [0xF0, MANUFACTURER_ID, PRODUCT_ID]
            || frame[4] != 0x38
            || frame[62] != 0xF7
        {
            return Err("invalid Reflex active setup frame");
        }
        let channel = frame[3] & 0x0F;
        if frame[3] >> 4 != 0 {
            return Err("invalid Reflex active setup header");
        }
        let packed = &frame[5..61];
        if checksum(packed) != frame[61] || packed.iter().any(|byte| *byte > 127) {
            return Err("Reflex active setup checksum or data invalid");
        }
        decode_packed_setup(packed).map(|raw| (channel, raw))
    }

    /// Decodes a Rev.1 type-1 stored-register setup frame into 49 raw bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when framing, register, checksum, or packed size is invalid.
    pub fn decode_register_setup_frame(frame: &[u8]) -> Result<(u8, u8, Vec<u8>), &'static str> {
        if frame.len() != 64
            || frame[0..3] != [0xF0, MANUFACTURER_ID, PRODUCT_ID]
            || frame[5] != 0x38
            || frame[63] != 0xF7
        {
            return Err("invalid Reflex register setup frame");
        }
        let channel = frame[3] & 0x0F;
        if frame[3] >> 4 != 1 {
            return Err("invalid Reflex register setup header");
        }
        let packed = &frame[6..62];
        if checksum(packed) != frame[62] || packed.iter().any(|byte| *byte > 127) {
            return Err("Reflex register checksum or data invalid");
        }
        decode_packed_setup(packed).map(|raw| (channel, frame[4], raw))
    }

    fn decode_packed_setup(packed: &[u8]) -> Result<Vec<u8>, &'static str> {
        if packed.len() != SETUP_PACKED_BYTES {
            return Err("Reflex packed setup has invalid size");
        }
        let mut raw = Vec::with_capacity(SETUP_RAW_BYTES);
        for group in packed.chunks(8) {
            let msb = group[0];
            for (index, byte) in group[1..].iter().copied().enumerate() {
                raw.push(byte | (((msb >> index) & 1) << 7));
            }
        }
        if raw.len() == SETUP_RAW_BYTES {
            Ok(raw)
        } else {
            Err("Reflex setup decoded to invalid size")
        }
    }

    /// Validates and decodes a complete type-4 all-register transfer.
    ///
    /// # Errors
    ///
    /// Returns an error for bad framing, count, checksum, or packed data.
    pub fn decode_all_registers_frame(frame: &[u8]) -> Result<(u8, Vec<u8>), &'static str> {
        if frame.len() != 7_176
            || frame[0..3] != [0xF0, MANUFACTURER_ID, PRODUCT_ID]
            || frame[4..6] != [0x38, 0x00]
            || frame[7_175] != 0xF7
        {
            return Err("invalid Reflex all-register frame");
        }
        let channel = frame[3] & 0x0F;
        if frame[3] >> 4 != 4 {
            return Err("invalid Reflex all-register header");
        }
        let packed = &frame[6..7_174];
        if checksum(packed) != frame[7_174] || packed.iter().any(|byte| *byte > 127) {
            return Err("Reflex all-register checksum or data invalid");
        }
        let mut raw = Vec::with_capacity(ALL_REGISTERS_RAW_BYTES);
        for group in packed.chunks(8) {
            let msb = group[0];
            for (index, byte) in group[1..].iter().copied().enumerate() {
                raw.push(byte | (((msb >> index) & 1) << 7));
            }
        }
        if raw.len() != ALL_REGISTERS_RAW_BYTES {
            return Err("Reflex register bank decoded to invalid size");
        }
        Ok((channel, raw))
    }

    /// Typed result of decoding one complete Rev. 1 wire message.
    #[allow(missing_docs)]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum DecodedMessage {
        /// Type 0 active setup output.
        ActiveSetup { channel: u8, setup: Vec<u8> },
        /// Type 1 stored-register setup output.
        RegisterSetup { channel: u8, register: u8, setup: Vec<u8> },
        /// Type 2 packed parameter adjustment.
        PackedParameter { channel: u8, parameter: u8, value: u16 },
        /// Type 3 request.
        Request { channel: u8, request: u8, argument: u8 },
        /// Type 4 all-register transfer.
        AllRegisters { channel: u8, registers: Vec<u8> },
        /// Type 5 nibblized parameter adjustment.
        NibblizedParameter { channel: u8, parameter: u8, value: u16 },
        /// Type 6 system task.
        Task { channel: u8, task: u8, argument: u8 },
    }

    /// Decodes and classifies every supported Rev. 1 message type.
    ///
    /// The type nibble is used only for dispatch; each branch applies its complete
    /// framing, length, checksum, and payload validation before returning data.
    ///
    /// # Errors
    ///
    /// Returns an error when the header, message type, framing, checksum, or payload is invalid.
    pub fn decode_message(frame: &[u8]) -> Result<DecodedMessage, &'static str> {
        if frame.len() < 4
            || frame[0] != 0xF0
            || frame[1] != MANUFACTURER_ID
            || frame[2] != PRODUCT_ID
        {
            return Err("invalid Reflex message header");
        }
        match frame[3] >> 4 {
            0 => decode_active_setup_frame(frame)
                .map(|(channel, setup)| DecodedMessage::ActiveSetup { channel, setup }),
            1 => decode_register_setup_frame(frame).map(|(channel, register, setup)| {
                DecodedMessage::RegisterSetup { channel, register, setup }
            }),
            2 => decode_packed_parameter(frame).map(|(channel, parameter, value)| {
                DecodedMessage::PackedParameter { channel, parameter, value }
            }),
            3 => decode_request(frame).map(|(channel, request, argument)| {
                DecodedMessage::Request { channel, request, argument }
            }),
            4 => decode_all_registers_frame(frame)
                .map(|(channel, registers)| DecodedMessage::AllRegisters { channel, registers }),
            5 => decode_nibblized_parameter(frame).map(|(channel, parameter, value)| {
                DecodedMessage::NibblizedParameter { channel, parameter, value }
            }),
            6 => decode_task(frame).map(|(channel, task, argument)| DecodedMessage::Task {
                channel,
                task,
                argument,
            }),
            _ => Err("unsupported Reflex message type"),
        }
    }

    /// Validates a packed setup dump and its trailing checksum.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid length, checksum, or non-MIDI bytes.
    pub fn validate_setup_dump(frame: &[u8]) -> Result<&[u8], &'static str> {
        if frame.len() != 65 {
            return Err("Reflex setup dump has invalid length");
        }
        let (packed, checksum_byte) = frame.split_at(64);
        if checksum(packed) != checksum_byte[0] {
            return Err("Reflex setup checksum mismatch");
        }
        if packed.iter().any(|byte| *byte > 127) {
            return Err("Reflex setup contains non-MIDI data");
        }
        Ok(packed)
    }

    /// Computes the Reflex dump checksum.
    #[must_use]
    pub fn checksum(packed_data: &[u8]) -> u8 {
        packed_data.iter().fold(0_u8, |sum, byte| sum.wrapping_add(*byte)) & 0x7F
    }

    /// Nibblizes a 16-bit value from most-significant to least-significant nibble.
    #[must_use]
    pub const fn nibblize(value: u16) -> [u8; 4] {
        [
            ((value >> 12) & 0x0F) as u8,
            ((value >> 8) & 0x0F) as u8,
            ((value >> 4) & 0x0F) as u8,
            (value & 0x0F) as u8,
        ]
    }
}

/// Declarative effect family used for grouping and UI coloring.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectType {
    /// Reverb and ambience processors.
    Reverb,
    /// Delay processors.
    Delay,
    /// Modulation processors.
    Modulation,
    /// Cabinet and speaker simulation.
    Cabinet,
    /// Other MIDI-controlled devices.
    Other,
}

/// Semantic visual token shared by all device workspaces.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorToken {
    /// Reverb and ambience controls.
    Reverb,
    /// Modulation controls.
    Modulation,
    /// Cabinet/speaker controls.
    Cabinet,
    /// Neutral setup and navigation controls.
    Neutral,
    /// Hazardous or destructive action.
    Hazard,
}

/// State intensity used by every semantic color token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorIntensity {
    /// Control is unavailable.
    Dim,
    /// Control is available.
    Normal,
    /// Control is selected.
    Bright,
    /// Action requires attention; static marker remains mandatory.
    Hazard,
}

impl ColorIntensity {
    /// Returns a stable text suffix for non-color renderers.
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Dim => "(off)",
            Self::Normal => "",
            Self::Bright => "*",
            Self::Hazard => "(!)",
        }
    }
}

impl ColorToken {
    /// Complete semantic token set required by the default palette.
    pub const ALL: [Self; 5] =
        [Self::Reverb, Self::Modulation, Self::Cabinet, Self::Neutral, Self::Hazard];

    /// Stable serialized token name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Reverb => "reverb",
            Self::Modulation => "modulation",
            Self::Cabinet => "cabinet",
            Self::Neutral => "neutral",
            Self::Hazard => "hazard",
        }
    }

    /// Returns the canonical RGB palette value and a non-color text marker.
    #[must_use]
    pub const fn presentation(self) -> ((u8, u8, u8), &'static str) {
        match self {
            Self::Reverb => ((0, 128, 255), "[RVB]"),
            Self::Modulation => ((128, 0, 255), "[MOD]"),
            Self::Cabinet => ((255, 128, 0), "[CAB]"),
            Self::Neutral => ((128, 128, 128), "[---]"),
            Self::Hazard => ((255, 0, 0), "[!]"),
        }
    }

    /// Returns the nearest documented ANSI-16 color index.
    #[must_use]
    pub const fn ansi16(self) -> u8 {
        match self {
            Self::Reverb => 12,
            Self::Modulation => 13,
            Self::Cabinet => 11,
            Self::Neutral => 7,
            Self::Hazard => 9,
        }
    }

    /// Returns the nearest documented ANSI-256 color index.
    #[must_use]
    pub const fn ansi256(self) -> u8 {
        match self {
            Self::Reverb => 33,
            Self::Modulation => 93,
            Self::Cabinet => 208,
            Self::Neutral => gray_ansi256(),
            Self::Hazard => 196,
        }
    }
}

/// Maps a semantic token and intensity to the nearest documented Mk1 LED state.
#[must_use]
pub const fn launch_control_led_state(token: ColorToken, intensity: ColorIntensity) -> LedState {
    let color = match token {
        ColorToken::Reverb => LedColor::Amber,
        ColorToken::Modulation => LedColor::Green,
        ColorToken::Cabinet | ColorToken::Hazard => LedColor::Red,
        ColorToken::Neutral => LedColor::Yellow,
    };
    match intensity {
        ColorIntensity::Dim => LedState::new(LedColor::Off, 0, false),
        ColorIntensity::Normal => LedState::new(color, 64, false),
        ColorIntensity::Bright => LedState::new(color, 127, false),
        ColorIntensity::Hazard => LedState::new(LedColor::Red, 127, false),
    }
}

const fn gray_ansi256() -> u8 {
    8
}

/// Maps a profile effect family to its global semantic color token.
#[must_use]
pub const fn effect_color(effect: EffectType) -> ColorToken {
    match effect {
        EffectType::Reverb => ColorToken::Reverb,
        EffectType::Modulation => ColorToken::Modulation,
        EffectType::Cabinet => ColorToken::Cabinet,
        _ => ColorToken::Neutral,
    }
}

fn eventide_cc_control(label: &str, cc: u8) -> ControlDefinition {
    ControlDefinition { label: label.into(), cc: Some(cc), program: None, range: (0, 127) }
}

/// Built-in Eventide `MicroPitch` profile from the firmware 1.0+ quick-reference MIDI table.
#[must_use]
pub fn eventide_micropitch_profile() -> DeviceProfile {
    DeviceProfile {
        id: "eventide.micropitch".into(),
        version: 1,
        name: "Eventide MicroPitch".into(),
        effect_type: EffectType::Modulation,
        identity_probes: Vec::new(),
        provided_capabilities: vec![
            "pitch_shift".into(),
            "detune".into(),
            "delay".into(),
            "chorus".into(),
            "modulation".into(),
        ],
        capabilities: vec![CapabilityDefinition {
            id: "midi-cc-pc".into(),
            transport: ControlTransport::Midi,
            unsafe_on_connect: false,
        }],
        controls: vec![
            eventide_cc_control("Expression Pedal", 4),
            eventide_cc_control("TAP TEMPO", 9),
            eventide_cc_control("ACTIVE/BYPASS", 14),
            eventide_cc_control("FLEX", 15),
            eventide_cc_control("Mix", 20),
            eventide_cc_control("Pitch A", 21),
            eventide_cc_control("Pitch B", 22),
            eventide_cc_control("Depth", 23),
            eventide_cc_control("Rate/Sens", 24),
            eventide_cc_control("Pitch Mix", 25),
            eventide_cc_control("Tone", 26),
            eventide_cc_control("Delay A", 27),
            eventide_cc_control("Delay B", 28),
            eventide_cc_control("Mod", 29),
            eventide_cc_control("Feedback", 30),
            eventide_cc_control("Out Lvl", 31),
            ControlDefinition {
                label: "Preset 1".into(),
                cc: None,
                program: Some(1),
                range: (0, 0),
            },
        ],
        queries: Vec::new(),
        replies: Vec::new(),
        templates: Vec::new(),
        max_message_size: 1024,
        documented_features: vec![
            "MIDI Program Change loads presets 1–127".into(),
            "MIDI Program Change in Save Mode stores presets 1–127".into(),
            "MIDI over USB or EXP pedal jack".into(),
            "Catch Up knob function".into(),
        ],
    }
}

/// Conservative built-in profile for the M-VAVE IR Box MIDI endpoint.
///
/// The endpoint identity is known from host enumeration; no vendor-specific
/// commands are enabled until an authoritative MIDI map is available.
#[must_use]
pub fn mvave_ir_box_profile() -> DeviceProfile {
    DeviceProfile {
        id: "m-vave.ir-box".into(),
        version: 1,
        name: "M-VAVE IR Box".into(),
        effect_type: EffectType::Cabinet,
        identity_probes: Vec::new(),
        provided_capabilities: vec!["cabinet_ir".into(), "eq".into()],
        capabilities: vec![CapabilityDefinition {
            id: "midi-endpoint".into(),
            transport: ControlTransport::Midi,
            unsafe_on_connect: false,
        }],
        controls: Vec::new(),
        queries: Vec::new(),
        replies: Vec::new(),
        templates: Vec::new(),
        max_message_size: 1024,
        documented_features: vec![
            "32 IR/cabinet presets".into(),
            "IR module enable/disable".into(),
            "Custom IR import/export".into(),
            "9-band EQ enable/disable".into(),
            "EQ frequency and gain".into(),
            "Low Cut".into(),
            "Hi Cut".into(),
            "Output volume".into(),
            "IR volume".into(),
            "Bypass".into(),
            "Factory reset".into(),
        ],
    }
}

/// Conservative built-in Lexicon Reflex profile anchor.
#[must_use]
pub fn lexicon_reflex_profile() -> DeviceProfile {
    DeviceProfile {
        id: "lexicon.reflex".into(),
        version: 1,
        name: "Lexicon Reflex".into(),
        effect_type: EffectType::Reverb,
        identity_probes: Vec::new(),
        provided_capabilities: vec![
            "hall_reverb".into(),
            "room_reverb".into(),
            "plate_reverb".into(),
            "spring_reverb".into(),
            "gated_reverb".into(),
            "inverse_reverb".into(),
            "chorus".into(),
            "flanger".into(),
            "multi_tap_delay".into(),
            "resonator".into(),
        ],
        capabilities: vec![CapabilityDefinition {
            id: "midi-sysex".into(),
            transport: ControlTransport::Midi,
            unsafe_on_connect: false,
        }],
        controls: Vec::new(),
        queries: Vec::new(),
        replies: Vec::new(),
        templates: Vec::new(),
        max_message_size: 1024,
        documented_features: Vec::new(),
    }
}

/// Conservative built-in profile for the Valeton Arena2000 multi-effect.
///
/// The device is represented as a MIDI endpoint until an authoritative MIDI
/// implementation map is available. No vendor-specific controls are guessed
/// or enabled by this profile.
#[must_use]
pub fn valeton_arena2000_profile() -> DeviceProfile {
    // MIDI program numbers are 7-bit (0–127). The unit's 150 presets are
    // selected through its own bank/preset mapping, so expose every legal PC.
    let mut controls: Vec<ControlDefinition> = (0_u8..128)
        .map(|program| ControlDefinition {
            label: format!("Preset {}", program + 1),
            cc: None,
            program: Some(program),
            range: (0, 0),
        })
        .collect();
    controls.extend([
        arena2000_cc_control("Preset - / previous", 49),
        arena2000_cc_control("Drums on/off", 50),
        arena2000_cc_control("Tuner on/off", 51),
        arena2000_cc_control("Cab IR on/off", 52),
        arena2000_cc_control("Reverb on/off", 53),
    ]);

    DeviceProfile {
        id: "valeton.arena2000".into(),
        version: 1,
        name: "Valeton Arena2000".into(),
        effect_type: EffectType::Other,
        identity_probes: Vec::new(),
        provided_capabilities: vec![
            "gain".into(),
            "gate".into(),
            "cabinet_ir".into(),
            "eq".into(),
            "compression".into(),
            "drive".into(),
            "amp".into(),
            "modulation".into(),
            "delay".into(),
            "reverb".into(),
            "wah".into(),
            "noise_reduction".into(),
            "pitch_shift".into(),
            "looper".into(),
            "drum_machine".into(),
            "tuner".into(),
        ],
        capabilities: vec![CapabilityDefinition {
            id: "midi-endpoint".into(),
            transport: ControlTransport::Midi,
            unsafe_on_connect: false,
        }],
        controls,
        queries: Vec::new(),
        replies: Vec::new(),
        templates: Vec::new(),
        max_message_size: 1024,
        documented_features: vec!["Multi-effect preset processing".into(), "MIDI endpoint".into()],
    }
}

fn arena2000_cc_control(label: &str, cc: u8) -> ControlDefinition {
    ControlDefinition { label: label.into(), cc: Some(cc), program: None, range: (0, 127) }
}

/// Returns the built-in conservative device-profile catalog in stable order.
#[must_use]
pub fn builtin_profiles() -> Vec<DeviceProfile> {
    vec![
        lexicon_reflex_profile(),
        eventide_micropitch_profile(),
        mvave_ir_box_profile(),
        valeton_arena2000_profile(),
    ]
}

/// Looks up a built-in profile by its stable identifier.
#[must_use]
pub fn builtin_profile(id: &str) -> Option<DeviceProfile> {
    builtin_profiles().into_iter().find(|profile| profile.id == id)
}

/// Returns built-in profiles that provide a capability, in catalog order.
#[must_use]
pub fn builtin_capability_providers(capability: &str) -> Vec<DeviceProfile> {
    let capability = capability.trim();
    builtin_profiles()
        .into_iter()
        .filter(|profile| profile.provides_capability(capability))
        .collect()
}

/// Selects the deterministic built-in default provider for a capability.
#[must_use]
pub fn default_capability_provider(capability: &str) -> Option<DeviceProfile> {
    builtin_capability_providers(capability).into_iter().next()
}

/// Transport implementing a profile capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ControlTransport {
    /// MIDI 1.0 CC/PC/SysEx transport.
    Midi,
    /// USB vendor protocol requiring a reviewed compiled adapter.
    UsbVendor,
    /// External bridge process or service.
    ExternalBridge,
}

/// Supported controller identity for the Launch Control profile gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LaunchControlIdentity {
    /// Novation Launch Control XL first generation.
    Mk1,
    /// A known but unsupported second generation device.
    Mk2,
    /// A Launchpad-family device, not Launch Control XL.
    LaunchpadFamily,
    /// Any other or unrecognized controller.
    Unknown,
}

/// Launch Control XL Mk1 `SysEx` manufacturer header.
pub const LAUNCH_CONTROL_XL_SYSEX_HEADER: [u8; 6] = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x11];
/// Mk1 LED index for the Device button.
pub const LAUNCH_CONTROL_DEVICE_INDEX: u8 = 40;
/// Mk1 LED index for the Mute button.
pub const LAUNCH_CONTROL_MUTE_INDEX: u8 = 41;
/// Mk1 LED index for the Solo button.
pub const LAUNCH_CONTROL_SOLO_INDEX: u8 = 42;
/// Mk1 LED index for the Record Arm button.
pub const LAUNCH_CONTROL_RECORD_ARM_INDEX: u8 = 43;
/// Mk1 LED index for the Up button.
pub const LAUNCH_CONTROL_UP_INDEX: u8 = 44;
/// Mk1 LED index for the Down button.
pub const LAUNCH_CONTROL_DOWN_INDEX: u8 = 45;
/// Mk1 LED index for the Left button.
pub const LAUNCH_CONTROL_LEFT_INDEX: u8 = 46;
/// Mk1 LED index for the Right button.
pub const LAUNCH_CONTROL_RIGHT_INDEX: u8 = 47;

/// Returns the programmer-reference label for a Mk1 LED/control index.
#[must_use]
pub fn launch_control_index_label(index: u8) -> Option<String> {
    match index {
        0..=7 => Some(format!("Top knob {}", index + 1)),
        8..=15 => Some(format!("Middle knob {}", index - 7)),
        16..=23 => Some(format!("Bottom knob {}", index - 15)),
        24..=31 => Some(format!("Top channel button {}", index - 23)),
        32..=39 => Some(format!("Bottom channel button {}", index - 31)),
        40 => Some("Device".into()),
        41 => Some("Mute".into()),
        42 => Some("Solo".into()),
        43 => Some("Record Arm".into()),
        44 => Some("Up".into()),
        45 => Some("Down".into()),
        46 => Some("Left".into()),
        47 => Some("Right".into()),
        _ => None,
    }
}

/// MIDI message kind assigned to one Launch Control XL template control.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LaunchControlMessageKind {
    /// Control-change message.
    Cc,
    /// Note message.
    Note,
}

/// One imported or learned Launch Control XL template assignment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LaunchControlAssignment {
    /// Reference control/LED index (0–47).
    pub index: u8,
    /// Zero-based MIDI channel (0–15).
    pub channel: u8,
    /// CC or note number.
    pub number: u8,
    /// Message kind.
    pub kind: LaunchControlMessageKind,
}

/// User-template assignments for one Launch Control XL template.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LaunchControlTemplate {
    /// Template slot (0–15).
    pub template: u8,
    /// Assignments in deterministic declaration order.
    pub assignments: Vec<LaunchControlAssignment>,
}

impl LaunchControlTemplate {
    /// Validates template and assignment bounds and rejects duplicate controls.
    ///
    /// # Errors
    ///
    /// Returns an error when a slot, channel, control index, or message number is invalid,
    /// or when a control is assigned more than once.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.template >= 16 {
            return Err("Launch Control template is out of range");
        }
        let mut seen = std::collections::BTreeSet::new();
        for assignment in &self.assignments {
            if assignment.index >= 48 || assignment.channel >= 16 {
                return Err("Launch Control assignment is out of range");
            }
            if !seen.insert(assignment.index) {
                return Err("Launch Control assignment duplicates a control");
            }
        }
        Ok(())
    }

    /// Returns the assignment for a control index, if present.
    #[must_use]
    pub fn assignment(&self, index: u8) -> Option<&LaunchControlAssignment> {
        self.assignments.iter().find(|assignment| assignment.index == index)
    }

    /// Returns assignments sorted by reference index for deterministic rendering.
    #[must_use]
    pub fn assignments_by_index(&self) -> Vec<&LaunchControlAssignment> {
        let mut assignments: Vec<_> = self.assignments.iter().collect();
        assignments.sort_by_key(|assignment| assignment.index);
        assignments
    }
}

/// Index of a Launch Control XL control in the documented LED protocol.
#[must_use]
pub const fn launch_control_led_index(row: u8, column: u8) -> Option<u8> {
    if column >= 8 {
        return None;
    }
    match row {
        0..=3 => Some(row * 8 + column),
        _ => None,
    }
}

/// Encodes the documented Mk1 background LED `SysEx` message for one control.
/// `template` is 0–15 and `index` is the manual's control index.
#[must_use]
pub fn encode_launch_control_led(template: u8, index: u8, value: u8) -> Option<Vec<u8>> {
    (template < 16 && index < 48).then(|| {
        let mut bytes = LAUNCH_CONTROL_XL_SYSEX_HEADER.to_vec();
        bytes.extend_from_slice(&[0x78, template, index, value & 0x7F, 0xF7]);
        bytes
    })
}

/// Encodes the documented Mk1 template-selection `SysEx` message.
#[must_use]
pub fn encode_launch_control_template(template: u8) -> Option<[u8; 9]> {
    (template < 16).then_some([0xF0, 0x00, 0x20, 0x29, 0x02, 0x11, 0x77, template, 0xF7])
}

/// Encodes the documented Mk1 toggle-button state `SysEx` message.
#[must_use]
pub fn encode_launch_control_toggle(template: u8, index: u8, on: bool) -> Option<Vec<u8>> {
    (template < 16 && index < 24).then(|| {
        let mut bytes = LAUNCH_CONTROL_XL_SYSEX_HEADER.to_vec();
        bytes.extend_from_slice(&[0x7B, template, index, u8::from(on) * 0x7F, 0xF7]);
        bytes
    })
}

/// Encodes a traditional template-local Note LED update (`Note On`/`Note Off`).
#[must_use]
pub const fn encode_launch_control_note_led(
    channel: u8,
    note: u8,
    velocity: u8,
) -> Option<[u8; 3]> {
    if channel < 16 {
        Some([0x90 | channel, note, velocity & 0x7F])
    } else {
        None
    }
}

/// Encodes a traditional template-local CC LED update.
#[must_use]
pub const fn encode_launch_control_cc_led(
    channel: u8,
    controller: u8,
    velocity: u8,
) -> Option<[u8; 3]> {
    if channel < 16 {
        Some([0xB0 | channel, controller, velocity & 0x7F])
    } else {
        None
    }
}

/// USB identity tuple observed during endpoint discovery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UsbIdentity {
    /// USB vendor ID.
    pub vendor_id: u16,
    /// USB product ID.
    pub product_id: u16,
}

/// Observed Eventide `MicroPitch` USB identity.
pub const EVENTIDE_MICROPITCH_USB: UsbIdentity =
    UsbIdentity { vendor_id: 0x1B12, product_id: 0x003A };
/// Observed M-VAVE IR Box USB identity (Jieli/SINCO interface).
pub const MVAVE_IR_BOX_USB: UsbIdentity = UsbIdentity { vendor_id: 0x4353, product_id: 0x4B4D };

/// Builds the community-captured IR Box preset-recall `SysEx` (1..=32).
///
/// This command family is experimental and must be sent only after the
/// endpoint identity has been confirmed as [`MVAVE_IR_BOX_USB`].
///
/// # Errors
///
/// Returns an error when the preset is outside the device's 1–32 range.
pub fn mvave_ir_box_preset_sysex(preset: u8) -> Result<Vec<u8>, &'static str> {
    if !(1..=32).contains(&preset) {
        return Err("IR Box preset must be 1..=32");
    }
    let index = preset - 1;
    let (low, high) =
        if index < 14 { (0x34_u8 - index * 4, 0) } else { (0x7C_u8 - (index - 14) * 4, 3) };
    Ok(vec![
        0xF0, 0x00, 0x32, 0x09, 0x49, 0x00, 0x00, 0x00, 0x02, index, 0x00, 0x00, 0x00, 0x1E, 0x00,
        0x00, 0x00, index, low, high, 0xF7,
    ])
}

/// Builds the community-captured IR/EQ module toggle `SysEx`.
///
/// The device acknowledges these messages, but the capture source reports no
/// reliable state read-back; callers must therefore classify the result as
/// sent-unverified until an independent state query is established.
#[must_use]
pub fn mvave_ir_box_module_sysex(module: MvaveIrBoxModule, enabled: bool) -> Vec<u8> {
    let selector = match module {
        MvaveIrBoxModule::Ir => 0x11,
        MvaveIrBoxModule::Eq => 0x12,
    };
    let value = u8::from(enabled);
    let checksum = match module {
        MvaveIrBoxModule::Ir => {
            if enabled {
                [0x4E, 0x03]
            } else {
                [0x50, 0x03]
            }
        }
        MvaveIrBoxModule::Eq => {
            if enabled {
                [0x4C, 0x03]
            } else {
                [0x4E, 0x03]
            }
        }
    };
    vec![
        0xF0,
        0x00,
        0x32,
        0x09,
        0x49,
        0x00,
        0x00,
        0x40,
        0x02,
        selector,
        0x00,
        0x00,
        0x00,
        0x10,
        0x00,
        0x00,
        0x00,
        value,
        checksum[0],
        checksum[1],
        0xF7,
    ]
}

/// IR Box module selectors used by the experimental `SysEx` family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MvaveIrBoxModule {
    /// Impulse-response module.
    Ir,
    /// Equalizer module.
    Eq,
}
/// MIDISPORT 4x4 USB loader identity before firmware initialization.
pub const MIDISPORT_4X4_LOADER_USB: UsbIdentity =
    UsbIdentity { vendor_id: 0x0763, product_id: 0x1020 };
/// MIDISPORT 4x4 runtime identity after firmware initialization.
pub const MIDISPORT_4X4_RUNTIME_USB: UsbIdentity =
    UsbIdentity { vendor_id: 0x0763, product_id: 0x1021 };

/// Compares two USB identity tuples exactly.
#[must_use]
pub const fn usb_identity_matches(observed: UsbIdentity, expected: UsbIdentity) -> bool {
    observed.vendor_id == expected.vendor_id && observed.product_id == expected.product_id
}

/// Classifies Launch Control XL using exact Mk1 USB identity plus product name.
#[must_use]
pub fn classify_launch_control_usb(identity: UsbIdentity, name: &str) -> LaunchControlIdentity {
    let name_class = classify_launch_control(name);
    if identity.vendor_id == 0x1235 && identity.product_id == 0x0061 {
        if name_class == LaunchControlIdentity::LaunchpadFamily {
            LaunchControlIdentity::LaunchpadFamily
        } else {
            LaunchControlIdentity::Mk1
        }
    } else {
        name_class
    }
}

/// Classifies a backend-reported product name without fuzzy acceptance.
#[must_use]
pub fn classify_launch_control(name: &str) -> LaunchControlIdentity {
    let normalized = name.to_ascii_lowercase();
    if normalized.contains("launchpad") {
        LaunchControlIdentity::LaunchpadFamily
    } else if normalized.contains("launch control xl mk2")
        || normalized.contains("launch control xl 2")
    {
        LaunchControlIdentity::Mk2
    } else if normalized.contains("launch control xl") {
        LaunchControlIdentity::Mk1
    } else {
        LaunchControlIdentity::Unknown
    }
}

/// Abstract LED color token; concrete MIDI values require the verified Mk1 map.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LedColor {
    /// LED off.
    Off,
    /// Red.
    Red,
    /// Amber.
    Amber,
    /// Green.
    Green,
    /// Yellow.
    Yellow,
    /// Unknown or unsupported color.
    Unknown,
}

/// Desired LED state for one controller control.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LedState {
    /// Color token.
    pub color: LedColor,
    /// Intensity, 0–127.
    pub intensity: u8,
    /// Whether the LED should blink.
    pub blink: bool,
}

/// Encodes the Launch Control XL's documented red/green brightness byte.
#[must_use]
pub const fn launch_control_led_value(color: LedColor, intensity: u8, flags: u8) -> u8 {
    let level = match intensity {
        0 => 0,
        1..=42 => 1,
        43..=84 => 2,
        _ => 3,
    };
    let (red, green) = match color {
        LedColor::Red => (level, 0),
        LedColor::Green => (0, level),
        LedColor::Amber | LedColor::Yellow => (level, level),
        LedColor::Off | LedColor::Unknown => (0, 0),
    };
    ((green << 4) | red | (flags & 0x0C)) & 0x7F
}

impl LedState {
    /// Creates a validated LED state from a MIDI intensity value.
    #[must_use]
    pub const fn new(color: LedColor, intensity: u8, blink: bool) -> Self {
        Self { color, intensity, blink }
    }
}

/// Coalesces redundant LED updates and tracks desired/sent state.
#[derive(Clone, Debug, Default)]
pub struct LedCoalescer {
    desired: std::collections::BTreeMap<u8, LedState>,
    sent: std::collections::BTreeMap<u8, LedState>,
}

impl LedCoalescer {
    /// Sets desired state for a control.
    pub fn set_desired(&mut self, control: u8, state: LedState) {
        self.desired.insert(control, state);
    }
    /// Returns pending updates in deterministic control order and marks them sent.
    pub fn drain_pending(&mut self) -> Vec<(u8, LedState)> {
        let pending: Vec<_> = self
            .desired
            .iter()
            .filter(|(control, state)| self.sent.get(control) != Some(state))
            .map(|(control, state)| (*control, *state))
            .collect();
        for (control, state) in &pending {
            self.sent.insert(*control, *state);
        }
        pending
    }

    /// Drains at most `limit` pending updates, retaining the remainder.
    pub fn drain_pending_limited(&mut self, limit: usize) -> Vec<(u8, LedState)> {
        if limit == 0 {
            return Vec::new();
        }
        let pending: Vec<_> = self
            .desired
            .iter()
            .filter(|(control, state)| self.sent.get(control) != Some(state))
            .take(limit)
            .map(|(control, state)| (*control, *state))
            .collect();
        for (control, state) in &pending {
            self.sent.insert(*control, *state);
        }
        pending
    }

    /// Invalidates the last-sent cache after reconnect, page change, or scene change.
    /// The next drain therefore emits a complete deterministic render of desired state.
    pub fn request_full_resync(&mut self) {
        self.sent.clear();
    }

    /// Returns the number of controls with desired LED state.
    #[must_use]
    pub fn desired_len(&self) -> usize {
        self.desired.len()
    }
}

/// A named capability and its implementing transport.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CapabilityDefinition {
    /// Stable capability identifier.
    pub id: String,
    /// Implementing transport.
    pub transport: ControlTransport,
    /// Whether enabling it may write during connect.
    pub unsafe_on_connect: bool,
}

/// A documented MIDI control exposed by a device profile.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ControlDefinition {
    /// Manual-matching label.
    pub label: String,
    /// MIDI CC number, when applicable.
    pub cc: Option<u8>,
    /// MIDI program number, when applicable.
    pub program: Option<u8>,
    /// Safe inclusive value range.
    pub range: (u16, u16),
}

/// A declarative identity probe: masked bytes must match at the same offset.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IdentityProbe {
    /// Byte offset in the observed identity response.
    pub offset: usize,
    /// Expected bytes.
    pub value: Vec<u8>,
    /// Match mask; a set bit participates in comparison.
    pub mask: Vec<u8>,
}

/// Matches all identity probes against one captured response.
#[must_use]
pub fn match_identity_probes(probes: &[IdentityProbe], response: &[u8]) -> bool {
    probes.iter().all(|probe| {
        probe.value.len() == probe.mask.len()
            && probe.offset.checked_add(probe.value.len()).is_some_and(|end| end <= response.len())
            && response[probe.offset..probe.offset + probe.value.len()]
                .iter()
                .zip(&probe.value)
                .zip(&probe.mask)
                .all(|((actual, expected), mask)| (actual & mask) == (expected & mask))
    })
}

/// Declarative read query paired with a named reply definition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QueryDefinition {
    /// Stable query identifier.
    pub id: String,
    /// Serialized request payload.
    pub request: Vec<u8>,
    /// Optional reusable profile template used to render the request.
    #[serde(default)]
    pub template_id: Option<String>,
    /// Reply definition used for correlation/decoding.
    pub reply_id: String,
}

/// Declarative reply matcher for a query response.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplyDefinition {
    /// Stable reply identifier.
    pub id: String,
    /// Masked prefix/value matcher.
    pub value: Vec<u8>,
    /// Match mask; set bits participate in comparison.
    pub mask: Vec<u8>,
}

/// Validates query/reply references and bounded MIDI-safe payloads.
///
/// # Errors
///
/// Returns an error when identifiers, references, payload sizes, or MIDI data
/// bytes are invalid.
pub fn validate_query_definitions(
    queries: &[QueryDefinition],
    replies: &[ReplyDefinition],
    max_message_size: usize,
) -> Result<(), &'static str> {
    if max_message_size == 0 {
        return Err("maximum message size must be nonzero");
    }
    for reply in replies {
        if reply.id.trim().is_empty() || reply.value.len() != reply.mask.len() {
            return Err("reply definition is invalid");
        }
        if reply.value.len() > max_message_size
            || reply.value.iter().chain(&reply.mask).any(|byte| *byte > 127)
        {
            return Err("reply definition exceeds MIDI/message bounds");
        }
    }
    for query in queries {
        if query.id.trim().is_empty()
            || query.reply_id.trim().is_empty()
            || query.request.is_empty()
            || query.request.len() > max_message_size
            || query.request.iter().any(|byte| *byte > 127)
            || !replies.iter().any(|reply| reply.id == query.reply_id)
        {
            return Err("query definition is invalid or references a missing reply");
        }
    }
    Ok(())
}

/// Versioned declarative device profile.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeviceProfile {
    /// Stable profile identifier.
    pub id: String,
    /// Monotonically increasing declarative profile version.
    #[serde(default = "default_profile_version")]
    pub version: u32,
    /// Human-readable manufacturer/model name.
    pub name: String,
    /// Effect family for UI grouping.
    pub effect_type: EffectType,
    /// Effect blocks/services provided by this device, beyond its primary family.
    #[serde(default)]
    pub provided_capabilities: Vec<String>,
    /// Identity probes used to select this profile.
    #[serde(default)]
    pub identity_probes: Vec<IdentityProbe>,
    /// Declared capabilities and transports.
    pub capabilities: Vec<CapabilityDefinition>,
    /// Controls documented by the manufacturer.
    pub controls: Vec<ControlDefinition>,
    /// Read queries and correlated replies.
    #[serde(default)]
    pub queries: Vec<QueryDefinition>,
    /// Reply matchers referenced by queries.
    #[serde(default)]
    pub replies: Vec<ReplyDefinition>,
    /// Declarative `SysEx` payload templates owned by this profile.
    #[serde(default)]
    pub templates: Vec<SysexTemplate>,
    /// Maximum encoded message size accepted by this profile.
    #[serde(default = "default_max_message_size")]
    pub max_message_size: usize,
    /// Documented features without an asserted wire-level MIDI mapping.
    #[serde(default)]
    pub documented_features: Vec<String>,
}

const fn default_profile_version() -> u32 {
    1
}

const fn default_max_message_size() -> usize {
    1024
}

/// Validated effective profile catalog containing one highest version per identifier.
#[derive(Clone, Debug)]
pub struct ProfileCatalog {
    profiles: std::collections::BTreeMap<String, DeviceProfile>,
}

impl ProfileCatalog {
    /// Builds the catalog from compiled built-ins and user profiles.
    ///
    /// # Errors
    ///
    /// Rejects invalid profiles, duplicate ID/version pairs, user claims on the reserved
    /// Reflex identity/aliases, and user replacements without a strictly newer version.
    pub fn load(user_profiles: Vec<DeviceProfile>) -> Result<Self, &'static str> {
        let builtins = builtin_profiles();
        let mut profiles = std::collections::BTreeMap::new();
        for profile in builtins {
            profile.validate()?;
            profiles.insert(profile.id.clone(), profile);
        }
        let mut seen = std::collections::BTreeSet::new();
        for profile in user_profiles {
            profile.validate()?;
            if reserved_reflex_identity(&profile.id) {
                return Err(
                    "Lexicon Reflex profile identity is reserved for the compiled built-in",
                );
            }
            if !seen.insert((profile.id.clone(), profile.version)) {
                return Err("duplicate user profile ID and version");
            }
            if let Some(current) = profiles.get(&profile.id) {
                if profile.version <= current.version {
                    return Err("user profile replacement requires a newer version");
                }
            }
            profiles.insert(profile.id.clone(), profile);
        }
        Ok(Self { profiles })
    }

    /// Looks up the effective highest-version profile.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&DeviceProfile> {
        self.profiles.get(id)
    }

    /// Returns effective profiles in stable identifier order.
    #[must_use]
    pub fn profiles(&self) -> Vec<&DeviceProfile> {
        self.profiles.values().collect()
    }
}

fn reserved_reflex_identity(id: &str) -> bool {
    let normalized: String =
        id.chars().filter(char::is_ascii_alphanumeric).flat_map(char::to_lowercase).collect();
    matches!(normalized.as_str(), "lexiconreflex" | "lexiconreflexrev1" | "reflexrev1")
}

/// Bounded declarative `SysEx` template segment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TemplateSegment {
    /// Literal MIDI data bytes (0..=127).
    Literal(Vec<u8>),
    /// A 7-bit parameter value by index.
    Parameter(usize),
    /// A checked parameter expression that renders one MIDI data byte.
    Expression(String),
}

/// A non-executable, bounded `SysEx` payload template.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SysexTemplate {
    /// Optional stable name for reuse by declarative queries or controls.
    #[serde(default)]
    pub id: Option<String>,
    /// Ordered payload segments.
    pub segments: Vec<TemplateSegment>,
    /// Maximum rendered payload length.
    pub max_bytes: usize,
}

/// Parses a signed decimal or hexadecimal integer literal without allocation.
///
/// # Errors
///
/// Returns an error for empty, malformed, or overflowing literals.
pub fn parse_integer_literal(input: &str) -> Result<i64, &'static str> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("integer literal is empty");
    }
    let (negative, digits) =
        trimmed.strip_prefix('-').map_or((false, trimmed), |rest| (true, rest));
    let (radix, digits) = digits.strip_prefix("0x").map_or((10, digits), |rest| (16, rest));
    if digits.is_empty() {
        return Err("integer literal has no digits");
    }
    let magnitude = i64::from_str_radix(digits, radix)
        .map_err(|_| "integer literal is invalid or overflowing")?;
    if negative {
        magnitude.checked_neg().ok_or("integer literal is overflowing")
    } else {
        Ok(magnitude)
    }
}

/// Evaluates one checked binary expression operation.
///
/// # Errors
///
/// Returns an error for overflow, division/modulo by zero, or invalid shifts.
pub fn eval_binary(lhs: i64, operator: &str, rhs: i64) -> Result<i64, &'static str> {
    match operator {
        "+" => lhs.checked_add(rhs),
        "-" => lhs.checked_sub(rhs),
        "*" => lhs.checked_mul(rhs),
        "/" => lhs.checked_div(rhs),
        "%" => lhs.checked_rem(rhs),
        "&" => Some(lhs & rhs),
        "|" => Some(lhs | rhs),
        "^" => Some(lhs ^ rhs),
        "<<" => u32::try_from(rhs).ok().and_then(|shift| lhs.checked_shl(shift)),
        ">>" => u32::try_from(rhs).ok().and_then(|shift| lhs.checked_shr(shift)),
        "==" => Some(i64::from(lhs == rhs)),
        "!=" => Some(i64::from(lhs != rhs)),
        "<" => Some(i64::from(lhs < rhs)),
        "<=" => Some(i64::from(lhs <= rhs)),
        ">" => Some(i64::from(lhs > rhs)),
        ">=" => Some(i64::from(lhs >= rhs)),
        _ => return Err("unsupported expression operator"),
    }
    .ok_or("expression operation overflow or invalid divisor/shift")
}

/// Parses and evaluates a deliberately small, non-recursive binary expression.
///
/// Whitespace-separated operands/operators keep the grammar deterministic and
/// prevent arbitrary code execution in profile files.
///
/// # Errors
///
/// Returns an error for malformed operands, unsupported operators, or extra
/// tokens.
pub fn parse_binary_expression(input: &str) -> Result<i64, &'static str> {
    let mut parts = input.split_whitespace();
    let lhs = parse_integer_literal(parts.next().ok_or("missing left operand")?)?;
    let operator = parts.next().ok_or("missing operator")?;
    let rhs = parse_integer_literal(parts.next().ok_or("missing right operand")?)?;
    if parts.next().is_some() {
        return Err("unexpected expression tokens");
    }
    eval_binary(lhs, operator, rhs)
}

/// Evaluates a bounded parameter-aware binary expression.
///
/// Operands may reference parameters using `$N` notation. The function accepts
/// exactly one binary operator and never executes profile-provided code.
///
/// # Errors
///
/// Returns an error for malformed or out-of-range references and checked
/// arithmetic failures.
pub fn parse_parameter_expression(input: &str, parameters: &[i64]) -> Result<i64, &'static str> {
    let mut parts = input.split_whitespace();
    let resolve = |token: &str| {
        if let Some(index) = token.strip_prefix('$') {
            let index = index.parse::<usize>().map_err(|_| "parameter reference is invalid")?;
            parameters.get(index).copied().ok_or("parameter reference is out of range")
        } else {
            parse_integer_literal(token)
        }
    };
    let lhs = resolve(parts.next().ok_or("missing left operand")?)?;
    let operator = parts.next().ok_or("missing operator")?;
    let rhs = resolve(parts.next().ok_or("missing right operand")?)?;
    if parts.next().is_some() {
        return Err("unexpected expression tokens");
    }
    eval_binary(lhs, operator, rhs)
}

/// Parses a bounded arithmetic expression with parameter references and
/// parentheses. This is the safe, composable entry point used by richer
/// template parsers; it never evaluates host-language code.
///
/// # Errors
///
/// Returns an error for malformed expressions, missing parameters, checked
/// arithmetic failures, or expressions exceeding the node limit.
#[allow(clippy::too_many_lines)]
pub fn parse_parameter_expression_full(
    input: &str,
    parameters: &[i64],
) -> Result<i64, &'static str> {
    struct Parser<'a> {
        chars: Vec<char>,
        pos: usize,
        parameters: &'a [i64],
        nodes: u16,
    }
    impl Parser<'_> {
        fn skip(&mut self) {
            while self.chars.get(self.pos).is_some_and(|c| c.is_whitespace()) {
                self.pos += 1;
            }
        }
        fn eat(&mut self, c: char) -> bool {
            self.skip();
            if self.chars.get(self.pos) == Some(&c) {
                self.pos += 1;
                true
            } else {
                false
            }
        }
        fn bump(&mut self) -> Result<(), &'static str> {
            self.nodes = self.nodes.checked_add(1).ok_or("expression is too complex")?;
            if self.nodes > 256 {
                Err("expression node limit exceeded")
            } else {
                Ok(())
            }
        }
        fn atom(&mut self) -> Result<i64, &'static str> {
            self.bump()?;
            self.skip();
            if self.eat('(') {
                let v = self.expr(0)?;
                if !self.eat(')') {
                    return Err("closing parenthesis is missing");
                }
                return Ok(v);
            }
            let start = self.pos;
            while self
                .chars
                .get(self.pos)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$' || *c == '-')
            {
                self.pos += 1;
            }
            if start == self.pos {
                return Err("expression operand is missing");
            }
            let token: String = self.chars[start..self.pos].iter().collect();
            if token == "-" {
                return Err("expression operand is missing");
            }
            if self.eat('(') {
                let mut args = Vec::new();
                if !self.eat(')') {
                    loop {
                        args.push(self.expr(0)?);
                        if args.len() > 16 {
                            return Err("too many function arguments");
                        }
                        if self.eat(')') {
                            break;
                        }
                        if !self.eat(',') {
                            return Err("function argument separator is missing");
                        }
                    }
                }
                return eval_function(&token, &args);
            }
            if let Some(index) = token.strip_prefix('$') {
                return index
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| self.parameters.get(i).copied())
                    .ok_or("parameter reference is invalid or out of range");
            }
            parse_integer_literal(&token)
        }
        fn expr(&mut self, min_prec: u8) -> Result<i64, &'static str> {
            let mut lhs = self.atom()?;
            loop {
                self.skip();
                let (op, prec) = if self.chars[self.pos..].starts_with(&['=', '=']) {
                    ("==", 5)
                } else if self.chars[self.pos..].starts_with(&['!', '=']) {
                    ("!=", 5)
                } else if self.chars[self.pos..].starts_with(&['<', '=']) {
                    ("<=", 5)
                } else if self.chars[self.pos..].starts_with(&['>', '=']) {
                    (">=", 5)
                } else if self.chars[self.pos..].starts_with(&['<', '<']) {
                    ("<<", 4)
                } else if self.chars[self.pos..].starts_with(&['>', '>']) {
                    (">>", 4)
                } else {
                    match self.chars.get(self.pos) {
                        Some('<') => ("<", 5),
                        Some('>') => (">", 5),
                        Some('+') => ("+", 5),
                        Some('-') => ("-", 5),
                        Some('*') => ("*", 6),
                        Some('/') => ("/", 6),
                        Some('%') => ("%", 6),
                        Some('&') => ("&", 3),
                        Some('^') => ("^", 2),
                        Some('|') => ("|", 1),
                        _ => break,
                    }
                };
                if prec < min_prec {
                    break;
                }
                self.pos += op.len();
                let rhs = self.expr(prec + 1)?;
                lhs = eval_binary(lhs, op, rhs)?;
            }
            if min_prec == 0 && self.eat('?') {
                let when_true = self.expr(0)?;
                if !self.eat(':') {
                    return Err("ternary separator is missing");
                }
                let when_false = self.expr(0)?;
                lhs = if lhs != 0 { when_true } else { when_false };
            }
            Ok(lhs)
        }
    }
    let mut parser = Parser { chars: input.chars().collect(), pos: 0, parameters, nodes: 0 };
    let value = parser.expr(0)?;
    parser.skip();
    if parser.pos != parser.chars.len() {
        return Err("unexpected expression tokens");
    }
    Ok(value)
}

/// Parses an approved function call with integer literal arguments.
///
/// # Errors
///
/// Returns an error for malformed syntax, excessive arguments, invalid
/// literals, or functions outside the approved set.
pub fn parse_function_expression(input: &str) -> Result<i64, &'static str> {
    let open = input.find('(').ok_or("missing function argument list")?;
    if !input.ends_with(')') || open == 0 {
        return Err("malformed function expression");
    }
    let name = input[..open].trim();
    let body = &input[open + 1..input.len() - 1];
    let mut args = Vec::new();
    if !body.trim().is_empty() {
        for token in body.split(',') {
            args.push(parse_integer_literal(token.trim())?);
            if args.len() > 16 {
                return Err("too many function arguments");
            }
        }
    }
    eval_function(name, &args)
}

/// Parses an approved function call whose arguments may reference `$N`
/// parameters.
///
/// # Errors
///
/// Returns an error for malformed syntax, invalid references, excessive
/// arguments, or an unapproved function.
pub fn parse_parameter_function_expression(
    input: &str,
    parameters: &[i64],
) -> Result<i64, &'static str> {
    let open = input.find('(').ok_or("missing function argument list")?;
    if !input.ends_with(')') || open == 0 {
        return Err("malformed function expression");
    }
    let name = input[..open].trim();
    let body = &input[open + 1..input.len() - 1];
    let mut args = Vec::new();
    for token in body.split(',').filter(|token| !token.trim().is_empty()) {
        let token = token.trim();
        let value = token.strip_prefix('$').map_or_else(
            || parse_integer_literal(token),
            |index| {
                index.parse::<usize>().map_err(|_| "parameter reference is invalid").and_then(
                    |index| {
                        parameters.get(index).copied().ok_or("parameter reference is out of range")
                    },
                )
            },
        )?;
        args.push(value);
        if args.len() > 16 {
            return Err("too many function arguments");
        }
    }
    eval_function(name, &args)
}

/// Evaluates a parameter-aware function while charging a bounded budget.
///
/// # Errors
///
/// Returns parsing, function, or budget errors.
pub fn parse_parameter_function_expression_budgeted(
    input: &str,
    parameters: &[i64],
    budget: &mut EvaluationBudget,
) -> Result<i64, &'static str> {
    budget.consume(1)?;
    parse_parameter_function_expression(input, parameters)
}

/// Evaluates one approved, bounded expression function.
///
/// # Errors
///
/// Returns an error for unknown functions, wrong arity, or out-of-range values.
pub fn eval_function(name: &str, args: &[i64]) -> Result<i64, &'static str> {
    match name {
        "min" if args.len() == 2 => Ok(args[0].min(args[1])),
        "max" if args.len() == 2 => Ok(args[0].max(args[1])),
        "clamp" if args.len() == 3 => Ok(args[0].clamp(args[1], args[2])),
        "sum" if !args.is_empty() => args
            .iter()
            .try_fold(0_i64, |total, value| total.checked_add(*value))
            .ok_or("function overflow"),
        "xor" if !args.is_empty() => Ok(args.iter().fold(0_i64, |total, value| total ^ value)),
        "hi7" if args.len() == 1 => u16::try_from(args[0])
            .ok()
            .map(|value| i64::from((value >> 7) & 0x7F))
            .ok_or("hi7 input out of range"),
        "lo7" if args.len() == 1 => u16::try_from(args[0])
            .ok()
            .map(|value| i64::from(value & 0x7F))
            .ok_or("lo7 input out of range"),
        "lookup" if args.len() >= 2 => {
            let index = usize::try_from(args[0]).map_err(|_| "lookup index is invalid")?;
            let table_index = index.checked_add(1).ok_or("lookup index is out of range")?;
            args.get(table_index).copied().ok_or("lookup index is out of range")
        }
        "min" | "max" | "clamp" | "sum" | "xor" | "hi7" | "lo7" => {
            Err("function arity or input is invalid")
        }
        _ => Err("unknown expression function"),
    }
}

/// Performs a bounded table lookup for the approved `lookup` function.
///
/// # Errors
///
/// Returns an error for empty/oversized tables or an index outside the table.
pub fn eval_lookup(index: i64, table: &[i64]) -> Result<i64, &'static str> {
    if table.is_empty() || table.len() > 1024 {
        return Err("lookup table size is invalid");
    }
    let index = usize::try_from(index).map_err(|_| "lookup index is invalid")?;
    table.get(index).copied().ok_or("lookup index is out of range")
}

/// Parses whitespace-separated hexadecimal `SysEx` bytes.
///
/// # Errors
///
/// Returns an error for malformed hex, empty input, or invalid `SysEx` framing/data bytes.
pub fn parse_sysex_hex(input: &str) -> Result<Vec<u8>, &'static str> {
    let tokens: Vec<_> = input.split_whitespace().collect();
    if tokens.is_empty() {
        return Err("SysEx input is empty");
    }
    let mut bytes = Vec::with_capacity(tokens.len());
    for token in tokens {
        if token.len() != 2 {
            return Err("SysEx byte must be two hex digits");
        }
        bytes.push(u8::from_str_radix(token, 16).map_err(|_| "SysEx byte is not hexadecimal")?);
    }
    if bytes.first() == Some(&0xF0) {
        if bytes.last() != Some(&0xF7) {
            return Err("SysEx framing terminator is missing");
        }
        if bytes[1..bytes.len() - 1].iter().any(|byte| *byte > 0x7F) {
            return Err("SysEx data byte is out of range");
        }
    }
    Ok(bytes)
}

/// Matches a captured `SysEx` payload against a bounded value/mask pattern.
/// A mask bit of one requires the corresponding value bit to match.
#[must_use]
pub fn sysex_mask_matches(payload: &[u8], value: &[u8], mask: &[u8]) -> bool {
    payload.len() == value.len()
        && value.len() == mask.len()
        && payload
            .iter()
            .zip(value)
            .zip(mask)
            .all(|((actual, expected), mask)| (actual & mask) == (expected & mask))
}

/// Deterministic expression evaluation budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvaluationBudget {
    remaining: u32,
}

impl Default for EvaluationBudget {
    fn default() -> Self {
        Self::new()
    }
}

impl EvaluationBudget {
    /// Maximum permitted operations for one evaluation.
    pub const MAX: u32 = 10_000;
    /// Creates a full budget.
    #[must_use]
    pub const fn new() -> Self {
        Self { remaining: Self::MAX }
    }
    /// Consumes operations.
    ///
    /// # Errors
    ///
    /// Returns an error when the budget would be exceeded.
    pub const fn consume(&mut self, operations: u32) -> Result<(), &'static str> {
        if operations > self.remaining {
            Err("expression evaluation budget exceeded")
        } else {
            self.remaining -= operations;
            Ok(())
        }
    }
    /// Returns remaining operations.
    #[must_use]
    pub const fn remaining(self) -> u32 {
        self.remaining
    }
}

impl SysexTemplate {
    /// Renders a payload from 7-bit parameters.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid literal bytes, missing parameters, or size overflow.
    pub fn render(&self, parameters: &[u8]) -> Result<Vec<u8>, &'static str> {
        let mut output = Vec::new();
        for segment in &self.segments {
            match segment {
                TemplateSegment::Literal(bytes) => {
                    if bytes.iter().any(|byte| *byte > 127) {
                        return Err("literal is not MIDI data");
                    }
                    output.extend(bytes);
                }
                TemplateSegment::Parameter(index) => {
                    let value = *parameters.get(*index).ok_or("parameter missing")?;
                    if value > 127 {
                        return Err("parameter is not MIDI data");
                    }
                    output.push(value);
                }
                TemplateSegment::Expression(expression) => {
                    let values: Vec<i64> =
                        parameters.iter().map(|value| i64::from(*value)).collect();
                    let value = parse_parameter_expression_full(expression, &values)
                        .or_else(|_| parse_parameter_expression(expression, &values))
                        .or_else(|_| parse_parameter_function_expression(expression, &values))?;
                    let value =
                        u8::try_from(value).map_err(|_| "expression is outside MIDI range")?;
                    if value > 127 {
                        return Err("expression is not MIDI data");
                    }
                    output.push(value);
                }
            }
            if output.len() > self.max_bytes {
                return Err("template exceeds maximum size");
            }
        }
        Ok(output)
    }

    /// Renders a template while charging one budget operation per segment.
    ///
    /// # Errors
    ///
    /// Returns the same validation errors as [`Self::render`] or a budget
    /// exhaustion error.
    pub fn render_with_budget(
        &self,
        parameters: &[u8],
        budget: &mut EvaluationBudget,
    ) -> Result<Vec<u8>, &'static str> {
        budget.consume(u32::try_from(self.segments.len()).map_err(|_| "template is too large")?)?;
        self.render(parameters)
    }
}

impl DeviceProfile {
    /// Returns whether this profile provides the named effect/block capability.
    #[must_use]
    pub fn provides_capability(&self, capability: &str) -> bool {
        let capability = capability.trim();
        !capability.is_empty()
            && self
                .provided_capabilities
                .iter()
                .any(|provided| provided.eq_ignore_ascii_case(capability))
    }

    /// Renders the request payload for a named declarative query.
    ///
    /// Queries without a template retain their validated raw request bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the query/template is missing or rendering exceeds profile bounds.
    pub fn render_query_request(
        &self,
        query_id: &str,
        parameters: &[u8],
    ) -> Result<Vec<u8>, &'static str> {
        let query =
            self.queries.iter().find(|query| query.id == query_id).ok_or("query not found")?;
        let request = match query.template_id.as_deref() {
            Some(template_id) => self
                .templates
                .iter()
                .find(|template| template.id.as_deref() == Some(template_id))
                .ok_or("query references a missing template")?
                .render(parameters)?,
            None => query.request.clone(),
        };
        if request.len() > self.max_message_size || request.iter().any(|byte| *byte > 127) {
            return Err("query request exceeds profile bounds");
        }
        Ok(request)
    }

    /// Renders a validated channel voice message for a documented control.
    ///
    /// This produces bytes only; transmission remains owned by the caller and its safety policy.
    ///
    /// # Errors
    ///
    /// Returns an error when the control is unknown, the channel is invalid, or the value is
    /// outside the control's declared range.
    pub fn render_control_message(
        &self,
        label: &str,
        channel: u8,
        value: u16,
    ) -> Result<Vec<u8>, &'static str> {
        if !(1..=16).contains(&channel) {
            return Err("MIDI channel must be between 1 and 16");
        }
        let control = self
            .controls
            .iter()
            .find(|control| control.label == label)
            .ok_or("control not found")?;
        if value < control.range.0 || value > control.range.1 || value > 127 {
            return Err("control value is outside its declared MIDI range");
        }
        let value = u8::try_from(value).map_err(|_| "control value is invalid")?;
        let status = channel - 1;
        if let Some(cc) = control.cc {
            return Ok(vec![0xB0 | status, cc, value]);
        }
        if let Some(program) = control.program {
            return Ok(vec![0xC0 | status, program]);
        }
        Err("control has no supported MIDI mapping")
    }

    /// Validates profile identity, ranges, and mutually exclusive mappings.
    ///
    /// # Errors
    ///
    /// Returns a static validation message for malformed definitions.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.trim().is_empty() || self.name.trim().is_empty() {
            return Err("profile identity must not be empty");
        }
        if self.capabilities.iter().any(|capability| capability.id.trim().is_empty()) {
            return Err("capability ID must not be empty");
        }
        for (index, capability) in self.provided_capabilities.iter().enumerate() {
            if capability.trim().is_empty() {
                return Err("provided capability must not be empty");
            }
            if self.provided_capabilities[..index].iter().any(|prior| prior == capability) {
                return Err("duplicate provided capability");
            }
        }
        for (index, capability) in self.capabilities.iter().enumerate() {
            if self.capabilities[..index].iter().any(|prior| prior.id == capability.id) {
                return Err("duplicate capability ID");
            }
        }
        for control in &self.controls {
            if control.cc.is_some() == control.program.is_some() {
                return Err("control must map to exactly one CC or program");
            }
            if control.range.0 > control.range.1 || control.range.1 > 16_383 {
                return Err("control range is invalid");
            }
            if control.cc.is_some_and(|cc| cc > 127)
                || control.program.is_some_and(|program| program > 127)
            {
                return Err("MIDI control number is out of range");
            }
        }
        for (index, control) in self.controls.iter().enumerate() {
            if self.controls[..index].iter().any(|prior| {
                prior.label == control.label
                    || (control.cc.is_some() && prior.cc == control.cc)
                    || (control.program.is_some() && prior.program == control.program)
            }) {
                return Err("duplicate control definition");
            }
        }
        if self.identity_probes.iter().any(|probe| probe.value.len() != probe.mask.len()) {
            return Err("identity probe mask is invalid");
        }
        for (index, query) in self.queries.iter().enumerate() {
            if self.queries[..index].iter().any(|prior| prior.id == query.id) {
                return Err("duplicate query ID");
            }
        }
        for (index, reply) in self.replies.iter().enumerate() {
            if self.replies[..index].iter().any(|prior| prior.id == reply.id) {
                return Err("duplicate reply ID");
            }
        }
        validate_query_definitions(&self.queries, &self.replies, self.max_message_size)?;
        for (index, template) in self.templates.iter().enumerate() {
            if template.max_bytes == 0 || template.max_bytes > self.max_message_size {
                return Err("profile template size is invalid");
            }
            if template.id.as_ref().is_some_and(|id| {
                id.trim().is_empty()
                    || self.templates[..index].iter().any(|prior| prior.id.as_ref() == Some(id))
            }) {
                return Err("template ID is empty or duplicated");
            }
        }
        for query in &self.queries {
            if query.template_id.as_ref().is_some_and(|id| {
                !self.templates.iter().any(|template| template.id.as_ref() == Some(id))
            }) {
                return Err("query references a missing template");
            }
        }
        if self
            .templates
            .iter()
            .any(|template| template.max_bytes == 0 || template.max_bytes > self.max_message_size)
        {
            return Err("profile template size is invalid");
        }
        Ok(())
    }
}

/// Stable observed identity for a backend endpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedEndpoint {
    /// Backend-reported name.
    pub name: String,
    /// Direction label.
    pub direction: String,
    /// USB vendor/product pair when available.
    pub vid_pid: Option<(u16, u16)>,
    /// Device serial when available and permitted.
    pub serial: Option<String>,
    /// Interface number when available.
    pub interface: Option<u8>,
    /// Stable physical path when available.
    pub physical_path: Option<String>,
}

/// Persisted alias selector; ephemeral backend indexes are intentionally absent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AliasSelector {
    /// User-facing alias.
    pub alias: String,
    /// Preferred serial identity.
    pub serial: Option<String>,
    /// Preferred VID/PID identity.
    pub vid_pid: Option<(u16, u16)>,
    /// Preferred interface.
    pub interface: Option<u8>,
    /// Approved name pattern (literal substring).
    pub name_pattern: Option<String>,
}

/// Persistent alias registry; only stable identity fields are serialized.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AliasRegistry {
    /// User-defined selectors.
    pub aliases: Vec<AliasSelector>,
}

impl AliasRegistry {
    /// Validates stable alias identifiers before persistence or resolution.
    ///
    /// # Errors
    ///
    /// Returns an error for empty, whitespace-padded, or duplicate aliases.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = std::collections::BTreeSet::new();
        for selector in &self.aliases {
            if selector.alias.trim().is_empty() || selector.alias != selector.alias.trim() {
                return Err("alias must be non-empty and have no surrounding whitespace".into());
            }
            if !seen.insert(&selector.alias) {
                return Err(format!("duplicate alias '{}'", selector.alias));
            }
        }
        Ok(())
    }

    /// Loads a registry from JSON, rejecting malformed or unreadable files.
    ///
    /// # Errors
    ///
    /// Returns an I/O or JSON decoding error.
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| error.to_string())?;
        let registry: Self = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        registry.validate()?;
        Ok(registry)
    }

    /// Atomically saves a registry, retaining a single backup when replacing a file.
    ///
    /// # Errors
    ///
    /// Returns an I/O or JSON encoding error.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let encoded = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        let temporary = path.with_extension("tmp");
        fs::write(&temporary, encoded).map_err(|error| error.to_string())?;
        if path.exists() {
            fs::copy(path, path.with_extension("bak")).map_err(|error| error.to_string())?;
        }
        fs::rename(temporary, path).map_err(|error| error.to_string())
    }
}

/// Result of deterministic alias resolution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Resolution {
    /// Exactly one endpoint matched.
    Matched,
    /// Multiple endpoints matched and require operator selection.
    Ambiguous,
    /// No endpoint matched.
    Missing,
}

/// Published lifecycle state for a discovered endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EndpointState {
    /// Endpoint is usable.
    Online,
    /// Endpoint exists but has reduced service.
    Degraded,
    /// Endpoint is not currently present.
    Offline,
    /// Matching requires operator resolution.
    Ambiguous,
}

/// A state transition emitted by the alias registry monitor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StateTransition {
    /// Alias affected by the transition.
    pub alias: String,
    /// Previous state, if this is the first observation.
    pub previous: Option<EndpointState>,
    /// New state.
    pub current: EndpointState,
}

/// Tracks endpoint state and emits only actual changes.
#[derive(Clone, Debug, Default)]
pub struct StateTracker {
    states: std::collections::BTreeMap<String, EndpointState>,
}

impl StateTracker {
    /// Records a state and returns a transition when it changed.
    pub fn update(
        &mut self,
        alias: impl Into<String>,
        current: EndpointState,
    ) -> Option<StateTransition> {
        let alias = alias.into();
        let previous = self.states.insert(alias.clone(), current);
        (previous != Some(current)).then_some(StateTransition { alias, previous, current })
    }
}

/// Resolves aliases with serial, then VID/PID/interface/name precedence.
#[must_use]
pub fn resolve_alias<'a>(
    selector: &AliasSelector,
    endpoints: &'a [ObservedEndpoint],
) -> (Resolution, Vec<&'a ObservedEndpoint>) {
    let serial_matches: Vec<_> = selector.serial.as_ref().map_or_else(Vec::new, |serial| {
        endpoints.iter().filter(|endpoint| endpoint.serial.as_ref() == Some(serial)).collect()
    });
    if !serial_matches.is_empty() {
        return resolve(serial_matches);
    }
    let candidates: Vec<_> = endpoints
        .iter()
        .filter(|endpoint| {
            selector.vid_pid.is_none_or(|value| endpoint.vid_pid == Some(value))
                && selector.interface.is_none_or(|value| endpoint.interface == Some(value))
                && selector
                    .name_pattern
                    .as_ref()
                    .is_none_or(|pattern| endpoint.name.contains(pattern))
        })
        .collect();
    resolve(candidates)
}

fn resolve(matches: Vec<&ObservedEndpoint>) -> (Resolution, Vec<&ObservedEndpoint>) {
    match matches.len() {
        0 => (Resolution::Missing, matches),
        1 => (Resolution::Matched, matches),
        _ => (Resolution::Ambiguous, matches),
    }
}

/// Capped reconnect backoff state for hot-plug recovery.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReconnectBackoff {
    attempt: u32,
}

impl ReconnectBackoff {
    /// Returns the next delay and advances the failure counter.
    #[must_use]
    pub fn next_delay_ms(&mut self) -> u64 {
        let delay = 250_u64.saturating_mul(1_u64 << self.attempt.min(6));
        self.attempt = self.attempt.saturating_add(1);
        delay.min(10_000)
    }

    /// Resets backoff after a stable connection.
    pub const fn reset(&mut self) {
        self.attempt = 0;
    }
}

/// Small deterministic reconnect coordinator used by the daemon monitor.
#[derive(Clone, Debug, Default)]
pub struct ReconnectController {
    backoff: ReconnectBackoff,
    state: Option<EndpointState>,
}

impl ReconnectController {
    /// Observes alias resolution and emits a state transition plus retry delay
    /// when the endpoint is missing or ambiguous.
    pub fn observe(
        &mut self,
        alias: impl Into<String>,
        resolution: Resolution,
    ) -> (Option<StateTransition>, Option<u64>) {
        let next = match resolution {
            Resolution::Matched => EndpointState::Online,
            Resolution::Ambiguous => EndpointState::Ambiguous,
            Resolution::Missing => EndpointState::Offline,
        };
        let transition = (self.state != Some(next)).then(|| StateTransition {
            alias: alias.into(),
            previous: self.state.replace(next),
            current: next,
        });
        let retry = match next {
            EndpointState::Online => {
                self.backoff.reset();
                None
            }
            EndpointState::Offline | EndpointState::Ambiguous | EndpointState::Degraded => {
                Some(self.backoff.next_delay_ms())
            }
        };
        (transition, retry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(serial: &str, name: &str) -> ObservedEndpoint {
        ObservedEndpoint {
            name: name.into(),
            direction: "in".into(),
            vid_pid: Some((1, 2)),
            serial: Some(serial.into()),
            interface: Some(0),
            physical_path: None,
        }
    }

    #[test]
    fn device_profile_validates_and_round_trips() {
        let profile = DeviceProfile {
            id: "lexicon.reflex".into(),
            version: 1,
            name: "Lexicon Reflex".into(),
            effect_type: EffectType::Reverb,
            identity_probes: Vec::new(),
            provided_capabilities: vec!["reverb".into()],
            capabilities: vec![CapabilityDefinition {
                id: "midi-control".into(),
                transport: ControlTransport::Midi,
                unsafe_on_connect: false,
            }],
            controls: vec![ControlDefinition {
                label: "Mix".into(),
                cc: Some(12),
                program: None,
                range: (0, 127),
            }],
            queries: Vec::new(),
            replies: Vec::new(),
            templates: Vec::new(),
            max_message_size: 1024,
            documented_features: Vec::new(),
        };
        profile.validate().expect("valid profile");
        let encoded = serde_json::to_vec(&profile).expect("encode");
        assert_eq!(serde_json::from_slice::<DeviceProfile>(&encoded).expect("decode"), profile);
    }

    #[test]
    fn identity_probes_are_masked_and_bounded() {
        let probes = [IdentityProbe { offset: 1, value: vec![0x10, 0x20], mask: vec![0xF0, 0xF0] }];
        assert!(match_identity_probes(&probes, &[0, 0x1F, 0x2A]));
        assert!(!match_identity_probes(&probes, &[0, 0x0F, 0x2A]));
        assert!(!match_identity_probes(&probes, &[0, 0x1F]));
        assert!(!match_identity_probes(
            &[IdentityProbe { offset: 0, value: vec![1], mask: vec![] }],
            &[1]
        ));
    }

    #[test]
    fn query_definitions_require_bounded_correlated_replies() {
        let replies = [ReplyDefinition { id: "state".into(), value: vec![0x01], mask: vec![0x7F] }];
        let queries = [QueryDefinition {
            id: "read-state".into(),
            request: vec![0x10],
            template_id: None,
            reply_id: "state".into(),
        }];
        assert_eq!(validate_query_definitions(&queries, &replies, 8), Ok(()));
        assert!(validate_query_definitions(
            &[QueryDefinition { reply_id: "missing".into(), ..queries[0].clone() }],
            &replies,
            8
        )
        .is_err());
        assert!(validate_query_definitions(
            &[QueryDefinition { request: vec![0x80], ..queries[0].clone() }],
            &replies,
            8
        )
        .is_err());
        assert!(validate_query_definitions(&queries, &[], 8).is_err());
    }

    #[test]
    fn profile_rejects_duplicate_query_and_reply_ids() {
        let mut profile = eventide_micropitch_profile();
        profile.replies = vec![
            ReplyDefinition { id: "state".into(), value: vec![1], mask: vec![127] },
            ReplyDefinition { id: "state".into(), value: vec![2], mask: vec![127] },
        ];
        assert_eq!(profile.validate(), Err("duplicate reply ID"));
        profile.replies.pop();
        profile.queries = vec![
            QueryDefinition {
                id: "read".into(),
                request: vec![16],
                template_id: None,
                reply_id: "state".into(),
            },
            QueryDefinition {
                id: "read".into(),
                request: vec![17],
                template_id: None,
                reply_id: "state".into(),
            },
        ];
        assert_eq!(profile.validate(), Err("duplicate query ID"));
    }

    #[test]
    fn profile_catalog_versions_user_profiles_and_reserves_reflex() {
        let catalog = ProfileCatalog::load(Vec::new()).expect("built-ins");
        assert_eq!(catalog.profiles().len(), 4);
        assert_eq!(catalog.get("eventide.micropitch").map(|profile| profile.version), Some(1));

        let mut newer = eventide_micropitch_profile();
        newer.version = 2;
        newer.name = "Customized MicroPitch".into();
        let catalog = ProfileCatalog::load(vec![newer.clone()]).expect("newer user version");
        assert_eq!(
            catalog.get("eventide.micropitch").map(|profile| profile.name.as_str()),
            Some("Customized MicroPitch")
        );
        assert!(ProfileCatalog::load(vec![eventide_micropitch_profile()]).is_err());
        assert!(ProfileCatalog::load(vec![newer.clone(), newer]).is_err());

        for reserved in ["lexicon.reflex", "lexicon-reflex-rev1", "Reflex Rev1"] {
            let mut claimed = eventide_micropitch_profile();
            claimed.id = reserved.into();
            claimed.version = 99;
            assert!(ProfileCatalog::load(vec![claimed]).is_err());
        }
    }

    #[test]
    fn eventide_micropitch_profile_includes_documented_controls() {
        let profile = eventide_micropitch_profile();
        assert_eq!(profile.id, "eventide.micropitch");
        assert_eq!(profile.effect_type, EffectType::Modulation);
        assert!(profile.validate().is_ok());
        assert_eq!(
            profile.controls.iter().map(|control| control.cc).collect::<Vec<_>>(),
            vec![
                Some(4),
                Some(9),
                Some(14),
                Some(15),
                Some(20),
                Some(21),
                Some(22),
                Some(23),
                Some(24),
                Some(25),
                Some(26),
                Some(27),
                Some(28),
                Some(29),
                Some(30),
                Some(31),
                None,
            ]
        );
        assert_eq!(
            profile.controls.iter().map(|control| control.label.as_str()).collect::<Vec<_>>(),
            vec![
                "Expression Pedal",
                "TAP TEMPO",
                "ACTIVE/BYPASS",
                "FLEX",
                "Mix",
                "Pitch A",
                "Pitch B",
                "Depth",
                "Rate/Sens",
                "Pitch Mix",
                "Tone",
                "Delay A",
                "Delay B",
                "Mod",
                "Feedback",
                "Out Lvl",
                "Preset 1",
            ]
        );
        assert!(!profile.controls.iter().any(|control| control.cc == Some(2)));
        assert_eq!(profile.controls[16].program, Some(1));
    }

    #[test]
    fn profile_renders_validated_control_messages_without_transmitting() {
        let profile = eventide_micropitch_profile();
        assert_eq!(profile.render_control_message("Mix", 1, 64).expect("CC"), vec![0xB0, 20, 64]);
        assert_eq!(
            profile.render_control_message("Preset 1", 2, 0).expect("program"),
            vec![0xC1, 1]
        );
        assert!(profile.render_control_message("Mix", 0, 64).is_err());
        assert!(profile.render_control_message("Mix", 1, 128).is_err());
        assert!(profile.render_control_message("missing", 1, 1).is_err());
    }

    #[test]
    fn lexicon_reflex_profile_is_reverb_sysex_anchor() {
        let profile = lexicon_reflex_profile();
        assert_eq!(profile.effect_type, EffectType::Reverb);
        assert_eq!(profile.capabilities[0].id, "midi-sysex");
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn arena2000_profile_is_conservative_midi_anchor() {
        let profile = valeton_arena2000_profile();
        assert_eq!(profile.id, "valeton.arena2000");
        assert_eq!(profile.name, "Valeton Arena2000");
        assert_eq!(profile.effect_type, EffectType::Other);
        assert_eq!(profile.capabilities[0].transport, ControlTransport::Midi);
        assert_eq!(profile.controls.len(), 133);
        assert!(profile.controls.iter().any(|control| control.cc == Some(53)));
        assert!(profile.validate().is_ok());
        assert!(profile.provides_capability("cabinet_ir"));
        assert!(!profile.provides_capability("hall_reverb"));
        assert_eq!(builtin_capability_providers("cabinet_ir").len(), 2);
        assert_eq!(builtin_capability_providers(" CABINET_IR ").len(), 2);
        assert_eq!(default_capability_provider("cabinet_ir").unwrap().id, "m-vave.ir-box");
        assert_eq!(default_capability_provider("unknown"), None);
        assert_eq!(builtin_profiles().len(), 4);
        assert!(builtin_profile("m-vave.ir-box").is_some());
        assert!(builtin_profile("eventide.micropitch").is_some());
        assert!(builtin_profile("valeton.arena2000").is_some());
        assert!(builtin_profile("missing").is_none());
    }

    #[test]
    fn midisport_loader_and_runtime_identities_are_distinct() {
        assert!(!usb_identity_matches(MIDISPORT_4X4_LOADER_USB, MIDISPORT_4X4_RUNTIME_USB));
        assert_eq!(MIDISPORT_4X4_LOADER_USB.product_id, 0x1020);
        assert_eq!(MIDISPORT_4X4_RUNTIME_USB.product_id, 0x1021);
    }

    #[test]
    fn launch_control_identity_gate_rejects_ambiguous_products() {
        assert_eq!(
            classify_launch_control("Novation Launch Control XL"),
            LaunchControlIdentity::Mk1
        );
        assert_eq!(
            classify_launch_control("Novation Launch Control XL Mk2"),
            LaunchControlIdentity::Mk2
        );
        assert_eq!(
            classify_launch_control("Novation Launchpad X"),
            LaunchControlIdentity::LaunchpadFamily
        );
        assert_eq!(classify_launch_control("MIDI Controller"), LaunchControlIdentity::Unknown);
        assert_eq!(
            classify_launch_control_usb(
                UsbIdentity { vendor_id: 0x1235, product_id: 0x0061 },
                "Focusrite Launch Control XL"
            ),
            LaunchControlIdentity::Mk1
        );
        assert_eq!(
            classify_launch_control_usb(
                UsbIdentity { vendor_id: 0x1235, product_id: 0x0061 },
                "Launchpad XL"
            ),
            LaunchControlIdentity::LaunchpadFamily
        );
        assert!(usb_identity_matches(EVENTIDE_MICROPITCH_USB, EVENTIDE_MICROPITCH_USB));
        assert!(usb_identity_matches(MVAVE_IR_BOX_USB, MVAVE_IR_BOX_USB));
    }

    #[test]
    fn launch_control_mk1_led_protocol_matches_programmers_reference() {
        assert_eq!(launch_control_led_index(0, 0), Some(0));
        assert_eq!(launch_control_led_index(3, 7), Some(31));
        assert_eq!(launch_control_led_index(4, 0), None);
        assert_eq!(launch_control_led_index(0, 8), None);
        assert_eq!(LAUNCH_CONTROL_DEVICE_INDEX, 40);
        assert_eq!(LAUNCH_CONTROL_RIGHT_INDEX, 47);
        assert_eq!(launch_control_index_label(0).as_deref(), Some("Top knob 1"));
        assert_eq!(launch_control_index_label(39).as_deref(), Some("Bottom channel button 8"));
        assert_eq!(launch_control_index_label(43).as_deref(), Some("Record Arm"));
        assert_eq!(launch_control_index_label(48), None);
        assert_eq!(launch_control_led_value(LedColor::Off, 127, 0), 0);
        assert_eq!(launch_control_led_value(LedColor::Red, 127, 0), 3);
        assert_eq!(launch_control_led_value(LedColor::Green, 127, 0), 0x30);
        assert_eq!(launch_control_led_value(LedColor::Amber, 127, 0x0c), 0x3f);
        let labels: std::collections::BTreeSet<_> = (0..48)
            .map(|index| launch_control_index_label(index).expect("documented index"))
            .collect();
        assert_eq!(labels.len(), 48);
        let template = LaunchControlTemplate {
            template: 0,
            assignments: vec![LaunchControlAssignment {
                index: 0,
                channel: 0,
                number: 14,
                kind: LaunchControlMessageKind::Cc,
            }],
        };
        assert!(template.validate().is_ok());
        assert_eq!(template.assignment(0).map(|a| a.number), Some(14));
        assert_eq!(template.assignment(1), None);
        assert_eq!(template.assignments_by_index().len(), 1);
        let encoded = serde_json::to_vec(&template).expect("template encode");
        assert_eq!(
            serde_json::from_slice::<LaunchControlTemplate>(&encoded).expect("template decode"),
            template
        );
        let mut duplicate = template;
        duplicate.assignments.push(duplicate.assignments[0].clone());
        assert!(duplicate.validate().is_err());
        assert!(LaunchControlTemplate { template: 16, assignments: Vec::new() }
            .validate()
            .is_err());
        assert_eq!(
            encode_launch_control_led(0, 24, 0x7f),
            Some(vec![0xf0, 0x00, 0x20, 0x29, 0x02, 0x11, 0x78, 0x00, 0x18, 0x7f, 0xf7])
        );
        assert_eq!(encode_launch_control_led(16, 0, 0), None);
        assert_eq!(encode_launch_control_led(0, 48, 0), None);
        assert_eq!(
            encode_launch_control_toggle(2, 7, true),
            Some(vec![0xf0, 0x00, 0x20, 0x29, 0x02, 0x11, 0x7b, 0x02, 0x07, 0x7f, 0xf7])
        );
        assert_eq!(encode_launch_control_toggle(2, 7, false).expect("toggle")[9], 0);
        assert_eq!(encode_launch_control_toggle(0, 24, true), None);
        assert_eq!(encode_launch_control_note_led(0, 40, 0x7f), Some([0x90, 40, 0x7f]));
        assert_eq!(encode_launch_control_note_led(16, 40, 1), None);
        assert_eq!(encode_launch_control_cc_led(2, 14, 0x80), Some([0xb2, 14, 0]));
        assert_eq!(encode_launch_control_cc_led(16, 14, 1), None);
        assert_eq!(
            encode_launch_control_template(7),
            Some([0xf0, 0x00, 0x20, 0x29, 0x02, 0x11, 0x77, 0x07, 0xf7])
        );
    }

    #[test]
    fn led_coalescer_suppresses_redundant_updates() {
        let state = LedState::new(LedColor::Green, 100, false);
        let mut coalescer = LedCoalescer::default();
        coalescer.set_desired(2, state);
        assert_eq!(coalescer.drain_pending(), vec![(2, state)]);
        assert!(coalescer.drain_pending().is_empty());
        coalescer.set_desired(2, LedState { blink: true, ..state });
        assert_eq!(coalescer.drain_pending().len(), 1);
        coalescer.set_desired(3, state);
        assert_eq!(coalescer.drain_pending_limited(1).len(), 1);
        assert_eq!(coalescer.drain_pending_limited(0).len(), 0);
        assert_eq!(coalescer.desired_len(), 2);
        coalescer.request_full_resync();
        assert_eq!(coalescer.drain_pending_limited(1).len(), 1);
        assert_eq!(coalescer.drain_pending(), vec![(3, state)]);
        assert!(coalescer.drain_pending().is_empty());
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn reflex_packing_checksum_and_nibbles_match_contract() {
        for length in [1, 6, 7, 8, 55, 56, 127] {
            let source: Vec<u8> = (0..length)
                .map(|index| u8::try_from(index).expect("bounded test index").wrapping_mul(37))
                .collect();
            let packed = lexicon_reflex::pack(&source);
            assert_eq!(lexicon_reflex::unpack(&packed, length).expect("unpack"), source);
        }
        assert!(lexicon_reflex::unpack(&[0x80], 1).is_err());
        assert!(lexicon_reflex::unpack(&[0], 0).is_err());
        assert_eq!(
            lexicon_reflex::pack_group(&[0x00, 0x80, 0xFF]).expect("group"),
            vec![0x06, 0x00, 0x00, 0x7F]
        );
        assert_eq!(lexicon_reflex::pack(&[0x80]), vec![0x01, 0x00]);
        assert_eq!(lexicon_reflex::checksum(&[1, 2, 0x80]), 3);
        assert_eq!(lexicon_reflex::nibblize(0xABCD), [0x0A, 0x0B, 0x0C, 0x0D]);
        let setup =
            lexicon_reflex::ReflexSetup::new(&[0; lexicon_reflex::SETUP_RAW_BYTES]).expect("setup");
        assert_eq!(setup.packed().len(), lexicon_reflex::SETUP_PACKED_BYTES);
        assert_eq!(
            lexicon_reflex::ReflexSetup::from_packed(&setup.packed()).expect("packed setup"),
            setup
        );
        assert!(lexicon_reflex::ReflexSetup::from_packed(&[0; 55]).is_err());
        let bank = lexicon_reflex::ReflexRegisterBank::new(vec![setup.clone(); 128]).expect("bank");
        assert_eq!(bank.raw_bytes().len(), lexicon_reflex::ALL_REGISTERS_RAW_BYTES);
        assert_eq!(
            lexicon_reflex::ReflexRegisterBank::from_raw(&bank.raw_bytes()).expect("bank raw"),
            bank
        );
        assert_eq!(bank.encode_frame(1).expect("bank frame").len(), 7_176);
        let frame = bank.encode_frame(1).expect("bank frame");
        assert_eq!(
            lexicon_reflex::ReflexRegisterBank::from_frame(&frame).expect("bank decode").0,
            1
        );
        assert!(bank.get(127).is_some());
        let mut editable = bank.clone();
        editable.get_mut(127).expect("register").set_parameter(0, 1).expect("parameter");
        assert_eq!(editable.get(127).and_then(|setup| setup.parameter(0)), Some(1));
        editable.set(0, setup.clone()).expect("replace register");
        assert!(editable.set(128, setup.clone()).is_err());
        assert!(bank.get(127).is_some());
        assert!(lexicon_reflex::ReflexRegisterBank::new(vec![setup.clone()]).is_err());
        assert_eq!(setup.algorithm(), None);
        let mut named = [0_u8; lexicon_reflex::SETUP_RAW_BYTES];
        named[0] = 3;
        named[21..27].copy_from_slice(b"Reflex");
        let named_setup = lexicon_reflex::ReflexSetup::new(&named).expect("named setup");
        assert_eq!(named_setup.algorithm(), Some(3));
        assert_eq!(&named_setup.name_bytes()[..6], b"Reflex");
        let mut parameters = named_setup;
        parameters.set_name(b"New Name").expect("name");
        assert_eq!(&parameters.name_bytes()[..8], b"New Name");
        assert_eq!(parameters.name_bytes()[8], 0);
        assert!(parameters.set_name(&[b'x'; 17]).is_err());
        assert_eq!(parameters.parameter(0), Some(0));
        parameters.set_parameter(9, 0xABCD).expect("parameter");
        assert_eq!(parameters.parameter(9), Some(0xABCD));
        assert!(parameters.patch().is_ok());
        assert_eq!(parameters.parameter(10), None);
        assert!(parameters.set_parameter(10, 1).is_err());
        let encoded = serde_json::to_vec(&setup).expect("setup json");
        assert_eq!(
            serde_json::from_slice::<lexicon_reflex::ReflexSetup>(&encoded).expect("setup decode"),
            setup
        );
        assert!(serde_json::from_slice::<lexicon_reflex::ReflexSetup>(b"[0,1]").is_err());
        assert!(lexicon_reflex::ReflexSetup::new(&[0; 48]).is_err());
        let patch = lexicon_reflex::ReflexPatch {
            sources: [1, 2, 3, 4],
            destinations: [0, 1, 8, 9],
            scales: [-128, -1, 0, 127],
        };
        let mut setup_bytes = [0_u8; lexicon_reflex::SETUP_RAW_BYTES];
        patch.encode(&mut setup_bytes).expect("patch");
        assert_eq!(lexicon_reflex::ReflexPatch::decode(&setup_bytes).expect("patch decode"), patch);
        assert_eq!(&setup_bytes[37..45], &[1, 2, 3, 4, 0, 1, 8, 9]);
        assert_eq!(&setup_bytes[45..49], &[128, 255, 0, 127]);
        assert!(lexicon_reflex::ReflexPatch { destinations: [10, 0, 0, 0], ..patch }
            .validate()
            .is_err());
        let packed_parameter =
            lexicon_reflex::encode_packed_parameter(2, 7, 0xABCD).expect("packed parameter");
        assert_eq!(packed_parameter.len(), 9);
        assert_eq!(
            lexicon_reflex::decode_packed_parameter(&packed_parameter).expect("packed decode"),
            (2, 7, 0xABCD)
        );
        let dump =
            lexicon_reflex::encode_setup_dump(&[0; lexicon_reflex::SETUP_BYTES]).expect("dump");
        assert_eq!(dump.len(), 65);
        assert_eq!(dump.last().copied(), Some(0));
        assert!(lexicon_reflex::validate_setup_dump(&dump).is_ok());
        assert_eq!(lexicon_reflex::decode_setup_dump(&dump).expect("decode"), vec![0; 56]);
        let raw_setup = vec![0x80; lexicon_reflex::SETUP_RAW_BYTES];
        let active = lexicon_reflex::encode_active_setup_frame(2, &raw_setup).expect("active");
        assert_eq!(active.len(), 63);
        assert_eq!(
            lexicon_reflex::decode_active_setup_frame(&active).expect("active decode"),
            (2, raw_setup.clone())
        );
        let register =
            lexicon_reflex::encode_register_setup_frame(2, 127, &raw_setup).expect("register");
        assert_eq!(register.len(), 64);
        assert_eq!(
            lexicon_reflex::decode_register_setup_frame(&register).expect("register decode"),
            (2, 127, raw_setup)
        );
        let setup_frame = lexicon_reflex::encode_setup_frame(2, &[0; 56]).expect("frame");
        assert_eq!(setup_frame.len(), 70);
        assert_eq!(
            lexicon_reflex::decode_setup_frame(&setup_frame).expect("frame decode"),
            (2, vec![0; 56])
        );
        let bank = vec![0x80; lexicon_reflex::ALL_REGISTERS_RAW_BYTES];
        let all = lexicon_reflex::encode_all_registers_frame(3, &bank).expect("all frame");
        assert_eq!(all.len(), 7_176);
        assert_eq!(
            lexicon_reflex::decode_all_registers_frame(&all).expect("all decode"),
            (3, bank)
        );
        let mut corrupt = dump;
        corrupt[0] ^= 1;
        assert!(lexicon_reflex::validate_setup_dump(&corrupt).is_err());
        assert!(lexicon_reflex::encode_setup_dump(&[0; 55]).is_err());
        assert_eq!(
            lexicon_reflex::encode_nibblized_parameter(2, 7, 0xABCD).expect("frame"),
            vec![0xF0, 6, 2, 0x52, 7, 0x0A, 0x0B, 0x0C, 0x0D, 0xF7]
        );
        let nib = lexicon_reflex::encode_nibblized_parameter(2, 7, 0xABCD).expect("nib");
        assert_eq!(
            lexicon_reflex::decode_nibblized_parameter(&nib).expect("nib decode"),
            (2, 7, 0xABCD)
        );
        assert_eq!(
            lexicon_reflex::decode_message(&active).expect("typed active"),
            lexicon_reflex::DecodedMessage::ActiveSetup { channel: 2, setup: vec![0x80; 49] }
        );
        assert_eq!(
            lexicon_reflex::decode_message(&nib).expect("typed nib"),
            lexicon_reflex::DecodedMessage::NibblizedParameter {
                channel: 2,
                parameter: 7,
                value: 0xABCD
            }
        );
        let malformed = vec![0xF0, 6, 2, 0x72, 0x70, 0, 0xF7];
        assert!(lexicon_reflex::decode_message(&malformed).is_err());
        assert_eq!(
            lexicon_reflex::encode_task(1, lexicon_reflex::TASK_BYPASS, 1).expect("task"),
            [0xF0, 6, 2, 0x61, 0x72, 1, 0xF7]
        );
        assert_eq!(
            lexicon_reflex::encode_task(1, lexicon_reflex::TASK_BYPASS, 2),
            Err("invalid bypass argument")
        );
        let task = lexicon_reflex::encode_task(1, lexicon_reflex::TASK_RECALL, 7).expect("task");
        assert_eq!(lexicon_reflex::decode_task(&task), Ok((1, lexicon_reflex::TASK_RECALL, 7)));
        assert_eq!(lexicon_reflex::decode_task(&[0; 7]), Err("invalid Reflex task frame"));
        assert_eq!(
            lexicon_reflex::encode_request(0, lexicon_reflex::REQUEST_ACTIVE, 0).expect("request"),
            [0xF0, 6, 2, 0x30, 0x60, 0, 0xF7]
        );
        assert_eq!(lexicon_reflex::encode_request(0, 0x63, 0), Err("unsupported Reflex request"));
        let request = lexicon_reflex::encode_request(4, lexicon_reflex::REQUEST_REGISTER, 127)
            .expect("request");
        assert_eq!(
            lexicon_reflex::decode_request(&request).expect("request decode"),
            (4, lexicon_reflex::REQUEST_REGISTER, 127)
        );
        assert!(lexicon_reflex::decode_request(&[0; 7]).is_err());
    }

    #[test]
    fn reflex_algorithm_registry_matches_manual_order_and_numbers() {
        let algorithms = lexicon_reflex::algorithms();
        assert_eq!(algorithms.len(), 8);
        assert_eq!(
            algorithms.iter().map(|a| a.number).collect::<Vec<_>>(),
            (1..=8).collect::<Vec<_>>()
        );
        assert_eq!(algorithms[0].name, "Reverb");
        assert_eq!(algorithms[2].name, "Chorus 1");
        assert_eq!(algorithms[7].description, "Chorus/Delays");
        assert_eq!(algorithms[0].preset_numbers, &[1, 2, 3, 4, 5, 6]);
        assert_eq!(algorithms[7].preset_numbers, &[13, 14]);
    }

    #[test]
    fn reflex_parameter_metadata_excludes_unused_slots_and_bounds_values() {
        let reverb = lexicon_reflex::parameters(1);
        assert_eq!(reverb.len(), 10);
        assert_eq!(reverb[0].description, "Mid Reverb Decay");
        assert_eq!(reverb[0].min, 0x8000);
        assert_eq!(reverb[0].max, 0xBC00);
        assert!(reverb[3].bipolar);
        assert!(lexicon_reflex::parameters(2)
            .iter()
            .all(|parameter| parameter.number != 8 && parameter.number != 9));
        assert_eq!(lexicon_reflex::parameters(3).len(), 8);
        assert_eq!(lexicon_reflex::parameters(6).len(), 7);
    }

    #[test]
    fn reflex_echo_rhythm_is_bounded_and_ordered() {
        assert_eq!(lexicon_reflex::ECHO_RHYTHMS.len(), 14);
        assert_eq!(lexicon_reflex::echo_rhythm(1).expect("first").label, "64th");
        assert_eq!(lexicon_reflex::echo_rhythm(14).expect("last").label, "Whole note");
        assert!(lexicon_reflex::echo_rhythm(0).is_none());
        assert!(lexicon_reflex::echo_rhythm(15).is_none());
    }

    #[test]
    fn sysex_template_is_bounded_and_7_bit_safe() {
        let template = SysexTemplate {
            id: None,
            segments: vec![TemplateSegment::Literal(vec![1, 2]), TemplateSegment::Parameter(0)],
            max_bytes: 3,
        };
        assert_eq!(template.render(&[127]).expect("render"), vec![1, 2, 127]);
        assert_eq!(template.render(&[128]), Err("parameter is not MIDI data"));
        assert_eq!(template.render(&[]), Err("parameter missing"));
        let expression = SysexTemplate {
            id: None,
            segments: vec![TemplateSegment::Expression("$0 + 1".into())],
            max_bytes: 1,
        };
        assert_eq!(expression.render(&[41]).expect("expression"), vec![42]);
        let function = SysexTemplate {
            id: None,
            segments: vec![TemplateSegment::Expression("max($0, 9)".into())],
            max_bytes: 1,
        };
        assert_eq!(function.render(&[4]).expect("function"), vec![9]);
        let mut budget = EvaluationBudget::new();
        assert_eq!(function.render_with_budget(&[4], &mut budget).expect("budget"), vec![9]);
        let oversized = SysexTemplate {
            id: None,
            segments: vec![TemplateSegment::Literal(vec![1, 2, 3, 4])],
            max_bytes: 3,
        };
        assert_eq!(oversized.render(&[]), Err("template exceeds maximum size"));
    }

    #[test]
    fn profile_validates_template_identity_and_query_references() {
        let mut profile = builtin_profiles()[0].clone();
        profile.templates = vec![SysexTemplate {
            id: Some("active-query".into()),
            segments: vec![TemplateSegment::Literal(vec![0x10])],
            max_bytes: 1,
        }];
        profile.replies = vec![ReplyDefinition {
            id: "active-reply".into(),
            value: vec![0x01],
            mask: vec![0x7F],
        }];
        profile.queries = vec![QueryDefinition {
            id: "active".into(),
            request: vec![0x10],
            template_id: Some("active-query".into()),
            reply_id: "active-reply".into(),
        }];
        assert!(profile.validate().is_ok());
        assert_eq!(profile.render_query_request("active", &[]), Ok(vec![0x10]));
        profile.queries[0].template_id = Some("missing".into());
        assert_eq!(profile.validate(), Err("query references a missing template"));
        assert_eq!(
            profile.render_query_request("active", &[]),
            Err("query references a missing template")
        );
    }

    #[test]
    fn integer_literals_are_strict_and_checked() {
        assert_eq!(parse_integer_literal(" 0x7f "), Ok(127));
        assert_eq!(parse_integer_literal("-42"), Ok(-42));
        assert!(parse_integer_literal("0x").is_err());
        assert!(parse_integer_literal("9223372036854775808").is_err());
        assert_eq!(parse_parameter_expression("$0 + 3", &[4]), Ok(7));
        assert_eq!(parse_parameter_expression_full("($0 + 3) * 2", &[4]), Ok(14));
        assert_eq!(parse_parameter_expression_full("1 << 3 | 2", &[]), Ok(10));
        assert_eq!(parse_parameter_expression_full("2 + 3 == 5", &[]), Ok(1));
        assert_eq!(parse_parameter_expression_full("2 < 3 & 1", &[]), Ok(1));
        assert_eq!(parse_parameter_expression_full("$0 ? 11 : 22", &[1]), Ok(11));
        assert_eq!(parse_parameter_expression_full("$0 ? 11 : 22", &[0]), Ok(22));
        assert_eq!(parse_parameter_expression_full("1 ? 0 ? 2 : 3 : 4", &[]), Ok(3));
        assert_eq!(parse_parameter_expression_full("clamp($0 + 1, 0, 7)", &[4]), Ok(5));
        assert_eq!(parse_parameter_expression_full("max(2, min(9, 4))", &[]), Ok(4));
        assert!(parse_parameter_expression_full("unknown(1)", &[]).is_err());
        assert!(parse_parameter_expression_full("1 ? 2", &[]).is_err());
        assert!(parse_parameter_expression_full("(1 + 2", &[]).is_err());
        assert!(parse_parameter_expression("$1 + 3", &[4]).is_err());
        assert_eq!(parse_parameter_function_expression("max($0, 9)", &[4]), Ok(9));
    }

    #[test]
    fn sysex_hex_parser_accepts_framed_and_rejects_malformed_input() {
        assert_eq!(parse_sysex_hex("F0 06 02 F7").expect("hex"), vec![0xF0, 6, 2, 0xF7]);
        assert_eq!(parse_sysex_hex("06 02").expect("raw"), vec![6, 2]);
        assert!(parse_sysex_hex("F0 06").is_err());
        assert!(parse_sysex_hex("F0 80 F7").is_err());
        assert!(parse_sysex_hex("GG").is_err());
        assert!(sysex_mask_matches(
            &[0xF0, 0x06, 0x12, 0xF7],
            &[0xF0, 0x06, 0x10, 0xF7],
            &[0xFF, 0xFF, 0xF0, 0xFF]
        ));
        assert!(!sysex_mask_matches(&[0xF0, 0x06], &[0xF0], &[0xFF]));
    }

    #[test]
    fn binary_evaluation_is_checked() {
        assert_eq!(eval_binary(2, "+", 3), Ok(5));
        assert_eq!(eval_binary(7, "&", 3), Ok(3));
        assert_eq!(eval_binary(2, "<<", 3), Ok(16));
        assert_eq!(
            eval_binary(1, "/", 0),
            Err("expression operation overflow or invalid divisor/shift")
        );
        assert!(eval_binary(i64::MAX, "+", 1).is_err());
        assert_eq!(parse_binary_expression("0x10 << 2"), Ok(64));
        assert!(parse_binary_expression("1 + 2 extra").is_err());
        assert_eq!(parse_function_expression("clamp(10, 0, 7)"), Ok(7));
        assert!(parse_function_expression("unknown(1)").is_err());
    }

    #[test]
    fn approved_functions_have_strict_arity() {
        assert_eq!(eval_function("min", &[4, 2]), Ok(2));
        assert_eq!(eval_function("clamp", &[10, 0, 7]), Ok(7));
        assert_eq!(eval_function("sum", &[1, 2, 3]), Ok(6));
        assert_eq!(eval_function("xor", &[7, 3]), Ok(4));
        assert_eq!(eval_function("hi7", &[256]), Ok(2));
        assert_eq!(eval_function("lo7", &[130]), Ok(2));
        assert_eq!(eval_function("lookup", &[1, 10, 20]), Ok(20));
        assert!(eval_function("lookup", &[2, 10, 20]).is_err());
        assert!(eval_function("lookup", &[-1, 10]).is_err());
        assert!(eval_function("lookup", &[i64::MAX, 10]).is_err());
        assert!(eval_function("min", &[1]).is_err());
        assert!(eval_function("unknown", &[1]).is_err());
        assert_eq!(eval_lookup(1, &[10, 20]), Ok(20));
        assert!(eval_lookup(-1, &[10]).is_err());
        assert!(eval_lookup(2, &[10]).is_err());
    }

    #[test]
    fn evaluation_budget_is_bounded() {
        let mut budget = EvaluationBudget::new();
        assert_eq!(budget.consume(9_000), Ok(()));
        assert_eq!(budget.remaining(), 1_000);
        assert_eq!(budget.consume(1_001), Err("expression evaluation budget exceeded"));
        assert_eq!(budget.remaining(), 1_000);
        assert_eq!(
            parse_parameter_function_expression_budgeted("max($0, 9)", &[4], &mut budget),
            Ok(9)
        );
    }

    #[test]
    fn malformed_expression_corpus_never_panics() {
        let malformed = [
            "",
            "(",
            ")",
            "$",
            "$x",
            "$99",
            "1 +",
            "1 2",
            "1 ** 2",
            "min(1)",
            "lookup(0)",
            "clamp(1, 2)",
            "1 / 0",
            "1 << 64",
            "unknown(1)",
        ];
        for expression in malformed {
            assert!(parse_parameter_expression(expression, &[1]).is_err(), "{expression}");
            assert!(parse_parameter_function_expression(expression, &[1]).is_err(), "{expression}");
            assert!(parse_parameter_expression_full(expression, &[1]).is_err(), "{expression}");
        }
        let oversized = "(".repeat(1024);
        assert!(parse_parameter_expression(&oversized, &[1]).is_err());
        assert!(parse_parameter_function_expression(&oversized, &[1]).is_err());
        assert!(parse_parameter_expression_full(&oversized, &[1]).is_err());
    }

    #[test]
    fn serial_wins_over_renumbering_and_name_changes() {
        let endpoints = vec![endpoint("A", "renamed"), endpoint("B", "same")];
        let selector = AliasSelector {
            alias: "reflex".into(),
            serial: Some("A".into()),
            vid_pid: Some((1, 2)),
            interface: Some(0),
            name_pattern: Some("missing".into()),
        };
        let (result, matches) = resolve_alias(&selector, &endpoints);
        assert_eq!(result, Resolution::Matched);
        assert_eq!(matches[0].serial.as_deref(), Some("A"));
    }

    #[test]
    fn mvave_ir_box_preset_sysex_matches_captured_family() {
        assert_eq!(
            mvave_ir_box_preset_sysex(5).expect("preset"),
            vec![
                0xF0, 0x00, 0x32, 0x09, 0x49, 0x00, 0x00, 0x00, 0x02, 0x04, 0x00, 0x00, 0x00, 0x1E,
                0x00, 0x00, 0x00, 0x04, 0x24, 0x00, 0xF7
            ]
        );
        assert_eq!(mvave_ir_box_preset_sysex(32).expect("preset")[18..20], [0x38, 0x03]);
        assert!(mvave_ir_box_preset_sysex(0).is_err());
        assert!(mvave_ir_box_preset_sysex(33).is_err());
        assert_eq!(
            mvave_ir_box_module_sysex(MvaveIrBoxModule::Ir, true),
            vec![
                0xF0, 0x00, 0x32, 0x09, 0x49, 0x00, 0x00, 0x40, 0x02, 0x11, 0x00, 0x00, 0x00, 0x10,
                0x00, 0x00, 0x00, 0x01, 0x4E, 0x03, 0xF7
            ]
        );
    }

    #[test]
    fn duplicate_identity_is_ambiguous() {
        let endpoints = vec![endpoint("A", "x"), endpoint("B", "x")];
        let selector = AliasSelector {
            alias: "device".into(),
            serial: None,
            vid_pid: Some((1, 2)),
            interface: Some(0),
            name_pattern: Some("x".into()),
        };
        assert_eq!(resolve_alias(&selector, &endpoints).0, Resolution::Ambiguous);
    }

    #[test]
    fn reconnect_backoff_caps_and_resets() {
        let mut backoff = ReconnectBackoff::default();
        assert_eq!(backoff.next_delay_ms(), 250);
        assert_eq!(backoff.next_delay_ms(), 500);
        for _ in 0..10 {
            let _ = backoff.next_delay_ms();
        }
        assert_eq!(backoff.next_delay_ms(), 10_000);
        backoff.reset();
        assert_eq!(backoff.next_delay_ms(), 250);
    }

    #[test]
    fn reconnect_controller_emits_transitions_and_resets_on_online() {
        let mut controller = ReconnectController::default();
        let (transition, retry) = controller.observe("reflex", Resolution::Missing);
        assert_eq!(transition.expect("transition").current, EndpointState::Offline);
        assert_eq!(retry, Some(250));
        let (_, retry) = controller.observe("reflex", Resolution::Matched);
        assert_eq!(retry, None);
        let (_, retry) = controller.observe("reflex", Resolution::Missing);
        assert_eq!(retry, Some(250));
    }

    #[test]
    fn alias_registry_round_trips_and_creates_backup() {
        let path = std::env::temp_dir().join(format!("mackes-alias-{}.json", std::process::id()));
        let registry = AliasRegistry {
            aliases: vec![AliasSelector {
                alias: "reflex".into(),
                serial: Some("A".into()),
                vid_pid: None,
                interface: None,
                name_pattern: None,
            }],
        };
        registry.save(&path).expect("first save");
        registry.save(&path).expect("second save");
        assert_eq!(AliasRegistry::load(&path).expect("load"), registry);
        assert!(path.with_extension("bak").exists());
        assert!(AliasRegistry {
            aliases: vec![
                AliasSelector { alias: "reflex".into(), ..registry.aliases[0].clone() },
                registry.aliases[0].clone()
            ]
        }
        .validate()
        .is_err());
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(
            std::env::temp_dir().join(format!("mackes-alias-{}.bak", std::process::id())),
        );
    }

    #[test]
    fn state_tracker_emits_only_real_changes() {
        let mut tracker = StateTracker::default();
        assert_eq!(
            tracker.update("reflex", EndpointState::Online).expect("initial").previous,
            None
        );
        assert!(tracker.update("reflex", EndpointState::Online).is_none());
        assert_eq!(
            tracker.update("reflex", EndpointState::Offline).expect("change").previous,
            Some(EndpointState::Online)
        );
    }

    #[test]
    fn effect_colors_have_stable_non_color_markers() {
        assert_eq!(effect_color(EffectType::Reverb), ColorToken::Reverb);
        assert_eq!(ColorToken::Modulation.presentation().1, "[MOD]");
        assert_eq!(ColorToken::Hazard.presentation().1, "[!]");
        assert_eq!(ColorToken::Reverb.ansi16(), 12);
        assert_eq!(ColorToken::Hazard.ansi256(), 196);
        assert_eq!(ColorIntensity::Dim.marker(), "(off)");
        assert_eq!(ColorIntensity::Hazard.marker(), "(!)");
        assert_eq!(
            ColorToken::ALL.map(ColorToken::name),
            ["reverb", "modulation", "cabinet", "neutral", "hazard"]
        );
        assert_eq!(
            launch_control_led_state(ColorToken::Reverb, ColorIntensity::Normal),
            LedState::new(LedColor::Amber, 64, false)
        );
        assert_eq!(
            launch_control_led_state(ColorToken::Neutral, ColorIntensity::Dim),
            LedState::new(LedColor::Off, 0, false)
        );
        assert_eq!(
            launch_control_led_state(ColorToken::Hazard, ColorIntensity::Hazard),
            LedState::new(LedColor::Red, 127, false)
        );
    }
}
