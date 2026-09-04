//! Deterministic Learn catalog rendering for every assignment level.

use crate::{assignment_wizard_lines, AssignmentChoiceBrowser, AssignmentWizard, Viewport};

/// Renders the current catalog level, counts, selected item, and breadcrumb.
#[must_use]
pub fn assignment_catalog_lines(
    wizard: &AssignmentWizard,
    choices: &AssignmentChoiceBrowser,
    viewport: Viewport,
) -> Vec<String> {
    let mut lines = assignment_wizard_lines(wizard, viewport);
    if wizard.session.phase == mackes_ipc::AssignmentPhase::Idle {
        return lines;
    }
    let step = match wizard.session.phase {
        mackes_ipc::AssignmentPhase::ChooseDevice => 2,
        mackes_ipc::AssignmentPhase::ChooseEffect => 3,
        mackes_ipc::AssignmentPhase::ChooseParameter => 4,
        mackes_ipc::AssignmentPhase::ConfirmReplace
        | mackes_ipc::AssignmentPhase::Committing
        | mackes_ipc::AssignmentPhase::Succeeded
        | mackes_ipc::AssignmentPhase::Failed => 5,
        _ => 1,
    };
    let lcd = vec![
        format!("DEVICE  {step}/5"),
        match wizard.session.phase {
            mackes_ipc::AssignmentPhase::AwaitControl => "MOVE CONTROL".into(),
            mackes_ipc::AssignmentPhase::ChooseDevice => "CHOOSE DEVICE".into(),
            mackes_ipc::AssignmentPhase::ChooseEffect => "CHOOSE EFFECT".into(),
            mackes_ipc::AssignmentPhase::ChooseParameter => "CHOOSE PARAM".into(),
            mackes_ipc::AssignmentPhase::ConfirmReplace => "REPLACE? ENTER".into(),
            mackes_ipc::AssignmentPhase::Committing => "SAVING...".into(),
            mackes_ipc::AssignmentPhase::Succeeded => "ASSIGNED".into(),
            mackes_ipc::AssignmentPhase::Failed => "FAILED / RETRY".into(),
            _ => "BACK / CANCEL".into(),
        },
    ];
    lines.splice(0..0, lcd);
    lines.push("CATALOG  Device > Preset > Effect > Type > Parameter".into());
    lines.push(format!(
        "LEVEL    {}  COUNT {}",
        catalog_level_name(wizard.session.phase),
        catalog_count(wizard.session.phase, choices)
    ));
    lines.push(format!("SELECTED {}", catalog_selected(wizard.session.phase, choices)));
    if choices.presets.is_empty() {
        lines.push("PRESETS  NONE".into());
    }
    match wizard.session.phase {
        mackes_ipc::AssignmentPhase::ChooseDevice => {
            push_marked(&mut lines, "SELECT DEVICE", &choices.devices, choices.selected);
        }
        mackes_ipc::AssignmentPhase::ChoosePreset => {
            let labels: Vec<String> =
                choices.presets.iter().map(|(_, label)| label.clone()).collect();
            push_marked(&mut lines, "SELECT PRESET", &labels, choices.selected);
        }
        mackes_ipc::AssignmentPhase::ChooseEffect => {
            let labels: Vec<String> =
                choices.effects.iter().map(|(_, label)| label.clone()).collect();
            push_marked(&mut lines, "SELECT EFFECT", &labels, choices.selected);
        }
        mackes_ipc::AssignmentPhase::ChooseType => {
            push_marked(&mut lines, "SELECT TYPE", &choices.types, choices.selected);
        }
        mackes_ipc::AssignmentPhase::ChooseParameter => {
            let labels: Vec<String> = choices
                .parameters
                .iter()
                .map(|choice| format!("{} / {}", choice.effect_label, choice.label))
                .collect();
            push_marked(&mut lines, "SELECT PARAMETER", &labels, choices.selected);
        }
        _ => {}
    }
    let width = usize::from(viewport.width);
    let height = usize::from(viewport.height).max(1);
    lines.into_iter().map(|line| line.chars().take(width).collect()).take(height).collect()
}

const fn catalog_level_name(phase: mackes_ipc::AssignmentPhase) -> &'static str {
    match phase {
        mackes_ipc::AssignmentPhase::ChooseDevice => "Device",
        mackes_ipc::AssignmentPhase::ChoosePreset => "Preset",
        mackes_ipc::AssignmentPhase::ChooseEffect => "Effect",
        mackes_ipc::AssignmentPhase::ChooseType => "Type",
        mackes_ipc::AssignmentPhase::ChooseParameter => "Parameter",
        mackes_ipc::AssignmentPhase::AwaitControl => "Control",
        _ => "Assignment",
    }
}

fn catalog_count(phase: mackes_ipc::AssignmentPhase, choices: &AssignmentChoiceBrowser) -> usize {
    match phase {
        mackes_ipc::AssignmentPhase::ChooseDevice => choices.devices.len(),
        mackes_ipc::AssignmentPhase::ChoosePreset => choices.presets.len(),
        mackes_ipc::AssignmentPhase::ChooseEffect => choices.effects.len(),
        mackes_ipc::AssignmentPhase::ChooseType => choices.types.len(),
        mackes_ipc::AssignmentPhase::ChooseParameter => choices.parameters.len(),
        _ => 0,
    }
}

fn catalog_selected(
    phase: mackes_ipc::AssignmentPhase,
    choices: &AssignmentChoiceBrowser,
) -> String {
    match phase {
        mackes_ipc::AssignmentPhase::ChooseDevice => {
            choices.devices.get(choices.selected).cloned().unwrap_or_else(|| "NONE".into())
        }
        mackes_ipc::AssignmentPhase::ChoosePreset => choices
            .presets
            .get(choices.selected)
            .map_or_else(|| "NONE".into(), |(_, label)| label.clone()),
        mackes_ipc::AssignmentPhase::ChooseEffect => choices
            .effects
            .get(choices.selected)
            .map_or_else(|| "NONE".into(), |(_, label)| label.clone()),
        mackes_ipc::AssignmentPhase::ChooseType => {
            choices.types.get(choices.selected).cloned().unwrap_or_else(|| "NONE".into())
        }
        mackes_ipc::AssignmentPhase::ChooseParameter => choices
            .parameters
            .get(choices.selected)
            .map_or_else(|| "NONE".into(), |choice| choice.label.clone()),
        _ => "NONE".into(),
    }
}

fn push_marked(lines: &mut Vec<String>, heading: &str, items: &[String], selected: usize) {
    lines.push(heading.to_owned());
    if items.is_empty() {
        lines.push("  NONE".into());
        return;
    }
    for (index, item) in items.iter().enumerate() {
        lines.push(format!("{} {item}", if index == selected { ">" } else { " " }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppSection;

    fn wizard(phase: mackes_ipc::AssignmentPhase) -> AssignmentWizard {
        AssignmentWizard {
            session: mackes_ipc::AssignmentSession {
                phase,
                prior_screen: "Map Controls".into(),
                index: 0,
                total: 2,
                has_draft: true,
                interrupted_phase: None,
                cursors: mackes_ipc::AssignmentCursors::default(),
                catalog: mackes_ipc::AssignmentCatalog::default(),
            },
            prior_section: AppSection::MapControls,
            candidates: vec!["knob-r1-c1".into()],
            generation: 1,
        }
    }

    #[test]
    fn catalog_renders_every_level_with_count_and_selection() {
        let choices = AssignmentChoiceBrowser::from_profiles(
            &["lexicon.reflex"],
            Some("lexicon.reflex"),
            mackes_profiles::SourceRole::Continuous,
        );
        let expected = [
            (mackes_ipc::AssignmentPhase::ChooseDevice, "LEVEL    Device", "SELECT DEVICE"),
            (mackes_ipc::AssignmentPhase::ChoosePreset, "LEVEL    Preset", "SELECT PRESET"),
            (mackes_ipc::AssignmentPhase::ChooseEffect, "LEVEL    Effect", "SELECT EFFECT"),
            (mackes_ipc::AssignmentPhase::ChooseType, "LEVEL    Type", "SELECT TYPE"),
            (
                mackes_ipc::AssignmentPhase::ChooseParameter,
                "LEVEL    Parameter",
                "SELECT PARAMETER",
            ),
        ];
        for (phase, level, heading) in expected {
            let lines = assignment_catalog_lines(&wizard(phase), &choices, Viewport::new(80, 24));
            assert!(lines.iter().any(|line| line.contains("CATALOG  Device > Preset")));
            assert!(lines.iter().any(|line| line.contains(level)));
            assert!(lines.iter().any(|line| line.contains("COUNT")));
            assert!(lines.iter().any(|line| line.contains("SELECTED")));
            assert!(lines.iter().any(|line| line.contains(heading)));
            assert!(lines.iter().any(|line| line.starts_with("> ")));
            assert!(!lines.iter().any(|line| line.contains("PRESETS  NONE")));
        }
    }

    #[test]
    fn catalog_shows_presets_none_for_eventide() {
        let choices = AssignmentChoiceBrowser::from_profiles(
            &["eventide.micropitch"],
            Some("eventide.micropitch"),
            mackes_profiles::SourceRole::Continuous,
        );
        let lines = assignment_catalog_lines(
            &wizard(mackes_ipc::AssignmentPhase::ChoosePreset),
            &choices,
            Viewport::new(80, 24),
        );
        assert!(lines.iter().any(|line| line.contains("PRESETS  NONE")));
        assert!(lines.iter().any(|line| line.contains("CHOOSE PRESET")));
        assert!(lines.iter().any(|line| line.contains("SELECT PRESET")));
    }
}
