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
    /// Atomic hardware-first parameter mappings.
    #[serde(default)]
    pub control_mappings: Vec<ControlMapping>,
    /// Resumable but inactive mapping drafts.
    #[serde(default)]
    pub control_mapping_drafts: Vec<ControlMappingDraft>,
}

/// Validated mapping behavior applied between source and destination ranges.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MappingBehavior {
    /// Inclusive source value range.
    #[serde(default = "default_mapping_range")]
    pub source_range: (u16, u16),
    /// Inclusive destination value range.
    #[serde(default = "default_mapping_range")]
    pub destination_range: (u16, u16),
    /// Whether the destination range is inverted.
    #[serde(default)]
    pub invert: bool,
    /// Approved curve name.
    #[serde(default = "default_curve")]
    pub curve: String,
}

fn default_curve() -> String {
    "linear".into()
}

const fn default_mapping_range() -> (u16, u16) {
    (0, 127)
}

/// Durable, complete hardware-first parameter mapping.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ControlMapping {
    /// Stable mapping identity.
    pub id: String,
    /// Controller profile identity.
    pub controller_profile: String,
    /// Stable physical source identity.
    pub physical_control_id: String,
    /// Exact source endpoint identity.
    pub source_endpoint: String,
    /// Exact source MIDI message kind.
    pub source_kind: String,
    /// Zero-based source channel.
    pub source_channel: u8,
    /// Source MIDI controller/note number.
    pub source_number: u8,
    /// Exact destination output endpoint identity.
    pub destination_endpoint: String,
    /// Destination profile identity.
    pub destination_profile: String,
    /// Destination effect/block identity.
    pub destination_effect: String,
    /// Destination parameter identity.
    pub destination_parameter: String,
    /// Mapping behavior.
    pub behavior: MappingBehavior,
    /// Whether this mapping is active.
    pub enabled: bool,
    /// Profile provenance version.
    pub profile_version: u32,
}

/// Incomplete mapping wizard state; never executable.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ControlMappingDraft {
    /// Stable draft identity.
    pub id: String,
    /// Last completed wizard step.
    pub step: String,
    /// Optional source identity.
    #[serde(default)]
    pub physical_control_id: Option<String>,
    /// Optional destination identity.
    #[serde(default)]
    pub destination: Option<String>,
}

/// In-memory authoritative mapping state used for transactional persistence.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ControlMappingStore {
    /// Current mapping generation.
    pub generation: u64,
    /// Active mappings.
    pub active: Vec<ControlMapping>,
    /// Inactive resumable drafts.
    pub drafts: Vec<ControlMappingDraft>,
    undo: Option<(Vec<ControlMapping>, Vec<ControlMappingDraft>)>,
}

#[allow(clippy::missing_errors_doc)]
impl ControlMappingStore {
    /// Returns whether a successful mutation can currently be undone.
    #[must_use]
    pub const fn undo_available(&self) -> bool {
        self.undo.is_some()
    }

    /// Returns immutable active mappings and drafts for snapshot projection.
    #[must_use]
    pub fn snapshot(&self) -> (&[ControlMapping], &[ControlMappingDraft]) {
        (&self.active, &self.drafts)
    }

    /// Loads mapping state from a validated configuration document.
    ///
    /// # Errors
    ///
    /// Returns an error when the document fails semantic validation.
    pub fn from_document(document: &ConfigDocument) -> Result<Self, String> {
        validate(document)?;
        Ok(Self {
            generation: 0,
            active: document.control_mappings.clone(),
            drafts: document.control_mapping_drafts.clone(),
            undo: None,
        })
    }

    /// Projects authoritative mapping state into a cloned document.
    #[must_use]
    pub fn apply_to_document(&self, document: &ConfigDocument) -> ConfigDocument {
        let mut result = document.clone();
        result.control_mappings.clone_from(&self.active);
        result.control_mapping_drafts.clone_from(&self.drafts);
        result
    }

    fn begin_mutation(&mut self, expected_generation: u64) -> Result<(), &'static str> {
        if self.generation != expected_generation {
            return Err("mapping generation conflict");
        }
        self.undo = Some((self.active.clone(), self.drafts.clone()));
        Ok(())
    }

    /// Saves or replaces an inactive draft without activating it.
    pub fn save_draft(
        &mut self,
        expected_generation: u64,
        draft: ControlMappingDraft,
    ) -> Result<(), &'static str> {
        if draft.id.trim().is_empty() || draft.id.len() > 64 || draft.step.trim().is_empty() {
            return Err("mapping draft is invalid");
        }
        if draft.physical_control_id.as_deref().is_some_and(|id| !valid_physical_control_id(id)) {
            return Err("mapping draft control identity is invalid");
        }
        self.begin_mutation(expected_generation)?;
        if let Some(existing) = self.drafts.iter_mut().find(|existing| existing.id == draft.id) {
            *existing = draft;
        } else {
            self.drafts.push(draft);
        }
        self.generation = self.generation.saturating_add(1);
        Ok(())
    }

    /// Atomically activates a mapping at the expected generation.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale generation, invalid mapping, or source/destination conflict.
    pub fn activate(
        &mut self,
        expected_generation: u64,
        mapping: ControlMapping,
    ) -> Result<(), &'static str> {
        if self.generation != expected_generation {
            return Err("mapping generation conflict");
        }
        validate_mapping(&mapping)?;
        if self.active.iter().any(|existing| {
            existing.physical_control_id == mapping.physical_control_id
                || (existing.destination_profile == mapping.destination_profile
                    && existing.destination_effect == mapping.destination_effect
                    && existing.destination_parameter == mapping.destination_parameter)
        }) {
            return Err("mapping source or destination is already occupied");
        }
        self.undo = Some((self.active.clone(), self.drafts.clone()));
        self.active.push(mapping);
        self.generation = self.generation.saturating_add(1);
        Ok(())
    }

    /// Undoes the latest successful mutation at the expected generation.
    pub fn undo(&mut self, expected_generation: u64) -> Result<(), &'static str> {
        if self.generation != expected_generation {
            return Err("mapping generation conflict");
        }
        let Some((active, drafts)) = self.undo.take() else {
            return Err("no mapping change is available to undo");
        };
        self.active = active;
        self.drafts = drafts;
        self.generation = self.generation.saturating_add(1);
        Ok(())
    }

    /// Explicitly replaces a mapping, requiring the target identity to exist.
    pub fn replace(
        &mut self,
        expected_generation: u64,
        mapping: ControlMapping,
    ) -> Result<(), &'static str> {
        validate_mapping(&mapping)?;
        if !self.active.iter().any(|existing| existing.id == mapping.id) {
            return Err("mapping to replace was not found");
        }
        if self.active.iter().any(|existing| {
            existing.id != mapping.id
                && (existing.physical_control_id == mapping.physical_control_id
                    || (existing.destination_profile == mapping.destination_profile
                        && existing.destination_effect == mapping.destination_effect
                        && existing.destination_parameter == mapping.destination_parameter))
        }) {
            return Err("replacement source or destination is already occupied");
        }
        self.begin_mutation(expected_generation)?;
        let Some(slot) = self.active.iter_mut().find(|existing| existing.id == mapping.id) else {
            return Err("mapping to replace was not found");
        };
        *slot = mapping;
        self.generation = self.generation.saturating_add(1);
        Ok(())
    }

    /// Replaces a mapping and rolls the store back when runtime application fails.
    ///
    /// # Errors
    ///
    /// Returns the validation/conflict error, or the runtime error after restoring the prior
    /// active and draft collections and generation.
    pub fn replace_with_runtime<F, E>(
        &mut self,
        expected_generation: u64,
        mapping: ControlMapping,
        mut apply_runtime: F,
    ) -> Result<(), String>
    where
        F: FnMut(&ControlMapping) -> Result<(), E>,
        E: std::fmt::Display,
    {
        let before = self.clone();
        let runtime_mapping = mapping.clone();
        self.replace(expected_generation, mapping).map_err(str::to_owned)?;
        if let Err(error) = apply_runtime(&runtime_mapping) {
            *self = before;
            return Err(format!("runtime replacement failed: {error}"));
        }
        Ok(())
    }

    /// Updates behavior without changing mapping identity.
    pub fn update_behavior(
        &mut self,
        expected_generation: u64,
        id: &str,
        behavior: MappingBehavior,
    ) -> Result<(), &'static str> {
        behavior.validate()?;
        if !self.active.iter().any(|mapping| mapping.id == id) {
            return Err("mapping to update was not found");
        }
        self.begin_mutation(expected_generation)?;
        let Some(mapping) = self.active.iter_mut().find(|mapping| mapping.id == id) else {
            return Err("mapping to update was not found");
        };
        mapping.behavior = behavior;
        self.generation = self.generation.saturating_add(1);
        Ok(())
    }

    /// Enables or disables an existing mapping.
    pub fn set_enabled(
        &mut self,
        expected_generation: u64,
        id: &str,
        enabled: bool,
    ) -> Result<(), &'static str> {
        if !self.active.iter().any(|mapping| mapping.id == id) {
            return Err("mapping to update was not found");
        }
        self.begin_mutation(expected_generation)?;
        let Some(mapping) = self.active.iter_mut().find(|mapping| mapping.id == id) else {
            return Err("mapping to update was not found");
        };
        mapping.enabled = enabled;
        self.generation = self.generation.saturating_add(1);
        Ok(())
    }

    /// Deletes an existing mapping.
    pub fn delete(&mut self, expected_generation: u64, id: &str) -> Result<(), &'static str> {
        if !self.active.iter().any(|mapping| mapping.id == id) {
            return Err("mapping to delete was not found");
        }
        self.begin_mutation(expected_generation)?;
        let before = self.active.len();
        self.active.retain(|mapping| mapping.id != id);
        if self.active.len() == before {
            return Err("mapping to delete was not found");
        }
        self.generation = self.generation.saturating_add(1);
        Ok(())
    }
}

fn validate_mapping(mapping: &ControlMapping) -> Result<(), &'static str> {
    if mapping.id.trim().is_empty()
        || mapping.id.len() > 64
        || mapping.controller_profile.trim().is_empty()
        || !valid_physical_control_id(&mapping.physical_control_id)
        || mapping.source_endpoint.trim().is_empty()
        || mapping.source_kind.trim().is_empty()
        || mapping.source_channel > 15
        || mapping.source_number > 127
        || mapping.destination_endpoint.trim().is_empty()
        || mapping.destination_profile.trim().is_empty()
        || mapping.destination_effect.trim().is_empty()
        || mapping.destination_parameter.trim().is_empty()
        || mapping.profile_version == 0
    {
        return Err("control mapping is incomplete or invalid");
    }
    mapping.behavior.validate()
}

impl MappingBehavior {
    /// Validates approved curve and bounded behavior fields.
    ///
    /// # Errors
    ///
    /// Returns an error when the curve is not in the approved set.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !matches!(self.curve.as_str(), "linear" | "square" | "square_root")
            || self.source_range.0 >= self.source_range.1
            || self.destination_range.0 > self.destination_range.1
            || self.source_range.1 > 16_383
            || self.destination_range.1 > 16_383
        {
            return Err("mapping curve is unsupported");
        }
        Ok(())
    }
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
    /// Optional imported Launch Control XL template assignments.
    #[serde(default)]
    pub launch_control_template: Option<LaunchControlTemplateConfig>,
}

/// Serializable Launch Control XL assignment map used by the faceplate.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchControlTemplateConfig {
    /// Template slot (0–15).
    pub template: u8,
    /// Bounded physical-to-MIDI assignments.
    #[serde(default)]
    pub assignments: Vec<LaunchControlAssignmentConfig>,
}

/// One serialized Launch Control assignment.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchControlAssignmentConfig {
    /// Physical control index (0–47).
    pub index: u8,
    /// Stable profile-owned identity; absent only for legacy released data.
    #[serde(default)]
    pub physical_control_id: Option<String>,
    /// Zero-based MIDI channel.
    pub channel: u8,
    /// MIDI number.
    pub number: u8,
    /// `cc` or `note`.
    pub kind: String,
    /// Optional bounded destination summary shown by the faceplate.
    #[serde(default)]
    pub destination: Option<String>,
    /// Legacy ambiguity marker; such records are inert until recaptured.
    #[serde(default)]
    pub needs_review: bool,
}

impl LaunchControlAssignmentConfig {
    /// Migrates a released numeric assignment without guessing ambiguous faders.
    #[must_use]
    pub fn migrated_physical_control_id(&self) -> Option<String> {
        self.physical_control_id.clone().or_else(|| match self.index {
            0..=7 => Some(format!("knob-r1-c{}", self.index + 1)),
            8..=15 => Some(format!("knob-r2-c{}", self.index - 7)),
            16..=23 => Some(format!("knob-r3-c{}", self.index - 15)),
            24..=31 => Some(format!("button-r1-c{}", self.index - 23)),
            32..=39 => Some(format!("button-r2-c{}", self.index - 31)),
            _ => None,
        })
    }

    /// Returns whether this legacy entry must be reviewed before activation.
    #[must_use]
    pub const fn requires_review(&self) -> bool {
        self.needs_review || self.physical_control_id.is_none() && self.index >= 40
    }

    /// Migrates a legacy User 1 tuple to the authoritative Factory 1 tuple.
    ///
    /// The stable physical identity and assignment destination are preserved. An
    /// entry is rejected when its identity is absent/ambiguous or its tuple does
    /// not belong to the known legacy inventory, so migration never guesses.
    #[must_use]
    pub fn migrated_factory1(&self) -> Option<Self> {
        let id = self.migrated_physical_control_id()?;
        if self.requires_review() {
            return None;
        }
        let (kind, number) = if let Some(rest) = id.strip_prefix("knob-r") {
            let (row, column) = rest.split_once("-c")?;
            let row = row.parse::<u8>().ok()?;
            let column = column.parse::<u8>().ok()?;
            ("cc".to_owned(), (row.checked_sub(1)? * 16 + column.checked_sub(1)? + 13))
        } else if let Some(rest) = id.strip_prefix("button-r") {
            let (row, column) = rest.split_once("-c")?;
            let row = row.parse::<u8>().ok()?;
            let column = column.parse::<u8>().ok()?;
            ("note".to_owned(), ((row.checked_sub(1)? * 16 + column.checked_sub(1)?) + 41))
        } else {
            let column = id.strip_prefix("fader-").and_then(|v| v.parse::<u8>().ok())?;
            ("cc".to_owned(), column.checked_sub(1)? + 77)
        };
        let mut migrated = self.clone();
        migrated.physical_control_id = Some(id);
        migrated.channel = 8;
        migrated.kind = kind;
        migrated.number = number;
        Some(migrated)
    }
}

/*
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetiredEditorMap {
    /// Stable profile identity required by the importer.
    pub profile_id: String,
    /// Firmware identity the map was exported from.
    pub firmware: String,
    /// SHA-256 digest of the raw editor artifact.
    pub artifact_sha256: String,
    /// Bounded documented control assignments.
    #[serde(default)]
    pub assignments: Vec<LaunchControlAssignmentConfig>,
}

impl RetiredEditorMap {
    /// Validates identity, digest, bounds, and duplicate assignments.
    ///
    /// # Errors
    ///
    /// Returns an error when identity, firmware, digest, or assignment bounds are invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.profile_id != "retired.device"
            || self.firmware.trim().is_empty()
            || self.artifact_sha256.len() != 64
            || !self.artifact_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            || self.assignments.len() > 48
        {
            return Err("retired editor map identity or artifact metadata is invalid".into());
        }
        let mut indices = std::collections::BTreeSet::new();
        for assignment in &self.assignments {
            if assignment.index >= 48
                || assignment.channel >= 16
                || assignment.number > 127
                || !matches!(assignment.kind.as_str(), "cc" | "note")
                || !indices.insert(assignment.index)
            {
                return Err("retired editor map assignment is invalid or duplicated".into());
            }
        }
        Ok(())
    }

    /// Verifies the map's artifact digest before import.
    ///
    /// # Errors
    ///
    /// Returns an error when the artifact digest differs from the declared digest.
    pub fn verify_artifact(&self, artifact: &[u8]) -> Result<(), String> {
        if BackupManifest::digest(artifact) == self.artifact_sha256 {
            Ok(())
        } else {
            Err("retired editor artifact hash does not match map".into())
        }
    }

    /// Validates this map against an expected firmware, requiring explicit approval for drift.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid map or an unapproved firmware mismatch.
    pub fn validate_for_firmware(
        &self,
        expected_firmware: &str,
        approve_mismatch: bool,
    ) -> Result<(), String> {
        self.validate()?;
        if self.firmware != expected_firmware && !approve_mismatch {
            return Err(format!(
                "retired device firmware mismatch: map={} expected={} (approval required)",
                self.firmware, expected_firmware
            ));
        }
        Ok(())
    }
}

*/
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
    /// Ordered actions executed when the scene is activated.
    #[serde(default)]
    pub actions: Vec<SceneAction>,
}

/// Persisted action metadata consumed by the activation planner.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneAction {
    /// Stable action identifier within the scene.
    pub id: String,
    /// Operator-facing action description.
    pub description: String,
    /// Whether this action requires unsafe mode.
    #[serde(default)]
    pub unsafe_action: bool,
    /// Optional prerequisite action identifier.
    #[serde(default)]
    pub depends_on: Option<String>,
    /// Optional named MIDI output for an executable action.
    #[serde(default)]
    pub destination: Option<String>,
    /// Optional complete MIDI wire message for an executable action.
    #[serde(default)]
    pub message: Option<Vec<u8>>,
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
            actions: scene.actions.clone(),
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
        actions: source.actions.clone(),
    });
    Ok(result)
}

/// Appends one executable action to a named scene and validates the resulting project.
///
/// # Errors
///
/// Returns an error when the scene or action is invalid, duplicated, or makes the project invalid.
pub fn add_scene_action(
    project: &Project,
    scene_id: &str,
    action: SceneAction,
) -> Result<Project, String> {
    if !project.scenes.iter().any(|scene| scene.id == scene_id) {
        return Err(format!("unknown scene ID '{scene_id}'"));
    }
    let mut result = project.clone();
    let scene =
        result.scenes.iter_mut().find(|scene| scene.id == scene_id).ok_or("scene disappeared")?;
    if scene.actions.iter().any(|existing| existing.id == action.id) {
        return Err(format!("scene action '{}' already exists", action.id));
    }
    scene.actions.push(action);
    let document = ConfigDocument { projects: vec![result.clone()], ..ConfigDocument::default() };
    validate(&document)?;
    Ok(result)
}

/// Removes one action from a named scene and validates the resulting project.
///
/// # Errors
///
/// Returns an error when the scene/action is missing or another action depends on it.
pub fn remove_scene_action(
    project: &Project,
    scene_id: &str,
    action_id: &str,
) -> Result<Project, String> {
    let mut result = project.clone();
    let scene = result
        .scenes
        .iter_mut()
        .find(|scene| scene.id == scene_id)
        .ok_or_else(|| format!("unknown scene ID '{scene_id}'"))?;
    if scene.actions.iter().any(|action| action.depends_on.as_deref() == Some(action_id)) {
        return Err(format!("scene action '{action_id}' is required by another action"));
    }
    let Some(index) = scene.actions.iter().position(|action| action.id == action_id) else {
        return Err(format!("unknown scene action '{action_id}'"));
    };
    scene.actions.remove(index);
    let document = ConfigDocument { projects: vec![result.clone()], ..ConfigDocument::default() };
    validate(&document)?;
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

/// Returns scenes in declaration order whose category exactly matches the normalized query.
#[must_use]
pub fn scenes_in_category<'a>(project: &'a Project, category: &str) -> Vec<&'a SceneRef> {
    let category = category.trim().to_ascii_lowercase();
    project
        .scenes
        .iter()
        .filter(|scene| {
            scene.category.as_deref().is_some_and(|value| value.to_ascii_lowercase() == category)
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
#[allow(clippy::too_many_lines)]
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
        for scene in &project.scenes {
            if scene.actions.len() > 128 {
                return Err(format!("scene {} has more than 128 actions", scene.id));
            }
            unique(scene.actions.iter().map(|action| action.id.as_str()), "scene action")?;
            if scene.actions.iter().any(|action| action.description.trim().is_empty()) {
                return Err(format!("scene {} has an empty action description", scene.id));
            }
            for action in &scene.actions {
                if let Some(dependency) = &action.depends_on {
                    if !scene.actions.iter().any(|candidate| candidate.id == *dependency) {
                        return Err(format!(
                            "scene action '{}' has unknown dependency '{}'",
                            action.id, dependency
                        ));
                    }
                }
                let mut visited = std::collections::BTreeSet::new();
                let mut current = Some(action.id.as_str());
                while let Some(id) = current {
                    if !visited.insert(id) {
                        return Err(format!("scene action dependency cycle includes '{id}'"));
                    }
                    current = scene
                        .actions
                        .iter()
                        .find(|candidate| candidate.id == id)
                        .and_then(|candidate| candidate.depends_on.as_deref());
                }
                match (&action.destination, &action.message) {
                    (Some(destination), Some(message)) if !destination.trim().is_empty() => {
                        if message.is_empty() || message.len() > 8192 {
                            return Err(format!(
                                "scene action '{}' has an invalid message",
                                action.id
                            ));
                        }
                        mackes_domain::MidiMessage::from_wire(message).map_err(|_| {
                            format!("scene action '{}' has invalid MIDI bytes", action.id)
                        })?;
                    }
                    (None, None) => {}
                    _ => {
                        return Err(format!(
                            "scene action '{}' requires destination and message",
                            action.id
                        ))
                    }
                }
            }
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
    if let Some(template) = &document.settings.launch_control_template {
        if template.template >= 16 || template.assignments.len() > 48 {
            return Err("Launch Control template or assignment count is out of range".into());
        }
        let mut indices = std::collections::BTreeSet::new();
        for assignment in &template.assignments {
            if assignment.index >= 48
                || assignment.channel >= 16
                || assignment.number > 127
                || !matches!(assignment.kind.as_str(), "cc" | "note")
                || assignment
                    .physical_control_id
                    .as_deref()
                    .is_some_and(|id| !valid_physical_control_id(id))
                || assignment.destination.as_deref().is_some_and(|destination| {
                    destination.trim().is_empty() || destination.len() > 96
                })
                || !indices.insert(assignment.index)
            {
                return Err("Launch Control assignment is invalid or duplicated".into());
            }
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
    let mut mapping_ids = std::collections::BTreeSet::new();
    for mapping in &document.control_mappings {
        if mapping.id.trim().is_empty()
            || mapping.id.len() > 64
            || !mapping_ids.insert(mapping.id.as_str())
            || !valid_physical_control_id(&mapping.physical_control_id)
            || mapping.controller_profile.trim().is_empty()
            || mapping.source_endpoint.trim().is_empty()
            || mapping.source_kind.trim().is_empty()
            || mapping.source_channel > 15
            || mapping.source_number > 127
            || mapping.destination_endpoint.trim().is_empty()
            || mapping.destination_profile.trim().is_empty()
            || mapping.destination_effect.trim().is_empty()
            || mapping.destination_parameter.trim().is_empty()
            || mapping.profile_version == 0
        {
            return Err("control mapping identity or provenance is invalid or duplicated".into());
        }
        mapping.behavior.validate()?;
    }
    for draft in &document.control_mapping_drafts {
        if draft.id.trim().is_empty()
            || draft.id.len() > 64
            || !mapping_ids.insert(draft.id.as_str())
            || draft.step.trim().is_empty()
            || draft.physical_control_id.as_deref().is_some_and(|id| !valid_physical_control_id(id))
        {
            return Err("control mapping draft identity or step is invalid or duplicated".into());
        }
    }
    Ok(())
}

fn valid_physical_control_id(id: &str) -> bool {
    let valid_grid = |prefix: &str, rows: u8| {
        id.strip_prefix(prefix)
            .and_then(|tail| {
                let (row, column) = tail.split_once("-c")?;
                Some((row.parse::<u8>().ok()?, column.parse::<u8>().ok()?))
            })
            .is_some_and(|(row, column)| (1..=rows).contains(&row) && (1..=8).contains(&column))
    };
    let valid_numbered = |prefix: &str| {
        id.strip_prefix(prefix)
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (1..=8).contains(&n))
    };
    valid_grid("knob-r", 3)
        || valid_grid("button-r", 2)
        || valid_numbered("fader-")
        || valid_numbered("utility-")
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

/// Persists an authoritative mapping store through the validated atomic config writer.
///
/// # Errors
///
/// Returns the underlying load or atomic-save error; the in-memory store is never changed.
pub fn save_control_mapping_store(
    path: &Path,
    store: &ControlMappingStore,
    backup_count: usize,
) -> Result<(), ConfigError> {
    let document = load(path)?;
    save(path, &store.apply_to_document(&document), backup_count)
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
                launch_control_template: None,
            },
            endpoints: vec![],
            projects: vec![Project {
                id: "demo".to_owned(),
                scenes: vec![SceneRef {
                    id: "intro".to_owned(),
                    name: None,
                    category: None,
                    actions: Vec::new(),
                }],
            }],
            profiles: vec![],
            setlists: vec![],
            learned_mappings: vec![],
            control_mappings: vec![],
            control_mapping_drafts: vec![],
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
            set_default_provider(&selected, "reverb", "eventide.micropitch").expect("replace");
        assert_eq!(replaced.settings.default_providers.len(), 1);
        assert_eq!(default_provider(&replaced, "reverb"), Some("eventide.micropitch"));
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
            destination: "processor.delay.mix".into(),
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
    #[allow(clippy::too_many_lines)]
    fn scene_reorder_and_copy_are_explicit_and_non_mutating() {
        let mut project = document().projects[0].clone();
        project.scenes[0].name = Some("Intro ambience".into());
        project.scenes[0].category = Some("opening".into());
        project.scenes[0].actions = vec![SceneAction {
            id: "fade-in".into(),
            description: "Set opening level".into(),
            unsafe_action: false,
            depends_on: None,
            destination: None,
            message: None,
        }];
        let reordered = reorder_scenes(&project, &["intro"]).expect("reorder");
        assert_eq!(reordered.scenes, project.scenes);
        let copied = copy_scene(&project, "intro", "intro-copy").expect("copy");
        assert_eq!(project.scenes.len(), 1);
        assert_eq!(copied.scenes.last().map(|scene| scene.id.as_str()), Some("intro-copy"));
        assert_eq!(search_scenes(&project, "ambience").len(), 1);
        assert_eq!(search_scenes(&project, "OPENING").len(), 1);
        assert_eq!(scenes_in_category(&project, " opening ").len(), 1);
        assert!(scenes_in_category(&project, "missing").is_empty());
        assert_eq!(
            copied.scenes.last().and_then(|scene| scene.category.as_deref()),
            Some("opening")
        );
        assert_eq!(copied.scenes.last().map(|scene| scene.actions.len()), Some(1));
        let mut cyclic = document();
        cyclic.projects[0].scenes[0].actions = vec![
            SceneAction {
                id: "a".into(),
                description: "A".into(),
                unsafe_action: false,
                depends_on: Some("b".into()),
                destination: None,
                message: None,
            },
            SceneAction {
                id: "b".into(),
                description: "B".into(),
                unsafe_action: false,
                depends_on: Some("a".into()),
                destination: None,
                message: None,
            },
        ];
        assert!(validate(&cyclic).is_err());
        let mut oversized = document();
        oversized.projects[0].scenes[0].actions = (0..129)
            .map(|index| SceneAction {
                id: format!("action-{index}"),
                description: "bounded action".into(),
                unsafe_action: false,
                depends_on: None,
                destination: None,
                message: None,
            })
            .collect();
        assert!(validate(&oversized).is_err());
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
                scenes: vec![SceneRef {
                    id: "outro".into(),
                    name: None,
                    category: None,
                    actions: Vec::new(),
                }],
            },
        ];
        assert_eq!(search_projects(&projects, "OUT"), vec![&projects[1]]);
        assert_eq!(search_projects(&projects, "DEMO"), vec![&projects[0]]);
        assert!(search_projects(&projects, "missing").is_empty());
        let mut invalid = project.clone();
        invalid.scenes.push(SceneRef {
            id: "intro".into(),
            name: None,
            category: None,
            actions: Vec::new(),
        });
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

    /* #[test]
    fn retired_editor_map_requires_identity_and_artifact_hash() {
        let artifact = br"editor-export";
        let map = RetiredEditorMap {
            profile_id: "retired.device".into(),
            firmware: "1.0".into(),
            artifact_sha256: BackupManifest::digest(artifact),
            assignments: vec![],
        };
        assert!(map.validate().is_ok());
        assert!(map.validate_for_firmware("1.0", false).is_ok());
        assert!(map.validate_for_firmware("2.0", false).is_err());
        assert!(map.validate_for_firmware("2.0", true).is_ok());
        assert!(map.verify_artifact(artifact).is_ok());
        assert!(map.verify_artifact(b"tampered").is_err());
        let invalid = RetiredEditorMap { profile_id: "unknown".into(), ..map };
        assert!(invalid.validate().is_err());
    } */

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

    #[test]
    fn launch_control_legacy_assignments_migrate_without_guessing_faders() {
        let knob = LaunchControlAssignmentConfig {
            index: 4,
            physical_control_id: None,
            channel: 0,
            number: 20,
            kind: "cc".into(),
            destination: None,
            needs_review: false,
        };
        assert_eq!(knob.migrated_physical_control_id().as_deref(), Some("knob-r1-c5"));
        assert!(!knob.requires_review());
        let ambiguous = LaunchControlAssignmentConfig { index: 40, ..knob.clone() };
        assert_eq!(ambiguous.migrated_physical_control_id(), None);
        assert!(ambiguous.requires_review());
        let migrated = knob.migrated_factory1().expect("factory migration");
        assert_eq!(migrated.physical_control_id.as_deref(), Some("knob-r1-c5"));
        assert_eq!((migrated.channel, migrated.kind.as_str(), migrated.number), (8, "cc", 17));
        let button = LaunchControlAssignmentConfig {
            index: 24,
            physical_control_id: None,
            channel: 0,
            number: 69,
            kind: "cc".into(),
            destination: None,
            needs_review: false,
        };
        let migrated_button = button.migrated_factory1().expect("button migration");
        assert_eq!(migrated_button.physical_control_id.as_deref(), Some("button-r1-c1"));
        assert_eq!(
            (migrated_button.channel, migrated_button.kind.as_str(), migrated_button.number),
            (8, "note", 41)
        );
        assert!(LaunchControlAssignmentConfig { index: 4, number: 99, ..knob }
            .migrated_factory1()
            .is_some());
    }

    #[test]
    fn control_mapping_store_is_generation_guarded_and_undoable() {
        let mapping = ControlMapping {
            id: "map-1".into(),
            controller_profile: "novation.launch-control-xl.mk2".into(),
            physical_control_id: "knob-r1-c1".into(),
            source_endpoint: "input".into(),
            source_kind: "cc".into(),
            source_channel: 0,
            source_number: 21,
            destination_endpoint: "2".into(),
            destination_profile: "eventide.micropitch".into(),
            destination_effect: "modulation".into(),
            destination_parameter: "Mix".into(),
            behavior: MappingBehavior {
                source_range: (0, 127),
                destination_range: (0, 127),
                invert: false,
                curve: "linear".into(),
            },
            enabled: true,
            profile_version: 1,
        };
        let mut store = ControlMappingStore::default();
        assert!(store.activate(1, mapping.clone()).is_err());
        assert!(store.activate(0, mapping).is_ok());
        assert_eq!(store.generation, 1);
        assert!(store.undo_available());
        assert_eq!(store.snapshot().0.len(), 1);
        assert!(store.undo(0).is_err());
        assert!(store.undo(1).is_ok());
        assert!(store.active.is_empty());
        assert!(!store.undo_available());
    }

    #[test]
    fn control_mapping_store_supports_drafts_and_all_atomic_mutations() {
        let mapping = ControlMapping {
            id: "map-1".into(),
            controller_profile: "novation.launch-control-xl.mk2".into(),
            physical_control_id: "knob-r1-c1".into(),
            source_endpoint: "input".into(),
            source_kind: "cc".into(),
            source_channel: 0,
            source_number: 21,
            destination_endpoint: "2".into(),
            destination_profile: "eventide.micropitch".into(),
            destination_effect: "modulation".into(),
            destination_parameter: "Mix".into(),
            behavior: MappingBehavior {
                source_range: (0, 127),
                destination_range: (0, 127),
                invert: false,
                curve: "linear".into(),
            },
            enabled: true,
            profile_version: 1,
        };
        let mut store = ControlMappingStore::default();
        let draft = ControlMappingDraft {
            id: "draft-1".into(),
            step: "source".into(),
            physical_control_id: Some("knob-r1-c1".into()),
            destination: None,
        };
        assert!(store.save_draft(0, draft.clone()).is_ok());
        assert_eq!(store.generation, 1);
        assert!(store
            .save_draft(1, ControlMappingDraft { step: "destination".into(), ..draft })
            .is_ok());
        assert!(store.activate(2, mapping.clone()).is_ok());
        assert!(store
            .update_behavior(
                3,
                "map-1",
                MappingBehavior {
                    source_range: (0, 127),
                    destination_range: (0, 127),
                    invert: true,
                    curve: "square".into()
                }
            )
            .is_ok());
        assert!(store.set_enabled(4, "map-1", false).is_ok());
        assert!(store
            .replace(5, ControlMapping { destination_parameter: "Depth".into(), ..mapping })
            .is_ok());
        assert!(store.delete(6, "map-1").is_ok());
        let before = store.clone();
        assert!(store.delete(7, "missing").is_err());
        assert_eq!(store, before);
    }

    #[test]
    fn control_mapping_store_round_trips_through_config_document() {
        let document = document();
        let store = ControlMappingStore::from_document(&document).expect("valid document");
        assert_eq!(store.apply_to_document(&document), document);
    }

    #[test]
    fn control_mapping_store_persists_atomically_and_reloads() {
        let path =
            std::env::temp_dir().join(format!("mackes-mapping-{}.json5", std::process::id()));
        let document = document();
        save(&path, &document, 0).expect("initial config");
        let mut store = ControlMappingStore::from_document(&document).expect("valid document");
        let mapping = ControlMapping {
            id: "persisted".into(),
            controller_profile: "controller".into(),
            physical_control_id: "knob-r1-c1".into(),
            source_endpoint: "input-1".into(),
            source_kind: "cc".into(),
            source_channel: 0,
            source_number: 1,
            destination_endpoint: "output-1".into(),
            destination_profile: "eventide.micropitch".into(),
            destination_effect: "modulation".into(),
            destination_parameter: "control-0".into(),
            behavior: MappingBehavior {
                source_range: (0, 127),
                destination_range: (0, 127),
                invert: false,
                curve: "linear".into(),
            },
            enabled: true,
            profile_version: 1,
        };
        store.activate(0, mapping).expect("activate");
        save_control_mapping_store(&path, &store, 0).expect("persist mapping");
        let loaded = ControlMappingStore::from_document(&load(&path).expect("reload"))
            .expect("loaded store");
        assert_eq!(loaded.active, store.active);
        let before = store.clone();
        let missing = path.with_extension("missing.json5");
        assert!(save_control_mapping_store(&missing, &store, 0).is_err());
        assert_eq!(store, before);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn replacing_mapping_rejects_control_or_destination_collisions() {
        let first = ControlMapping {
            id: "first".into(),
            controller_profile: "controller".into(),
            physical_control_id: "knob-r1-c1".into(),
            source_endpoint: "input".into(),
            source_kind: "cc".into(),
            source_channel: 0,
            source_number: 1,
            destination_endpoint: "2".into(),
            destination_profile: "device".into(),
            destination_effect: "effect".into(),
            destination_parameter: "one".into(),
            behavior: MappingBehavior {
                source_range: (0, 127),
                destination_range: (0, 127),
                invert: false,
                curve: "linear".into(),
            },
            enabled: true,
            profile_version: 1,
        };
        let second = ControlMapping {
            id: "second".into(),
            physical_control_id: "knob-r1-c2".into(),
            destination_parameter: "two".into(),
            ..first.clone()
        };
        let mut store = ControlMappingStore {
            active: vec![first.clone(), second],
            ..ControlMappingStore::default()
        };
        let collision = ControlMapping { destination_parameter: "two".into(), ..first };
        assert!(store.replace(0, collision).is_err());
        assert_eq!(store.active[0].destination_parameter, "one");
    }

    #[test]
    fn runtime_replacement_failure_restores_store_atomically() {
        let mapping = ControlMapping {
            id: "runtime".into(),
            controller_profile: "controller".into(),
            physical_control_id: "knob-r1-c1".into(),
            source_endpoint: "input".into(),
            source_kind: "cc".into(),
            source_channel: 0,
            source_number: 1,
            destination_endpoint: "output".into(),
            destination_profile: "profile".into(),
            destination_effect: "effect".into(),
            destination_parameter: "one".into(),
            behavior: MappingBehavior {
                source_range: (0, 127),
                destination_range: (0, 127),
                invert: false,
                curve: "linear".into(),
            },
            enabled: true,
            profile_version: 1,
        };
        let mut store = ControlMappingStore::default();
        store.activate(0, mapping.clone()).expect("activate");
        let before = store.clone();
        let replacement = ControlMapping { destination_parameter: "two".into(), ..mapping };
        assert!(store
            .replace_with_runtime(1, replacement, |_| Err("adapter unavailable"))
            .is_err());
        assert_eq!(store, before);
    }
}
