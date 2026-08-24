use dravyn_common::Workspace;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const BASELINE_FILE: &str = "baseline.json";
const LATEST_FILE: &str = "latest.json";
const HISTORY_DIR: &str = "history";
const MAX_SURFACES: usize = 96;
const MAX_HISTORY: usize = 50;
const MAX_ISSUES: usize = 32;
const MAX_KEY_CHARS: usize = 80;
const MAX_LABEL_CHARS: usize = 120;
const MAX_CATEGORY_CHARS: usize = 48;
const MAX_VALUE_CHARS: usize = 8_192;
const MAX_ISSUE_CHARS: usize = 500;
static SNAPSHOT_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SurfaceSubmission {
    pub key: String,
    pub label: String,
    pub category: String,
    pub value: String,
    #[serde(default = "default_stable")]
    pub stable: bool,
}

fn default_stable() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AuditSubmission {
    #[serde(default)]
    pub surfaces: Vec<SurfaceSubmission>,
    #[serde(default)]
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SurfaceRecord {
    pub key: String,
    pub label: String,
    pub category: String,
    pub value: String,
    pub stable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DriftItem {
    pub key: String,
    pub label: String,
    pub category: String,
    pub baseline_value: String,
    pub current_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FingerprintSnapshot {
    pub id: String,
    pub profile_id: String,
    pub captured_at: u64,
    pub consistency_score: u8,
    pub surfaces: Vec<SurfaceRecord>,
    pub issues: Vec<String>,
    pub drift: Vec<DriftItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FingerprintBaseline {
    pub profile_id: String,
    pub snapshot_id: String,
    pub created_at: u64,
    pub surfaces: BTreeMap<String, BaselineSurface>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineSurface {
    pub label: String,
    pub category: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FingerprintSummary {
    pub profile_id: String,
    pub baseline_present: bool,
    pub snapshot_count: usize,
    pub last_captured_at: Option<u64>,
    pub consistency_score: Option<u8>,
    pub drift_count: usize,
    pub issue_count: usize,
    pub surface_count: usize,
    pub state: String,
}

impl FingerprintSummary {
    pub fn empty(profile_id: impl Into<String>) -> Self {
        Self {
            profile_id: profile_id.into(),
            baseline_present: false,
            snapshot_count: 0,
            last_captured_at: None,
            consistency_score: None,
            drift_count: 0,
            issue_count: 0,
            surface_count: 0,
            state: "not_audited".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FingerprintHistoryEntry {
    pub snapshot_id: String,
    pub captured_at: u64,
    pub consistency_score: u8,
    pub drift_count: usize,
    pub issue_count: usize,
    pub surface_count: usize,
}

#[derive(Debug, Clone)]
pub struct FingerprintStore {
    workspace: Workspace,
}

impl FingerprintStore {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }

    pub fn ensure_layout(&self) -> Result<(), FingerprintError> {
        fs::create_dir_all(self.workspace.fingerprints_dir())?;
        Ok(())
    }

    pub fn capture(
        &self,
        profile_id: &str,
        submission: AuditSubmission,
    ) -> Result<FingerprintSnapshot, FingerprintError> {
        validate_profile_id(profile_id)?;
        let surfaces = normalize_surfaces(submission.surfaces)?;
        if surfaces.is_empty() {
            return Err(FingerprintError::Validation(
                "fingerprint audit must contain at least one surface".to_owned(),
            ));
        }
        let issues = normalize_issues(submission.issues);
        self.ensure_profile_layout(profile_id)?;

        let captured_at = epoch_seconds();
        let counter = SNAPSHOT_COUNTER.fetch_add(1, Ordering::Relaxed);
        let snapshot_id = format!("{captured_at:016x}-{counter:08x}");

        let baseline_path = self.profile_dir_unchecked(profile_id).join(BASELINE_FILE);
        let baseline = if baseline_path.is_file() {
            Some(read_json::<FingerprintBaseline>(&baseline_path)?)
        } else {
            None
        };
        let drift = baseline
            .as_ref()
            .map(|baseline| compare_baseline(baseline, &surfaces))
            .unwrap_or_default();
        let penalty = issues.len().saturating_mul(5) + drift.len().saturating_mul(7);
        let consistency_score = 100u8.saturating_sub(penalty.min(100) as u8);

        let snapshot = FingerprintSnapshot {
            id: snapshot_id,
            profile_id: profile_id.to_owned(),
            captured_at,
            consistency_score,
            surfaces,
            issues,
            drift,
        };

        if baseline.is_none() {
            let initial = baseline_from_snapshot(&snapshot);
            write_json_atomic(&baseline_path, &initial)?;
        }

        let profile_dir = self.profile_dir_unchecked(profile_id);
        write_json_atomic(&profile_dir.join(LATEST_FILE), &snapshot)?;
        write_json_atomic(
            &profile_dir
                .join(HISTORY_DIR)
                .join(format!("{}.json", snapshot.id)),
            &snapshot,
        )?;
        self.prune_history(profile_id)?;
        Ok(snapshot)
    }

    pub fn latest(&self, profile_id: &str) -> Result<Option<FingerprintSnapshot>, FingerprintError> {
        validate_profile_id(profile_id)?;
        let path = self.profile_dir_unchecked(profile_id).join(LATEST_FILE);
        if !path.is_file() {
            return Ok(None);
        }
        Ok(Some(read_json(&path)?))
    }

    pub fn baseline(&self, profile_id: &str) -> Result<Option<FingerprintBaseline>, FingerprintError> {
        validate_profile_id(profile_id)?;
        let path = self.profile_dir_unchecked(profile_id).join(BASELINE_FILE);
        if !path.is_file() {
            return Ok(None);
        }
        Ok(Some(read_json(&path)?))
    }

    pub fn summary(&self, profile_id: &str) -> Result<FingerprintSummary, FingerprintError> {
        validate_profile_id(profile_id)?;
        let baseline_present = self
            .profile_dir_unchecked(profile_id)
            .join(BASELINE_FILE)
            .is_file();
        let snapshot_count = self.history_files(profile_id)?.len();
        let Some(latest) = self.latest(profile_id)? else {
            let mut empty = FingerprintSummary::empty(profile_id);
            empty.baseline_present = baseline_present;
            empty.snapshot_count = snapshot_count;
            return Ok(empty);
        };

        let state = if !latest.drift.is_empty() {
            "drift"
        } else if !latest.issues.is_empty() {
            "review"
        } else {
            "stable"
        };
        Ok(FingerprintSummary {
            profile_id: profile_id.to_owned(),
            baseline_present,
            snapshot_count,
            last_captured_at: Some(latest.captured_at),
            consistency_score: Some(latest.consistency_score),
            drift_count: latest.drift.len(),
            issue_count: latest.issues.len(),
            surface_count: latest.surfaces.len(),
            state: state.to_owned(),
        })
    }

    pub fn history(
        &self,
        profile_id: &str,
        limit: usize,
    ) -> Result<Vec<FingerprintHistoryEntry>, FingerprintError> {
        validate_profile_id(profile_id)?;
        let mut entries = Vec::new();
        for path in self.history_files(profile_id)? {
            let snapshot: FingerprintSnapshot = read_json(&path)?;
            entries.push(FingerprintHistoryEntry {
                snapshot_id: snapshot.id,
                captured_at: snapshot.captured_at,
                consistency_score: snapshot.consistency_score,
                drift_count: snapshot.drift.len(),
                issue_count: snapshot.issues.len(),
                surface_count: snapshot.surfaces.len(),
            });
        }
        entries.sort_by(|left, right| right.captured_at.cmp(&left.captured_at));
        entries.truncate(limit.min(MAX_HISTORY));
        Ok(entries)
    }

    pub fn set_baseline_from_latest(
        &self,
        profile_id: &str,
    ) -> Result<FingerprintBaseline, FingerprintError> {
        let latest = self.latest(profile_id)?.ok_or_else(|| {
            FingerprintError::Validation(
                "run a fingerprint audit before setting the baseline".to_owned(),
            )
        })?;
        let baseline = baseline_from_snapshot(&latest);
        self.ensure_profile_layout(profile_id)?;
        write_json_atomic(
            &self.profile_dir_unchecked(profile_id).join(BASELINE_FILE),
            &baseline,
        )?;
        Ok(baseline)
    }

    pub fn clear_profile(&self, profile_id: &str) -> Result<(), FingerprintError> {
        validate_profile_id(profile_id)?;
        let path = self.profile_dir_unchecked(profile_id);
        if path.exists() {
            fs::remove_dir_all(path)?;
        }
        Ok(())
    }

    fn ensure_profile_layout(&self, profile_id: &str) -> Result<(), FingerprintError> {
        validate_profile_id(profile_id)?;
        fs::create_dir_all(self.profile_dir_unchecked(profile_id).join(HISTORY_DIR))?;
        Ok(())
    }

    fn profile_dir_unchecked(&self, profile_id: &str) -> PathBuf {
        self.workspace.fingerprints_dir().join(profile_id)
    }

    fn history_files(&self, profile_id: &str) -> Result<Vec<PathBuf>, FingerprintError> {
        validate_profile_id(profile_id)?;
        let directory = self.profile_dir_unchecked(profile_id).join(HISTORY_DIR);
        if !directory.is_dir() {
            return Ok(Vec::new());
        }
        let mut files = fs::read_dir(directory)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        files.sort();
        Ok(files)
    }

    fn prune_history(&self, profile_id: &str) -> Result<(), FingerprintError> {
        let mut files = self.history_files(profile_id)?;
        if files.len() <= MAX_HISTORY {
            return Ok(());
        }
        files.sort();
        let remove_count = files.len() - MAX_HISTORY;
        for path in files.into_iter().take(remove_count) {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

fn baseline_from_snapshot(snapshot: &FingerprintSnapshot) -> FingerprintBaseline {
    let surfaces = snapshot
        .surfaces
        .iter()
        .filter(|surface| surface.stable)
        .map(|surface| {
            (
                surface.key.clone(),
                BaselineSurface {
                    label: surface.label.clone(),
                    category: surface.category.clone(),
                    value: surface.value.clone(),
                },
            )
        })
        .collect();
    FingerprintBaseline {
        profile_id: snapshot.profile_id.clone(),
        snapshot_id: snapshot.id.clone(),
        created_at: epoch_seconds(),
        surfaces,
    }
}

fn compare_baseline(
    baseline: &FingerprintBaseline,
    surfaces: &[SurfaceRecord],
) -> Vec<DriftItem> {
    let current = surfaces
        .iter()
        .filter(|surface| surface.stable)
        .map(|surface| (surface.key.as_str(), surface))
        .collect::<BTreeMap<_, _>>();
    let mut drift = Vec::new();
    for (key, expected) in &baseline.surfaces {
        match current.get(key.as_str()) {
            Some(actual) if actual.value != expected.value => drift.push(DriftItem {
                key: key.clone(),
                label: expected.label.clone(),
                category: expected.category.clone(),
                baseline_value: expected.value.clone(),
                current_value: actual.value.clone(),
            }),
            None => drift.push(DriftItem {
                key: key.clone(),
                label: expected.label.clone(),
                category: expected.category.clone(),
                baseline_value: expected.value.clone(),
                current_value: "Unavailable".to_owned(),
            }),
            _ => {}
        }
    }
    drift
}

fn normalize_surfaces(input: Vec<SurfaceSubmission>) -> Result<Vec<SurfaceRecord>, FingerprintError> {
    if input.len() > MAX_SURFACES {
        return Err(FingerprintError::Validation(format!(
            "fingerprint audit cannot exceed {MAX_SURFACES} surfaces"
        )));
    }
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for surface in input {
        let key = surface.key.trim().to_owned();
        if key.is_empty()
            || key.chars().count() > MAX_KEY_CHARS
            || !key
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_'))
        {
            return Err(FingerprintError::Validation(format!(
                "invalid fingerprint surface key: {}",
                surface.key
            )));
        }
        if !seen.insert(key.clone()) {
            continue;
        }
        normalized.push(SurfaceRecord {
            key,
            label: truncate(surface.label.trim(), MAX_LABEL_CHARS),
            category: truncate(surface.category.trim(), MAX_CATEGORY_CHARS),
            value: truncate(surface.value.trim(), MAX_VALUE_CHARS),
            stable: surface.stable,
        });
    }
    normalized.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(normalized)
}

fn normalize_issues(input: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    input
        .into_iter()
        .map(|issue| truncate(issue.trim(), MAX_ISSUE_CHARS))
        .filter(|issue| !issue.is_empty() && seen.insert(issue.clone()))
        .take(MAX_ISSUES)
        .collect()
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    value.chars().take(max_chars).collect()
}

fn validate_profile_id(id: &str) -> Result<(), FingerprintError> {
    if id.is_empty()
        || id.len() > 96
        || !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        return Err(FingerprintError::InvalidProfileId(id.to_owned()));
    }
    Ok(())
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, FingerprintError> {
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(FingerprintError::Json)
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), FingerprintError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug)]
pub enum FingerprintError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Validation(String),
    InvalidProfileId(String),
}

impl fmt::Display for FingerprintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FingerprintError::Io(error) => write!(f, "fingerprint storage error: {error}"),
            FingerprintError::Json(error) => write!(f, "invalid fingerprint JSON: {error}"),
            FingerprintError::Validation(message) => write!(f, "invalid fingerprint audit: {message}"),
            FingerprintError::InvalidProfileId(id) => write!(f, "invalid fingerprint profile id: {id}"),
        }
    }
}

impl std::error::Error for FingerprintError {}

impl From<std::io::Error> for FingerprintError {
    fn from(value: std::io::Error) -> Self {
        FingerprintError::Io(value)
    }
}

impl From<serde_json::Error> for FingerprintError {
    fn from(value: serde_json::Error) -> Self {
        FingerprintError::Json(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn store(tag: &str) -> FingerprintStore {
        let root = env::temp_dir().join(format!(
            "dravyn-fingerprint-{tag}-{}-{}",
            std::process::id(),
            epoch_seconds()
        ));
        let _ = fs::remove_dir_all(&root);
        FingerprintStore::new(Workspace::from_root(root))
    }

    fn audit(canvas: &str, platform: &str) -> AuditSubmission {
        AuditSubmission {
            surfaces: vec![
                SurfaceSubmission {
                    key: "canvas.hash".to_owned(),
                    label: "Canvas".to_owned(),
                    category: "Rendering".to_owned(),
                    value: canvas.to_owned(),
                    stable: true,
                },
                SurfaceSubmission {
                    key: "identity.platform".to_owned(),
                    label: "Platform".to_owned(),
                    category: "Identity".to_owned(),
                    value: platform.to_owned(),
                    stable: true,
                },
            ],
            issues: Vec::new(),
        }
    }

    #[test]
    fn first_capture_creates_per_profile_baseline() {
        let store = store("baseline");
        let snapshot = store.capture("profile-a", audit("a1", "Linux")).unwrap();
        assert!(snapshot.drift.is_empty());
        let baseline = store.baseline("profile-a").unwrap().unwrap();
        assert_eq!(baseline.snapshot_id, snapshot.id);
        assert_eq!(store.summary("profile-a").unwrap().state, "stable");
        let _ = fs::remove_dir_all(store.workspace.fingerprints_dir().parent().unwrap());
    }

    #[test]
    fn later_capture_detects_stable_surface_drift() {
        let store = store("drift");
        store.capture("profile-a", audit("a1", "Linux")).unwrap();
        let snapshot = store.capture("profile-a", audit("b2", "Linux")).unwrap();
        assert_eq!(snapshot.drift.len(), 1);
        assert_eq!(snapshot.drift[0].key, "canvas.hash");
        let summary = store.summary("profile-a").unwrap();
        assert_eq!(summary.state, "drift");
        assert!(summary.consistency_score.unwrap() < 100);
    }

    #[test]
    fn profiles_have_independent_baselines() {
        let store = store("independent");
        store.capture("profile-a", audit("a1", "Linux")).unwrap();
        store.capture("profile-b", audit("z9", "Linux")).unwrap();
        assert_eq!(
            store
                .baseline("profile-a")
                .unwrap()
                .unwrap()
                .surfaces["canvas.hash"]
                .value,
            "a1"
        );
        assert_eq!(
            store
                .baseline("profile-b")
                .unwrap()
                .unwrap()
                .surfaces["canvas.hash"]
                .value,
            "z9"
        );
    }
}
