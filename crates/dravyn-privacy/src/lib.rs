use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyPreset {
    Standard,
    #[default]
    Balanced,
    Strict,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NetworkGuardMode {
    Off,
    #[default]
    Monitor,
    Strict,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WebRtcPolicy {
    #[default]
    Default,
    ProxiedOnly,
}

impl WebRtcPolicy {
    pub fn chromium_value(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::ProxiedOnly => "disable_non_proxied_udp",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct PrivacyPolicy {
    pub preset: PrivacyPreset,
    pub network_guard: NetworkGuardMode,
    pub webrtc: WebRtcPolicy,
    pub block_third_party_cookies: bool,
    pub block_notifications: bool,
    pub block_geolocation: bool,
    pub block_camera: bool,
    pub block_microphone: bool,
}

impl Default for PrivacyPolicy {
    fn default() -> Self {
        Self {
            preset: PrivacyPreset::Balanced,
            network_guard: NetworkGuardMode::Monitor,
            webrtc: WebRtcPolicy::Default,
            block_third_party_cookies: true,
            block_notifications: true,
            block_geolocation: false,
            block_camera: false,
            block_microphone: false,
        }
    }
}

impl PrivacyPolicy {
    pub fn standard() -> Self {
        Self {
            preset: PrivacyPreset::Standard,
            network_guard: NetworkGuardMode::Off,
            webrtc: WebRtcPolicy::Default,
            block_third_party_cookies: false,
            block_notifications: false,
            block_geolocation: false,
            block_camera: false,
            block_microphone: false,
        }
    }

    pub fn strict() -> Self {
        Self {
            preset: PrivacyPreset::Strict,
            network_guard: NetworkGuardMode::Strict,
            webrtc: WebRtcPolicy::ProxiedOnly,
            block_third_party_cookies: true,
            block_notifications: true,
            block_geolocation: true,
            block_camera: true,
            block_microphone: true,
        }
    }

    pub fn validate(&self) -> Result<(), PrivacyError> {
        if self.preset == PrivacyPreset::Strict && self.network_guard == NetworkGuardMode::Off {
            return Err(PrivacyError::Validation(
                "strict privacy preset cannot disable the network guard".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn chromium_arguments(&self) -> Vec<String> {
        match self.webrtc {
            WebRtcPolicy::Default => Vec::new(),
            WebRtcPolicy::ProxiedOnly => vec![
                "--force-webrtc-ip-handling-policy=disable_non_proxied_udp".to_owned(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivacyAppliedStatus {
    pub preferences_path: String,
    pub preferences_present: bool,
    pub applied: bool,
    pub expected_webrtc_policy: String,
    pub actual_webrtc_policy: Option<String>,
    pub third_party_cookies_blocked: bool,
    pub blocked_permission_count: usize,
    pub message: String,
}

pub fn apply_to_user_data(
    user_data_dir: &Path,
    policy: &PrivacyPolicy,
) -> Result<PrivacyAppliedStatus, PrivacyError> {
    policy.validate()?;
    let preferences_path = user_data_dir.join("Default").join("Preferences");
    if let Some(parent) = preferences_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut root = if preferences_path.is_file() {
        let bytes = fs::read(&preferences_path)?;
        serde_json::from_slice::<Value>(&bytes)?
    } else {
        Value::Object(Map::new())
    };
    if !root.is_object() {
        return Err(PrivacyError::Validation(
            "Chromium Preferences root must be a JSON object".to_owned(),
        ));
    }

    set_path(
        &mut root,
        &["profile", "block_third_party_cookies"],
        Value::Bool(policy.block_third_party_cookies),
    );
    set_path(
        &mut root,
        &["webrtc", "ip_handling_policy"],
        Value::String(policy.webrtc.chromium_value().to_owned()),
    );
    set_path(
        &mut root,
        &["webrtc", "multiple_routes_enabled"],
        Value::Bool(policy.webrtc == WebRtcPolicy::Default),
    );
    set_path(
        &mut root,
        &["webrtc", "nonproxied_udp_enabled"],
        Value::Bool(policy.webrtc == WebRtcPolicy::Default),
    );

    set_permission(&mut root, "notifications", policy.block_notifications);
    set_permission(&mut root, "geolocation", policy.block_geolocation);
    set_permission(&mut root, "media_stream_camera", policy.block_camera);
    set_permission(&mut root, "media_stream_mic", policy.block_microphone);

    let temporary = preferences_path.with_extension("dravyn.tmp");
    let bytes = serde_json::to_vec_pretty(&root)?;
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, &preferences_path)?;

    inspect_user_data(user_data_dir, policy)
}

pub fn inspect_user_data(
    user_data_dir: &Path,
    policy: &PrivacyPolicy,
) -> Result<PrivacyAppliedStatus, PrivacyError> {
    policy.validate()?;
    let preferences_path = user_data_dir.join("Default").join("Preferences");
    if !preferences_path.is_file() {
        return Ok(PrivacyAppliedStatus {
            preferences_path: preferences_path.display().to_string(),
            preferences_present: false,
            applied: false,
            expected_webrtc_policy: policy.webrtc.chromium_value().to_owned(),
            actual_webrtc_policy: None,
            third_party_cookies_blocked: false,
            blocked_permission_count: 0,
            message: "Privacy preferences have not been applied yet. They are applied before the next browser launch.".to_owned(),
        });
    }

    let bytes = fs::read(&preferences_path)?;
    let root: Value = serde_json::from_slice(&bytes)?;
    let actual_webrtc_policy = get_path(&root, &["webrtc", "ip_handling_policy"])
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let third_party_cookies_blocked = get_path(
        &root,
        &["profile", "block_third_party_cookies"],
    )
    .and_then(Value::as_bool)
    .unwrap_or(false);

    let blocked_permissions = [
        ("notifications", policy.block_notifications),
        ("geolocation", policy.block_geolocation),
        ("media_stream_camera", policy.block_camera),
        ("media_stream_mic", policy.block_microphone),
    ];
    let blocked_permission_count = blocked_permissions
        .iter()
        .filter(|entry| {
            let name = entry.0;
            let expected_blocked = entry.1;
            expected_blocked
                && get_path(
                    &root,
                    &["profile", "default_content_setting_values", name],
                )
                .and_then(Value::as_i64)
                    == Some(2)
        })
        .count();

    let expected_blocked_count = blocked_permissions.iter().filter(|entry| entry.1).count();
    let webrtc_matches = actual_webrtc_policy.as_deref() == Some(policy.webrtc.chromium_value());
    let cookies_match = third_party_cookies_blocked == policy.block_third_party_cookies;
    let permissions_match = blocked_permission_count == expected_blocked_count;
    let applied = webrtc_matches && cookies_match && permissions_match;

    Ok(PrivacyAppliedStatus {
        preferences_path: preferences_path.display().to_string(),
        preferences_present: true,
        applied,
        expected_webrtc_policy: policy.webrtc.chromium_value().to_owned(),
        actual_webrtc_policy,
        third_party_cookies_blocked,
        blocked_permission_count,
        message: if applied {
            "Stored Chromium preferences match the selected Dravyn privacy policy.".to_owned()
        } else {
            "Stored Chromium preferences do not fully match the selected privacy policy. Stop and relaunch the profile to re-apply it.".to_owned()
        },
    })
}

fn set_permission(root: &mut Value, name: &str, blocked: bool) {
    set_path(
        root,
        &["profile", "default_content_setting_values", name],
        Value::from(if blocked { 2 } else { 0 }),
    );
}

fn set_path(root: &mut Value, path: &[&str], value: Value) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };
    let mut current = root;
    for segment in parents {
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }
        let object = current.as_object_mut().expect("object was just created");
        current = object
            .entry((*segment).to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
    }
    if !current.is_object() {
        *current = Value::Object(Map::new());
    }
    current
        .as_object_mut()
        .expect("object was just created")
        .insert((*last).to_owned(), value);
}

fn get_path<'a>(root: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = root;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

#[derive(Debug)]
pub enum PrivacyError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Validation(String),
}

impl fmt::Display for PrivacyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "privacy policy I/O error: {error}"),
            Self::Json(error) => write!(f, "privacy policy JSON error: {error}"),
            Self::Validation(message) => write!(f, "invalid privacy policy: {message}"),
        }
    }
}

impl std::error::Error for PrivacyError {}

impl From<std::io::Error> for PrivacyError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for PrivacyError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        env::temp_dir().join(format!("dravyn-privacy-{tag}-{}-{nanos}", std::process::id()))
    }

    #[test]
    fn strict_policy_applies_and_verifies() {
        let root = temp_dir("strict");
        let status = apply_to_user_data(&root, &PrivacyPolicy::strict()).unwrap();
        assert!(status.applied);
        assert_eq!(
            status.actual_webrtc_policy.as_deref(),
            Some("disable_non_proxied_udp")
        );
        assert!(status.third_party_cookies_blocked);
        assert_eq!(status.blocked_permission_count, 4);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_preferences_report_not_applied() {
        let root = temp_dir("missing");
        let status = inspect_user_data(&root, &PrivacyPolicy::default()).unwrap();
        assert!(!status.preferences_present);
        assert!(!status.applied);
    }
}
