use dravyn_common::Workspace;
use dravyn_network::NetworkConfig;
use dravyn_privacy::{PrivacyPolicy, PRIVACY_SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const PROFILE_FILE: &str = "profile.json";
const USER_DATA_DIR: &str = "user-data";
const MAX_NAME_CHARS: usize = 80;
const MAX_NOTES_CHARS: usize = 4_000;
const MAX_TAGS: usize = 20;
const MAX_TAG_CHARS: usize = 40;
static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct BrowserConfig {
    pub start_url: Option<String>,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            start_url: None,
            window_width: Some(1280),
            window_height: Some(800),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub browser: BrowserConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub privacy: PrivacyPolicy,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct ProfileDraft {
    pub name: String,
    pub notes: String,
    pub tags: Vec<String>,
    pub browser: BrowserConfig,
    pub network: NetworkConfig,
    pub privacy: PrivacyPolicy,
}

#[derive(Debug, Clone)]
pub struct ProfileStore {
    workspace: Workspace,
}

impl ProfileStore {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn ensure_layout(&self) -> Result<(), StoreError> {
        fs::create_dir_all(self.workspace.profiles_dir())?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<Profile>, StoreError> {
        let root = self.workspace.profiles_dir();
        if !root.exists() {
            return Ok(Vec::new());
        }

        let mut profiles = Vec::new();
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let path = entry.path().join(PROFILE_FILE);
            if !path.is_file() {
                continue;
            }
            profiles.push(read_profile(&path)?);
        }
        profiles.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });
        Ok(profiles)
    }

    pub fn get(&self, id: &str) -> Result<Profile, StoreError> {
        validate_profile_id(id)?;
        let path = self.profile_dir_unchecked(id).join(PROFILE_FILE);
        if !path.is_file() {
            return Err(StoreError::NotFound(id.to_owned()));
        }
        let profile = read_profile(&path)?;
        if profile.id != id {
            return Err(StoreError::Corrupt(format!(
                "profile file {} contains a mismatched id",
                path.display()
            )));
        }
        Ok(profile)
    }

    pub fn create(&self, draft: ProfileDraft) -> Result<Profile, StoreError> {
        let mut draft = normalize_and_validate_draft(draft)?;
        draft.privacy.schema_version = PRIVACY_SCHEMA_VERSION;
        draft.privacy.policy_version = 1;
        self.ensure_layout()?;

        let now = epoch_seconds();
        let profile = Profile {
            id: generate_profile_id(),
            name: draft.name,
            notes: draft.notes,
            tags: draft.tags,
            browser: draft.browser,
            network: draft.network,
            privacy: draft.privacy,
            created_at: now,
            updated_at: now,
        };

        let profile_dir = self.profile_dir_unchecked(&profile.id);
        fs::create_dir_all(profile_dir.join(USER_DATA_DIR))?;
        self.write_profile(&profile)?;
        Ok(profile)
    }

    pub fn update(&self, id: &str, draft: ProfileDraft) -> Result<Profile, StoreError> {
        let current = self.get(id)?;
        let mut draft = normalize_and_validate_draft(draft)?;
        draft.privacy.schema_version = PRIVACY_SCHEMA_VERSION;
        draft.privacy.policy_version = if privacy_semantics_equal(&current.privacy, &draft.privacy) {
            current.privacy.policy_version
        } else {
            current.privacy.policy_version.saturating_add(1).max(1)
        };
        let updated = Profile {
            id: current.id,
            name: draft.name,
            notes: draft.notes,
            tags: draft.tags,
            browser: draft.browser,
            network: draft.network,
            privacy: draft.privacy,
            created_at: current.created_at,
            updated_at: epoch_seconds(),
        };
        self.write_profile(&updated)?;
        Ok(updated)
    }

    pub fn delete(&self, id: &str) -> Result<(), StoreError> {
        self.get(id)?;
        fs::remove_dir_all(self.profile_dir_unchecked(id))?;
        Ok(())
    }

    pub fn reset_user_data(&self, id: &str) -> Result<(), StoreError> {
        self.get(id)?;
        let user_data = self.user_data_dir(id)?;
        if user_data.exists() {
            fs::remove_dir_all(&user_data)?;
        }
        fs::create_dir_all(user_data)?;
        Ok(())
    }

    pub fn profile_dir(&self, id: &str) -> Result<PathBuf, StoreError> {
        validate_profile_id(id)?;
        Ok(self.profile_dir_unchecked(id))
    }

    pub fn user_data_dir(&self, id: &str) -> Result<PathBuf, StoreError> {
        Ok(self.profile_dir(id)?.join(USER_DATA_DIR))
    }

    fn profile_dir_unchecked(&self, id: &str) -> PathBuf {
        self.workspace.profiles_dir().join(id)
    }

    fn write_profile(&self, profile: &Profile) -> Result<(), StoreError> {
        validate_profile_id(&profile.id)?;
        let dir = self.profile_dir_unchecked(&profile.id);
        fs::create_dir_all(&dir)?;
        let path = dir.join(PROFILE_FILE);
        let temporary = dir.join(format!("{PROFILE_FILE}.tmp"));
        let data = serde_json::to_vec_pretty(profile)?;
        fs::write(&temporary, data)?;
        fs::rename(&temporary, &path)?;
        Ok(())
    }
}

fn privacy_semantics_equal(left: &PrivacyPolicy, right: &PrivacyPolicy) -> bool {
    left.verification_max_age_hours == right.verification_max_age_hours
        && left.preset == right.preset
        && left.network_guard == right.network_guard
        && left.webrtc == right.webrtc
        && left.block_third_party_cookies == right.block_third_party_cookies
        && left.block_notifications == right.block_notifications
        && left.block_geolocation == right.block_geolocation
        && left.block_camera == right.block_camera
        && left.block_microphone == right.block_microphone
}

fn read_profile(path: &Path) -> Result<Profile, StoreError> {
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(StoreError::Json)
}

fn normalize_and_validate_draft(mut draft: ProfileDraft) -> Result<ProfileDraft, StoreError> {
    draft.name = draft.name.trim().to_owned();
    draft.notes = draft.notes.trim().to_owned();
    draft.tags = normalize_tags(draft.tags)?;
    draft.browser.start_url = normalize_start_url(draft.browser.start_url)?;

    if draft.name.is_empty() {
        return Err(StoreError::Validation("profile name cannot be empty".to_owned()));
    }
    if draft.name.chars().count() > MAX_NAME_CHARS {
        return Err(StoreError::Validation(format!(
            "profile name cannot exceed {MAX_NAME_CHARS} characters"
        )));
    }
    if draft.notes.chars().count() > MAX_NOTES_CHARS {
        return Err(StoreError::Validation(format!(
            "profile notes cannot exceed {MAX_NOTES_CHARS} characters"
        )));
    }

    validate_window_size(
        draft.browser.window_width,
        draft.browser.window_height,
    )?;
    draft
        .network
        .validate()
        .map_err(|error| StoreError::Validation(error.to_string()))?;
    draft
        .privacy
        .validate()
        .map_err(|error| StoreError::Validation(error.to_string()))?;
    Ok(draft)
}

fn normalize_start_url(value: Option<String>) -> Result<Option<String>, StoreError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().any(char::is_whitespace)
        || !(value.starts_with("https://") || value.starts_with("http://"))
    {
        return Err(StoreError::Validation(
            "start URL must be an http:// or https:// URL without spaces".to_owned(),
        ));
    }
    Ok(Some(value))
}

fn normalize_tags(tags: Vec<String>) -> Result<Vec<String>, StoreError> {
    if tags.len() > MAX_TAGS {
        return Err(StoreError::Validation(format!(
            "a profile cannot have more than {MAX_TAGS} tags"
        )));
    }

    let mut normalized = Vec::new();
    for tag in tags {
        let tag = tag.trim().to_owned();
        if tag.is_empty() || normalized.contains(&tag) {
            continue;
        }
        if tag.chars().count() > MAX_TAG_CHARS {
            return Err(StoreError::Validation(format!(
                "profile tags cannot exceed {MAX_TAG_CHARS} characters"
            )));
        }
        normalized.push(tag);
    }
    Ok(normalized)
}

fn validate_window_size(width: Option<u32>, height: Option<u32>) -> Result<(), StoreError> {
    if let Some(width) = width {
        if !(640..=7680).contains(&width) {
            return Err(StoreError::Validation(
                "window width must be between 640 and 7680".to_owned(),
            ));
        }
    }
    if let Some(height) = height {
        if !(480..=4320).contains(&height) {
            return Err(StoreError::Validation(
                "window height must be between 480 and 4320".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_profile_id(id: &str) -> Result<(), StoreError> {
    if id.is_empty()
        || id.len() > 96
        || !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        return Err(StoreError::InvalidId(id.to_owned()));
    }
    Ok(())
}

fn generate_profile_id() -> String {
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
pub enum StoreError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Validation(String),
    InvalidId(String),
    NotFound(String),
    Corrupt(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Io(error) => write!(f, "profile storage error: {error}"),
            StoreError::Json(error) => write!(f, "invalid profile JSON: {error}"),
            StoreError::Validation(message) => write!(f, "invalid profile: {message}"),
            StoreError::InvalidId(id) => write!(f, "invalid profile id: {id}"),
            StoreError::NotFound(id) => write!(f, "profile not found: {id}"),
            StoreError::Corrupt(message) => write!(f, "corrupt profile: {message}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(value: std::io::Error) -> Self {
        StoreError::Io(value)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(value: serde_json::Error) -> Self {
        StoreError::Json(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_store(tag: &str) -> ProfileStore {
        let root = env::temp_dir().join(format!(
            "dravyn-profile-{tag}-{}-{}",
            std::process::id(),
            epoch_seconds()
        ));
        let _ = fs::remove_dir_all(&root);
        ProfileStore::new(Workspace::from_root(root))
    }

    fn draft(name: &str) -> ProfileDraft {
        ProfileDraft {
            name: name.to_owned(),
            ..ProfileDraft::default()
        }
    }

    #[test]
    fn create_get_update_and_delete_profile() {
        let store = temp_store("crud");
        let created = store.create(draft("Primary")).unwrap();
        assert!(store.user_data_dir(&created.id).unwrap().is_dir());
        assert_eq!(store.get(&created.id).unwrap().name, "Primary");
        assert_eq!(store.get(&created.id).unwrap().privacy, PrivacyPolicy::default());

        let mut update = draft("Updated");
        update.notes = "note".to_owned();
        let updated = store.update(&created.id, update).unwrap();
        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.notes, "note");
        assert_eq!(updated.privacy.policy_version, 1);

        store.delete(&created.id).unwrap();
        assert!(matches!(
            store.get(&created.id),
            Err(StoreError::NotFound(_))
        ));
        let _ = fs::remove_dir_all(store.workspace().root());
    }

    #[test]
    fn privacy_change_bumps_policy_version() {
        let store = temp_store("privacy-version");
        let created = store.create(draft("Primary")).unwrap();
        let mut update = draft("Primary");
        update.privacy = PrivacyPolicy::strict();
        let updated = store.update(&created.id, update).unwrap();
        assert_eq!(updated.privacy.policy_version, 2);
        let _ = fs::remove_dir_all(store.workspace().root());
    }

    #[test]
    fn list_is_sorted_by_most_recent_update() {
        let store = temp_store("list");
        let first = store.create(draft("First")).unwrap();
        let second = store.create(draft("Second")).unwrap();
        store.update(&first.id, draft("First updated")).unwrap();
        let profiles = store.list().unwrap();
        assert_eq!(profiles.len(), 2);
        assert!(profiles.iter().any(|profile| profile.id == second.id));
        assert_eq!(profiles[0].id, first.id);
        let _ = fs::remove_dir_all(store.workspace().root());
    }

    #[test]
    fn rejects_path_like_ids_and_bad_urls() {
        let store = temp_store("validation");
        assert!(store.get("../escape").is_err());

        let mut bad = draft("Bad URL");
        bad.browser.start_url = Some("javascript:alert(1)".to_owned());
        assert!(matches!(
            store.create(bad),
            Err(StoreError::Validation(_))
        ));
        let _ = fs::remove_dir_all(store.workspace().root());
    }

    #[test]
    fn reset_user_data_keeps_profile_metadata() {
        let store = temp_store("reset");
        let created = store.create(draft("Resettable")).unwrap();
        let user_data = store.user_data_dir(&created.id).unwrap();
        fs::write(user_data.join("marker"), "data").unwrap();

        store.reset_user_data(&created.id).unwrap();
        assert!(!user_data.join("marker").exists());
        assert_eq!(store.get(&created.id).unwrap().name, "Resettable");
        let _ = fs::remove_dir_all(store.workspace().root());
    }
}
