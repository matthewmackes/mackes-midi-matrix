//! Declarative and built-in device profile boundary.
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

mod eventide_micropitch;
pub use eventide_micropitch::{eventide_controller_assignments, EventideControllerAssignment};

/// Hardcoded Lexicon Reflex Rev. 1 protocol constants.
pub mod lexicon_reflex;

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

/// Built-in Eventide `MicroPitch` profile from the firmware 1.0+ quick-reference MIDI table.
#[must_use]
pub fn eventide_micropitch_profile() -> DeviceProfile {
    eventide_micropitch::profile()
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
        controls: lexicon_reflex::pcm70_translations()
            .iter()
            .map(|translation| ControlDefinition {
                label: translation.name.into(),
                cc: None,
                program: None,
                range: (0, 1),
                operation: Some(format!("pcm70_reflex:{}", translation.id)),
            })
            .collect(),
        queries: Vec::new(),
        replies: Vec::new(),
        templates: Vec::new(),
        max_message_size: 1024,
        documented_features: vec![
            "PCM70 factory-sound translation catalog (approximate)".into(),
            "Concert Wave, Circular Reverbs, INF Reverb, Rich Plate, Mod Wobble".into(),
            "Validated Reflex active-setup SysEx generation".into(),
        ],
    }
}

/// Returns the built-in conservative device-profile catalog in stable order.
#[must_use]
pub fn builtin_profiles() -> Vec<DeviceProfile> {
    vec![lexicon_reflex_profile(), eventide_micropitch_profile()]
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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LaunchControlIdentity {
    /// Novation Launch Control XL Mk2, the platform template identity.
    Mk1,
    /// Retained as a compatibility identity for explicit Mk1 hardware.
    Mk2,
    /// A Launchpad-family device, not Launch Control XL.
    LaunchpadFamily,
    /// Any other or unrecognized controller.
    Unknown,
}

/// Versioned, typed capability contract for the supported Launch Control XL surface.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LaunchControlCapabilityDescriptor {
    /// Contract version, independent of firmware version.
    pub version: u16,
    /// Supported hardware identity.
    pub identity: LaunchControlIdentity,
    /// Number of stable physical controls in the catalog.
    pub physical_control_count: u8,
    /// Number of addressable background LEDs.
    pub led_count: u8,
    /// Factory 1 wire template byte.
    pub factory1_template: u8,
    /// Whether template selection is documented and supported.
    pub template_selection: bool,
    /// Whether LED readback is supported (it is not for this contract).
    pub led_readback: bool,
}

/// Returns the supported Launch Control XL capability contract.
#[must_use]
pub const fn launch_control_capability_descriptor() -> LaunchControlCapabilityDescriptor {
    LaunchControlCapabilityDescriptor {
        version: 1,
        identity: LaunchControlIdentity::Mk1,
        physical_control_count: 56,
        led_count: 48,
        factory1_template: LAUNCH_CONTROL_MK2_FACTORY1_SLOT,
        template_selection: true,
        led_readback: false,
    }
}

/// Novation product families recognized by the platform discovery layer.
///
/// Only Launch Control XL Mk2 has a concrete controller template today; the
/// remaining families are intentionally discoverable but fail closed until a
/// model-specific mapping is installed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum NovationProductFamily {
    /// Launch Control controllers.
    LaunchControl,
    /// Launchpad grid controllers.
    Launchpad,
    /// Launchkey keyboard controllers.
    Launchkey,
    /// Circuit grooveboxes.
    Circuit,
    /// SL keyboard controllers.
    Sl,
    /// Peak synthesizer.
    Peak,
    /// Summit synthesizer.
    Summit,
    /// Bass Station synthesizers.
    BassStation,
    /// `MiniNova` synthesizer.
    MiniNova,
    /// `UltraNova` synthesizer.
    UltraNova,
    /// Impulse keyboard controllers.
    Impulse,
    /// `FLkey` keyboard controllers.
    Flkey,
    /// An unrecognized product family.
    Unknown,
}

/// Classifies a Novation product name without claiming unsupported mappings.
#[must_use]
pub fn classify_novation_product(name: &str) -> NovationProductFamily {
    let normalized = name.to_ascii_lowercase();
    if normalized.contains("launch control") {
        NovationProductFamily::LaunchControl
    } else if normalized.contains("launchpad") {
        NovationProductFamily::Launchpad
    } else if normalized.contains("launchkey") {
        NovationProductFamily::Launchkey
    } else if normalized.contains("circuit") {
        NovationProductFamily::Circuit
    } else if normalized.contains("sl mk") || normalized == "sl" {
        NovationProductFamily::Sl
    } else if normalized.contains("bass station") {
        NovationProductFamily::BassStation
    } else if normalized.contains("mininova") {
        NovationProductFamily::MiniNova
    } else if normalized.contains("ultranova") {
        NovationProductFamily::UltraNova
    } else if normalized.contains("impulse") {
        NovationProductFamily::Impulse
    } else if normalized.contains("flkey") {
        NovationProductFamily::Flkey
    } else if normalized.contains("summit") {
        NovationProductFamily::Summit
    } else if normalized.contains("peak") {
        NovationProductFamily::Peak
    } else {
        NovationProductFamily::Unknown
    }
}

/// Returns whether the platform has a reviewed template for a product name.
#[must_use]
pub fn novation_template_available(name: &str) -> bool {
    classify_novation_product(name) == NovationProductFamily::LaunchControl
        && classify_launch_control(name) == LaunchControlIdentity::Mk2
}

/// Launch Control XL Mk2-compatible `SysEx` manufacturer header.
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

/// Returns the two channel-button LED indices used as one fader-column proxy.
#[must_use]
pub const fn fader_column_led_proxy(column: u8) -> Option<(u8, u8)> {
    if column < 8 {
        Some((24 + column, 32 + column))
    } else {
        None
    }
}

/// Priority layer for controller LED feedback.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum LedFeedbackLayer {
    /// Normal mapping/activity state.
    Base,
    /// Active assignment guidance.
    Assignment,
    /// Terminal success/failure result overlay.
    Result,
}

/// Deterministic state consumed by the Launch Control feedback renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LedFeedbackScheduler {
    /// Authoritative normal mapping/activity state.
    pub base: LedState,
    /// Assignment-state state, when a session is active.
    pub assignment: Option<LedState>,
    /// Terminal result, with its elapsed fake-clock time.
    pub result: Option<(bool, u64)>,
}

impl LedFeedbackScheduler {
    /// Creates a scheduler with only the normal base layer.
    #[must_use]
    pub const fn new(base: LedState) -> Self {
        Self { base, assignment: None, result: None }
    }

    /// Returns the currently authoritative layer output at a fake-clock instant.
    #[must_use]
    pub const fn state_at(self, elapsed_ms: u64) -> LedState {
        if let Some((success, started_ms)) = self.result {
            if elapsed_ms.saturating_sub(started_ms) >= 1_600 {
                return self.base;
            }
            return LedState::new(
                result_overlay_color(elapsed_ms.saturating_sub(started_ms), success),
                127,
                false,
            );
        }
        if let Some(assignment) = self.assignment {
            return assignment;
        }
        self.base
    }

    /// Removes completed result/assignment overlays and restores the base layer.
    pub const fn restore_base(&mut self) {
        self.assignment = None;
        self.result = None;
    }
}

/// Selects the highest-priority active LED layer, restoring base when overlays end.
#[must_use]
pub const fn select_led_feedback_layer(
    base: bool,
    assignment: bool,
    result: bool,
) -> Option<LedFeedbackLayer> {
    if result {
        Some(LedFeedbackLayer::Result)
    } else if assignment {
        Some(LedFeedbackLayer::Assignment)
    } else if base {
        Some(LedFeedbackLayer::Base)
    } else {
        None
    }
}

/// Returns whether a terminal result overlay should currently be lit.
///
/// The overlay consists of exactly two 400 ms pulses separated by 400 ms gaps.
#[must_use]
pub const fn result_overlay_lit(elapsed_ms: u64) -> bool {
    if elapsed_ms >= 1_600 {
        return false;
    }
    let phase = elapsed_ms % 800;
    phase < 400
}

/// Selects the terminal overlay color while its deterministic pulse is active.
#[must_use]
pub const fn result_overlay_color(elapsed_ms: u64, success: bool) -> LedColor {
    if result_overlay_lit(elapsed_ms) {
        if success {
            LedColor::Green
        } else {
            LedColor::Red
        }
    } else {
        LedColor::Off
    }
}

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

/// Physical control category used by renderer-owned Launch Control faceplates.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LaunchControlControlKind {
    /// Rotary encoder.
    Knob,
    /// Channel button.
    Button,
    /// Utility/navigation button.
    Utility,
}

/// Stable identity for a physical Mk1 control, independent of MIDI/LED numbers.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct PhysicalControlId(String);

impl PhysicalControlId {
    /// Creates an identity only from the bounded canonical catalog.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown or unsupported identity.
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if launch_control_physical_catalog().iter().any(|control| control.id.0 == value) {
            Ok(Self(value))
        } else {
            Err("unknown Launch Control physical control ID")
        }
    }

    /// Returns the stable serialized identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Role of a physical control in the controller layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PhysicalControlRole {
    /// Rotary encoder.
    Knob,
    /// Channel button.
    ChannelButton,
    /// Dedicated fader.
    Fader,
    /// Utility/navigation button.
    Utility,
}

/// MIDI message family emitted by the authoritative Launch Control XL Mk2 layout.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchControlSourceKind {
    /// MIDI Control Change.
    ControlChange,
    /// MIDI Note On/Off.
    Note,
}

/// Press/value behavior for one control in the authoritative Mk2 layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchControlInputBehavior {
    /// A continuous 0-127 value.
    Continuous,
    /// A nonzero press followed by a zero-valued release.
    PressRelease,
}

/// One exact input/feedback tuple in the authoritative Factory Template 1 layout.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LaunchControlLayoutControl {
    /// Stable physical identity, independent of the wire tuple.
    pub physical_control_id: String,
    /// Physical role.
    pub role: PhysicalControlRole,
    /// Zero-based MIDI channel (wire channel 9).
    pub channel: u8,
    /// Input message family.
    pub source_kind: LaunchControlSourceKind,
    /// Note or controller number.
    pub source_number: u8,
    /// Value/press behavior.
    pub behavior: LaunchControlInputBehavior,
    /// Optional documented LED index. Faders have no individual LED.
    pub feedback_address: Option<u8>,
}

/// Factory Template 1 slot selected on the hardware. Novation numbers User
/// templates 0–7 and Factory templates 8–15, so Factory Template 1 is 8.
pub const LAUNCH_CONTROL_MK2_FACTORY1_SLOT: u8 = 8;

/// The sole machine-readable Launch Control XL Mk2 production layout.
///
/// Factory Template 1 is selected on the hardware. The table is based on the
/// physically captured Mk2 input tuples recorded in the qualification matrix;
/// it does not define or transmit a Components template.
#[must_use]
pub fn launch_control_mk2_factory1_layout() -> Vec<LaunchControlLayoutControl> {
    let catalog = launch_control_physical_catalog();
    catalog
        .into_iter()
        .filter_map(|control| {
            let (source_kind, source_number, behavior) = match control.role {
                PhysicalControlRole::Knob | PhysicalControlRole::Fader => (
                    LaunchControlSourceKind::ControlChange,
                    control.source_address?,
                    LaunchControlInputBehavior::Continuous,
                ),
                PhysicalControlRole::ChannelButton => (
                    LaunchControlSourceKind::Note,
                    control.source_address?,
                    LaunchControlInputBehavior::PressRelease,
                ),
                PhysicalControlRole::Utility => {
                    let order = control.order.checked_sub(48)?;
                    if order < 4 {
                        (
                            LaunchControlSourceKind::Note,
                            105 + order,
                            LaunchControlInputBehavior::PressRelease,
                        )
                    } else {
                        (
                            LaunchControlSourceKind::ControlChange,
                            100 + order,
                            LaunchControlInputBehavior::PressRelease,
                        )
                    }
                }
            };
            Some(LaunchControlLayoutControl {
                physical_control_id: control.id.as_str().to_owned(),
                role: control.role,
                channel: 8,
                source_kind,
                source_number,
                behavior,
                feedback_address: control.feedback_address,
            })
        })
        .collect()
}

/// Resolves an active Factory Template 1 input without guessing or aliases.
#[must_use]
pub fn resolve_launch_control_mk2_factory1_input(
    channel: u8,
    source_kind: LaunchControlSourceKind,
    source_number: u8,
    value: u8,
) -> Option<String> {
    if value == 0 {
        return None;
    }
    let mut matches = launch_control_mk2_factory1_layout().into_iter().filter(|control| {
        control.channel == channel
            && control.source_kind == source_kind
            && control.source_number == source_number
    });
    let control = matches.next()?;
    matches.next().is_none().then_some(control.physical_control_id)
}

/// Complete stable physical-control metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PhysicalControl {
    /// Stable identity.
    pub id: PhysicalControlId,
    /// Physical role.
    pub role: PhysicalControlRole,
    /// Physical row.
    pub row: u8,
    /// Physical column.
    pub column: u8,
    /// Stable physical ordering.
    pub order: u8,
    /// Optional MIDI source address.
    pub source_address: Option<u8>,
    /// Optional LED feedback address.
    pub feedback_address: Option<u8>,
    /// Profile-owned label.
    pub label: String,
}

/// Returns the complete, non-overlapping Mk1 physical catalog.
#[must_use]
pub fn launch_control_physical_catalog() -> Vec<PhysicalControl> {
    let mut controls = Vec::with_capacity(56);
    for order in 0..24u8 {
        let row = order / 8 + 1;
        let column = order % 8 + 1;
        controls.push(PhysicalControl {
            id: PhysicalControlId(format!("knob-r{row}-c{column}")),
            role: PhysicalControlRole::Knob,
            row,
            column,
            order,
            source_address: Some(match row {
                1 => 13 + (column - 1),
                2 => 29 + (column - 1),
                _ => 49 + (column - 1),
            }),
            feedback_address: Some(order),
            label: launch_control_index_label(order).unwrap_or_default(),
        });
    }
    for order in 0..16u8 {
        let row = order / 8 + 1;
        let column = order % 8 + 1;
        let index = 24 + order;
        controls.push(PhysicalControl {
            id: PhysicalControlId(format!("button-r{row}-c{column}")),
            role: PhysicalControlRole::ChannelButton,
            row,
            column,
            order: 24 + order,
            // Factory Template 1 follows the Launchpad-style four-column
            // note grid: 41–44 / 57–60 on the top row and 73–76 / 89–92 on
            // the bottom row. Physical identity and LED index remain row-major.
            source_address: Some(41 + (row - 1) * 32 + ((column - 1) / 4) * 16 + (column - 1) % 4),
            feedback_address: Some(index),
            label: launch_control_index_label(index).unwrap_or_default(),
        });
    }
    for order in 0..8u8 {
        controls.push(PhysicalControl {
            id: PhysicalControlId(format!("fader-{}", order + 1)),
            role: PhysicalControlRole::Fader,
            row: 3,
            column: order + 1,
            order: 40 + order,
            source_address: Some(77 + order),
            feedback_address: None,
            label: format!("Fader {}", order + 1),
        });
        controls.push(PhysicalControl {
            id: PhysicalControlId(format!("utility-{}", order + 1)),
            role: PhysicalControlRole::Utility,
            row: 4,
            column: order + 1,
            order: 48 + order,
            source_address: None,
            feedback_address: Some(40 + order),
            label: ["Device", "Mute", "Solo", "Record Arm", "Up", "Down", "Left", "Right"]
                [usize::from(order)]
            .into(),
        });
    }
    controls
}

/// One documented Launch Control XL Mk2 faceplate control.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LaunchControlFaceplateControl {
    /// Stable documented zero-based index.
    pub index: u8,
    /// Short profile-owned display label.
    pub label: String,
    /// Physical category.
    pub kind: LaunchControlControlKind,
}

/// Returns every documented Mk1 faceplate control in physical order.
#[must_use]
pub fn launch_control_faceplate() -> Vec<LaunchControlFaceplateControl> {
    (0..48)
        .filter_map(|index| {
            let kind = match index {
                0..=23 => LaunchControlControlKind::Knob,
                24..=39 => LaunchControlControlKind::Button,
                40..=47 => LaunchControlControlKind::Utility,
                _ => return None,
            };
            Some(LaunchControlFaceplateControl {
                index,
                label: launch_control_index_label(index)?,
                kind,
            })
        })
        .collect()
}

/// Renderer-neutral identity contract for a bidirectional controller/HUD.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BidirectionalHudFaceplate {
    /// Stable device identity marker shared by source and destination lanes.
    pub identity_marker: String,
    /// Profile-owned display label.
    pub label: String,
    /// Whether the profile documents feedback capability.
    pub feedback_capable: bool,
    /// Whether the wire protocol is verified for production use.
    pub protocol_verified: bool,
}

/// Builds a conservative HUD contract without asserting unsupported protocol behavior.
#[must_use]
pub fn bidirectional_hud_faceplate(
    identity_marker: impl Into<String>,
    label: impl Into<String>,
    feedback_capable: bool,
) -> BidirectionalHudFaceplate {
    BidirectionalHudFaceplate {
        identity_marker: identity_marker.into(),
        label: label.into(),
        feedback_capable,
        protocol_verified: false,
    }
}

/// One fixed effects-control group on the Launch Control XL faceplate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectsFaceplateGroup {
    /// Stable group identifier.
    pub id: String,
    /// Signal-path row label.
    pub row: String,
    /// Exact group label.
    pub label: String,
    /// Default provider owning this group.
    pub owner: String,
    /// Four parameter-control indices in physical order.
    pub parameter_indices: Vec<u8>,
    /// Stable identities for the parameter controls, in the same order.
    pub parameter_control_ids: Vec<PhysicalControlId>,
    /// Enable button index.
    pub enable_index: u8,
    /// Stable identity for the enable button.
    pub enable_control_id: PhysicalControlId,
    /// Type/model button index.
    pub type_index: u8,
    /// Stable identity for the type/model button.
    pub type_control_id: PhysicalControlId,
}

/// Static, renderer-neutral effects faceplate contract.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectsFaceplateCatalog {
    /// Six fixed signal-path groups.
    pub groups: Vec<EffectsFaceplateGroup>,
    /// Eight physical fader indices.
    pub fader_indices: Vec<u8>,
    /// Stable identities for the eight faders.
    pub fader_control_ids: Vec<PhysicalControlId>,
    /// Controls intentionally unused by this effects surface.
    pub unused_indices: Vec<u8>,
    /// Stable identities for controls intentionally unused by this surface.
    pub unused_control_ids: Vec<PhysicalControlId>,
}

/// Logical effect-group state used by the pickup-aware LED policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectGroupState {
    /// Group is enabled and active.
    Enabled,
    /// Group is explicitly bypassed.
    Disabled,
    /// Group is unavailable on the connected profile.
    Unavailable,
    /// Group is selected for type/model navigation.
    Selected,
    /// Group is known but has not yet synchronized.
    Unknown,
}

/// Logical LED policy result; wire encoding remains owned by the verified device profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectGroupLed {
    /// Green enabled indicator.
    Green,
    /// Solid red disabled indicator.
    SolidRed,
    /// Blinking red unavailable indicator.
    BlinkingRed,
    /// Blue/teal selected type indicator.
    BlueTeal,
    /// No trustworthy state to display.
    Off,
}

/// Returns the deterministic pickup-aware group LED policy.
#[must_use]
pub const fn effect_group_led(state: EffectGroupState, pickup_ready: bool) -> EffectGroupLed {
    match state {
        EffectGroupState::Enabled if pickup_ready => EffectGroupLed::Green,
        EffectGroupState::Enabled | EffectGroupState::Unknown => EffectGroupLed::Off,
        EffectGroupState::Disabled => EffectGroupLed::SolidRed,
        EffectGroupState::Unavailable => EffectGroupLed::BlinkingRed,
        EffectGroupState::Selected => EffectGroupLed::BlueTeal,
    }
}

/// Runtime state for the six bounded effects groups and fader bank.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectsGroupRuntime {
    /// Group state in fixed faceplate order.
    pub groups: Vec<EffectGroupState>,
    /// Bounded fader values in physical order.
    pub faders: Vec<u8>,
    /// Whether the next state publication must resend all feedback.
    pub resync_required: bool,
}

impl EffectsGroupRuntime {
    /// Creates an offline-safe unknown state for the fixed faceplate.
    #[must_use]
    pub fn new() -> Self {
        Self {
            groups: vec![EffectGroupState::Unknown; 6],
            faders: vec![0; 8],
            resync_required: true,
        }
    }

    /// Updates one group, rejecting indices outside the fixed six-group contract.
    ///
    /// # Errors
    ///
    /// Returns an error when the group index is outside the six-group faceplate.
    pub fn set_group(&mut self, index: usize, state: EffectGroupState) -> Result<(), &'static str> {
        let Some(group) = self.groups.get_mut(index) else {
            return Err("effects group index is out of range");
        };
        *group = state;
        Ok(())
    }

    /// Updates one fader with MIDI-safe clamping.
    ///
    /// # Errors
    ///
    /// Returns an error when the fader index is outside the eight-fader bank.
    pub fn set_fader(&mut self, index: usize, value: u16) -> Result<(), &'static str> {
        let Some(fader) = self.faders.get_mut(index) else {
            return Err("effects fader index is out of range");
        };
        *fader = value.min(127) as u8;
        Ok(())
    }

    /// Invalidates feedback after reconnect or scene activation.
    pub const fn request_resync(&mut self) {
        self.resync_required = true;
    }

    /// Clears the resync marker after a complete feedback publication.
    pub const fn acknowledge_resync(&mut self) {
        self.resync_required = false;
    }
}

impl Default for EffectsGroupRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// One documented parameter assignment on the fixed effects faceplate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectsParameterAssignment {
    /// Stable profile parameter identifier.
    pub parameter_id: String,
    /// Physical control index, when assigned.
    pub control_index: Option<u8>,
    /// Display unit.
    pub unit: String,
    /// Legal inclusive range copied from profile metadata.
    pub range: (u16, u16),
    /// Conservative default value.
    pub default_value: u16,
    /// Direction of the physical control.
    pub direction: String,
    /// Explicit reason when this parameter cannot be assigned.
    pub unsupported_reason: Option<String>,
}

/// Derives bounded fixed-control assignments from profile-owned parameters.
#[must_use]
pub fn effects_parameter_assignments(
    faceplate: &EffectsFaceplateCatalog,
    profile: &DeviceProfile,
) -> Vec<EffectsParameterAssignment> {
    let owned_indices = faceplate
        .groups
        .iter()
        .filter(|group| group.owner == profile.id)
        .flat_map(|group| group.parameter_indices.iter().copied())
        .collect::<Vec<_>>();
    destination_parameters(profile)
        .into_iter()
        .take(128)
        .enumerate()
        .map(|(index, parameter)| {
            let control_index = owned_indices.get(index).copied();
            EffectsParameterAssignment {
                parameter_id: parameter.id,
                control_index,
                unit: "value".into(),
                range: parameter.range,
                default_value: parameter.range.0,
                direction: "clockwise-increases".into(),
                unsupported_reason: control_index.is_none().then(|| {
                    "no fixed faceplate control is available for this documented parameter".into()
                }),
            }
        })
        .collect()
}

/// One bounded operation in the immutable effects signal path.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectsAutomationOperation {
    /// Fixed group being changed.
    pub group_id: String,
    /// Profile owner responsible for the operation.
    pub owner: String,
    /// Operation kind (`enable` or `type`), never a guessed wire message.
    pub operation: String,
    /// Whether the owning profile documents the operation.
    pub supported: bool,
    /// Actionable reason when unsupported or unverified.
    pub reason: Option<String>,
}

/// Plans enabled effect-group changes in the immutable signal-path order.
#[must_use]
pub fn plan_effects_automation(
    faceplate: &EffectsFaceplateCatalog,
    enabled_groups: &[String],
    pickup_ready: bool,
) -> Vec<EffectsAutomationOperation> {
    const OWNER_ORDER: [&str; 2] = ["eventide.micropitch", "lexicon.reflex"];
    let mut operations = Vec::new();
    for owner in OWNER_ORDER {
        for group in faceplate.groups.iter().filter(|group| group.owner == owner) {
            if enabled_groups.iter().any(|id| id == &group.id) {
                operations.push(EffectsAutomationOperation {
                    group_id: group.id.clone(),
                    owner: group.owner.clone(),
                    operation: "enable".into(),
                    supported: false,
                    reason: Some(
                        if pickup_ready {
                            "profile-backed write operation is not verified"
                        } else {
                            "awaiting pickup before operation can be armed"
                        }
                        .into(),
                    ),
                });
            }
        }
    }
    operations
}

/// Minimal reusable effects configuration generated from selected faceplate groups.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReusableEffectsConfiguration {
    /// Deterministic signal-path name.
    pub name: String,
    /// Selected group identifiers in signal-path order.
    pub groups: Vec<String>,
    /// Documented parameter assignments included in this configuration.
    pub assignments: Vec<EffectsParameterAssignment>,
}

/// Generates a minimal reusable configuration without unrelated groups.
#[must_use]
pub fn generate_reusable_effects_configuration(
    faceplate: &EffectsFaceplateCatalog,
    profiles: &[DeviceProfile],
    enabled_groups: &[String],
) -> ReusableEffectsConfiguration {
    let groups = faceplate
        .groups
        .iter()
        .filter(|group| enabled_groups.iter().any(|id| id == &group.id))
        .map(|group| group.id.clone())
        .collect::<Vec<_>>();
    let assignments = faceplate
        .groups
        .iter()
        .filter(|group| groups.iter().any(|id| id == &group.id))
        .flat_map(|group| profiles.iter().filter(move |profile| profile.id == group.owner))
        .flat_map(|profile| effects_parameter_assignments(faceplate, profile))
        .filter(|assignment| assignment.control_index.is_some())
        .collect();
    let name = if groups.is_empty() {
        "Effects — empty".into()
    } else {
        format!("Effects — {}", groups.join(" → "))
    };
    ReusableEffectsConfiguration { name, groups, assignments }
}

/// One deterministic offline effects demo frame.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectsDemoFrame {
    /// Frame sequence number.
    pub sequence: u8,
    /// Group states in fixed faceplate order.
    pub groups: Vec<EffectGroupState>,
    /// Eight bounded fader values.
    pub faders: Vec<u8>,
    /// Whether the frame represents a reconnect resynchronization.
    pub resync: bool,
}

/// Builds bounded, hardware-free effects demo frames for qualification.
#[must_use]
pub fn effects_demo_frames() -> Vec<EffectsDemoFrame> {
    let mut frames = Vec::with_capacity(4);
    for sequence in 0..4 {
        frames.push(EffectsDemoFrame {
            sequence,
            groups: match sequence {
                0 => vec![EffectGroupState::Unknown; 6],
                1 => vec![EffectGroupState::Enabled; 6],
                2 => vec![EffectGroupState::Selected; 6],
                _ => vec![EffectGroupState::Unavailable; 6],
            },
            faders: (0..8)
                .map(|index| sequence.saturating_mul(32).saturating_add(index * 8).min(127))
                .collect(),
            resync: sequence == 3,
        });
    }
    frames
}

impl EffectsFaceplateCatalog {
    /// Validates the fixed physical-index partition.
    ///
    /// # Errors
    ///
    /// Returns an error when the six groups or their physical indices overlap or are incomplete.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.groups.len() != 6 || self.fader_indices != (40..48).collect::<Vec<_>>() {
            return Err("effects faceplate geometry is incomplete");
        }
        let mut seen = std::collections::BTreeSet::new();
        for group in &self.groups {
            if group.parameter_indices.len() != 4
                || group.parameter_control_ids.len() != 4
                || group.parameter_control_ids.windows(2).any(|pair| pair[0] == pair[1])
                || group.parameter_indices.iter().any(|index| !seen.insert(*index))
                || !seen.insert(group.enable_index)
                || !seen.insert(group.type_index)
            {
                return Err("effects faceplate indices conflict");
            }
        }
        if self.fader_control_ids
            != (1..=8).map(|n| PhysicalControlId(format!("fader-{n}"))).collect::<Vec<_>>()
            || self.unused_control_ids.len() != self.unused_indices.len()
        {
            return Err("effects faceplate stable identities are incomplete");
        }
        if self.unused_indices.iter().any(|index| !seen.insert(*index)) {
            return Err("effects faceplate unused indices conflict");
        }
        Ok(())
    }
}

/// Returns the fixed Launch Control XL effects faceplate.
#[must_use]
pub fn launch_control_effects_faceplate() -> EffectsFaceplateCatalog {
    let groups = [
        ("gain", "Row 1", "Gain", "eventide.micropitch", 24, 25, 0),
        ("gate", "Row 1", "Gate", "eventide.micropitch", 26, 27, 4),
        ("compressor", "Row 2", "Compressor", "eventide.micropitch", 28, 29, 8),
        ("modulation", "Row 2", "Modulation", "eventide.micropitch", 30, 31, 12),
        ("delay", "Row 3", "Delay", "lexicon.reflex", 32, 33, 16),
        ("reverb", "Row 3", "Reverb", "lexicon.reflex", 34, 35, 20),
    ]
    .into_iter()
    .map(|(id, row, label, owner, enable_index, type_index, start)| EffectsFaceplateGroup {
        id: id.into(),
        row: row.into(),
        label: label.into(),
        owner: owner.into(),
        parameter_indices: (start..start + 4).collect(),
        parameter_control_ids: (start..start + 4)
            .map(|index| PhysicalControlId(format!("knob-r{}-c{}", index / 8 + 1, index % 8 + 1)))
            .collect(),
        enable_index,
        enable_control_id: PhysicalControlId(format!(
            "button-r{}-c{}",
            (enable_index - 24) / 8 + 1,
            (enable_index - 24) % 8 + 1
        )),
        type_index,
        type_control_id: PhysicalControlId(format!(
            "button-r{}-c{}",
            (type_index - 24) / 8 + 1,
            (type_index - 24) % 8 + 1
        )),
    })
    .collect();
    EffectsFaceplateCatalog {
        groups,
        fader_indices: (40..48).collect(),
        fader_control_ids: (1..=8).map(|n| PhysicalControlId(format!("fader-{n}"))).collect(),
        unused_indices: (36..40).collect(),
        unused_control_ids: (5..=8).map(|n| PhysicalControlId(format!("button-r2-c{n}"))).collect(),
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
    /// Optional user-facing destination summary for the mapped control.
    #[serde(default)]
    pub destination: Option<String>,
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
            if assignment
                .destination
                .as_deref()
                .is_some_and(|destination| destination.trim().is_empty() || destination.len() > 96)
            {
                return Err("Launch Control assignment destination is invalid");
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

    /// Resolves one observed MIDI control to a physical faceplate index.
    ///
    /// Returns `None` when no assignment matches or when multiple assignments
    /// match, preventing activity from being shown on an invented control.
    #[must_use]
    pub fn resolve_activity_control(
        &self,
        channel: u8,
        number: u8,
        kind: LaunchControlMessageKind,
    ) -> Option<u8> {
        if channel > 15 || number > 127 {
            return None;
        }
        let mut result = None;
        for assignment in self.assignments.iter().filter(|assignment| {
            assignment.channel == channel && assignment.number == number && assignment.kind == kind
        }) {
            if result.is_some() {
                return None;
            }
            result = Some(assignment.index);
        }
        result
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

/// Encodes one bounded background update containing multiple LED index/value pairs.
///
/// Callers must provide pairs in deterministic index order. Duplicate indices are
/// rejected so one frame cannot contain competing states for the same control.
#[must_use]
pub fn encode_launch_control_led_batch(template: u8, pairs: &[(u8, u8)]) -> Option<Vec<u8>> {
    if template >= 16 || pairs.is_empty() || pairs.len() > 48 {
        return None;
    }
    let mut bytes = LAUNCH_CONTROL_XL_SYSEX_HEADER.to_vec();
    bytes.extend_from_slice(&[0x78, template]);
    let mut previous = None;
    for (index, value) in pairs {
        if *index >= 48 || previous.is_some_and(|prior| *index <= prior) {
            return None;
        }
        bytes.extend_from_slice(&[*index, *value & 0x7F]);
        previous = Some(*index);
    }
    bytes.push(0xF7);
    Some(bytes)
}

/// Encodes the documented Mk1 template-selection `SysEx` message.
#[must_use]
pub fn encode_launch_control_template(template: u8) -> Option<[u8; 9]> {
    (template < 16).then_some([0xF0, 0x00, 0x20, 0x29, 0x02, 0x11, 0x77, template, 0xF7])
}

/// Encodes the template-scoped Launch Control XL reset command.
#[must_use]
pub const fn encode_launch_control_reset(template: u8) -> Option<[u8; 3]> {
    if template < 16 {
        Some([0xB0 | template, 0x00, 0x00])
    } else {
        None
    }
}

/// Returns the documented Mk1 frame selecting User 1 (template slot zero).
#[must_use]
pub const fn launch_control_user1_selection_frame() -> [u8; 9] {
    [0xF0, 0x00, 0x20, 0x29, 0x02, 0x11, 0x77, 0, 0xF7]
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
            LaunchControlIdentity::Mk2
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
    } else if normalized.contains("launch control xl") {
        LaunchControlIdentity::Mk2
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
    let level: u8 = match intensity {
        0 => 0,
        1..=42 => 1,
        43..=84 => 2,
        _ => 3,
    };
    let (red, green) = match color {
        LedColor::Red => (level, 0),
        LedColor::Green => (0, level),
        LedColor::Amber => (level, level),
        // The XL distinguishes full yellow (0x3e) from full amber (0x3f)
        // by reducing the red component one step.
        LedColor::Yellow => (level.saturating_sub(1), level),
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

/// Encodes one scheduled logical LED state using the documented Mk1 address map.
///
/// Keeping this conversion beside the profile prevents daemon/TUI callers from
/// constructing device-specific `SysEx` bytes or silently addressing reserved LEDs.
#[must_use]
pub fn encode_launch_control_feedback(template: u8, index: u8, state: LedState) -> Option<Vec<u8>> {
    if matches!(state.color, LedColor::Unknown) {
        return None;
    }
    encode_launch_control_led(
        template,
        index,
        launch_control_led_value(
            state.color,
            state.intensity,
            if state.blink { 0x08 } else { 0x0c },
        ),
    )
}

/// Returns the deterministic Mk1 LED test pattern.
///
/// Every documented LED index is addressed once, using the device's full
/// supported color palette. The caller owns transmission, so this is safe to
/// use in offline demos and previews.
#[must_use]
pub fn launch_control_led_test_pattern(template: u8) -> Option<Vec<Vec<u8>>> {
    let colors = [LedColor::Red, LedColor::Green, LedColor::Amber, LedColor::Yellow];
    (template < 16)
        .then(|| {
            (0..48)
                .map(|index| {
                    let color = colors[usize::from(index) % colors.len()];
                    encode_launch_control_led(
                        template,
                        index,
                        launch_control_led_value(color, 127, 0x0c),
                    )
                })
                .collect::<Option<Vec<_>>>()
        })
        .flatten()
}

/// Returns four deterministic demo frames covering the supported semantic states.
///
/// Frames contain protocol messages only; no hardware I/O or timing occurs
/// here, making the demo reproducible in the TUI, CLI, and tests.
#[must_use]
pub fn launch_control_led_demo_frames(template: u8) -> Option<Vec<Vec<Vec<u8>>>> {
    let states = [
        LedState::new(LedColor::Off, 0, false),
        LedState::new(LedColor::Green, 127, false),
        LedState::new(LedColor::Amber, 127, false),
        LedState::new(LedColor::Red, 127, false),
    ];
    (template < 16)
        .then(|| {
            states
                .into_iter()
                .map(|state| {
                    (0..48)
                        .map(|index| {
                            encode_launch_control_led(
                                template,
                                index,
                                launch_control_led_value(state.color, state.intensity, 0x0c),
                            )
                        })
                        .collect::<Option<Vec<_>>>()
                })
                .collect::<Option<Vec<_>>>()
        })
        .flatten()
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

    /// Returns how many desired frames still differ from last-sent state.
    #[must_use]
    pub fn pending_len(&self) -> usize {
        self.desired.iter().filter(|(control, state)| self.sent.get(control) != Some(state)).count()
    }

    /// Clears last-sent tracking for one control after a failed write.
    pub fn revert_sent(&mut self, control: u8) {
        self.sent.remove(&control);
    }

    /// Returns the last successfully sent state for one LED index.
    #[must_use]
    pub fn actual(&self, control: u8) -> Option<LedState> {
        self.sent.get(&control).copied()
    }

    /// Returns the desired state for one LED index.
    #[must_use]
    pub fn desired(&self, control: u8) -> Option<LedState> {
        self.desired.get(&control).copied()
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
    /// Optional profile-specific operation identifier.
    #[serde(default)]
    pub operation: Option<String>,
}

/// Renderer-neutral support state for a destination parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ParameterSupport {
    /// The profile documents safe read/write interaction.
    ReadWrite,
    /// The parameter may be written but has no documented readback.
    WriteOnly,
    /// The parameter can be observed but must not be written.
    ReadOnly,
    /// The profile does not establish a synchronized current value.
    Unknown,
}

/// Plain-language reason a destination choice is or is not actionable.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SupportReason {
    /// Device and profile agree and the parameter can be used.
    Compatible,
    /// The destination device is not currently connected.
    Disconnected,
    /// The source role cannot operate this parameter.
    IncompatibleSourceRole,
    /// The parameter is observable but protected from writes.
    ReadOnly,
    /// The mapping is explicitly experimental and requires unsafe authorization.
    Experimental,
}

/// A parameter plus its bounded compatibility decision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompatibleParameter {
    /// Profile-owned parameter metadata.
    pub parameter: DestinationParameter,
    /// Compatibility result presented to the operator.
    pub reason: SupportReason,
}

/// Filters profile parameters using explicit role and connection facts.
#[must_use]
pub fn compatible_parameters(
    profile: &DeviceProfile,
    role: SourceRole,
    connected: bool,
) -> Vec<CompatibleParameter> {
    effect_blocks(profile)
        .into_iter()
        .flat_map(|block| block.parameters)
        .map(|parameter| {
            let reason = if !connected {
                SupportReason::Disconnected
            } else if parameter.support == ParameterSupport::ReadOnly {
                SupportReason::ReadOnly
            } else if parameter.evidence == Some(EvidenceLevel::Experimental) {
                SupportReason::Experimental
            } else if parameter.source_role.is_some_and(|expected| expected != role) {
                SupportReason::IncompatibleSourceRole
            } else {
                SupportReason::Compatible
            };
            CompatibleParameter { parameter, reason }
        })
        .collect()
}

/// Profile-owned destination parameter metadata for destination-first mapping.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DestinationParameter {
    /// Stable parameter identifier.
    pub id: String,
    /// Exact profile/documentation label.
    pub label: String,
    /// Bounded category used by keyboard-only browsing.
    pub category: String,
    /// Inclusive legal value range.
    pub range: (u16, u16),
    /// Support and feedback state.
    pub support: ParameterSupport,
    /// Whether a hazard marker is required before an action.
    pub hazardous: bool,
    /// Accepted physical source role, when explicitly established.
    #[serde(default)]
    pub source_role: Option<SourceRole>,
    /// Documented default value, when available.
    #[serde(default)]
    pub default_value: Option<u16>,
    /// Display units, when documented.
    #[serde(default)]
    pub units: Option<String>,
    /// Evidence level for this parameter contract.
    #[serde(default)]
    pub evidence: Option<EvidenceLevel>,
}

/// Accepted physical source roles for profile-owned parameters.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SourceRole {
    /// Continuous knob or fader input.
    Continuous,
    /// One-shot button action.
    ButtonAction,
    /// Two-state button toggle.
    ButtonToggle,
    /// Button cycling through documented values.
    ButtonCycle,
}

/// Strength of the profile evidence supporting a parameter mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EvidenceLevel {
    /// Confirmed by an authoritative device document.
    Documented,
    /// Captured or validated on the target device.
    Captured,
    /// Deliberately experimental and operator-gated.
    Experimental,
}

/// Stable profile-owned effect/block catalog entry.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectBlock {
    /// Stable block identifier.
    pub id: String,
    /// Exact profile-owned label.
    pub label: String,
    /// Signal-chain order.
    pub signal_order: u16,
    /// Parameters owned by this block.
    pub parameters: Vec<DestinationParameter>,
    /// Accepted source role for these parameters.
    pub source_role: SourceRole,
    /// Evidence supporting this block.
    pub evidence: EvidenceLevel,
}

/// Derives a deterministic, conservative block catalog from existing profile facts.
#[must_use]
pub fn effect_blocks(profile: &DeviceProfile) -> Vec<EffectBlock> {
    let known_block = match profile.effect_type {
        EffectType::Reverb => ("reverb", "Reverb", 2),
        EffectType::Delay => ("delay", "Delay", 1),
        EffectType::Modulation => ("modulation", "Modulation", 1),
        EffectType::Cabinet => ("cabinet", "Cabinet", 0),
        EffectType::Other => ("general", "General", 0),
    };
    let category = known_block.1.to_owned();
    let mut parameters = profile
        .controls
        .iter()
        .enumerate()
        .map(|(index, control)| DestinationParameter {
            id: control.operation.clone().unwrap_or_else(|| format!("control-{}", index + 1)),
            label: control.label.clone(),
            category: category.clone(),
            range: control.range,
            support: if control.cc.is_some() || control.program.is_some() {
                ParameterSupport::WriteOnly
            } else {
                ParameterSupport::Unknown
            },
            hazardous: false,
            source_role: Some(source_role(control)),
            default_value: Some(control.range.0),
            units: None,
            evidence: Some(EvidenceLevel::Documented),
        })
        .collect::<Vec<_>>();
    parameters.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.label.cmp(&b.label)));
    vec![EffectBlock {
        id: known_block.0.into(),
        label: known_block.1.into(),
        signal_order: known_block.2,
        parameters,
        source_role: SourceRole::Continuous,
        evidence: EvidenceLevel::Documented,
    }]
}

/// Derives a bounded destination catalog from documented profile controls.
#[must_use]
pub fn destination_parameters(profile: &DeviceProfile) -> Vec<DestinationParameter> {
    profile
        .controls
        .iter()
        .enumerate()
        .take(128)
        .map(|(index, control)| {
            let category = control
                .operation
                .as_deref()
                .and_then(|operation| operation.split([':', '/']).next())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("General")
                .to_owned();
            DestinationParameter {
                id: control.operation.clone().unwrap_or_else(|| format!("control-{index}")),
                label: control.label.clone(),
                category,
                range: control.range,
                support: if control.cc.is_some() || control.program.is_some() {
                    ParameterSupport::ReadWrite
                } else {
                    ParameterSupport::Unknown
                },
                hazardous: false,
                source_role: Some(source_role(control)),
                default_value: Some(control.range.0),
                units: None,
                evidence: Some(EvidenceLevel::Documented),
            }
        })
        .collect()
}

fn source_role(control: &ControlDefinition) -> SourceRole {
    if control.program.is_some() && control.cc.is_none() {
        SourceRole::ButtonAction
    } else if control.label.eq_ignore_ascii_case("ACTIVE/BYPASS")
        || (control.range.0 == 0 && control.range.1 == 1)
    {
        SourceRole::ButtonToggle
    } else if control.range.0 == control.range.1 && control.operation.is_some() {
        SourceRole::ButtonAction
    } else {
        SourceRole::Continuous
    }
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
    /// Renders a documented parameter by its stable operation/parameter identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when the parameter is unknown, read-only, or outside its declared range.
    pub fn render_parameter_message(
        &self,
        parameter_id: &str,
        channel: u8,
        value: u16,
    ) -> Result<Vec<u8>, &'static str> {
        if self.id == "lexicon.reflex" {
            if let Some(algorithm) = parameter_id.strip_prefix("reflex.algorithm-") {
                let algorithm = algorithm.parse::<u8>().map_err(|_| "invalid Reflex algorithm")?;
                return lexicon_reflex::encode_algorithm_selection(
                    algorithm,
                    channel.saturating_sub(1),
                );
            }
            if let Some(preset) = parameter_id.strip_prefix("pcm70_reflex:") {
                return lexicon_reflex::encode_pcm70_translation(preset, channel.saturating_sub(1));
            }
            let parameter = parameter_id
                .strip_prefix("reflex.parameter-")
                .and_then(|value| value.parse::<u8>().ok())
                .ok_or("parameter is not declared by profile")?;
            if parameter > 9 {
                return Err("parameter is outside the Reflex range");
            }
            return lexicon_reflex::encode_nibblized_parameter(
                channel.saturating_sub(1),
                parameter,
                value,
            );
        }
        let control = self
            .controls
            .iter()
            .enumerate()
            .find(|(index, control)| {
                control.operation.as_deref() == Some(parameter_id)
                    || parameter_id == format!("control-{index}")
            })
            .map(|(_, control)| control)
            .ok_or("parameter is not declared by profile")?;
        self.render_control_message(&control.label, channel, value)
    }

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
        if let Some(operation) = control.operation.as_deref() {
            return match operation {
                operation if operation.starts_with("pcm70_reflex:") => operation
                    .strip_prefix("pcm70_reflex:")
                    .ok_or("control operation is unsupported")
                    .and_then(|preset| {
                        lexicon_reflex::encode_pcm70_translation(preset, channel - 1)
                    }),
                _ => Err("control operation is unsupported"),
            };
        }
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
            if control.operation.is_none() && control.cc.is_some() == control.program.is_some() {
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
mod tests;
