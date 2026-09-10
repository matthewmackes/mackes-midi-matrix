//! Bounded persistence helpers for scene editing.
use std::path::Path;

pub fn replace_actions(
    path: &Path,
    scene_id: &str,
    actions_value: &serde_json::Value,
) -> Result<(), String> {
    let mut document = mackes_config::load(path).map_err(|error| error.to_string())?;
    let actions = serde_json::from_value::<Vec<mackes_config::SceneAction>>(actions_value.clone())
        .map_err(|_| "invalid scene actions".to_owned())?;
    if actions.len() > 128 {
        return Err("scene has more than 128 actions".to_owned());
    }
    let project_id = document
        .settings
        .active_project
        .clone()
        .or_else(|| document.projects.first().map(|project| project.id.clone()))
        .ok_or_else(|| "no active project is configured".to_owned())?;
    let project = document
        .projects
        .iter_mut()
        .find(|project| project.id == project_id)
        .ok_or_else(|| "active project was not found".to_owned())?;
    let scene = project
        .scenes
        .iter_mut()
        .find(|scene| scene.id == scene_id)
        .ok_or_else(|| "scene was not found".to_owned())?;
    scene.actions = actions;
    mackes_config::validate(&document)?;
    mackes_config::save(path, &document, 1).map_err(|error| error.to_string())
}

/// Replaces one complete project through the daemon-owned configuration path.
/// The candidate is validated before the atomic configuration save, so failed
/// edits leave the previously committed project untouched.
pub fn replace_project(path: &Path, project_value: &serde_json::Value) -> Result<String, String> {
    let mut document = mackes_config::load(path).map_err(|error| error.to_string())?;
    let project = serde_json::from_value::<mackes_config::Project>(project_value.clone())
        .map_err(|_| "invalid project document".to_owned())?;
    let id = project.id.clone();
    let candidate = mackes_config::replace_project(&document, project)?;
    mackes_config::validate(&candidate)?;
    document = candidate;
    mackes_config::save(path, &document, 1).map_err(|error| error.to_string())?;
    Ok(id)
}

pub fn replace_setlist(path: &Path, setlist_value: &serde_json::Value) -> Result<String, String> {
    let mut document = mackes_config::load(path).map_err(|error| error.to_string())?;
    let setlist = serde_json::from_value::<mackes_config::Setlist>(setlist_value.clone())
        .map_err(|_| "invalid setlist document".to_owned())?;
    let id = setlist.id.clone();
    if !document.setlists.iter().any(|candidate| candidate.id == id) {
        return Err(format!("unknown setlist ID '{id}'"));
    }
    setlist.validate_against(&document.projects)?;
    let index = document
        .setlists
        .iter()
        .position(|candidate| candidate.id == id)
        .ok_or_else(|| "setlist replacement target disappeared".to_owned())?;
    document.setlists[index] = setlist;
    mackes_config::validate(&document)?;
    mackes_config::save(path, &document, 1).map_err(|error| error.to_string())?;
    Ok(id)
}

pub fn create_setlist(path: &Path, setlist_value: &serde_json::Value) -> Result<String, String> {
    let mut document = mackes_config::load(path).map_err(|error| error.to_string())?;
    let setlist = serde_json::from_value::<mackes_config::Setlist>(setlist_value.clone())
        .map_err(|_| "invalid setlist document".to_owned())?;
    let id = setlist.id.clone();
    if document.setlists.iter().any(|candidate| candidate.id == id) {
        return Err(format!("setlist '{id}' already exists"));
    }
    setlist.validate_against(&document.projects)?;
    document.setlists.push(setlist);
    mackes_config::validate(&document)?;
    mackes_config::save(path, &document, 1).map_err(|error| error.to_string())?;
    Ok(id)
}

pub fn delete_setlist(path: &Path, id: &str) -> Result<String, String> {
    let mut document = mackes_config::load(path).map_err(|error| error.to_string())?;
    let index = document
        .setlists
        .iter()
        .position(|candidate| candidate.id == id)
        .ok_or_else(|| format!("unknown setlist ID '{id}'"))?;
    document.setlists.remove(index);
    mackes_config::validate(&document)?;
    mackes_config::save(path, &document, 1).map_err(|error| error.to_string())?;
    Ok(id.to_owned())
}

pub fn copy_project(path: &Path, value: &serde_json::Value) -> Result<String, String> {
    let mut document = mackes_config::load(path).map_err(|error| error.to_string())?;
    let source = value
        .get("source")
        .and_then(serde_json::Value::as_str)
        .ok_or("project copy source is required")?;
    let new_id = value
        .get("new_id")
        .and_then(serde_json::Value::as_str)
        .ok_or("project copy new_id is required")?;
    let project = mackes_config::copy_project(&document.projects, source, new_id)?;
    let id = project.id.clone();
    document.projects.push(project);
    mackes_config::validate(&document)?;
    mackes_config::save(path, &document, 1).map_err(|error| error.to_string())?;
    Ok(id)
}

pub fn copy_setlist(path: &Path, value: &serde_json::Value) -> Result<String, String> {
    let mut document = mackes_config::load(path).map_err(|error| error.to_string())?;
    let source = value
        .get("source")
        .and_then(serde_json::Value::as_str)
        .ok_or("setlist copy source is required")?;
    let new_id = value
        .get("new_id")
        .and_then(serde_json::Value::as_str)
        .ok_or("setlist copy new_id is required")?;
    let copy = mackes_config::copy_setlist(&document.setlists, source, new_id)?;
    let id = copy.id.clone();
    document.setlists.push(copy);
    mackes_config::validate(&document)?;
    mackes_config::save(path, &document, 1).map_err(|error| error.to_string())?;
    Ok(id)
}

pub fn preview_scene(path: &Path, scene_id: &str) -> Result<serde_json::Value, String> {
    let document = mackes_config::load(path).map_err(|error| error.to_string())?;
    let project_id = document
        .settings
        .active_project
        .as_deref()
        .or_else(|| document.projects.first().map(|project| project.id.as_str()))
        .ok_or("no active project is configured")?;
    let project = document
        .projects
        .iter()
        .find(|project| project.id == project_id)
        .ok_or("active project was not found")?;
    let scene =
        project.scenes.iter().find(|scene| scene.id == scene_id).ok_or("scene was not found")?;
    let plan = crate::compile_scene_actions(scene).map_err(|_| "scene plan is invalid")?;
    Ok(
        serde_json::json!({"project": project.id, "scene": scene.id, "actions": plan.actions.iter().map(|action| serde_json::json!({"id": action.id, "description": action.description, "unsafe": action.unsafe_action, "depends_on": action.depends_on})).collect::<Vec<_>>() }),
    )
}

/// Build an ordered, non-mutating recall plan for a daemon-owned setlist.
pub fn preview_setlist(path: &Path, setlist_id: &str) -> Result<serde_json::Value, String> {
    let document = mackes_config::load(path).map_err(|error| error.to_string())?;
    let setlist = document
        .setlists
        .iter()
        .find(|setlist| setlist.id == setlist_id)
        .ok_or("setlist was not found")?;
    let mut projects = Vec::with_capacity(setlist.projects.len());
    for project_id in &setlist.projects {
        let project = document
            .projects
            .iter()
            .find(|project| project.id == *project_id)
            .ok_or_else(|| format!("setlist project was not found: {project_id}"))?;
        let scenes = project
            .scenes
            .iter()
            .map(|scene| {
                serde_json::json!({
                    "id": scene.id,
                    "name": scene.name,
                    "action_count": scene.actions.len(),
                })
            })
            .collect::<Vec<_>>();
        projects.push(serde_json::json!({"id": project.id, "scenes": scenes}));
    }
    Ok(serde_json::json!({"setlist": setlist.id, "projects": projects}))
}
