//! Daemon-owned Learn catalog: filtering, selection, and commit payloads.

use mackes_ipc::{AssignmentCatalogEntry, AssignmentPhase, AssignmentSession};
use mackes_profiles::{PhysicalControlRole, SourceRole};

/// Maximum catalog rows published in one snapshot.
const MAX_ROWS: usize = 64;

/// Rebuilds catalog rows from live profile inventory and locked selections.
#[allow(clippy::too_many_lines)]
pub fn refresh(session: &mut AssignmentSession, profile_ids: &[String]) {
    let control_id = session.catalog.captured_control_id.clone();
    let role = captured_role(control_id.as_deref());
    let source_role = match role {
        Some(PhysicalControlRole::ChannelButton) => SourceRole::ButtonAction,
        _ => SourceRole::Continuous,
    };
    let devices = profile_ids
        .iter()
        .filter(|id| mackes_profiles::builtin_profile(id).is_some())
        .map(|id| entry(id, id, None))
        .take(MAX_ROWS)
        .collect::<Vec<_>>();
    if session
        .catalog
        .selected_device
        .as_ref()
        .is_none_or(|id| !devices.iter().any(|row| &row.id == id))
    {
        session.catalog.selected_device = devices.first().map(|row| row.id.clone());
    }
    let profile_id = session.catalog.selected_device.clone();
    let profile = profile_id.as_deref().and_then(mackes_profiles::builtin_profile);
    let mut presets = Vec::new();
    if profile_id.as_deref() == Some("lexicon.reflex") {
        presets.extend(
            mackes_profiles::lexicon_reflex::pcm70_translations()
                .iter()
                .map(|preset| entry(preset.id, preset.name, Some("reverb"))),
        );
    }
    presets.insert(0, entry("NONE", "NONE", None));
    presets.truncate(MAX_ROWS);
    let mut effects = Vec::new();
    let mut types = Vec::new();
    let mut parameters = Vec::new();
    if let Some(profile) = profile {
        let blocks = mackes_profiles::effect_blocks(&profile);
        effects = blocks
            .iter()
            .map(|block| entry(&block.id, &block.label, None))
            .take(MAX_ROWS)
            .collect();
        if session
            .catalog
            .selected_effect
            .as_ref()
            .is_none_or(|id| !effects.iter().any(|row| &row.id == id))
        {
            session.catalog.selected_effect = effects.first().map(|row| row.id.clone());
        }
        let mut rows: Vec<AssignmentCatalogEntry> =
            mackes_profiles::compatible_parameters(&profile, source_role, true)
                .into_iter()
                .filter(|item| {
                    matches!(
                        item.reason,
                        mackes_profiles::SupportReason::Compatible
                            | mackes_profiles::SupportReason::Experimental
                    )
                })
                .map(|item| {
                    let block = blocks.iter().find(|block| {
                        block.parameters.iter().any(|parameter| parameter.id == item.parameter.id)
                    });
                    let effect_id =
                        block.map_or_else(|| "general".to_owned(), |block| block.id.clone());
                    entry(item.parameter.id, item.parameter.label, Some(&effect_id))
                })
                .collect();
        if rows.is_empty() && profile_id.as_deref() == Some("lexicon.reflex") {
            rows = if source_role == SourceRole::Continuous {
                mackes_profiles::lexicon_reflex::parameters(1)
                    .iter()
                    .map(|parameter| {
                        entry(
                            format!("reflex.parameter-{}", parameter.number),
                            format!("{} ({})", parameter.description, parameter.mrc_name),
                            Some("reverb"),
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            };
        }
        if let Some(effect_id) = session.catalog.selected_effect.as_deref() {
            rows.retain(|row| row.group.as_deref() == Some(effect_id) || row.group.is_none());
        }
        for row in &rows {
            let label = row.group.clone().unwrap_or_else(|| "General".into());
            if !types.iter().any(|item: &AssignmentCatalogEntry| item.label == label) {
                types.push(entry(&label, &label, None));
            }
        }
        if session
            .catalog
            .selected_type
            .as_ref()
            .is_none_or(|id| !types.iter().any(|row| &row.id == id))
        {
            session.catalog.selected_type = types.first().map(|row| row.id.clone());
        }
        if let Some(kind) = session.catalog.selected_type.as_deref() {
            rows.retain(|row| row.group.as_deref() == Some(kind) || row.label.contains(kind));
        }
        parameters = rows.into_iter().take(MAX_ROWS).collect();
        types.truncate(MAX_ROWS);
    }
    session.catalog.devices = devices;
    session.catalog.presets = presets;
    session.catalog.effects = effects;
    session.catalog.types = types;
    session.catalog.parameters = parameters;
    if control_id.is_some() {
        session.catalog.captured_control_id = control_id;
        session.catalog.captured_role = role.map(|role| format!("{role:?}"));
        if let Some(layout) = layout_for(session.catalog.captured_control_id.as_deref()) {
            session.catalog.source_channel = Some(layout.channel);
            session.catalog.source_number = Some(layout.source_number);
        }
    }
    session.catalog.breadcrumb = match session.phase {
        AssignmentPhase::ChooseDevice => "Device".into(),
        AssignmentPhase::ChoosePreset => "Device > Preset".into(),
        AssignmentPhase::ChooseEffect => "Device > Preset > Effect".into(),
        AssignmentPhase::ChooseType => "Device > Preset > Effect > Type".into(),
        AssignmentPhase::ChooseParameter => "Device > Preset > Effect > Type > Parameter".into(),
        _ => "Assignment".into(),
    };
    let total = match session.phase {
        AssignmentPhase::ChooseDevice => session.catalog.devices.len(),
        AssignmentPhase::ChoosePreset => session.catalog.presets.len(),
        AssignmentPhase::ChooseEffect => session.catalog.effects.len(),
        AssignmentPhase::ChooseType => session.catalog.types.len(),
        AssignmentPhase::ChooseParameter => session.catalog.parameters.len(),
        _ => 0,
    };
    session.set_total(u16::try_from(total).unwrap_or(u16::MAX));
}

/// Locks the active-level selection from the current cursor before advancing.
pub fn lock_active_selection(session: &mut AssignmentSession) {
    let index = usize::from(session.active_cursor());
    match session.phase {
        AssignmentPhase::ChooseDevice => {
            session.catalog.selected_device =
                session.catalog.devices.get(index).map(|row| row.id.clone());
            session.catalog.selected_preset = None;
            session.catalog.selected_effect = None;
            session.catalog.selected_type = None;
            session.catalog.selected_parameter = None;
        }
        AssignmentPhase::ChoosePreset => {
            session.catalog.selected_preset =
                session.catalog.presets.get(index).map(|row| row.id.clone());
        }
        AssignmentPhase::ChooseEffect => {
            session.catalog.selected_effect =
                session.catalog.effects.get(index).map(|row| row.id.clone());
            session.catalog.selected_type = None;
            session.catalog.selected_parameter = None;
        }
        AssignmentPhase::ChooseType => {
            session.catalog.selected_type =
                session.catalog.types.get(index).map(|row| row.id.clone());
            session.catalog.selected_parameter = None;
        }
        AssignmentPhase::ChooseParameter => {
            session.catalog.selected_parameter =
                session.catalog.parameters.get(index).map(|row| row.id.clone());
        }
        _ => {}
    }
}

/// Returns a complete commit payload from authoritative catalog state.
#[must_use]
pub fn commit_from_catalog(
    session: &AssignmentSession,
) -> Option<(String, String, String, String)> {
    let control = session.catalog.captured_control_id.clone()?;
    let profile = session.catalog.selected_device.clone()?;
    let button = session.catalog.captured_role.as_deref() == Some("ChannelButton");
    if button {
        if let Some(preset) = session.catalog.selected_preset.as_deref().filter(|id| *id != "NONE")
        {
            return Some((control, profile, "reverb".into(), format!("pcm70_reflex:{preset}")));
        }
    }
    let parameter = session.catalog.selected_parameter.clone().or_else(|| {
        session
            .catalog
            .parameters
            .get(usize::from(session.cursors.parameter))
            .map(|row| row.id.clone())
    })?;
    let effect = session.catalog.selected_effect.clone().or_else(|| {
        session
            .catalog
            .parameters
            .iter()
            .find(|row| row.id == parameter)
            .and_then(|row| row.group.clone())
    })?;
    Some((control, profile, effect, parameter))
}

/// Returns whether a button session should commit from the preset level.
#[must_use]
pub fn button_preset_ready(session: &AssignmentSession) -> bool {
    session.phase == AssignmentPhase::ChoosePreset
        && captured_role(session.catalog.captured_control_id.as_deref())
            == Some(PhysicalControlRole::ChannelButton)
        && session.catalog.selected_preset.as_deref().is_some_and(|id| id != "NONE")
}

/// Returns whether a continuous control attempted an invalid preset commit.
#[must_use]
pub fn continuous_preset_forbidden(session: &AssignmentSession) -> bool {
    session.phase == AssignmentPhase::ChoosePreset
        && session.catalog.captured_role.as_deref() != Some("ChannelButton")
        && session.catalog.selected_preset.as_deref().is_some_and(|id| id != "NONE")
}

fn captured_role(control_id: Option<&str>) -> Option<PhysicalControlRole> {
    let id = control_id?;
    mackes_profiles::launch_control_physical_catalog()
        .into_iter()
        .find(|control| control.id.as_str() == id)
        .map(|control| control.role)
}

fn layout_for(control_id: Option<&str>) -> Option<mackes_profiles::LaunchControlLayoutControl> {
    let id = control_id?;
    mackes_profiles::launch_control_mk2_factory1_layout()
        .into_iter()
        .find(|control| control.physical_control_id == id)
}

fn entry(
    id: impl Into<String>,
    label: impl Into<String>,
    group: Option<&str>,
) -> AssignmentCatalogEntry {
    AssignmentCatalogEntry { id: id.into(), label: label.into(), group: group.map(str::to_owned) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mackes_ipc::AssignmentAction;

    #[test]
    fn catalog_filters_effect_and_type_and_commits_selected_ids() {
        let mut session = AssignmentSession::new("map");
        session.catalog.captured_control_id = Some("knob-r1-c1".into());
        session.phase = AssignmentPhase::ChooseDevice;
        refresh(&mut session, &["lexicon.reflex".into(), "eventide.micropitch".into()]);
        assert!(session.catalog.devices.iter().any(|row| row.id == "lexicon.reflex"));
        session.apply(AssignmentAction::Down);
        lock_active_selection(&mut session);
        refresh(&mut session, &["lexicon.reflex".into(), "eventide.micropitch".into()]);
        session.phase = AssignmentPhase::ChoosePreset;
        session.catalog.selected_preset = Some("NONE".into());
        session.phase = AssignmentPhase::ChooseEffect;
        refresh(&mut session, &["lexicon.reflex".into()]);
        lock_active_selection(&mut session);
        session.phase = AssignmentPhase::ChooseType;
        refresh(&mut session, &["lexicon.reflex".into()]);
        lock_active_selection(&mut session);
        session.phase = AssignmentPhase::ChooseParameter;
        refresh(&mut session, &["lexicon.reflex".into()]);
        lock_active_selection(&mut session);
        let (control, profile, effect, parameter) = commit_from_catalog(&session).expect("commit");
        assert_eq!(control, "knob-r1-c1");
        assert_eq!(profile, "lexicon.reflex");
        assert!(!effect.is_empty());
        assert!(!parameter.is_empty());
        assert_eq!(session.catalog.source_channel, Some(8));
        assert_eq!(session.catalog.source_number, Some(13));
    }

    #[test]
    fn eventide_exposes_explicit_preset_none() {
        let mut session = AssignmentSession::new("map");
        session.catalog.captured_control_id = Some("fader-1".into());
        session.catalog.selected_device = Some("eventide.micropitch".into());
        session.phase = AssignmentPhase::ChoosePreset;
        refresh(&mut session, &["eventide.micropitch".into()]);
        assert_eq!(session.catalog.presets[0].id, "NONE");
        assert!(session.catalog.presets.iter().all(|row| row.id == "NONE"));
    }
}
