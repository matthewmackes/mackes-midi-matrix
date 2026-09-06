//! Characterization tests for public crate behavior.

use super::*;

fn endpoint(serial: &str, name: &str) -> ObservedEndpoint {
    ObservedEndpoint {
        name: name.into(),
        direction: "in".into(),
        vid_pid: Some((1, 2)),
        serial: Some(serial.into()),
        interface: Some(0),
        physical_path: None,
    }
}

#[test]
fn device_profile_validates_and_round_trips() {
    let profile = DeviceProfile {
        id: "lexicon.reflex".into(),
        version: 1,
        name: "Lexicon Reflex".into(),
        effect_type: EffectType::Reverb,
        identity_probes: Vec::new(),
        provided_capabilities: vec!["reverb".into()],
        capabilities: vec![CapabilityDefinition {
            id: "midi-control".into(),
            transport: ControlTransport::Midi,
            unsafe_on_connect: false,
        }],
        controls: vec![ControlDefinition {
            label: "Mix".into(),
            cc: Some(12),
            program: None,
            range: (0, 127),
            operation: None,
        }],
        queries: Vec::new(),
        replies: Vec::new(),
        templates: Vec::new(),
        max_message_size: 1024,
        documented_features: Vec::new(),
    };
    profile.validate().expect("valid profile");
    let encoded = serde_json::to_vec(&profile).expect("encode");
    assert_eq!(serde_json::from_slice::<DeviceProfile>(&encoded).expect("decode"), profile);
}

#[test]
fn identity_probes_are_masked_and_bounded() {
    let probes = [IdentityProbe { offset: 1, value: vec![0x10, 0x20], mask: vec![0xF0, 0xF0] }];
    assert!(match_identity_probes(&probes, &[0, 0x1F, 0x2A]));
    assert!(!match_identity_probes(&probes, &[0, 0x0F, 0x2A]));
    assert!(!match_identity_probes(&probes, &[0, 0x1F]));
    assert!(!match_identity_probes(
        &[IdentityProbe { offset: 0, value: vec![1], mask: vec![] }],
        &[1]
    ));
}

#[test]
fn query_definitions_require_bounded_correlated_replies() {
    let replies = [ReplyDefinition { id: "state".into(), value: vec![0x01], mask: vec![0x7F] }];
    let queries = [QueryDefinition {
        id: "read-state".into(),
        request: vec![0x10],
        template_id: None,
        reply_id: "state".into(),
    }];
    assert_eq!(validate_query_definitions(&queries, &replies, 8), Ok(()));
    assert!(validate_query_definitions(
        &[QueryDefinition { reply_id: "missing".into(), ..queries[0].clone() }],
        &replies,
        8
    )
    .is_err());
    assert!(validate_query_definitions(
        &[QueryDefinition { request: vec![0x80], ..queries[0].clone() }],
        &replies,
        8
    )
    .is_err());
    assert!(validate_query_definitions(&queries, &[], 8).is_err());
}

#[test]
fn profile_rejects_duplicate_query_and_reply_ids() {
    let mut profile = eventide_micropitch_profile();
    profile.replies = vec![
        ReplyDefinition { id: "state".into(), value: vec![1], mask: vec![127] },
        ReplyDefinition { id: "state".into(), value: vec![2], mask: vec![127] },
    ];
    assert_eq!(profile.validate(), Err("duplicate reply ID"));
    profile.replies.pop();
    profile.queries = vec![
        QueryDefinition {
            id: "read".into(),
            request: vec![16],
            template_id: None,
            reply_id: "state".into(),
        },
        QueryDefinition {
            id: "read".into(),
            request: vec![17],
            template_id: None,
            reply_id: "state".into(),
        },
    ];
    assert_eq!(profile.validate(), Err("duplicate query ID"));
}

#[test]
fn profile_catalog_versions_user_profiles_and_reserves_reflex() {
    let catalog = ProfileCatalog::load(Vec::new()).expect("built-ins");
    assert_eq!(catalog.profiles().len(), 2);
    assert_eq!(catalog.get("eventide.micropitch").map(|profile| profile.version), Some(1));

    let mut newer = eventide_micropitch_profile();
    newer.version = 2;
    newer.name = "Customized MicroPitch".into();
    let catalog = ProfileCatalog::load(vec![newer.clone()]).expect("newer user version");
    assert_eq!(
        catalog.get("eventide.micropitch").map(|profile| profile.name.as_str()),
        Some("Customized MicroPitch")
    );
    assert!(ProfileCatalog::load(vec![eventide_micropitch_profile()]).is_err());
    assert!(ProfileCatalog::load(vec![newer.clone(), newer]).is_err());

    for reserved in ["lexicon.reflex", "lexicon-reflex-rev1", "Reflex Rev1"] {
        let mut claimed = eventide_micropitch_profile();
        claimed.id = reserved.into();
        claimed.version = 99;
        assert!(ProfileCatalog::load(vec![claimed]).is_err());
    }
}

#[test]
fn eventide_micropitch_profile_includes_documented_controls() {
    let profile = eventide_micropitch_profile();
    assert_eq!(profile.id, "eventide.micropitch");
    assert_eq!(profile.effect_type, EffectType::Modulation);
    assert!(profile.validate().is_ok());
    assert_eq!(
        profile.controls.iter().map(|control| control.cc).collect::<Vec<_>>(),
        vec![
            Some(4),
            Some(9),
            Some(14),
            Some(15),
            Some(20),
            Some(21),
            Some(22),
            Some(23),
            Some(24),
            Some(25),
            Some(26),
            Some(27),
            Some(28),
            Some(29),
            Some(30),
            Some(31),
            None,
        ]
    );
    assert_eq!(
        profile.controls.iter().map(|control| control.label.as_str()).collect::<Vec<_>>(),
        vec![
            "Expression Pedal",
            "TAP TEMPO",
            "ACTIVE/BYPASS",
            "FLEX",
            "Mix",
            "Pitch A",
            "Pitch B",
            "Depth",
            "Rate/Sens",
            "Pitch Mix",
            "Tone",
            "Delay A",
            "Delay B",
            "Mod",
            "Feedback",
            "Out Lvl",
            "Preset 1",
        ]
    );
    assert!(!profile.controls.iter().any(|control| control.cc == Some(2)));
    assert_eq!(profile.controls[16].program, Some(1));
}

#[test]
fn eventide_active_bypass_accepts_button_toggle_sources() {
    let profile = eventide_micropitch_profile();
    let choices = compatible_parameters(&profile, SourceRole::ButtonToggle, true);
    let active = choices.iter().find(|choice| choice.parameter.label == "ACTIVE/BYPASS");
    assert_eq!(active.map(|choice| choice.reason), Some(SupportReason::Compatible));
}

#[test]
fn approved_eventide_controller_layout_covers_knobs_mix_and_safe_bypass() {
    let assignments = eventide_controller_assignments();
    assert_eq!(assignments.len(), 14);
    assert_eq!(assignments[0].physical_control_id, "knob-r1-c1");
    assert_eq!(assignments[0].label, "Pitch A");
    assert_eq!(assignments[1].label, "Pitch B");
    assert_eq!(assignments[2].label, "Delay A");
    assert_eq!(assignments[3].label, "Delay B");
    assert_eq!(assignments[4].label, "Feedback");
    assert_eq!(assignments[8].physical_control_id, "fader-1");
    assert_eq!(assignments[11].label, "Out Lvl");
    assert_eq!(assignments[12].parameter_id.as_deref(), Some("control-2"));
    assert!(assignments[13].parameter_id.is_none());
}

#[test]
fn destination_catalog_derives_exact_labels_categories_and_ranges() {
    let profile = eventide_micropitch_profile();
    let parameters = destination_parameters(&profile);
    assert_eq!(parameters.len(), profile.controls.len());
    assert_eq!(parameters[0].label, profile.controls[0].label);
    assert_eq!(parameters[0].range, profile.controls[0].range);
    assert_eq!(parameters[0].category, "General");
    assert!(parameters.iter().all(|parameter| !parameter.label.is_empty()));
}

#[test]
fn profile_renders_validated_control_messages_without_transmitting() {
    let profile = eventide_micropitch_profile();
    assert_eq!(profile.render_control_message("Mix", 1, 64).expect("CC"), vec![0xB0, 20, 64]);
    assert_eq!(profile.render_control_message("Preset 1", 2, 0).expect("program"), vec![0xC1, 1]);
    assert!(profile.render_control_message("Mix", 0, 64).is_err());
    assert!(profile.render_control_message("Mix", 1, 128).is_err());
    assert!(profile.render_control_message("missing", 1, 1).is_err());
    assert_eq!(
        profile.render_parameter_message("control-4", 1, 64).expect("stable parameter")[..],
        [0xB0, 20, 64]
    );
    assert!(profile.render_parameter_message("unknown", 1, 64).is_err());
    let reflex = lexicon_reflex_profile();
    assert_eq!(
        reflex.render_parameter_message("reflex.parameter-7", 2, 0xABCD).expect("Reflex SysEx"),
        lexicon_reflex::encode_nibblized_parameter(1, 7, 0xABCD).expect("fixture")
    );
}

#[test]
fn led_feedback_layers_prioritize_result_then_assignment_then_base() {
    assert_eq!(select_led_feedback_layer(true, false, false), Some(LedFeedbackLayer::Base));
    assert_eq!(select_led_feedback_layer(true, true, false), Some(LedFeedbackLayer::Assignment));
    assert_eq!(select_led_feedback_layer(true, true, true), Some(LedFeedbackLayer::Result));
    assert_eq!(select_led_feedback_layer(false, false, false), None);
    assert!(result_overlay_lit(0));
    assert!(result_overlay_lit(399));
    assert!(!result_overlay_lit(400));
    assert!(result_overlay_lit(800));
    assert!(result_overlay_lit(1_199));
    assert!(!result_overlay_lit(1_200));
    assert!(!result_overlay_lit(1_600));
    assert_eq!(result_overlay_color(0, true), LedColor::Green);
    assert_eq!(result_overlay_color(0, false), LedColor::Red);
    assert_eq!(result_overlay_color(400, true), LedColor::Off);
    let base = LedState::new(LedColor::Amber, 32, false);
    let assignment = LedState::new(LedColor::Green, 64, false);
    let mut scheduler = LedFeedbackScheduler::new(base);
    assert_eq!(scheduler.state_at(0), base);
    scheduler.assignment = Some(assignment);
    assert_eq!(scheduler.state_at(0), assignment);
    scheduler.result = Some((true, 100));
    assert_eq!(scheduler.state_at(100), LedState::new(LedColor::Green, 127, false));
    assert_eq!(scheduler.state_at(500), LedState::new(LedColor::Off, 127, false));
    assert_eq!(scheduler.state_at(1_700), base);
    scheduler.restore_base();
    assert_eq!(scheduler.state_at(0), base);
    assert_eq!(fader_column_led_proxy(0), Some((24, 32)));
    assert_eq!(fader_column_led_proxy(7), Some((31, 39)));
    assert_eq!(fader_column_led_proxy(8), None);
}

#[test]
fn lexicon_reflex_profile_is_reverb_sysex_anchor() {
    let profile = lexicon_reflex_profile();
    assert_eq!(profile.effect_type, EffectType::Reverb);
    assert_eq!(profile.capabilities[0].id, "midi-sysex");
    assert!(profile.validate().is_ok());
}

#[test]
fn effect_blocks_are_deterministic_and_use_general_fallback() {
    let profile = eventide_micropitch_profile();
    let _blocks = effect_blocks(&profile);
    assert_eq!(
        classify_launch_control_usb(
            UsbIdentity { vendor_id: 0x1235, product_id: 0x0061 },
            "Focusrite Launch Control XL"
        ),
        LaunchControlIdentity::Mk2
    );
    assert_eq!(
        classify_launch_control_usb(
            UsbIdentity { vendor_id: 0x1235, product_id: 0x0061 },
            "Launchpad XL"
        ),
        LaunchControlIdentity::LaunchpadFamily
    );
    assert!(usb_identity_matches(EVENTIDE_MICROPITCH_USB, EVENTIDE_MICROPITCH_USB));
    assert!(usb_identity_matches(MIDISPORT_4X4_LOADER_USB, MIDISPORT_4X4_LOADER_USB));
}

#[test]
fn novation_family_discovery_covers_platform_families_and_mk2_template_gate() {
    assert_eq!(
        classify_novation_product("Novation Launchkey 49"),
        NovationProductFamily::Launchkey
    );
    assert_eq!(classify_novation_product("Novation Launchpad X"), NovationProductFamily::Launchpad);
    assert_eq!(
        classify_novation_product("Novation Circuit Tracks"),
        NovationProductFamily::Circuit
    );
    assert_eq!(classify_novation_product("Novation SL MkIII"), NovationProductFamily::Sl);
    assert_eq!(classify_novation_product("Novation Peak"), NovationProductFamily::Peak);
    assert_eq!(classify_novation_product("Novation Summit"), NovationProductFamily::Summit);
    assert_eq!(
        classify_novation_product("Novation Bass Station II"),
        NovationProductFamily::BassStation
    );
    assert_eq!(classify_novation_product("Novation MiniNova"), NovationProductFamily::MiniNova);
    assert_eq!(classify_novation_product("Novation UltraNova"), NovationProductFamily::UltraNova);
    assert_eq!(classify_novation_product("Novation Impulse 49"), NovationProductFamily::Impulse);
    assert_eq!(classify_novation_product("Novation FLkey 37"), NovationProductFamily::Flkey);
    assert!(novation_template_available("Novation Launch Control XL Mk2"));
    assert!(!novation_template_available("Novation Launchpad X"));
}

#[test]
fn launch_control_faceplate_covers_all_controls_in_order() {
    let controls = launch_control_faceplate();
    assert_eq!(controls.len(), 48);
    assert!(controls.iter().enumerate().all(|(position, control)| {
        usize::from(control.index) == position && !control.label.is_empty()
    }));
    assert!(controls[..24].iter().all(|control| control.kind == LaunchControlControlKind::Knob));
    assert!(controls[24..40]
        .iter()
        .all(|control| control.kind == LaunchControlControlKind::Button));
    assert!(controls[40..].iter().all(|control| control.kind == LaunchControlControlKind::Utility));
}

#[test]
fn physical_catalog_is_complete_unique_and_separates_faders_from_utilities() {
    let controls = launch_control_physical_catalog();
    assert_eq!(controls.len(), 56);
    let ids: std::collections::BTreeSet<_> = controls.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids.len(), 56);
    assert_eq!(controls.iter().filter(|c| c.role == PhysicalControlRole::Knob).count(), 24);
    assert_eq!(
        controls.iter().filter(|c| c.role == PhysicalControlRole::ChannelButton).count(),
        16
    );
    assert_eq!(controls.iter().filter(|c| c.role == PhysicalControlRole::Fader).count(), 8);
    assert_eq!(controls.iter().filter(|c| c.role == PhysicalControlRole::Utility).count(), 8);
    assert!(PhysicalControlId::new("fader-1").is_ok());
    assert!(PhysicalControlId::new("utility-1").is_ok());
    assert!(PhysicalControlId::new("fader-9").is_err());
    assert_eq!(controls[0].source_address, Some(13));
    assert_eq!(controls[8].source_address, Some(29));
    assert_eq!(controls[24].source_address, Some(41));
    assert_eq!(controls[32].source_address, Some(73));
    assert_eq!(controls[40].source_address, Some(77));
}

#[test]
fn mk2_factory1_layout_is_complete_unique_and_exact() {
    let layout = launch_control_mk2_factory1_layout();
    assert_eq!(layout.len(), 56);
    let identities = layout
        .iter()
        .map(|control| control.physical_control_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let tuples = layout
        .iter()
        .map(|control| (control.channel, control.source_kind, control.source_number))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(identities.len(), 56);
    assert_eq!(tuples.len(), 56);
    assert!(layout.iter().all(|control| control.channel == 8));
    assert_eq!(
        resolve_launch_control_mk2_factory1_input(
            8,
            LaunchControlSourceKind::ControlChange,
            13,
            127
        ),
        Some("knob-r1-c1".into())
    );
    assert_eq!(
        resolve_launch_control_mk2_factory1_input(8, LaunchControlSourceKind::Note, 105, 127),
        Some("utility-1".into())
    );
    assert!(resolve_launch_control_mk2_factory1_input(
        0,
        LaunchControlSourceKind::ControlChange,
        13,
        127
    )
    .is_none());
    assert!(resolve_launch_control_mk2_factory1_input(
        8,
        LaunchControlSourceKind::ControlChange,
        21,
        127
    )
    .is_none());
    assert!(resolve_launch_control_mk2_factory1_input(8, LaunchControlSourceKind::Note, 105, 0)
        .is_none());
}

#[test]
fn mk2_factory1_every_control_press_resolves_and_release_is_rejected() {
    for control in launch_control_mk2_factory1_layout() {
        assert_eq!(
            resolve_launch_control_mk2_factory1_input(
                control.channel,
                control.source_kind,
                control.source_number,
                127
            ),
            Some(control.physical_control_id.clone())
        );
        assert!(resolve_launch_control_mk2_factory1_input(
            control.channel,
            control.source_kind,
            control.source_number,
            0
        )
        .is_none());
    }
    let knobs: Vec<u8> = launch_control_mk2_factory1_layout()
        .into_iter()
        .filter(|control| control.role == PhysicalControlRole::Knob)
        .map(|control| control.source_number)
        .collect();
    let buttons: Vec<u8> = launch_control_mk2_factory1_layout()
        .into_iter()
        .filter(|control| control.role == PhysicalControlRole::ChannelButton)
        .map(|control| control.source_number)
        .collect();
    let faders: Vec<u8> = launch_control_mk2_factory1_layout()
        .into_iter()
        .filter(|control| control.role == PhysicalControlRole::Fader)
        .map(|control| control.source_number)
        .collect();
    assert_eq!(
        knobs,
        vec![
            13, 14, 15, 16, 17, 18, 19, 20, 29, 30, 31, 32, 33, 34, 35, 36, 49, 50, 51, 52, 53, 54,
            55, 56
        ]
    );
    assert_eq!(buttons, vec![41, 42, 43, 44, 57, 58, 59, 60, 73, 74, 75, 76, 89, 90, 91, 92]);
    assert_eq!(faders, vec![77, 78, 79, 80, 81, 82, 83, 84]);
    assert_eq!(LAUNCH_CONTROL_MK2_FACTORY1_SLOT, 8);
}

#[test]
fn effects_faceplate_has_six_groups_eight_faders_and_no_conflicts() {
    let faceplate = launch_control_effects_faceplate();
    assert_eq!(faceplate.groups.len(), 6);
    assert_eq!(faceplate.fader_indices, (40..48).collect::<Vec<_>>());
    assert_eq!(faceplate.unused_indices, (36..40).collect::<Vec<_>>());
    assert_eq!(faceplate.groups[0].label, "Gain");
    assert_eq!(faceplate.groups[5].label, "Reverb");
    assert!(faceplate.validate().is_ok());
}

#[test]
fn effect_group_led_policy_is_pickup_aware_and_fail_closed() {
    assert_eq!(effect_group_led(EffectGroupState::Enabled, false), EffectGroupLed::Off);
    assert_eq!(effect_group_led(EffectGroupState::Enabled, true), EffectGroupLed::Green);
    assert_eq!(effect_group_led(EffectGroupState::Disabled, false), EffectGroupLed::SolidRed);
    assert_eq!(effect_group_led(EffectGroupState::Unavailable, true), EffectGroupLed::BlinkingRed);
    assert_eq!(effect_group_led(EffectGroupState::Selected, false), EffectGroupLed::BlueTeal);
    assert_eq!(effect_group_led(EffectGroupState::Unknown, true), EffectGroupLed::Off);
}

#[test]
fn effects_assignments_preserve_ranges_and_explain_unassigned_parameters() {
    let faceplate = launch_control_effects_faceplate();
    let profile = eventide_micropitch_profile();
    let assignments = effects_parameter_assignments(&faceplate, &profile);
    assert!(!assignments.is_empty());
    assert!(assignments.iter().all(|assignment| assignment.range.0 <= assignment.range.1));
    assert!(assignments.iter().any(|assignment| assignment.control_index.is_some()));
    assert_eq!(assignments[0].unit, "value");
}

#[test]
fn effects_automation_is_ordered_bounded_and_fail_closed() {
    let faceplate = launch_control_effects_faceplate();
    let requested = ["reverb", "gain", "modulation"].map(String::from);
    let plan = plan_effects_automation(&faceplate, &requested, false);
    assert_eq!(
        plan.iter().map(|operation| operation.group_id.as_str()).collect::<Vec<_>>(),
        vec!["gain", "modulation", "reverb"]
    );
    assert!(plan.iter().all(|operation| !operation.supported && operation.reason.is_some()));
    assert!(plan[0].reason.as_deref().is_some_and(|reason| reason.contains("pickup")));
}

#[test]
fn reusable_effects_configuration_is_minimal_and_signal_ordered() {
    let faceplate = launch_control_effects_faceplate();
    let profiles = builtin_profiles();
    let config = generate_reusable_effects_configuration(
        &faceplate,
        &profiles,
        &["reverb".into(), "gain".into()],
    );
    assert_eq!(config.groups, vec!["gain", "reverb"]);
    assert_eq!(config.name, "Effects — gain → reverb");
    assert!(config.assignments.iter().all(|assignment| assignment.control_index.is_some()));
    assert!(generate_reusable_effects_configuration(&faceplate, &profiles, &[])
        .assignments
        .is_empty());
}

#[test]
fn effects_demo_frames_cover_states_faders_and_resync_deterministically() {
    let frames = effects_demo_frames();
    assert_eq!(frames, effects_demo_frames());
    assert_eq!(frames.len(), 4);
    assert!(frames.iter().all(|frame| frame.groups.len() == 6 && frame.faders.len() == 8));
    assert!(frames.iter().all(|frame| frame.faders.iter().all(|value| *value <= 127)));
    assert!(frames.last().is_some_and(|frame| frame.resync));
}

#[test]
fn effects_group_runtime_bounds_pickup_states_and_resync() {
    let mut runtime = EffectsGroupRuntime::new();
    assert!(runtime.resync_required);
    runtime.set_group(0, EffectGroupState::Enabled).expect("group");
    runtime.set_fader(0, 200).expect("fader");
    assert_eq!(runtime.faders[0], 127);
    runtime.acknowledge_resync();
    runtime.request_resync();
    assert!(runtime.resync_required);
    assert!(runtime.set_group(6, EffectGroupState::Disabled).is_err());
    assert!(runtime.set_fader(8, 0).is_err());
}

#[test]
fn bidirectional_hud_contract_preserves_identity_and_fails_closed() {
    let hud = bidirectional_hud_faceplate("hud-1", "Controller HUD", true);
    assert_eq!(hud.identity_marker, "hud-1");
    assert_eq!(hud.label, "Controller HUD");
    assert!(hud.feedback_capable);
    assert!(!hud.protocol_verified);
}

#[test]
fn launch_control_activity_resolution_requires_unique_valid_assignment() {
    let template = LaunchControlTemplate {
        template: 0,
        assignments: vec![LaunchControlAssignment {
            index: 3,
            channel: 1,
            number: 17,
            kind: LaunchControlMessageKind::Cc,
            destination: None,
        }],
    };
    assert_eq!(template.resolve_activity_control(1, 17, LaunchControlMessageKind::Cc), Some(3));
    assert_eq!(template.resolve_activity_control(1, 18, LaunchControlMessageKind::Cc), None);
    assert_eq!(template.resolve_activity_control(16, 17, LaunchControlMessageKind::Cc), None);
}

#[test]
fn launch_control_mk1_led_protocol_matches_programmers_reference() {
    assert_eq!(launch_control_led_index(0, 0), Some(0));
    assert_eq!(launch_control_led_index(3, 7), Some(31));
    assert_eq!(launch_control_led_index(4, 0), None);
    assert_eq!(launch_control_led_index(0, 8), None);
    assert_eq!(LAUNCH_CONTROL_DEVICE_INDEX, 40);
    assert_eq!(LAUNCH_CONTROL_RIGHT_INDEX, 47);
    assert_eq!(launch_control_index_label(0).as_deref(), Some("Top knob 1"));
    assert_eq!(launch_control_index_label(39).as_deref(), Some("Bottom channel button 8"));
    assert_eq!(launch_control_index_label(43).as_deref(), Some("Record Arm"));
    assert_eq!(launch_control_index_label(48), None);
    assert_eq!(launch_control_led_value(LedColor::Off, 127, 0x0c), 0x0c);
    assert_eq!(launch_control_led_value(LedColor::Red, 127, 0x0c), 0x0f);
    assert_eq!(launch_control_led_value(LedColor::Green, 127, 0x0c), 0x3c);
    assert_eq!(launch_control_led_value(LedColor::Amber, 127, 0x0c), 0x3f);
    assert_eq!(launch_control_led_value(LedColor::Yellow, 127, 0x0c), 0x3e);
    let labels: std::collections::BTreeSet<_> =
        (0..48).map(|index| launch_control_index_label(index).expect("documented index")).collect();
    assert_eq!(labels.len(), 48);
    let template = LaunchControlTemplate {
        template: 0,
        assignments: vec![LaunchControlAssignment {
            index: 0,
            channel: 0,
            number: 14,
            kind: LaunchControlMessageKind::Cc,
            destination: None,
        }],
    };
    assert!(template.validate().is_ok());
    assert_eq!(template.assignment(0).map(|a| a.number), Some(14));
    assert_eq!(template.assignment(1), None);
    assert_eq!(template.assignments_by_index().len(), 1);
    let encoded = serde_json::to_vec(&template).expect("template encode");
    assert_eq!(
        serde_json::from_slice::<LaunchControlTemplate>(&encoded).expect("template decode"),
        template
    );
    let mut duplicate = template;
    duplicate.assignments.push(duplicate.assignments[0].clone());
    assert!(duplicate.validate().is_err());
    assert!(LaunchControlTemplate { template: 16, assignments: Vec::new() }.validate().is_err());
    assert_eq!(
        encode_launch_control_led(0, 24, 0x7f),
        Some(vec![0xf0, 0x00, 0x20, 0x29, 0x02, 0x11, 0x78, 0x00, 0x18, 0x7f, 0xf7])
    );
    assert_eq!(encode_launch_control_led(16, 0, 0), None);
    assert_eq!(encode_launch_control_led(0, 48, 0), None);
    assert_eq!(
        encode_launch_control_toggle(2, 7, true),
        Some(vec![0xf0, 0x00, 0x20, 0x29, 0x02, 0x11, 0x7b, 0x02, 0x07, 0x7f, 0xf7])
    );
    assert_eq!(encode_launch_control_toggle(2, 7, false).expect("toggle")[9], 0);
    assert_eq!(encode_launch_control_toggle(0, 24, true), None);
    assert_eq!(encode_launch_control_note_led(0, 40, 0x7f), Some([0x90, 40, 0x7f]));
    assert_eq!(encode_launch_control_note_led(16, 40, 1), None);
    assert_eq!(encode_launch_control_cc_led(2, 14, 0x80), Some([0xb2, 14, 0]));
    assert_eq!(encode_launch_control_cc_led(16, 14, 1), None);
    assert_eq!(
        encode_launch_control_template(7),
        Some([0xf0, 0x00, 0x20, 0x29, 0x02, 0x11, 0x77, 0x07, 0xf7])
    );
}

#[test]
fn factory1_led_feedback_golden_bytes_cover_every_index_and_reject_unknown() {
    for index in 0..48_u8 {
        for (color, value) in [
            (LedColor::Off, 0x0c_u8),
            (LedColor::Red, 0x0f),
            (LedColor::Green, 0x3c),
            (LedColor::Amber, 0x3f),
            (LedColor::Yellow, 0x3e),
        ] {
            assert_eq!(
                encode_launch_control_feedback(8, index, LedState::new(color, 127, false)),
                Some(vec![0xf0, 0x00, 0x20, 0x29, 0x02, 0x11, 0x78, 8, index, value, 0xf7])
            );
        }
    }
    assert_eq!(
        encode_launch_control_feedback(8, 0, LedState::new(LedColor::Unknown, 127, false)),
        None
    );
    assert_eq!(
        encode_launch_control_feedback(8, 48, LedState::new(LedColor::Red, 127, false)),
        None
    );
}

#[test]
fn scheduled_feedback_uses_profile_owned_led_address_and_value_encoding() {
    let frame = encode_launch_control_feedback(0, 24, LedState::new(LedColor::Green, 127, true))
        .expect("valid feedback frame");
    assert_eq!(frame, vec![0xf0, 0x00, 0x20, 0x29, 0x02, 0x11, 0x78, 0x00, 0x18, 0x38, 0xf7]);
    assert!(
        encode_launch_control_feedback(16, 24, LedState::new(LedColor::Red, 127, false)).is_none()
    );
    assert!(
        encode_launch_control_feedback(0, 48, LedState::new(LedColor::Red, 127, false)).is_none()
    );
}

#[test]
fn led_coalescer_suppresses_redundant_updates() {
    let state = LedState::new(LedColor::Green, 100, false);
    let mut coalescer = LedCoalescer::default();
    coalescer.set_desired(2, state);
    assert_eq!(coalescer.drain_pending(), vec![(2, state)]);
    assert!(coalescer.drain_pending().is_empty());
    coalescer.set_desired(2, LedState { blink: true, ..state });
    assert_eq!(coalescer.drain_pending().len(), 1);
    coalescer.set_desired(3, state);
    assert_eq!(coalescer.drain_pending_limited(1).len(), 1);
    assert_eq!(coalescer.drain_pending_limited(0).len(), 0);
    assert_eq!(coalescer.desired_len(), 2);
    coalescer.request_full_resync();
    assert_eq!(coalescer.drain_pending_limited(1).len(), 1);
    assert_eq!(coalescer.drain_pending(), vec![(3, state)]);
    assert!(coalescer.drain_pending().is_empty());
}

#[test]
fn led_test_and_demo_modes_are_bounded_and_deterministic() {
    let test = launch_control_led_test_pattern(0).expect("valid template");
    assert_eq!(test.len(), 48);
    assert_eq!(test.first().expect("first LED")[6], 0x78);
    assert!(launch_control_led_test_pattern(16).is_none());
    let demo = launch_control_led_demo_frames(0).expect("valid template");
    assert_eq!(demo.len(), 4);
    assert_eq!(demo.iter().map(Vec::len).collect::<Vec<_>>(), vec![48; 4]);
    assert_eq!(demo[0][0][9], 0x0c);
    assert_eq!(demo[1][0][9], 0x3c);
}

#[allow(clippy::too_many_lines)]
#[test]
fn reflex_packing_checksum_and_nibbles_match_contract() {
    for length in [1, 6, 7, 8, 55, 56, 127] {
        let source: Vec<u8> = (0..length)
            .map(|index| u8::try_from(index).expect("bounded test index").wrapping_mul(37))
            .collect();
        let packed = lexicon_reflex::pack(&source);
        assert_eq!(lexicon_reflex::unpack(&packed, length).expect("unpack"), source);
    }
    assert!(lexicon_reflex::unpack(&[0x80], 1).is_err());
    assert!(lexicon_reflex::unpack(&[0], 0).is_err());
    assert_eq!(
        lexicon_reflex::pack_group(&[0x00, 0x80, 0xFF]).expect("group"),
        vec![0x06, 0x00, 0x00, 0x7F]
    );
    assert_eq!(lexicon_reflex::pack(&[0x80]), vec![0x01, 0x00]);
    assert_eq!(lexicon_reflex::checksum(&[1, 2, 0x80]), 3);
    assert_eq!(lexicon_reflex::nibblize(0xABCD), [0x0A, 0x0B, 0x0C, 0x0D]);
    let setup =
        lexicon_reflex::ReflexSetup::new(&[0; lexicon_reflex::SETUP_RAW_BYTES]).expect("setup");
    assert_eq!(setup.packed().len(), lexicon_reflex::SETUP_PACKED_BYTES);
    assert_eq!(
        lexicon_reflex::ReflexSetup::from_packed(&setup.packed()).expect("packed setup"),
        setup
    );
    assert!(lexicon_reflex::ReflexSetup::from_packed(&[0; 55]).is_err());
    let bank = lexicon_reflex::ReflexRegisterBank::new(vec![setup.clone(); 128]).expect("bank");
    assert_eq!(bank.raw_bytes().len(), lexicon_reflex::ALL_REGISTERS_RAW_BYTES);
    assert_eq!(
        lexicon_reflex::ReflexRegisterBank::from_raw(&bank.raw_bytes()).expect("bank raw"),
        bank
    );
    assert_eq!(bank.encode_frame(1).expect("bank frame").len(), 7_176);
    let frame = bank.encode_frame(1).expect("bank frame");
    assert_eq!(lexicon_reflex::ReflexRegisterBank::from_frame(&frame).expect("bank decode").0, 1);
    assert!(bank.get(127).is_some());
    let mut editable = bank.clone();
    editable.get_mut(127).expect("register").set_parameter(0, 1).expect("parameter");
    assert_eq!(editable.get(127).and_then(|setup| setup.parameter(0)), Some(1));
    editable.set(0, setup.clone()).expect("replace register");
    assert!(editable.set(128, setup.clone()).is_err());
    assert!(bank.get(127).is_some());
    assert!(lexicon_reflex::ReflexRegisterBank::new(vec![setup.clone()]).is_err());
    assert_eq!(setup.algorithm(), None);
    let mut named = [0_u8; lexicon_reflex::SETUP_RAW_BYTES];
    named[0] = 3;
    named[21..27].copy_from_slice(b"Reflex");
    let named_setup = lexicon_reflex::ReflexSetup::new(&named).expect("named setup");
    assert_eq!(named_setup.algorithm(), Some(3));
    assert_eq!(&named_setup.name_bytes()[..6], b"Reflex");
    let mut parameters = named_setup;
    parameters.set_name(b"New Name").expect("name");
    assert_eq!(&parameters.name_bytes()[..8], b"New Name");
    assert_eq!(parameters.name_bytes()[8], 0);
    assert!(parameters.set_name(&[b'x'; 17]).is_err());
    assert_eq!(parameters.parameter(0), Some(0));
    parameters.set_parameter(9, 0xABCD).expect("parameter");
    assert_eq!(parameters.parameter(9), Some(0xABCD));
    assert!(parameters.patch().is_ok());
    assert_eq!(parameters.parameter(10), None);
    assert!(parameters.set_parameter(10, 1).is_err());
    let encoded = serde_json::to_vec(&setup).expect("setup json");
    assert_eq!(
        serde_json::from_slice::<lexicon_reflex::ReflexSetup>(&encoded).expect("setup decode"),
        setup
    );
    assert!(serde_json::from_slice::<lexicon_reflex::ReflexSetup>(b"[0,1]").is_err());
    assert!(lexicon_reflex::ReflexSetup::new(&[0; 48]).is_err());
    let patch = lexicon_reflex::ReflexPatch {
        sources: [1, 2, 3, 4],
        destinations: [0, 1, 8, 9],
        scales: [-128, -1, 0, 127],
    };
    let mut setup_bytes = [0_u8; lexicon_reflex::SETUP_RAW_BYTES];
    patch.encode(&mut setup_bytes).expect("patch");
    assert_eq!(lexicon_reflex::ReflexPatch::decode(&setup_bytes).expect("patch decode"), patch);
    assert_eq!(&setup_bytes[37..45], &[1, 2, 3, 4, 0, 1, 8, 9]);
    assert_eq!(&setup_bytes[45..49], &[128, 255, 0, 127]);
    assert!(lexicon_reflex::ReflexPatch { destinations: [10, 0, 0, 0], ..patch }
        .validate()
        .is_err());
    let packed_parameter =
        lexicon_reflex::encode_packed_parameter(2, 7, 0xABCD).expect("packed parameter");
    assert_eq!(packed_parameter.len(), 9);
    assert_eq!(
        lexicon_reflex::decode_packed_parameter(&packed_parameter).expect("packed decode"),
        (2, 7, 0xABCD)
    );
    let dump = lexicon_reflex::encode_setup_dump(&[0; lexicon_reflex::SETUP_BYTES]).expect("dump");
    assert_eq!(dump.len(), 65);
    assert_eq!(dump.last().copied(), Some(0));
    assert!(lexicon_reflex::validate_setup_dump(&dump).is_ok());
    assert_eq!(lexicon_reflex::decode_setup_dump(&dump).expect("decode"), vec![0; 56]);
    let raw_setup = vec![0x80; lexicon_reflex::SETUP_RAW_BYTES];
    let active = lexicon_reflex::encode_active_setup_frame(2, &raw_setup).expect("active");
    assert_eq!(active.len(), 63);
    assert_eq!(
        lexicon_reflex::decode_active_setup_frame(&active).expect("active decode"),
        (2, raw_setup.clone())
    );
    let register =
        lexicon_reflex::encode_register_setup_frame(2, 127, &raw_setup).expect("register");
    assert_eq!(register.len(), 64);
    assert_eq!(
        lexicon_reflex::decode_register_setup_frame(&register).expect("register decode"),
        (2, 127, raw_setup)
    );
    let setup_frame = lexicon_reflex::encode_setup_frame(2, &[0; 56]).expect("frame");
    assert_eq!(setup_frame.len(), 70);
    assert_eq!(
        lexicon_reflex::decode_setup_frame(&setup_frame).expect("frame decode"),
        (2, vec![0; 56])
    );
    let bank = vec![0x80; lexicon_reflex::ALL_REGISTERS_RAW_BYTES];
    let all = lexicon_reflex::encode_all_registers_frame(3, &bank).expect("all frame");
    assert_eq!(all.len(), 7_176);
    assert_eq!(lexicon_reflex::decode_all_registers_frame(&all).expect("all decode"), (3, bank));
    let mut corrupt = dump;
    corrupt[0] ^= 1;
    assert!(lexicon_reflex::validate_setup_dump(&corrupt).is_err());
    assert!(lexicon_reflex::encode_setup_dump(&[0; 55]).is_err());
    assert_eq!(
        lexicon_reflex::encode_nibblized_parameter(2, 7, 0xABCD).expect("frame"),
        vec![0xF0, 6, 2, 0x52, 7, 0x0A, 0x0B, 0x0C, 0x0D, 0xF7]
    );
    let nib = lexicon_reflex::encode_nibblized_parameter(2, 7, 0xABCD).expect("nib");
    assert_eq!(
        lexicon_reflex::decode_nibblized_parameter(&nib).expect("nib decode"),
        (2, 7, 0xABCD)
    );
    assert_eq!(
        lexicon_reflex::decode_message(&active).expect("typed active"),
        lexicon_reflex::DecodedMessage::ActiveSetup { channel: 2, setup: vec![0x80; 49] }
    );
    assert_eq!(
        lexicon_reflex::decode_message(&nib).expect("typed nib"),
        lexicon_reflex::DecodedMessage::NibblizedParameter {
            channel: 2,
            parameter: 7,
            value: 0xABCD
        }
    );
    let malformed = vec![0xF0, 6, 2, 0x72, 0x70, 0, 0xF7];
    assert!(lexicon_reflex::decode_message(&malformed).is_err());
    assert_eq!(
        lexicon_reflex::encode_task(1, lexicon_reflex::TASK_BYPASS, 1).expect("task"),
        [0xF0, 6, 2, 0x61, 0x72, 1, 0xF7]
    );
    assert_eq!(
        lexicon_reflex::encode_task(1, lexicon_reflex::TASK_BYPASS, 2),
        Err("invalid bypass argument")
    );
    let task = lexicon_reflex::encode_task(1, lexicon_reflex::TASK_RECALL, 7).expect("task");
    assert_eq!(lexicon_reflex::decode_task(&task), Ok((1, lexicon_reflex::TASK_RECALL, 7)));
    assert_eq!(lexicon_reflex::decode_task(&[0; 7]), Err("invalid Reflex task frame"));
    assert_eq!(
        lexicon_reflex::encode_request(0, lexicon_reflex::REQUEST_ACTIVE, 0).expect("request"),
        [0xF0, 6, 2, 0x30, 0x60, 0, 0xF7]
    );
    assert_eq!(lexicon_reflex::encode_request(0, 0x63, 0), Err("unsupported Reflex request"));
    let request =
        lexicon_reflex::encode_request(4, lexicon_reflex::REQUEST_REGISTER, 127).expect("request");
    assert_eq!(
        lexicon_reflex::decode_request(&request).expect("request decode"),
        (4, lexicon_reflex::REQUEST_REGISTER, 127)
    );
    assert!(lexicon_reflex::decode_request(&[0; 7]).is_err());
}

#[test]
fn pcm70_catalog_translates_to_valid_named_reflex_setups_and_sysex() {
    let catalog = lexicon_reflex::pcm70_translations();
    assert_eq!(catalog.len(), 5);
    assert_eq!(
        catalog.iter().map(|preset| preset.name).collect::<Vec<_>>(),
        vec!["Concert Wave", "Circular Reverbs", "INF Reverb", "Rich Plate", "Mod Wobble"]
    );
    for preset in catalog {
        let setup = lexicon_reflex::translate_pcm70(preset.id).expect("translation");
        assert_eq!(setup.algorithm(), Some(preset.reflex_algorithm));
        assert!(setup.name_bytes().starts_with(preset.name.as_bytes()));
        for parameter in lexicon_reflex::parameters(preset.reflex_algorithm) {
            let value = setup.parameter(parameter.number).expect("translated parameter");
            assert!((parameter.min..=parameter.max).contains(&value));
        }
        let frame = lexicon_reflex::encode_pcm70_translation(preset.id, 0).expect("SysEx");
        assert_eq!(frame.first(), Some(&0xF0));
        assert_eq!(frame.last(), Some(&0xF7));
        let (_, decoded) =
            lexicon_reflex::decode_active_setup_frame(&frame).expect("active setup decode");
        assert_eq!(decoded, setup.as_bytes());
    }
    assert!(lexicon_reflex::translate_pcm70("missing").is_err());
    assert!(lexicon_reflex::encode_pcm70_translation("concert-wave", 16).is_err());
    let projected =
        lexicon_reflex::pcm70_translation_parameters("concert-wave").expect("parameter projection");
    assert!(!projected.is_empty());
    assert!(projected.iter().all(|(number, _value)| *number <= 9));
    let controller_values = lexicon_reflex::pcm70_translation_controller_values("concert-wave")
        .expect("controller projection");
    assert_eq!(controller_values.len(), projected.len());
    assert!(controller_values.iter().all(|(_, value)| *value <= 127));
}

#[test]
fn reflex_decodes_hardware_active_setup_with_reserved_tail() {
    let frame = crate::parse_sysex_hex(
        "F0 06 02 00 38 50 01 00 00 00 14 40 36 0A 00 08 00 3C 00 00 00 54 00 40 36 00 26 00 30 00 43 6F 6E 63 65 72 74 00 20 57 61 76 65 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 4A F7",
    )
    .expect("hardware frame");
    let (_, setup) = lexicon_reflex::decode_active_setup_frame(&frame).expect("active setup");
    assert_eq!(lexicon_reflex::ReflexSetup::new(&setup).unwrap().algorithm(), Some(1));
}

#[test]
fn pcm70_translation_values_are_on_reflex_wire_steps() {
    let setup = lexicon_reflex::translate_pcm70("concert-wave").expect("translation");
    assert_eq!(
        (0..10).map(|number| setup.parameter(number).unwrap()).collect::<Vec<_>>(),
        vec![0xB400, 0x9400, 0xB640, 0x8800, 0xBC00, 0xBF00, 0x8880, 0xB640, 0xA600, 0xB000]
    );
}

#[test]
fn reflex_profile_exposes_pcm70_translations_as_controller_operations() {
    let profile = lexicon_reflex_profile();
    assert_eq!(profile.controls.len(), 5);
    let rendered = profile.render_control_message("Concert Wave", 1, 1).expect("render");
    let (_, setup) =
        lexicon_reflex::decode_active_setup_frame(&rendered).expect("active setup decode");
    let parameter_rendered = profile
        .render_parameter_message("pcm70_reflex:concert-wave", 1, 127)
        .expect("translator parameter render");
    assert_eq!(parameter_rendered, rendered);
    assert_eq!(setup[0], 1);
    assert_eq!(&setup[21..33], b"Concert Wave");
}

#[test]
fn reflex_algorithm_registry_matches_manual_order_and_numbers() {
    let algorithms = lexicon_reflex::algorithms();
    assert_eq!(algorithms.len(), 8);
    assert_eq!(
        algorithms.iter().map(|a| a.number).collect::<Vec<_>>(),
        (1..=8).collect::<Vec<_>>()
    );
    assert_eq!(algorithms[0].name, "Reverb");
    assert_eq!(algorithms[2].name, "Chorus 1");
    assert_eq!(algorithms[7].description, "Chorus/Delays");
    assert_eq!(algorithms[0].preset_numbers, &[1, 2, 3, 4, 5, 6]);
    assert_eq!(algorithms[7].preset_numbers, &[13, 14]);
}

#[test]
fn reflex_parameter_metadata_excludes_unused_slots_and_bounds_values() {
    let reverb = lexicon_reflex::parameters(1);
    assert_eq!(reverb.len(), 10);
    assert_eq!(reverb[0].description, "Mid Reverb Decay");
    assert_eq!(reverb[0].min, 0x8000);
    assert_eq!(reverb[0].max, 0xBC00);
    assert!(reverb[3].bipolar);
    assert!(lexicon_reflex::parameters(2)
        .iter()
        .all(|parameter| parameter.number != 8 && parameter.number != 9));
    assert_eq!(lexicon_reflex::parameters(3).len(), 8);
    assert_eq!(lexicon_reflex::parameters(6).len(), 7);
    assert_eq!(lexicon_reflex::normalize_parameter(1, 0, 0), Some(0x8000));
    assert_eq!(lexicon_reflex::normalize_parameter(1, 0, 127), Some(0xBC00));
    assert_eq!(lexicon_reflex::normalize_parameter(2, 8, 64), None);
    assert_eq!(lexicon_reflex::normalize_parameter(5, 0, 64), None);
}

#[test]
fn reflex_controller_layout_uniquely_owns_all_parameters_and_algorithms() {
    let assignments = lexicon_reflex::CONTROLLER_ASSIGNMENTS;
    assert_eq!(assignments.len(), 18);
    let controls = assignments
        .iter()
        .map(|assignment| assignment.physical_control_id)
        .collect::<std::collections::BTreeSet<_>>();
    let destinations = assignments
        .iter()
        .map(|assignment| assignment.destination_parameter)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(controls.len(), 18);
    assert_eq!(destinations.len(), 18);
    assert_eq!(
        lexicon_reflex::controller_assignment("button-r2-c8")
            .map(|assignment| assignment.destination_parameter),
        Some("reflex.algorithm-8")
    );
    assert_eq!(
        lexicon_reflex::controller_assignment("fader-8")
            .map(|assignment| assignment.destination_parameter),
        Some("reflex.parameter-9")
    );
    assert_eq!(
        lexicon_reflex::controller_assignment("fader-4")
            .map(|assignment| assignment.destination_parameter),
        Some("reflex.parameter-2")
    );
    assert_eq!(
        lexicon_reflex::controller_assignment("knob-r1-c6")
            .map(|assignment| assignment.destination_parameter),
        Some("reflex.parameter-5")
    );
}

#[test]
fn reflex_echo_rhythm_is_bounded_and_ordered() {
    assert_eq!(lexicon_reflex::ECHO_RHYTHMS.len(), 14);
    assert_eq!(lexicon_reflex::echo_rhythm(1).expect("first").label, "64th");
    assert_eq!(lexicon_reflex::echo_rhythm(14).expect("last").label, "Whole note");
    assert!(lexicon_reflex::echo_rhythm(0).is_none());
    assert!(lexicon_reflex::echo_rhythm(15).is_none());
}

#[test]
fn sysex_template_is_bounded_and_7_bit_safe() {
    let template = SysexTemplate {
        id: None,
        segments: vec![TemplateSegment::Literal(vec![1, 2]), TemplateSegment::Parameter(0)],
        max_bytes: 3,
    };
    assert_eq!(template.render(&[127]).expect("render"), vec![1, 2, 127]);
    assert_eq!(template.render(&[128]), Err("parameter is not MIDI data"));
    assert_eq!(template.render(&[]), Err("parameter missing"));
    let expression = SysexTemplate {
        id: None,
        segments: vec![TemplateSegment::Expression("$0 + 1".into())],
        max_bytes: 1,
    };
    assert_eq!(expression.render(&[41]).expect("expression"), vec![42]);
    let function = SysexTemplate {
        id: None,
        segments: vec![TemplateSegment::Expression("max($0, 9)".into())],
        max_bytes: 1,
    };
    assert_eq!(function.render(&[4]).expect("function"), vec![9]);
    let mut budget = EvaluationBudget::new();
    assert_eq!(function.render_with_budget(&[4], &mut budget).expect("budget"), vec![9]);
    let oversized = SysexTemplate {
        id: None,
        segments: vec![TemplateSegment::Literal(vec![1, 2, 3, 4])],
        max_bytes: 3,
    };
    assert_eq!(oversized.render(&[]), Err("template exceeds maximum size"));
}

#[test]
fn profile_validates_template_identity_and_query_references() {
    let mut profile = builtin_profiles()[0].clone();
    profile.templates = vec![SysexTemplate {
        id: Some("active-query".into()),
        segments: vec![TemplateSegment::Literal(vec![0x10])],
        max_bytes: 1,
    }];
    profile.replies =
        vec![ReplyDefinition { id: "active-reply".into(), value: vec![0x01], mask: vec![0x7F] }];
    profile.queries = vec![QueryDefinition {
        id: "active".into(),
        request: vec![0x10],
        template_id: Some("active-query".into()),
        reply_id: "active-reply".into(),
    }];
    assert!(profile.validate().is_ok());
    assert_eq!(profile.render_query_request("active", &[]), Ok(vec![0x10]));
    profile.queries[0].template_id = Some("missing".into());
    assert_eq!(profile.validate(), Err("query references a missing template"));
    assert_eq!(
        profile.render_query_request("active", &[]),
        Err("query references a missing template")
    );
}

#[test]
fn integer_literals_are_strict_and_checked() {
    assert_eq!(parse_integer_literal(" 0x7f "), Ok(127));
    assert_eq!(parse_integer_literal("-42"), Ok(-42));
    assert!(parse_integer_literal("0x").is_err());
    assert!(parse_integer_literal("9223372036854775808").is_err());
    assert_eq!(parse_parameter_expression("$0 + 3", &[4]), Ok(7));
    assert_eq!(parse_parameter_expression_full("($0 + 3) * 2", &[4]), Ok(14));
    assert_eq!(parse_parameter_expression_full("1 << 3 | 2", &[]), Ok(10));
    assert_eq!(parse_parameter_expression_full("2 + 3 == 5", &[]), Ok(1));
    assert_eq!(parse_parameter_expression_full("2 < 3 & 1", &[]), Ok(1));
    assert_eq!(parse_parameter_expression_full("$0 ? 11 : 22", &[1]), Ok(11));
    assert_eq!(parse_parameter_expression_full("$0 ? 11 : 22", &[0]), Ok(22));
    assert_eq!(parse_parameter_expression_full("1 ? 0 ? 2 : 3 : 4", &[]), Ok(3));
    assert_eq!(parse_parameter_expression_full("clamp($0 + 1, 0, 7)", &[4]), Ok(5));
    assert_eq!(parse_parameter_expression_full("max(2, min(9, 4))", &[]), Ok(4));
    assert!(parse_parameter_expression_full("unknown(1)", &[]).is_err());
    assert!(parse_parameter_expression_full("1 ? 2", &[]).is_err());
    assert!(parse_parameter_expression_full("(1 + 2", &[]).is_err());
    assert!(parse_parameter_expression("$1 + 3", &[4]).is_err());
    assert_eq!(parse_parameter_function_expression("max($0, 9)", &[4]), Ok(9));
}

#[test]
fn sysex_hex_parser_accepts_framed_and_rejects_malformed_input() {
    assert_eq!(parse_sysex_hex("F0 06 02 F7").expect("hex"), vec![0xF0, 6, 2, 0xF7]);
    assert_eq!(parse_sysex_hex("06 02").expect("raw"), vec![6, 2]);
    assert!(parse_sysex_hex("F0 06").is_err());
    assert!(parse_sysex_hex("F0 80 F7").is_err());
    assert!(parse_sysex_hex("GG").is_err());
    assert!(sysex_mask_matches(
        &[0xF0, 0x06, 0x12, 0xF7],
        &[0xF0, 0x06, 0x10, 0xF7],
        &[0xFF, 0xFF, 0xF0, 0xFF]
    ));
    assert!(!sysex_mask_matches(&[0xF0, 0x06], &[0xF0], &[0xFF]));
}

#[test]
fn binary_evaluation_is_checked() {
    assert_eq!(eval_binary(2, "+", 3), Ok(5));
    assert_eq!(eval_binary(7, "&", 3), Ok(3));
    assert_eq!(eval_binary(2, "<<", 3), Ok(16));
    assert_eq!(
        eval_binary(1, "/", 0),
        Err("expression operation overflow or invalid divisor/shift")
    );
    assert!(eval_binary(i64::MAX, "+", 1).is_err());
    assert_eq!(parse_binary_expression("0x10 << 2"), Ok(64));
    assert!(parse_binary_expression("1 + 2 extra").is_err());
    assert_eq!(parse_function_expression("clamp(10, 0, 7)"), Ok(7));
    assert!(parse_function_expression("unknown(1)").is_err());
}

#[test]
fn approved_functions_have_strict_arity() {
    assert_eq!(eval_function("min", &[4, 2]), Ok(2));
    assert_eq!(eval_function("clamp", &[10, 0, 7]), Ok(7));
    assert_eq!(eval_function("sum", &[1, 2, 3]), Ok(6));
    assert_eq!(eval_function("xor", &[7, 3]), Ok(4));
    assert_eq!(eval_function("hi7", &[256]), Ok(2));
    assert_eq!(eval_function("lo7", &[130]), Ok(2));
    assert_eq!(eval_function("lookup", &[1, 10, 20]), Ok(20));
    assert!(eval_function("lookup", &[2, 10, 20]).is_err());
    assert!(eval_function("lookup", &[-1, 10]).is_err());
    assert!(eval_function("lookup", &[i64::MAX, 10]).is_err());
    assert!(eval_function("min", &[1]).is_err());
    assert!(eval_function("unknown", &[1]).is_err());
    assert_eq!(eval_lookup(1, &[10, 20]), Ok(20));
    assert!(eval_lookup(-1, &[10]).is_err());
    assert!(eval_lookup(2, &[10]).is_err());
}

#[test]
fn evaluation_budget_is_bounded() {
    let mut budget = EvaluationBudget::new();
    assert_eq!(budget.consume(9_000), Ok(()));
    assert_eq!(budget.remaining(), 1_000);
    assert_eq!(budget.consume(1_001), Err("expression evaluation budget exceeded"));
    assert_eq!(budget.remaining(), 1_000);
    assert_eq!(
        parse_parameter_function_expression_budgeted("max($0, 9)", &[4], &mut budget),
        Ok(9)
    );
}

#[test]
fn malformed_expression_corpus_never_panics() {
    let malformed = [
        "",
        "(",
        ")",
        "$",
        "$x",
        "$99",
        "1 +",
        "1 2",
        "1 ** 2",
        "min(1)",
        "lookup(0)",
        "clamp(1, 2)",
        "1 / 0",
        "1 << 64",
        "unknown(1)",
    ];
    for expression in malformed {
        assert!(parse_parameter_expression(expression, &[1]).is_err(), "{expression}");
        assert!(parse_parameter_function_expression(expression, &[1]).is_err(), "{expression}");
        assert!(parse_parameter_expression_full(expression, &[1]).is_err(), "{expression}");
    }
    let oversized = "(".repeat(1024);
    assert!(parse_parameter_expression(&oversized, &[1]).is_err());
    assert!(parse_parameter_function_expression(&oversized, &[1]).is_err());
    assert!(parse_parameter_expression_full(&oversized, &[1]).is_err());
}

#[test]
fn serial_wins_over_renumbering_and_name_changes() {
    let endpoints = vec![endpoint("A", "renamed"), endpoint("B", "same")];
    let selector = AliasSelector {
        alias: "reflex".into(),
        serial: Some("A".into()),
        vid_pid: Some((1, 2)),
        interface: Some(0),
        name_pattern: Some("missing".into()),
    };
    let (result, matches) = resolve_alias(&selector, &endpoints);
    assert_eq!(result, Resolution::Matched);
    assert_eq!(matches[0].serial.as_deref(), Some("A"));
}

#[test]
fn duplicate_identity_is_ambiguous() {
    let endpoints = vec![endpoint("A", "x"), endpoint("B", "x")];
    let selector = AliasSelector {
        alias: "device".into(),
        serial: None,
        vid_pid: Some((1, 2)),
        interface: Some(0),
        name_pattern: Some("x".into()),
    };
    assert_eq!(resolve_alias(&selector, &endpoints).0, Resolution::Ambiguous);
}

#[test]
fn reconnect_backoff_caps_and_resets() {
    let mut backoff = ReconnectBackoff::default();
    assert_eq!(backoff.next_delay_ms(), 250);
    assert_eq!(backoff.next_delay_ms(), 500);
    for _ in 0..10 {
        let _ = backoff.next_delay_ms();
    }
    assert_eq!(backoff.next_delay_ms(), 10_000);
    backoff.reset();
    assert_eq!(backoff.next_delay_ms(), 250);
}

#[test]
fn reconnect_controller_emits_transitions_and_resets_on_online() {
    let mut controller = ReconnectController::default();
    let (transition, retry) = controller.observe("reflex", Resolution::Missing);
    assert_eq!(transition.expect("transition").current, EndpointState::Offline);
    assert_eq!(retry, Some(250));
    let (_, retry) = controller.observe("reflex", Resolution::Matched);
    assert_eq!(retry, None);
    let (_, retry) = controller.observe("reflex", Resolution::Missing);
    assert_eq!(retry, Some(250));
}

#[test]
fn alias_registry_round_trips_and_creates_backup() {
    let path = std::env::temp_dir().join(format!("mackes-alias-{}.json", std::process::id()));
    let registry = AliasRegistry {
        aliases: vec![AliasSelector {
            alias: "reflex".into(),
            serial: None,
            vid_pid: None,
            interface: None,
            name_pattern: None,
        }],
    };
    registry.save(&path).expect("first save");
    registry.save(&path).expect("second save");
    assert_eq!(AliasRegistry::load(&path).expect("load"), registry);
    assert!(path.with_extension("bak").exists());
    assert!(AliasRegistry {
        aliases: vec![
            AliasSelector { alias: "reflex".into(), ..registry.aliases[0].clone() },
            registry.aliases[0].clone()
        ]
    }
    .validate()
    .is_err());
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(
        std::env::temp_dir().join(format!("mackes-alias-{}.bak", std::process::id())),
    );
}

#[test]
fn state_tracker_emits_only_real_changes() {
    let mut tracker = StateTracker::default();
    assert_eq!(tracker.update("reflex", EndpointState::Online).expect("initial").previous, None);
    assert!(tracker.update("reflex", EndpointState::Online).is_none());
    assert_eq!(
        tracker.update("reflex", EndpointState::Offline).expect("change").previous,
        Some(EndpointState::Online)
    );
}

#[test]
fn effect_colors_have_stable_non_color_markers() {
    assert_eq!(effect_color(EffectType::Reverb), ColorToken::Reverb);
    assert_eq!(ColorToken::Modulation.presentation().1, "[MOD]");
    assert_eq!(ColorToken::Hazard.presentation().1, "[!]");
    assert_eq!(ColorToken::Reverb.ansi16(), 12);
    assert_eq!(ColorToken::Hazard.ansi256(), 196);
    assert_eq!(ColorIntensity::Dim.marker(), "(off)");
    assert_eq!(ColorIntensity::Hazard.marker(), "(!)");
    assert_eq!(
        ColorToken::ALL.map(ColorToken::name),
        ["reverb", "modulation", "cabinet", "neutral", "hazard"]
    );
    assert_eq!(
        launch_control_led_state(ColorToken::Reverb, ColorIntensity::Normal),
        LedState::new(LedColor::Amber, 64, false)
    );
    assert_eq!(
        launch_control_led_state(ColorToken::Neutral, ColorIntensity::Dim),
        LedState::new(LedColor::Off, 0, false)
    );
    assert_eq!(
        launch_control_led_state(ColorToken::Hazard, ColorIntensity::Hazard),
        LedState::new(LedColor::Red, 127, false)
    );
}
