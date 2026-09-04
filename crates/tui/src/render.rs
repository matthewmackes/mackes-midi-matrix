//! Ratatui draw adapters over authoritative TUI state.

use super::{
    faceplate_state_marker, launch_control_index_label, mapping_browser_lines, AppSection,
    BackupWorkspace, Block, Borders, ClientState, Color, Constraint, DashboardState,
    DeviceWorkspace, DiagnosticsState, Direction, FaceplateControlState, Frame, Keymap,
    LaunchControlMessageKind, LaunchControlTemplate, Layout, LearnPhase, LearnWorkspace, Line,
    MappingDraft, Modifier, MonitorState, Paragraph, PhysicalDevice, RackShellLayout, Rect,
    ReducerError, ReflexWorkspace, RoutingEditor, SetlistEditor, Span, StateEvent, StateSnapshot,
    Style, TaskShellState, UiCommand, Viewport,
};

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
    // An active Device assignment is deliberately modal: the operator should
    // never have to hunt through the task rail while the controller is waiting
    // for a physical gesture. The daemon-projected feedback already contains
    // the authoritative catalog; this view only presents it.
    if !dashboard.assignment_feedback.is_empty() {
        draw_assignment_takeover(frame, area, dashboard);
        return;
    }
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

fn draw_assignment_takeover(frame: &mut Frame<'_>, area: Rect, dashboard: &DashboardState) {
    let mut lines = dashboard.assignment_feedback.clone();
    lines.extend([
        String::new(),
        "↑↓ SELECT   ENTER NEXT   ← BACK   ESC CANCEL".into(),
        "MOVE ONE CONTROL AT A TIME".into(),
    ]);
    let width = usize::from(area.width.saturating_sub(2));
    let text = lines
        .into_iter()
        .map(|line| line.chars().take(width).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::default().title("DEVICE ASSIGNMENT").borders(Borders::ALL)),
        area,
    );
}

pub(crate) const fn section_help(section: AppSection) -> &'static str {
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

pub(crate) fn clamp_lines(lines: Vec<String>, viewport: Viewport, width: usize) -> Vec<String> {
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

pub(crate) fn draw_lines(frame: &mut Frame<'_>, area: Rect, title: &str, lines: &[String]) {
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

pub(crate) fn value_bar(value: u16) -> String {
    let bounded = value.min(127);
    let filled = usize::from(bounded.saturating_mul(16) / 127);
    format!("[{}{}]", "#".repeat(filled), "-".repeat(16 - filled))
}

pub(crate) fn device_tabs_line(devices: &[PhysicalDevice]) -> String {
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

pub(crate) fn input_inventory_line(devices: &[PhysicalDevice], selected: Option<usize>) -> String {
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

pub(crate) fn mapping_chain_line(drafts: &[MappingDraft]) -> String {
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

pub(crate) fn destination_inventory_line(
    devices: &[PhysicalDevice],
    selected: Option<usize>,
) -> String {
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

pub(crate) fn health_is_online(health: &str) -> bool {
    matches!(health, "online" | "ready")
}

pub(crate) const fn live_activity_age_label(age_nanos: u64) -> &'static str {
    if age_nanos < 1_000_000_000 {
        "ACTIVE"
    } else {
        "STALE"
    }
}

pub(crate) fn destination_parameter_line(
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

pub(crate) fn destination_profile_for_name(name: &str) -> Option<mackes_profiles::DeviceProfile> {
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

pub(crate) fn control_cell(
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

pub(crate) fn faceplate_control_state(
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

pub(crate) fn launch_control_activity_index(dashboard: &DashboardState) -> Option<u8> {
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

/// Returns the operator-facing readiness state for Factory Template 1.
///
/// The decision is fail-closed: an absent, invalid, non-Factory-1, or
/// incomplete layout cannot be treated as ready for deterministic capture.
#[must_use]
pub fn launch_control_template_readiness(template: Option<&LaunchControlTemplate>) -> &'static str {
    let Some(template) = template else {
        return "MACKES TEMPLATE REQUIRED";
    };
    if template.validate().is_err()
        || template.template != mackes_profiles::LAUNCH_CONTROL_MK2_FACTORY1_SLOT
    {
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
