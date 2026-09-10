//! Pure projection of persisted v2 layers onto runtime v1 mappings.
use mackes_config::{ControlMapping, ControlMappingStore, MappingLayersV2};

pub fn select_layer(
    current: Option<&MappingLayersV2>,
    expected: u64,
    generation: u64,
    active: Option<String>,
) -> Result<(MappingLayersV2, u64), String> {
    if generation != expected {
        return Err("mapping generation conflict".into());
    }
    let Some(layers) = current else { return Err("mapping layers are not configured".into()) };
    let mut next = layers.clone();
    next.active_layer = active;
    next.validate()?;
    Ok((next, generation.saturating_add(1)))
}

pub fn effective_mappings(
    store: &ControlMappingStore,
    layers: Option<&MappingLayersV2>,
) -> Vec<ControlMapping> {
    let Some(layers) = layers else { return store.active.clone() };
    let selected = layers.effective_layer();
    store
        .active
        .iter()
        .flat_map(|mapping| {
            let Some(destinations) = selected
                .control_id
                .eq_ignore_ascii_case(&mapping.physical_control_id)
                .then_some(&selected.destinations)
                .filter(|items| !items.is_empty())
            else {
                return vec![mapping.clone()];
            };
            destinations
                .iter()
                .map(|destination| {
                    let mut projected = mapping.clone();
                    projected.id = format!("{}::{}", mapping.id, destination.id);
                    projected.destination_endpoint.clone_from(&destination.endpoint);
                    projected.destination_profile.clone_from(&destination.profile);
                    projected.destination_effect.clone_from(&destination.effect);
                    projected.destination_parameter.clone_from(&destination.parameter);
                    projected.destination_channel = Some(destination.channel.saturating_sub(1));
                    projected.behavior.clone_from(&destination.behavior);
                    projected
                })
                .collect()
        })
        .collect()
}

pub fn persist(
    path: Option<&std::path::Path>,
    operation: mackes_ipc::MappingOperation,
    layers: Option<&MappingLayersV2>,
    store: &ControlMappingStore,
) -> bool {
    let Some(path) = path else { return false };
    if matches!(operation, mackes_ipc::MappingOperation::SelectLayer) {
        layers.is_some_and(|value| mackes_config::save_mapping_layers(path, value, 1).is_ok())
    } else {
        mackes_config::save_control_mapping_store(path, store, 1).is_ok()
    }
}
