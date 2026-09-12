use std::{
    io,
    path::{Path, PathBuf},
};

pub(crate) fn routes_path(config_path: &Path) -> PathBuf {
    config_path.with_extension("routes.json")
}

pub(crate) fn routes_undo_path(config_path: &Path) -> PathBuf {
    config_path.with_extension("routes.undo.json")
}

pub(crate) fn persist_routes(config_path: &Path, routes: &serde_json::Value) -> io::Result<()> {
    super::persistence_projection::persist_json_atomic(
        &routes_path(config_path),
        routes,
        "routes.json",
    )
}
