//! Hardcoded Lexicon Reflex Rev. 1 protocol constants.

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

/// One controller-to-Reflex destination assignment in the production layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControllerAssignment {
    /// Stable Launch Control physical identity.
    pub physical_control_id: &'static str,
    /// Stable Reflex destination parameter or operation.
    pub destination_parameter: &'static str,
}

/// Production Launch Control layout for all Reflex algorithms and parameters.
///
/// This table owns destination placement. MIDI source tuples remain owned by
/// the Launch Control Factory Template 1 catalog.
pub const CONTROLLER_ASSIGNMENTS: [ControllerAssignment; 18] = [
    ControllerAssignment {
        physical_control_id: "knob-r1-c4",
        destination_parameter: "reflex.parameter-0",
    },
    ControllerAssignment {
        physical_control_id: "knob-r1-c5",
        destination_parameter: "reflex.parameter-1",
    },
    ControllerAssignment {
        physical_control_id: "knob-r1-c6",
        destination_parameter: "reflex.parameter-2",
    },
    ControllerAssignment {
        physical_control_id: "knob-r1-c7",
        destination_parameter: "reflex.parameter-3",
    },
    ControllerAssignment {
        physical_control_id: "knob-r1-c8",
        destination_parameter: "reflex.parameter-4",
    },
    ControllerAssignment {
        physical_control_id: "fader-4",
        destination_parameter: "reflex.parameter-5",
    },
    ControllerAssignment {
        physical_control_id: "fader-5",
        destination_parameter: "reflex.parameter-6",
    },
    ControllerAssignment {
        physical_control_id: "fader-6",
        destination_parameter: "reflex.parameter-7",
    },
    ControllerAssignment {
        physical_control_id: "fader-7",
        destination_parameter: "reflex.parameter-8",
    },
    ControllerAssignment {
        physical_control_id: "fader-8",
        destination_parameter: "reflex.parameter-9",
    },
    ControllerAssignment {
        physical_control_id: "button-r1-c5",
        destination_parameter: "reflex.algorithm-1",
    },
    ControllerAssignment {
        physical_control_id: "button-r1-c6",
        destination_parameter: "reflex.algorithm-2",
    },
    ControllerAssignment {
        physical_control_id: "button-r1-c7",
        destination_parameter: "reflex.algorithm-3",
    },
    ControllerAssignment {
        physical_control_id: "button-r1-c8",
        destination_parameter: "reflex.algorithm-4",
    },
    ControllerAssignment {
        physical_control_id: "button-r2-c5",
        destination_parameter: "reflex.algorithm-5",
    },
    ControllerAssignment {
        physical_control_id: "button-r2-c6",
        destination_parameter: "reflex.algorithm-6",
    },
    ControllerAssignment {
        physical_control_id: "button-r2-c7",
        destination_parameter: "reflex.algorithm-7",
    },
    ControllerAssignment {
        physical_control_id: "button-r2-c8",
        destination_parameter: "reflex.algorithm-8",
    },
];

/// Returns the production controller assignment for a physical control.
#[must_use]
pub fn controller_assignment(control_id: &str) -> Option<&'static ControllerAssignment> {
    CONTROLLER_ASSIGNMENTS.iter().find(|assignment| assignment.physical_control_id == control_id)
}

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
    /// Selects one of the eight documented DSP algorithms.
    ///
    /// # Errors
    ///
    /// Returns an error when the algorithm is outside 1–8.
    pub fn set_algorithm(&mut self, algorithm: u8) -> Result<(), &'static str> {
        if !(1..=8).contains(&algorithm) {
            return Err("Reflex algorithm is out of range");
        }
        self.0[0] = algorithm;
        Ok(())
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

/// One documented PCM70 factory sound translated into the closest Reflex algorithm.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Pcm70Translation {
    /// Stable command/config identifier.
    pub id: &'static str,
    /// PCM70 factory display name.
    pub name: &'static str,
    /// PCM70 source program where documented.
    pub source_program: &'static str,
    /// Closest Reflex algorithm number.
    pub reflex_algorithm: u8,
    /// Normalized semantic parameter targets, indexed by Reflex parameter number.
    pub normalized: [u8; 10],
    /// Honest compatibility note shown to operators.
    pub note: &'static str,
}

/// Curated PCM70-to-Reflex approximation catalog.
pub const PCM70_TRANSLATIONS: [Pcm70Translation; 5] = [
    Pcm70Translation {
        id: "concert-wave",
        name: "Concert Wave",
        source_program: "3.1",
        reflex_algorithm: 1,
        normalized: [110, 40, 108, 78, 127, 127, 72, 108, 76, 96],
        note: "Late-attack, maximum-size, bright five-second hall approximation",
    },
    Pcm70Translation {
        id: "circular-reverbs",
        name: "Circular Reverbs",
        source_program: "1.3-inspired",
        reflex_algorithm: 8,
        normalized: [92, 52, 104, 88, 42, 84, 94, 90, 54, 64],
        note: "Rotating multi-delay texture translated to Reflex Delay 1",
    },
    Pcm70Translation {
        id: "inf-reverb",
        name: "INF Reverb",
        source_program: "4.4",
        reflex_algorithm: 1,
        normalized: [127, 18, 110, 96, 104, 127, 100, 118, 70, 82],
        note: "Maximum-decay Reflex approximation; the Reflex cannot freeze indefinitely",
    },
    Pcm70Translation {
        id: "rich-plate",
        name: "Rich Plate",
        source_program: "5.0",
        reflex_algorithm: 2,
        normalized: [96, 20, 112, 74, 122, 94, 66, 122, 64, 64],
        note: "Bright, dense, high-initial-diffusion plate approximation",
    },
    Pcm70Translation {
        id: "mod-wobble",
        name: "Mod Wobble",
        source_program: "0.0",
        reflex_algorithm: 3,
        normalized: [82, 116, 106, 86, 72, 88, 100, 96, 64, 64],
        note: "Modulation-heavy chorus/flange approximation",
    },
];

/// Returns the stable PCM70 translation catalog.
#[must_use]
pub const fn pcm70_translations() -> &'static [Pcm70Translation; 5] {
    &PCM70_TRANSLATIONS
}

/// Encodes an active-setup `SysEx` selection for any documented algorithm.
///
/// # Errors
///
/// Returns an error when the algorithm or channel is outside the documented range.
pub fn encode_algorithm_selection(algorithm: u8, channel: u8) -> Result<Vec<u8>, &'static str> {
    if !(1..=8).contains(&algorithm) {
        return Err("Reflex algorithm is out of range");
    }
    if let Some(translation) =
        PCM70_TRANSLATIONS.iter().find(|translation| translation.reflex_algorithm == algorithm)
    {
        return encode_pcm70_translation(translation.id, channel);
    }
    let mut raw = [0_u8; SETUP_RAW_BYTES];
    raw[0] = algorithm;
    let mut setup = ReflexSetup::new(&raw)?;
    let name = algorithms()
        .iter()
        .find(|item| item.number == algorithm)
        .map(|item| item.name)
        .ok_or("Reflex algorithm is out of range")?;
    setup.set_name(name.as_bytes())?;
    for parameter in parameters(algorithm) {
        setup.set_parameter(parameter.number, parameter.min)?;
    }
    encode_active_setup_frame(channel, setup.as_bytes())
}

const fn parameter_step(algorithm: u8, number: u8) -> u16 {
    match (algorithm, number) {
        (1 | 2, 0 | 4) | (3 | 8, 8) | (8, 3 | 9) => 0x0400,
        (1 | 2, 3) | (3, 5) => 0x0800,
        (1 | 2, 5) => 0x0100,
        (1 | 2, 6 | 8 | 9) | (3, 3 | 4 | 6 | 7) | (8, 4..=6) => 0x0080,
        _ => 0x0040,
    }
}

fn normalized_parameter(algorithm: u8, metadata: ParameterMetadata, normalized: u8) -> u16 {
    let span = u32::from(metadata.max - metadata.min);
    let target = u16::try_from((span * u32::from(normalized)) / 127)
        .expect("normalized Reflex parameter span fits u16");
    let step = parameter_step(algorithm, metadata.number);
    metadata
        .min
        .saturating_add(((target.saturating_add(step / 2)) / step).saturating_mul(step))
        .min(metadata.max)
}

/// Converts a normalized controller value into the documented wire range for
/// one parameter of the selected algorithm.
///
/// Returns `None` when the algorithm is invalid or the parameter slot is not
/// implemented by that algorithm. This is the single runtime authority for
/// both parameter availability and value scaling.
#[must_use]
pub fn normalize_parameter(algorithm: u8, number: u8, normalized: u8) -> Option<u16> {
    parameters(algorithm)
        .iter()
        .copied()
        .find(|parameter| parameter.number == number)
        .map(|metadata| normalized_parameter(algorithm, metadata, normalized.min(127)))
}

/// Builds a validated Reflex setup for a named PCM70 translation.
///
/// # Errors
///
/// Returns an error when the preset identifier is unknown or its static mapping is invalid.
pub fn translate_pcm70(id: &str) -> Result<ReflexSetup, &'static str> {
    let translation = PCM70_TRANSLATIONS
        .iter()
        .find(|translation| translation.id.eq_ignore_ascii_case(id))
        .ok_or("unknown PCM70 translation")?;
    let mut raw = [0_u8; SETUP_RAW_BYTES];
    raw[0] = translation.reflex_algorithm;
    let mut setup = ReflexSetup::new(&raw)?;
    setup.set_name(translation.name.as_bytes())?;
    for parameter in parameters(translation.reflex_algorithm) {
        setup.set_parameter(
            parameter.number,
            normalized_parameter(
                translation.reflex_algorithm,
                *parameter,
                translation.normalized[usize::from(parameter.number)],
            ),
        )?;
    }
    Ok(setup)
}

/// Encodes a translated PCM70 preset as a documented Reflex active-setup `SysEx` frame.
///
/// # Errors
///
/// Returns an error for an unknown preset or invalid zero-based Reflex MIDI channel.
pub fn encode_pcm70_translation(id: &str, channel: u8) -> Result<Vec<u8>, &'static str> {
    let setup = translate_pcm70(id)?;
    encode_active_setup_frame(channel, setup.as_bytes())
}

/// Returns the documented parameter values carried by a translated PCM70 preset.
///
/// The result is ordered by the Reflex parameter number and is bounded by the
/// profile's ten audio parameters so callers can safely project it to controls.
///
/// # Errors
///
/// Returns an error for an unknown translation or invalid compiled preset data.
pub fn pcm70_translation_parameters(id: &str) -> Result<Vec<(u8, u16)>, &'static str> {
    let setup = translate_pcm70(id)?;
    let algorithm = setup.algorithm().ok_or("translated Reflex algorithm is invalid")?;
    Ok(parameters(algorithm)
        .iter()
        .filter_map(|metadata| {
            setup.parameter(metadata.number).map(|value| (metadata.number, value))
        })
        .collect())
}

/// Returns translated preset parameters normalized for a MIDI controller.
///
/// Each tuple is `(Reflex parameter number, 0..=127 controller value)`.
/// Normalization uses the algorithm-specific documented parameter range.
///
/// # Errors
///
/// Returns an error for an unknown translation or invalid compiled preset data.
pub fn pcm70_translation_controller_values(id: &str) -> Result<Vec<(u8, u8)>, &'static str> {
    let setup = translate_pcm70(id)?;
    let algorithm = setup.algorithm().ok_or("translated Reflex algorithm is invalid")?;
    Ok(parameters(algorithm)
        .iter()
        .filter_map(|metadata| {
            let value = setup.parameter(metadata.number)?;
            let span = u32::from(metadata.max.saturating_sub(metadata.min));
            let normalized = u8::try_from(
                (u32::from(value.saturating_sub(metadata.min)) * 127)
                    .checked_div(span)
                    .unwrap_or(0)
                    .min(127),
            )
            .ok()?;
            Some((metadata.number, normalized))
        })
        .collect())
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
    if frame.len() != 7 || frame[0..3] != [0xF0, MANUFACTURER_ID, PRODUCT_ID] || frame[6] != 0xF7 {
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
    if frame.len() != 10 || frame[0..3] != [0xF0, MANUFACTURER_ID, PRODUCT_ID] || frame[9] != 0xF7 {
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
    if frame.len() != 9 || frame[0..3] != [0xF0, MANUFACTURER_ID, PRODUCT_ID] || frame[8] != 0xF7 {
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
    if !matches!(frame.len(), 63 | 65)
        || frame[0..3] != [0xF0, MANUFACTURER_ID, PRODUCT_ID]
        || frame[4] != 0x38
        || frame[frame.len() - 1] != 0xF7
    {
        return Err("invalid Reflex active setup frame");
    }
    let channel = frame[3] & 0x0F;
    if frame[3] >> 4 != 0 {
        return Err("invalid Reflex active setup header");
    }
    let packed = &frame[5..61];
    if checksum(packed) != frame[frame.len() - 2] || packed.iter().any(|byte| *byte > 127) {
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
    if frame.len() < 4 || frame[0] != 0xF0 || frame[1] != MANUFACTURER_ID || frame[2] != PRODUCT_ID
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
        3 => decode_request(frame).map(|(channel, request, argument)| DecodedMessage::Request {
            channel,
            request,
            argument,
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
