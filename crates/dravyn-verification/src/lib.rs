use dravyn_common::Workspace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const HISTORY_DIR: &str = "history";
const MAX_HISTORY: usize = 100;
static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum VerificationTest {
    BrowserleaksIp,
    BrowserleaksWebrtc,
    BrowserleaksDns,
    BrowserleaksIpv6,
    BrowserleaksCanvas,
    BrowserleaksWebgl,
    Eff,
    Amiunique,
}

impl VerificationTest {
    pub fn label(self) -> &'static str {
        match self {
            Self::BrowserleaksIp => "Public IP",
            Self::BrowserleaksWebrtc => "WebRTC",
            Self::BrowserleaksDns => "DNS",
            Self::BrowserleaksIpv6 => "IPv6",
            Self::BrowserleaksCanvas => "Canvas",
            Self::BrowserleaksWebgl => "WebGL",
            Self::Eff => "EFF Cover Your Tracks",
            Self::Amiunique => "AmIUnique",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationResult {
    Pass,
    Warning,
    Critical,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationState {
    Unverified,
    Healthy,
    Review,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationDraft {
    pub test: VerificationTest,
    pub result: VerificationResult,
    #[serde(default)]
    pub expected: Option<String>,
    #[serde(default)]
    pub observed: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub chromium_version: Option<String>,
    #[serde(default = "default_policy_version")]
    pub policy_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationRecord {
    pub id: String,
    pub profile_id: String,
    pub test: VerificationTest,
    pub result: VerificationResult,
    pub expected: Option<String>,
    pub observed: Option<String>,
    pub notes: String,
    pub source_url: Option<String>,
    pub chromium_version: Option<String>,
    pub policy_version: u32,
    pub verified_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationSummary {
    pub profile_id: String,
    pub record_count: usize,
    pub latest_test_count: usize,
    pub pass_count: usize,
    pub warning_count: usize,
    pub critical_count: usize,
    pub inconclusive_count: usize,
    pub last_verified_at: Option<u64>,
    pub state: VerificationState,
}

impl VerificationSummary {
    fn empty(profile_id: &str) -> Self {
        Self {
            profile_id: profile_id.to_owned(),
            record_count: 0,
            latest_test_count: 0,
            pass_count: 0,
            warning_count: 0,
            critical_count: 0,
            inconclusive_count: 0,
            last_verified_at: None,
            state: VerificationState::Unverified,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VerificationStore {
    workspace: Workspace,
}

impl VerificationStore {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }

    pub fn ensure_layout(&self) -> Result<(), VerificationError> {
        fs::create_dir_all(self.workspace.verifications_dir())?;
        Ok(())
    }

    pub fn record(
        &self,
        profile_id: &str,
        mut draft: VerificationDraft,
    ) -> Result<VerificationRecord, VerificationError> {
        validate_profile_id(profile_id)?;
        normalize_draft(&mut draft)?;
        self.ensure_layout()?;

        let record = VerificationRecord {
            id: generate_id(),
            profile_id: profile_id.to_owned(),
            test: draft.test,
            result: draft.result,
            expected: draft.expected,
            observed: draft.observed,
            notes: draft.notes,
            source_url: draft.source_url,
            chromium_version: draft.chromium_version,
            policy_version: draft.policy_version,
            verified_at: epoch_seconds(),
        };

        let dir = self.history_dir(profile_id);
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", record.id));
        let temporary = dir.join(format!("{}.tmp", record.id));
        fs::write(&temporary, serde_json::to_vec_pretty(&record)?)?;
        fs::rename(&temporary, &path)?;
        self.prune(profile_id)?;
        Ok(record)
    }

    pub fn history(
        &self,
        profile_id: &str,
        limit: usize,
    ) -> Result<Vec<VerificationRecord>, VerificationError> {
        validate_profile_id(profile_id)?;
        let mut records = self.read_all(profile_id)?;
        records.truncate(limit);
        Ok(records)
    }

    pub fn summary(&self, profile_id: &str) -> Result<VerificationSummary, VerificationError> {
        let history = self.history(profile_id, MAX_HISTORY)?;
        if history.is_empty() {
            return Ok(VerificationSummary::empty(profile_id));
        }

        let mut latest: HashMap<VerificationTest, &VerificationRecord> = HashMap::new();
        for record in &history {
            latest.entry(record.test).or_insert(record);
        }

        let mut summary = VerificationSummary::empty(profile_id);
        summary.record_count = history.len();
        summary.latest_test_count = latest.len();
        summary.last_verified_at = history.first().map(|record| record.verified_at);
        for record in latest.values() {
            match record.result {
                VerificationResult::Pass => summary.pass_count += 1,
                VerificationResult::Warning => summary.warning_count += 1,
                VerificationResult::Critical => summary.critical_count += 1,
                VerificationResult::Inconclusive => summary.inconclusive_count += 1,
            }
        }
        summary.state = if summary.critical_count > 0 {
            VerificationState::Critical
        } else if summary.warning_count > 0 || summary.inconclusive_count > 0 {
            VerificationState::Review
        } else {
            VerificationState::Healthy
        };
        Ok(summary)
    }

    pub fn clear_profile(&self, profile_id: &str) -> Result<(), VerificationError> {
        validate_profile_id(profile_id)?;
        let dir = self.profile_dir(profile_id);
        if dir.exists() {
            fs::remove_dir_all(dir)?;
        }
        Ok(())
    }

    fn read_all(&self, profile_id: &str) -> Result<Vec<VerificationRecord>, VerificationError> {
        let dir = self.history_dir(profile_id);
        if !dir.is_dir() {
            return Ok(Vec::new());
        }

        let mut records = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let bytes = fs::read(entry.path())?;
            let record: VerificationRecord = serde_json::from_slice(&bytes)?;
            if record.profile_id == profile_id {
                records.push(record);
            }
        }
        records.sort_by(|left, right| {
            right
                .verified_at
                .cmp(&left.verified_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        Ok(records)
    }

    fn profile_dir(&self, profile_id: &str) -> PathBuf {
        self.workspace.verifications_dir().join(profile_id)
    }

    fn history_dir(&self, profile_id: &str) -> PathBuf {
        self.profile_dir(profile_id).join(HISTORY_DIR)
    }

    fn prune(&self, profile_id: &str) -> Result<(), VerificationError> {
        let records = self.read_all(profile_id)?;
        for record in records.iter().skip(MAX_HISTORY) {
            let path = self.history_dir(profile_id).join(format!("{}.json", record.id));
            if path.is_file() {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }
}

fn normalize_draft(draft: &mut VerificationDraft) -> Result<(), VerificationError> {
    draft.notes = draft.notes.trim().to_owned();
    draft.expected = normalize_optional(draft.expected.take());
    draft.observed = normalize_optional(draft.observed.take());
    draft.source_url = normalize_optional(draft.source_url.take());
    draft.chromium_version = normalize_optional(draft.chromium_version.take());
    if draft.notes.chars().count() > 4_000 {
        return Err(VerificationError::Validation(
            "verification notes cannot exceed 4000 characters".to_owned(),
        ));
    }
    if draft.policy_version == 0 {
        return Err(VerificationError::Validation(
            "policy version must be at least 1".to_owned(),
        ));
    }
    Ok(())
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_owned();
        (!value.is_empty()).then_some(value)
    })
}

fn validate_profile_id(id: &str) -> Result<(), VerificationError> {
    if id.is_empty()
        || id.len() > 96
        || !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        return Err(VerificationError::InvalidProfileId(id.to_owned()));
    }
    Ok(())
}

fn default_policy_version() -> u32 {
    1
}

fn generate_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:032x}-{:08x}-{counter:08x}", std::process::id())
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug)]
pub enum VerificationError {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidProfileId(String),
    Validation(String),
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "verification storage error: {error}"),
            Self::Json(error) => write!(f, "invalid verification JSON: {error}"),
            Self::InvalidProfileId(id) => write!(f, "invalid verification profile id: {id}"),
            Self::Validation(message) => write!(f, "invalid verification record: {message}"),
        }
    }
}

impl std::error::Error for VerificationError {}

impl From<std::io::Error> for VerificationError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for VerificationError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn store(tag: &str) -> VerificationStore {
        let root = env::temp_dir().join(format!(
            "dravyn-verification-{tag}-{}-{}",
            std::process::id(),
            epoch_seconds()
        ));
        let _ = fs::remove_dir_all(&root);
        VerificationStore::new(Workspace::from_root(root))
    }

    fn draft(test: VerificationTest, result: VerificationResult) -> VerificationDraft {
        VerificationDraft {
            test,
            result,
            expected: None,
            observed: None,
            notes: String::new(),
            source_url: None,
            chromium_version: None,
            policy_version: 1,
        }
    }

    #[test]
    fn records_history_and_latest_summary() {
        let store = store("summary");
        store
            .record(
                "profile-a",
                draft(VerificationTest::BrowserleaksIp, VerificationResult::Critical),
            )
            .unwrap();
        store
            .record(
                "profile-a",
                draft(VerificationTest::BrowserleaksIp, VerificationResult::Pass),
            )
            .unwrap();
        store
            .record(
                "profile-a",
                draft(VerificationTest::BrowserleaksWebrtc, VerificationResult::Warning),
            )
            .unwrap();

        let summary = store.summary("profile-a").unwrap();
        assert_eq!(summary.record_count, 3);
        assert_eq!(summary.latest_test_count, 2);
        assert_eq!(summary.pass_count, 1);
        assert_eq!(summary.warning_count, 1);
        assert_eq!(summary.critical_count, 0);
        assert_eq!(summary.state, VerificationState::Review);
    }

    #[test]
    fn clear_profile_removes_records() {
        let store = store("clear");
        store
            .record(
                "profile-a",
                draft(VerificationTest::BrowserleaksDns, VerificationResult::Pass),
            )
            .unwrap();
        store.clear_profile("profile-a").unwrap();
        assert!(store.history("profile-a", 20).unwrap().is_empty());
    }
}
