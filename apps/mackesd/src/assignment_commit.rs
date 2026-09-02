//! Typed assignment commit mapping construction and source-role checks.

/// Builds a durable mapping from a complete assignment commit request.
///
/// Preset destinations (`pcm70_reflex:*`) are accepted only from channel buttons.
pub fn mapping_from_request(
    request: &mackes_ipc::AssignmentRequest,
    catalog: &mackes_ipc::AssignmentCatalog,
) -> Result<mackes_config::ControlMapping, String> {
    let control_id =
        request.physical_control_id.as_deref().ok_or("assignment commit requires a control")?;
    let control = mackes_profiles::launch_control_physical_catalog()
        .into_iter()
        .find(|item| item.id.as_str() == control_id)
        .ok_or_else(|| "assignment control is reserved or unknown".to_owned())?;
    let destination_parameter = request
        .destination_parameter
        .clone()
        .ok_or("assignment commit requires a complete destination")?;
    if destination_parameter.starts_with("pcm70_reflex:")
        && control.role != mackes_profiles::PhysicalControlRole::ChannelButton
    {
        return Err("preset destinations require a channel button".into());
    }
    let layout = mackes_profiles::launch_control_mk2_factory1_layout()
        .into_iter()
        .find(|item| item.physical_control_id == control_id);
    let source_channel =
        catalog.source_channel.or_else(|| layout.as_ref().map(|item| item.channel)).unwrap_or(8);
    let source_number = catalog
        .source_number
        .or_else(|| layout.as_ref().map(|item| item.source_number))
        .or(control.source_address)
        .unwrap_or_default();
    let source_endpoint = catalog
        .source_endpoint
        .clone()
        .filter(|id| !id.is_empty() && id != "controller")
        .ok_or_else(|| "assignment commit requires a captured source endpoint".to_owned())?;
    let destination_endpoint = catalog
        .destination_endpoint
        .clone()
        .filter(|id| !id.is_empty() && id != "processor")
        .ok_or_else(|| "assignment commit requires a selected destination endpoint".to_owned())?;
    Ok(mackes_config::ControlMapping {
        id: format!("assignment-{control_id}"),
        controller_profile: "launch-control-xl-mk2".into(),
        physical_control_id: control_id.into(),
        source_endpoint,
        source_kind: if control.role == mackes_profiles::PhysicalControlRole::ChannelButton {
            "note".into()
        } else {
            "cc".into()
        },
        source_channel,
        source_number,
        destination_endpoint,
        destination_profile: request
            .destination_profile
            .clone()
            .ok_or("assignment commit requires a complete destination")?,
        destination_effect: request
            .destination_effect
            .clone()
            .ok_or("assignment commit requires a complete destination")?,
        destination_parameter,
        behavior: mackes_config::MappingBehavior {
            source_range: (0, 127),
            destination_range: (0, 127),
            invert: false,
            curve: "linear".into(),
        },
        enabled: true,
        profile_version: 1,
    })
}

/// Returns whether a persisted mapping obeys preset/button role rules.
#[must_use]
pub fn mapping_role_compatible(mapping: &mackes_config::ControlMapping) -> bool {
    if !mapping.destination_parameter.starts_with("pcm70_reflex:") {
        return true;
    }
    mackes_profiles::launch_control_physical_catalog().iter().any(|item| {
        item.id.as_str() == mapping.physical_control_id
            && item.role == mackes_profiles::PhysicalControlRole::ChannelButton
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> mackes_ipc::AssignmentCatalog {
        mackes_ipc::AssignmentCatalog {
            source_endpoint: Some("launch-control-xl-mk2".into()),
            destination_endpoint: Some("lexicon-reflex".into()),
            source_channel: Some(8),
            ..mackes_ipc::AssignmentCatalog::default()
        }
    }

    fn request(control: &str, parameter: &str) -> mackes_ipc::AssignmentRequest {
        mackes_ipc::AssignmentRequest {
            generation: 0,
            action: mackes_ipc::AssignmentAction::Commit,
            physical_control_id: Some(control.into()),
            destination_profile: Some("lexicon.reflex".into()),
            destination_effect: Some("reverb".into()),
            destination_parameter: Some(parameter.into()),
        }
    }

    #[test]
    fn channel_button_can_commit_reflex_preset() {
        let mapping =
            mapping_from_request(&request("button-r1-c1", "pcm70_reflex:concert-wave"), &catalog())
                .expect("ok");
        assert_eq!(mapping.source_kind, "note");
        assert_eq!(mapping.source_channel, 8);
        assert_eq!(mapping.source_endpoint, "launch-control-xl-mk2");
        assert_eq!(mapping.destination_endpoint, "lexicon-reflex");
        assert_eq!(mapping.destination_parameter, "pcm70_reflex:concert-wave");
        assert!(mapping_role_compatible(&mapping));
    }

    #[test]
    fn knob_and_fader_cannot_commit_reflex_preset() {
        assert!(mapping_from_request(
            &request("knob-r1-c1", "pcm70_reflex:concert-wave"),
            &catalog()
        )
        .is_err());
        assert!(mapping_from_request(&request("fader-1", "pcm70_reflex:concert-wave"), &catalog())
            .is_err());
    }

    #[test]
    fn knob_can_commit_continuous_parameter() {
        let mapping =
            mapping_from_request(&request("knob-r1-c1", "reflex.parameter-1"), &catalog())
                .expect("ok");
        assert_eq!(mapping.source_kind, "cc");
        assert!(mapping_role_compatible(&mapping));
    }

    #[test]
    fn persisted_knob_preset_mapping_is_rejected_on_reload() {
        let mapping = mackes_config::ControlMapping {
            id: "bad-preset".into(),
            controller_profile: "launch-control-xl-mk2".into(),
            physical_control_id: "knob-r1-c1".into(),
            source_endpoint: "controller".into(),
            source_kind: "cc".into(),
            source_channel: 0,
            source_number: 13,
            destination_endpoint: "processor".into(),
            destination_profile: "lexicon.reflex".into(),
            destination_effect: "reverb".into(),
            destination_parameter: "pcm70_reflex:concert-wave".into(),
            behavior: mackes_config::MappingBehavior {
                source_range: (0, 127),
                destination_range: (0, 127),
                invert: false,
                curve: "linear".into(),
            },
            enabled: true,
            profile_version: 1,
        };
        assert!(!mapping_role_compatible(&mapping));
    }
}
