use mackes_config::NovationDeviceConfig;
use serde_json::Value;

pub fn device_snapshot(
    physical_devices: &Value,
    profile_bindings: &[(String, String)],
    binding_generation: u64,
    policy: Option<&NovationDeviceConfig>,
) -> mackes_profiles::LaunchControlDeviceSnapshot {
    let mut snapshot = mackes_profiles::launch_control_device_snapshot();
    snapshot.binding_generation = binding_generation;
    let mut launch_control_present = false;
    if let Some(device) = physical_devices.as_array().and_then(|devices| {
        devices.iter().find(|device| {
            device
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| name.to_ascii_lowercase().contains("launch control"))
        })
    }) {
        launch_control_present = true;
        let state = device.get("state").and_then(Value::as_str);
        snapshot.lifecycle = match state {
            Some("ambiguous") => mackes_profiles::LaunchControlLifecycle::Ambiguous,
            Some(state) if state != "online" => mackes_profiles::LaunchControlLifecycle::Present,
            _ => snapshot.lifecycle,
        };
        for (key, default_role) in [
            ("inputs", mackes_profiles::LaunchControlEndpointRole::Midi),
            ("outputs", mackes_profiles::LaunchControlEndpointRole::Midi),
        ] {
            if let Some(endpoints) = device.get(key).and_then(Value::as_array) {
                for endpoint in endpoints.iter().filter_map(Value::as_str) {
                    let role = if endpoint.to_ascii_lowercase().contains("hui") {
                        mackes_profiles::LaunchControlEndpointRole::Hui
                    } else {
                        default_role
                    };
                    snapshot.endpoints.push((role, endpoint.to_owned()));
                }
            }
        }
    }
    if let Some(stable_id) = policy.and_then(|value| value.stable_id.clone()) {
        snapshot.stable_id = Some(stable_id);
        if launch_control_present || !snapshot.endpoints.is_empty() {
            snapshot.lifecycle = mackes_profiles::LaunchControlLifecycle::Ready;
        }
    }
    if let Some((_, endpoint)) =
        profile_bindings.iter().find(|(profile, _)| profile == "launch-control-xl-mk2")
    {
        snapshot.stable_id =
            policy.and_then(|value| value.stable_id.clone()).or_else(|| Some(endpoint.clone()));
        snapshot.lifecycle = mackes_profiles::LaunchControlLifecycle::Ready;
        snapshot
            .endpoints
            .push((mackes_profiles::LaunchControlEndpointRole::Midi, endpoint.clone()));
    }
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_policy_identity_projects_for_present_controller_without_profile_binding() {
        let policy = NovationDeviceConfig {
            version: 1,
            stable_id: Some("novation:1235:0061".into()),
            template: 1,
            auto_reapply: true,
            feedback_enabled: true,
        };
        let devices = serde_json::json!([{
            "name": "Launch Control XL",
            "state": "connected",
            "inputs": ["midir-in"],
            "outputs": ["midir-out"]
        }]);
        let snapshot = device_snapshot(&devices, &[], 4, Some(&policy));
        assert_eq!(snapshot.stable_id.as_deref(), Some("novation:1235:0061"));
        assert_eq!(snapshot.binding_generation, 4);
        assert_eq!(snapshot.lifecycle, mackes_profiles::LaunchControlLifecycle::Ready);
    }
}
