//! `PiPedal` request branches kept outside the daemon's main IPC dispatcher.

use mackes_ipc::PiPedalRequest;

/// Handles bounded readback and preset-save requests carried by `Apply`.
pub fn apply_readback_or_preset(
    worker: &mut mackes_pipedal_adapter::Worker,
    request: &PiPedalRequest,
) -> Option<serde_json::Value> {
    if request.query_show_status_monitor == Some(true) {
        let reply_to = worker.allocate_reply_id();
        return Some(match worker.query_show_status_monitor(request.generation, Some(reply_to)) {
            Ok(()) => {
                serde_json::json!({"ok": true, "queued": true, "query": "show_status_monitor", "generation": request.generation})
            }
            Err(error) => serde_json::json!({"ok": false, "error": error}),
        });
    }
    if let Some(preset_instance_id) = request.load_preset_instance_id {
        return Some(
            match worker.apply_load_preset(
                request.generation,
                preset_instance_id,
                None,
                request.confirm,
            ) {
                Ok(()) => {
                    serde_json::json!({"ok": true, "queued": true, "operation": "loadPreset", "generation": request.generation})
                }
                Err(error) => serde_json::json!({"ok": false, "error": error}),
            },
        );
    }
    if let (Some(bank), Some(name), Some(after)) =
        (request.bank_instance_id, request.preset_name.clone(), request.save_after_instance_id)
    {
        let reply_to = worker.allocate_reply_id();
        return Some(
            match worker.apply_save_current_preset_as(
                request.generation,
                bank,
                name,
                after,
                Some(reply_to),
                request.confirm,
            ) {
                Ok(()) => {
                    serde_json::json!({"ok": true, "queued": true, "operation": "saveCurrentPresetAs", "generation": request.generation})
                }
                Err(error) => serde_json::json!({"ok": false, "error": error}),
            },
        );
    }
    if let (Some(instance), Some(name)) =
        (request.plugin_instance_id, request.plugin_preset_name.clone())
    {
        let reply_to = worker.allocate_reply_id();
        return Some(
            match worker.apply_save_plugin_preset_as(
                request.generation,
                instance,
                name,
                Some(reply_to),
                request.confirm,
            ) {
                Ok(()) => {
                    serde_json::json!({"ok": true, "queued": true, "operation": "savePluginPresetAs", "generation": request.generation})
                }
                Err(error) => serde_json::json!({"ok": false, "error": error}),
            },
        );
    }
    if let Some(favorites) = request.favorites.clone() {
        if !request.confirm {
            return Some(
                serde_json::json!({"ok": false, "error": "PiPedal favorites require confirmation"}),
            );
        }
        return Some(match worker.apply_set_favorites(request.generation, favorites) {
            Ok(()) => {
                serde_json::json!({"ok": true, "applied": true, "favorites": true, "generation": request.generation})
            }
            Err(error) => serde_json::json!({"ok": false, "error": error}),
        });
    }
    None
}
