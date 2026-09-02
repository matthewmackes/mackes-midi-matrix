//! Ratatui presentation boundary; this crate must not open MIDI ports.

mod assignment_catalog;
mod led_status;

pub use assignment_catalog::assignment_catalog_lines;

use mackes_ipc::{StateEvent, StateSnapshot};
use mackes_midi_engine::MidiLearnCandidate;
use mackes_profiles::{
    launch_control_index_label, LaunchControlMessageKind, LaunchControlTemplate,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Reducer error when reconnect continuity cannot be proven.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReducerError {
    /// Event sequence is not contiguous.
    SequenceGap,
}

/// Client-owned TUI state reconstructed only from daemon IPC data.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ClientState {
    /// Last applied daemon event sequence.
    pub last_sequence: u64,
    /// Opaque daemon snapshot payload.
    pub payload: Vec<u8>,
}

/// Commands exposed by the TUI command palette.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiCommand {
    /// Move to the next scene.
    NextScene,
    /// Move to the previous scene.
    PreviousScene,
    /// Trigger the daemon panic command.
    Panic,
    /// Open the command palette.
    OpenPalette,
    /// Quit the client.
    Quit,
    /// Move focus up.
    MoveUp,
    /// Move focus down.
    MoveDown,
    /// Move focus left.
    MoveLeft,
    /// Move focus right.
    MoveRight,
    /// Open a numbered workspace directly.
    OpenWorkspace(u8),
}

/// Musician-facing application sections in navigation order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppSection {
    /// Performance and live status.
    Live,
    /// Controller assignment workflow.
    MapControls,
    /// Scene and setlist management.
    Scenes,
    /// Connected devices and profiles.
    Devices,
    /// Diagnostics, monitor, and backups.
    System,
}

impl AppSection {
    /// Returns the stable display label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Live => "Live",
            Self::MapControls => "Map Controls",
            Self::Scenes => "Scenes",
            Self::Devices => "Devices",
            Self::System => "System",
        }
    }
    /// Returns the concise operator-facing purpose of the section.
    #[must_use]
    pub const fn purpose(self) -> &'static str {
        match self {
            Self::Live => "Perform and monitor the active scene",
            Self::MapControls => "Assign physical controls to parameters",
            Self::Scenes => "Recall scenes and organize setlists",
            Self::Devices => "Inspect connected devices and profiles",
            Self::System => "Review diagnostics, monitor, and backups",
        }
    }
    /// Returns sections in persistent rail order.
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [Self::Live, Self::MapControls, Self::Scenes, Self::Devices, Self::System]
    }
}

/// One reducer-owned focus location, distinct from live activity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FocusPath {
    /// Current application section.
    pub section: AppSection,
    /// Stable target within that section.
    pub target: String,
}

impl FocusPath {
    /// Creates a bounded focus path.
    ///
    /// # Errors
    ///
    /// Returns an error when the target is empty or exceeds 64 characters.
    pub fn new(section: AppSection, target: impl Into<String>) -> Result<Self, &'static str> {
        let target = target.into();
        if target.trim().is_empty() || target.len() > 64 {
            return Err("focus target must be 1-64 characters");
        }
        Ok(Self { section, target })
    }
    /// Returns the exact breadcrumb for the focused target.
    #[must_use]
    pub fn breadcrumb(&self) -> String {
        format!("{} / {}", self.section.label(), self.target)
    }
}

/// Primary keyboard actions for the task shell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellAction {
    /// Move focus to the previous item.
    Up,
    /// Move focus to the next item.
    Down,
    /// Move to the previous task section.
    Left,
    /// Move to the next task section.
    Right,
    /// Activate the focused item.
    Enter,
    /// Return to the parent task.
    Back,
    /// Toggle contextual help.
    Help,
}

/// Terminal-independent key input used by the executable adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellKey {
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Enter key.
    Enter,
    /// Escape key.
    Esc,
    /// Help key.
    Help,
    /// Other character input.
    Char(char),
}

/// Resolves primary shell keys and compatibility aliases.
#[must_use]
pub const fn shell_action_for_key(key: ShellKey) -> Option<ShellAction> {
    match key {
        ShellKey::Up => Some(ShellAction::Up),
        ShellKey::Down => Some(ShellAction::Down),
        ShellKey::Left => Some(ShellAction::Left),
        ShellKey::Right => Some(ShellAction::Right),
        ShellKey::Enter => Some(ShellAction::Enter),
        ShellKey::Esc => Some(ShellAction::Back),
        ShellKey::Help => Some(ShellAction::Help),
        ShellKey::Char(_) => None,
    }
}

/// Resolves Vim-style movement aliases to the same shell actions.
#[must_use]
pub const fn shell_action_for_char(key: char) -> Option<ShellAction> {
    match key {
        'k' => Some(ShellAction::Up),
        'j' => Some(ShellAction::Down),
        'h' => Some(ShellAction::Left),
        'l' => Some(ShellAction::Right),
        _ => None,
    }
}

/// Reducer-owned task-shell navigation state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskShellState {
    /// Current section and target.
    pub focus: FocusPath,
    /// Number of focusable targets in the current section.
    pub target_count: usize,
    /// Zero-based focused target position.
    pub target_index: usize,
    /// Whether contextual help is visible.
    pub help_visible: bool,
}

impl TaskShellState {
    /// Creates a shell at the Live landing task.
    ///
    /// # Errors
    ///
    /// Returns an error when no focusable target exists.
    pub fn initial(target_count: usize) -> Result<Self, &'static str> {
        if target_count == 0 {
            return Err("task shell requires a focus target");
        }
        Ok(Self {
            focus: FocusPath::new(AppSection::Live, "Target 1")?,
            target_count,
            target_index: 0,
            help_visible: false,
        })
    }

    /// Applies one primary navigation action with bounded focus movement.
    pub fn apply(&mut self, action: ShellAction) {
        match action {
            ShellAction::Up => {
                self.target_index = self.target_index.saturating_sub(1);
                self.refresh_target_label();
            }
            ShellAction::Down => {
                self.target_index = (self.target_index + 1).min(self.target_count - 1);
                self.refresh_target_label();
            }
            ShellAction::Left | ShellAction::Right => {
                let sections = AppSection::all();
                let current =
                    sections.iter().position(|section| *section == self.focus.section).unwrap_or(0);
                let next = if matches!(action, ShellAction::Left) {
                    current.saturating_sub(1)
                } else {
                    (current + 1).min(sections.len() - 1)
                };
                self.focus.section = sections[next];
                self.target_index = 0;
                self.refresh_target_label();
            }
            ShellAction::Enter | ShellAction::Back => self.help_visible = false,
            ShellAction::Help => self.help_visible = !self.help_visible,
        }
    }

    fn refresh_target_label(&mut self) {
        self.focus.target = format!("Target {}", self.target_index + 1);
    }
}

/// Maps TUI commands to the governed daemon command boundary.
#[must_use]
pub const fn ipc_command_for(command: UiCommand) -> Option<mackes_ipc::Command> {
    match command {
        UiCommand::NextScene | UiCommand::PreviousScene => Some(mackes_ipc::Command::Scenes),
        UiCommand::Panic => Some(mackes_ipc::Command::Panic),
        UiCommand::OpenPalette
        | UiCommand::Quit
        | UiCommand::MoveUp
        | UiCommand::MoveDown
        | UiCommand::MoveLeft
        | UiCommand::MoveRight
        | UiCommand::OpenWorkspace(_) => None,
    }
}

/// A deliberately explicit MIDI trigger for a dashboard command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DashboardMidiTrigger {
    /// Match a note-on on an exact one-based channel and note.
    NoteOn {
        /// One-based MIDI channel.
        channel: u8,
        /// Note number.
        note: u8,
    },
    /// Match a control change, optionally requiring an exact value.
    ControlChange {
        /// One-based MIDI channel.
        channel: u8,
        /// Controller number.
        controller: u8,
        /// Optional exact value; `None` matches every value.
        value: Option<u8>,
    },
}

/// One configured MIDI-to-dashboard command binding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DashboardMidiBinding {
    /// Trigger to match.
    pub trigger: DashboardMidiTrigger,
    /// Existing governed UI command to dispatch after a match.
    pub command: UiCommand,
}

/// Converts validated persisted bindings into runtime bindings for the TUI.
///
/// # Errors
///
/// Returns an error if a persisted command or trigger cannot be represented by
/// the runtime dashboard contract.
pub fn dashboard_bindings_from_config(
    bindings: &[mackes_config::DashboardMidiBinding],
) -> Result<Vec<DashboardMidiBinding>, &'static str> {
    let mut runtime = Vec::with_capacity(bindings.len().min(128));
    for binding in bindings.iter().take(128) {
        binding.validate()?;
        let trigger = match &binding.trigger {
            mackes_config::DashboardMidiTrigger::NoteOn { channel, note } => {
                DashboardMidiTrigger::NoteOn { channel: *channel, note: *note }
            }
            mackes_config::DashboardMidiTrigger::ControlChange { channel, controller, value } => {
                DashboardMidiTrigger::ControlChange {
                    channel: *channel,
                    controller: *controller,
                    value: *value,
                }
            }
        };
        let command = match binding.command.as_str() {
            "panic" => UiCommand::Panic,
            "next_scene" => UiCommand::NextScene,
            "previous_scene" => UiCommand::PreviousScene,
            _ => return Err("dashboard MIDI command is not allowed"),
        };
        runtime.push(DashboardMidiBinding { trigger, command });
    }
    if bindings.len() > 128 {
        return Err("dashboard MIDI binding limit exceeded");
    }
    if runtime.iter().enumerate().any(|(index, binding)| {
        runtime[..index].iter().any(|previous| previous.trigger == binding.trigger)
    }) {
        return Err("duplicate dashboard MIDI trigger");
    }
    Ok(runtime)
}

/// Resolves one MIDI message through explicit dashboard bindings.
///
/// Invalid MIDI ranges, unmapped messages, and ambiguous matches all fail closed.
#[must_use]
pub fn ui_command_for_midi(
    message: &mackes_domain::MidiMessage,
    bindings: &[DashboardMidiBinding],
) -> Option<UiCommand> {
    let mut match_result = None;
    for binding in bindings {
        let matched = match (&binding.trigger, message) {
            (
                DashboardMidiTrigger::NoteOn { channel, note },
                mackes_domain::MidiMessage::NoteOn {
                    channel: actual_channel,
                    note: actual_note,
                    ..
                },
            ) => {
                *channel >= 1
                    && *channel <= 16
                    && *note <= 127
                    && actual_channel.one_based() == *channel
                    && actual_note.as_u8() == *note
            }
            (
                DashboardMidiTrigger::ControlChange { channel, controller, value },
                mackes_domain::MidiMessage::ControlChange {
                    channel: actual_channel,
                    controller: actual_controller,
                    value: actual_value,
                },
            ) => {
                *channel >= 1
                    && *channel <= 16
                    && *controller <= 127
                    && value
                        .is_none_or(|expected| expected <= 127 && expected == actual_value.as_u8())
                    && actual_channel.one_based() == *channel
                    && actual_controller.as_u8() == *controller
            }
            _ => false,
        };
        if matched {
            if match_result.is_some() {
                return None;
            }
            match_result = Some(binding.command);
        }
    }
    match_result
}

/// Polls a bounded number of events and resolves explicit dashboard actions.
///
/// The input adapter remains owned by the daemon or application boundary; this
/// helper never opens ports, routes events, or dispatches commands itself.
#[must_use]
pub fn poll_dashboard_actions(
    input: &mut dyn mackes_midi_engine::MidiInputAdapter,
    bindings: &[DashboardMidiBinding],
    limit: usize,
) -> Vec<UiCommand> {
    let mut commands = Vec::with_capacity(limit.min(32));
    for _ in 0..limit.min(128) {
        let Some(event) = input.receive() else { break };
        if let Some(command) = ui_command_for_midi(&event.message, bindings) {
            commands.push(command);
        }
    }
    commands
}

/// Deterministic default keymap; device actions still travel through IPC.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Keymap;

/// Semantic color roles shared by all workspaces.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SemanticToken {
    /// Neutral setup/navigation state.
    Setup,
    /// MIDI transport state.
    Midi,
    /// `SysEx` transport state.
    Sysex,
    /// Healthy state.
    Success,
    /// Degraded state.
    Warning,
    /// Failed state.
    Error,
    /// Hazardous/destructive action.
    Hazard,
}

impl SemanticToken {
    /// Canonical registry required by every complete theme.
    pub const ALL: [Self; 7] = [
        Self::Setup,
        Self::Midi,
        Self::Sysex,
        Self::Success,
        Self::Warning,
        Self::Error,
        Self::Hazard,
    ];
}

/// Semantic display intensity independent of terminal color support.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenIntensity {
    /// Unavailable or disabled.
    Dim,
    /// Ordinary available state.
    Normal,
    /// Current selection.
    Selected,
    /// Hazardous or action-required state.
    Hazard,
}

/// Shared rack-appliance lamp state. The marker is intentionally textual so
/// monochrome terminals preserve the same meaning as ANSI terminals.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RackLamp {
    /// Device or feature is unavailable.
    Offline,
    /// Device or feature is available but inactive.
    Disabled,
    /// Device or feature is active.
    Enabled,
    /// Operator attention is required.
    Warning,
    /// An operation failed or is blocked.
    Error,
}

/// Renderer-neutral state for one profile-owned faceplate control.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceplateControlState {
    /// Device is disconnected or endpoint is offline.
    Offline,
    /// Device is present but no authoritative control assignment is known.
    Unknown,
    /// Control is known and not currently mapped.
    Unmapped,
    /// Control is mapped but has no recent activity.
    Mapped,
    /// Control is mapped and most recently active.
    Live,
}

/// Returns a stable ASCII marker for a faceplate control state.
#[must_use]
pub const fn faceplate_state_marker(state: FaceplateControlState) -> &'static str {
    match state {
        FaceplateControlState::Offline => "OFF",
        FaceplateControlState::Unknown => "UNK",
        FaceplateControlState::Unmapped => "---",
        FaceplateControlState::Mapped => "MAP",
        FaceplateControlState::Live => "LIVE",
    }
}

impl RackLamp {
    /// Returns the stable text marker used by every renderer.
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Offline => "o",
            Self::Disabled => "-",
            Self::Enabled => "*",
            Self::Warning => "!",
            Self::Error => "x",
        }
    }

    /// Returns the ANSI-16 color for this lamp.
    #[must_use]
    pub const fn color(self) -> Color {
        match self {
            Self::Offline => Color::DarkGray,
            Self::Disabled | Self::Error => Color::Red,
            Self::Enabled => Color::Green,
            Self::Warning => Color::Yellow,
        }
    }
}

/// Bounded value-bar view model shared by rack panels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RackValueBar {
    /// Current normalized value in the inclusive MIDI range.
    pub value: u8,
    /// Width available for the bar body.
    pub width: u16,
}

impl RackValueBar {
    /// Creates a bar, clamping its value and width to safe display bounds.
    #[must_use]
    pub fn new(value: u8, width: u16) -> Self {
        Self { value, width: width.min(64) }
    }

    /// Returns filled and unfilled cell counts without terminal access.
    #[must_use]
    pub fn cells(self) -> (u16, u16) {
        let filled = self.width * u16::from(self.value) / 127;
        (filled, self.width - filled)
    }
}

/// Builds a compact, color-independent lamp line for rack headers and panels.
#[must_use]
pub fn rack_lamp_line(label: &str, lamp: RackLamp) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{} ", lamp.marker()), Style::default().fg(lamp.color())),
        Span::raw(label.to_owned()),
    ])
}

/// Builds a bounded horizontal value bar using ASCII cells only.
#[must_use]
pub fn rack_value_bar_line(label: &str, bar: RackValueBar) -> Line<'static> {
    let (filled, empty) = bar.cells();
    Line::from(format!(
        "{label:<12} [{}{}] {value:>3}",
        "=".repeat(usize::from(filled)),
        "-".repeat(usize::from(empty)),
        value = bar.value
    ))
}

/// Required rack-shell regions after terminal-size adaptation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RackShellLayout {
    /// Whether secondary panels must collapse.
    pub compact: bool,
    /// Height reserved for the status band.
    pub status_rows: u16,
    /// Height reserved for the persistent alert band.
    pub alert_rows: u16,
    /// Height reserved for the footer legend.
    pub footer_rows: u16,
}

impl RackShellLayout {
    /// Computes a bounded layout while retaining critical operator controls.
    #[must_use]
    pub const fn for_terminal(width: u16, height: u16) -> Self {
        let compact = width < 100 || height < 37;
        Self { compact, status_rows: 1, alert_rows: 1, footer_rows: 1 }
    }
}

/// Renders the invariant rack-shell bands without terminal or daemon access.
///
/// The returned frame is bounded to the requested viewport and keeps the
/// status, alert, and panic controls visible in both layout modes.
#[must_use]
pub fn rack_shell_lines(
    layout: RackShellLayout,
    health: &str,
    alert: Option<&str>,
    panic_available: bool,
) -> Vec<String> {
    let width: usize = if layout.compact { 80 } else { 100 };
    let truncate = |text: &str| text.chars().take(width.saturating_sub(1)).collect::<String>();
    let mut lines = vec![truncate(&format!(
        "MACKES RACK | HEALTH {health} | PANIC {}",
        if panic_available { "READY" } else { "HELD" }
    ))];
    lines.push(truncate(&format!("ALERT  {}", alert.unwrap_or("none"))));
    lines.push(truncate(if layout.compact {
        "[1] HOME  [2] ROUTING  [3] DIAGNOSTICS"
    } else {
        "[1] HOME  [2] ROUTING  [3] SCENES  [4] DIAGNOSTICS  [!] PANIC"
    }));
    lines
}

/// Returns the stable text annotation for an intensity state.
#[must_use]
pub const fn intensity_marker(intensity: TokenIntensity) -> &'static str {
    match intensity {
        TokenIntensity::Dim => "(dim)",
        TokenIntensity::Normal => "",
        TokenIntensity::Selected => "(selected)",
        TokenIntensity::Hazard => "(hazard)",
    }
}

/// Returns the stable non-color marker for a semantic token.
#[must_use]
pub const fn token_marker(token: SemanticToken) -> &'static str {
    match token {
        SemanticToken::Setup => "[SETUP]",
        SemanticToken::Midi => "[MIDI]",
        SemanticToken::Sysex => "[SYSEX]",
        SemanticToken::Success => "[OK]",
        SemanticToken::Warning => "[WARN]",
        SemanticToken::Error => "[ERROR]",
        SemanticToken::Hazard => "[HAZARD]",
    }
}

/// One semantic palette assignment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaletteEntry {
    /// Semantic role being styled.
    pub token: SemanticToken,
    /// Foreground RGB color.
    pub foreground: (u8, u8, u8),
    /// Background RGB color.
    pub background: (u8, u8, u8),
}

/// Versioned user-selectable theme whose palette must cover the canonical token registry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Theme {
    /// Schema version for future migrations.
    pub version: u16,
    /// Complete semantic palette.
    pub palette: Vec<PaletteEntry>,
}

impl Theme {
    /// Validates a theme as a complete replacement palette.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported versions, missing/duplicate tokens, or contrast failures.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version == 0 {
            return Err("theme version must be nonzero");
        }
        validate_palette(&self.palette)?;
        if self.palette.len() != default_palette().len()
            || SemanticToken::ALL.iter().any(|token| palette_entry(&self.palette, *token).is_none())
        {
            return Err("theme must cover every semantic token");
        }
        Ok(())
    }
}

/// Validates a complete, duplicate-free semantic palette.
///
/// # Errors
///
/// Returns an error for empty palettes, duplicate roles, or insufficient
/// foreground/background contrast.
pub fn validate_palette(entries: &[PaletteEntry]) -> Result<(), &'static str> {
    if entries.is_empty() {
        return Err("palette must contain semantic entries");
    }
    for (index, entry) in entries.iter().enumerate() {
        if entries[..index].iter().any(|prior| prior.token == entry.token) {
            return Err("palette contains duplicate semantic token");
        }
        if !contrast_is_readable(entry.foreground, entry.background) {
            return Err("palette contrast is not readable");
        }
    }
    Ok(())
}

/// Looks up one semantic palette entry without allowing screen-local remapping.
#[must_use]
pub fn palette_entry(entries: &[PaletteEntry], token: SemanticToken) -> Option<PaletteEntry> {
    entries.iter().find(|entry| entry.token == token).copied()
}

/// Returns the built-in high-contrast palette covering every current token.
#[must_use]
pub const fn default_palette() -> [PaletteEntry; 7] {
    [
        PaletteEntry {
            token: SemanticToken::Setup,
            foreground: (255, 255, 255),
            background: (0, 0, 0),
        },
        PaletteEntry {
            token: SemanticToken::Midi,
            foreground: (255, 255, 255),
            background: (0, 0, 0),
        },
        PaletteEntry {
            token: SemanticToken::Sysex,
            foreground: (255, 255, 255),
            background: (0, 0, 0),
        },
        PaletteEntry {
            token: SemanticToken::Success,
            foreground: (255, 255, 255),
            background: (0, 0, 0),
        },
        PaletteEntry {
            token: SemanticToken::Warning,
            foreground: (255, 255, 255),
            background: (0, 0, 0),
        },
        PaletteEntry {
            token: SemanticToken::Error,
            foreground: (255, 255, 255),
            background: (0, 0, 0),
        },
        PaletteEntry {
            token: SemanticToken::Hazard,
            foreground: (255, 255, 255),
            background: (0, 0, 0),
        },
    ]
}

/// Returns whether an RGB foreground/background pair meets the terminal
/// contrast floor using relative luminance.
#[must_use]
#[allow(clippy::suboptimal_flops)]
pub fn contrast_is_readable(foreground: (u8, u8, u8), background: (u8, u8, u8)) -> bool {
    fn luminance((r, g, b): (u8, u8, u8)) -> f32 {
        fn linear(value: u8) -> f32 {
            let value = f32::from(value) / 255.0;
            if value <= 0.03928 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
    }
    let lighter = luminance(foreground).max(luminance(background));
    let darker = luminance(foreground).min(luminance(background));
    (lighter + 0.05) / (darker + 0.05) >= 4.5
}

/// Terminal viewport dimensions used by responsive layouts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Viewport {
    /// Width in columns.
    pub width: u16,
    /// Height in rows.
    pub height: u16,
}

/// Dashboard panels ordered by operator priority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DashboardPanel {
    /// Navigation/status footer.
    Navigation,
    /// Connection health.
    Health,
    /// Active scene.
    ActiveScene,
    /// Signal-flow diagram.
    SignalFlow,
    /// Recent event monitor.
    RecentEvents,
    /// Panic action.
    Panic,
}

/// Returns the canonical workspace name for a direct shortcut (1–9).
#[must_use]
pub const fn workspace_name(shortcut: u8) -> Option<&'static str> {
    match shortcut {
        1 => Some("Dashboard"),
        2 => Some("MIDI Learn"),
        3 => Some("Reflex"),
        4 => Some("Eventide"),
        5 => Some("Routing"),
        6 => Some("Diagnostics"),
        7 => Some("Monitor"),
        8 => Some("Backups"),
        9 => Some("Setlists"),
        _ => None,
    }
}

/// Returns panels visible at the given viewport density.
#[must_use]
pub fn dashboard_panels(compact: bool) -> Vec<DashboardPanel> {
    if compact {
        vec![
            DashboardPanel::Navigation,
            DashboardPanel::Health,
            DashboardPanel::ActiveScene,
            DashboardPanel::Panic,
        ]
    } else {
        vec![
            DashboardPanel::Navigation,
            DashboardPanel::Health,
            DashboardPanel::ActiveScene,
            DashboardPanel::SignalFlow,
            DashboardPanel::RecentEvents,
            DashboardPanel::Panic,
        ]
    }
}

/// Terminal lifecycle ownership guard state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TerminalGuard {
    raw_mode: bool,
    alternate_screen: bool,
}

impl TerminalGuard {
    /// Marks raw mode and alternate-screen ownership as acquired.
    pub const fn acquire(&mut self) {
        self.raw_mode = true;
        self.alternate_screen = true;
    }
    /// Releases all terminal ownership flags; safe to call repeatedly.
    pub const fn restore(&mut self) {
        self.raw_mode = false;
        self.alternate_screen = false;
    }
    /// Returns whether cleanup is still required.
    #[must_use]
    pub const fn needs_restore(self) -> bool {
        self.raw_mode || self.alternate_screen
    }
}

/// Compact dashboard state derived from daemon events.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct DashboardState {
    /// Active project/scene label.
    pub active_scene: Option<String>,
    /// Daemon health label.
    pub health: String,
    /// Current routing generation.
    pub route_generation: u64,
    /// Generation of the authoritative hardware-first mapping store.
    pub mapping_generation: u64,
    /// Whether the daemon retains one bounded mapping Undo record.
    pub mapping_undo_available: bool,
    /// Latest daemon-owned mapping source/result activity payload.
    pub mapping_activity: Option<serde_json::Value>,
    /// Bounded assignment feedback lines supplied by the authoritative wizard projection.
    pub assignment_feedback: Vec<String>,
    /// Whether a daemon-owned route Undo is available.
    pub route_undo_available: bool,
    /// Number of retained redacted mutation-audit decisions.
    pub audit_count: u64,
    /// Newest safe mutation-audit summary for operator visibility.
    pub latest_audit: Option<String>,
    /// Whether performance lock is active.
    pub performance_locked: bool,
    /// Panic is always available from the dashboard.
    pub panic_available: bool,
    /// Aggregate received-event count.
    pub received: u64,
    /// Aggregate sent-event count.
    pub sent: u64,
    /// Aggregate dropped-event count.
    pub dropped: u64,
    /// Activation actions completed versus total.
    pub activation_progress: (u32, u32),
    /// Most recent activation result summary, if one has been published.
    pub activation_result: Option<String>,
    /// Bounded per-device health labels and remediation state.
    pub device_health: Vec<(String, String)>,
    /// Bounded newest-first operator notifications.
    pub notifications: Vec<Notification>,
    /// Latest bounded source-to-destination MIDI activity.
    pub live_activity: Option<LiveActivity>,
    /// Local monotonic age of the displayed activity, in nanoseconds.
    pub live_activity_age_nanos: u64,
    /// Whether the mapping editor contains changes not yet committed to the daemon.
    pub mapping_dirty: bool,
    /// Selected output destination in the flattened visible inventory.
    pub selected_destination: Option<usize>,
    /// Selected input source in the flattened visible inventory.
    pub selected_input: Option<usize>,
    /// Selected parameter within the active profile-backed destination.
    pub selected_parameter: Option<usize>,
    /// Currently visible physical MIDI devices and their port identities.
    pub physical_devices: Vec<PhysicalDevice>,
    /// Optional imported or learned Launch Control assignment map.
    pub launch_control_template: Option<LaunchControlTemplate>,
    /// Profile-owned effects state; resync is required after reconnect/scene changes.
    pub effects_groups: mackes_profiles::EffectsGroupRuntime,
    /// Authoritative active mapping browser projection.
    pub mapping_browser: MappingBrowser,
    /// Latest daemon LED contract status line.
    pub led_status: Option<String>,
}

/// Renderer-safe physical MIDI device inventory record.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PhysicalDevice {
    /// Stable normalized device identity.
    pub id: String,
    /// Human-readable device name.
    pub name: String,
    /// Input endpoint identities.
    pub inputs: Vec<String>,
    /// Output endpoint identities.
    pub outputs: Vec<String>,
    /// Connection state reported by the daemon.
    pub state: String,
}

/// One renderer-safe MIDI activity sample from the daemon.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LiveActivity {
    /// Source endpoint identity.
    pub source_endpoint: u64,
    /// Stable inventory endpoint identity, when the daemon resolved it.
    pub source_endpoint_id: Option<String>,
    /// Stable source-control identity used by mapping projections.
    pub control_id: String,
    /// Monotonic source timestamp used for client-side age/highlight calculation.
    pub timestamp_nanos: u64,
    /// MIDI message family.
    pub kind: String,
    /// Zero-based MIDI channel when the message family has one.
    pub channel: Option<u8>,
    /// Optional note/controller/program number.
    pub number: Option<u8>,
    /// Optional MIDI value.
    pub value: Option<u16>,
    /// Routed destination endpoint identities.
    pub destination_endpoints: Vec<u64>,
    /// Source event sequence.
    pub sequence: u64,
}

/// A renderer-neutral notification suitable for a status area or overlay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Notification {
    /// Stable severity used by themed renderers.
    pub severity: SemanticToken,
    /// Human-readable, already-safe message.
    pub message: String,
}

/// Explicit phases of a MIDI Learn interaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LearnPhase {
    /// Ready to begin capture.
    Armed,
    /// Collecting input events.
    Capturing,
    /// Showing candidates for explicit review.
    Review,
    /// Choosing the mapping destination.
    Destination,
    /// Sending a live test.
    Testing,
    /// Mapping was explicitly committed.
    Committed,
    /// Capture was cancelled without saving.
    Cancelled,
}

/// Safe, renderer-neutral state for the MIDI Learn workspace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearnWorkspace {
    /// Current interaction phase.
    pub phase: LearnPhase,
    /// Inferred candidates awaiting explicit operator selection.
    pub candidates: Vec<MidiLearnCandidate>,
    /// Candidate selected for review/commit.
    pub selected: Option<usize>,
    /// Globally configured input endpoint alias; capture requires this to be set.
    pub learn_input_alias: Option<String>,
    /// Resolved stable endpoint ID used for bounded daemon capture.
    pub learn_endpoint_id: Option<String>,
    /// Explicit exact/any channel policy for the selected candidate.
    pub channel_policy: Option<LearnChannelPolicy>,
    /// Validated destination identifier and mapping class.
    pub destination: Option<(String, MappingMode)>,
    /// Whether the current candidate/destination pair passed its live test.
    pub live_test_passed: bool,
    /// Additional predicates selected for the learned mapping.
    pub filters: MappingFilterDraft,
}

/// Explicit MIDI-channel matching policy saved with a learned mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LearnChannelPolicy {
    /// Match the captured channel only.
    Exact(u8),
    /// Match the candidate signature on any channel.
    Any,
    /// Message has no MIDI channel.
    NotApplicable,
}

/// Transactional route draft used by the mapping editor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappingDraft {
    /// Source endpoint alias.
    pub source: String,
    /// Destination endpoint alias.
    pub destination: String,
    /// Optional profile-owned destination parameter selected by the operator.
    pub destination_parameter: Option<String>,
    /// Optional MIDI channel filter (1–16).
    pub channel: Option<u8>,
    /// Whether the draft is enabled.
    pub enabled: bool,
    /// Input mapping class.
    pub mode: MappingMode,
    /// Lower values execute first.
    pub priority: u16,
    /// Engine-owned value curve applied to continuous inputs.
    pub curve: mackes_midi_engine::Curve,
    /// Additional engine predicates applied by this mapping.
    pub filters: MappingFilterDraft,
    /// Explicitly authorizes this route to participate in a bounded cycle.
    pub allow_cycle: bool,
}

/// Visibility state for one authoritative mapping-browser row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingBrowserStatus {
    /// Mapping is enabled and its destination profile is available.
    Enabled,
    /// Mapping is retained but does not currently execute.
    Disabled,
    /// Destination profile/endpoint is not currently available.
    Offline,
    /// Mapping requires the bounded unsafe-arm window before execution.
    Experimental,
}

impl MappingBrowserStatus {
    /// Returns the bounded status marker shown beside a browser row.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Enabled => "ON",
            Self::Disabled => "OFF",
            Self::Offline => "OFFLINE",
            Self::Experimental => "EXPERIMENTAL",
        }
    }
}

/// Renderer-neutral row shown by the mapping browser.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappingBrowserRow {
    /// Full authoritative record used for explicit replacement.
    pub mapping: mackes_config::ControlMapping,
    /// Durable mapping identity.
    pub id: String,
    /// Stable physical control identity and musician-facing label.
    pub physical_control_id: String,
    /// Musician-facing label for the physical control.
    pub physical_label: String,
    /// Musical destination path.
    pub destination_path: String,
    /// Current source value, when observed.
    pub current_source_value: Option<u16>,
    /// Most recent destination value, when observed.
    pub last_destination_result: Option<u16>,
    /// Current authoritative status.
    pub status: MappingBrowserStatus,
}

/// Bounded browser projection and selection state. It never mutates daemon state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MappingBrowser {
    /// Stable, display-ready rows.
    pub rows: Vec<MappingBrowserRow>,
    /// Selected browser row, independent from the LIVE activity marker.
    pub selected: Option<usize>,
    /// First visible row for compact pagination.
    pub offset: usize,
}

/// Profile-safe button behavior shown in Advanced; the persisted contract remains unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingButtonMode {
    /// Send the mapped value only while the source is held.
    Momentary,
    /// Toggle between the mapped range endpoints on each press.
    Toggle,
    /// Use the destination profile's declared behavior.
    ProfileDefault,
}

/// Transactional Advanced inspector for one mapping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdvancedMappingEditor {
    /// Mapping being inspected.
    pub mapping_id: String,
    /// Current editable behavior, copied from the authoritative row.
    pub behavior: mackes_config::MappingBehavior,
    /// Profile-safe button mode visible to the operator.
    pub button_mode: MappingButtonMode,
    /// Inline validation error, if the last edit was rejected.
    pub error: Option<String>,
}

impl AdvancedMappingEditor {
    /// Creates an editor using the mapping and destination profile's legal range/defaults.
    #[must_use]
    pub fn from_mapping(mapping: &mackes_config::ControlMapping) -> Self {
        let mut behavior = mapping.behavior.clone();
        if let Some(profile) = mackes_profiles::builtin_profile(&mapping.destination_profile) {
            if let Some(parameter) = mackes_profiles::compatible_parameters(
                &profile,
                mackes_profiles::SourceRole::Continuous,
                true,
            )
            .into_iter()
            .find(|item| item.parameter.id == mapping.destination_parameter)
            {
                behavior.destination_range = parameter.parameter.range;
            }
        }
        Self {
            mapping_id: mapping.id.clone(),
            behavior,
            button_mode: MappingButtonMode::ProfileDefault,
            error: None,
        }
    }

    /// Applies an edit locally; the caller submits the resulting behavior through IPC.
    ///
    /// # Errors
    ///
    /// Returns the profile-independent validation error without changing the editor.
    pub fn set_behavior(&mut self, behavior: mackes_config::MappingBehavior) -> Result<(), String> {
        if let Err(error) = behavior.validate() {
            self.error = Some(error.to_owned());
            return Err(error.to_owned());
        }
        self.behavior = behavior;
        self.error = None;
        Ok(())
    }

    /// Edits the inclusive source range and validates it atomically.
    ///
    /// # Errors
    /// Returns an error when the range is invalid.
    pub fn set_source_range(&mut self, range: (u16, u16)) -> Result<(), String> {
        let mut behavior = self.behavior.clone();
        behavior.source_range = range;
        self.set_behavior(behavior)
    }

    /// Edits the inclusive destination range and validates it atomically.
    ///
    /// # Errors
    /// Returns an error when the range is invalid.
    pub fn set_destination_range(&mut self, range: (u16, u16)) -> Result<(), String> {
        let mut behavior = self.behavior.clone();
        behavior.destination_range = range;
        self.set_behavior(behavior)
    }

    /// Changes direction without mutating any other behavior field.
    ///
    /// # Errors
    /// Returns an error when the resulting behavior is invalid.
    pub fn set_invert(&mut self, invert: bool) -> Result<(), String> {
        let mut behavior = self.behavior.clone();
        behavior.invert = invert;
        self.set_behavior(behavior)
    }

    /// Selects one of the approved profile-independent curves.
    ///
    /// # Errors
    /// Returns an error when the curve is not approved or the resulting behavior is invalid.
    pub fn set_curve(&mut self, curve: &str) -> Result<(), String> {
        if !matches!(curve, "linear" | "square" | "square_root") {
            let error = "curve must be linear, square, or square_root".to_owned();
            self.error = Some(error.clone());
            return Err(error);
        }
        let mut behavior = self.behavior.clone();
        behavior.curve = curve.into();
        self.set_behavior(behavior)
    }
}

impl MappingBrowser {
    /// Builds a deterministic projection from authoritative mappings and optional activity.
    #[must_use]
    pub fn from_authoritative(
        mappings: &[mackes_config::ControlMapping],
        activity: Option<&serde_json::Value>,
    ) -> Self {
        let mut mappings = mappings.to_vec();
        mappings.sort_by_key(|mapping| {
            mackes_profiles::launch_control_physical_catalog()
                .iter()
                .position(|control| control.id.as_str() == mapping.physical_control_id)
                .unwrap_or(usize::MAX)
        });
        let rows = mappings
            .into_iter()
            .map(|mapping| {
                let (current_source_value, last_destination_result) = activity
                    .filter(|value| {
                        value.get("mapping_id").and_then(serde_json::Value::as_str)
                            == Some(mapping.id.as_str())
                    })
                    .map_or((None, None), |value| {
                        (
                            value
                                .get("source_value")
                                .and_then(serde_json::Value::as_u64)
                                .and_then(|v| u16::try_from(v).ok()),
                            value
                                .get("destination_value")
                                .and_then(serde_json::Value::as_u64)
                                .and_then(|v| u16::try_from(v).ok()),
                        )
                    });
                let status = if !mapping.enabled {
                    MappingBrowserStatus::Disabled
                } else if mackes_profiles::builtin_profile(&mapping.destination_profile).is_none() {
                    MappingBrowserStatus::Offline
                } else if mackes_profiles::builtin_profile(&mapping.destination_profile)
                    .is_some_and(|profile| {
                        mackes_profiles::compatible_parameters(
                            &profile,
                            mackes_profiles::SourceRole::Continuous,
                            true,
                        )
                        .iter()
                        .any(|parameter| {
                            parameter.parameter.id == mapping.destination_parameter
                                && parameter.parameter.evidence
                                    == Some(mackes_profiles::EvidenceLevel::Experimental)
                        })
                    })
                {
                    MappingBrowserStatus::Experimental
                } else {
                    MappingBrowserStatus::Enabled
                };
                let physical_label = mackes_profiles::launch_control_physical_catalog()
                    .iter()
                    .find(|control| control.id.as_str() == mapping.physical_control_id)
                    .map_or_else(
                        || mapping.physical_control_id.clone(),
                        |control| control.label.clone(),
                    );
                MappingBrowserRow {
                    mapping: mapping.clone(),
                    id: mapping.id.clone(),
                    physical_control_id: mapping.physical_control_id.clone(),
                    physical_label,
                    destination_path: format!(
                        "{} › {} › {}",
                        mapping.destination_profile,
                        mapping.destination_effect,
                        mapping.destination_parameter
                    ),
                    current_source_value,
                    last_destination_result,
                    status,
                }
            })
            .collect();
        Self { rows, selected: None, offset: 0 }
    }

    /// Selects a bounded row and returns its physical control for HUD focus sync.
    pub fn select(&mut self, index: usize) -> Option<&str> {
        let row = self.rows.get(index)?;
        self.selected = Some(index);
        row.physical_control_id.as_str().into()
    }

    /// Returns a compact page without wrapping or exposing hidden rows.
    #[must_use]
    pub fn page(&self, capacity: usize) -> &[MappingBrowserRow] {
        let end = self.offset.saturating_add(capacity).min(self.rows.len());
        self.rows.get(self.offset.min(self.rows.len())..end).unwrap_or(&[])
    }
}

/// Produces bounded browser lines for wide, standard, and compact layouts.
#[must_use]
pub fn mapping_browser_lines(browser: &MappingBrowser, viewport: Viewport) -> Vec<String> {
    let width = usize::from(viewport.width);
    let capacity = if viewport.width >= 120 {
        8
    } else if viewport.width >= 80 {
        5
    } else {
        3
    };
    let mut lines = vec![format!("MAPPINGS  {} total", browser.rows.len())];
    lines.extend(browser.page(capacity).iter().enumerate().map(|(offset, row)| {
        let index = browser.offset + offset;
        let marker = if browser.selected == Some(index) { ">" } else { " " };
        let source =
            row.current_source_value.map_or_else(|| "—".to_owned(), |value| value.to_string());
        let result =
            row.last_destination_result.map_or_else(|| "—".to_owned(), |value| value.to_string());
        format!(
            "{marker} {} | {} | {} | src={} dst={}",
            row.physical_label,
            row.destination_path,
            row.status.label(),
            source,
            result
        )
    }));
    lines.into_iter().map(|line| line.chars().take(width).collect()).collect()
}

/// Converts an authoritative mapping outcome into a concise inline recovery instruction.
#[must_use]
pub const fn mapping_outcome_recovery(outcome: mackes_ipc::MappingOutcome) -> &'static str {
    match outcome {
        mackes_ipc::MappingOutcome::Applied => "Applied",
        mackes_ipc::MappingOutcome::GenerationConflict => {
            "State changed; refresh mappings and retry"
        }
        mackes_ipc::MappingOutcome::Conflict => {
            "Occupied source/destination; choose Replace or Cancel"
        }
        mackes_ipc::MappingOutcome::PersistenceFailed => "Not saved; check config path and retry",
        mackes_ipc::MappingOutcome::Invalid => "Invalid mapping; correct the affected field",
        mackes_ipc::MappingOutcome::NothingToUndo => "Nothing to undo",
    }
}

/// Parses only the bounded typed outcome field for inline display.
#[must_use]
pub fn mapping_response_notice(response: &str) -> String {
    serde_json::from_str::<mackes_ipc::MappingResult>(response).map_or_else(
        |_| "Mapping request failed; retry".to_owned(),
        |result| mapping_outcome_recovery(result.outcome).to_owned(),
    )
}

/// TUI projection for the daemon-owned assignment session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssignmentWizard {
    /// Last authoritative session projection.
    pub session: mackes_ipc::AssignmentSession,
    /// Section to restore after cancellation or completion.
    pub prior_section: AppSection,
    /// Candidate identities captured during the bounded uniqueness window.
    pub candidates: Vec<String>,
    /// Client generation used for the next typed request.
    pub generation: u64,
}

impl AssignmentWizard {
    /// Creates an idle wizard returning to Live.
    #[must_use]
    pub fn new() -> Self {
        Self {
            session: mackes_ipc::AssignmentSession::new("Live"),
            prior_section: AppSection::Live,
            candidates: Vec::new(),
            generation: 0,
        }
    }

    /// Enters capture from any task section and remembers the prior location.
    pub fn start(&mut self, section: AppSection) -> mackes_ipc::AssignmentRequest {
        self.prior_section = section;
        self.candidates.clear();
        self.request(mackes_ipc::AssignmentAction::Start, None)
    }

    /// Captures one stable physical ID; repeated IDs are debounced locally.
    pub fn capture(&mut self, physical_control_id: &str) -> Option<mackes_ipc::AssignmentRequest> {
        let control = mackes_profiles::launch_control_physical_catalog()
            .into_iter()
            .find(|control| control.id.as_str() == physical_control_id)?;
        if control.role == mackes_profiles::PhysicalControlRole::Utility {
            return None;
        }
        if self.candidates.iter().any(|candidate| candidate == physical_control_id) {
            return None;
        }
        self.candidates.push(physical_control_id.to_owned());
        Some(self.request(
            mackes_ipc::AssignmentAction::ControlCaptured,
            Some(physical_control_id.to_owned()),
        ))
    }

    /// Classifies a bounded simultaneous capture without guessing physical identity from MIDI.
    #[must_use]
    pub fn classify_capture(control_ids: &[&str]) -> mackes_ipc::CandidateCapture {
        mackes_ipc::classify_candidates(control_ids)
    }

    /// Builds a generation-checked typed action request for keyboard/controller parity.
    #[must_use]
    pub const fn request(
        &self,
        action: mackes_ipc::AssignmentAction,
        physical_control_id: Option<String>,
    ) -> mackes_ipc::AssignmentRequest {
        mackes_ipc::AssignmentRequest {
            generation: self.generation,
            action,
            physical_control_id,
            destination_profile: None,
            destination_effect: None,
            destination_parameter: None,
        }
    }

    /// Builds the typed commit request for one profile-owned parameter choice.
    #[must_use]
    pub fn destination_request(
        &self,
        choice: &AssignmentParameterChoice,
    ) -> mackes_ipc::AssignmentRequest {
        mackes_ipc::AssignmentRequest {
            generation: self.generation,
            action: mackes_ipc::AssignmentAction::Commit,
            physical_control_id: self.candidates.first().cloned(),
            destination_profile: Some(choice.profile_id.clone()),
            destination_effect: Some(choice.effect_id.clone()),
            destination_parameter: Some(choice.id.clone()),
        }
    }

    /// Builds a commit request from the browser's current bounded parameter selection.
    #[must_use]
    pub fn selected_destination_request(
        &self,
        choices: &AssignmentChoiceBrowser,
    ) -> Option<mackes_ipc::AssignmentRequest> {
        if self.candidates.first().is_some_and(|id| {
            mackes_profiles::launch_control_physical_catalog().into_iter().any(|control| {
                control.id.as_str() == id
                    && control.role == mackes_profiles::PhysicalControlRole::ChannelButton
            })
        }) {
            if let Some((preset_id, _)) = choices.presets.get(choices.selected) {
                return Some(self.destination_request(&AssignmentParameterChoice {
                    profile_id: "lexicon.reflex".into(),
                    effect_id: "reverb".into(),
                    effect_label: "Reverb".into(),
                    id: format!("pcm70_reflex:{preset_id}"),
                    label: preset_id.clone(),
                    reason: mackes_profiles::SupportReason::Compatible,
                }));
            }
        }
        choices.parameters.get(choices.selected).map(|choice| self.destination_request(choice))
    }

    /// Applies an authoritative result without creating local optimistic success.
    pub fn reconcile(&mut self, result: mackes_ipc::AssignmentResult) {
        self.generation = result.generation;
        self.session = result.session;
        if matches!(self.session.phase, mackes_ipc::AssignmentPhase::Idle) {
            self.candidates.clear();
        }
    }
}

impl Default for AssignmentWizard {
    fn default() -> Self {
        Self::new()
    }
}

/// Produces deterministic assignment feedback lines for the terminal renderer.
#[must_use]
pub fn assignment_wizard_lines(wizard: &AssignmentWizard, viewport: Viewport) -> Vec<String> {
    let phase = match wizard.session.phase {
        mackes_ipc::AssignmentPhase::Idle => "READY",
        mackes_ipc::AssignmentPhase::AwaitControl => "MOVE ONLY ONE CONTROL",
        mackes_ipc::AssignmentPhase::ChooseDevice => "CHOOSE DEVICE",
        mackes_ipc::AssignmentPhase::ChoosePreset => "CHOOSE PRESET",
        mackes_ipc::AssignmentPhase::ChooseEffect => "CHOOSE EFFECT",
        mackes_ipc::AssignmentPhase::ChooseType => "CHOOSE TYPE",
        mackes_ipc::AssignmentPhase::ChooseParameter => "CHOOSE PARAMETER",
        mackes_ipc::AssignmentPhase::ConfirmReplace => "REPLACE EXISTING MAPPING?",
        mackes_ipc::AssignmentPhase::Committing => "ASSIGNING",
        mackes_ipc::AssignmentPhase::Succeeded => "ASSIGNED",
        mackes_ipc::AssignmentPhase::Failed => "ASSIGNMENT FAILED — RETRY",
        mackes_ipc::AssignmentPhase::Interrupted => "INTERRUPTED — RESUME OR DISCARD",
    };
    let mut lines = vec![
        format!("ASSIGNMENT / {}", wizard.prior_section.label()),
        phase.to_owned(),
        format!(
            "Position: {} OF {}",
            wizard.session.index.saturating_add(1),
            wizard.session.total.max(1)
        ),
        format!("Candidates: {}", wizard.candidates.len()),
    ];
    if wizard.candidates.len() > 1 {
        lines.push("MOVE ONLY ONE CONTROL".into());
    }
    let width = usize::from(viewport.width);
    lines.into_iter().map(|line| line.chars().take(width).collect()).collect()
}

/// One profile-backed destination choice with an explicit support reason.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssignmentParameterChoice {
    /// Stable destination profile identity.
    pub profile_id: String,
    /// Stable profile parameter identity.
    pub id: String,
    /// Exact profile-owned label.
    pub label: String,
    /// Stable effect-block identity and label.
    pub effect_id: String,
    /// Exact effect-block label.
    pub effect_label: String,
    /// Compatibility reason shown to the operator.
    pub reason: mackes_profiles::SupportReason,
}

/// Deterministic device/effect/parameter chooser derived only from profile metadata.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AssignmentChoiceBrowser {
    /// Connected profile IDs in catalog order.
    pub devices: Vec<String>,
    /// Profile-owned preset choices shown between device and effect.
    pub presets: Vec<(String, String)>,
    /// Effect blocks for the selected profile in signal order.
    pub effects: Vec<(String, String)>,
    /// Parameter types shown between effect and parameter.
    pub types: Vec<String>,
    /// Role-filtered parameter choices in profile order.
    pub parameters: Vec<AssignmentParameterChoice>,
    /// Current bounded index at the active chooser level.
    pub selected: usize,
}

impl AssignmentChoiceBrowser {
    /// Creates a chooser from connected profiles and one captured physical role.
    #[must_use]
    pub fn from_profiles(
        connected_profile_ids: &[&str],
        selected_profile: Option<&str>,
        role: mackes_profiles::SourceRole,
    ) -> Self {
        let devices = connected_profile_ids
            .iter()
            .copied()
            .filter(|id| mackes_profiles::builtin_profile(id).is_some())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let Some(profile_id) = selected_profile.filter(|id| devices.iter().any(|item| item == id))
        else {
            return Self { devices, ..Self::default() };
        };
        let Some(profile) = mackes_profiles::builtin_profile(profile_id) else {
            return Self { devices, ..Self::default() };
        };
        let presets = if profile_id == "lexicon.reflex" {
            mackes_profiles::lexicon_reflex::pcm70_translations()
                .iter()
                .map(|preset| (preset.id.to_owned(), preset.name.to_owned()))
                .collect()
        } else {
            Vec::new()
        };
        let blocks = mackes_profiles::effect_blocks(&profile);
        let effects =
            blocks.iter().map(|block| (block.id.clone(), block.label.clone())).collect::<Vec<_>>();
        let mut parameters: Vec<AssignmentParameterChoice> =
            mackes_profiles::compatible_parameters(&profile, role, true)
                .into_iter()
                .filter(|item| {
                    matches!(
                        item.reason,
                        mackes_profiles::SupportReason::Compatible
                            | mackes_profiles::SupportReason::Experimental
                    )
                })
                .map(|item| {
                    let (effect_id, effect_label) = blocks
                        .iter()
                        .find(|block| {
                            block
                                .parameters
                                .iter()
                                .any(|parameter| parameter.id == item.parameter.id)
                        })
                        .map_or_else(
                            || ("general".to_owned(), "General".to_owned()),
                            |block| (block.id.clone(), block.label.clone()),
                        );
                    AssignmentParameterChoice {
                        profile_id: profile.id.clone(),
                        effect_id,
                        effect_label,
                        id: item.parameter.id,
                        label: item.parameter.label,
                        reason: item.reason,
                    }
                })
                .collect();
        // PCM70 translator controls are profile-owned preset selectors.  They
        // intentionally have a 0..1 value range, but are valid destinations
        // for a continuous controller such as a knob or fader.
        if parameters.is_empty() && profile_id == "lexicon.reflex" {
            parameters = if role == mackes_profiles::SourceRole::Continuous {
                mackes_profiles::lexicon_reflex::parameters(1)
                    .iter()
                    .map(|parameter| AssignmentParameterChoice {
                        profile_id: profile.id.clone(),
                        effect_id: "reverb".to_owned(),
                        effect_label: "Reverb".to_owned(),
                        id: format!("reflex.parameter-{}", parameter.number),
                        label: format!("{} ({})", parameter.description, parameter.mrc_name),
                        reason: mackes_profiles::SupportReason::Compatible,
                    })
                    .collect()
            } else {
                mackes_profiles::destination_parameters(&profile)
                    .into_iter()
                    .map(|parameter| AssignmentParameterChoice {
                        profile_id: profile.id.clone(),
                        effect_id: "reverb".to_owned(),
                        effect_label: "Reverb".to_owned(),
                        id: parameter.id,
                        label: parameter.label,
                        reason: mackes_profiles::SupportReason::Compatible,
                    })
                    .collect()
            };
        }
        let types = parameters.iter().map(|parameter| parameter.effect_label.clone()).fold(
            Vec::new(),
            |mut types, label| {
                if !types.contains(&label) {
                    types.push(label);
                }
                types
            },
        );
        Self { devices, presets, effects, types, parameters, selected: 0 }
    }

    /// Reconstructs the chooser from an authoritative daemon snapshot.
    #[must_use]
    pub fn from_session(session: &mackes_ipc::AssignmentSession) -> Self {
        Self {
            devices: session.catalog.devices.iter().map(|row| row.label.clone()).collect(),
            presets: session
                .catalog
                .presets
                .iter()
                .map(|row| (row.id.clone(), row.label.clone()))
                .collect(),
            effects: session
                .catalog
                .effects
                .iter()
                .map(|row| (row.id.clone(), row.label.clone()))
                .collect(),
            types: session.catalog.types.iter().map(|row| row.label.clone()).collect(),
            parameters: session
                .catalog
                .parameters
                .iter()
                .map(|row| AssignmentParameterChoice {
                    profile_id: session.catalog.selected_device.clone().unwrap_or_default(),
                    id: row.id.clone(),
                    label: row.label.clone(),
                    effect_id: row.group.clone().unwrap_or_default(),
                    effect_label: row.group.clone().unwrap_or_default(),
                    reason: mackes_profiles::SupportReason::Compatible,
                })
                .collect(),
            selected: usize::from(session.active_cursor()),
        }
    }

    /// Moves selection without wrapping and reports whether it changed.
    pub fn move_selection(&mut self, down: bool) -> bool {
        let count = self.parameters.len().max(self.effects.len()).max(self.devices.len());
        if count == 0 {
            return false;
        }
        let next = if down {
            self.selected.saturating_add(1).min(count - 1)
        } else {
            self.selected.saturating_sub(1)
        };
        let changed = next != self.selected;
        self.selected = next;
        changed
    }

    /// Moves selection within the bounded preset level.
    pub fn move_preset_selection(&mut self, down: bool) -> bool {
        if self.presets.is_empty() {
            return false;
        }
        let next = if down {
            self.selected.saturating_add(1).min(self.presets.len() - 1)
        } else {
            self.selected.saturating_sub(1)
        };
        let changed = next != self.selected;
        self.selected = next;
        changed
    }

    /// Moves selection within the bounded effect level.
    pub fn move_effect_selection(&mut self, down: bool) -> bool {
        if self.effects.is_empty() {
            return false;
        }
        let next = if down {
            self.selected.saturating_add(1).min(self.effects.len() - 1)
        } else {
            self.selected.saturating_sub(1)
        };
        let changed = next != self.selected;
        self.selected = next;
        changed
    }

    /// Moves selection within the bounded parameter-type level.
    pub fn move_type_selection(&mut self, down: bool) -> bool {
        if self.types.is_empty() {
            return false;
        }
        let next = if down {
            self.selected.saturating_add(1).min(self.types.len() - 1)
        } else {
            self.selected.saturating_sub(1)
        };
        let changed = next != self.selected;
        self.selected = next;
        changed
    }
}

/// Transactional fine-grained filter draft for a mapping editor.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MappingFilterDraft {
    /// Predicates applied after source/channel/message-class matching.
    pub predicates: Vec<mackes_midi_engine::RoutePredicate>,
}

impl MappingFilterDraft {
    /// Validates every predicate without mutating a route or daemon state.
    ///
    /// # Errors
    ///
    /// Returns the first engine predicate validation error.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.predicates.len() > 32 {
            return Err("mapping filter count exceeds bound");
        }
        self.predicates.iter().try_for_each(mackes_midi_engine::validate_route_predicate)
    }

    /// Returns a deterministic copy suitable for preview/submission.
    #[must_use]
    pub fn preview(&self) -> Vec<mackes_midi_engine::RoutePredicate> {
        self.predicates.clone()
    }
}

/// Converts persisted learned filters into validated engine predicates.
///
/// # Errors
///
/// Returns the first predicate validation error.
pub fn predicates_from_learned_filters(
    filters: &[mackes_config::LearnedFilter],
) -> Result<Vec<mackes_midi_engine::RoutePredicate>, &'static str> {
    let predicates = filters
        .iter()
        .map(|filter| match filter {
            mackes_config::LearnedFilter::NumberRange { minimum, maximum } => {
                mackes_midi_engine::RoutePredicate::NumberRange {
                    minimum: *minimum,
                    maximum: *maximum,
                }
            }
            mackes_config::LearnedFilter::ValueRange { minimum, maximum } => {
                mackes_midi_engine::RoutePredicate::ValueRange {
                    minimum: *minimum,
                    maximum: *maximum,
                }
            }
            mackes_config::LearnedFilter::Realtime { message } => {
                let message = match message {
                    mackes_config::LearnedRealtime::Clock => mackes_domain::RealtimeMessage::Clock,
                    mackes_config::LearnedRealtime::Start => mackes_domain::RealtimeMessage::Start,
                    mackes_config::LearnedRealtime::Continue => {
                        mackes_domain::RealtimeMessage::Continue
                    }
                    mackes_config::LearnedRealtime::Stop => mackes_domain::RealtimeMessage::Stop,
                    mackes_config::LearnedRealtime::ActiveSensing => {
                        mackes_domain::RealtimeMessage::ActiveSensing
                    }
                    mackes_config::LearnedRealtime::Reset => mackes_domain::RealtimeMessage::Reset,
                };
                mackes_midi_engine::RoutePredicate::Realtime(message)
            }
            mackes_config::LearnedFilter::SysExMask { pattern, mask } => {
                mackes_midi_engine::RoutePredicate::SysExMask {
                    pattern: pattern.clone(),
                    mask: mask.clone(),
                }
            }
        })
        .collect::<Vec<_>>();
    MappingFilterDraft { predicates: predicates.clone() }.validate()?;
    Ok(predicates)
}

/// Converts validated engine predicates into the persisted Learn filter model.
#[must_use]
pub fn learned_filters_from_predicates(
    predicates: &[mackes_midi_engine::RoutePredicate],
) -> Option<Vec<mackes_config::LearnedFilter>> {
    predicates
        .iter()
        .map(|predicate| match predicate {
            mackes_midi_engine::RoutePredicate::NumberRange { minimum, maximum } => {
                Some(mackes_config::LearnedFilter::NumberRange {
                    minimum: *minimum,
                    maximum: *maximum,
                })
            }
            mackes_midi_engine::RoutePredicate::ValueRange { minimum, maximum } => {
                Some(mackes_config::LearnedFilter::ValueRange {
                    minimum: *minimum,
                    maximum: *maximum,
                })
            }
            mackes_midi_engine::RoutePredicate::Realtime(message) => {
                Some(mackes_config::LearnedFilter::Realtime {
                    message: match message {
                        mackes_domain::RealtimeMessage::Clock => {
                            mackes_config::LearnedRealtime::Clock
                        }
                        mackes_domain::RealtimeMessage::Start => {
                            mackes_config::LearnedRealtime::Start
                        }
                        mackes_domain::RealtimeMessage::Continue => {
                            mackes_config::LearnedRealtime::Continue
                        }
                        mackes_domain::RealtimeMessage::Stop => {
                            mackes_config::LearnedRealtime::Stop
                        }
                        mackes_domain::RealtimeMessage::ActiveSensing => {
                            mackes_config::LearnedRealtime::ActiveSensing
                        }
                        mackes_domain::RealtimeMessage::Reset => {
                            mackes_config::LearnedRealtime::Reset
                        }
                    },
                })
            }
            mackes_midi_engine::RoutePredicate::SysExMask { pattern, mask } => {
                Some(mackes_config::LearnedFilter::SysExMask {
                    pattern: pattern.clone(),
                    mask: mask.clone(),
                })
            }
        })
        .collect()
}

/// Supported operator-facing mapping classes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingMode {
    /// Continuous controller.
    Cc,
    /// Program change.
    ProgramChange,
    /// Note event.
    Note,
    /// Pitch bend.
    PitchBend,
    /// System exclusive pattern.
    Sysex,
}

impl Default for MappingDraft {
    fn default() -> Self {
        Self {
            source: String::new(),
            destination: String::new(),
            destination_parameter: None,
            channel: None,
            enabled: false,
            mode: MappingMode::Cc,
            priority: 0,
            curve: mackes_midi_engine::Curve::Linear,
            filters: MappingFilterDraft { predicates: Vec::new() },
            allow_cycle: false,
        }
    }
}

/// Explicit phases for backup inspection and restore confirmation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupPhase {
    /// Showing available artifacts.
    Listing,
    /// Inspecting selected metadata.
    Inspecting,
    /// Dry-run plan is available.
    Planned,
    /// Operator confirmed the plan.
    Confirmed,
    /// Restore is being applied.
    Applying,
    /// Applied and read-back verified.
    Verified,
    /// Sent without read-back verification.
    SentUnverified,
    /// Operation failed.
    Failed,
}

impl From<mackes_config::BackupStatus> for BackupPhase {
    fn from(status: mackes_config::BackupStatus) -> Self {
        match status {
            mackes_config::BackupStatus::Verified => Self::Verified,
            mackes_config::BackupStatus::SentUnverified => Self::SentUnverified,
            mackes_config::BackupStatus::Failed => Self::Failed,
        }
    }
}

/// Renderer-neutral state for the backup workspace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupWorkspace {
    /// Current operation phase.
    pub phase: BackupPhase,
    /// Selected artifact path/label.
    pub selected: Option<String>,
    /// Last operation message, suitable for a notification area.
    pub message: Option<String>,
    /// Whether the source/target device identity mismatch requires operator attention.
    pub identity_warning: bool,
}

/// Risk class shown before any profile or raw device operation is sent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceOperationRisk {
    /// Read-only query or capture.
    ReadOnly,
    /// Volatile parameter change.
    VolatileWrite,
    /// Persistent or destructive device write.
    PersistentWrite,
}

/// Renderer-neutral preflight preview for a device operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceOperationPreview {
    /// Destination alias that will receive the operation.
    pub destination: String,
    /// Profile-owned operation label.
    pub operation: String,
    /// Policy risk shown to the operator.
    pub risk: DeviceOperationRisk,
}

impl DeviceOperationPreview {
    /// Creates a preview after validating operator-visible identity fields.
    ///
    /// # Errors
    ///
    /// Returns an error when destination or operation is blank.
    pub fn new(
        destination: impl Into<String>,
        operation: impl Into<String>,
        risk: DeviceOperationRisk,
    ) -> Result<Self, &'static str> {
        let destination = destination.into();
        let operation = operation.into();
        if destination.trim().is_empty() || operation.trim().is_empty() {
            return Err("device operation preview fields must not be empty");
        }
        Ok(Self { destination, operation, risk })
    }

    /// Returns whether policy requires explicit confirmation before sending.
    #[must_use]
    pub const fn requires_confirmation(&self) -> bool {
        !matches!(self.risk, DeviceOperationRisk::ReadOnly)
    }
}

/// Diagnostic severity used by the monitor workspace.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MonitorSeverity {
    /// Informational event.
    Info,
    /// Recoverable issue.
    Warning,
    /// Operation failure.
    Error,
}

/// Structured degraded-health explanation for the diagnostics workspace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthDiagnostic {
    /// Stable subsystem or endpoint identifier.
    pub subject: String,
    /// Severity presented to the operator.
    pub severity: MonitorSeverity,
    /// Observed cause, without raw payloads.
    pub reason: String,
    /// Concrete next action the operator can take.
    pub remediation: String,
}

impl HealthDiagnostic {
    /// Returns a compact, renderer-neutral diagnostic line.
    #[must_use]
    pub fn line(&self) -> String {
        format!("{:?} {}: {} -> {}", self.severity, self.subject, self.reason, self.remediation)
    }
}

/// Bounded newest-first health explanations for the diagnostics workspace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticsState {
    /// Newest diagnostics first.
    pub entries: Vec<HealthDiagnostic>,
}

/// Transactional setlist editor draft; persistence occurs only through `commit`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetlistEditor {
    /// Uncommitted ordered setlists.
    pub drafts: Vec<mackes_config::Setlist>,
    /// Currently selected setlist index.
    pub selected: Option<usize>,
    /// Project IDs available for assignment, projected from the daemon catalog.
    pub available_projects: Vec<String>,
}

impl SetlistEditor {
    /// Starts an editor from a persisted snapshot.
    #[must_use]
    pub fn from_snapshot(setlists: &[mackes_config::Setlist]) -> Self {
        Self { drafts: setlists.to_vec(), selected: None, available_projects: Vec::new() }
    }

    /// Adds a new empty setlist using a deterministic collision-safe identifier.
    ///
    /// The new row remains a local draft until the caller explicitly persists the editor.
    pub fn add_empty(&mut self) -> String {
        let base = "new-setlist";
        let mut suffix = 1_u32;
        let id = loop {
            let candidate = format!("{base}-{suffix}");
            if !self.drafts.iter().any(|setlist| setlist.id == candidate) {
                break candidate;
            }
            suffix = suffix.saturating_add(1);
        };
        self.drafts.push(mackes_config::Setlist { id: id.clone(), projects: Vec::new() });
        self.selected = Some(self.drafts.len() - 1);
        id
    }

    /// Selects a setlist by index.
    ///
    /// # Errors
    ///
    /// Returns an error when the index is outside the draft list.
    pub fn select(&mut self, index: usize) -> Result<(), &'static str> {
        if index >= self.drafts.len() {
            return Err("setlist index is out of range");
        }
        self.selected = Some(index);
        Ok(())
    }

    /// Appends a project to the selected setlist, rejecting duplicates and empty IDs.
    ///
    /// # Errors
    ///
    /// Returns an error when no setlist is selected, the ID is empty, or the project is already present.
    pub fn append_project(&mut self, project_id: impl Into<String>) -> Result<(), &'static str> {
        let index = self.selected.ok_or("no setlist selected")?;
        let project_id = project_id.into();
        if project_id.trim().is_empty() {
            return Err("project ID must not be empty");
        }
        if self.drafts[index].projects.iter().any(|project| project == &project_id) {
            return Err("project is already in setlist");
        }
        self.drafts[index].projects.push(project_id);
        Ok(())
    }

    /// Removes the last project from the selected setlist.
    ///
    /// # Errors
    ///
    /// Returns an error when no setlist is selected or it has no projects.
    pub fn remove_last_project(&mut self) -> Result<String, &'static str> {
        let index = self.selected.ok_or("no setlist selected")?;
        self.drafts[index].projects.pop().ok_or("selected setlist has no projects")
    }

    /// Rotates project order one position toward the requested edge.
    ///
    /// # Errors
    ///
    /// Returns an error when no setlist is selected or it has fewer than two projects.
    pub fn move_last_project(&mut self, toward_end: bool) -> Result<(), &'static str> {
        let index = self.selected.ok_or("no setlist selected")?;
        let projects = &mut self.drafts[index].projects;
        if projects.len() < 2 {
            return Err("selected setlist has fewer than two projects");
        }
        if toward_end {
            projects.rotate_right(1);
        } else {
            projects.rotate_left(1);
        }
        Ok(())
    }

    /// Reorders the selected setlist using the complete project-ID order.
    ///
    /// # Errors
    ///
    /// Returns an error when no setlist is selected or the order is invalid.
    pub fn reorder_selected(&mut self, order: &[&str]) -> Result<(), String> {
        let index = self.selected.ok_or_else(|| "no setlist selected".to_owned())?;
        self.drafts[index] = mackes_config::reorder_setlist(&self.drafts[index], order)?;
        Ok(())
    }

    /// Copies the selected setlist under a new ID without touching persisted state.
    ///
    /// # Errors
    ///
    /// Returns an error when no setlist is selected or the destination conflicts.
    pub fn copy_selected(&mut self, new_id: &str) -> Result<(), String> {
        let index = self.selected.ok_or_else(|| "no setlist selected".to_owned())?;
        let copy = mackes_config::copy_setlist(&self.drafts, &self.drafts[index].id, new_id)?;
        self.drafts.push(copy);
        self.selected = Some(self.drafts.len() - 1);
        Ok(())
    }

    /// Returns the current draft snapshot for an atomic persistence operation.
    #[must_use]
    pub fn commit(self) -> Vec<mackes_config::Setlist> {
        self.drafts
    }
}

impl RoutingEditor {
    /// Returns the transactional routing draft in execution priority order.
    #[must_use]
    pub fn frame_lines(&self, viewport: Viewport) -> Vec<String> {
        let mut lines = vec![
            "Routing & mappings — uncommitted draft (a add, j/k select, m mode, c channel, r curve, f filter, +/- priority, e enable, y cycle, d remove, s save)"
                .into(),
        ];
        lines.extend(self.drafts.iter().enumerate().map(|(index, draft)| {
            format!(
                "{} p{} {} -> {} {:?} ch={:?} {} curve={:?} cycle={}",
                if self.selected == Some(index) { ">" } else { " " },
                draft.priority,
                draft.source,
                draft.destination,
                draft.mode,
                draft.channel,
                if draft.enabled { "enabled" } else { "disabled" },
                draft.curve,
                if draft.allow_cycle { "allowed" } else { "denied" }
            )
        }));
        clamp_lines(lines, viewport, usize::from(viewport.width))
    }
}

impl DiagnosticsState {
    /// Maximum retained diagnostics.
    pub const CAPACITY: usize = 32;

    /// Creates an empty diagnostics projection.
    #[must_use]
    pub const fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Adds a diagnostic, retaining only the newest bounded entries.
    pub fn push(&mut self, diagnostic: HealthDiagnostic) {
        self.entries.insert(0, diagnostic);
        self.entries.truncate(Self::CAPACITY);
    }

    /// Returns display-ready lines in newest-first order.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        self.entries.iter().map(HealthDiagnostic::line).collect()
    }
}

impl Default for DiagnosticsState {
    fn default() -> Self {
        Self::new()
    }
}

/// One bounded monitor entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonitorEntry {
    /// Severity.
    pub severity: MonitorSeverity,
    /// Display-safe message.
    pub message: String,
}

/// Bounded diagnostics monitor state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonitorState {
    /// Newest entries first.
    pub entries: Vec<MonitorEntry>,
    /// Maximum retained entries.
    pub capacity: usize,
    /// Whether incoming entries are temporarily paused.
    pub paused: bool,
}

impl MonitorState {
    /// Creates a monitor with a positive retention bound.
    #[must_use]
    pub fn new(capacity: usize) -> Option<Self> {
        (capacity > 0).then(|| Self { entries: Vec::new(), capacity, paused: false })
    }
    /// Adds an entry and evicts the oldest entry when full.
    pub fn push(&mut self, entry: MonitorEntry) {
        if self.paused {
            return;
        }
        self.entries.insert(0, entry);
        self.entries.truncate(self.capacity);
    }
    /// Pauses or resumes collection without discarding retained entries.
    pub const fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }
    /// Returns a bounded export snapshot in newest-first order.
    #[must_use]
    pub fn export(&self) -> Vec<MonitorEntry> {
        self.entries.clone()
    }
    /// Returns an export with operator-marked sensitive entries redacted.
    #[must_use]
    pub fn export_redacted(&self, sensitive_indices: &[usize]) -> Vec<MonitorEntry> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                if sensitive_indices.contains(&index) {
                    MonitorEntry { severity: entry.severity, message: "<redacted>".into() }
                } else {
                    entry.clone()
                }
            })
            .collect()
    }
    /// Returns entries at or above the requested severity.
    #[must_use]
    pub fn filtered(&self, minimum: MonitorSeverity) -> Vec<&MonitorEntry> {
        self.entries.iter().filter(|entry| entry.severity >= minimum).collect()
    }

    /// Returns bounded renderer-ready monitor lines.
    #[must_use]
    pub fn frame_lines(&self, viewport: Viewport) -> Vec<String> {
        let width = usize::from(viewport.width);
        let mut lines = vec![format!(
            "monitor={} entries={}",
            if self.paused { "paused" } else { "live" },
            self.entries.len()
        )];
        lines.extend(
            self.entries
                .iter()
                .take(usize::from(viewport.height.saturating_sub(1)))
                .map(|entry| format!("{:?} {}", entry.severity, entry.message)),
        );
        clamp_lines(lines, viewport, width)
    }
}

impl BackupWorkspace {
    /// Returns a bounded, non-actionable backup status view.
    #[must_use]
    pub fn frame_lines(&self, viewport: Viewport) -> Vec<String> {
        let mut lines = vec![format!("Backups — {:?}", self.phase)];
        lines.push(format!("selected={}", self.selected.as_deref().unwrap_or("none")));
        lines.push(format!("identity_warning={}", self.identity_warning));
        if let Some(message) = &self.message {
            lines.push(format!("message={message}"));
        }
        clamp_lines(lines, viewport, usize::from(viewport.width))
    }

    /// Creates an empty listing view.
    #[must_use]
    pub const fn new() -> Self {
        Self { phase: BackupPhase::Listing, selected: None, message: None, identity_warning: false }
    }
    /// Selects an artifact for inspection.
    pub fn inspect(&mut self, artifact: impl Into<String>) {
        self.selected = Some(artifact.into());
        self.phase = BackupPhase::Inspecting;
    }
    /// Records a dry-run restore plan.
    pub fn plan(&mut self, message: impl Into<String>) {
        if self.selected.is_some() {
            self.message = Some(message.into());
            self.phase = BackupPhase::Planned;
        }
    }
    /// Records whether the selected backup has an identity mismatch warning.
    pub const fn set_identity_warning(&mut self, warning: bool) {
        self.identity_warning = warning;
    }
    /// Confirms the planned restore; applying remains a separate phase.
    pub fn confirm(&mut self) {
        if self.phase == BackupPhase::Planned {
            self.phase = BackupPhase::Confirmed;
        }
    }
    /// Marks the operation as applying.
    pub fn begin_apply(&mut self) {
        if self.phase == BackupPhase::Confirmed {
            self.phase = BackupPhase::Applying;
        }
    }
    /// Cancels a non-applying operation and returns to the listing view.
    pub fn cancel(&mut self) {
        if !matches!(
            self.phase,
            BackupPhase::Applying | BackupPhase::Verified | BackupPhase::SentUnverified
        ) {
            self.selected = None;
            self.message = None;
            self.identity_warning = false;
            self.phase = BackupPhase::Listing;
        }
    }
    /// Records a terminal result.
    pub fn finish(&mut self, phase: BackupPhase, message: impl Into<String>) {
        self.message = Some(message.into());
        self.phase = phase;
    }
}

impl Default for BackupWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl MappingDraft {
    /// Validates identifiers and channel bounds without mutating daemon state.
    ///
    /// # Errors
    ///
    /// Returns an error when an endpoint is empty or the channel is outside 1–16.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.source.trim().is_empty() || self.destination.trim().is_empty() {
            return Err("mapping endpoints must not be empty");
        }
        if self.channel.is_some_and(|channel| !(1..=16).contains(&channel)) {
            return Err("mapping channel must be between 1 and 16");
        }
        self.filters.validate()?;
        Ok(())
    }

    /// Returns whether two valid drafts compete for the same filtered input.
    #[must_use]
    pub fn conflicts_with(&self, other: &Self) -> bool {
        self.enabled
            && other.enabled
            && self.source == other.source
            && self.destination == other.destination
            && self.mode == other.mode
            && self.channel == other.channel
    }
}

/// Validates a mapping batch before it can be submitted as one transaction.
///
/// # Errors
///
/// Returns an error when a draft is invalid or conflicts with an earlier draft.
pub fn validate_mapping_batch(drafts: &[MappingDraft]) -> Result<(), String> {
    for draft in drafts {
        draft.validate().map_err(str::to_owned)?;
    }
    for (index, draft) in drafts.iter().enumerate() {
        if drafts[..index].iter().any(|previous| draft.conflicts_with(previous)) {
            return Err(format!("mapping {index} conflicts with an earlier mapping"));
        }
    }
    Ok(())
}

/// Reorders mapping drafts using a complete permutation of their indices.
///
/// # Errors
///
/// Returns an error when an index is missing, duplicated, or out of bounds.
pub fn reorder_mapping_drafts(
    drafts: &[MappingDraft],
    order: &[usize],
) -> Result<Vec<MappingDraft>, String> {
    if order.len() != drafts.len() {
        return Err("mapping order must contain every draft exactly once".into());
    }
    let mut result = Vec::with_capacity(order.len());
    let mut used = vec![false; drafts.len()];
    for &index in order {
        if index >= drafts.len() || used[index] {
            return Err("mapping order contains a duplicate or invalid index".into());
        }
        used[index] = true;
        result.push(drafts[index].clone());
    }
    Ok(result)
}

/// Returns mappings ordered by priority, preserving declaration order for ties.
#[must_use]
pub fn order_mapping_drafts(drafts: &[MappingDraft]) -> Vec<MappingDraft> {
    let mut ordered = drafts.to_vec();
    ordered.sort_by_key(|draft| draft.priority);
    ordered
}

/// Transactional mapping collection used by Learn and the routing editor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappingBank {
    generation: u64,
    drafts: Vec<MappingDraft>,
    history: Vec<Vec<MappingDraft>>,
}

impl MappingBank {
    /// Creates an empty mapping bank at generation zero.
    #[must_use]
    pub const fn new() -> Self {
        Self { generation: 0, drafts: Vec::new(), history: Vec::new() }
    }

    /// Returns the last committed generation and mappings.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }
    #[must_use]
    /// Returns the currently committed drafts.
    pub fn drafts(&self) -> &[MappingDraft] {
        &self.drafts
    }

    /// Returns whether a bounded prior committed mapping snapshot is available.
    #[must_use]
    pub fn undo_available(&self) -> bool {
        !self.history.is_empty()
    }

    /// Atomically validates and commits a complete replacement batch.
    ///
    /// # Errors
    ///
    /// Returns an error without changing the bank when validation fails.
    pub fn commit(&mut self, drafts: Vec<MappingDraft>) -> Result<u64, String> {
        validate_mapping_batch(&drafts)?;
        self.history.push(self.drafts.clone());
        if self.history.len() > 16 {
            self.history.remove(0);
        }
        self.generation = self.generation.saturating_add(1);
        self.drafts = drafts;
        Ok(self.generation)
    }

    /// Restores the most recent committed snapshot and advances the generation.
    ///
    /// The operation is deterministic and bounded; an empty history is a no-op error.
    ///
    /// # Errors
    ///
    /// Returns an error when no prior committed snapshot is available.
    pub fn undo(&mut self) -> Result<u64, String> {
        let previous =
            self.history.pop().ok_or_else(|| String::from("mapping undo history is empty"))?;
        self.history.push(self.drafts.clone());
        if self.history.len() > 16 {
            self.history.remove(0);
        }
        self.generation = self.generation.saturating_add(1);
        self.drafts = previous;
        Ok(self.generation)
    }

    /// Commits only when the editor's snapshot generation is still current.
    ///
    /// # Errors
    ///
    /// Returns an error without mutation when another writer has advanced the generation or
    /// when the replacement batch is invalid.
    pub fn commit_if_generation(
        &mut self,
        expected_generation: u64,
        drafts: Vec<MappingDraft>,
    ) -> Result<u64, String> {
        if self.generation != expected_generation {
            return Err("mapping generation changed concurrently".into());
        }
        self.commit(drafts)
    }
}

impl Default for MappingBank {
    fn default() -> Self {
        Self::new()
    }
}

/// Draft-oriented routing editor state. All daemon changes occur only on commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutingEditor {
    /// Uncommitted mappings shown by the editor.
    pub drafts: Vec<MappingDraft>,
    /// Focused row, if any.
    pub selected: Option<usize>,
}

impl RoutingEditor {
    /// Starts an editor from the bank's last committed snapshot.
    #[must_use]
    pub fn from_bank(bank: &MappingBank) -> Self {
        Self { drafts: bank.drafts().to_vec(), selected: None }
    }
    /// Adds a validated draft to the editor without touching daemon state.
    ///
    /// # Errors
    ///
    /// Returns an error when the new batch is invalid or conflicts.
    pub fn add(&mut self, draft: MappingDraft) -> Result<(), String> {
        let mut candidate = self.drafts.clone();
        candidate.push(draft);
        validate_mapping_batch(&candidate)?;
        self.drafts = candidate;
        self.selected = Some(self.drafts.len() - 1);
        Ok(())
    }
    /// Removes one row, returning an error for an invalid index.
    ///
    /// # Errors
    ///
    /// Returns an error when the index is outside the draft list.
    pub fn remove(&mut self, index: usize) -> Result<MappingDraft, &'static str> {
        if index >= self.drafts.len() {
            return Err("mapping index is out of range");
        }
        let removed = self.drafts.remove(index);
        self.selected =
            if self.drafts.is_empty() { None } else { Some(index.min(self.drafts.len() - 1)) };
        Ok(removed)
    }
    /// Cycles the selected mapping through the supported MIDI message classes.
    ///
    /// The edit is transactional and is rejected if changing class would create a conflict.
    ///
    /// # Errors
    ///
    /// Returns an error when no valid row is selected or the edited batch conflicts.
    pub fn cycle_selected_mode(&mut self) -> Result<(), String> {
        let index = self.selected.ok_or("no mapping is selected")?;
        let mut candidate = self.drafts.clone();
        if index >= candidate.len() {
            return Err("selected mapping is out of range".into());
        }
        let mode = match candidate[index].mode {
            MappingMode::Cc => MappingMode::ProgramChange,
            MappingMode::ProgramChange => MappingMode::Note,
            MappingMode::Note => MappingMode::PitchBend,
            MappingMode::PitchBend => MappingMode::Sysex,
            MappingMode::Sysex => MappingMode::Cc,
        };
        candidate[index].mode = mode;
        validate_mapping_batch(&candidate)?;
        self.drafts = candidate;
        Ok(())
    }

    /// Cycles the selected mapping channel through any-channel and MIDI channels 1–16.
    ///
    /// # Errors
    ///
    /// Returns an error when no valid row is selected or the edited batch conflicts.
    pub fn cycle_selected_channel(&mut self) -> Result<(), String> {
        let index = self.selected.ok_or("no mapping is selected")?;
        let mut candidate = self.drafts.clone();
        if index >= candidate.len() {
            return Err("selected mapping is out of range".into());
        }
        candidate[index].channel = match candidate[index].channel {
            None => Some(1),
            Some(channel) if channel < 16 => Some(channel + 1),
            Some(_) => None,
        };
        validate_mapping_batch(&candidate)?;
        self.drafts = candidate;
        Ok(())
    }

    /// Toggles the selected mapping without mutating the persisted bank.
    ///
    /// # Errors
    ///
    /// Returns an error when no valid row is selected.
    pub fn toggle_selected_enabled(&mut self) -> Result<(), String> {
        let index = self.selected.ok_or("no mapping is selected")?;
        let Some(draft) = self.drafts.get_mut(index) else {
            return Err("selected mapping is out of range".into());
        };
        draft.enabled = !draft.enabled;
        Ok(())
    }

    /// Toggles explicit bounded-cycle authorization for the selected route.
    ///
    /// # Errors
    ///
    /// Returns an error when no valid row is selected.
    pub fn toggle_selected_cycle(&mut self) -> Result<(), String> {
        let index = self.selected.ok_or("no mapping is selected")?;
        let Some(draft) = self.drafts.get_mut(index) else {
            return Err("selected mapping is out of range".into());
        };
        draft.allow_cycle = !draft.allow_cycle;
        Ok(())
    }

    /// Adjusts the selected mapping priority with saturating bounds.
    ///
    /// # Errors
    ///
    /// Returns an error when no valid row is selected.
    pub fn adjust_selected_priority(&mut self, delta: i32) -> Result<(), String> {
        let index = self.selected.ok_or("no mapping is selected")?;
        let Some(draft) = self.drafts.get_mut(index) else {
            return Err("selected mapping is out of range".into());
        };
        draft.priority = if delta.is_negative() {
            draft.priority.saturating_sub(
                u16::try_from(delta.unsigned_abs().min(u32::from(u16::MAX))).unwrap_or(u16::MAX),
            )
        } else {
            draft.priority.saturating_add(u16::try_from(delta).unwrap_or(u16::MAX))
        };
        Ok(())
    }

    /// Cycles the selected route through the bounded engine curves.
    ///
    /// # Errors
    ///
    /// Returns an error when no valid row is selected.
    pub fn cycle_selected_curve(&mut self) -> Result<(), String> {
        let index = self.selected.ok_or("no mapping is selected")?;
        let Some(draft) = self.drafts.get_mut(index) else {
            return Err("selected mapping is out of range".into());
        };
        draft.curve = match draft.curve {
            mackes_midi_engine::Curve::Linear => mackes_midi_engine::Curve::Square,
            mackes_midi_engine::Curve::Square => mackes_midi_engine::Curve::SquareRoot,
            mackes_midi_engine::Curve::SquareRoot => mackes_midi_engine::Curve::Linear,
        };
        Ok(())
    }

    /// Cycles common bounded filter presets for the selected route.
    ///
    /// The cycle is no filter, MIDI-number lower half, value upper half, realtime clock,
    /// masked `SysEx` marker, then no filter.
    ///
    /// # Errors
    ///
    /// Returns an error when no valid row is selected or the edited filter is invalid.
    pub fn cycle_selected_filter(&mut self) -> Result<(), String> {
        let index = self.selected.ok_or("no mapping is selected")?;
        let Some(draft) = self.drafts.get_mut(index) else {
            return Err("selected mapping is out of range".into());
        };
        draft.filters.predicates = match draft.filters.predicates.as_slice() {
            [] => vec![mackes_midi_engine::RoutePredicate::NumberRange { minimum: 0, maximum: 63 }],
            [mackes_midi_engine::RoutePredicate::NumberRange { .. }] => {
                vec![mackes_midi_engine::RoutePredicate::ValueRange { minimum: 64, maximum: 127 }]
            }
            [mackes_midi_engine::RoutePredicate::ValueRange { .. }] => {
                vec![mackes_midi_engine::RoutePredicate::Realtime(
                    mackes_domain::RealtimeMessage::Clock,
                )]
            }
            [mackes_midi_engine::RoutePredicate::Realtime(_)] => {
                vec![mackes_midi_engine::RoutePredicate::SysExMask {
                    pattern: vec![0x7d],
                    mask: vec![0x7f],
                }]
            }
            _ => Vec::new(),
        };
        draft.filters.validate().map_err(str::to_owned)
    }
    /// Reorders rows using a complete permutation after validation.
    ///
    /// # Errors
    ///
    /// Returns an error when the order is not a complete valid permutation.
    pub fn reorder(&mut self, order: &[usize]) -> Result<(), String> {
        let reordered = reorder_mapping_drafts(&self.drafts, order)?;
        self.drafts = reordered;
        Ok(())
    }
    /// Validates the current draft set and atomically commits it.
    ///
    /// # Errors
    ///
    /// Returns an error when validation fails; the bank is unchanged.
    pub fn commit(self, bank: &mut MappingBank) -> Result<u64, String> {
        bank.commit(self.drafts)
    }
}

/// Keyboard actions recognized by the Learn workspace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LearnKey {
    /// Finish or commit the current explicit selection.
    Enter,
    /// Cancel and discard unsaved capture state.
    Escape,
    /// Any other key, which cannot commit.
    Other,
}

impl LearnWorkspace {
    /// Creates an armed workspace with no implicit candidate.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            phase: LearnPhase::Armed,
            candidates: Vec::new(),
            selected: None,
            learn_input_alias: None,
            learn_endpoint_id: None,
            channel_policy: None,
            destination: None,
            live_test_passed: false,
            filters: MappingFilterDraft { predicates: Vec::new() },
        }
    }
    /// Selects the global input endpoint used for all Learn captures.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty alias or while capture is active.
    pub fn set_input_alias(&mut self, alias: impl Into<String>) -> Result<(), &'static str> {
        let alias = alias.into();
        if alias.is_empty() {
            return Err("Learn input alias must not be empty");
        }
        if self.phase == LearnPhase::Armed {
            self.learn_input_alias = Some(alias);
            Ok(())
        } else {
            Err("Learn input cannot change during capture")
        }
    }
    /// Sets the daemon-resolved endpoint for the configured alias.
    pub fn set_endpoint_id(&mut self, endpoint_id: impl Into<String>) {
        self.learn_endpoint_id = Some(endpoint_id.into());
    }
    /// Begins bounded capture.
    pub fn start_capture(&mut self) {
        if self.phase == LearnPhase::Armed && self.learn_input_alias.is_some() {
            self.phase = LearnPhase::Capturing;
        }
    }
    /// Ends capture and enters review without selecting a candidate.
    pub fn finish_capture(&mut self, candidates: Vec<MidiLearnCandidate>) {
        if self.phase == LearnPhase::Capturing {
            self.candidates = candidates;
            self.selected = None;
            self.channel_policy = None;
            self.destination = None;
            self.live_test_passed = false;
            self.phase = LearnPhase::Review;
        }
    }
    /// Selects a candidate explicitly for destination/test/commit.
    pub fn select(&mut self, index: usize) {
        if self.phase == LearnPhase::Review && index < self.candidates.len() {
            self.selected = Some(index);
            self.channel_policy = self.candidates[index]
                .channel
                .map_or(Some(LearnChannelPolicy::NotApplicable), |channel| {
                    Some(LearnChannelPolicy::Exact(channel))
                });
            self.destination = None;
            self.live_test_passed = false;
            self.phase = LearnPhase::Destination;
        }
    }
    /// Sets an explicit channel policy.
    ///
    /// # Errors
    ///
    /// Rejects channel policies incompatible with the selected candidate.
    pub fn set_channel_policy(&mut self, policy: LearnChannelPolicy) -> Result<(), &'static str> {
        let candidate = self
            .selected
            .and_then(|index| self.candidates.get(index))
            .ok_or("no Learn candidate selected")?;
        match (candidate.channel, policy) {
            (Some(channel), LearnChannelPolicy::Exact(actual)) if channel == actual => {}
            (Some(_), LearnChannelPolicy::Any) | (None, LearnChannelPolicy::NotApplicable) => {}
            _ => return Err("Learn channel policy is incompatible with candidate"),
        }
        self.channel_policy = Some(policy);
        self.live_test_passed = false;
        Ok(())
    }
    /// Selects a compatible destination for live testing.
    ///
    /// # Errors
    ///
    /// Rejects empty destinations and incompatible message classes.
    pub fn set_destination(
        &mut self,
        destination: impl Into<String>,
        mode: MappingMode,
    ) -> Result<(), &'static str> {
        let destination = destination.into();
        let candidate = self
            .selected
            .and_then(|index| self.candidates.get(index))
            .ok_or("no Learn candidate selected")?;
        if destination.trim().is_empty() || !candidate_supports_mode(candidate, mode) {
            return Err("Learn destination is empty or incompatible");
        }
        self.destination = Some((destination, mode));
        self.live_test_passed = false;
        Ok(())
    }
    /// Starts the mandatory live test after candidate, channel, and destination selection.
    pub fn begin_live_test(&mut self) {
        if self.phase == LearnPhase::Destination
            && self.selected.is_some()
            && self.channel_policy.is_some()
            && self.destination.is_some()
        {
            self.phase = LearnPhase::Testing;
        }
    }
    /// Records the live test outcome and returns to destination review.
    pub fn finish_live_test(&mut self, passed: bool) {
        if self.phase == LearnPhase::Testing {
            self.live_test_passed = passed;
            self.phase = LearnPhase::Destination;
        }
    }
    /// Commits only an explicitly selected candidate.
    pub fn commit(&mut self) {
        if self.phase == LearnPhase::Destination
            && self.selected.is_some()
            && self.destination.is_some()
            && self.channel_policy.is_some()
            && self.live_test_passed
        {
            self.phase = LearnPhase::Committed;
        }
    }
    /// Returns the committed one-source/one-destination mapping draft.
    #[must_use]
    pub fn committed_mapping(&self) -> Option<MappingDraft> {
        if self.phase != LearnPhase::Committed {
            return None;
        }
        let source = self.learn_input_alias.clone()?;
        let (destination, mode) = self.destination.clone()?;
        let channel = match self.channel_policy? {
            LearnChannelPolicy::Exact(channel) => Some(channel),
            LearnChannelPolicy::Any | LearnChannelPolicy::NotApplicable => None,
        };
        Some(MappingDraft {
            source,
            destination,
            destination_parameter: None,
            channel,
            enabled: true,
            mode,
            priority: 0,
            curve: mackes_midi_engine::Curve::Linear,
            filters: MappingFilterDraft::default(),
            allow_cycle: false,
        })
    }
    /// Returns the durable mapping, including the captured source signature
    /// and raw evidence, only after the mandatory live test and commit.
    #[must_use]
    pub fn committed_learned_mapping(&self) -> Option<mackes_config::LearnedMapping> {
        if self.phase != LearnPhase::Committed {
            return None;
        }
        let candidate = self.selected.and_then(|index| self.candidates.get(index))?;
        let (destination, mode) = self.destination.clone()?;
        let filters = learned_filters_from_predicates(&self.filters.predicates)?;
        let channel_policy = match self.channel_policy? {
            LearnChannelPolicy::Exact(channel) => {
                mackes_config::LearnedChannelPolicy::Exact(channel)
            }
            LearnChannelPolicy::Any => mackes_config::LearnedChannelPolicy::Any,
            LearnChannelPolicy::NotApplicable => mackes_config::LearnedChannelPolicy::NotApplicable,
        };
        Some(mackes_config::LearnedMapping {
            source_alias: self.learn_input_alias.clone()?,
            message_kind: learn_kind_name(candidate.kind).to_owned(),
            channel_policy,
            number: candidate.number,
            raw: candidate.raw.clone(),
            destination,
            mode: mapping_mode_name(mode).to_owned(),
            enabled: true,
            priority: 0,
            filters,
        })
    }
    /// Applies the committed mapping and global input to a configuration copy.
    /// The caller owns the final atomic filesystem save.
    ///
    /// # Errors
    ///
    /// Returns an error unless Learn is committed or either persisted contract
    /// fails semantic validation. The source document is never mutated.
    pub fn apply_to_config(
        &self,
        document: &mackes_config::ConfigDocument,
    ) -> Result<mackes_config::ConfigDocument, String> {
        let alias = self.learn_input_alias.as_deref().ok_or("Learn input alias is not selected")?;
        let mapping = self.committed_learned_mapping().ok_or("Learn mapping is not committed")?;
        let selected = mackes_config::set_learn_input_alias(document, alias)?;
        mackes_config::add_learned_mapping(&selected, mapping)
    }
    /// Cancels without retaining a selected mapping.
    pub fn cancel(&mut self) {
        self.candidates.clear();
        self.selected = None;
        self.channel_policy = None;
        self.destination = None;
        self.live_test_passed = false;
        self.phase = LearnPhase::Cancelled;
    }

    /// Rolls back the current Learn transaction and returns to the armed state.
    /// The explicitly selected input alias is retained; no mapping is committed.
    pub fn rollback(&mut self) {
        self.candidates.clear();
        self.selected = None;
        self.channel_policy = None;
        self.destination = None;
        self.live_test_passed = false;
        self.phase = LearnPhase::Armed;
    }

    /// Handles a key without allowing incidental navigation to commit a mapping.
    pub fn handle_key(&mut self, key: LearnKey) {
        match key {
            LearnKey::Escape => self.cancel(),
            LearnKey::Enter => self.commit(),
            LearnKey::Other => {}
        }
    }
}

const fn learn_kind_name(kind: mackes_midi_engine::LearnMessageKind) -> &'static str {
    use mackes_midi_engine::LearnMessageKind;
    match kind {
        LearnMessageKind::NoteOn => "note_on",
        LearnMessageKind::NoteOff => "note_off",
        LearnMessageKind::PolyPressure => "poly_pressure",
        LearnMessageKind::ControlChange => "control_change",
        LearnMessageKind::ProgramChange => "program_change",
        LearnMessageKind::ChannelPressure => "channel_pressure",
        LearnMessageKind::PitchBend => "pitch_bend",
        LearnMessageKind::SystemCommon => "system_common",
        LearnMessageKind::Realtime => "realtime",
        LearnMessageKind::SysEx => "sysex",
    }
}

const fn mapping_mode_name(mode: MappingMode) -> &'static str {
    match mode {
        MappingMode::Cc => "cc",
        MappingMode::ProgramChange => "program_change",
        MappingMode::Note => "note",
        MappingMode::PitchBend => "pitch_bend",
        MappingMode::Sysex => "sysex",
    }
}

const fn candidate_supports_mode(candidate: &MidiLearnCandidate, mode: MappingMode) -> bool {
    use mackes_midi_engine::LearnMessageKind;
    matches!(
        (candidate.kind, mode),
        (LearnMessageKind::ControlChange, MappingMode::Cc)
            | (LearnMessageKind::ProgramChange, MappingMode::ProgramChange)
            | (
                LearnMessageKind::NoteOn
                    | LearnMessageKind::NoteOff
                    | LearnMessageKind::PolyPressure,
                MappingMode::Note
            )
            | (LearnMessageKind::PitchBend, MappingMode::PitchBend)
            | (LearnMessageKind::SysEx, MappingMode::Sysex)
    )
}

impl Default for LearnWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed event projection consumed by dashboard widgets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DashboardEvent {
    /// Daemon health label changed.
    Health(String),
    /// Active scene changed.
    ActiveScene(Option<String>),
    /// Route generation changed.
    RouteGeneration(u64),
    /// Route Undo availability changed.
    RouteUndoAvailable(bool),
    /// Performance lock changed.
    PerformanceLock(bool),
    /// Activity counters changed.
    Activity {
        /// Number of received events.
        received: u64,
        /// Number of sent events.
        sent: u64,
        /// Number of dropped events.
        dropped: u64,
    },
    /// Activation progress changed.
    ActivationProgress {
        /// Completed action count.
        completed: u32,
        /// Total action count.
        total: u32,
    },
    /// Final activation result summary changed.
    ActivationResult(String),
    /// Replaces the bounded per-device health projection.
    DeviceHealth(Vec<(String, String)>),
    /// Adds one operator notification to the bounded dashboard queue.
    Notification {
        /// Semantic severity marker.
        severity: SemanticToken,
        /// Safe display message.
        message: String,
    },
    /// Latest bounded source-to-destination activity sample.
    LiveActivity(LiveActivity),
    /// Replaces the physical-device inventory projection.
    PhysicalDevices(Vec<PhysicalDevice>),
    /// Replaces the authoritative active mapping projection.
    ControlMappings(Vec<mackes_config::ControlMapping>),
    /// Updates the authoritative mapping-store generation and Undo availability.
    MappingStore {
        /// Mapping-store generation.
        generation: u64,
        /// Whether one bounded Undo record is available.
        undo_available: bool,
    },
    /// Updates the latest mapping source/result activity.
    MappingActivity(serde_json::Value),
    /// Updates the retained mutation-audit count.
    AuditCount(u64),
    /// Updates the newest safe mutation-audit summary.
    LatestAudit(String),
    /// Updates the daemon-owned LED contract status line.
    LedStatus(String),
}

impl DashboardEvent {
    /// Decodes the bounded fields understood by dashboard widgets from one daemon payload.
    #[must_use]
    pub fn from_payload(payload: &serde_json::Value) -> Vec<Self> {
        let mut events = Vec::with_capacity(8);
        if let Some(value) = payload.get("health").and_then(serde_json::Value::as_str) {
            events.push(Self::Health(value.to_owned()));
        }
        if let Some(value) = payload.get("active_scene").and_then(|value| {
            if value.is_null() {
                Some(None)
            } else {
                value.as_str().map(|value| Some(value.to_owned()))
            }
        }) {
            events.push(Self::ActiveScene(value));
        }
        if let Some(value) = payload.get("route_generation").and_then(serde_json::Value::as_u64) {
            events.push(Self::RouteGeneration(value));
        }
        if let Some(generation) = payload.get("generation").and_then(serde_json::Value::as_u64) {
            events.push(Self::MappingStore {
                generation,
                undo_available: payload
                    .get("mapping_undo_available")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            });
        }
        if let Some(value) =
            payload.get("route_undo_available").and_then(serde_json::Value::as_bool)
        {
            events.push(Self::RouteUndoAvailable(value));
        }
        if let Some(value) = payload.get("audit_count").and_then(serde_json::Value::as_u64) {
            events.push(Self::AuditCount(value));
        }
        if let Some(latest) = payload
            .get("audit")
            .and_then(serde_json::Value::as_array)
            .and_then(|audit| audit.first())
        {
            if let (Some(action), Some(allowed)) = (
                latest.get("action").and_then(serde_json::Value::as_str),
                latest.get("allowed").and_then(serde_json::Value::as_bool),
            ) {
                events.push(Self::LatestAudit(format!(
                    "{} {}",
                    if allowed { "ALLOW" } else { "DENY" },
                    action
                )));
            }
        }
        if let (Some(received), Some(sent), Some(dropped)) = (
            payload.get("received").and_then(serde_json::Value::as_u64),
            payload.get("sent").and_then(serde_json::Value::as_u64),
            payload.get("dropped").and_then(serde_json::Value::as_u64),
        ) {
            events.push(Self::Activity { received, sent, dropped });
        }
        if let Some(value) = payload.get("activation_result").and_then(serde_json::Value::as_str) {
            events.push(Self::ActivationResult(value.to_owned()));
        }
        if let Some(activity) = payload.get("last_activity").and_then(parse_live_activity) {
            events.push(Self::LiveActivity(activity));
        }
        if let Some(devices) = payload.get("physical_devices").and_then(parse_physical_devices) {
            events.push(Self::PhysicalDevices(devices));
        }
        if let Some(mappings) = payload.get("control_mappings").and_then(|value| {
            serde_json::from_value::<Vec<mackes_config::ControlMapping>>(value.clone()).ok()
        }) {
            events.push(Self::ControlMappings(mappings));
        }
        if let Some(activity) = payload.get("last_mapping_activity") {
            events.push(Self::MappingActivity(activity.clone()));
        }
        if let Some(line) = led_status::line_from_payload(payload) {
            events.push(Self::LedStatus(line));
        }
        events
    }
}

fn parse_physical_devices(value: &serde_json::Value) -> Option<Vec<PhysicalDevice>> {
    value.as_array()?.iter().map(parse_physical_device).collect()
}

fn parse_physical_device(value: &serde_json::Value) -> Option<PhysicalDevice> {
    let object = value.as_object()?;
    let strings = |key: &str| {
        object
            .get(key)?
            .as_array()?
            .iter()
            .map(serde_json::Value::as_str)
            .map(|value| value.map(str::to_owned))
            .collect::<Option<Vec<_>>>()
    };
    Some(PhysicalDevice {
        id: object.get("id")?.as_str()?.to_owned(),
        name: object.get("name")?.as_str()?.to_owned(),
        inputs: strings("inputs")?,
        outputs: strings("outputs")?,
        state: object.get("state")?.as_str()?.to_owned(),
    })
}

fn parse_live_activity(value: &serde_json::Value) -> Option<LiveActivity> {
    let object = value.as_object()?;
    let source_endpoint = object.get("source_endpoint")?.as_u64()?;
    let source_endpoint_id =
        object.get("source_endpoint_id").and_then(|value| value.as_str().map(str::to_owned));
    let kind = object.get("kind")?.as_str()?.to_owned();
    let channel = object
        .get("channel")
        .filter(|value| !value.is_null())
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u8::try_from(value).ok());
    let control_id = object.get("control_id")?.as_str()?.to_owned();
    let timestamp_nanos = object.get("timestamp_nanos")?.as_u64()?;
    let number = object
        .get("number")
        .filter(|value| !value.is_null())
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u8::try_from(value).ok());
    let value = object
        .get("value")
        .filter(|value| !value.is_null())
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u16::try_from(value).ok());
    let destination_endpoints = object
        .get("destination_endpoints")?
        .as_array()?
        .iter()
        .map(serde_json::Value::as_u64)
        .collect::<Option<Vec<_>>>()?;
    Some(LiveActivity {
        source_endpoint,
        source_endpoint_id,
        control_id,
        timestamp_nanos,
        kind,
        channel,
        number,
        value,
        destination_endpoints,
        sequence: object.get("sequence")?.as_u64()?,
    })
}

/// Bounded controller-page navigation state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageState {
    /// Current zero-based page.
    pub current: u16,
    /// Number of available pages.
    pub total: u16,
}

/// One logical node shown in the signal-flow diagram.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignalFlowNode {
    /// Stable endpoint or processing identifier.
    pub id: String,
    /// Display label (resolved from the device profile).
    pub label: String,
    /// Whether this node is currently online.
    pub online: bool,
}

/// Ordered, renderer-neutral signal-flow diagram model.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SignalFlowDiagram {
    /// Nodes in signal order.
    pub nodes: Vec<SignalFlowNode>,
    /// Routing generation used to build the view.
    pub generation: u64,
}

/// Renders a signal-flow diagram using the scoped Blueprint visual contract.
///
/// The output is text-first so terminals without color retain the grid, connectors, and labels.
#[must_use]
pub fn blueprint_lines(diagram: &SignalFlowDiagram, non_authoritative: bool) -> Vec<String> {
    let mut lines = Vec::new();
    if non_authoritative {
        lines.push("Inferred logical/control view — not authoritative DSP topology".into());
    } else {
        lines.push("Logical/control view".into());
    }
    for (index, node) in diagram.nodes.iter().enumerate() {
        lines.push(format!("+--[{}]--+", node.label));
        lines.push(format!("| {} {} |", node.id, if node.online { "online" } else { "offline" }));
        if index + 1 < diagram.nodes.len() {
            lines.push("      |      v".into());
        }
    }
    lines
}

/// A documented Reflex control shown on an algorithm page.
/// Labels are supplied by the compiled profile; this type deliberately cannot rename them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReflexControl {
    /// Stable compiled parameter identifier.
    pub id: String,
    /// Exact label from the device documentation.
    pub label: String,
    /// Signal-flow node owning this control.
    pub node_id: String,
    /// Whether the control is currently available.
    pub available: bool,
}

/// Compiled parameter facts needed by a Reflex control renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReflexParameterView {
    /// Zero-based wire parameter number.
    pub number: u8,
    /// Inclusive legal wire value range.
    pub range: (u16, u16),
    /// Whether the parameter is bipolar.
    pub bipolar: bool,
    /// Effective documented steps.
    pub effective_steps: u16,
}

/// Metadata-driven Reflex algorithm workspace model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReflexWorkspace {
    /// Algorithm identifier from the compiled profile.
    pub algorithm_id: String,
    /// Exact algorithm label from the compiled profile.
    pub algorithm_label: String,
    /// Shared setup controls reachable from every algorithm page.
    pub shared_controls: Vec<ReflexControl>,
    /// Algorithm-specific controls in documented order.
    pub controls: Vec<ReflexControl>,
    /// Blueprint signal-flow diagram.
    pub diagram: SignalFlowDiagram,
    /// Currently highlighted diagram node, if any.
    pub selected_node: Option<String>,
}

/// A documented control group used by profile-backed device pages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceControlGroup {
    /// Stable profile-provided group identifier.
    pub id: String,
    /// Exact documentation label.
    pub label: String,
    /// Signal-flow block owning this group.
    pub block_id: String,
    /// Controls remain in profile order.
    pub control_ids: Vec<String>,
}

/// Explicit presentation state for a profile-owned device control.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlAvailability {
    /// Control is available for ordinary interaction.
    Available,
    /// Control is known but cannot be edited on this transport/firmware.
    ReadOnly,
    /// Control is intentionally hidden because its capability is unverified.
    Unavailable,
    /// Control is available only through the governed hazardous-action path.
    Hazardous,
}

/// Returns whether a control may be rendered as an actionable input.
#[must_use]
pub const fn control_is_actionable(state: ControlAvailability) -> bool {
    matches!(state, ControlAvailability::Available | ControlAvailability::Hazardous)
}

/// Shared renderer-neutral workspace model for non-Reflex devices.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceWorkspace {
    /// Stable device profile identifier.
    pub device_id: String,
    /// Display name from the profile.
    pub device_label: String,
    /// Ordered signal-flow diagram.
    pub diagram: SignalFlowDiagram,
    /// Shared controls available on every page.
    pub shared_controls: Vec<String>,
    /// Documented control groups.
    pub groups: Vec<DeviceControlGroup>,
    /// Whether the diagram is an inferred logical/control view.
    pub non_authoritative_diagram: bool,
    /// Currently selected diagram block.
    pub selected_block: Option<String>,
    /// Profile-owned controls available for the bounded editor.
    pub control_labels: Vec<String>,
    /// Selected control index in `control_labels`.
    pub selected_control: Option<usize>,
    /// Current 7-bit value used by the control editor.
    pub control_value: u8,
}

impl DeviceWorkspace {
    /// Builds the Eventide `MicroPitch` page from its compiled documented
    /// control labels while applying the fixed signal-flow grouping.
    ///
    /// # Panics
    ///
    /// Panics only if the static signal-flow diagram is invalid.
    #[must_use]
    pub fn eventide_micropitch() -> Self {
        let profile = mackes_profiles::eventide_micropitch_profile();
        let labels = |wanted: &[&str]| {
            profile
                .controls
                .iter()
                .filter(|control| wanted.contains(&control.label.as_str()))
                .map(|control| control.label.clone())
                .collect::<Vec<_>>()
        };
        let diagram = SignalFlowDiagram::new(
            vec![
                SignalFlowNode { id: "input".into(), label: "Input".into(), online: true },
                SignalFlowNode { id: "pitch".into(), label: "Pitch Shift".into(), online: true },
                SignalFlowNode { id: "delay".into(), label: "Delay".into(), online: true },
                SignalFlowNode { id: "mod".into(), label: "Modulation".into(), online: true },
                SignalFlowNode { id: "output".into(), label: "Output".into(), online: true },
            ],
            8,
        )
        .expect("static Eventide signal-flow diagram is valid");
        Self::new(
            profile.id,
            profile.name,
            diagram,
            labels(&["Expression Pedal", "TAP TEMPO", "ACTIVE/BYPASS", "FLEX", "Preset 1"]),
            vec![
                DeviceControlGroup {
                    id: "input".into(),
                    label: "Input/Mix".into(),
                    block_id: "input".into(),
                    control_ids: labels(&["Mix", "Tone"]),
                },
                DeviceControlGroup {
                    id: "pitch".into(),
                    label: "Pitch".into(),
                    block_id: "pitch".into(),
                    control_ids: labels(&["Pitch A", "Pitch B", "Pitch Mix"]),
                },
                DeviceControlGroup {
                    id: "delay".into(),
                    label: "Delay".into(),
                    block_id: "delay".into(),
                    control_ids: labels(&["Delay A", "Delay B", "Feedback"]),
                },
                DeviceControlGroup {
                    id: "mod".into(),
                    label: "Modulation".into(),
                    block_id: "mod".into(),
                    control_ids: labels(&["Depth", "Rate/Sens", "Mod"]),
                },
                DeviceControlGroup {
                    id: "output".into(),
                    label: "Output".into(),
                    block_id: "output".into(),
                    control_ids: labels(&["Out Lvl"]),
                },
            ],
            false,
        )
        .expect("static Eventide workspace is valid")
    }

    /// Creates a device page while rejecting empty profile-owned identifiers.
    ///
    /// # Errors
    ///
    /// Returns an error when any profile-owned identity or group field is empty.
    pub fn new(
        device_id: String,
        device_label: String,
        diagram: SignalFlowDiagram,
        shared_controls: Vec<String>,
        groups: Vec<DeviceControlGroup>,
        non_authoritative_diagram: bool,
    ) -> Result<Self, &'static str> {
        if device_id.is_empty() || device_label.is_empty() {
            return Err("device workspace identity must not be empty");
        }
        if shared_controls.iter().any(String::is_empty)
            || groups.iter().any(|group| {
                group.id.is_empty()
                    || group.label.is_empty()
                    || group.block_id.is_empty()
                    || group.control_ids.iter().any(String::is_empty)
            })
        {
            return Err("device workspace fields must not be empty");
        }
        let mut control_labels = shared_controls.clone();
        for group in &groups {
            for control in &group.control_ids {
                if !control_labels.iter().any(|existing| existing == control) {
                    control_labels.push(control.clone());
                }
            }
        }
        Ok(Self {
            device_id,
            device_label,
            diagram,
            shared_controls,
            groups,
            non_authoritative_diagram,
            selected_block: None,
            selected_control: (!control_labels.is_empty()).then_some(0),
            control_labels,
            control_value: 64,
        })
    }

    /// Selects a diagram block, returning groups whose controls are linked to it.
    ///
    /// # Errors
    ///
    /// Returns an error when the block is not present in the diagram.
    pub fn select_block(
        &mut self,
        block_id: &str,
    ) -> Result<Vec<&DeviceControlGroup>, &'static str> {
        if !self.diagram.nodes.iter().any(|node| node.id == block_id) {
            return Err("unknown device signal-flow block");
        }
        self.selected_block = Some(block_id.to_owned());
        Ok(self.groups.iter().filter(|group| group.block_id == block_id).collect())
    }

    /// Moves the selected profile control by a bounded signed step.
    pub fn move_control(&mut self, step: i8) {
        if self.control_labels.is_empty() {
            self.selected_control = None;
            return;
        }
        let current = self.selected_control.unwrap_or(0);
        let length = self.control_labels.len();
        let next = if step.is_negative() {
            current.saturating_sub(usize::from(step.unsigned_abs()))
        } else {
            current.saturating_add(usize::try_from(step).unwrap_or(0)).min(length - 1)
        };
        self.selected_control = Some(next);
    }

    /// Adjusts the selected control value within the MIDI 7-bit range.
    pub fn adjust_control_value(&mut self, delta: i16) {
        let value = i16::from(self.control_value).saturating_add(delta).clamp(0, 127);
        self.control_value = u8::try_from(value).unwrap_or(0);
    }

    /// Returns the selected profile control and current value.
    #[must_use]
    pub fn selected_control_request(&self) -> Option<(&str, u8)> {
        Some((self.control_labels.get(self.selected_control?)?.as_str(), self.control_value))
    }

    /// Returns the permanent diagram notice required for inferred topologies.
    #[must_use]
    pub fn diagram_notice(&self) -> Option<&'static str> {
        self.non_authoritative_diagram
            .then_some("Inferred logical/control view — not authoritative DSP topology")
    }

    /// Returns a deterministic device page retaining shared controls, signal
    /// flow, selected block, and profile-owned control ordering.
    #[must_use]
    pub fn frame_lines(&self, viewport: Viewport) -> Vec<String> {
        let mut lines = vec![format!(
            "{} [{}] (q query, j/k control, +/- value, W send)",
            self.device_label, self.device_id
        )];
        if let Some(notice) = self.diagram_notice() {
            lines.push(notice.to_owned());
        } else {
            lines.push("Logical/control view".into());
        }
        lines.push(format!("shared: {}", self.shared_controls.join(" | ")));
        if let Some((control, value)) = self.selected_control_request() {
            lines
                .push(format!("control: {control} value={value} (j/k select, +/- adjust, W send)"));
        }
        for node in &self.diagram.nodes {
            let marker =
                if self.selected_block.as_deref() == Some(node.id.as_str()) { ">" } else { " " };
            lines.push(format!("{marker} [{}] {}", node.id, node.label));
            lines.extend(
                self.groups
                    .iter()
                    .filter(|group| group.block_id == node.id)
                    .map(|group| format!("    {}: {}", group.label, group.control_ids.join(", "))),
            );
        }
        clamp_lines(lines, viewport, usize::from(viewport.width))
    }
}

impl ReflexWorkspace {
    /// Returns compiled value metadata for one algorithm in documented order.
    ///
    /// # Errors
    ///
    /// Returns an error when the algorithm number is unknown.
    pub fn compiled_parameter_views(number: u8) -> Result<Vec<ReflexParameterView>, &'static str> {
        if !mackes_profiles::lexicon_reflex::algorithms()
            .iter()
            .any(|algorithm| algorithm.number == number)
        {
            return Err("unknown Reflex algorithm");
        }
        Ok(mackes_profiles::lexicon_reflex::parameters(number)
            .iter()
            .map(|parameter| ReflexParameterView {
                number: parameter.number,
                range: (parameter.min, parameter.max),
                bipolar: parameter.bipolar,
                effective_steps: parameter.effective_steps,
            })
            .collect())
    }

    /// Builds a Reflex page directly from the compiled algorithm and parameter metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when the algorithm number is not part of the Rev. 1 table.
    pub fn from_compiled_algorithm(number: u8) -> Result<Self, &'static str> {
        let algorithm = mackes_profiles::lexicon_reflex::algorithms()
            .iter()
            .find(|algorithm| algorithm.number == number)
            .ok_or("unknown Reflex algorithm")?;
        let node_id = format!("algorithm-{}", algorithm.number);
        let controls = mackes_profiles::lexicon_reflex::parameters(number)
            .iter()
            .map(|parameter| ReflexControl {
                id: format!("parameter-{}", parameter.number),
                label: if parameter.mrc_name.is_empty() {
                    parameter.description.to_owned()
                } else {
                    parameter.mrc_name.to_owned()
                },
                node_id: node_id.clone(),
                available: true,
            })
            .collect();
        let diagram = SignalFlowDiagram::new(
            vec![SignalFlowNode {
                id: node_id.clone(),
                label: algorithm.name.to_owned(),
                online: false,
            }],
            0,
        )?;
        Self::new(node_id, algorithm.name.to_owned(), Vec::new(), controls, diagram)
    }

    /// Builds a workspace while preserving profile-provided order and labels.
    ///
    /// # Errors
    ///
    /// Returns an error when the algorithm identity or control fields are empty.
    pub fn new(
        algorithm_id: String,
        algorithm_label: String,
        shared_controls: Vec<ReflexControl>,
        controls: Vec<ReflexControl>,
        diagram: SignalFlowDiagram,
    ) -> Result<Self, &'static str> {
        if algorithm_id.is_empty() || algorithm_label.is_empty() {
            return Err("Reflex algorithm identity must not be empty");
        }
        if shared_controls.iter().chain(controls.iter()).any(|control| {
            control.id.is_empty() || control.label.is_empty() || control.node_id.is_empty()
        }) {
            return Err("Reflex control fields must not be empty");
        }
        Ok(Self {
            algorithm_id,
            algorithm_label,
            shared_controls,
            controls,
            diagram,
            selected_node: None,
        })
    }

    /// Highlights a diagram node and returns its linked controls in display order.
    ///
    /// # Errors
    ///
    /// Returns an error when the node is not present in the diagram.
    pub fn select_node(&mut self, node_id: &str) -> Result<Vec<&ReflexControl>, &'static str> {
        if !self.diagram.nodes.iter().any(|node| node.id == node_id) {
            return Err("unknown Reflex signal-flow node");
        }
        self.selected_node = Some(node_id.to_owned());
        Ok(self.controls.iter().filter(|control| control.node_id == node_id).collect())
    }

    /// Returns shared controls regardless of the selected processing node.
    #[must_use]
    pub fn shared_controls(&self) -> &[ReflexControl] {
        &self.shared_controls
    }
}

impl ReflexWorkspace {
    /// Returns the compiled algorithm page with permanent logical-view label,
    /// selected signal block, shared controls, and documented control order.
    #[must_use]
    pub fn frame_lines(&self, viewport: Viewport) -> Vec<String> {
        let mut lines = vec![
            format!("Reflex {} [{}]", self.algorithm_label, self.algorithm_id),
            "Logical/control view".into(),
            format!(
                "shared: {}",
                self.shared_controls
                    .iter()
                    .map(|control| control.label.as_str())
                    .collect::<Vec<_>>()
                    .join(" | ")
            ),
        ];
        for node in &self.diagram.nodes {
            let marker =
                if self.selected_node.as_deref() == Some(node.id.as_str()) { ">" } else { " " };
            lines.push(format!("{marker} [{}] {}", node.id, node.label));
            lines.extend(self.controls.iter().filter(|control| control.node_id == node.id).map(
                |control| {
                    format!(
                        "    {} {}",
                        if control.available { "[available]" } else { "[unavailable]" },
                        control.label
                    )
                },
            ));
        }
        clamp_lines(lines, viewport, usize::from(viewport.width))
    }
}

impl SignalFlowDiagram {
    /// Builds a diagram, rejecting empty identifiers and preserving order.
    ///
    /// # Errors
    ///
    /// Returns an error when a node identifier or label is empty.
    pub fn new(nodes: Vec<SignalFlowNode>, generation: u64) -> Result<Self, &'static str> {
        if nodes.iter().any(|node| node.id.is_empty() || node.label.is_empty()) {
            return Err("signal-flow node fields must not be empty");
        }
        Ok(Self { nodes, generation })
    }
}

impl PageState {
    /// Creates page state, clamping total to at least one.
    #[must_use]
    pub const fn new(current: u16, total: u16) -> Self {
        let total = if total == 0 { 1 } else { total };
        Self { current: if current >= total { total - 1 } else { current }, total }
    }
    /// Selects the next page with wraparound.
    pub const fn next(&mut self) {
        self.current = (self.current + 1) % self.total;
    }
    /// Selects the previous page with wraparound.
    pub const fn previous(&mut self) {
        self.current = if self.current == 0 { self.total - 1 } else { self.current - 1 };
    }
    /// Selects a page, clamping to the last page.
    pub const fn select(&mut self, page: u16) {
        self.current = if page >= self.total { self.total - 1 } else { page };
    }
}

impl DashboardState {
    /// Creates a safe initial dashboard state.
    #[must_use]
    pub fn initial() -> Self {
        Self { health: "starting".into(), panic_available: true, ..Self::default() }
    }

    /// Replaces activity counters from a daemon snapshot.
    pub const fn set_activity(&mut self, received: u64, sent: u64, dropped: u64) {
        self.received = received;
        self.sent = sent;
        self.dropped = dropped;
    }

    /// Advances the local activity age without depending on wall-clock time.
    pub const fn advance_live_activity_age(&mut self, elapsed_nanos: u64) {
        self.live_activity_age_nanos = self.live_activity_age_nanos.saturating_add(elapsed_nanos);
    }

    /// Replaces the bounded per-device health projection.
    pub fn set_device_health(&mut self, mut values: Vec<(String, String)>) {
        values.truncate(32);
        self.device_health = values;
    }

    /// Invalidates effects feedback when the device inventory is refreshed.
    pub fn refresh_effects_devices(&mut self, values: Vec<PhysicalDevice>) {
        self.physical_devices = values;
        self.effects_groups.request_resync();
    }

    /// Adds a notification and retains only the newest 16 entries.
    pub fn notify(&mut self, severity: SemanticToken, message: impl Into<String>) {
        self.notifications.insert(0, Notification { severity, message: message.into() });
        self.notifications.truncate(16);
    }

    /// Updates activation progress, clamping completed actions to total.
    pub const fn set_activation_progress(&mut self, completed: u32, total: u32) {
        let completed = if completed > total { total } else { completed };
        let previous = self.activation_progress;
        let bounded_completed =
            if total == previous.1 && completed < previous.0 { previous.0 } else { completed };
        self.activation_progress =
            (if bounded_completed > total { total } else { bounded_completed }, total);
    }

    /// Applies one typed daemon projection while preserving dashboard safety invariants.
    pub fn apply_event(&mut self, event: DashboardEvent) {
        match event {
            DashboardEvent::Health(value) => self.health = value,
            DashboardEvent::ActiveScene(value) => {
                self.active_scene = value;
                self.effects_groups.request_resync();
            }
            DashboardEvent::RouteGeneration(value) => self.route_generation = value,
            DashboardEvent::MappingStore { generation, undo_available } => {
                self.mapping_generation = generation;
                self.mapping_undo_available = undo_available;
            }
            DashboardEvent::MappingActivity(activity) => {
                self.mapping_activity = Some(activity.clone());
                let mappings = self
                    .mapping_browser
                    .rows
                    .iter()
                    .map(|row| row.mapping.clone())
                    .collect::<Vec<_>>();
                self.mapping_browser =
                    MappingBrowser::from_authoritative(&mappings, Some(&activity));
            }
            DashboardEvent::RouteUndoAvailable(value) => self.route_undo_available = value,
            DashboardEvent::AuditCount(value) => self.audit_count = value,
            DashboardEvent::LatestAudit(value) => self.latest_audit = Some(value),
            DashboardEvent::PerformanceLock(value) => self.performance_locked = value,
            DashboardEvent::Activity { received, sent, dropped } => {
                self.set_activity(received, sent, dropped);
            }
            DashboardEvent::ActivationProgress { completed, total } => {
                self.set_activation_progress(completed, total);
            }
            DashboardEvent::ActivationResult(value) => self.activation_result = Some(value),
            DashboardEvent::DeviceHealth(values) => self.set_device_health(values),
            DashboardEvent::Notification { severity, message } => self.notify(severity, message),
            DashboardEvent::LiveActivity(activity) => {
                self.live_activity = Some(activity);
                self.live_activity_age_nanos = 0;
            }
            DashboardEvent::PhysicalDevices(devices) => self.refresh_effects_devices(devices),
            DashboardEvent::ControlMappings(mappings) => {
                self.mapping_browser =
                    MappingBrowser::from_authoritative(&mappings, self.mapping_activity.as_ref());
            }
            DashboardEvent::LedStatus(value) => self.led_status = Some(value),
        }
    }

    /// Advances the selected output destination with wraparound.
    pub fn cycle_destination(&mut self) {
        let count = self.physical_devices.iter().map(|device| device.outputs.len()).sum::<usize>();
        self.selected_destination = if count == 0 {
            None
        } else {
            Some(self.selected_destination.map_or(0, |index| (index + 1) % count))
        };
        self.selected_parameter = None;
    }

    /// Advances the selected physical input source with wraparound.
    pub fn cycle_input(&mut self) {
        let count = self.physical_devices.iter().map(|device| device.inputs.len()).sum::<usize>();
        self.selected_input = if count == 0 {
            None
        } else {
            Some(self.selected_input.map_or(0, |index| (index + 1) % count))
        };
    }

    /// Advances the selected profile parameter for the selected destination.
    pub fn cycle_parameter(&mut self) {
        let Some(destination) = self.selected_destination else { return };
        let Some((device, _)) = self
            .physical_devices
            .iter()
            .flat_map(|device| device.outputs.iter().map(move |endpoint| (device, endpoint)))
            .nth(destination)
        else {
            return;
        };
        let count = destination_profile_for_name(&device.name)
            .map_or(0, |profile| mackes_profiles::destination_parameters(&profile).len());
        if count > 0 {
            self.selected_parameter =
                Some(self.selected_parameter.map_or(0, |index| (index + 1) % count));
        }
    }

    /// Returns the selected profile-owned destination parameter label.
    #[must_use]
    pub fn selected_parameter_label(&self) -> Option<String> {
        let destination = self.selected_destination?;
        let (device, _) = self
            .physical_devices
            .iter()
            .flat_map(|device| device.outputs.iter().map(move |endpoint| (device, endpoint)))
            .nth(destination)?;
        let profile = destination_profile_for_name(&device.name)?;
        mackes_profiles::destination_parameters(&profile)
            .get(self.selected_parameter.unwrap_or(0))
            .map(|parameter| parameter.id.clone())
    }

    /// Returns the selected output as the numeric route endpoint contract.
    #[must_use]
    pub fn selected_destination_endpoint(&self) -> Option<u64> {
        let index = self.selected_destination?;
        self.physical_devices
            .iter()
            .flat_map(|device| device.outputs.iter())
            .nth(index)
            .and_then(|endpoint| mackes_midi_engine::numeric_endpoint_id(endpoint))
            .map(mackes_domain::EndpointId::get)
    }

    /// Returns the first visible physical input as the numeric route endpoint contract.
    #[must_use]
    pub fn first_input_endpoint(&self) -> Option<u64> {
        self.physical_devices
            .iter()
            .flat_map(|device| device.inputs.iter())
            .next()
            .and_then(|endpoint| mackes_midi_engine::numeric_endpoint_id(endpoint))
            .map(mackes_domain::EndpointId::get)
    }

    /// Returns the selected input as the numeric route endpoint contract.
    #[must_use]
    pub fn selected_input_endpoint(&self) -> Option<u64> {
        let index = self.selected_input?;
        self.physical_devices
            .iter()
            .flat_map(|device| device.inputs.iter())
            .nth(index)
            .and_then(|endpoint| mackes_midi_engine::numeric_endpoint_id(endpoint))
            .map(mackes_domain::EndpointId::get)
    }

    /// Returns the canonical compact dashboard lines for terminal renderers.
    #[must_use]
    pub fn frame_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!(
                "mackes-midi-matrix  health={}  scene={}",
                self.health,
                self.active_scene.as_deref().unwrap_or("none")
            ),
            format!(
                "routes={}  midi in={} out={} dropped={}",
                self.route_generation, self.received, self.sent, self.dropped
            ),
            format!(
                "activation={}/{}  performance_lock={}",
                self.activation_progress.0, self.activation_progress.1, self.performance_locked
            ),
            format!("activation_result={}", self.activation_result.as_deref().unwrap_or("none")),
            format!("PANIC: {}", if self.panic_available { "available" } else { "unavailable" }),
            self.led_status.clone().unwrap_or_else(|| "led sent=0 failed=0".into()),
            "keys: 1 dashboard 2 learn 3 reflex 4 eventide 5 routing | n/p scene | ! panic | q quit"
                .into(),
        ];
        lines.extend(
            self.device_health
                .iter()
                .map(|(device, health)| format!("device={device} health={health}")),
        );
        lines.extend(
            self.notifications
                .iter()
                .map(|notice| format!("{} {}", token_marker(notice.severity), notice.message)),
        );
        lines
    }

    /// Returns width-safe dashboard lines, preserving all required panels.
    #[must_use]
    pub fn frame_lines_for(&self, viewport: Viewport) -> Vec<String> {
        let width = usize::from(viewport.width);
        let lines = if viewport.compact() {
            vec![
                format!("health={}", self.health),
                format!("scene={}", self.active_scene.as_deref().unwrap_or("none")),
                format!(
                    "routes={} in={} out={} drop={}",
                    self.route_generation, self.received, self.sent, self.dropped
                ),
                format!("activation={}/{}", self.activation_progress.0, self.activation_progress.1),
                format!("result={}", self.activation_result.as_deref().unwrap_or("none")),
                format!("devices={}", self.device_health.len()),
                format!("PANIC {}", if self.panic_available { "ON" } else { "OFF" }),
                "keys: 1-9 workspaces n/p scenes ! panic q quit".into(),
            ]
        } else {
            self.frame_lines()
        };
        lines.into_iter().map(|line| line.chars().take(width).collect()).collect()
    }
}

mod render;
pub use render::*;

#[cfg(test)]
mod tests;
