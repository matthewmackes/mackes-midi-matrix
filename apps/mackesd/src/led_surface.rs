//! Daemon-owned Launch Control XL Mk2 LED desired/actual surface.

use mackes_config::ControlMappingStore;
use mackes_domain::{EndpointId, MidiEvent, MidiMessage, TimestampNanos};
use mackes_ipc::{AssignmentPhase, AssignmentSession};
use mackes_midi_engine::{
    numeric_endpoint_id, AlsaSequencerAddress, AlsaSequencerPort, EndpointInfo, OutputRegistry,
};
use mackes_profiles::{
    encode_launch_control_feedback, fader_column_led_proxy, launch_control_physical_catalog,
    LedCoalescer, LedColor, LedFeedbackScheduler, LedState, PhysicalControlRole,
    LAUNCH_CONTROL_DEVICE_INDEX,
};
use std::collections::{BTreeMap, BTreeSet};

/// Mk1 LED `SysEx` template byte for Factory 1 (User 0–7, Factory 8–15).
const FACTORY1_LED_TEMPLATE: u8 = mackes_profiles::LAUNCH_CONTROL_MK2_FACTORY1_SLOT;

const LED_SEND_ATTEMPTS: u8 = 3;
const LED_FLUSH_LIMIT: usize = 48;
const IDLE_SLEEP_AFTER_MS: u64 = 5 * 60 * 1_000;
const IDLE_SWEEP_STEP_MS: u64 = 600;
const RECONNECT_SHOW_MS: u64 = 12_000;
const ARROW_MIN_VISIBLE_MS: u64 = 120;
const CONFIRMATION_MIN_VISIBLE_MS: u64 = 1_200;
const KNOB_ACTIVITY_VISIBLE_MS: u64 = 180;

#[derive(Clone, Debug)]
struct BackendConfirmation {
    control_id: String,
    active: bool,
    started_ms: u64,
    delivered: Option<bool>,
}

/// Counters and identity published on the daemon snapshot.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LedDiagnostics {
    /// Frames the surface attempted to write.
    pub attempted: u64,
    /// Frames accepted by a registered unique output.
    pub sent: u64,
    /// Unchanged frames skipped by the coalescer.
    pub coalesced: u64,
    /// Encode, uniqueness, lock, or send failures.
    pub failed: u64,
    /// Most recent failure reason.
    pub last_error: Option<String>,
    /// Stable output identity selected for writes.
    pub target_id: Option<String>,
    /// Factory 1 LED `SysEx` template byte (8). Operator slot remains 1.
    pub template: u8,
    /// Milliseconds until the next result-overlay deadline.
    pub pending_deadline_ms: Option<u64>,
    /// Current backend-control delivery state.
    pub backend_confirmation: Option<String>,
    /// Last successfully delivered logical backend state.
    pub backend_state: Option<String>,
    /// Last Lexicon algorithm confirmed by a selection or active-setup readback.
    pub active_lexicon_algorithm: Option<u8>,
    /// Most recent Lexicon active-setup decode error, if any.
    pub lexicon_readback_error: Option<String>,
}

impl LedDiagnostics {
    const fn new() -> Self {
        Self {
            attempted: 0,
            sent: 0,
            coalesced: 0,
            failed: 0,
            last_error: None,
            target_id: None,
            template: FACTORY1_LED_TEMPLATE,
            pending_deadline_ms: None,
            backend_confirmation: None,
            backend_state: None,
            active_lexicon_algorithm: None,
            lexicon_readback_error: None,
        }
    }
}

/// Authoritative desired/actual LED surface for one Launch Control XL Mk2.
#[derive(Clone, Debug)]
pub struct LedSurface {
    /// Overlay scheduler for the captured (or Device) LED.
    pub scheduler: LedFeedbackScheduler,
    coalescer: LedCoalescer,
    result_started_ms: u64,
    last_touch_ms: u64,
    sleep_active: bool,
    reconnect_show_started_ms: Option<u64>,
    template_reselect_pending: bool,
    arrow_states: BTreeMap<u8, (bool, u64)>,
    knob_activity: BTreeMap<String, u64>,
    backend_confirmation: Option<BackendConfirmation>,
    backend_states: BTreeMap<String, bool>,
    active_lexicon_algorithm: Option<u8>,
    diagnostics: LedDiagnostics,
}

impl Default for LedSurface {
    fn default() -> Self {
        Self {
            scheduler: LedFeedbackScheduler::new(LedState::new(LedColor::Off, 0, false)),
            coalescer: LedCoalescer::default(),
            result_started_ms: 0,
            last_touch_ms: 0,
            sleep_active: false,
            reconnect_show_started_ms: None,
            template_reselect_pending: false,
            arrow_states: BTreeMap::new(),
            knob_activity: BTreeMap::new(),
            backend_confirmation: None,
            backend_states: BTreeMap::new(),
            active_lexicon_algorithm: None,
            diagnostics: LedDiagnostics::new(),
        }
    }
}

impl LedSurface {
    /// Records the last Lexicon algorithm selection delivered by the daemon.
    pub fn set_active_lexicon_algorithm(&mut self, algorithm: u8) {
        self.active_lexicon_algorithm = Some(algorithm);
        self.diagnostics.active_lexicon_algorithm = Some(algorithm);
        self.diagnostics.lexicon_readback_error = None;
        self.request_full_resync();
    }

    pub fn set_lexicon_readback_error(&mut self, error: impl Into<String>) {
        self.diagnostics.lexicon_readback_error = Some(error.into());
    }
    /// Forces a complete replay of desired state on the next flush.
    pub fn request_full_resync(&mut self) {
        self.coalescer.request_full_resync();
    }

    /// Records controller activity and immediately wakes the normal LED surface.
    pub fn record_touch(&mut self, now_ms: u64) {
        self.last_touch_ms = now_ms;
        if self.sleep_active {
            self.sleep_active = false;
            self.request_full_resync();
        }
    }

    /// Mirrors the four navigation buttons in green for exactly as long as held.
    pub fn record_arrow(&mut self, control_id: &str, pressed: bool, now_ms: u64) {
        let index = match control_id {
            "utility-5" => 44,
            "utility-6" => 45,
            "utility-7" => 46,
            "utility-8" => 47,
            _ => return,
        };
        if pressed {
            self.arrow_states.insert(index, (true, now_ms.saturating_add(ARROW_MIN_VISIBLE_MS)));
        } else if let Some(state) = self.arrow_states.get_mut(&index) {
            state.0 = false;
        }
        self.request_full_resync();
    }

    /// Blinks a mapped knob green while its value is moving.
    pub fn record_knob_activity(&mut self, control_id: &str, now_ms: u64) {
        if control_id.starts_with("knob-") {
            self.knob_activity
                .insert(control_id.into(), now_ms.saturating_add(KNOB_ACTIVITY_VISIBLE_MS));
            self.request_full_resync();
        }
    }

    /// Starts visible feedback before a mapped backend write is attempted.
    pub fn begin_backend_confirmation(&mut self, control_id: &str, active: bool, now_ms: u64) {
        self.backend_confirmation = Some(BackendConfirmation {
            control_id: control_id.into(),
            active,
            started_ms: now_ms,
            delivered: None,
        });
        self.diagnostics.backend_confirmation = Some("pending".into());
        self.request_full_resync();
    }

    /// Records whether the mapped backend write completed successfully.
    pub fn finish_backend_confirmation(&mut self, delivered: bool) {
        if let Some(confirmation) = self.backend_confirmation.as_mut() {
            confirmation.delivered = Some(delivered);
            if delivered {
                self.backend_states.insert(confirmation.control_id.clone(), confirmation.active);
                self.diagnostics.backend_state =
                    Some(if confirmation.active { "active" } else { "bypassed" }.into());
            }
        }
        self.diagnostics.backend_confirmation =
            Some(if delivered { "delivered_unconfirmed" } else { "failed" }.into());
        self.request_full_resync();
    }

    fn apply_backend_state(&self, desired: &mut BTreeMap<u8, LedState>, now_ms: u64) {
        let catalog = launch_control_physical_catalog();
        let led_index = |control_id: &str| {
            catalog
                .iter()
                .find(|control| control.id.as_str() == control_id)
                .and_then(|control| control.feedback_address)
        };
        for (control_id, active) in &self.backend_states {
            if let Some(index) = led_index(control_id) {
                desired.insert(
                    index,
                    LedState::new(
                        if *active { LedColor::Green } else { LedColor::Red },
                        127,
                        false,
                    ),
                );
            }
        }
        if let Some(confirmation) = &self.backend_confirmation {
            let complete = confirmation.delivered == Some(true)
                && now_ms.saturating_sub(confirmation.started_ms) >= CONFIRMATION_MIN_VISIBLE_MS;
            if let Some(index) = led_index(&confirmation.control_id) {
                let color = if confirmation.delivered == Some(false) || !confirmation.active {
                    LedColor::Red
                } else {
                    LedColor::Green
                };
                desired.insert(index, LedState::new(color, 127, !complete));
            }
        }
    }

    /// Starts the bounded reconnect celebration; normal mapping state resumes afterward.
    pub fn start_reconnect_show(&mut self, now_ms: u64) {
        self.reconnect_show_started_ms = Some(now_ms);
        self.template_reselect_pending = true;
        self.request_full_resync();
    }

    /// Snapshot diagnostics for IPC/TUI status.
    #[must_use]
    pub const fn diagnostics(&self) -> &LedDiagnostics {
        &self.diagnostics
    }

    /// Updates overlays from the assignment session without transmitting.
    pub fn sync_session(
        &mut self,
        session: &AssignmentSession,
        captured: Option<&str>,
        mappings: &ControlMappingStore,
        now_ms: u64,
    ) {
        self.scheduler.base = captured.map_or_else(
            || LedState::new(LedColor::Off, 0, false),
            |control_id| owner_state_for(mappings, control_id),
        );
        match session.phase {
            AssignmentPhase::Idle => self.scheduler.restore_base(),
            AssignmentPhase::Interrupted => {
                self.scheduler.assignment = Some(LedState::new(LedColor::Red, 127, true));
            }
            AssignmentPhase::Succeeded | AssignmentPhase::Failed => {
                self.scheduler.assignment = None;
                if self.scheduler.result.is_none() {
                    self.scheduler.result = Some((session.phase == AssignmentPhase::Succeeded, 0));
                    self.result_started_ms = now_ms;
                }
            }
            _ => {
                self.scheduler.assignment = Some(LedState::new(LedColor::Yellow, 127, true));
            }
        }
        self.diagnostics.pending_deadline_ms =
            pending_deadline(self.scheduler, now_ms, self.result_started_ms);
    }

    /// Rebuilds desired state and emits changed Factory 1 frames to one unique output.
    #[allow(clippy::too_many_arguments)]
    pub fn flush(
        &mut self,
        now_ms: u64,
        mappings: &ControlMappingStore,
        session: &AssignmentSession,
        captured: Option<&str>,
        outputs: &mut OutputRegistry,
        performance_locked: bool,
    ) {
        self.sync_session(session, captured, mappings, now_ms);
        let mut desired = compose_desired(
            mappings,
            session,
            captured,
            self.scheduler,
            now_ms,
            self.result_started_ms,
            self.active_lexicon_algorithm,
        );
        if let Some(started) = self.reconnect_show_started_ms {
            let elapsed = now_ms.saturating_sub(started);
            if elapsed < RECONNECT_SHOW_MS && session.phase == AssignmentPhase::Idle {
                apply_reconnect_show(&mut desired, elapsed);
            } else {
                self.reconnect_show_started_ms = None;
                self.request_full_resync();
            }
        }
        // Preserve quick taps long enough to survive a batched press/release pair.
        self.arrow_states.retain(|_, (held, until)| *held || now_ms < *until);
        for index in self.arrow_states.keys() {
            desired.insert(*index, LedState::new(LedColor::Green, 127, false));
        }
        self.knob_activity.retain(|_, until| now_ms < *until);
        let catalog = launch_control_physical_catalog();
        for control_id in self.knob_activity.keys() {
            if let Some(index) = catalog
                .iter()
                .find(|control| control.id.as_str() == control_id)
                .and_then(|control| control.feedback_address)
            {
                desired.insert(index, LedState::new(LedColor::Green, 127, true));
            }
        }
        self.apply_backend_state(&mut desired, now_ms);
        let should_sleep = session.phase == AssignmentPhase::Idle
            && now_ms.saturating_sub(self.last_touch_ms) >= IDLE_SLEEP_AFTER_MS;
        if should_sleep {
            if !self.sleep_active {
                self.sleep_active = true;
                self.request_full_resync();
            }
            for state in desired.values_mut() {
                *state = off();
            }
            let elapsed = now_ms.saturating_sub(self.last_touch_ms + IDLE_SLEEP_AFTER_MS);
            let index = u8::try_from((elapsed / IDLE_SWEEP_STEP_MS) % 48).unwrap_or(0);
            desired.insert(index, LedState::new(LedColor::Red, 127, false));
        } else if self.sleep_active {
            self.sleep_active = false;
            self.request_full_resync();
        }
        for (index, state) in &desired {
            self.coalescer.set_desired(*index, *state);
        }
        if performance_locked {
            self.fail("LED writes blocked by performance lock");
            return;
        }
        let infos = outputs.output_infos();
        let selected = match unique_launch_control_midi_output(
            infos.iter().map(|info| (info.id.as_str(), info.name.as_str())),
        ) {
            Ok(selected) => selected,
            Err(error) => {
                self.fail(&error);
                return;
            }
        };
        self.diagnostics.target_id = Some(selected.1);
        if self.template_reselect_pending {
            if let Some(bytes) = mackes_profiles::encode_launch_control_template(
                mackes_profiles::LAUNCH_CONTROL_MK2_FACTORY1_SLOT,
            ) {
                if let Ok(message) = MidiMessage::from_wire(&bytes) {
                    let event = MidiEvent {
                        timestamp: TimestampNanos::new(0),
                        sequence: 0,
                        endpoint: selected.0,
                        message,
                    };
                    if outputs.send_to_endpoint(selected.0, event).is_ok() {
                        self.template_reselect_pending = false;
                    }
                }
            }
        }
        let pending_len = self.coalescer.pending_len();
        self.diagnostics.coalesced = self
            .diagnostics
            .coalesced
            .saturating_add((self.coalescer.desired_len().saturating_sub(pending_len)) as u64);
        let pending = self.coalescer.drain_pending_limited(LED_FLUSH_LIMIT);
        for (index, state) in pending {
            self.emit_frame(outputs, selected.0, index, state);
        }
        self.diagnostics.pending_deadline_ms =
            pending_deadline(self.scheduler, now_ms, self.result_started_ms);
    }

    fn emit_frame(
        &mut self,
        outputs: &mut OutputRegistry,
        endpoint: EndpointId,
        index: u8,
        state: LedState,
    ) {
        self.diagnostics.attempted = self.diagnostics.attempted.saturating_add(1);
        let Some(bytes) = encode_launch_control_feedback(FACTORY1_LED_TEMPLATE, index, state)
        else {
            self.coalescer.revert_sent(index);
            self.fail("unsupported LED color or address");
            return;
        };
        let Ok(message) = MidiMessage::from_wire(&bytes) else {
            self.coalescer.revert_sent(index);
            self.fail("LED SysEx framing failed");
            return;
        };
        let event = MidiEvent { timestamp: TimestampNanos::new(0), sequence: 0, endpoint, message };
        for _ in 0..LED_SEND_ATTEMPTS {
            if outputs.send_to_endpoint(endpoint, event.clone()).is_ok() {
                self.diagnostics.sent = self.diagnostics.sent.saturating_add(1);
                self.diagnostics.last_error = None;
                return;
            }
        }
        self.coalescer.revert_sent(index);
        self.fail("LED destination output is not registered");
    }

    fn fail(&mut self, error: &str) {
        self.diagnostics.failed = self.diagnostics.failed.saturating_add(1);
        self.diagnostics.last_error = Some(error.into());
    }
}

fn apply_reconnect_show(desired: &mut BTreeMap<u8, LedState>, elapsed_ms: u64) {
    for state in desired.values_mut() {
        *state = off();
    }
    let phase = elapsed_ms / 1_500;
    match phase {
        0 => {
            let step = usize::try_from(elapsed_ms / 100).unwrap_or(0).min(14);
            for row in 0..6 {
                let left = row * 8 + step.min(7);
                let right = row * 8 + 7 - step.min(7);
                desired.insert(
                    u8::try_from(left).unwrap_or(0),
                    LedState::new(LedColor::Red, 127, false),
                );
                desired.insert(
                    u8::try_from(right).unwrap_or(0),
                    LedState::new(LedColor::Red, 127, false),
                );
            }
        }
        1 => {
            let step = usize::try_from((elapsed_ms - 1_500) / 150).unwrap_or(0).min(10);
            for row in 0..3 {
                for col in 0..=step.min(7) {
                    desired.insert(
                        u8::try_from(row * 8 + col).unwrap_or(0),
                        LedState::new(LedColor::Yellow, 127, false),
                    );
                }
            }
        }
        2 | 3 => {
            let count = usize::try_from((elapsed_ms - 3_000) / 180).unwrap_or(0).min(16);
            for index in 24..(24 + count) {
                desired.insert(
                    u8::try_from(index).unwrap_or(0),
                    LedState::new(LedColor::Green, 127, false),
                );
            }
        }
        4..=7 => {
            let digit = usize::try_from((elapsed_ms - 6_000) / 1_000).unwrap_or(0).min(5);
            let glyph = COUNTDOWN_GLYPHS[5 - digit];
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..3 {
                    if bits & (1 << (4 - col)) != 0 {
                        let index = row * 8 + col + 2;
                        desired.insert(
                            u8::try_from(index).unwrap_or(0),
                            LedState::new(
                                if digit == 5 { LedColor::Green } else { LedColor::Yellow },
                                127,
                                false,
                            ),
                        );
                    }
                }
            }
        }
        _ => {
            for index in 0..48 {
                desired.insert(index, LedState::new(LedColor::Green, 127, false));
            }
        }
    }
}

/// Five-row, three-column-friendly glyphs rendered on the top three knob rows.
const COUNTDOWN_GLYPHS: [[u8; 3]; 6] = [
    [0b111, 0b100, 0b111], // 0
    [0b010, 0b110, 0b010], // 1
    [0b110, 0b010, 0b111], // 2
    [0b110, 0b010, 0b110], // 3
    [0b101, 0b111, 0b001], // 4
    [0b111, 0b110, 0b110], // 5
];

/// Selects the single Launch Control XL Mk2 MIDI output, refusing HUI and duplicates.
pub fn unique_launch_control_midi_output<'a>(
    outputs: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<(EndpointId, String), String> {
    let mut matches = Vec::new();
    for (id, name) in outputs {
        let lowered = name.to_ascii_lowercase();
        if lowered.contains("hui") {
            continue;
        }
        if mackes_profiles::classify_launch_control(name)
            != mackes_profiles::LaunchControlIdentity::Mk2
        {
            continue;
        }
        matches.push((id, name));
    }
    match matches.as_slice() {
        [(id, _)] => numeric_endpoint_id(id)
            .map(|endpoint| (endpoint, (*id).to_owned()))
            .ok_or_else(|| "Launch Control XL output identity is not addressable".into()),
        [] => Err("no unique Launch Control XL MIDI output".into()),
        _ => Err("duplicate Launch Control XL MIDI outputs; LED writes refused".into()),
    }
}

/// Unique writable Mk2 MIDI sequencer port, ignoring HUI and duplicates.
#[must_use]
pub fn unique_mk2_midi_writable_address(
    ports: &[AlsaSequencerPort],
) -> Option<AlsaSequencerAddress> {
    let mut matches = Vec::new();
    for port in ports {
        if !port.writable {
            continue;
        }
        let name = format!("{} {}", port.client_name, port.port_name);
        let lowered = name.to_ascii_lowercase();
        if lowered.contains("hui") {
            continue;
        }
        if mackes_profiles::classify_launch_control(&name)
            != mackes_profiles::LaunchControlIdentity::Mk2
        {
            continue;
        }
        matches.push(port.address);
    }
    match matches.as_slice() {
        [address] => Some(*address),
        _ => None,
    }
}

fn owner_led_color(profile: &str) -> LedColor {
    let lowered = profile.to_ascii_lowercase();
    if lowered.contains("eventide") {
        LedColor::Red
    } else if lowered.contains("lexicon") {
        LedColor::Amber
    } else {
        LedColor::Green
    }
}

fn owner_state(profile: &str) -> LedState {
    LedState::new(owner_led_color(profile), 127, false)
}

fn owner_state_for_control(profile: &str, role: PhysicalControlRole) -> LedState {
    if profile.eq_ignore_ascii_case("lexicon.reflex") && role == PhysicalControlRole::ChannelButton
    {
        return LedState::new(LedColor::Green, 127, false);
    }
    owner_state(profile)
}

fn owner_state_for(mappings: &ControlMappingStore, control_id: &str) -> LedState {
    mappings.active.iter().find(|mapping| mapping.physical_control_id == control_id).map_or_else(
        || LedState::new(LedColor::Off, 0, false),
        |mapping| owner_state(&mapping.destination_profile),
    )
}

const fn off() -> LedState {
    LedState::new(LedColor::Off, 0, false)
}

fn pending_deadline(scheduler: LedFeedbackScheduler, now_ms: u64, started_ms: u64) -> Option<u64> {
    scheduler.result?;
    let elapsed = now_ms.saturating_sub(started_ms);
    [400_u64, 800, 1_200, 1_600].into_iter().find(|deadline| *deadline > elapsed)
}

fn focus_index(captured: Option<&str>) -> u8 {
    captured
        .and_then(|control_id| {
            launch_control_physical_catalog()
                .into_iter()
                .find(|control| control.id.as_str() == control_id)
                .and_then(|control| control.feedback_address)
        })
        .unwrap_or(LAUNCH_CONTROL_DEVICE_INDEX)
}

#[allow(clippy::too_many_arguments)]
fn compose_desired(
    mappings: &ControlMappingStore,
    session: &AssignmentSession,
    captured: Option<&str>,
    scheduler: LedFeedbackScheduler,
    now_ms: u64,
    result_started_ms: u64,
    active_lexicon_algorithm: Option<u8>,
) -> BTreeMap<u8, LedState> {
    let mut desired: BTreeMap<u8, LedState> = (0..48).map(|index| (index, off())).collect();
    let catalog = launch_control_physical_catalog();
    let mut claimed = BTreeSet::new();
    for mapping in &mappings.active {
        if !mapping.enabled {
            continue;
        }
        let Some(control) =
            catalog.iter().find(|item| item.id.as_str() == mapping.physical_control_id)
        else {
            continue;
        };
        if let Some(index) = control.feedback_address {
            let state = if mapping.destination_profile == "lexicon.reflex"
                && control.role == PhysicalControlRole::ChannelButton
            {
                let algorithm = mapping
                    .destination_parameter
                    .strip_prefix("reflex.algorithm-")
                    .and_then(|value| value.parse::<u8>().ok());
                if algorithm == active_lexicon_algorithm {
                    LedState::new(LedColor::Green, 127, false)
                } else {
                    LedState::new(LedColor::Amber, 127, false)
                }
            } else {
                owner_state_for_control(&mapping.destination_profile, control.role)
            };
            desired.insert(index, state);
            claimed.insert(index);
        }
    }
    for mapping in &mappings.active {
        if !mapping.enabled {
            continue;
        }
        let Some(control) =
            catalog.iter().find(|item| item.id.as_str() == mapping.physical_control_id)
        else {
            continue;
        };
        if control.role != PhysicalControlRole::Fader {
            continue;
        }
        let Some((upper, lower)) = fader_column_led_proxy(control.column.saturating_sub(1)) else {
            continue;
        };
        let color = owner_state_for_control(&mapping.destination_profile, control.role);
        if claimed.insert(upper) {
            desired.insert(upper, color);
        }
        if claimed.insert(lower) {
            desired.insert(lower, color);
        }
    }
    let learn_active = !matches!(
        session.phase,
        AssignmentPhase::Idle | AssignmentPhase::Succeeded | AssignmentPhase::Failed
    );
    if learn_active {
        desired.insert(LAUNCH_CONTROL_DEVICE_INDEX, LedState::new(LedColor::Yellow, 127, true));
        if let Some(index) = captured.and_then(|id| {
            catalog
                .iter()
                .find(|control| control.id.as_str() == id)
                .and_then(|control| control.feedback_address)
        }) {
            desired.insert(index, LedState::new(LedColor::Yellow, 127, true));
        }
    }
    let elapsed = now_ms.saturating_sub(result_started_ms);
    if scheduler.result.is_some() && elapsed < 1_600 {
        desired.insert(focus_index(captured), scheduler.state_at(elapsed));
    }
    desired
}

/// Resolves one unique Mk2 MIDI output from registry metadata.
pub fn unique_output_from_infos(infos: &[EndpointInfo]) -> Result<(EndpointId, String), String> {
    unique_launch_control_midi_output(
        infos.iter().map(|info| (info.id.as_str(), info.name.as_str())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use mackes_config::{ControlMapping, MappingBehavior};
    use mackes_midi_engine::{EndpointDirection, VirtualEndpoint};

    fn mapping(control_id: &str, profile: &str) -> ControlMapping {
        ControlMapping {
            id: format!("map-{control_id}"),
            controller_profile: "launch-control-xl-mk2".into(),
            physical_control_id: control_id.into(),
            source_endpoint: "controller".into(),
            source_kind: "cc".into(),
            source_channel: 8,
            destination_channel: None,
            source_number: 13,
            destination_endpoint: "processor".into(),
            destination_profile: profile.into(),
            destination_effect: "algorithm-1".into(),
            destination_parameter: "reflex.parameter-1".into(),
            behavior: MappingBehavior {
                source_range: (0, 127),
                destination_range: (0, 127),
                invert: false,
                curve: "linear".into(),
            },
            enabled: true,
            profile_version: 1,
        }
    }

    fn recording(id: &str, name: &str) -> (VirtualEndpoint, EndpointId) {
        let endpoint = numeric_endpoint_id(id).expect("id");
        (VirtualEndpoint::new(id, name, EndpointDirection::Output), endpoint)
    }

    #[test]
    fn unique_output_rejects_duplicates_and_ignores_hui() {
        assert!(unique_launch_control_midi_output([
            ("a", "Launch Control XL MIDI 1"),
            ("b", "Launch Control XL MIDI 1"),
        ])
        .is_err());
        let (endpoint, id) = unique_launch_control_midi_output([
            ("midi", "Launch Control XL MIDI 1"),
            ("hui", "Launch Control XL HUI"),
        ])
        .expect("unique midi");
        assert_eq!(id, "midi");
        assert_eq!(endpoint, numeric_endpoint_id("midi").expect("hashed"));
        assert!(unique_launch_control_midi_output([("x", "Midi Through")]).is_err());
    }

    #[test]
    fn unique_writable_address_ignores_hui_and_duplicates() {
        let midi = AlsaSequencerPort {
            address: AlsaSequencerAddress { client: 24, port: 0 },
            client_name: "Launch Control XL MK2".into(),
            port_name: "Launch Control XL MK2 MIDI 1".into(),
            readable: true,
            writable: true,
        };
        let hui = AlsaSequencerPort {
            address: AlsaSequencerAddress { client: 24, port: 1 },
            client_name: "Launch Control XL MK2".into(),
            port_name: "Launch Control XL MK2 HUI".into(),
            readable: true,
            writable: true,
        };
        assert_eq!(
            unique_mk2_midi_writable_address(&[midi.clone(), hui]),
            Some(AlsaSequencerAddress { client: 24, port: 0 })
        );
        let twin = AlsaSequencerPort {
            address: AlsaSequencerAddress { client: 25, port: 0 },
            client_name: "Launch Control XL MK2".into(),
            port_name: "Launch Control XL MK2 MIDI 1".into(),
            readable: true,
            writable: true,
        };
        assert_eq!(unique_mk2_midi_writable_address(&[midi, twin]), None);
    }

    #[test]
    fn compose_applies_owner_colors_and_fader_proxy_without_overwriting_buttons() {
        let mut store = ControlMappingStore::default();
        store.active.push(mapping("knob-r1-c1", "lexicon.reflex"));
        store.active.push(mapping("button-r1-c2", "eventide.micropitch"));
        store.active.push(mapping("fader-1", "lexicon.reflex"));
        store.active.push(mapping("fader-2", "lexicon.reflex"));
        let session = AssignmentSession::new("live");
        let scheduler = LedFeedbackScheduler::new(off());
        let desired = compose_desired(&store, &session, None, scheduler, 0, 0, None);
        assert_eq!(desired.get(&0).map(|state| state.color), Some(LedColor::Amber));
        assert_eq!(desired.get(&25).map(|state| state.color), Some(LedColor::Red));
        assert_eq!(desired.get(&24).map(|state| state.color), Some(LedColor::Amber));
        assert_eq!(desired.get(&32).map(|state| state.color), Some(LedColor::Amber));
        assert_eq!(desired.get(&33).map(|state| state.color), Some(LedColor::Amber));
        assert_eq!(desired.get(&1).map(|state| state.color), Some(LedColor::Off));
    }

    #[test]
    fn lexicon_effect_buttons_turn_green_when_assigned() {
        let mut store = ControlMappingStore::default();
        store.active.push(mapping("button-r1-c1", "lexicon.reflex"));
        store.active.push(mapping("button-r1-c2", "lexicon.reflex"));
        store.active[0].destination_parameter = "reflex.algorithm-1".into();
        store.active[1].destination_parameter = "reflex.algorithm-2".into();
        let desired = compose_desired(
            &store,
            &AssignmentSession::new("live"),
            None,
            LedFeedbackScheduler::new(off()),
            0,
            0,
            None,
        );
        assert_eq!(desired.get(&24).map(|state| state.color), Some(LedColor::Amber));
        assert_eq!(desired.get(&25).map(|state| state.color), Some(LedColor::Amber));
        let desired = compose_desired(
            &store,
            &AssignmentSession::new("live"),
            None,
            LedFeedbackScheduler::new(off()),
            0,
            0,
            Some(2),
        );
        assert_eq!(desired.get(&24).map(|state| state.color), Some(LedColor::Amber));
        assert_eq!(desired.get(&25).map(|state| state.color), Some(LedColor::Green));
    }

    #[test]
    fn eventide_requested_controls_have_explicit_led_policy() {
        let mut store = ControlMappingStore::default();
        for assignment in mackes_profiles::eventide_controller_assignments() {
            if assignment.parameter_id.is_some()
                && assignment.physical_control_id.starts_with("knob-r")
            {
                store.active.push(mapping(&assignment.physical_control_id, "eventide.micropitch"));
            }
        }
        store.active.push(mapping("fader-1", "eventide.micropitch"));
        store.active.push(mapping("button-r1-c1", "eventide.micropitch"));
        let session = AssignmentSession::new("live");
        let desired =
            compose_desired(&store, &session, None, LedFeedbackScheduler::new(off()), 0, 0, None);
        let catalog = launch_control_physical_catalog();
        for assignment in mackes_profiles::eventide_controller_assignments()
            .into_iter()
            .filter(|assignment| assignment.physical_control_id.starts_with("knob-r"))
        {
            let index = catalog
                .iter()
                .find(|control| control.id.as_str() == assignment.physical_control_id)
                .and_then(|control| control.feedback_address)
                .expect("Eventide knob LED address");
            assert_eq!(desired.get(&index).map(|state| state.color), Some(LedColor::Red));
        }
        assert_eq!(
            desired.get(&24).map(|state| state.color),
            Some(LedColor::Red),
            "Slider 1 Button 1 is the Eventide bypass indicator"
        );
        assert_eq!(
            desired.get(&32).map(|state| state.color),
            Some(LedColor::Red),
            "Slider 1 Button 2 shows the fader Mix proxy, not an unsupported Delay bypass"
        );
    }

    #[test]
    fn flush_sends_factory1_sysex_and_coalesces_unchanged_frames() {
        let mut outputs = OutputRegistry::new(4);
        let (adapter, _) = recording("xl-midi", "Launch Control XL MIDI 1");
        outputs.insert(Box::new(adapter)).expect("output");
        let mut surface = LedSurface::default();
        let mut store = ControlMappingStore::default();
        store.active.push(mapping("knob-r1-c1", "lexicon.reflex"));
        let session = AssignmentSession::new("live");
        surface.flush(0, &store, &session, None, &mut outputs, false);
        assert!(surface.diagnostics.sent > 0);
        assert_eq!(surface.diagnostics.template, 8);
        let first_sent = surface.diagnostics.sent;
        surface.flush(10, &store, &session, None, &mut outputs, false);
        assert_eq!(surface.diagnostics.sent, first_sent);
        assert!(surface.diagnostics.coalesced > 0);
        assert_eq!(surface.coalescer.desired(0).map(|state| state.color), Some(LedColor::Amber));
    }

    #[test]
    fn arrow_led_is_green_only_while_pressed() {
        let mut outputs = OutputRegistry::new(4);
        outputs
            .insert(Box::new(recording("xl-midi", "Launch Control XL MIDI 1").0))
            .expect("output");
        let mut surface = LedSurface::default();
        let store = ControlMappingStore::default();
        let session = AssignmentSession::new("live");

        surface.record_arrow("utility-8", true, 0);
        surface.flush(0, &store, &session, None, &mut outputs, false);
        assert_eq!(surface.coalescer.desired(47).map(|state| state.color), Some(LedColor::Green));

        surface.record_arrow("utility-8", false, 1);
        surface.flush(1, &store, &session, None, &mut outputs, false);
        assert_eq!(surface.coalescer.desired(47).map(|state| state.color), Some(LedColor::Green));
        surface.flush(120, &store, &session, None, &mut outputs, false);
        assert_eq!(surface.coalescer.desired(47).map(|state| state.color), Some(LedColor::Off));
    }

    #[test]
    fn backend_button_blinks_until_delivery_is_complete() {
        let mut outputs = OutputRegistry::new(4);
        outputs
            .insert(Box::new(recording("xl-midi", "Launch Control XL MIDI 1").0))
            .expect("output");
        let mut surface = LedSurface::default();
        let mut store = ControlMappingStore::default();
        store.active.push(mapping("button-r1-c1", "eventide.micropitch"));
        let session = AssignmentSession::new("live");

        surface.begin_backend_confirmation("button-r1-c1", false, 0);
        surface.flush(0, &store, &session, None, &mut outputs, false);
        assert_eq!(surface.coalescer.desired(24).map(|state| state.blink), Some(true));
        surface.finish_backend_confirmation(true);
        surface.flush(1_199, &store, &session, None, &mut outputs, false);
        assert_eq!(surface.coalescer.desired(24).map(|state| state.blink), Some(true));
        surface.flush(1_200, &store, &session, None, &mut outputs, false);
        assert_eq!(surface.coalescer.desired(24).map(|state| state.blink), Some(false));
        assert_eq!(surface.coalescer.desired(24).map(|state| state.color), Some(LedColor::Red));
        assert_eq!(
            surface.diagnostics.backend_confirmation.as_deref(),
            Some("delivered_unconfirmed")
        );

        surface.begin_backend_confirmation("button-r1-c1", true, 500);
        surface.finish_backend_confirmation(false);
        surface.flush(1_000, &store, &session, None, &mut outputs, false);
        assert_eq!(surface.coalescer.desired(24).map(|state| state.blink), Some(true));
        assert_eq!(surface.diagnostics.backend_confirmation.as_deref(), Some("failed"));
    }

    #[test]
    fn flush_fails_closed_without_a_unique_output() {
        let mut outputs = OutputRegistry::new(4);
        outputs.insert(Box::new(recording("one", "Launch Control XL MIDI 1").0)).expect("one");
        outputs.insert(Box::new(recording("two", "Launch Control XL MIDI 2").0)).expect("two");
        let mut surface = LedSurface::default();
        let store = ControlMappingStore::default();
        let session = AssignmentSession::new("live");
        surface.flush(0, &store, &session, None, &mut outputs, false);
        assert_eq!(surface.diagnostics.sent, 0);
        assert!(surface.diagnostics.failed > 0);
        assert_eq!(
            surface.diagnostics.last_error.as_deref(),
            Some("duplicate Launch Control XL MIDI outputs; LED writes refused")
        );
    }

    #[test]
    fn yellow_learn_and_two_green_pulses_restore_owner_color() {
        let mut store = ControlMappingStore::default();
        store.active.push(mapping("knob-r1-c1", "lexicon.reflex"));
        let mut session = AssignmentSession::new("live");
        session.phase = AssignmentPhase::ChooseParameter;
        let mut scheduler = LedFeedbackScheduler::new(owner_state("lexicon.reflex"));
        scheduler.assignment = Some(LedState::new(LedColor::Yellow, 127, true));
        let desired = compose_desired(&store, &session, Some("knob-r1-c1"), scheduler, 0, 0, None);
        assert_eq!(desired.get(&0).map(|state| state.color), Some(LedColor::Yellow));
        assert_eq!(desired.get(&40).map(|state| state.color), Some(LedColor::Yellow));
        session.phase = AssignmentPhase::Succeeded;
        scheduler.assignment = None;
        scheduler.result = Some((true, 0));
        assert_eq!(
            compose_desired(&store, &session, Some("knob-r1-c1"), scheduler, 0, 0, None)
                .get(&0)
                .map(|state| state.color),
            Some(LedColor::Green)
        );
        assert_eq!(
            compose_desired(&store, &session, Some("knob-r1-c1"), scheduler, 400, 0, None)
                .get(&0)
                .map(|state| state.color),
            Some(LedColor::Off)
        );
        assert_eq!(
            compose_desired(&store, &session, Some("knob-r1-c1"), scheduler, 800, 0, None)
                .get(&0)
                .map(|state| state.color),
            Some(LedColor::Green)
        );
        assert_eq!(
            compose_desired(&store, &session, Some("knob-r1-c1"), scheduler, 1_600, 0, None)
                .get(&0)
                .map(|state| state.color),
            Some(LedColor::Amber)
        );
        scheduler.result = Some((false, 0));
        assert_eq!(
            compose_desired(&store, &session, Some("knob-r1-c1"), scheduler, 0, 0, None)
                .get(&0)
                .map(|state| state.color),
            Some(LedColor::Red)
        );
    }

    #[test]
    fn five_minute_idle_sweep_wakes_and_restores_mapping_colors() {
        let mut outputs = OutputRegistry::new(4);
        let (adapter, _) = recording("xl-midi", "Launch Control XL MIDI 1");
        outputs.insert(Box::new(adapter)).expect("output");
        let mut surface = LedSurface::default();
        let mut store = ControlMappingStore::default();
        store.active.push(mapping("knob-r1-c1", "lexicon.reflex"));
        let session = AssignmentSession::new("live");

        surface.flush(IDLE_SLEEP_AFTER_MS - 1, &store, &session, None, &mut outputs, false);
        assert_eq!(surface.coalescer.desired(0).map(|state| state.color), Some(LedColor::Amber));

        surface.flush(
            IDLE_SLEEP_AFTER_MS + IDLE_SWEEP_STEP_MS,
            &store,
            &session,
            None,
            &mut outputs,
            false,
        );
        assert!(surface.sleep_active);
        assert_eq!(surface.coalescer.desired(0).map(|state| state.color), Some(LedColor::Off));
        assert_eq!(surface.coalescer.desired(1).map(|state| state.color), Some(LedColor::Red));

        surface.record_touch(IDLE_SLEEP_AFTER_MS + IDLE_SWEEP_STEP_MS + 1);
        surface.flush(
            IDLE_SLEEP_AFTER_MS + IDLE_SWEEP_STEP_MS + 1,
            &store,
            &session,
            None,
            &mut outputs,
            false,
        );
        assert!(!surface.sleep_active);
        assert_eq!(surface.coalescer.desired(0).map(|state| state.color), Some(LedColor::Amber));
        assert_eq!(surface.coalescer.desired(1).map(|state| state.color), Some(LedColor::Off));
    }
}
