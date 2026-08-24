use crate::chromium;
use dravyn_common::Workspace;
use dravyn_network::{NetworkMode, probe_network};
use dravyn_privacy::{NetworkGuardMode, apply_to_user_data};
use dravyn_profile::{Profile, ProfileStore};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TERMINATION_WAIT: Duration = Duration::from_millis(50);
const TERMINATION_ATTEMPTS: usize = 40;
const KILL_CONFIRM_ATTEMPTS: usize = 20;
const NETWORK_PREFLIGHT_TIMEOUT: Duration = Duration::from_millis(1_500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub started_at: Option<u64>,
}

impl RuntimeStatus {
    pub fn stopped() -> Self {
        Self {
            running: false,
            pid: None,
            started_at: None,
        }
    }
}

pub fn status(
    workspace: &Workspace,
    store: &ProfileStore,
    profile: &Profile,
) -> Result<RuntimeStatus, RuntimeError> {
    let pid_path = pid_file(workspace, &profile.id);
    if !pid_path.is_file() {
        return Ok(RuntimeStatus::stopped());
    }

    let record = read_pid_record(&pid_path)?;
    let user_data = store
        .user_data_dir(&profile.id)
        .map_err(|error| RuntimeError::Profile(error.to_string()))?;
    if process_matches_profile(record.pid, &workspace.chrome_binary(), &user_data) {
        return Ok(RuntimeStatus {
            running: true,
            pid: Some(record.pid),
            started_at: Some(record.started_at),
        });
    }

    let _ = fs::remove_file(pid_path);
    Ok(RuntimeStatus::stopped())
}

pub fn launch(
    workspace: &Workspace,
    store: &ProfileStore,
    profile: &Profile,
) -> Result<RuntimeStatus, RuntimeError> {
    let existing = status(workspace, store, profile)?;
    if existing.running {
        return Ok(existing);
    }

    let detection = chromium::detect(workspace);
    if !detection.build_available {
        return Err(RuntimeError::BrowserNotBuilt(detection.browser_binary));
    }
    if std::env::var_os("WAYLAND_DISPLAY").is_none() && std::env::var_os("DISPLAY").is_none() {
        return Err(RuntimeError::NoDisplay);
    }

    profile
        .network
        .validate()
        .map_err(|error| RuntimeError::Profile(error.to_string()))?;
    profile
        .privacy
        .validate()
        .map_err(|error| RuntimeError::PrivacyPolicy(error.to_string()))?;

    let strict_proxy = profile.privacy.network_guard == NetworkGuardMode::Strict
        && profile.network.mode == NetworkMode::Proxy;
    if strict_proxy {
        let probe = probe_network(&profile.network, NETWORK_PREFLIGHT_TIMEOUT);
        if probe.reachable != Some(true) {
            return Err(RuntimeError::NetworkGuard(format!(
                "strict network guard blocked launch: {}",
                probe.message
            )));
        }
    }

    let user_data = store
        .user_data_dir(&profile.id)
        .map_err(|error| RuntimeError::Profile(error.to_string()))?;
    fs::create_dir_all(&user_data)?;
    fs::create_dir_all(workspace.profile_runtime_dir())?;

    let applied = apply_to_user_data(&user_data, &profile.privacy)
        .map_err(|error| RuntimeError::PrivacyPolicy(error.to_string()))?;
    if !applied.applied {
        return Err(RuntimeError::PrivacyPolicy(format!(
            "privacy policy verification failed before launch: {}",
            applied.message
        )));
    }

    let mut command = Command::new(&detection.browser_binary);
    command
        .arg(format!("--user-data-dir={}", user_data.display()))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--ozone-platform-hint=auto")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Some(proxy_arg) = profile
        .network
        .chromium_argument()
        .map_err(|error| RuntimeError::Profile(error.to_string()))?
    {
        command.arg(proxy_arg);
    }

    // QUIC is UDP-based and is not required for Dravyn's strict manual-proxy
    // profile mode. Disabling it narrows the runtime's network transport surface
    // while the continuous Network Shield watches the configured proxy endpoint.
    // This is defensive routing hardening, not proof that remote IP/DNS exposure
    // matches policy; external verification remains a separate evidence layer.
    if strict_proxy {
        command.arg("--disable-quic");
    }

    for argument in profile.privacy.chromium_arguments() {
        command.arg(argument);
    }

    if let (Some(width), Some(height)) = (
        profile.browser.window_width,
        profile.browser.window_height,
    ) {
        command.arg(format!("--window-size={width},{height}"));
    }

    if let Some(url) = &profile.browser.start_url {
        command.arg(url);
    }

    let child = command.spawn()?;
    let record = PidRecord {
        pid: child.id(),
        started_at: epoch_seconds(),
    };
    write_pid_record(&pid_file(workspace, &profile.id), record)?;

    Ok(RuntimeStatus {
        running: true,
        pid: Some(record.pid),
        started_at: Some(record.started_at),
    })
}

pub fn stop(
    workspace: &Workspace,
    store: &ProfileStore,
    profile: &Profile,
) -> Result<RuntimeStatus, RuntimeError> {
    let current = status(workspace, store, profile)?;
    let Some(pid) = current.pid else {
        return Ok(RuntimeStatus::stopped());
    };

    send_signal(pid, "-TERM")?;
    for _ in 0..TERMINATION_ATTEMPTS {
        if !process_exists(pid) {
            let _ = fs::remove_file(pid_file(workspace, &profile.id));
            return Ok(RuntimeStatus::stopped());
        }
        thread::sleep(TERMINATION_WAIT);
    }

    send_signal(pid, "-KILL")?;
    for _ in 0..KILL_CONFIRM_ATTEMPTS {
        if !process_exists(pid) {
            let _ = fs::remove_file(pid_file(workspace, &profile.id));
            return Ok(RuntimeStatus::stopped());
        }
        thread::sleep(TERMINATION_WAIT);
    }

    Err(RuntimeError::TerminationTimeout(pid))
}

fn send_signal(pid: u32, signal: &str) -> Result<(), RuntimeError> {
    #[cfg(unix)]
    {
        let result = Command::new("kill")
            .arg(signal)
            .arg(pid.to_string())
            .status()?;
        if result.success() || !process_exists(pid) {
            Ok(())
        } else {
            Err(RuntimeError::SignalFailed {
                pid,
                signal: signal.to_owned(),
            })
        }
    }

    #[cfg(not(unix))]
    {
        let _ = (pid, signal);
        Err(RuntimeError::UnsupportedPlatform)
    }
}

fn process_exists(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        Path::new("/proc").join(pid.to_string()).exists()
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        false
    }
}

fn process_matches_profile(pid: u32, browser_binary: &Path, user_data: &Path) -> bool {
    #[cfg(target_os = "linux")]
    {
        let Ok(cmdline) = fs::read(format!("/proc/{pid}/cmdline")) else {
            return false;
        };
        let text = String::from_utf8_lossy(&cmdline).replace('\0', " ");
        let browser = browser_binary.to_string_lossy();
        let user_data_arg = format!("--user-data-dir={}", user_data.display());
        text.contains(browser.as_ref()) && text.contains(&user_data_arg)
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (pid, browser_binary, user_data);
        false
    }
}

#[derive(Debug, Clone, Copy)]
struct PidRecord {
    pid: u32,
    started_at: u64,
}

fn pid_file(workspace: &Workspace, profile_id: &str) -> PathBuf {
    workspace
        .profile_runtime_dir()
        .join(format!("{profile_id}.pid"))
}

fn read_pid_record(path: &Path) -> Result<PidRecord, RuntimeError> {
    let content = fs::read_to_string(path)?;
    let mut lines = content.lines();
    let pid = lines
        .next()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .ok_or_else(|| RuntimeError::InvalidPidFile(path.to_path_buf()))?;
    let started_at = lines
        .next()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .ok_or_else(|| RuntimeError::InvalidPidFile(path.to_path_buf()))?;
    Ok(PidRecord { pid, started_at })
}

fn write_pid_record(path: &Path, record: PidRecord) -> Result<(), RuntimeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("pid.tmp");
    fs::write(&temporary, format!("{}\n{}\n", record.pid, record.started_at))?;
    fs::rename(&temporary, path)?;
    Ok(())
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug)]
pub enum RuntimeError {
    Io(std::io::Error),
    BrowserNotBuilt(PathBuf),
    NoDisplay,
    Profile(String),
    PrivacyPolicy(String),
    NetworkGuard(String),
    InvalidPidFile(PathBuf),
    SignalFailed { pid: u32, signal: String },
    TerminationTimeout(u32),
    UnsupportedPlatform,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Io(error) => write!(f, "runtime I/O error: {error}"),
            RuntimeError::BrowserNotBuilt(path) => write!(
                f,
                "Dravyn Chromium is not built at {}. Run `dravyn chromium build` first.",
                path.display()
            ),
            RuntimeError::NoDisplay => write!(
                f,
                "no GUI display detected; run Dravyn from a WSLg/desktop session"
            ),
            RuntimeError::Profile(message) => write!(f, "profile runtime error: {message}"),
            RuntimeError::PrivacyPolicy(message) => {
                write!(f, "privacy policy preflight failed: {message}")
            }
            RuntimeError::NetworkGuard(message) => write!(f, "network guard: {message}"),
            RuntimeError::InvalidPidFile(path) => {
                write!(f, "invalid runtime pid file: {}", path.display())
            }
            RuntimeError::SignalFailed { pid, signal } => {
                write!(f, "failed to send {signal} to browser process {pid}")
            }
            RuntimeError::TerminationTimeout(pid) => {
                write!(f, "browser process {pid} did not exit after TERM/KILL timeout")
            }
            RuntimeError::UnsupportedPlatform => {
                write!(f, "profile process control is currently supported on Linux/WSLg")
            }
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<std::io::Error> for RuntimeError {
    fn from(value: std::io::Error) -> Self {
        RuntimeError::Io(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dravyn_profile::ProfileDraft;
    use std::env;

    #[test]
    fn stale_pid_file_is_cleaned_up() {
        let root = env::temp_dir().join(format!(
            "dravyn-runtime-stale-{}-{}",
            std::process::id(),
            epoch_seconds()
        ));
        let _ = fs::remove_dir_all(&root);
        let workspace = Workspace::from_root(root.clone());
        let store = ProfileStore::new(workspace.clone());
        let profile = store
            .create(ProfileDraft {
                name: "Runtime test".to_owned(),
                ..ProfileDraft::default()
            })
            .unwrap();

        fs::create_dir_all(workspace.profile_runtime_dir()).unwrap();
        let path = pid_file(&workspace, &profile.id);
        fs::write(&path, format!("{}\n1\n", u32::MAX)).unwrap();

        assert_eq!(
            status(&workspace, &store, &profile).unwrap(),
            RuntimeStatus::stopped()
        );
        assert!(!path.exists());
        let _ = fs::remove_dir_all(root);
    }
}
