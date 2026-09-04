//! Startup restore and active-scene persistence helpers.

use std::path::Path;

use mackes_config::{load, ConfigDocument, ConfigError};

use super::RestoreResult;

/// Loads persisted state and computes automatic startup restore behavior.
///
/// # Errors
///
/// Returns a configuration error when persisted state cannot be loaded or validated.
pub fn startup_restore(path: &Path) -> Result<RestoreResult, ConfigError> {
    let document: ConfigDocument = load(path)?;
    let active_project = document.settings.active_project.clone();
    let active_scene = document.settings.active_scene.clone();
    let scenes = match active_project.as_deref() {
        None => {
            if active_scene.is_some() {
                return Err(ConfigError::Semantic {
                    path: path.to_owned(),
                    message: "active scene requires an active project".into(),
                });
            }
            Vec::new()
        }
        Some(project_id) => match document.projects.iter().find(|project| project.id == project_id)
        {
            Some(project) => {
                if let Some(scene) = active_scene.as_deref() {
                    if !project.scenes.iter().any(|entry| entry.id == scene) {
                        return Err(ConfigError::Semantic {
                            path: path.to_owned(),
                            message: format!(
                                "active scene '{scene}' is not present in the active project"
                            ),
                        });
                    }
                }
                project.scenes.iter().map(|scene| scene.id.clone()).collect()
            }
            None => {
                return Err(ConfigError::Semantic {
                    path: path.to_owned(),
                    message: format!(
                        "active project '{project_id}' is not present in the configuration"
                    ),
                })
            }
        },
    };
    let should_activate = active_project.is_some() && !scenes.is_empty();
    Ok(RestoreResult {
        active_project,
        active_scene,
        scenes,
        should_activate,
        unsafe_actions_blocked: usize::from(should_activate),
    })
}

/// Compiles one persisted scene's actions into the ordinary activation planner.
///
/// # Errors
///
/// Returns the planner validation error when the action graph is invalid.
#[cfg(target_os = "linux")]
pub fn compile_scene_actions(
    scene: &mackes_config::SceneRef,
) -> Result<mackes_scene_engine::ActivationPlan, &'static str> {
    mackes_scene_engine::ActivationPlan::compile(
        scene
            .actions
            .iter()
            .map(|action| mackes_scene_engine::ActivationAction {
                id: action.id.clone(),
                description: action.description.clone(),
                unsafe_action: action.unsafe_action,
                depends_on: action.depends_on.clone(),
                destination: action.destination.clone(),
                message: action.message.clone(),
            })
            .collect(),
    )
}

/// Persists an accepted active-scene selection through validated atomic config saving.
///
/// # Errors
///
/// Returns the load, validation, or atomic-save error without modifying daemon state.
#[cfg(target_os = "linux")]
pub fn persist_active_scene(path: &Path, scene: Option<&str>) -> Result<(), String> {
    let document = mackes_config::load(path).map_err(|error| error.to_string())?;
    let updated = mackes_config::set_active_scene(&document, scene)?;
    mackes_config::save(path, &updated, 10).map_err(|error| error.to_string())
}
