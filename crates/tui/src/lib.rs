//! Ratatui presentation boundary; this crate must not open MIDI ports.

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

/// Draws the dashboard using the canonical state projection and a single
/// bordered panel. Terminal setup and event polling remain the application's
/// responsibility.
pub fn draw_dashboard(frame: &mut Frame<'_>, area: Rect, dashboard: &DashboardState) {
    let text = dashboard.frame_lines_for(Viewport::new(area.width, area.height)).join("\n");
    let widget = Paragraph::new(text)
        .block(Block::default().title("mackes-midi-matrix").borders(Borders::ALL))
        .style(Style::default());
    frame.render_widget(widget, area);
}

/// Renders the task-oriented shell chrome around a focused workspace.
pub fn draw_task_shell(
    frame: &mut Frame<'_>,
    area: Rect,
    shell: &TaskShellState,
    dashboard: &DashboardState,
) {
    draw_task_shell_with_content(frame, area, shell, dashboard, None);
}

/// Renders the task shell with the focused section's capability view embedded.
pub fn draw_task_shell_with_content(
    frame: &mut Frame<'_>,
    area: Rect,
    shell: &TaskShellState,
    dashboard: &DashboardState,
    content: Option<&str>,
) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(3), Constraint::Length(1)])
        .split(area);
    let header = Paragraph::new(format!(
        "LIVE  health={}  scene={}  save=ready",
        dashboard.health,
        dashboard.active_scene.as_deref().unwrap_or("none")
    ))
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, vertical[0]);
    let rail_width = if area.width >= 80 { 16 } else { 10 };
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(rail_width), Constraint::Min(1)])
        .split(vertical[1]);
    let rail = AppSection::all()
        .iter()
        .map(|section| {
            let marker = if *section == shell.focus.section { "▶" } else { " " };
            format!("{marker} {}", section.label())
        })
        .collect::<Vec<_>>()
        .join("\n");
    frame.render_widget(
        Paragraph::new(rail).block(Block::default().borders(Borders::RIGHT)),
        body[0],
    );
    let mut main_lines = format!(
        "{}\n{}\n\nFocused: {}\nPosition: {} OF {}\n\n{}",
        shell.focus.breadcrumb(),
        section_landing(
            shell.focus.section,
            match shell.focus.section {
                AppSection::MapControls => !dashboard.mapping_browser.rows.is_empty(),
                AppSection::Devices => !dashboard.physical_devices.is_empty(),
                _ => dashboard.active_scene.is_some(),
            },
        ),
        shell.focus.target,
        shell.target_index + 1,
        shell.target_count,
        dashboard.health
    );
    if shell.focus.section == AppSection::MapControls {
        main_lines.push_str("\n\n");
        main_lines.push_str(
            &mapping_browser_lines(
                &dashboard.mapping_browser,
                Viewport::new(body[1].width, body[1].height),
            )
            .join("\n"),
        );
    }
    if !dashboard.assignment_feedback.is_empty() {
        main_lines.push_str("\n\n");
        main_lines.push_str(&dashboard.assignment_feedback.join("\n"));
    }
    if let Some(content) = content.filter(|content| !content.is_empty()) {
        main_lines.push_str("\n\n");
        main_lines.push_str(content);
    }
    let template_status =
        launch_control_template_readiness(dashboard.launch_control_template.as_ref());
    if template_status != "MACKES TEMPLATE READY" {
        main_lines.push_str("\n\n");
        main_lines.push_str(template_status);
    }
    let main = Paragraph::new(main_lines)
        .block(Block::default().borders(Borders::ALL).title(shell.focus.section.label()));
    frame.render_widget(main, body[1]);
    frame.render_widget(
        Paragraph::new("↑↓ focus  ←→ section  Enter select  Esc back  ? help  ! panic  q quit"),
        vertical[2],
    );
    if shell.help_visible {
        let width = 58.min(area.width);
        let height = 9.min(area.height);
        let popup = Rect {
            x: area.x + area.width.saturating_sub(width) / 2,
            y: area.y + area.height.saturating_sub(height) / 2,
            width,
            height,
        };
        frame.render_widget(
            Paragraph::new(format!(
                "HELP — {}\n{}\n↑↓ move focus   ←→ change task\nEnter select   Esc back\n! panic      q quit\n? close help",
                shell.focus.section.label(),
                section_help(shell.focus.section)
            ))
                .block(Block::default().borders(Borders::ALL).title("Keyboard")),
            popup,
        );
    }
}

const fn section_help(section: AppSection) -> &'static str {
    match section {
        AppSection::Live => "Perform the active scene; activity and health are read-only here.",
        AppSection::MapControls => {
            "Assign or inspect controls; Learn, browser, Advanced, enable, replace, delete, Undo."
        }
        AppSection::Scenes => {
            "Recall or organize projects, scenes, songs, and setlists; results stay inline."
        }
        AppSection::Devices => {
            "Inspect Reflex and MicroPitch profiles; unavailable devices remain visible with recovery."
        }
        AppSection::System => {
            "Review diagnostics, monitor, backups, configuration, and Advanced → Legacy tools."
        }
    }
}

/// Returns the bounded landing or empty-state copy for a task section.
#[must_use]
pub const fn section_landing(section: AppSection, has_content: bool) -> &'static str {
    if has_content {
        return section.purpose();
    }
    match section {
        AppSection::Live => "No active scene — select a scene to begin performance.",
        AppSection::MapControls => "No mappings yet — press Device on a controller to assign one.",
        AppSection::Scenes => "No scenes loaded — create or import a scene to continue.",
        AppSection::Devices => "No devices connected — connect a device and refresh.",
        AppSection::System => "No diagnostics available — the daemon will report status here.",
    }
}

impl Viewport {
    /// Creates a viewport, clamping each dimension to one cell.
    #[must_use]
    pub const fn new(width: u16, height: u16) -> Self {
        Self {
            width: if width == 0 { 1 } else { width },
            height: if height == 0 { 1 } else { height },
        }
    }
    /// Returns whether compact layout should be used.
    #[must_use]
    pub const fn compact(self) -> bool {
        self.width < 80 || self.height < 24
    }
}

impl Keymap {
    /// Resolves one key to an optional command.
    #[must_use]
    pub const fn command_for(self, key: char) -> Option<UiCommand> {
        match key {
            'n' => Some(UiCommand::NextScene),
            'p' => Some(UiCommand::PreviousScene),
            '!' => Some(UiCommand::Panic),
            ':' => Some(UiCommand::OpenPalette),
            'q' => Some(UiCommand::Quit),
            'k' => Some(UiCommand::MoveUp),
            'j' => Some(UiCommand::MoveDown),
            'h' => Some(UiCommand::MoveLeft),
            'l' => Some(UiCommand::MoveRight),
            '1'..='5' => Some(UiCommand::OpenWorkspace((key as u8) - b'0')),
            _ => None,
        }
    }

    /// Returns a concise operator-facing description for a key.
    #[must_use]
    pub const fn description(self, key: char) -> Option<&'static str> {
        match key {
            'h' => Some("Left"),
            'j' => Some("Down"),
            'k' => Some("Up"),
            'l' => Some("Right"),
            'n' => Some("Next scene"),
            'p' => Some("Previous scene"),
            '!' => Some("Panic"),
            ':' => Some("Command palette"),
            'q' => Some("Quit"),
            '1'..='9' => Some("Open workspace"),
            _ => None,
        }
    }
}

impl ClientState {
    /// Marks the client ready for a fresh reconnect snapshot while retaining
    /// the last trusted payload for degraded rendering.
    pub const fn begin_reconnect(&mut self) {
        self.last_sequence = 0;
    }

    /// Replaces state from a daemon snapshot.
    pub fn apply_snapshot(&mut self, snapshot: StateSnapshot) {
        self.last_sequence = snapshot.last_sequence;
        self.payload = snapshot.payload;
    }
    /// Applies a snapshot only when it is not older than the current state.
    ///
    /// # Errors
    ///
    /// Returns [`ReducerError::SequenceGap`] for a stale snapshot.
    pub fn apply_snapshot_if_newer(&mut self, snapshot: StateSnapshot) -> Result<(), ReducerError> {
        if snapshot.last_sequence < self.last_sequence {
            return Err(ReducerError::SequenceGap);
        }
        self.apply_snapshot(snapshot);
        Ok(())
    }
    /// Applies one event only when sequence continuity is preserved.
    ///
    /// # Errors
    ///
    /// Returns `SequenceGap` when the event does not immediately follow the
    /// last applied sequence.
    pub fn apply_event(&mut self, event: StateEvent) -> Result<(), ReducerError> {
        if event.sequence != self.last_sequence.saturating_add(1) {
            return Err(ReducerError::SequenceGap);
        }
        self.last_sequence = event.sequence;
        self.payload = event.payload;
        Ok(())
    }

    /// Rebuilds state from a snapshot and contiguous post-snapshot events.
    /// The receiver is unchanged if continuity cannot be proven.
    ///
    /// # Errors
    ///
    /// Returns [`ReducerError::SequenceGap`] when events are stale or skipped.
    pub fn apply_reconnect(
        &mut self,
        snapshot: StateSnapshot,
        events: &[StateEvent],
    ) -> Result<(), ReducerError> {
        if mackes_ipc::validate_reconnect(&snapshot, events).is_err() {
            return Err(ReducerError::SequenceGap);
        }
        let mut candidate = Self::default();
        candidate.apply_snapshot(snapshot);
        for event in events {
            candidate.apply_event(event.clone())?;
        }
        *self = candidate;
        Ok(())
    }
}

fn clamp_lines(lines: Vec<String>, viewport: Viewport, width: usize) -> Vec<String> {
    lines
        .into_iter()
        .take(usize::from(viewport.height))
        .map(|line| line.chars().take(width).collect())
        .collect()
}

impl LearnWorkspace {
    /// Returns a deterministic Learn frame with explicit phase, selected
    /// endpoint, candidates, raw evidence, destination, and live-test state.
    #[must_use]
    pub fn frame_lines(&self, viewport: Viewport) -> Vec<String> {
        let mut lines = vec![
            format!(
                "MIDI Learn — {:?} (l capture, j/k select, Enter accept, r route, t test, Esc cancel)",
                self.phase
            ),
            format!("input={}", self.learn_input_alias.as_deref().unwrap_or("not selected")),
        ];
        for (index, candidate) in self.candidates.iter().enumerate() {
            let marker = if self.selected == Some(index) { ">" } else { " " };
            let raw = candidate
                .raw
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(format!(
                "{marker} {:?} ch={:?} num={:?} count={} range={:?}..{:?}",
                candidate.kind,
                candidate.channel,
                candidate.number,
                candidate.observations,
                candidate.minimum,
                candidate.maximum
            ));
            lines.push(format!("    raw={raw}"));
        }
        if let Some((destination, mode)) = &self.destination {
            lines.push(format!("destination={destination} mode={mode:?}"));
        }
        lines.push(format!(
            "live_test={}",
            if self.live_test_passed {
                "passed"
            } else if self.phase == LearnPhase::Testing {
                "awaiting daemon/device result (Esc cancels)"
            } else {
                "required"
            }
        ));
        clamp_lines(lines, viewport, usize::from(viewport.width))
    }
}

impl DiagnosticsState {
    /// Returns bounded actionable diagnostics for rendering.
    #[must_use]
    pub fn frame_lines(&self, viewport: Viewport) -> Vec<String> {
        let mut lines = vec![format!("Diagnostics — {} issue(s)", self.entries.len())];
        lines.extend(self.lines());
        clamp_lines(lines, viewport, usize::from(viewport.width))
    }
}

impl SetlistEditor {
    /// Returns the transactional setlist draft without implying persistence.
    #[must_use]
    pub fn frame_lines(&self, viewport: Viewport) -> Vec<String> {
        let mut lines = vec![
            "Setlists — uncommitted draft (a add, p project, x remove, [/ ] project order, j/k select, </> reorder, c copy, d delete, s save)"
                .into(),
        ];
        lines.extend(self.drafts.iter().enumerate().map(|(index, setlist)| {
            format!(
                "{} {}: {}",
                if self.selected == Some(index) { ">" } else { " " },
                setlist.id,
                setlist.projects.join(" -> ")
            )
        }));
        clamp_lines(lines, viewport, usize::from(viewport.width))
    }
}

fn draw_lines(frame: &mut Frame<'_>, area: Rect, title: &str, lines: &[String]) {
    let widget = Paragraph::new(lines.join("\n"))
        .block(Block::default().title(title).borders(Borders::ALL))
        .style(Style::default());
    frame.render_widget(widget, area);
}

/// Draws the MIDI Learn workspace.
pub fn draw_learn(frame: &mut Frame<'_>, area: Rect, workspace: &LearnWorkspace) {
    draw_lines(
        frame,
        area,
        "MIDI Learn",
        &workspace.frame_lines(Viewport::new(area.width, area.height)),
    );
}

/// Draws the Reflex algorithm workspace.
pub fn draw_reflex(frame: &mut Frame<'_>, area: Rect, workspace: &ReflexWorkspace) {
    draw_lines(
        frame,
        area,
        "Lexicon Reflex",
        &workspace.frame_lines(Viewport::new(area.width, area.height)),
    );
}

/// Draws a profile-backed device workspace such as Eventide `MicroPitch`.
pub fn draw_device(frame: &mut Frame<'_>, area: Rect, workspace: &DeviceWorkspace) {
    draw_lines(
        frame,
        area,
        &workspace.device_label,
        &workspace.frame_lines(Viewport::new(area.width, area.height)),
    );
}

/// Draws the diagnostics workspace.
pub fn draw_diagnostics(frame: &mut Frame<'_>, area: Rect, diagnostics: &DiagnosticsState) {
    draw_lines(
        frame,
        area,
        "Diagnostics",
        &diagnostics.frame_lines(Viewport::new(area.width, area.height)),
    );
}

/// Draws the bounded event monitor.
pub fn draw_monitor(frame: &mut Frame<'_>, area: Rect, monitor: &MonitorState) {
    draw_lines(
        frame,
        area,
        "Monitor",
        &monitor.frame_lines(Viewport::new(area.width, area.height)),
    );
}

/// Draws the backup operation workspace.
pub fn draw_backups(frame: &mut Frame<'_>, area: Rect, workspace: &BackupWorkspace) {
    draw_lines(
        frame,
        area,
        "Backups",
        &workspace.frame_lines(Viewport::new(area.width, area.height)),
    );
}

/// Draws the transactional setlist editor.
pub fn draw_setlists(frame: &mut Frame<'_>, area: Rect, editor: &SetlistEditor) {
    draw_lines(
        frame,
        area,
        "Setlists",
        &editor.frame_lines(Viewport::new(area.width, area.height)),
    );
}

/// Draws the transactional routing and mapping editor.
pub fn draw_routing(frame: &mut Frame<'_>, area: Rect, editor: &RoutingEditor) {
    draw_lines(
        frame,
        area,
        "Routing & mappings",
        &editor.frame_lines(Viewport::new(area.width, area.height)),
    );
}

/// Draws the operator-facing Launch Control XL mapping surface.
///
/// The hardware layout is intentionally spatial: the three rotary rows, two
/// button rows, and eight faders match the controller so an operator can read
/// the screen from a distance and locate a physical control immediately.
#[allow(clippy::too_many_lines)]
pub fn draw_controller_mapping(
    frame: &mut Frame<'_>,
    area: Rect,
    dashboard: &DashboardState,
    editor: &RoutingEditor,
) {
    let launch_control_present = dashboard
        .physical_devices
        .iter()
        .any(|device| device.name.to_ascii_lowercase().contains("launch control"));
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(12), Constraint::Length(8)])
        .split(area);
    let online = health_is_online(&dashboard.health);
    let status_style = Style::default()
        .fg(if online { Color::LightGreen } else { Color::LightYellow })
        .bg(Color::Black);
    let header_line = Line::from(vec![
        Span::styled(
            " MIDI MAPPING ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("SCENE {:<12}", dashboard.active_scene.as_deref().unwrap_or("NONE")),
            Style::default().fg(Color::White),
        ),
        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("IN {:>5}  OUT {:>5}", dashboard.received, dashboard.sent),
            Style::default().fg(Color::LightBlue),
        ),
        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            if online { "● ONLINE" } else { "● DEGRADED" },
            status_style.add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            if dashboard.mapping_dirty { "◆ SAVE REQUIRED" } else { "✓ SAVED" },
            Style::default()
                .fg(if dashboard.mapping_dirty { Color::Yellow } else { Color::Green })
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(header_line)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan).bg(Color::Black))
                    .title("MACKES  /  CONNECTED MIDI RACK  /  PANIC"),
            ),
        vertical[0],
    );

    let compact = RackShellLayout::for_terminal(area.width, area.height).compact;
    let main = if compact {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1)])
            .split(vertical[1])
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(68), Constraint::Length(28)])
            .split(vertical[1])
    };
    let mut lines = vec![
        format!("  DEVICE TABS: {}", device_tabs_line(&dashboard.physical_devices)),
        format!(
            "  SOURCE: {}",
            input_inventory_line(&dashboard.physical_devices, dashboard.selected_input)
        ),
        format!(
            "  DESTINATIONS: {}",
            destination_inventory_line(&dashboard.physical_devices, dashboard.selected_destination)
        ),
        format!(
            "  PARAMETERS: {}",
            destination_parameter_line(
                &dashboard.physical_devices,
                dashboard.selected_destination,
                dashboard.selected_parameter
            )
        ),
        "  KNOBS                                      BUTTONS                         FADERS"
            .into(),
        "  ┌────────┬────────┬────────┬────────┬────────┬────────┬────────┬────────┐".into(),
    ];
    for row in 0..3 {
        let mut line = format!(
            "  │ {}",
            (0..8)
                .map(|column| {
                    let index = row * 8 + column;
                    control_cell(
                        index,
                        editor,
                        launch_control_present,
                        launch_control_activity_index(dashboard),
                        faceplate_control_state(index, dashboard, editor),
                    )
                })
                .collect::<Vec<_>>()
                .join(" │ ")
        );
        line.push_str(" │");
        lines.push(line);
        lines.push(
            "  ├────────┼────────┼────────┼────────┼────────┼────────┼────────┼────────┤".into(),
        );
    }
    if launch_control_present {
        lines.push(
            "  │  CHANNEL BUTTONS:  [T01] [T02] [T03] [T04] [T05] [T06] [T07] [T08] │".into(),
        );
        lines.push(
            "  │                    [B01] [B02] [B03] [B04] [B05] [B06] [B07] [B08] │".into(),
        );
    } else {
        lines.push(
            "  │  GENERIC MIDI CONTROLS — PROFILE FACEPLATE UNAVAILABLE                 │".into(),
        );
        lines.push(
            "  │  Select a supported device tab to show its documented control layout.   │".into(),
        );
    }
    lines.push("  └──────────────────────────────────────────────────────────────────────────────────────┘".into());
    lines.push(if launch_control_present {
        "  FADERS:   [F01]    [F02]    [F03]    [F04]    [F05]    [F06]    [F07]    [F08]".into()
    } else {
        "  FADERS:   profile-specific controls unavailable; mappings remain editable".into()
    });
    lines.push("            │  ░░  │  ░░  │  ░░  │  ░░  │  ░░  │  ░░  │  ░░  │  ░░  │  8-channel level bank".into());
    let effects = mackes_profiles::launch_control_effects_faceplate();
    lines.push(format!(
        "  EFFECTS: {} groups [{}]",
        effects.groups.len(),
        effects.groups.iter().map(|group| group.label.clone()).collect::<Vec<_>>().join(" ")
    ));
    lines.push("  EFFECTS: FADERS F01–F08  UNUSED C37–C40  LED OFF/UNKNOWN".into());
    if let Some(index) = launch_control_activity_index(dashboard) {
        let label = launch_control_index_label(index).unwrap_or_else(|| "unknown".into());
        let value = dashboard.live_activity.as_ref().and_then(|activity| activity.value);
        let destination = dashboard
            .launch_control_template
            .as_ref()
            .and_then(|template| template.assignment(index))
            .and_then(|assignment| assignment.destination.as_deref())
            .unwrap_or("UNASSIGNED");
        lines.push(format!(
            "  LIVE CONTROL: #{index:02} {label}  VALUE {}  DEST {destination}  {}",
            value.map_or_else(|| "--".into(), |value| value.to_string()),
            live_activity_age_label(dashboard.live_activity_age_nanos)
        ));
    }
    lines.push("  UTILITY:  DEVICE  MUTE  SOLO  RECORD ARM  UP  DOWN  LEFT  RIGHT".into());
    lines.push(format!(
        "  ACTIVE CHAIN: {}   UNDO: {}   AUDIT: {}{}",
        if editor.drafts.is_empty() { "NO MAPPINGS" } else { "MAPPING BANK" },
        if dashboard.route_undo_available { "AVAILABLE" } else { "EMPTY" },
        dashboard.audit_count,
        dashboard.latest_audit.as_deref().map_or(String::new(), |value| format!(" ({value})"))
    ));
    lines.push(format!("  ROUTES: {}", mapping_chain_line(&editor.drafts)));
    if let Some(activity) = &dashboard.live_activity {
        let value = activity.value.map_or_else(|| "--".to_owned(), |value| value.to_string());
        let number = activity.number.map_or_else(|| "--".to_owned(), |number| number.to_string());
        lines.push(format!(
            "  LIVE: {} [{}]  #{}  VALUE {}  AGE {}ms  → {} DESTINATION(S)",
            activity.kind,
            live_activity_age_label(dashboard.live_activity_age_nanos),
            number,
            value,
            dashboard.live_activity_age_nanos / 1_000_000,
            activity.destination_endpoints.len()
        ));
        lines.push(format!(
            "  LEVEL: {}  SOURCE EP {:>3}  SEQ {:>6}  T={}",
            activity.value.map_or_else(|| "N/A".to_owned(), value_bar),
            activity.source_endpoint,
            activity.sequence,
            activity.timestamp_nanos
        ));
        lines.push(format!(
            "  SOURCE ID: {}  CONTROL ID: {}",
            activity.source_endpoint_id.as_deref().unwrap_or("unresolved"),
            activity.control_id
        ));
    } else {
        lines.push("  LIVE: waiting for MIDI activity".into());
    }
    let surface_title = if !launch_control_present {
        "CONTROL SURFACE  ·  GENERIC MIDI"
    } else if editor.drafts.is_empty() {
        "CONTROL SURFACE  ·  NO ACTIVE MAPPINGS"
    } else {
        "CONTROL SURFACE  ·  ACTIVE MAPPINGS"
    };
    frame.render_widget(
        Paragraph::new(lines.join("\n"))
            .style(Style::default().fg(Color::Gray).bg(Color::Black))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(Span::styled(
                        surface_title,
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )),
            ),
        main[0],
    );

    let selected = editor.selected.and_then(|index| editor.drafts.get(index));
    let inspector = selected.map_or_else(
        || " SELECT A CONTROL\n\n j/k  move mapping\n a    add mapping\n e    enable/disable\n s    save changes\n !    panic\n\n No control selected".into(),
        |mapping| format!(
            " SELECTED CONTROL\n\n SOURCE\n {}\n\n MESSAGE\n {:?}\n\n DESTINATION\n {}\n PARAMETER\n {}\n\n STATE\n {}\n\n PRIORITY\n {}",
            mapping.source, mapping.mode, mapping.destination,
            mapping.destination_parameter.as_deref().unwrap_or("not selected"),
            if mapping.enabled { "● ENABLED" } else { "○ DISABLED" }, mapping.priority
        ),
    );
    if !compact {
        frame.render_widget(
            Paragraph::new(inspector)
                .style(Style::default().fg(Color::White).bg(Color::Black))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow))
                        .title(Span::styled(
                            "MAPPING INSPECTOR",
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                        )),
                ),
            main[1],
        );
    }
    let footer_line = if compact {
        Line::from(" ! PANIC  1 HOME  2 LEARN  5 ROUTING  j/k SELECT  q QUIT")
    } else {
        Line::from(vec![
            Span::styled(" 1 HOME ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(
                " 2 LEARN ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " 5 ROUTING ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  j/k SELECT  I INPUT  D DESTINATION  P PARAMETER  a ADD  u UNDO  s SAVE  ",
                Style::default().fg(Color::White),
            ),
            Span::styled("! PANIC ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(" q QUIT", Style::default().fg(Color::White)),
        ])
    };
    frame.render_widget(
        Paragraph::new(footer_line)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(Span::styled(
                        "OPERATOR CONTROLS",
                        Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD),
                    )),
            ),
        vertical[2],
    );
}

fn value_bar(value: u16) -> String {
    let bounded = value.min(127);
    let filled = usize::from(bounded.saturating_mul(16) / 127);
    format!("[{}{}]", "#".repeat(filled), "-".repeat(16 - filled))
}

fn device_tabs_line(devices: &[PhysicalDevice]) -> String {
    if devices.is_empty() {
        return "[ NO CONNECTED DEVICES ]".to_owned();
    }
    devices
        .iter()
        .take(6)
        .map(|device| {
            let state = if device.state.eq_ignore_ascii_case("connected") { "●" } else { "○" };
            format!("[{state} {}]", device.name)
        })
        .collect::<Vec<_>>()
        .join("  ")
}

fn input_inventory_line(devices: &[PhysicalDevice], selected: Option<usize>) -> String {
    let inputs = devices
        .iter()
        .flat_map(|device| {
            device.inputs.iter().map(move |endpoint| device.name.clone() + " / " + endpoint)
        })
        .enumerate()
        .map(|(index, endpoint)| {
            format!("{}{}", if selected == Some(index) { ">" } else { " " }, endpoint)
        })
        .collect::<Vec<_>>();
    if inputs.is_empty() {
        "NO INPUT SOURCES".to_owned()
    } else {
        inputs.join("  |  ")
    }
}

fn mapping_chain_line(drafts: &[MappingDraft]) -> String {
    if drafts.is_empty() {
        return "NONE — press a to add a source → destination route".to_owned();
    }
    drafts
        .iter()
        .take(4)
        .map(|draft| {
            format!(
                "{} {}→{}",
                if draft.enabled { "●" } else { "○" },
                draft.source,
                draft.destination
            )
        })
        .collect::<Vec<_>>()
        .join("  |  ")
}

fn destination_inventory_line(devices: &[PhysicalDevice], selected: Option<usize>) -> String {
    let destinations = devices
        .iter()
        .flat_map(|device| {
            device.outputs.iter().map(|endpoint| (device.name.clone(), endpoint.clone()))
        })
        .take(6)
        .enumerate()
        .map(|(index, (device, endpoint))| {
            format!("{}{}:{}", if selected == Some(index) { ">" } else { " " }, device, endpoint)
        })
        .collect::<Vec<_>>();
    if destinations.is_empty() {
        "NO OUTPUT DESTINATIONS".to_owned()
    } else {
        destinations.join("  →  ")
    }
}

fn health_is_online(health: &str) -> bool {
    matches!(health, "online" | "ready")
}

const fn live_activity_age_label(age_nanos: u64) -> &'static str {
    if age_nanos < 1_000_000_000 {
        "ACTIVE"
    } else {
        "STALE"
    }
}

fn destination_parameter_line(
    devices: &[PhysicalDevice],
    selected: Option<usize>,
    selected_parameter: Option<usize>,
) -> String {
    let Some(index) = selected else { return "SELECT A DESTINATION WITH D".to_owned() };
    let Some((device, _)) = devices
        .iter()
        .flat_map(|device| device.outputs.iter().map(move |endpoint| (device, endpoint)))
        .nth(index)
    else {
        return "DESTINATION UNAVAILABLE".to_owned();
    };
    let Some(profile) = destination_profile_for_name(&device.name) else {
        return "PROFILE PARAMETERS UNVERIFIED".to_owned();
    };
    let parameters = mackes_profiles::destination_parameters(&profile);
    if parameters.is_empty() {
        return "NO DOCUMENTED PARAMETERS".to_owned();
    }
    parameters
        .iter()
        .enumerate()
        .take(24)
        .map(|(index, parameter)| {
            let marker = if selected_parameter == Some(index) { ">" } else { "" };
            let support = match parameter.support {
                mackes_profiles::ParameterSupport::ReadWrite => "RW",
                mackes_profiles::ParameterSupport::WriteOnly => "WO",
                mackes_profiles::ParameterSupport::ReadOnly => "RO",
                mackes_profiles::ParameterSupport::Unknown => "??",
            };
            format!(
                "{marker}{} [{} {}-{} {support}]",
                parameter.label, parameter.range.0, parameter.range.1, parameter.category
            )
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn destination_profile_for_name(name: &str) -> Option<mackes_profiles::DeviceProfile> {
    let name = name.to_ascii_lowercase();
    let id = if name.contains("micropitch") {
        "eventide.micropitch"
    } else if name.contains("reflex") || name.contains("lexicon") {
        "lexicon.reflex"
    } else {
        return None;
    };
    mackes_profiles::builtin_profile(id)
}

fn control_cell(
    index: usize,
    editor: &RoutingEditor,
    launch_control_present: bool,
    live_index: Option<u8>,
    state: FaceplateControlState,
) -> String {
    let marker = if editor.selected == Some(index) {
        ">"
    } else if live_index == u8::try_from(index).ok() {
        "*"
    } else {
        faceplate_state_marker(state).get(..1).unwrap_or("?")
    };
    let mapped =
        editor.drafts.get(index).map_or("·", |draft| if draft.enabled { "●" } else { "○" });
    let label = if launch_control_present {
        u8::try_from(index).ok().and_then(launch_control_index_label).map_or_else(
            || format!("C{:02}", index + 1),
            |label| match index {
                0..=7 => format!("T{:02}", index + 1),
                8..=15 => format!("M{:02}", index - 7),
                16..=23 => format!("B{:02}", index - 15),
                _ => label,
            },
        )
    } else {
        format!("C{:02}", index + 1)
    };
    format!("{marker}{mapped}{label:<6}")
}

fn faceplate_control_state(
    index: usize,
    dashboard: &DashboardState,
    editor: &RoutingEditor,
) -> FaceplateControlState {
    let index_usize = index;
    if dashboard.physical_devices.iter().all(|device| device.state != "connected") {
        return FaceplateControlState::Offline;
    }
    let Some(index) = u8::try_from(index).ok() else { return FaceplateControlState::Unknown };
    if launch_control_index_label(index).is_none() {
        return FaceplateControlState::Unknown;
    }
    if launch_control_activity_index(dashboard) == Some(index) {
        return FaceplateControlState::Live;
    }
    if editor.drafts.get(index_usize).is_some_and(|draft| draft.enabled) {
        FaceplateControlState::Mapped
    } else {
        FaceplateControlState::Unmapped
    }
}

fn launch_control_activity_index(dashboard: &DashboardState) -> Option<u8> {
    let activity = dashboard.live_activity.as_ref()?;
    let template = dashboard.launch_control_template.as_ref()?;
    let kind = match activity.kind.as_str() {
        "control_change" => LaunchControlMessageKind::Cc,
        "note_on" | "note_off" => LaunchControlMessageKind::Note,
        _ => return None,
    };
    template.resolve_activity_control(activity.channel?, activity.number?, kind)
}

/// Converts validated persisted assignments into the profile contract.
#[must_use]
pub fn launch_control_template_from_config(
    template: &mackes_config::LaunchControlTemplateConfig,
) -> Option<LaunchControlTemplate> {
    let result = LaunchControlTemplate {
        template: template.template,
        assignments: template
            .assignments
            .iter()
            .map(|assignment| {
                Some(mackes_profiles::LaunchControlAssignment {
                    index: assignment.index,
                    channel: assignment.channel,
                    number: assignment.number,
                    kind: match assignment.kind.as_str() {
                        "cc" => LaunchControlMessageKind::Cc,
                        "note" => LaunchControlMessageKind::Note,
                        _ => return None,
                    },
                    destination: assignment.destination.clone(),
                })
            })
            .collect::<Option<Vec<_>>>()?,
    };
    (result.validate().is_ok()).then_some(result)
}

/// Returns the operator-facing readiness state for the documented User 1 layout.
///
/// The decision is deliberately fail-closed: an absent, invalid, non-User-1, or
/// incomplete template cannot be treated as ready for deterministic capture.
#[must_use]
pub fn launch_control_template_readiness(template: Option<&LaunchControlTemplate>) -> &'static str {
    let Some(template) = template else {
        return "MACKES TEMPLATE REQUIRED";
    };
    if template.validate().is_err() || template.template != 0 {
        return "MACKES TEMPLATE MISMATCH";
    }
    let assignable = mackes_profiles::launch_control_physical_catalog()
        .iter()
        .filter(|control| control.role != mackes_profiles::PhysicalControlRole::Utility)
        .count();
    if template.assignments.len() != assignable {
        return "MACKES TEMPLATE REQUIRED";
    }
    "MACKES TEMPLATE READY"
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn rack_lamps_have_ansi_safe_markers_and_semantic_colors() {
        assert_eq!(RackLamp::Offline.marker(), "o");
        assert_eq!(RackLamp::Enabled.marker(), "*");
        assert_eq!(RackLamp::Error.color(), Color::Red);
        assert_ne!(RackLamp::Offline.marker(), RackLamp::Enabled.marker());
    }

    #[test]
    fn faceplate_control_states_have_distinct_ascii_markers() {
        let states = [
            FaceplateControlState::Offline,
            FaceplateControlState::Unknown,
            FaceplateControlState::Unmapped,
            FaceplateControlState::Mapped,
            FaceplateControlState::Live,
        ];
        let markers: std::collections::HashSet<_> =
            states.into_iter().map(faceplate_state_marker).collect();
        assert_eq!(markers.len(), 5);
        assert_eq!(faceplate_state_marker(FaceplateControlState::Live), "LIVE");
    }

    #[test]
    fn rack_value_bar_is_bounded_and_monotonic() {
        assert_eq!(RackValueBar::new(0, 100).cells(), (0, 64));
        assert_eq!(RackValueBar::new(127, 8).cells(), (8, 0));
        assert!(RackValueBar::new(71, 16).cells().0 < RackValueBar::new(72, 16).cells().0);
    }

    #[test]
    fn rack_renderers_are_bounded_and_color_independent() {
        let lamp = rack_lamp_line("ONLINE", RackLamp::Enabled);
        assert_eq!(lamp.to_string(), "* ONLINE");
        let bar = rack_value_bar_line("Mix", RackValueBar::new(127, 8));
        assert_eq!(bar.to_string(), "Mix          [========] 127");
        assert!(!bar.to_string().contains('\u{1b}'));
    }

    #[test]
    fn rack_shell_layout_collapses_only_below_required_viewport() {
        assert!(!RackShellLayout::for_terminal(100, 37).compact);
        assert!(RackShellLayout::for_terminal(80, 24).compact);
        let layout = RackShellLayout::for_terminal(80, 24);
        assert_eq!((layout.status_rows, layout.alert_rows, layout.footer_rows), (1, 1, 1));
    }

    #[test]
    fn rack_shell_keeps_critical_bands_visible_at_required_sizes() {
        let expanded = rack_shell_lines(
            RackShellLayout::for_terminal(100, 37),
            "ready",
            Some("route saved"),
            true,
        );
        let compact = rack_shell_lines(
            RackShellLayout::for_terminal(80, 24),
            "offline",
            Some("device missing"),
            false,
        );
        assert_eq!(expanded.len(), 3);
        assert!(expanded[0].contains("PANIC READY"));
        assert!(expanded[1].contains("route saved"));
        assert!(expanded[2].contains("[!] PANIC"));
        assert!(compact[0].contains("PANIC HELD"));
        assert!(compact.iter().all(|line| line.len() <= 79));
    }

    #[test]
    fn rack_semantic_states_and_long_alerts_remain_distinguishable() {
        let markers: Vec<&str> = [
            RackLamp::Offline,
            RackLamp::Disabled,
            RackLamp::Enabled,
            RackLamp::Warning,
            RackLamp::Error,
        ]
        .into_iter()
        .map(RackLamp::marker)
        .collect();
        assert_eq!(
            markers.len(),
            markers.iter().copied().collect::<std::collections::HashSet<_>>().len()
        );
        let long_alert = "warning: ".to_owned() + &"x".repeat(200);
        let lines = rack_shell_lines(
            RackShellLayout::for_terminal(100, 37),
            "ready",
            Some(&long_alert),
            true,
        );
        assert!(lines[1].len() <= 99);
        assert!(lines[1].starts_with("ALERT"));
    }

    #[test]
    fn rack_shell_golden_frames_are_stable() {
        assert_eq!(
            rack_shell_lines(RackShellLayout::for_terminal(100, 37), "ready", Some("none"), true),
            vec![
                "MACKES RACK | HEALTH ready | PANIC READY",
                "ALERT  none",
                "[1] HOME  [2] ROUTING  [3] SCENES  [4] DIAGNOSTICS  [!] PANIC",
            ]
        );
        assert_eq!(
            rack_shell_lines(RackShellLayout::for_terminal(80, 24), "offline", None, false),
            vec![
                "MACKES RACK | HEALTH offline | PANIC HELD",
                "ALERT  none",
                "[1] HOME  [2] ROUTING  [3] DIAGNOSTICS",
            ]
        );
    }

    #[test]
    fn controller_renderer_draws_required_viewports_without_overflow() {
        for (width, height) in [(100, 37), (80, 24)] {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("test backend");
            let dashboard = DashboardState { health: "ready".into(), ..DashboardState::default() };
            let editor = RoutingEditor::from_bank(&MappingBank::new());
            terminal
                .draw(|frame| draw_controller_mapping(frame, frame.area(), &dashboard, &editor))
                .expect("draw");
            let buffer = terminal.backend().buffer();
            let rendered: String =
                buffer.content().iter().map(ratatui::buffer::Cell::symbol).collect();
            assert!(rendered.contains("CONNECTED MIDI RACK"));
            assert!(rendered.contains("PANIC"));
        }
    }

    #[test]
    fn task_shell_renderer_keeps_rail_focus_and_footer_visible() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend");
        let mut shell = TaskShellState::initial(3).expect("shell");
        shell.apply(ShellAction::Help);
        terminal
            .draw(|frame| draw_task_shell(frame, frame.area(), &shell, &DashboardState::default()))
            .expect("draw");
        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(rendered.contains("▶ Live"));
        assert!(rendered.contains("Map Controls"));
        assert!(rendered.contains("Esc back"));
        assert!(rendered.contains("HELP"));
        assert!(!rendered.to_ascii_lowercase().contains("workspace"));
        assert!(!rendered.contains("1-9"));
    }

    #[test]
    fn task_shell_renderer_shows_assignment_feedback_at_compact_width() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend");
        let shell = TaskShellState::initial(3).expect("shell");
        let dashboard = DashboardState {
            assignment_feedback: vec![
                "ASSIGNMENT  MOVE ONLY ONE CONTROL".into(),
                "CHOOSE DEVICE  1/2".into(),
            ],
            ..DashboardState::default()
        };
        terminal
            .draw(|frame| draw_task_shell(frame, frame.area(), &shell, &dashboard))
            .expect("draw");
        let buffer = terminal.backend().buffer();
        let rendered: String = buffer.content().iter().map(ratatui::buffer::Cell::symbol).collect();
        assert!(rendered.contains("MOVE ONLY ONE CONTROL"));
        assert!(rendered.contains("CHOOSE DEVICE"));
    }

    #[test]
    fn task_shell_renderer_surfaces_missing_controller_template() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend");
        let shell = TaskShellState::initial(3).expect("shell");
        terminal
            .draw(|frame| draw_task_shell(frame, frame.area(), &shell, &DashboardState::default()))
            .expect("draw");
        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(rendered.contains("MACKES TEMPLATE REQUIRED"));
    }

    #[test]
    fn controller_renderer_preserves_degraded_and_dirty_state() {
        let backend = TestBackend::new(100, 37);
        let mut terminal = Terminal::new(backend).expect("test backend");
        let dashboard = DashboardState {
            health: "offline".into(),
            mapping_dirty: true,
            ..DashboardState::default()
        };
        let editor = RoutingEditor::from_bank(&MappingBank::new());
        terminal
            .draw(|frame| draw_controller_mapping(frame, frame.area(), &dashboard, &editor))
            .expect("draw");
        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(rendered.contains("DEGRADED"));
        assert!(rendered.contains("SAVE REQUIRED"));
    }

    #[test]
    fn launch_control_renderer_exposes_documented_faceplate_banks() {
        let backend = TestBackend::new(100, 37);
        let mut terminal = Terminal::new(backend).expect("test backend");
        let dashboard = DashboardState {
            health: "ready".into(),
            physical_devices: vec![PhysicalDevice {
                id: "launch-control-xl".into(),
                name: "Launch Control XL Mk2".into(),
                inputs: vec!["lc-in".into()],
                outputs: vec!["lc-out".into()],
                state: "connected".into(),
            }],
            ..DashboardState::default()
        };
        let editor = RoutingEditor::from_bank(&MappingBank::new());
        terminal
            .draw(|frame| draw_controller_mapping(frame, frame.area(), &dashboard, &editor))
            .expect("draw");
        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        for label in ["T01", "M01", "B01", "DEVICE", "MUTE", "SOLO", "RECORD ARM", "UP", "DOWN"] {
            assert!(rendered.contains(label), "missing faceplate label {label}");
        }
        for label in [
            "EFFECTS: 6 groups",
            "Gain",
            "Gate",
            "Compressor",
            "Modulation",
            "Delay",
            "Reverb",
            "FADERS F01–F08",
            "UNUSED C37–C40",
        ] {
            assert!(rendered.contains(label), "missing effects faceplate label {label}");
        }
    }

    #[test]
    fn primary_mapping_surface_keeps_multi_device_workflow_visible() {
        let backend = TestBackend::new(160, 37);
        let mut terminal = Terminal::new(backend).expect("test backend");
        let dashboard = DashboardState {
            health: "ready".into(),
            physical_devices: vec![
                PhysicalDevice {
                    name: "Launch Control XL Mk2".into(),
                    outputs: vec!["lc".into()],
                    ..Default::default()
                },
                PhysicalDevice {
                    name: "MicroPitch Pedal".into(),
                    outputs: vec!["pitch".into()],
                    ..Default::default()
                },
                PhysicalDevice {
                    name: "Lexicon Reflex".into(),
                    outputs: vec!["reflex".into()],
                    ..Default::default()
                },
            ],
            ..DashboardState::default()
        };
        let editor = RoutingEditor::from_bank(&MappingBank::new());
        terminal
            .draw(|frame| draw_controller_mapping(frame, frame.area(), &dashboard, &editor))
            .expect("draw");
        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        for label in [
            "Launch Control XL Mk2",
            "MicroPitch Pedal",
            "Lexicon Reflex",
            "SOURCE:",
            "DESTINATIONS:",
            "PARAMETERS:",
            "PANIC",
        ] {
            assert!(rendered.contains(label), "missing primary workflow element {label}");
        }
    }

    #[test]
    fn launch_control_faceplate_highlights_only_validated_live_assignment() {
        let dashboard = DashboardState {
            live_activity: Some(LiveActivity {
                kind: "control_change".into(),
                channel: Some(1),
                number: Some(17),
                ..LiveActivity::default()
            }),
            launch_control_template: Some(LaunchControlTemplate {
                template: 0,
                assignments: vec![mackes_profiles::LaunchControlAssignment {
                    index: 3,
                    channel: 1,
                    number: 17,
                    kind: LaunchControlMessageKind::Cc,
                    destination: None,
                }],
            }),
            ..DashboardState::default()
        };
        assert_eq!(launch_control_activity_index(&dashboard), Some(3));
        let mut without_template = dashboard;
        without_template.launch_control_template = None;
        assert_eq!(launch_control_activity_index(&without_template), None);
    }

    #[test]
    fn persisted_launch_control_template_converts_fail_closed() {
        let template = mackes_config::LaunchControlTemplateConfig {
            template: 2,
            assignments: vec![mackes_config::LaunchControlAssignmentConfig {
                index: 4,
                channel: 1,
                number: 21,
                kind: "cc".into(),
                destination: Some("mixer:gain".into()),
                physical_control_id: None,
                needs_review: false,
            }],
        };
        let converted = launch_control_template_from_config(&template).expect("valid template");
        assert_eq!(converted.assignment(4).map(|assignment| assignment.number), Some(21));
        let invalid = mackes_config::LaunchControlTemplateConfig {
            assignments: vec![mackes_config::LaunchControlAssignmentConfig {
                kind: "unknown".into(),
                ..template.assignments[0].clone()
            }],
            ..template
        };
        assert_eq!(launch_control_template_from_config(&invalid), None);
    }

    #[test]
    fn launch_control_template_readiness_fails_closed_for_missing_and_incomplete_layouts() {
        assert_eq!(launch_control_template_readiness(None), "MACKES TEMPLATE REQUIRED");
        let incomplete = LaunchControlTemplate { template: 0, assignments: Vec::new() };
        assert_eq!(
            launch_control_template_readiness(Some(&incomplete)),
            "MACKES TEMPLATE REQUIRED"
        );
        let wrong_slot = LaunchControlTemplate { template: 1, assignments: Vec::new() };
        assert_eq!(
            launch_control_template_readiness(Some(&wrong_slot)),
            "MACKES TEMPLATE MISMATCH"
        );
    }

    #[test]
    fn reducer_rejects_gaps_and_accepts_contiguous_events() {
        let mut state = ClientState::default();
        state.apply_snapshot(StateSnapshot { last_sequence: 4, payload: b"snapshot".to_vec() });
        state.begin_reconnect();
        assert_eq!(state.payload, b"snapshot");
        assert_eq!(state.last_sequence, 0);
        assert!(state
            .apply_event(StateEvent { sequence: 1, payload: b"reconnected".to_vec() })
            .is_ok());
        assert_eq!(state.payload, b"reconnected");
        state.apply_snapshot(StateSnapshot { last_sequence: 4, payload: b"snapshot".to_vec() });
        assert_eq!(
            state.apply_snapshot_if_newer(StateSnapshot {
                last_sequence: 3,
                payload: b"stale".to_vec()
            }),
            Err(ReducerError::SequenceGap)
        );
        assert_eq!(state.payload, b"snapshot");
        assert_eq!(
            state.apply_event(StateEvent { sequence: 6, payload: b"gap".to_vec() }),
            Err(ReducerError::SequenceGap)
        );
        assert!(state.apply_event(StateEvent { sequence: 5, payload: b"event".to_vec() }).is_ok());
        assert_eq!(state.last_sequence, 5);
    }

    #[test]
    fn reconnect_is_transactional_on_gap() {
        let mut state = ClientState { last_sequence: 9, payload: b"old".to_vec() };
        assert_eq!(
            state.apply_reconnect(
                StateSnapshot { last_sequence: 2, payload: b"new".to_vec() },
                &[StateEvent { sequence: 4, payload: b"gap".to_vec() }]
            ),
            Err(ReducerError::SequenceGap)
        );
        assert_eq!(state.last_sequence, 9);
        assert_eq!(state.payload, b"old");
    }

    #[test]
    fn reconnect_applies_snapshot_and_events() {
        let mut state = ClientState::default();
        state
            .apply_reconnect(
                StateSnapshot { last_sequence: 2, payload: b"new".to_vec() },
                &[StateEvent { sequence: 3, payload: b"event".to_vec() }],
            )
            .expect("contiguous reconnect");
        assert_eq!(state.last_sequence, 3);
        assert_eq!(state.payload, b"event");
    }

    #[test]
    fn default_keymap_exposes_safe_commands() {
        let keymap = Keymap;
        assert_eq!(keymap.command_for('n'), Some(UiCommand::NextScene));
        assert_eq!(keymap.command_for('!'), Some(UiCommand::Panic));
        assert_eq!(keymap.command_for('h'), Some(UiCommand::MoveLeft));
        assert_eq!(keymap.command_for('j'), Some(UiCommand::MoveDown));
        assert_eq!(keymap.command_for('k'), Some(UiCommand::MoveUp));
        assert_eq!(keymap.command_for('l'), Some(UiCommand::MoveRight));
        assert_eq!(keymap.command_for('3'), Some(UiCommand::OpenWorkspace(3)));
        assert_eq!(workspace_name(1), Some("Dashboard"));
        assert_eq!(workspace_name(5), Some("Routing"));
        assert_eq!(workspace_name(6), Some("Diagnostics"));
        assert_eq!(workspace_name(9), Some("Setlists"));
        assert_eq!(workspace_name(10), None);
        assert_eq!(keymap.description('h'), Some("Left"));
        assert_eq!(keymap.description('!'), Some("Panic"));
        assert_eq!(keymap.description('x'), None);
        assert_eq!(keymap.command_for('x'), None);
    }

    #[test]
    fn task_shell_has_five_stable_sections_and_exact_breadcrumbs() {
        assert_eq!(
            AppSection::all().map(AppSection::label),
            ["Live", "Map Controls", "Scenes", "Devices", "System"]
        );
        assert_eq!(AppSection::MapControls.purpose(), "Assign physical controls to parameters");
        let focus = FocusPath::new(AppSection::MapControls, "Source control").expect("focus");
        assert_eq!(focus.breadcrumb(), "Map Controls / Source control");
        assert!(FocusPath::new(AppSection::Live, "").is_err());
    }

    #[test]
    fn task_shell_navigation_is_bounded_and_resets_target_on_section_change() {
        let mut shell = TaskShellState::initial(2).expect("shell");
        shell.apply(ShellAction::Down);
        shell.apply(ShellAction::Down);
        assert_eq!(shell.target_index, 1);
        assert_eq!(shell.focus.target, "Target 2");
        shell.apply(ShellAction::Right);
        assert_eq!(shell.focus.section, AppSection::MapControls);
        assert_eq!(shell.target_index, 0);
        shell.apply(ShellAction::Left);
        shell.apply(ShellAction::Up);
        assert_eq!(shell.focus.section, AppSection::Live);
        assert_eq!(shell.target_index, 0);
    }

    #[test]
    fn task_shell_primary_keys_and_vim_aliases_share_actions() {
        assert_eq!(shell_action_for_key(ShellKey::Up), Some(ShellAction::Up));
        assert_eq!(shell_action_for_key(ShellKey::Enter), Some(ShellAction::Enter));
        assert_eq!(shell_action_for_key(ShellKey::Esc), Some(ShellAction::Back));
        assert_eq!(shell_action_for_char('h'), Some(ShellAction::Left));
        assert_eq!(shell_action_for_char('l'), Some(ShellAction::Right));
        assert_eq!(shell_action_for_key(ShellKey::Help), Some(ShellAction::Help));
    }

    #[test]
    fn ui_commands_project_only_to_governed_ipc_operations() {
        assert_eq!(ipc_command_for(UiCommand::Panic), Some(mackes_ipc::Command::Panic));
        assert_eq!(ipc_command_for(UiCommand::NextScene), Some(mackes_ipc::Command::Scenes));
        assert_eq!(ipc_command_for(UiCommand::OpenWorkspace(3)), None);
    }

    #[test]
    fn mapping_filter_draft_validates_and_previews_engine_predicates() {
        let draft = MappingFilterDraft {
            predicates: vec![mackes_midi_engine::RoutePredicate::NumberRange {
                minimum: 10,
                maximum: 20,
            }],
        };
        assert!(draft.validate().is_ok());
        assert_eq!(draft.preview(), draft.predicates);
        let invalid = MappingFilterDraft {
            predicates: vec![mackes_midi_engine::RoutePredicate::ValueRange {
                minimum: 20,
                maximum: 10,
            }],
        };
        assert_eq!(invalid.validate(), Err("invalid MIDI value range"));
        let persisted = vec![
            mackes_config::LearnedFilter::NumberRange { minimum: 1, maximum: 2 },
            mackes_config::LearnedFilter::ValueRange { minimum: 3, maximum: 4 },
            mackes_config::LearnedFilter::Realtime {
                message: mackes_config::LearnedRealtime::Reset,
            },
            mackes_config::LearnedFilter::SysExMask { pattern: vec![1], mask: vec![127] },
        ];
        assert_eq!(predicates_from_learned_filters(&persisted).expect("convert").len(), 4);
    }

    #[test]
    fn mapped_midi_dashboard_actions_require_one_explicit_match() {
        let note = mackes_domain::MidiMessage::NoteOn {
            channel: mackes_domain::MidiChannel::new(1).expect("channel"),
            note: mackes_domain::SevenBit::new(36).expect("note"),
            velocity: mackes_domain::SevenBit::new(127).expect("velocity"),
        };
        let binding = DashboardMidiBinding {
            trigger: DashboardMidiTrigger::NoteOn { channel: 1, note: 36 },
            command: UiCommand::Panic,
        };
        assert_eq!(ui_command_for_midi(&note, &[binding]), Some(UiCommand::Panic));
        assert_eq!(ui_command_for_midi(&note, &[]), None);
        assert_eq!(
            ui_command_for_midi(&note, &[binding, binding]),
            None,
            "ambiguous mappings must not dispatch"
        );
    }

    #[test]
    fn persisted_dashboard_bindings_convert_to_runtime_commands() {
        let bindings = vec![mackes_config::DashboardMidiBinding {
            trigger: mackes_config::DashboardMidiTrigger::ControlChange {
                channel: 1,
                controller: 20,
                value: None,
            },
            command: "next_scene".into(),
        }];
        let runtime = dashboard_bindings_from_config(&bindings).expect("runtime bindings");
        assert_eq!(runtime[0].command, UiCommand::NextScene);
        assert!(dashboard_bindings_from_config(&[mackes_config::DashboardMidiBinding {
            trigger: mackes_config::DashboardMidiTrigger::NoteOn { channel: 0, note: 1 },
            command: "panic".into(),
        }])
        .is_err());
    }

    #[test]
    fn dashboard_action_polling_is_bounded_and_observational() {
        struct FakeInput {
            info: mackes_midi_engine::EndpointInfo,
            events: std::collections::VecDeque<mackes_domain::MidiEvent>,
        }
        impl mackes_midi_engine::MidiInputAdapter for FakeInput {
            fn info(&self) -> &mackes_midi_engine::EndpointInfo {
                &self.info
            }
            fn receive(&mut self) -> Option<mackes_domain::MidiEvent> {
                self.events.pop_front()
            }
        }
        let event = mackes_domain::MidiEvent {
            timestamp: mackes_domain::TimestampNanos::new(1),
            sequence: 1,
            endpoint: mackes_domain::EndpointId::new(1).expect("endpoint"),
            message: mackes_domain::MidiMessage::NoteOn {
                channel: mackes_domain::MidiChannel::new(1).expect("channel"),
                note: mackes_domain::SevenBit::new(36).expect("note"),
                velocity: mackes_domain::SevenBit::new(127).expect("velocity"),
            },
        };
        let mut input = FakeInput {
            info: mackes_midi_engine::EndpointInfo {
                id: "learn-input".into(),
                name: "test input".into(),
                direction: mackes_midi_engine::EndpointDirection::Input,
            },
            events: std::collections::VecDeque::from([event.clone(), event]),
        };
        let binding = DashboardMidiBinding {
            trigger: DashboardMidiTrigger::NoteOn { channel: 1, note: 36 },
            command: UiCommand::NextScene,
        };
        assert_eq!(poll_dashboard_actions(&mut input, &[binding], 1), vec![UiCommand::NextScene]);
        assert_eq!(input.events.len(), 1);
    }

    #[test]
    fn signal_flow_diagram_preserves_order_and_rejects_empty_nodes() {
        let diagram = SignalFlowDiagram::new(
            vec![SignalFlowNode {
                id: "reflex".into(),
                label: "Lexicon Reflex".into(),
                online: true,
            }],
            3,
        )
        .expect("diagram");
        assert_eq!(diagram.generation, 3);
        assert_eq!(diagram.nodes[0].label, "Lexicon Reflex");
        assert!(SignalFlowDiagram::new(
            vec![SignalFlowNode { id: String::new(), label: "x".into(), online: false }],
            1
        )
        .is_err());
    }

    #[test]
    fn blueprint_renderer_is_deterministic_and_labels_inferred_topology() {
        let diagram = SignalFlowDiagram::new(
            vec![
                SignalFlowNode { id: "input".into(), label: "Input".into(), online: true },
                SignalFlowNode { id: "cabinet".into(), label: "Cabinet".into(), online: false },
            ],
            3,
        )
        .expect("diagram");
        let lines = blueprint_lines(&diagram, true);
        assert_eq!(lines[0], "Inferred logical/control view — not authoritative DSP topology");
        assert_eq!(lines[1], "+--[Input]--+");
        assert!(lines[2].contains("input online"));
        assert_eq!(lines[3], "      |      v");
        assert!(lines[5].contains("cabinet offline"));
    }

    #[test]
    fn viewport_clamps_and_selects_compact_layout() {
        assert_eq!(Viewport::new(0, 0), Viewport { width: 1, height: 1 });
        assert!(!Viewport::new(80, 24).compact());
        assert!(Viewport::new(79, 24).compact());
        assert!(dashboard_panels(false).contains(&DashboardPanel::RecentEvents));
        assert!(!dashboard_panels(true).contains(&DashboardPanel::SignalFlow));
        assert!(dashboard_panels(true).contains(&DashboardPanel::Panic));
    }

    #[test]
    fn page_navigation_is_bounded_and_wraps() {
        let mut page = PageState::new(9, 3);
        assert_eq!(page.current, 2);
        page.next();
        assert_eq!(page.current, 0);
        page.previous();
        assert_eq!(page.current, 2);
        page.select(99);
        assert_eq!(page.current, 2);
    }

    #[test]
    fn terminal_guard_restores_idempotently() {
        let mut guard = TerminalGuard::default();
        assert!(!guard.needs_restore());
        guard.acquire();
        assert!(guard.needs_restore());
        guard.restore();
        guard.restore();
        assert!(!guard.needs_restore());
    }

    #[test]
    fn dashboard_initial_state_keeps_panic_available() {
        let dashboard = DashboardState::initial();
        assert_eq!(dashboard.health, "starting");
        assert!(dashboard.panic_available);
        assert!(dashboard.frame_lines().iter().any(|line| line.contains("PANIC: available")));
        assert!(dashboard
            .frame_lines_for(Viewport::new(20, 10))
            .iter()
            .all(|line| line.len() <= 20));
        assert!(dashboard
            .frame_lines_for(Viewport::new(6, 10))
            .iter()
            .any(|line| line == "PANIC "));
        assert!(dashboard.frame_lines().iter().any(|line| line.contains("keys:")));
        assert!(dashboard
            .frame_lines_for(Viewport::new(20, 10))
            .iter()
            .any(|line| line.starts_with("keys: 1-9")));
        assert!(!dashboard.performance_locked);
        assert_eq!(dashboard.activation_progress, (0, 0));
        let mut dashboard = dashboard;
        dashboard.set_activity(10, 8, 1);
        dashboard.set_activation_progress(9, 4);
        dashboard.set_activation_progress(2, 4);
        assert_eq!((dashboard.received, dashboard.sent, dashboard.dropped), (10, 8, 1));
        assert_eq!(dashboard.activation_progress, (4, 4));
    }

    #[test]
    fn dashboard_event_projection_updates_widgets_and_keeps_panic() {
        let mut dashboard = DashboardState::initial();
        dashboard.apply_event(DashboardEvent::Health("ready".into()));
        dashboard.apply_event(DashboardEvent::ActiveScene(Some("intro".into())));
        dashboard.apply_event(DashboardEvent::PerformanceLock(true));
        dashboard.apply_event(DashboardEvent::ActivationProgress { completed: 2, total: 3 });
        dashboard.apply_event(DashboardEvent::ActivationResult("partial: 1 failed".into()));
        dashboard
            .apply_event(DashboardEvent::DeviceHealth(vec![("processor".into(), "ready".into())]));
        dashboard.apply_event(DashboardEvent::Notification {
            severity: SemanticToken::Warning,
            message: "endpoint degraded".into(),
        });
        assert_eq!(dashboard.health, "ready");
        assert_eq!(dashboard.active_scene.as_deref(), Some("intro"));
        assert!(dashboard.performance_locked);
        assert_eq!(dashboard.activation_progress, (2, 3));
        assert_eq!(dashboard.activation_result.as_deref(), Some("partial: 1 failed"));
        assert_eq!(dashboard.device_health, vec![("processor".into(), "ready".into())]);
        assert!(dashboard.frame_lines().iter().any(|line| line == "device=processor health=ready"));
        assert!(dashboard.frame_lines().iter().any(|line| line == "[WARN] endpoint degraded"));
        dashboard.set_device_health(
            (0..33).map(|index| (format!("d{index}"), "ready".into())).collect(),
        );
        assert_eq!(dashboard.device_health.len(), 32);
        assert!(dashboard.panic_available);
    }

    #[test]
    fn live_activity_age_advances_and_resets_on_new_activity() {
        let mut dashboard = DashboardState::initial();
        dashboard.advance_live_activity_age(1_500_000_000);
        assert_eq!(dashboard.live_activity_age_nanos, 1_500_000_000);
        dashboard.apply_event(DashboardEvent::LiveActivity(LiveActivity {
            control_id: "endpoint:1:cc:7".into(),
            ..LiveActivity::default()
        }));
        assert_eq!(dashboard.live_activity_age_nanos, 0);
        dashboard.advance_live_activity_age(u64::MAX);
        assert_eq!(dashboard.live_activity_age_nanos, u64::MAX);
    }

    #[test]
    fn live_activity_age_label_decays_after_one_second() {
        assert_eq!(live_activity_age_label(0), "ACTIVE");
        assert_eq!(live_activity_age_label(999_999_999), "ACTIVE");
        assert_eq!(live_activity_age_label(1_000_000_000), "STALE");
    }

    #[test]
    fn ready_health_is_rendered_as_online() {
        assert!(health_is_online("ready"));
        assert!(health_is_online("online"));
        assert!(!health_is_online("degraded"));
    }

    #[test]
    fn dashboard_notifications_are_newest_first_and_bounded() {
        let mut dashboard = DashboardState::initial();
        dashboard.notify(SemanticToken::Warning, "peer reconnecting");
        dashboard.notify(SemanticToken::Success, "scene active");
        assert_eq!(dashboard.notifications[0].message, "scene active");
        assert!(dashboard.frame_lines().iter().any(|line| line == "[WARN] peer reconnecting"));
        for index in 0..20 {
            dashboard.notify(SemanticToken::Midi, format!("event-{index}"));
        }
        assert_eq!(dashboard.notifications.len(), 16);
        assert_eq!(dashboard.notifications[0].message, "event-19");
        assert!(!dashboard
            .notifications
            .iter()
            .any(|notice| notice.message == "peer reconnecting"));
    }

    #[test]
    fn dashboard_payload_projection_decodes_authoritative_fields() {
        let payload = serde_json::json!({
            "health": "ready",
            "active_scene": "intro",
            "route_generation": 7,
            "route_undo_available": true,
            "audit_count": 3,
            "audit": [{"action": "route_replace", "allowed": true}],
            "received": 10,
            "sent": 8,
            "dropped": 2,
            "activation_result": "total=2 succeeded=2 failed=0",
            "physical_devices": [{
                "id": "launch control xl",
                "name": "Launch Control XL",
                "inputs": ["input-1"],
                "outputs": ["output-1"],
                "state": "connected"
            }],
            "last_activity": {
                "source_endpoint": 11,
                "source_endpoint_id": "midir-in-source",
                "control_id": "endpoint:11:control_change:21",
                "timestamp_nanos": 123_456,
                "kind": "control_change",
                "number": 21,
                "value": 96,
                "destination_endpoints": [22, 23],
                "sequence": 44
            }
        });
        let mut dashboard = DashboardState::initial();
        for event in DashboardEvent::from_payload(&payload) {
            dashboard.apply_event(event);
        }
        assert_eq!(dashboard.health, "ready");
        assert_eq!(dashboard.active_scene.as_deref(), Some("intro"));
        assert_eq!(dashboard.route_generation, 7);
        assert_eq!((dashboard.received, dashboard.sent, dashboard.dropped), (10, 8, 2));
        assert_eq!(dashboard.activation_result.as_deref(), Some("total=2 succeeded=2 failed=0"));
        assert_eq!(dashboard.physical_devices.len(), 1);
        assert_eq!(dashboard.physical_devices[0].name, "Launch Control XL");
        assert_eq!(dashboard.physical_devices[0].inputs, vec!["input-1"]);
        assert_eq!(dashboard.physical_devices[0].outputs, vec!["output-1"]);
        assert!(dashboard.route_undo_available);
        assert_eq!(dashboard.audit_count, 3);
        assert_eq!(dashboard.latest_audit.as_deref(), Some("ALLOW route_replace"));
        let activity = dashboard.live_activity.expect("activity");
        assert_eq!(activity.source_endpoint, 11);
        assert_eq!(activity.source_endpoint_id.as_deref(), Some("midir-in-source"));
        assert_eq!(activity.control_id, "endpoint:11:control_change:21");
        assert_eq!(activity.timestamp_nanos, 123_456);
        assert_eq!(activity.kind, "control_change");
        assert_eq!(activity.number, Some(21));
        assert_eq!(activity.value, Some(96));
        assert_eq!(activity.destination_endpoints, vec![22, 23]);
        assert_eq!(activity.sequence, 44);
        assert!(DashboardEvent::from_payload(&serde_json::json!({
            "last_activity": {"kind": "invalid"}
        }))
        .iter()
        .all(|event| !matches!(event, DashboardEvent::LiveActivity(_))));
    }

    #[test]
    fn mapping_chain_line_exposes_enabled_state_and_destination() {
        let draft = MappingDraft {
            source: "Launch Control XL / K01".into(),
            destination: "MicroPitch / Mix".into(),
            destination_parameter: None,
            enabled: true,
            ..MappingDraft::default()
        };
        let line = mapping_chain_line(&[draft]);
        assert!(line.contains("● Launch Control XL / K01→MicroPitch / Mix"));
        assert_eq!(mapping_chain_line(&[]), "NONE — press a to add a source → destination route");
    }

    #[test]
    fn live_value_bar_is_bounded_and_monotonic() {
        assert_eq!(value_bar(0), "[----------------]");
        assert_eq!(value_bar(127), "[################]");
        assert!(value_bar(96).contains("############"));
        assert_eq!(value_bar(u16::MAX).len(), value_bar(127).len());
    }

    #[test]
    fn destination_inventory_line_is_bounded_and_explicit() {
        let devices = vec![PhysicalDevice {
            name: "MicroPitch Pedal".into(),
            inputs: vec!["midir-in-pedal".into(), "midir-in-pedal-2".into()],
            outputs: vec!["midir-out-pedal".into()],
            ..PhysicalDevice::default()
        }];
        assert_eq!(destination_inventory_line(&devices, None), " MicroPitch Pedal:midir-out-pedal");
        assert_eq!(
            destination_inventory_line(&devices, Some(0)),
            ">MicroPitch Pedal:midir-out-pedal"
        );
        assert_eq!(destination_inventory_line(&[], None), "NO OUTPUT DESTINATIONS");
        let mut dashboard = DashboardState::initial();
        dashboard.physical_devices = devices;
        dashboard.cycle_destination();
        assert_eq!(dashboard.selected_destination, Some(0));
        assert!(dashboard.first_input_endpoint().is_some());
        dashboard.cycle_input();
        assert_eq!(dashboard.selected_input, Some(0));
        assert!(dashboard.selected_input_endpoint().is_some());
        assert!(dashboard.selected_destination_endpoint().is_some());
        assert!(
            destination_parameter_line(&dashboard.physical_devices, Some(0), None).contains("Mix")
        );
        dashboard.cycle_parameter();
        assert!(destination_parameter_line(
            &dashboard.physical_devices,
            Some(0),
            dashboard.selected_parameter
        )
        .contains('>'));
    }

    #[test]
    fn destination_browser_filters_unknown_devices_and_cycles_supported_profiles() {
        let names = ["Reflex"];
        for name in names {
            let device = PhysicalDevice {
                name: name.into(),
                outputs: vec!["out".into()],
                ..Default::default()
            };
            let line = destination_parameter_line(&[device], Some(0), None);
            assert!(!line.contains("UNVERIFIED"), "{name}: {line}");
            assert!(!line.is_empty());
        }
        let unknown = PhysicalDevice {
            name: "Unrecognized Processor".into(),
            outputs: vec!["out".into()],
            ..Default::default()
        };
        assert_eq!(
            destination_parameter_line(&[unknown], Some(0), None),
            "PROFILE PARAMETERS UNVERIFIED"
        );
    }

    #[test]
    fn semantic_tokens_have_readable_contrast_contract() {
        assert!(contrast_is_readable((255, 255, 255), (0, 0, 0)));
        assert!(!contrast_is_readable((120, 120, 120), (128, 128, 128)));
        assert_eq!(SemanticToken::Hazard, SemanticToken::Hazard);
        assert_eq!(token_marker(SemanticToken::Setup), "[SETUP]");
        assert_eq!(token_marker(SemanticToken::Midi), "[MIDI]");
        assert_eq!(token_marker(SemanticToken::Sysex), "[SYSEX]");
        assert_eq!(token_marker(SemanticToken::Success), "[OK]");
        assert_eq!(token_marker(SemanticToken::Warning), "[WARN]");
        assert_eq!(token_marker(SemanticToken::Error), "[ERROR]");
        assert_eq!(token_marker(SemanticToken::Hazard), "[HAZARD]");
        assert_eq!(intensity_marker(TokenIntensity::Dim), "(dim)");
        assert_eq!(intensity_marker(TokenIntensity::Normal), "");
        assert_eq!(intensity_marker(TokenIntensity::Selected), "(selected)");
        assert_eq!(intensity_marker(TokenIntensity::Hazard), "(hazard)");
        let palette = [PaletteEntry {
            token: SemanticToken::Hazard,
            foreground: (255, 255, 255),
            background: (0, 0, 0),
        }];
        assert!(validate_palette(&palette).is_ok());
        assert!(validate_palette(&[
            palette[0],
            PaletteEntry {
                token: SemanticToken::Hazard,
                foreground: (255, 255, 255),
                background: (0, 0, 0)
            },
        ])
        .is_err());
        assert!(validate_palette(&[PaletteEntry {
            token: SemanticToken::Error,
            foreground: (120, 120, 120),
            background: (128, 128, 128)
        }])
        .is_err());
        assert_eq!(default_palette().len(), 7);
        assert!(validate_palette(&default_palette()).is_ok());
        assert_eq!(
            palette_entry(&default_palette(), SemanticToken::Midi).map(|entry| entry.foreground),
            Some((255, 255, 255))
        );
        assert!(palette_entry(&palette, SemanticToken::Midi).is_none());
        let theme = Theme { version: 1, palette: default_palette().to_vec() };
        assert!(theme.validate().is_ok());
        assert_eq!(
            Theme { version: 1, palette: theme.palette[..6].to_vec() }.validate(),
            Err("theme must cover every semantic token")
        );
        assert_eq!(
            Theme { version: 0, palette: default_palette().to_vec() }.validate(),
            Err("theme version must be nonzero")
        );
    }

    #[test]
    fn learn_requires_explicit_candidate_and_enter_commit_path() {
        let mut learn = LearnWorkspace::new();
        assert!(learn.set_input_alias("launch-control").is_ok());
        learn.start_capture();
        learn.finish_capture(vec![MidiLearnCandidate {
            kind: mackes_midi_engine::LearnMessageKind::ControlChange,
            channel: Some(1),
            number: Some(7),
            observations: 3,
            minimum: Some(0),
            maximum: Some(127),
            raw: vec![0xB0, 7, 127],
        }]);
        assert_eq!(learn.phase, LearnPhase::Review);
        learn.select(99);
        assert_eq!(learn.phase, LearnPhase::Review);
        learn.select(0);
        learn.handle_key(LearnKey::Other);
        assert_eq!(learn.phase, LearnPhase::Destination);
        learn.handle_key(LearnKey::Enter);
        assert_eq!(learn.phase, LearnPhase::Destination);
        assert!(learn.set_destination("pedal.mix", MappingMode::Note).is_err());
        assert!(learn.set_destination("pedal.mix", MappingMode::Cc).is_ok());
        assert!(learn.set_channel_policy(LearnChannelPolicy::Any).is_ok());
        learn.begin_live_test();
        assert_eq!(learn.phase, LearnPhase::Testing);
        assert!(learn
            .frame_lines(Viewport::new(80, 24))
            .iter()
            .any(|line| line.contains("awaiting daemon/device result")));
        learn.finish_live_test(false);
        learn.handle_key(LearnKey::Enter);
        assert_eq!(learn.phase, LearnPhase::Destination);
        learn.begin_live_test();
        learn.finish_live_test(true);
        learn.handle_key(LearnKey::Enter);
        assert_eq!(learn.phase, LearnPhase::Committed);
        let mapping = learn.committed_mapping().expect("committed mapping");
        assert_eq!(mapping.source, "launch-control");
        assert_eq!(mapping.destination, "pedal.mix");
        assert_eq!(mapping.mode, MappingMode::Cc);
        assert_eq!(mapping.channel, None);
        let durable = learn.committed_learned_mapping().expect("durable mapping");
        assert_eq!(durable.message_kind, "control_change");
        assert_eq!(durable.number, Some(7));
        assert_eq!(durable.raw, vec![0xB0, 7, 127]);
        assert_eq!(durable.channel_policy, mackes_config::LearnedChannelPolicy::Any);
        let config = mackes_config::ConfigDocument {
            schema_version: mackes_config::CURRENT_SCHEMA_VERSION,
            endpoints: vec![mackes_config::EndpointAlias {
                id: "launch-control".into(),
                name: None,
                vendor_id: None,
                product_id: None,
                serial: None,
            }],
            ..mackes_config::ConfigDocument::default()
        };
        let persisted = learn.apply_to_config(&config).expect("persisted projection");
        assert_eq!(persisted.settings.learn_input_alias.as_deref(), Some("launch-control"));
        assert_eq!(persisted.learned_mappings, vec![durable]);
        assert!(learn.apply_to_config(&persisted).is_err());
        let frame = learn.frame_lines(Viewport::new(80, 24));
        assert!(frame.iter().any(|line| line.contains("raw=B0 07 7F")));
        assert!(frame.iter().any(|line| line.contains("destination=pedal.mix")));
        learn.handle_key(LearnKey::Escape);
        assert_eq!(learn.phase, LearnPhase::Cancelled);
    }

    #[test]
    fn learn_refuses_capture_without_global_input_alias() {
        let mut learn = LearnWorkspace::new();
        learn.start_capture();
        assert_eq!(learn.phase, LearnPhase::Armed);
        assert!(learn.set_input_alias("").is_err());
        assert!(learn.set_input_alias("midi-in").is_ok());
        learn.start_capture();
        assert_eq!(learn.phase, LearnPhase::Capturing);
        assert!(learn.set_input_alias("other").is_err());
    }

    #[test]
    fn learn_rollback_clears_transaction_but_retains_input_alias() {
        let mut learn = LearnWorkspace::new();
        learn.set_input_alias("launch-control").expect("alias");
        learn.start_capture();
        learn.finish_capture(Vec::new());
        learn.rollback();
        assert_eq!(learn.phase, LearnPhase::Armed);
        assert!(learn.candidates.is_empty());
        assert_eq!(learn.learn_input_alias.as_deref(), Some("launch-control"));
        assert!(learn.committed_mapping().is_none());
    }

    #[test]
    fn mapping_draft_validates_without_partial_mutation() {
        let draft = MappingDraft {
            source: "launchpad".into(),
            destination: "reflex".into(),
            destination_parameter: None,
            channel: Some(1),
            enabled: true,
            mode: MappingMode::Cc,
            priority: 0,
            curve: mackes_midi_engine::Curve::Linear,
            filters: MappingFilterDraft::default(),
            allow_cycle: false,
        };
        assert!(draft.validate().is_ok());
        assert_eq!(draft.curve, mackes_midi_engine::Curve::Linear);
        let curved = MappingDraft { curve: mackes_midi_engine::Curve::Square, ..draft.clone() };
        assert!(curved.validate().is_ok());
        let filtered = MappingDraft {
            filters: MappingFilterDraft {
                predicates: vec![mackes_midi_engine::RoutePredicate::NumberRange {
                    minimum: 10,
                    maximum: 20,
                }],
            },
            ..draft.clone()
        };
        assert!(filtered.validate().is_ok());
        let invalid = MappingDraft { channel: Some(17), ..draft.clone() };
        assert!(invalid.validate().is_err());
        let duplicate = MappingDraft { priority: 1, ..draft.clone() };
        assert!(draft.conflicts_with(&duplicate));
        assert!(validate_mapping_batch(&[draft.clone(), duplicate]).is_err());
        assert!(reorder_mapping_drafts(std::slice::from_ref(&draft), &[0]).is_ok());
        assert!(reorder_mapping_drafts(std::slice::from_ref(&draft), &[1]).is_err());
        let reordered = order_mapping_drafts(&[
            MappingDraft { priority: 20, ..draft.clone() },
            MappingDraft { priority: 10, ..draft.clone() },
            MappingDraft { priority: 10, ..draft.clone() },
        ]);
        assert_eq!(
            reordered.iter().map(|draft| draft.priority).collect::<Vec<_>>(),
            vec![10, 10, 20]
        );
        let mut bank = MappingBank::new();
        assert_eq!(bank.commit(vec![draft.clone()]), Ok(1));
        assert_eq!(bank.drafts(), std::slice::from_ref(&draft));
        assert_eq!(
            bank.commit_if_generation(0, vec![draft.clone()]),
            Err("mapping generation changed concurrently".into())
        );
        assert_eq!(bank.generation(), 1);
        assert_eq!(bank.commit_if_generation(1, vec![draft.clone()]), Ok(2));
        let invalid = MappingDraft { source: String::new(), ..draft.clone() };
        assert!(bank.commit(vec![invalid]).is_err());
        assert_eq!(bank.generation(), 2);
        assert!(bank.undo_available());
        assert_eq!(bank.undo(), Ok(3));
        assert_eq!(bank.drafts(), std::slice::from_ref(&draft));
        let mut editor = RoutingEditor::from_bank(&bank);
        assert!(editor.reorder(&[0]).is_ok());
        editor.selected = Some(0);
        editor.cycle_selected_curve().expect("curve");
        assert_eq!(editor.drafts[0].curve, mackes_midi_engine::Curve::Square);
        editor.cycle_selected_filter().expect("number filter");
        assert_eq!(editor.drafts[0].filters.predicates.len(), 1);
        editor.cycle_selected_filter().expect("value filter");
        editor.cycle_selected_filter().expect("realtime filter");
        editor.cycle_selected_filter().expect("sysex filter");
        editor.cycle_selected_filter().expect("clear filter");
        assert!(editor.drafts[0].filters.predicates.is_empty());
        assert_eq!(editor.remove(4), Err("mapping index is out of range"));
    }

    #[test]
    fn routing_editor_cycles_channels_transactionally() {
        let draft = MappingDraft {
            source: "1".into(),
            destination: "2".into(),
            destination_parameter: None,
            channel: None,
            enabled: true,
            mode: MappingMode::Cc,
            priority: 0,
            curve: mackes_midi_engine::Curve::Linear,
            filters: MappingFilterDraft::default(),
            allow_cycle: false,
        };
        let mut editor = RoutingEditor::from_bank(&MappingBank::new());
        editor.add(draft).expect("draft");
        assert_eq!(editor.drafts[0].channel, None);
        editor.cycle_selected_channel().expect("channel 1");
        assert_eq!(editor.drafts[0].channel, Some(1));
        for _ in 0..15 {
            editor.cycle_selected_channel().expect("channel");
        }
        assert_eq!(editor.drafts[0].channel, Some(16));
        editor.cycle_selected_channel().expect("any channel");
        assert_eq!(editor.drafts[0].channel, None);
        editor.toggle_selected_enabled().expect("disable");
        assert!(!editor.drafts[0].enabled);
        editor.adjust_selected_priority(7).expect("raise priority");
        assert_eq!(editor.drafts[0].priority, 7);
        editor.adjust_selected_priority(-3).expect("lower priority");
        assert_eq!(editor.drafts[0].priority, 4);
        editor.adjust_selected_priority(-i32::from(u16::MAX)).expect("saturate priority");
        assert_eq!(editor.drafts[0].priority, 0);
    }

    #[test]
    fn backup_workspace_separates_plan_confirmation_and_result() {
        let mut workspace = BackupWorkspace::new();
        workspace.plan("nothing selected");
        assert_eq!(workspace.phase, BackupPhase::Listing);
        workspace.inspect("reflex.dump");
        workspace.set_identity_warning(true);
        workspace.plan("compatible");
        assert!(workspace.identity_warning);
        workspace.cancel();
        assert_eq!(workspace.phase, BackupPhase::Listing);
        workspace.inspect("reflex.dump");
        workspace.plan("compatible");
        workspace.confirm();
        workspace.begin_apply();
        workspace.finish(BackupPhase::Verified, "read-back matched");
        assert_eq!(workspace.phase, BackupPhase::Verified);
        assert_eq!(
            BackupPhase::from(mackes_config::BackupStatus::SentUnverified),
            BackupPhase::SentUnverified
        );
        assert_eq!(BackupPhase::from(mackes_config::BackupStatus::Failed), BackupPhase::Failed);
        workspace.cancel();
        assert_eq!(workspace.phase, BackupPhase::Verified);
    }

    #[test]
    fn device_operation_preview_exposes_destination_and_risk_before_send() {
        let query = DeviceOperationPreview::new(
            "reflex",
            "active setup query",
            DeviceOperationRisk::ReadOnly,
        )
        .expect("query preview");
        assert!(!query.requires_confirmation());
        let write = DeviceOperationPreview::new(
            "reflex",
            "store register",
            DeviceOperationRisk::PersistentWrite,
        )
        .expect("write preview");
        assert!(write.requires_confirmation());
        assert_eq!(
            DeviceOperationPreview::new("", "query", DeviceOperationRisk::ReadOnly),
            Err("device operation preview fields must not be empty")
        );
    }

    #[test]
    fn monitor_is_bounded_newest_first_and_filterable() {
        let mut monitor = MonitorState::new(2).expect("capacity");
        monitor.push(MonitorEntry { severity: MonitorSeverity::Info, message: "a".into() });
        monitor.push(MonitorEntry { severity: MonitorSeverity::Error, message: "b".into() });
        monitor.push(MonitorEntry { severity: MonitorSeverity::Warning, message: "c".into() });
        assert_eq!(monitor.entries[0].message, "c");
        assert_eq!(monitor.entries.len(), 2);
        assert_eq!(monitor.filtered(MonitorSeverity::Warning).len(), 2);
        monitor.set_paused(true);
        monitor.push(MonitorEntry { severity: MonitorSeverity::Error, message: "ignored".into() });
        assert_eq!(monitor.entries[0].message, "c");
        assert_eq!(monitor.export().len(), 2);
        assert_eq!(monitor.export_redacted(&[0])[0].message, "<redacted>");
        assert_eq!(monitor.export_redacted(&[0])[1].message, "b");
        monitor.set_paused(false);
        monitor.push(MonitorEntry { severity: MonitorSeverity::Error, message: "d".into() });
        assert_eq!(monitor.entries[0].message, "d");
        assert!(monitor.frame_lines(Viewport::new(30, 3))[0].contains("monitor=live"));
    }

    #[test]
    fn diagnostics_explain_degraded_health_with_bounded_lines() {
        let mut diagnostics = DiagnosticsState::new();
        diagnostics.push(HealthDiagnostic {
            subject: "reflex".into(),
            severity: MonitorSeverity::Warning,
            reason: "endpoint offline".into(),
            remediation: "reconnect MIDI cable".into(),
        });
        assert_eq!(
            diagnostics.lines(),
            vec!["Warning reflex: endpoint offline -> reconnect MIDI cable".to_owned()]
        );
        for index in 0..40 {
            diagnostics.push(HealthDiagnostic {
                subject: format!("d{index}"),
                severity: MonitorSeverity::Info,
                reason: "ready".into(),
                remediation: "none".into(),
            });
        }
        assert_eq!(diagnostics.entries.len(), DiagnosticsState::CAPACITY);
        assert_eq!(diagnostics.entries[0].subject, "d39");
        assert!(diagnostics
            .frame_lines(Viewport::new(40, 4))
            .iter()
            .all(|line| line.chars().count() <= 40));
    }

    #[test]
    fn setlist_editor_keeps_edits_transactional_and_validated() {
        let source =
            mackes_config::Setlist { id: "live".into(), projects: vec!["a".into(), "b".into()] };
        let mut editor = SetlistEditor::from_snapshot(std::slice::from_ref(&source));
        assert!(editor.reorder_selected(&["b", "a"]).is_err());
        editor.select(0).expect("selection");
        editor.reorder_selected(&["b", "a"]).expect("reorder");
        assert!(editor.append_project("c").is_ok());
        assert!(editor.append_project("c").is_err());
        assert_eq!(editor.remove_last_project(), Ok("c".into()));
        assert!(editor.append_project("c").is_ok());
        editor.move_last_project(false).expect("rotate left");
        assert_eq!(editor.drafts[0].projects, vec!["a", "c", "b"]);
        editor.move_last_project(true).expect("rotate right");
        assert_eq!(editor.drafts[0].projects, vec!["b", "a", "c"]);
        editor.copy_selected("encore").expect("copy");
        assert_eq!(editor.add_empty(), "new-setlist-1");
        assert_eq!(editor.add_empty(), "new-setlist-2");
        assert_eq!(editor.drafts[0].projects, vec!["b", "a", "c"]);
        assert_eq!(source.projects, vec!["a", "b"]);
        assert!(editor.frame_lines(Viewport::new(80, 24))[0].contains("uncommitted"));
        assert_eq!(editor.commit().len(), 4);
    }

    #[test]
    fn device_control_availability_never_treats_unverified_state_as_actionable() {
        assert!(control_is_actionable(ControlAvailability::Available));
        assert!(control_is_actionable(ControlAvailability::Hazardous));
        assert!(!control_is_actionable(ControlAvailability::ReadOnly));
        assert!(!control_is_actionable(ControlAvailability::Unavailable));
    }

    #[test]
    fn reflex_workspace_preserves_manual_labels_and_highlights_controls() {
        let diagram = SignalFlowDiagram::new(
            vec![
                SignalFlowNode { id: "input".into(), label: "Input".into(), online: true },
                SignalFlowNode { id: "algo".into(), label: "Algorithm".into(), online: true },
            ],
            4,
        )
        .expect("diagram");
        let mut workspace = ReflexWorkspace::new(
            "algorithm-1".into(),
            "Documented Algorithm".into(),
            vec![ReflexControl {
                id: "midi-channel".into(),
                label: "MIDI Channel".into(),
                node_id: "input".into(),
                available: true,
            }],
            vec![ReflexControl {
                id: "decay".into(),
                label: "Decay".into(),
                node_id: "algo".into(),
                available: true,
            }],
            diagram,
        )
        .expect("workspace");
        let linked = workspace.select_node("algo").expect("node");
        let linked_label = linked[0].label.clone();
        drop(linked);
        assert_eq!(workspace.selected_node.as_deref(), Some("algo"));
        assert_eq!(linked_label, "Decay");
        assert_eq!(workspace.shared_controls()[0].label, "MIDI Channel");
        let frame = workspace.frame_lines(Viewport::new(80, 24));
        assert_eq!(frame[1], "Logical/control view");
        assert!(frame.iter().any(|line| line == "> [algo] Algorithm"));
        assert!(frame.iter().any(|line| line.contains("[available] Decay")));
        assert!(workspace.select_node("missing").is_err());
    }

    #[test]
    fn profile_device_frame_preserves_signal_and_control_order() {
        let diagram = SignalFlowDiagram::new(
            vec![
                SignalFlowNode { id: "pitch".into(), label: "Pitch Shift".into(), online: true },
                SignalFlowNode { id: "delay".into(), label: "Delay".into(), online: true },
            ],
            4,
        )
        .expect("diagram");
        let mut workspace = DeviceWorkspace::new(
            "eventide.micropitch".into(),
            "Eventide MicroPitch".into(),
            diagram,
            vec!["Bypass".into(), "MIDI Channel".into()],
            vec![
                DeviceControlGroup {
                    id: "pitch".into(),
                    label: "Pitch".into(),
                    block_id: "pitch".into(),
                    control_ids: vec!["Pitch A".into(), "Pitch B".into()],
                },
                DeviceControlGroup {
                    id: "delay".into(),
                    label: "Delay".into(),
                    block_id: "delay".into(),
                    control_ids: vec!["Delay A".into(), "Delay B".into()],
                },
            ],
            false,
        )
        .expect("workspace");
        workspace.select_block("delay").expect("selection");
        let frame = workspace.frame_lines(Viewport::new(80, 24));
        assert_eq!(frame[1], "Logical/control view");
        assert!(frame.iter().any(|line| line == "> [delay] Delay"));
        assert!(frame.iter().any(|line| line == "    Delay: Delay A, Delay B"));
        assert!(frame.iter().all(|line| line.chars().count() <= 80));
    }

    #[test]
    fn eventide_workspace_exposes_every_documented_control_once() {
        let workspace = DeviceWorkspace::eventide_micropitch();
        let profile = mackes_profiles::eventide_micropitch_profile();
        let mut rendered = workspace.shared_controls.clone();
        rendered.extend(workspace.groups.iter().flat_map(|group| group.control_ids.clone()));
        let expected =
            profile.controls.iter().map(|control| control.label.clone()).collect::<Vec<_>>();
        assert_eq!(rendered.len(), expected.len());
        assert!(expected
            .iter()
            .all(|label| rendered.iter().filter(|item| *item == label).count() == 1));
        assert_eq!(workspace.diagram_notice(), None);
        let frame = workspace.frame_lines(Viewport::new(80, 24));
        assert!(frame.iter().any(|line| line.contains("Eventide MicroPitch")));
        assert!(frame.iter().any(|line| line.contains("Logical/control view")));
    }

    #[test]
    fn compiled_reflex_algorithms_build_navigable_pages() {
        for algorithm in mackes_profiles::lexicon_reflex::algorithms() {
            let mut workspace = ReflexWorkspace::from_compiled_algorithm(algorithm.number)
                .expect("compiled algorithm");
            assert_eq!(workspace.algorithm_label, algorithm.name);
            assert!(!workspace.controls.is_empty());
            let views = ReflexWorkspace::compiled_parameter_views(algorithm.number)
                .expect("parameter views");
            assert_eq!(views.len(), workspace.controls.len());
            assert!(views
                .iter()
                .all(|view| { view.range.0 <= view.range.1 && view.effective_steps > 0 }));
            let node = format!("algorithm-{}", algorithm.number);
            assert_eq!(workspace.select_node(&node).expect("node").len(), workspace.controls.len());
            let frame = workspace.frame_lines(Viewport::new(80, 24));
            assert_eq!(frame[1], "Logical/control view");
            assert!(frame.iter().any(|line| line.contains(algorithm.name)));
        }
        assert_eq!(ReflexWorkspace::from_compiled_algorithm(0), Err("unknown Reflex algorithm"));
        assert_eq!(ReflexWorkspace::compiled_parameter_views(0), Err("unknown Reflex algorithm"));
    }

    #[test]
    fn mapping_browser_is_stable_musical_and_compact() {
        let mapping = mackes_config::ControlMapping {
            id: "map-1".into(),
            controller_profile: "launch-control-xl-mk2".into(),
            physical_control_id: "knob-r1-c1".into(),
            source_endpoint: "controller".into(),
            source_kind: "cc".into(),
            source_channel: 0,
            source_number: 21,
            destination_endpoint: "processor".into(),
            destination_profile: "lexicon.reflex".into(),
            destination_effect: "algorithm-1".into(),
            destination_parameter: "reflex.parameter-1".into(),
            behavior: mackes_config::MappingBehavior {
                source_range: (0, 127),
                destination_range: (0, 127),
                invert: false,
                curve: "linear".into(),
            },
            enabled: true,
            profile_version: 1,
        };
        let activity = serde_json::json!({
            "mapping_id": "map-1", "source_value": 64, "destination_value": 32
        });
        let disabled = mackes_config::ControlMapping {
            id: "map-disabled".into(),
            physical_control_id: "knob-r1-c2".into(),
            enabled: false,
            ..mapping.clone()
        };
        let offline = mackes_config::ControlMapping {
            id: "map-offline".into(),
            physical_control_id: "knob-r1-c3".into(),
            destination_profile: "unknown.profile".into(),
            ..mapping.clone()
        };
        let mut browser =
            MappingBrowser::from_authoritative(&[mapping, disabled, offline], Some(&activity));
        assert_eq!(browser.rows[0].physical_label, "Top knob 1");
        assert_eq!(
            browser.rows[0].destination_path,
            "lexicon.reflex › algorithm-1 › reflex.parameter-1"
        );
        assert_eq!(browser.rows[0].status, MappingBrowserStatus::Enabled);
        assert_eq!(browser.rows[0].current_source_value, Some(64));
        assert_eq!(browser.rows[0].last_destination_result, Some(32));
        assert_eq!(browser.rows[1].status, MappingBrowserStatus::Disabled);
        assert_eq!(browser.rows[2].status, MappingBrowserStatus::Offline);
        assert_eq!(browser.select(0), Some("knob-r1-c1"));
        assert_eq!(browser.page(1).len(), 1);
        assert_eq!(browser.page(0).len(), 0);
        let lines = mapping_browser_lines(&browser, Viewport::new(80, 8));
        assert!(lines.iter().all(|line| line.chars().count() <= 80));
        assert!(lines[1].contains("Top knob 1") && lines[1].contains("lexicon.reflex"));
        assert!(lines.iter().any(|line| line.contains("| OFF |")));
        assert!(lines.iter().any(|line| line.contains("OFFLINE")));
        assert_eq!(mapping_browser_lines(&browser, Viewport::new(79, 8)).len(), 4);
    }

    #[test]
    fn advanced_mapping_editor_exposes_profile_range_and_rejects_partial_edits() {
        let mapping = mackes_config::ControlMapping {
            id: "map-advanced".into(),
            controller_profile: "launch-control-xl-mk2".into(),
            physical_control_id: "knob-r1-c1".into(),
            source_endpoint: "controller".into(),
            source_kind: "cc".into(),
            source_channel: 0,
            source_number: 21,
            destination_endpoint: "processor".into(),
            destination_profile: "lexicon.reflex".into(),
            destination_effect: "algorithm-1".into(),
            destination_parameter: "reflex.parameter-1".into(),
            behavior: mackes_config::MappingBehavior {
                source_range: (0, 127),
                destination_range: (0, 127),
                invert: false,
                curve: "linear".into(),
            },
            enabled: true,
            profile_version: 1,
        };
        let mut editor = AdvancedMappingEditor::from_mapping(&mapping);
        assert_eq!(editor.button_mode, MappingButtonMode::ProfileDefault);
        assert_eq!(editor.behavior.destination_range, (0, 127));
        editor.set_source_range((8, 96)).expect("source range");
        editor.set_destination_range((12, 100)).expect("destination range");
        editor.set_invert(true).expect("invert");
        editor.set_curve("square").expect("curve");
        assert_eq!(editor.behavior.source_range, (8, 96));
        assert_eq!(editor.behavior.destination_range, (12, 100));
        assert!(editor.behavior.invert);
        assert_eq!(editor.behavior.curve, "square");
        let before = editor.behavior.clone();
        assert!(editor
            .set_behavior(mackes_config::MappingBehavior {
                source_range: (127, 0),
                ..before.clone()
            })
            .is_err());
        assert_eq!(editor.behavior, before);
        assert!(editor.error.is_some());
    }

    #[test]
    fn mapping_outcomes_keep_recovery_action_inline_and_typed() {
        let result = mackes_ipc::MappingResult {
            generation: 4,
            undo_available: true,
            active: None,
            draft: None,
            outcome: mackes_ipc::MappingOutcome::Conflict,
        };
        let response = serde_json::to_string(&result).expect("mapping result");
        assert_eq!(
            mapping_response_notice(&response),
            "Occupied source/destination; choose Replace or Cancel"
        );
        assert_eq!(mapping_response_notice("{}"), "Mapping request failed; retry");
    }

    #[test]
    fn task_sections_have_explicit_empty_and_populated_landings() {
        for section in AppSection::all() {
            assert!(!section_landing(section, false).is_empty());
            assert_eq!(section_landing(section, true), section.purpose());
        }
        assert!(section_landing(AppSection::MapControls, false).contains("Device"));
    }

    #[test]
    fn every_task_section_has_actionable_contextual_help() {
        for section in AppSection::all() {
            let help = section_help(section);
            assert!(!help.is_empty());
            assert!(help.split_whitespace().count() >= 5);
        }
        assert!(section_help(AppSection::MapControls).contains("Advanced"));
        assert!(section_help(AppSection::Scenes).contains("setlists"));
        assert!(section_help(AppSection::Devices).contains("Reflex"));
        assert!(section_help(AppSection::System).contains("Legacy"));
    }

    #[test]
    fn assignment_wizard_preserves_prior_section_and_debounces_capture() {
        let mut wizard = AssignmentWizard::new();
        let start = wizard.start(AppSection::Devices);
        assert_eq!(start.action, mackes_ipc::AssignmentAction::Start);
        assert_eq!(wizard.prior_section, AppSection::Devices);
        let capture = wizard.capture("knob-r1-c1").expect("first capture");
        assert_eq!(capture.physical_control_id.as_deref(), Some("knob-r1-c1"));
        assert!(wizard.capture("knob-r1-c1").is_none());
        assert!(wizard.capture("utility-1").is_none());
        assert!(wizard.capture("unknown-control").is_none());
        assert_eq!(AssignmentWizard::classify_capture(&[]), mackes_ipc::CandidateCapture::None);
        assert_eq!(
            AssignmentWizard::classify_capture(&["knob-r1-c1", "knob-r1-c1"]),
            mackes_ipc::CandidateCapture::Unique
        );
        assert_eq!(
            AssignmentWizard::classify_capture(&["knob-r1-c1", "fader-1"]),
            mackes_ipc::CandidateCapture::Ambiguous
        );
        wizard.reconcile(mackes_ipc::AssignmentResult {
            generation: 1,
            session: mackes_ipc::AssignmentSession {
                phase: mackes_ipc::AssignmentPhase::ChooseDevice,
                prior_screen: "Devices".into(),
                index: 0,
                total: 2,
                has_draft: true,
                interrupted_phase: None,
                cursors: mackes_ipc::AssignmentCursors::default(),
            },
            applied: true,
            reason: None,
        });
        assert_eq!(wizard.generation, 1);
        assert_eq!(wizard.session.phase, mackes_ipc::AssignmentPhase::ChooseDevice);
        let lines = assignment_wizard_lines(&wizard, Viewport::new(32, 8));
        assert!(lines.iter().all(|line| line.chars().count() <= 32));
        assert!(lines.iter().any(|line| line.contains("CHOOSE DEVICE")));
        assert!(lines.iter().any(|line| line.contains("Position: 1 OF 2")));
    }

    #[test]
    fn assignment_wizard_entry_is_available_from_every_task_section() {
        for section in AppSection::all() {
            let mut wizard = AssignmentWizard::new();
            let request = wizard.start(section);
            assert_eq!(request.action, mackes_ipc::AssignmentAction::Start);
            assert_eq!(wizard.prior_section, section);
            assert_eq!(wizard.session.prior_screen, "Live");
            wizard.reconcile(mackes_ipc::AssignmentResult {
                generation: 1,
                session: mackes_ipc::AssignmentSession {
                    phase: mackes_ipc::AssignmentPhase::Idle,
                    prior_screen: section.label().into(),
                    index: 0,
                    total: 0,
                    has_draft: false,
                    interrupted_phase: None,
                    cursors: mackes_ipc::AssignmentCursors::default(),
                },
                applied: true,
                reason: None,
            });
            assert_eq!(wizard.prior_section, section);
            assert_eq!(wizard.session.phase, mackes_ipc::AssignmentPhase::Idle);
        }
    }

    #[test]
    fn assignment_wizard_builds_typed_destination_commit_payload() {
        let mut wizard = AssignmentWizard::new();
        wizard.start(AppSection::MapControls);
        wizard.capture("fader-1").expect("capture");
        wizard.generation = 9;
        let choice = AssignmentParameterChoice {
            profile_id: "lexicon.reflex".into(),
            id: "reflex.mix".into(),
            label: "Mix".into(),
            effect_id: "algorithm-1".into(),
            effect_label: "Algorithm 1".into(),
            reason: mackes_profiles::SupportReason::Compatible,
        };
        let request = wizard.destination_request(&choice);
        assert_eq!(request.generation, 9);
        assert_eq!(request.action, mackes_ipc::AssignmentAction::Commit);
        assert_eq!(request.physical_control_id.as_deref(), Some("fader-1"));
        assert_eq!(request.destination_profile.as_deref(), Some("lexicon.reflex"));
        assert_eq!(request.destination_effect.as_deref(), Some("algorithm-1"));
        assert_eq!(request.destination_parameter.as_deref(), Some("reflex.mix"));
        assert!(request.has_complete_destination());
        assert!(request.validate().is_ok());
    }

    #[test]
    fn assignment_wizard_selects_reflex_preset_for_channel_button() {
        let mut wizard = AssignmentWizard::new();
        wizard.start(AppSection::Devices);
        wizard.capture("button-r1-c1").expect("capture");
        let choices = AssignmentChoiceBrowser::from_profiles(
            &["lexicon.reflex"],
            Some("lexicon.reflex"),
            mackes_profiles::SourceRole::ButtonAction,
        );
        let request = wizard.selected_destination_request(&choices).expect("preset request");
        assert_eq!(request.destination_effect.as_deref(), Some("reverb"));
        assert_eq!(request.destination_parameter.as_deref(), Some("pcm70_reflex:concert-wave"));
        assert!(request.validate().is_ok());
    }

    #[test]
    fn assignment_frames_are_bounded_at_release_viewports() {
        let phases = [
            (mackes_ipc::AssignmentPhase::AwaitControl, "MOVE ONLY ONE CONTROL"),
            (mackes_ipc::AssignmentPhase::ChooseDevice, "CHOOSE DEVICE"),
            (mackes_ipc::AssignmentPhase::ChooseEffect, "CHOOSE EFFECT"),
            (mackes_ipc::AssignmentPhase::ChooseParameter, "CHOOSE PARAMETER"),
            (mackes_ipc::AssignmentPhase::ConfirmReplace, "REPLACE EXISTING MAPPING?"),
            (mackes_ipc::AssignmentPhase::Committing, "ASSIGNING"),
            (mackes_ipc::AssignmentPhase::Succeeded, "ASSIGNED"),
            (mackes_ipc::AssignmentPhase::Failed, "ASSIGNMENT FAILED"),
            (mackes_ipc::AssignmentPhase::Interrupted, "INTERRUPTED"),
        ];
        for (phase, marker) in phases {
            let wizard = AssignmentWizard {
                session: mackes_ipc::AssignmentSession {
                    phase,
                    prior_screen: "Devices".into(),
                    index: 0,
                    total: 1,
                    has_draft: phase == mackes_ipc::AssignmentPhase::Interrupted,
                    interrupted_phase: None,
                    cursors: mackes_ipc::AssignmentCursors::default(),
                },
                prior_section: AppSection::Devices,
                candidates: vec!["knob-r1-c1".into()],
                generation: 4,
            };
            for (width, height) in [(160, 37), (100, 37), (80, 24)] {
                let lines = assignment_wizard_lines(&wizard, Viewport::new(width, height));
                assert!(lines.iter().all(|line| line.chars().count() <= usize::from(width)));
                assert!(lines.len() <= usize::from(height));
                assert!(lines.iter().any(|line| line.contains(marker)));
            }
        }
    }

    #[test]
    fn assignment_choices_are_profile_owned_filtered_and_bounded() {
        let mut choices = AssignmentChoiceBrowser::from_profiles(
            &["lexicon.reflex", "unknown-device", "eventide.micropitch"],
            Some("lexicon.reflex"),
            mackes_profiles::SourceRole::Continuous,
        );
        assert_eq!(choices.devices, vec!["lexicon.reflex", "eventide.micropitch"]);
        assert_eq!(choices.presets.len(), 5);
        assert!(choices.presets.iter().any(|(_, label)| label == "Concert Wave"));
        assert!(!choices.effects.is_empty());
        assert!(choices.parameters.iter().all(|choice| {
            matches!(
                choice.reason,
                mackes_profiles::SupportReason::Compatible
                    | mackes_profiles::SupportReason::Experimental
            )
        }));
        assert!(choices
            .parameters
            .iter()
            .all(|choice| { choices.effects.iter().any(|(id, _)| id == &choice.effect_id) }));
        let eventide = AssignmentChoiceBrowser::from_profiles(
            &["eventide.micropitch"],
            Some("eventide.micropitch"),
            mackes_profiles::SourceRole::Continuous,
        );
        assert!(eventide.presets.is_empty());
        assert!(!eventide.parameters.is_empty());
        choices.selected = usize::MAX;
        assert!(choices.move_selection(true));
        assert!(!choices.move_selection(true));
    }
}
