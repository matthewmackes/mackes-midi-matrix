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
const FACTORY1_LED_TEMPLATE: u8 = 8;

const LED_SEND_ATTEMPTS: u8 = 3;
const LED_FLUSH_LIMIT: usize = 48;

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
    diagnostics: LedDiagnostics,
}

impl Default for LedSurface {
    fn default() -> Self {
        Self {
            scheduler: LedFeedbackScheduler::new(LedState::new(LedColor::Off, 0, false)),
            coalescer: LedCoalescer::default(),
            result_started_ms: 0,
            diagnostics: LedDiagnostics::new(),
        }
    }
}

impl LedSurface {
    /// Forces a complete replay of desired state on the next flush.
    pub fn request_full_resync(&mut self) {
        self.coalescer.request_full_resync();
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
        let desired = compose_desired(
            mappings,
            session,
            captured,
            self.scheduler,
            now_ms,
            self.result_started_ms,
        );
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
            desired.insert(index, owner_state(&mapping.destination_profile));
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
        let color = owner_state(&mapping.destination_profile);
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
        let desired = compose_desired(&store, &session, None, scheduler, 0, 0);
        assert_eq!(desired.get(&0).map(|state| state.color), Some(LedColor::Amber));
        assert_eq!(desired.get(&25).map(|state| state.color), Some(LedColor::Red));
        assert_eq!(desired.get(&24).map(|state| state.color), Some(LedColor::Amber));
        assert_eq!(desired.get(&32).map(|state| state.color), Some(LedColor::Amber));
        assert_eq!(desired.get(&33).map(|state| state.color), Some(LedColor::Amber));
        assert_eq!(desired.get(&1).map(|state| state.color), Some(LedColor::Off));
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
        let desired = compose_desired(&store, &session, Some("knob-r1-c1"), scheduler, 0, 0);
        assert_eq!(desired.get(&0).map(|state| state.color), Some(LedColor::Yellow));
        assert_eq!(desired.get(&40).map(|state| state.color), Some(LedColor::Yellow));
        session.phase = AssignmentPhase::Succeeded;
        scheduler.assignment = None;
        scheduler.result = Some((true, 0));
        assert_eq!(
            compose_desired(&store, &session, Some("knob-r1-c1"), scheduler, 0, 0)
                .get(&0)
                .map(|state| state.color),
            Some(LedColor::Green)
        );
        assert_eq!(
            compose_desired(&store, &session, Some("knob-r1-c1"), scheduler, 400, 0)
                .get(&0)
                .map(|state| state.color),
            Some(LedColor::Off)
        );
        assert_eq!(
            compose_desired(&store, &session, Some("knob-r1-c1"), scheduler, 800, 0)
                .get(&0)
                .map(|state| state.color),
            Some(LedColor::Green)
        );
        assert_eq!(
            compose_desired(&store, &session, Some("knob-r1-c1"), scheduler, 1_600, 0)
                .get(&0)
                .map(|state| state.color),
            Some(LedColor::Amber)
        );
        scheduler.result = Some((false, 0));
        assert_eq!(
            compose_desired(&store, &session, Some("knob-r1-c1"), scheduler, 0, 0)
                .get(&0)
                .map(|state| state.color),
            Some(LedColor::Red)
        );
    }
}
