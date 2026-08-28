//! Versioned JSON5 configuration, semantic validation, migration, and atomic persistence.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// Current configuration schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Result of validating a configuration path for CLI/TUI presentation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ValidationReport {
    /// Whether validation succeeded.
    pub valid: bool,
    /// Stable status category.
    pub status: String,
    /// Human-readable remediation or success message.
    pub message: String,
}

/// Complete persisted document root.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigDocument {
    /// Schema version.
    pub schema_version: u32,
    /// Application settings.
    #[serde(default)]
    pub settings: Settings,
    /// Stable local endpoint aliases.
    #[serde(default)]
    pub endpoints: Vec<EndpointAlias>,
    /// Projects and scene references.
    #[serde(default)]
    pub projects: Vec<Project>,
    /// Declarative device profiles.
    #[serde(default)]
    pub profiles: Vec<ProfileRef>,
    /// Ordered project setlists.
    #[serde(default)]
    pub setlists: Vec<Setlist>,
    /// Durable one-source/one-destination mappings created by MIDI Learn.
    #[serde(default)]
    pub learned_mappings: Vec<LearnedMapping>,
}

/// Verification state for a device backup artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BackupStatus {
    /// Payload was sent and read back successfully.
    Verified,
    /// Payload was sent but read-back was unavailable.
    SentUnverified,
    /// Restore or integrity validation failed.
    Failed,
}

/// Immutable metadata accompanying a backup payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BackupManifest {
    /// Profile identifier and version.
    pub profile: String,
    /// Device identity summary.
    pub device_identity: String,
    /// Source endpoint alias.
    pub source_alias: String,
    /// Capture timestamp in service nanoseconds.
    pub captured_at: u64,
    /// Lowercase SHA-256 digest of the payload.
    pub sha256: String,
    /// Verification result.
    pub status: BackupStatus,
}

impl BackupManifest {
    /// Validates required metadata and digest shape.
    ///
    /// # Errors
    ///
    /// Returns an error when identity fields are empty or the digest is not
    /// exactly 64 lowercase hexadecimal characters.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.profile.trim().is_empty()
            || self.device_identity.trim().is_empty()
            || self.source_alias.trim().is_empty()
        {
            return Err("backup identity fields must not be empty");
        }
        if self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("backup SHA-256 digest is invalid");
        }
        Ok(())
    }

    /// Computes a lowercase SHA-256 digest for a payload.
    #[must_use]
    pub fn digest(payload: &[u8]) -> String {
        let digest = Sha256::digest(payload);
        let mut encoded = String::with_capacity(64);
        for byte in digest {
            let _ = fmt::Write::write_fmt(&mut encoded, format_args!("{byte:02x}"));
        }
        encoded
    }

    /// Verifies a payload against this manifest's digest.
    #[must_use]
    pub fn matches_payload(&self, payload: &[u8]) -> bool {
        self.sha256 == Self::digest(payload)
    }
}

/// Atomically stores a backup payload and its validated manifest sidecar.
///
/// # Errors
///
/// Returns an error when the manifest is invalid, does not match the payload,
/// or either file cannot be written.
pub fn save_backup(path: &Path, payload: &[u8], manifest: &BackupManifest) -> Result<(), String> {
    manifest.validate().map_err(str::to_owned)?;
    if !manifest.matches_payload(payload) {
        return Err("backup payload digest does not match manifest".into());
    }
    let manifest_path = path.with_extension("manifest.json");
    if path.exists() || manifest_path.exists() {
        return Err("backup artifact already exists and is immutable".into());
    }
    let payload_tmp = path.with_extension("payload.tmp");
    let manifest_tmp = path.with_extension("manifest.tmp");
    fs::write(&payload_tmp, payload).map_err(|error| error.to_string())?;
    let encoded = serde_json::to_vec_pretty(manifest).map_err(|error| error.to_string())?;
    fs::write(&manifest_tmp, encoded).map_err(|error| error.to_string())?;
    fs::rename(payload_tmp, path).map_err(|error| error.to_string())?;
    fs::rename(manifest_tmp, manifest_path).map_err(|error| error.to_string())
}

/// Loads and verifies a backup payload and its manifest sidecar.
///
/// # Errors
///
/// Returns an error for missing/malformed files, invalid metadata, or digest mismatch.
pub fn load_backup(path: &Path) -> Result<(Vec<u8>, BackupManifest), String> {
    let payload = fs::read(path).map_err(|error| error.to_string())?;
    let manifest_path = path.with_extension("manifest.json");
    let manifest: BackupManifest =
        serde_json::from_slice(&fs::read(manifest_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    manifest.validate().map_err(str::to_owned)?;
    if !manifest.matches_payload(&payload) {
        return Err("backup payload digest mismatch".into());
    }
    Ok((payload, manifest))
}

/// Checks restore compatibility against the expected profile.
///
/// Device identity differences are intentionally warnings, not hard failures, per the
/// operator-selected restore policy; callers must surface [`backup_identity_warning`].
#[must_use]
pub fn backup_compatible(manifest: &BackupManifest, profile: &str, device_identity: &str) -> bool {
    let _ = device_identity;
    manifest.profile == profile && manifest.status != BackupStatus::Failed
}

/// Returns whether a restore should show an identity-mismatch warning.
#[must_use]
pub fn backup_identity_warning(manifest: &BackupManifest, device_identity: &str) -> bool {
    manifest.device_identity != device_identity
}

/// Whether a verified backup should be inspected only or written to its target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestoreMode {
    /// Validate and plan the restore without changing the target.
    DryRun,
    /// Atomically replace the target with the verified payload.
    Apply,
}

/// Outcome of a compatibility-gated restore operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RestoreResult {
    /// The backup passed all checks and was not written.
    Planned {
        /// Number of verified payload bytes.
        bytes: usize,
        /// Whether the source device identity differs from the target.
        identity_warning: bool,
        /// Verification state recorded by the backup manifest.
        status: BackupStatus,
    },
    /// The backup passed all checks and replaced the target.
    Applied {
        /// Number of verified payload bytes.
        bytes: usize,
        /// Whether the source device identity differs from the target.
        identity_warning: bool,
        /// Verification state recorded by the backup manifest.
        status: BackupStatus,
    },
}

/// Loads a backup, verifies its digest and identity, then optionally applies it atomically.
///
/// This primitive deliberately performs no device transmission; callers layer paced MIDI
/// sending and read-back verification on top of the verified payload.
///
/// # Errors
///
/// Returns an error when the artifact is missing, malformed, corrupt, incompatible, or cannot
/// be atomically written to the target.
pub fn restore_backup(
    backup: &Path,
    target: &Path,
    profile: &str,
    device_identity: &str,
    mode: RestoreMode,
) -> Result<RestoreResult, String> {
    let (payload, manifest) = load_backup(backup)?;
    let identity_warning = backup_identity_warning(&manifest, device_identity);
    if !backup_compatible(&manifest, profile, device_identity) {
        return Err("backup is incompatible with the selected device".into());
    }
    if mode == RestoreMode::DryRun {
        return Ok(RestoreResult::Planned {
            bytes: payload.len(),
            identity_warning,
            status: manifest.status,
        });
    }
    let temporary = target.with_extension("restore.tmp");
    fs::write(&temporary, &payload).map_err(|error| error.to_string())?;
    if let Err(error) = fs::rename(&temporary, target) {
        let _ = fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    Ok(RestoreResult::Applied { bytes: payload.len(), identity_warning, status: manifest.status })
}

/// Backward-compatible short root used by initial bootstrap callers.
pub type AppConfig = ConfigDocument;

/// Application settings.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// Last active project ID.
    #[serde(default)]
    pub active_project: Option<String>,
    /// Last active scene ID within the active project.
    #[serde(default)]
    pub active_scene: Option<String>,
    /// Preferred device profile for each effect capability.
    #[serde(default)]
    pub default_providers: Vec<DefaultProvider>,
    /// Globally selected endpoint alias used by MIDI Learn.
    #[serde(default)]
    pub learn_input_alias: Option<String>,
    /// Explicit MIDI triggers for dashboard commands.
    #[serde(default)]
    pub dashboard_midi_bindings: Vec<DashboardMidiBinding>,
}

/// Persisted MIDI trigger kind for a dashboard action.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DashboardMidiTrigger {
    /// Match an exact note-on channel and note.
    NoteOn {
        /// One-based MIDI channel.
        channel: u8,
        /// Note number.
        note: u8,
    },
    /// Match an exact CC channel/controller and optionally its value.
    ControlChange {
        /// One-based MIDI channel.
        channel: u8,
        /// Controller number.
        controller: u8,
        /// Optional exact value.
        value: Option<u8>,
    },
}

/// Persisted one-to-one MIDI dashboard action binding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DashboardMidiBinding {
    /// Explicit trigger.
    pub trigger: DashboardMidiTrigger,
    /// Governed command tag (`panic`, `next_scene`, or `previous_scene`).
    pub command: String,
}

impl DashboardMidiBinding {
    /// Validates MIDI ranges, command allowlisting, and nonblank identifiers.
    ///
    /// # Errors
    ///
    /// Returns an error when the trigger is out of range or the command is not
    /// one of the supported dashboard commands.
    pub fn validate(&self) -> Result<(), &'static str> {
        let valid_trigger = match &self.trigger {
            DashboardMidiTrigger::NoteOn { channel, note } => {
                (1..=16).contains(channel) && *note <= 127
            }
            DashboardMidiTrigger::ControlChange { channel, controller, value } => {
                (1..=16).contains(channel)
                    && *controller <= 127
                    && value.is_none_or(|value| value <= 127)
            }
        };
        if !valid_trigger {
            return Err("dashboard MIDI trigger is out of range");
        }
        if !matches!(self.command.as_str(), "panic" | "next_scene" | "previous_scene") {
            return Err("dashboard MIDI command is not allowed");
        }
        Ok(())
    }
}

/// Explicit channel matching policy for a persisted learned mapping.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LearnedChannelPolicy {
    /// Match only the captured one-based MIDI channel.
    Exact(u8),
    /// Match the source signature on any MIDI channel.
    Any,
    /// The source message has no MIDI channel.
    NotApplicable,
}

/// Serializable filter captured alongside a learned mapping.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[allow(missing_docs)]
pub enum LearnedFilter {
    NumberRange { minimum: u8, maximum: u8 },
    ValueRange { minimum: u16, maximum: u16 },
    Realtime { message: LearnedRealtime },
    SysExMask { pattern: Vec<u8>, mask: Vec<u8> },
}

/// MIDI real-time values supported by persisted learned filters.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum LearnedRealtime {
    Clock,
    Start,
    Continue,
    Stop,
    ActiveSensing,
    Reset,
}

impl LearnedFilter {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::NumberRange { minimum, maximum } if minimum > maximum => {
                Err("learned filter number range must be ordered".into())
            }
            Self::ValueRange { minimum, maximum } if minimum > maximum || *maximum > 16_383 => {
                Err("learned filter value range must be 0..=16383 and ordered".into())
            }
            Self::SysExMask { pattern, mask }
                if pattern.is_empty() || pattern.len() != mask.len() =>
            {
                Err("learned SysEx filter requires equal non-empty pattern and mask".into())
            }
            Self::SysExMask { pattern, mask }
                if pattern.len() > 1_024 || pattern.iter().chain(mask).any(|byte| *byte > 127) =>
            {
                Err("learned SysEx filter bytes must be seven-bit and bounded".into())
            }
            _ => Ok(()),
        }
    }
}

/// Durable evidence and destination for one learned MIDI mapping.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearnedMapping {
    /// Selected input endpoint alias.
    pub source_alias: String,
    /// Stable MIDI message-family name.
    pub message_kind: String,
    /// Explicit channel policy.
    pub channel_policy: LearnedChannelPolicy,
    /// Controller, note, program, or system subtype when applicable.
    pub number: Option<u8>,
    /// Complete captured wire evidence.
    pub raw: Vec<u8>,
    /// Stable destination identifier.
    pub destination: String,
    /// Operator-facing mapping mode.
    pub mode: String,
    /// Whether routing should execute this mapping.
    pub enabled: bool,
    /// Lower values execute first.
    pub priority: u16,
    #[serde(default)]
    #[allow(missing_docs)]
    pub filters: Vec<LearnedFilter>,
}

/// One persistent default device assignment for an effect capability.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultProvider {
    /// Normalized effect capability identifier.
    pub capability: String,
    /// Device profile identifier selected by the operator.
    pub profile_id: String,
}

/// Persistent endpoint identity and user alias.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointAlias {
    /// Stable alias.
    pub id: String,
    /// Backend display name pattern.
    #[serde(default)]
    pub name: Option<String>,
    /// Optional USB vendor ID.
    #[serde(default)]
    pub vendor_id: Option<u16>,
    /// Optional USB product ID.
    #[serde(default)]
    pub product_id: Option<u16>,
    /// Optional serial, never logged.
    #[serde(default)]
    pub serial: Option<String>,
}

/// Project containing ordered scenes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    /// Stable project ID.
    pub id: String,
    /// Ordered scenes.
    #[serde(default)]
    pub scenes: Vec<SceneRef>,
}

/// Ordered setlist of project IDs, kept separate from project contents.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Setlist {
    /// Stable setlist ID.
    pub id: String,
    /// Ordered project references.
    #[serde(default)]
    pub projects: Vec<String>,
}

/// References that must be resolved before a project may be deleted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectReferenceReport {
    /// Project being inspected.
    pub project_id: String,
    /// Setlist IDs that currently reference the project.
    pub setlists: Vec<String>,
    /// Whether the project is the active startup selection.
    pub active: bool,
}

/// Reports all persistent references to a project without mutating configuration.
#[must_use]
pub fn project_reference_report(
    document: &ConfigDocument,
    project_id: &str,
) -> ProjectReferenceReport {
    ProjectReferenceReport {
        project_id: project_id.to_owned(),
        setlists: document
            .setlists
            .iter()
            .filter(|setlist| setlist.projects.iter().any(|id| id == project_id))
            .map(|setlist| setlist.id.clone())
            .collect(),
        active: document.settings.active_project.as_deref() == Some(project_id),
    }
}

/// Removes an unreferenced project after validating the complete resulting document.
///
/// # Errors
///
/// Returns an error when the project is missing, still referenced, or removal would invalidate
/// the document. The original document is never mutated.
pub fn remove_project(
    document: &ConfigDocument,
    project_id: &str,
) -> Result<ConfigDocument, String> {
    let report = project_reference_report(document, project_id);
    if report.active || !report.setlists.is_empty() {
        return Err(format!("project '{project_id}' has unresolved references"));
    }
    let mut candidate = document.clone();
    let Some(index) = candidate.projects.iter().position(|project| project.id == project_id) else {
        return Err(format!("unknown project ID '{project_id}'"));
    };
    candidate.projects.remove(index);
    validate(&candidate)?;
    Ok(candidate)
}

impl Setlist {
    /// Validates identity, ordering uniqueness, and project references.
    ///
    /// # Errors
    ///
    /// Returns an error for blank or duplicate IDs and unknown projects.
    pub fn validate_against(&self, available: &[Project]) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("setlist ID must not be empty".into());
        }
        for (index, project_id) in self.projects.iter().enumerate() {
            if project_id.trim().is_empty() {
                return Err("setlist project ID must not be empty".into());
            }
            if self.projects[..index].iter().any(|prior| prior == project_id) {
                return Err(format!("setlist contains duplicate project ID '{project_id}'"));
            }
            if !available.iter().any(|project| project.id == *project_id) {
                return Err(format!("setlist references unknown project '{project_id}'"));
            }
        }
        Ok(())
    }
}

/// Reorders a setlist using a complete permutation of its project IDs.
///
/// # Errors
///
/// Returns an error when the order omits, duplicates, or adds a project ID.
pub fn reorder_setlist(setlist: &Setlist, order: &[&str]) -> Result<Setlist, String> {
    if order.len() != setlist.projects.len() {
        return Err("setlist order must contain every project exactly once".into());
    }
    let mut reordered = Vec::with_capacity(order.len());
    for project_id in order {
        if !setlist.projects.iter().any(|candidate| candidate == project_id) {
            return Err(format!("unknown setlist project ID '{project_id}'"));
        }
        if reordered.iter().any(|candidate: &String| candidate == project_id) {
            return Err(format!("duplicate setlist project ID '{project_id}'"));
        }
        reordered.push((*project_id).to_owned());
    }
    let mut result = setlist.clone();
    result.projects = reordered;
    Ok(result)
}

/// Finds setlists by case-insensitive ID or referenced project ID.
#[must_use]
pub fn search_setlists<'a>(setlists: &'a [Setlist], query: &str) -> Vec<&'a Setlist> {
    let query = query.to_ascii_lowercase();
    setlists
        .iter()
        .filter(|setlist| {
            setlist.id.to_ascii_lowercase().contains(&query)
                || setlist
                    .projects
                    .iter()
                    .any(|project| project.to_ascii_lowercase().contains(&query))
        })
        .collect()
}

/// Copies one setlist under a new stable ID without mutating the source collection.
///
/// # Errors
///
/// Returns an error when either ID is blank, the source is missing, or the destination exists.
pub fn copy_setlist(
    setlists: &[Setlist],
    source_id: &str,
    new_id: &str,
) -> Result<Setlist, String> {
    if source_id.trim().is_empty() || new_id.trim().is_empty() {
        return Err("setlist IDs must not be empty".into());
    }
    if setlists.iter().any(|setlist| setlist.id == new_id) {
        return Err(format!("setlist '{new_id}' already exists"));
    }
    let mut copy = setlists
        .iter()
        .find(|setlist| setlist.id == source_id)
        .cloned()
        .ok_or_else(|| format!("setlist '{source_id}' was not found"))?;
    new_id.clone_into(&mut copy.id);
    Ok(copy)
}

/// Scene reference within a project.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRef {
    /// Stable scene ID.
    pub id: String,
    /// Optional operator-facing scene name.
    #[serde(default)]
    pub name: Option<String>,
    /// Optional category used by setlist/editor filtering.
    #[serde(default)]
    pub category: Option<String>,
}

/// Replaces one project only after validating the complete resulting document.
///
/// # Errors
///
/// Returns an error when the project ID is missing or the resulting document is invalid.
pub fn replace_project(
    document: &ConfigDocument,
    replacement: Project,
) -> Result<ConfigDocument, String> {
    if !document.projects.iter().any(|project| project.id == replacement.id) {
        return Err(format!("unknown project ID '{}'", replacement.id));
    }
    let mut candidate = document.clone();
    let index = candidate
        .projects
        .iter()
        .position(|project| project.id == replacement.id)
        .ok_or_else(|| "project replacement target disappeared".to_owned())?;
    candidate.projects[index] = replacement;
    validate(&candidate)?;
    Ok(candidate)
}

/// Stages a new active project selection and validates the complete document.
///
/// # Errors
///
/// Returns an error when the project is not present or the resulting document
/// fails semantic validation; the original document is never mutated.
pub fn set_active_project(
    document: &ConfigDocument,
    project_id: Option<&str>,
) -> Result<ConfigDocument, String> {
    if let Some(project_id) = project_id {
        if !document.projects.iter().any(|project| project.id == project_id) {
            return Err(format!("active project reference '{project_id}' is dangling"));
        }
    }
    let mut candidate = document.clone();
    candidate.settings.active_project = project_id.map(str::to_owned);
    validate(&candidate)?;
    Ok(candidate)
}

/// Stages a new active scene selection within the current active project.
///
/// # Errors
///
/// Returns an error for a missing active project or dangling scene reference;
/// the original document is never mutated.
pub fn set_active_scene(
    document: &ConfigDocument,
    scene_id: Option<&str>,
) -> Result<ConfigDocument, String> {
    if let Some(scene_id) = scene_id {
        let project_id = document
            .settings
            .active_project
            .as_deref()
            .ok_or("active scene requires an active project")?;
        let project = document
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| format!("active project reference '{project_id}' is dangling"))?;
        if !project.scenes.iter().any(|scene| scene.id == scene_id) {
            return Err(format!("active scene reference '{scene_id}' is dangling"));
        }
    }
    let mut candidate = document.clone();
    candidate.settings.active_scene = scene_id.map(str::to_owned);
    validate(&candidate)?;
    Ok(candidate)
}

/// Stages a default profile assignment for one effect capability.
///
/// Existing assignments for the normalized capability are replaced atomically.
///
/// # Errors
///
/// Returns an error for blank identifiers or an invalid resulting document.
pub fn set_default_provider(
    document: &ConfigDocument,
    capability: &str,
    profile_id: &str,
) -> Result<ConfigDocument, String> {
    let capability = capability.trim().to_ascii_lowercase();
    let profile_id = profile_id.trim();
    if capability.is_empty() || profile_id.is_empty() {
        return Err("capability and profile ID must not be blank".into());
    }
    let mut candidate = document.clone();
    candidate.settings.default_providers.retain(|entry| entry.capability != capability);
    candidate
        .settings
        .default_providers
        .push(DefaultProvider { capability, profile_id: profile_id.to_owned() });
    candidate
        .settings
        .default_providers
        .sort_by(|left, right| left.capability.cmp(&right.capability));
    validate(&candidate)?;
    Ok(candidate)
}

/// Returns the configured default profile for a normalized effect capability.
#[must_use]
pub fn default_provider<'a>(document: &'a ConfigDocument, capability: &str) -> Option<&'a str> {
    let capability = capability.trim().to_ascii_lowercase();
    document
        .settings
        .default_providers
        .iter()
        .find(|entry| entry.capability == capability)
        .map(|entry| entry.profile_id.as_str())
}

/// Selects the global MIDI Learn input without mutating the source document.
///
/// # Errors
///
/// Returns an error unless the value is a configured endpoint alias and the
/// resulting document is semantically valid.
pub fn set_learn_input_alias(
    document: &ConfigDocument,
    alias: &str,
) -> Result<ConfigDocument, String> {
    let alias = alias.trim();
    if !document.endpoints.iter().any(|endpoint| endpoint.id == alias) {
        return Err(format!("Learn input references unknown endpoint '{alias}'"));
    }
    let mut candidate = document.clone();
    candidate.settings.learn_input_alias = Some(alias.to_owned());
    validate(&candidate)?;
    Ok(candidate)
}

/// Appends one fully reviewed learned mapping transactionally.
///
/// # Errors
///
/// Returns an error for invalid evidence, references, or an exact duplicate;
/// the source document remains unchanged.
pub fn add_learned_mapping(
    document: &ConfigDocument,
    mapping: LearnedMapping,
) -> Result<ConfigDocument, String> {
    if document.learned_mappings.iter().any(|entry| entry == &mapping) {
        return Err("learned mapping is already configured".into());
    }
    let mut candidate = document.clone();
    candidate.learned_mappings.push(mapping);
    validate(&candidate)?;
    Ok(candidate)
}

/// Copies a project under a new stable ID and regenerates each contained scene ID.
///
/// # Errors
///
/// Returns an error when the new project ID is blank, collides with an existing
/// project, or a generated scene ID would collide within the copy.
pub fn copy_project(
    projects: &[Project],
    source_id: &str,
    new_id: &str,
) -> Result<Project, String> {
    if new_id.trim().is_empty() || projects.iter().any(|project| project.id == new_id) {
        return Err("new project ID must be non-empty and unused".into());
    }
    let source = projects
        .iter()
        .find(|project| project.id == source_id)
        .ok_or_else(|| format!("unknown project ID '{source_id}'"))?;
    let scenes = source
        .scenes
        .iter()
        .enumerate()
        .map(|(index, scene)| SceneRef {
            id: format!("{new_id}.scene-{}-{}", index + 1, scene.id),
            name: scene.name.clone(),
            category: scene.category.clone(),
        })
        .collect();
    Ok(Project { id: new_id.to_owned(), scenes })
}

/// Reorders a project's scenes using a complete permutation of existing IDs.
///
/// # Errors
///
/// Returns an error when the order omits, duplicates, or adds an ID.
pub fn reorder_scenes(project: &Project, order: &[&str]) -> Result<Project, String> {
    if order.len() != project.scenes.len() {
        return Err("scene order must contain every scene exactly once".into());
    }
    let mut reordered = Vec::with_capacity(order.len());
    for id in order {
        let scene = project
            .scenes
            .iter()
            .find(|scene| scene.id == *id)
            .ok_or_else(|| format!("unknown scene ID '{id}'"))?;
        if reordered.iter().any(|entry: &SceneRef| entry.id == scene.id) {
            return Err(format!("duplicate scene ID '{id}'"));
        }
        reordered.push(scene.clone());
    }
    let mut result = project.clone();
    result.scenes = reordered;
    Ok(result)
}

/// Copies an existing scene reference under an explicit, unused ID.
///
/// # Errors
///
/// Returns an error when the source is missing or the new ID is empty/already used.
pub fn copy_scene(project: &Project, source_id: &str, new_id: &str) -> Result<Project, String> {
    if new_id.trim().is_empty() || project.scenes.iter().any(|scene| scene.id == new_id) {
        return Err("new scene ID must be non-empty and unused".into());
    }
    if !project.scenes.iter().any(|scene| scene.id == source_id) {
        return Err(format!("unknown scene ID '{source_id}'"));
    }
    let Some(source) = project.scenes.iter().find(|scene| scene.id == source_id) else {
        return Err(format!("unknown scene ID '{source_id}'"));
    };
    let mut result = project.clone();
    result.scenes.push(SceneRef {
        id: new_id.to_owned(),
        name: source.name.clone(),
        category: source.category.clone(),
    });
    Ok(result)
}

/// Finds scenes whose ID, name, or category contains a case-insensitive query, preserving order.
#[must_use]
pub fn search_scenes<'a>(project: &'a Project, query: &str) -> Vec<&'a SceneRef> {
    let query = query.to_ascii_lowercase();
    project
        .scenes
        .iter()
        .filter(|scene| {
            [Some(scene.id.as_str()), scene.name.as_deref(), scene.category.as_deref()]
                .into_iter()
                .flatten()
                .any(|value| value.to_ascii_lowercase().contains(&query))
        })
        .collect()
}

/// Finds projects whose ID or ordered scene IDs contain a case-insensitive query.
#[must_use]
pub fn search_projects<'a>(projects: &'a [Project], query: &str) -> Vec<&'a Project> {
    let query = query.to_ascii_lowercase();
    projects
        .iter()
        .filter(|project| {
            project.id.to_ascii_lowercase().contains(&query)
                || project.scenes.iter().any(|scene| scene.id.to_ascii_lowercase().contains(&query))
        })
        .collect()
}

/// Reference to a profile ID.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileRef {
    /// Stable profile ID.
    pub id: String,
    /// Profile version.
    pub version: u32,
}

/// Actionable configuration failure.
#[derive(Debug)]
pub enum ConfigError {
    /// Filesystem operation failed.
    Io {
        /// File path.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// JSON5 syntax/type/schema failure.
    Parse {
        /// File path.
        path: PathBuf,
        /// Parser message.
        message: String,
    },
    /// Schema version is unsupported.
    Version {
        /// File path.
        path: PathBuf,
        /// Encountered version.
        found: u32,
    },
    /// Semantic reference or identity failure.
    Semantic {
        /// File path.
        path: PathBuf,
        /// Semantic validation message.
        message: String,
    },
    /// Atomic replacement failed.
    Replace {
        /// Temporary or destination path.
        path: PathBuf,
        /// Underlying replacement error.
        source: io::Error,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::Parse { path, message } | Self::Semantic { path, message } => {
                write!(formatter, "invalid configuration {}: {message}", path.display())
            }
            Self::Version { path, found } => {
                write!(formatter, "unsupported schema version {found} in {}", path.display())
            }
            Self::Replace { path, source } => {
                write!(formatter, "cannot atomically replace {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Loads, migrates, deserializes, and semantically validates a JSON5 document.
///
/// # Errors
///
/// Returns an error when the file is unreadable, malformed, unsupported, or semantically invalid.
pub fn load(path: &Path) -> Result<ConfigDocument, ConfigError> {
    let text = fs::read_to_string(path)
        .map_err(|source| ConfigError::Io { path: path.to_owned(), source })?;
    let value: ConfigDocument = json5::from_str(&text).map_err(|error| ConfigError::Parse {
        path: path.to_owned(),
        message: error.to_string(),
    })?;
    if value.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(ConfigError::Version { path: path.to_owned(), found: value.schema_version });
    }
    validate(&value).map_err(|message| ConfigError::Semantic { path: path.to_owned(), message })?;
    Ok(value)
}

/// Applies all known migrations. Version 1 is currently a no-op migration.
///
/// # Errors
///
/// Returns an error if the document is from an unsupported version or fails semantic validation.
pub fn migrate(document: ConfigDocument) -> Result<ConfigDocument, String> {
    if document.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(format!("no migration path from schema version {}", document.schema_version));
    }
    validate(&document)?;
    Ok(document)
}

/// Produces a deterministic human or JSON validation report.
#[must_use]
pub fn validate_report(path: &Path, json_output: bool) -> String {
    let report = match load(path) {
        Ok(_) => ValidationReport {
            valid: true,
            status: "valid".to_owned(),
            message: format!("{} is valid", path.display()),
        },
        Err(error) => ValidationReport {
            valid: false,
            status: "invalid".to_owned(),
            message: error.to_string(),
        },
    };
    if json_output {
        serde_json::to_string(&report)
            .unwrap_or_else(|_| "{\"valid\":false,\"status\":\"internal_error\"}".to_owned())
    } else {
        format!("{}: {}\n", report.status, report.message)
    }
}

/// Validates IDs, duplicates, and project/profile references.
///
/// # Errors
///
/// Returns an error naming the first invalid ID, duplicate, or dangling reference.
pub fn validate(document: &ConfigDocument) -> Result<(), String> {
    if document.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(format!("schema_version must be {CURRENT_SCHEMA_VERSION}"));
    }
    unique(document.endpoints.iter().map(|entry| entry.id.as_str()), "endpoint")?;
    unique(document.projects.iter().map(|entry| entry.id.as_str()), "project")?;
    unique(document.profiles.iter().map(|entry| entry.id.as_str()), "profile")?;
    unique(document.setlists.iter().map(|entry| entry.id.as_str()), "setlist")?;
    for project in &document.projects {
        unique(project.scenes.iter().map(|entry| entry.id.as_str()), "scene")?;
        if project.scenes.iter().any(|scene| scene.id.trim().is_empty()) {
            return Err(format!("project {} has empty scene ID", project.id));
        }
    }
    for setlist in &document.setlists {
        setlist.validate_against(&document.projects)?;
    }
    if let Some(active) = &document.settings.active_project {
        if !document.projects.iter().any(|project| project.id == *active) {
            return Err(format!("active project reference '{active}' is dangling"));
        }
    }
    if let Some(active_scene) = &document.settings.active_scene {
        let project_id = document
            .settings
            .active_project
            .as_deref()
            .ok_or("active scene requires an active project")?;
        let project = document
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| format!("active project reference '{project_id}' is dangling"))?;
        if !project.scenes.iter().any(|scene| scene.id == *active_scene) {
            return Err(format!("active scene reference '{active_scene}' is dangling"));
        }
    }
    let mut capabilities = std::collections::BTreeSet::new();
    for entry in &document.settings.default_providers {
        if entry.capability.is_empty()
            || entry.capability != entry.capability.trim().to_ascii_lowercase()
            || entry.profile_id.trim().is_empty()
            || entry.profile_id != entry.profile_id.trim()
        {
            return Err("default provider identifiers must be normalized and non-empty".into());
        }
        if !capabilities.insert(entry.capability.as_str()) {
            return Err(format!("duplicate default provider capability '{}'", entry.capability));
        }
    }
    if let Some(alias) = document.settings.learn_input_alias.as_deref() {
        if !document.endpoints.iter().any(|endpoint| endpoint.id == alias) {
            return Err(format!("Learn input references unknown endpoint '{alias}'"));
        }
    }
    let mut dashboard_triggers = std::collections::HashSet::new();
    for binding in &document.settings.dashboard_midi_bindings {
        binding.validate().map_err(str::to_owned)?;
        let key =
            serde_json::to_string(&binding.trigger).map_err(|_| "invalid dashboard trigger")?;
        if !dashboard_triggers.insert(key) {
            return Err("duplicate dashboard MIDI trigger".into());
        }
    }
    for mapping in &document.learned_mappings {
        validate_learned_mapping(mapping, &document.endpoints)?;
    }
    Ok(())
}

fn validate_learned_mapping(
    mapping: &LearnedMapping,
    endpoints: &[EndpointAlias],
) -> Result<(), String> {
    const KINDS: [&str; 10] = [
        "note_on",
        "note_off",
        "poly_pressure",
        "control_change",
        "program_change",
        "channel_pressure",
        "pitch_bend",
        "system_common",
        "realtime",
        "sysex",
    ];
    const MODES: [&str; 5] = ["cc", "program_change", "note", "pitch_bend", "sysex"];
    if !endpoints.iter().any(|endpoint| endpoint.id == mapping.source_alias) {
        return Err(format!(
            "learned mapping references unknown endpoint '{}'",
            mapping.source_alias
        ));
    }
    if !KINDS.contains(&mapping.message_kind.as_str()) {
        return Err(format!("unknown learned message kind '{}'", mapping.message_kind));
    }
    if !MODES.contains(&mapping.mode.as_str()) {
        return Err(format!("unknown learned mapping mode '{}'", mapping.mode));
    }
    if mapping.destination.trim().is_empty() || mapping.raw.is_empty() || mapping.raw.len() > 65_536
    {
        return Err("learned mapping requires a destination and bounded raw evidence".into());
    }
    match mapping.channel_policy {
        LearnedChannelPolicy::Exact(channel) if !(1..=16).contains(&channel) => {
            return Err("learned mapping channel must be in 1..=16".into());
        }
        LearnedChannelPolicy::NotApplicable
            if !matches!(mapping.message_kind.as_str(), "system_common" | "realtime" | "sysex") =>
        {
            return Err("channel-bearing learned mapping requires exact or any policy".into());
        }
        LearnedChannelPolicy::Exact(_) | LearnedChannelPolicy::Any
            if matches!(mapping.message_kind.as_str(), "system_common" | "realtime" | "sysex") =>
        {
            return Err("channel-less learned mapping requires not_applicable policy".into());
        }
        _ => {}
    }
    if mapping.filters.len() > 32 {
        return Err("learned mapping filter count exceeds 32".into());
    }
    for filter in &mapping.filters {
        filter.validate()?;
    }
    Ok(())
}

fn unique<'a>(ids: impl Iterator<Item = &'a str>, kind: &str) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for id in ids {
        if id.trim().is_empty() {
            return Err(format!("{kind} ID must not be empty"));
        }
        if id != id.trim() {
            return Err(format!("{kind} ID must not have surrounding whitespace"));
        }
        if !seen.insert(id) {
            return Err(format!("duplicate {kind} ID '{id}'"));
        }
    }
    Ok(())
}

/// Saves valid configuration atomically and retains up to `backup_count` prior versions.
///
/// # Errors
///
/// Returns an error when semantic validation, backup rotation, serialization, or replacement fails.
pub fn save(
    path: &Path,
    document: &ConfigDocument,
    backup_count: usize,
) -> Result<(), ConfigError> {
    validate(document)
        .map_err(|message| ConfigError::Semantic { path: path.to_owned(), message })?;
    if path.exists() {
        rotate_backups(path, backup_count)
            .map_err(|source| ConfigError::Io { path: path.to_owned(), source })?;
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let temp = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        stamp
    ));
    let serialized = serde_json::to_string_pretty(document).map_err(|error| {
        ConfigError::Parse { path: path.to_owned(), message: error.to_string() }
    })?;
    fs::write(&temp, format!("{serialized}\n"))
        .map_err(|source| ConfigError::Replace { path: temp.clone(), source })?;
    fs::rename(&temp, path).map_err(|source| ConfigError::Replace { path: path.to_owned(), source })
}

/// Exports a validated configuration into a portable directory without machine-specific paths.
///
/// # Errors
///
/// Returns an error when validation, directory creation, serialization, or atomic replacement
/// fails.
pub fn export_portable(
    document: &ConfigDocument,
    directory: &Path,
) -> Result<PathBuf, ConfigError> {
    validate(document)
        .map_err(|message| ConfigError::Semantic { path: directory.to_owned(), message })?;
    fs::create_dir_all(directory)
        .map_err(|source| ConfigError::Io { path: directory.to_owned(), source })?;
    let target = directory.join("config.json5");
    let temporary = directory.join("config.json5.tmp");
    let encoded = serde_json::to_vec_pretty(document)
        .map_err(|error| ConfigError::Parse { path: target.clone(), message: error.to_string() })?;
    fs::write(&temporary, encoded)
        .map_err(|source| ConfigError::Io { path: temporary.clone(), source })?;
    fs::rename(&temporary, &target)
        .map_err(|source| ConfigError::Io { path: target.clone(), source })?;
    Ok(target)
}

/// Imports and validates a portable configuration directory artifact.
///
/// # Errors
///
/// Returns a configuration error when the directory is missing, the canonical artifact is
/// absent, or loading/migration/semantic validation fails.
pub fn import_portable(directory: &Path) -> Result<ConfigDocument, ConfigError> {
    if !directory.is_dir() {
        return Err(ConfigError::Io {
            path: directory.to_owned(),
            source: io::Error::new(io::ErrorKind::NotFound, "portable directory is missing"),
        });
    }
    load(&directory.join("config.json5"))
}

fn rotate_backups(path: &Path, backup_count: usize) -> io::Result<()> {
    if backup_count == 0 {
        return Ok(());
    }
    for index in (1..=backup_count).rev() {
        let source = path.with_extension(format!("json5.bak{index}"));
        let destination = path.with_extension(format!("json5.bak{}", index + 1));
        if source.exists() {
            fs::rename(source, destination)?;
        }
    }
    fs::copy(path, path.with_extension("json5.bak1"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> ConfigDocument {
        ConfigDocument {
            schema_version: 1,
            settings: Settings {
                active_project: Some("demo".to_owned()),
                active_scene: None,
                default_providers: Vec::new(),
                learn_input_alias: None,
                dashboard_midi_bindings: Vec::new(),
            },
            endpoints: vec![],
            projects: vec![Project {
                id: "demo".to_owned(),
                scenes: vec![SceneRef { id: "intro".to_owned(), name: None, category: None }],
            }],
            profiles: vec![],
            setlists: vec![],
            learned_mappings: vec![],
        }
    }

    #[test]
    fn parses_json5_and_rejects_unknown_fields() {
        let text = "{schema_version:1, settings:{active_project:'demo'}, projects:[{id:'demo',scenes:[]}]}";
        assert_eq!(json5::from_str::<ConfigDocument>(text).expect("valid").schema_version, 1);
        assert!(json5::from_str::<ConfigDocument>("{schema_version:1, unknown:true}").is_err());
    }

    #[test]
    fn dashboard_bindings_round_trip_and_reject_duplicates() {
        let mut value = document();
        let binding = DashboardMidiBinding {
            trigger: DashboardMidiTrigger::NoteOn { channel: 1, note: 36 },
            command: "panic".into(),
        };
        value.settings.dashboard_midi_bindings = vec![binding.clone()];
        let encoded = serde_json::to_string(&value).expect("encode");
        let decoded: ConfigDocument = serde_json::from_str(&encoded).expect("decode");
        assert_eq!(decoded.settings.dashboard_midi_bindings, vec![binding.clone()]);
        assert!(validate(&decoded).is_ok());
        value.settings.dashboard_midi_bindings.push(binding);
        assert_eq!(validate(&value), Err("duplicate dashboard MIDI trigger".into()));
    }

    #[test]
    fn rejects_duplicate_and_dangling_references() {
        let mut value = document();
        value.projects.push(Project { id: "demo".to_owned(), scenes: vec![] });
        assert!(validate(&value).is_err());
        value.projects.pop();
        value.settings.active_project = Some("missing".to_owned());
        assert!(validate(&value).is_err());
    }

    #[test]
    fn rejects_ids_with_surrounding_whitespace() {
        let mut value = document();
        value.projects[0].id = " demo".into();
        assert_eq!(validate(&value), Err("project ID must not have surrounding whitespace".into()));
        value = document();
        value.projects[0].scenes[0].id = "intro ".into();
        assert_eq!(validate(&value), Err("scene ID must not have surrounding whitespace".into()));
    }

    #[test]
    fn document_validates_persisted_setlists_against_projects() {
        let mut value = document();
        value.setlists = vec![Setlist { id: "live".into(), projects: vec!["demo".into()] }];
        assert!(validate(&value).is_ok());
        value.setlists[0].projects = vec!["missing".into()];
        assert!(validate(&value).is_err());
    }

    #[test]
    fn active_project_update_is_validated_and_non_mutating() {
        let document = document();
        let cleared = set_active_project(&document, None).expect("clear active project");
        assert_eq!(cleared.settings.active_project, None);
        assert_eq!(document.settings.active_project.as_deref(), Some("demo"));
        assert!(set_active_project(&document, Some("missing")).is_err());
        assert_eq!(document.settings.active_project.as_deref(), Some("demo"));
    }

    #[test]
    fn active_scene_update_is_validated_and_non_mutating() {
        let value = document();
        let selected = set_active_scene(&value, Some("intro")).expect("scene");
        assert_eq!(selected.settings.active_scene.as_deref(), Some("intro"));
        assert!(set_active_scene(&value, Some("missing")).is_err());
        assert_eq!(value.settings.active_scene, None);
        let mut no_project = document();
        no_project.settings.active_project = None;
        assert!(set_active_scene(&no_project, Some("intro")).is_err());
    }

    #[test]
    fn default_provider_assignment_is_normalized_replaced_and_persistent() {
        let value = document();
        let selected = set_default_provider(&value, " ReVerb ", "lexicon.reflex").expect("set");
        assert_eq!(default_provider(&selected, "REVERB"), Some("lexicon.reflex"));
        let replaced =
            set_default_provider(&selected, "reverb", "valeton.arena2000").expect("replace");
        assert_eq!(replaced.settings.default_providers.len(), 1);
        assert_eq!(default_provider(&replaced, "reverb"), Some("valeton.arena2000"));
        assert!(set_default_provider(&value, " ", "profile").is_err());
        assert!(set_default_provider(&value, "delay", " ").is_err());
        assert!(value.settings.default_providers.is_empty());
    }

    #[test]
    fn learned_mapping_and_global_input_are_validated_and_transactional() {
        let mut value = document();
        value.endpoints.push(EndpointAlias {
            id: "launch-control".into(),
            name: Some("Launch Control XL".into()),
            vendor_id: None,
            product_id: None,
            serial: None,
        });
        let selected = set_learn_input_alias(&value, " launch-control ").expect("input");
        assert_eq!(selected.settings.learn_input_alias.as_deref(), Some("launch-control"));
        assert!(set_learn_input_alias(&value, "missing").is_err());
        let mapping = LearnedMapping {
            source_alias: "launch-control".into(),
            message_kind: "control_change".into(),
            channel_policy: LearnedChannelPolicy::Any,
            number: Some(7),
            raw: vec![0xB0, 7, 127],
            destination: "arena.delay.mix".into(),
            mode: "cc".into(),
            enabled: true,
            priority: 0,
            filters: Vec::new(),
        };
        let mapped = add_learned_mapping(&selected, mapping.clone()).expect("mapping");
        assert_eq!(mapped.learned_mappings, vec![mapping.clone()]);
        assert!(add_learned_mapping(&mapped, mapping).is_err());
        assert!(selected.learned_mappings.is_empty());

        let mut invalid = mapped;
        invalid.learned_mappings[0].channel_policy = LearnedChannelPolicy::Exact(17);
        assert!(validate(&invalid).is_err());
    }

    #[test]
    fn learned_filters_round_trip_and_validate_bounds() {
        let filters = vec![
            LearnedFilter::NumberRange { minimum: 1, maximum: 7 },
            LearnedFilter::ValueRange { minimum: 100, maximum: 200 },
            LearnedFilter::Realtime { message: LearnedRealtime::Clock },
            LearnedFilter::SysExMask { pattern: vec![1, 2], mask: vec![127, 127] },
        ];
        let encoded = serde_json::to_string(&filters).expect("filters serialize");
        let decoded: Vec<LearnedFilter> = serde_json::from_str(&encoded).expect("filters parse");
        assert_eq!(decoded, filters);

        let mut document = document();
        document.endpoints.push(EndpointAlias {
            id: "input".into(),
            name: None,
            vendor_id: None,
            product_id: None,
            serial: None,
        });
        let mapping = LearnedMapping {
            source_alias: "input".into(),
            message_kind: "control_change".into(),
            channel_policy: LearnedChannelPolicy::Any,
            number: Some(1),
            raw: vec![0xB0, 1, 2],
            destination: "delay.mix".into(),
            mode: "cc".into(),
            enabled: true,
            priority: 0,
            filters,
        };
        assert!(add_learned_mapping(&document, mapping.clone()).is_ok());
        let mut invalid = mapping;
        invalid.filters = vec![LearnedFilter::NumberRange { minimum: 8, maximum: 7 }];
        assert!(add_learned_mapping(&document, invalid).is_err());
    }

    #[test]
    fn project_deletion_requires_explicit_reference_resolution() {
        let mut value = document();
        value.settings.active_project = None;
        value.setlists = vec![Setlist { id: "live".into(), projects: vec!["demo".into()] }];
        let report = project_reference_report(&value, "demo");
        assert_eq!(report.setlists, vec!["live"]);
        assert!(!report.active);
        assert!(remove_project(&value, "demo").is_err());
        value.setlists.clear();
        let removed = remove_project(&value, "demo").expect("unreferenced removal");
        assert!(removed.projects.is_empty());
        assert_eq!(value.projects.len(), 1);
    }

    #[test]
    fn scene_reorder_and_copy_are_explicit_and_non_mutating() {
        let mut project = document().projects[0].clone();
        project.scenes[0].name = Some("Intro ambience".into());
        project.scenes[0].category = Some("opening".into());
        let reordered = reorder_scenes(&project, &["intro"]).expect("reorder");
        assert_eq!(reordered.scenes, project.scenes);
        let copied = copy_scene(&project, "intro", "intro-copy").expect("copy");
        assert_eq!(project.scenes.len(), 1);
        assert_eq!(copied.scenes.last().map(|scene| scene.id.as_str()), Some("intro-copy"));
        assert_eq!(search_scenes(&project, "ambience").len(), 1);
        assert_eq!(search_scenes(&project, "OPENING").len(), 1);
        assert_eq!(
            copied.scenes.last().and_then(|scene| scene.category.as_deref()),
            Some("opening")
        );
        let copied_project = copy_project(std::slice::from_ref(&project), "demo", "demo-copy")
            .expect("project copy");
        assert_eq!(copied_project.id, "demo-copy");
        assert_eq!(copied_project.scenes[0].id, "demo-copy.scene-1-intro");
        assert!(copy_project(std::slice::from_ref(&project), "demo", "demo").is_err());
        assert!(reorder_scenes(&project, &["missing"]).is_err());
        assert!(copy_scene(&project, "intro", "intro").is_err());
        assert_eq!(search_scenes(&project, "INT").len(), 1);
        assert!(search_scenes(&project, "missing").is_empty());
        let setlist = Setlist { id: "live-set".into(), projects: vec![project.id.clone()] };
        setlist.validate_against(std::slice::from_ref(&project)).expect("setlist");
        let reordered_setlist = reorder_setlist(&setlist, &["demo"]).expect("reorder setlist");
        assert_eq!(reordered_setlist.projects, vec!["demo"]);
        assert!(reorder_setlist(&setlist, &["missing"]).is_err());
        let setlists =
            vec![setlist, Setlist { id: "rehearsal".into(), projects: vec!["other".into()] }];
        assert_eq!(search_setlists(&setlists, "LIVE"), vec![&setlists[0]]);
        assert_eq!(search_setlists(&setlists, "OTH"), vec![&setlists[1]]);
        let copied = copy_setlist(&setlists, "live-set", "encore").expect("copy setlist");
        assert_eq!(copied, Setlist { id: "encore".into(), projects: vec!["demo".into()] });
        assert!(copy_setlist(&setlists, "live-set", "live-set").is_err());
        assert!(copy_setlist(&setlists, "missing", "new").is_err());
        assert!(Setlist { id: "live-set".into(), projects: vec!["missing".into()] }
            .validate_against(std::slice::from_ref(&project))
            .is_err());
        assert!(Setlist {
            id: "live-set".into(),
            projects: vec![project.id.clone(), project.id.clone()]
        }
        .validate_against(std::slice::from_ref(&project))
        .is_err());
        let projects = vec![
            project.clone(),
            Project {
                id: "live".into(),
                scenes: vec![SceneRef { id: "outro".into(), name: None, category: None }],
            },
        ];
        assert_eq!(search_projects(&projects, "OUT"), vec![&projects[1]]);
        assert_eq!(search_projects(&projects, "DEMO"), vec![&projects[0]]);
        assert!(search_projects(&projects, "missing").is_empty());
        let mut invalid = project.clone();
        invalid.scenes.push(SceneRef { id: "intro".into(), name: None, category: None });
        assert!(replace_project(
            &ConfigDocument { projects: vec![project], ..document() },
            invalid
        )
        .is_err());
    }

    #[test]
    fn saves_loads_and_rotates_backup() {
        let path = std::env::temp_dir().join(format!("mackes-config-{}.json5", std::process::id()));
        save(&path, &document(), 2).expect("save");
        assert_eq!(load(&path).expect("load").settings.active_project.as_deref(), Some("demo"));
        save(&path, &document(), 2).expect("second save");
        assert!(path.with_extension("json5.bak1").exists());
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json5.bak1"));
    }

    #[test]
    fn current_version_migration_is_a_no_op() {
        assert_eq!(migrate(document()).expect("current migration"), document());
    }

    #[test]
    fn validation_report_is_deterministic_in_human_and_json_modes() {
        let path = std::env::temp_dir().join(format!("mackes-report-{}.json5", std::process::id()));
        save(&path, &document(), 0).expect("save");
        assert_eq!(validate_report(&path, false), format!("valid: {} is valid\n", path.display()));
        assert!(validate_report(&path, true).contains("\"valid\":true"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn portable_export_validates_and_replaces_atomically() {
        let directory =
            std::env::temp_dir().join(format!("mackes-portable-{}", std::process::id()));
        let target = export_portable(&document(), &directory).expect("export");
        assert_eq!(load(&target).expect("load export"), document());
        assert_eq!(import_portable(&directory).expect("import export"), document());
        assert!(import_portable(&target).is_err());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn backup_manifest_validates_digest_and_identity() {
        let payload = b"backup";
        let manifest = BackupManifest {
            profile: "lexicon.reflex.rev1".into(),
            device_identity: "serial:A".into(),
            source_alias: "reflex".into(),
            captured_at: 1,
            sha256: BackupManifest::digest(payload),
            status: BackupStatus::Verified,
        };
        assert!(manifest.validate().is_ok());
        assert!(manifest.matches_payload(payload));
        assert!(!manifest.matches_payload(b"tampered"));
        assert!(backup_compatible(&manifest, "lexicon.reflex.rev1", "serial:A"));
        assert!(backup_compatible(&manifest, "lexicon.reflex.rev1", "serial:B"));
        assert!(backup_identity_warning(&manifest, "serial:B"));
        assert!(!backup_identity_warning(&manifest, "serial:A"));
        assert!(!backup_compatible(&manifest, "other", "serial:A"));
        let mut invalid = manifest;
        invalid.sha256 = "z".repeat(64);
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn backup_storage_writes_payload_and_manifest_sidecar() {
        let path = std::env::temp_dir().join(format!("mackes-backup-{}.bin", std::process::id()));
        let payload = b"backup-payload";
        let manifest = BackupManifest {
            profile: "test".into(),
            device_identity: "device".into(),
            source_alias: "alias".into(),
            captured_at: 1,
            sha256: BackupManifest::digest(payload),
            status: BackupStatus::Verified,
        };
        save_backup(&path, payload, &manifest).expect("save backup");
        assert_eq!(fs::read(&path).expect("payload"), payload);
        assert!(path.with_extension("manifest.json").exists());
        assert_eq!(load_backup(&path).expect("load backup").0, payload);
        assert!(save_backup(&path, payload, &manifest).is_err());
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("manifest.json"));
    }

    #[test]
    fn restore_is_compatibility_gated_and_supports_dry_run() {
        let stem = format!("mackes-restore-{}", std::process::id());
        let backup = std::env::temp_dir().join(format!("{stem}.bin"));
        let target = std::env::temp_dir().join(format!("{stem}.target"));
        let payload = b"restore-payload";
        let manifest = BackupManifest {
            profile: "reflex".into(),
            device_identity: "serial:A".into(),
            source_alias: "alias".into(),
            captured_at: 1,
            sha256: BackupManifest::digest(payload),
            status: BackupStatus::Verified,
        };
        save_backup(&backup, payload, &manifest).expect("save");
        assert_eq!(
            restore_backup(&backup, &target, "reflex", "serial:A", RestoreMode::DryRun),
            Ok(RestoreResult::Planned {
                bytes: payload.len(),
                identity_warning: false,
                status: BackupStatus::Verified,
            })
        );
        assert!(!target.exists());
        assert_eq!(
            restore_backup(&backup, &target, "reflex", "serial:B", RestoreMode::DryRun),
            Ok(RestoreResult::Planned {
                bytes: payload.len(),
                identity_warning: true,
                status: BackupStatus::Verified,
            })
        );
        assert!(restore_backup(&backup, &target, "other", "serial:A", RestoreMode::Apply).is_err());
        assert_eq!(
            restore_backup(&backup, &target, "reflex", "serial:A", RestoreMode::Apply),
            Ok(RestoreResult::Applied {
                bytes: payload.len(),
                identity_warning: false,
                status: BackupStatus::Verified,
            })
        );
        assert_eq!(fs::read(&target).expect("target"), payload);
        let _ = fs::remove_file(&backup);
        let _ = fs::remove_file(backup.with_extension("manifest.json"));
        let _ = fs::remove_file(&target);
    }

    #[test]
    fn restore_preserves_sent_unverified_status() {
        let stem = format!("mackes-unverified-{}", std::process::id());
        let backup = std::env::temp_dir().join(format!("{stem}.bin"));
        let target = std::env::temp_dir().join(format!("{stem}.target"));
        let payload = b"unverified-payload";
        let manifest = BackupManifest {
            profile: "reflex".into(),
            device_identity: "serial:A".into(),
            source_alias: "alias".into(),
            captured_at: 1,
            sha256: BackupManifest::digest(payload),
            status: BackupStatus::SentUnverified,
        };
        save_backup(&backup, payload, &manifest).expect("save");
        assert_eq!(
            restore_backup(&backup, &target, "reflex", "serial:A", RestoreMode::DryRun),
            Ok(RestoreResult::Planned {
                bytes: payload.len(),
                identity_warning: false,
                status: BackupStatus::SentUnverified,
            })
        );
        let _ = fs::remove_file(&backup);
        let _ = fs::remove_file(backup.with_extension("manifest.json"));
        let _ = fs::remove_file(&target);
    }
}
