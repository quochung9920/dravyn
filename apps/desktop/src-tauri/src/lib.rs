use dravyn_common::Workspace;
use dravyn_core::{chromium, profile_runtime};
use dravyn_network::NetworkMode;
use dravyn_profile::{Profile, ProfileDraft, ProfileStore};
use serde::Serialize;
use std::fs;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const PRIVACY_AUDIT_HTML: &str = include_str!("../privacy_audit.html");

#[derive(Debug, Serialize)]
struct RuntimeView {
    running: bool,
    pid: Option<u32>,
    started_at: Option<u64>,
}

#[derive(Debug, Serialize)]
struct ProfileView {
    profile: Profile,
    runtime: RuntimeView,
}

#[derive(Debug, Serialize)]
struct AppStatus {
    chromium_ready: bool,
    chromium_state: String,
    browser_binary: String,
    workspace: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct NetworkProbe {
    mode: String,
    endpoint: Option<String>,
    valid: bool,
    reachable: Option<bool>,
    latency_ms: Option<u64>,
    message: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticItem {
    id: String,
    label: String,
    status: String,
    detail: String,
}

fn context() -> Result<(Workspace, ProfileStore), String> {
    let workspace = Workspace::from_env().map_err(|error| error.to_string())?;
    let store = ProfileStore::new(workspace.clone());
    Ok((workspace, store))
}

fn view(
    workspace: &Workspace,
    store: &ProfileStore,
    profile: Profile,
) -> Result<ProfileView, String> {
    let runtime = profile_runtime::status(workspace, store, &profile)
        .map_err(|error| error.to_string())?;
    Ok(ProfileView {
        profile,
        runtime: RuntimeView {
            running: runtime.running,
            pid: runtime.pid,
            started_at: runtime.started_at,
        },
    })
}

#[tauri::command]
fn app_status() -> Result<AppStatus, String> {
    let workspace = Workspace::from_env().map_err(|error| error.to_string())?;
    let detected = chromium::detect(&workspace);
    Ok(AppStatus {
        chromium_ready: detected.build_available,
        chromium_state: detected.state.label().to_owned(),
        browser_binary: detected.browser_binary.display().to_string(),
        workspace: workspace.root().display().to_string(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    })
}

#[tauri::command]
fn list_profiles() -> Result<Vec<ProfileView>, String> {
    let (workspace, store) = context()?;
    store
        .list()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|profile| view(&workspace, &store, profile))
        .collect()
}

#[tauri::command]
fn create_profile(draft: ProfileDraft) -> Result<ProfileView, String> {
    let (workspace, store) = context()?;
    let profile = store.create(draft).map_err(|error| error.to_string())?;
    view(&workspace, &store, profile)
}

#[tauri::command]
fn update_profile(id: String, draft: ProfileDraft) -> Result<ProfileView, String> {
    let (workspace, store) = context()?;
    let profile = store
        .update(&id, draft)
        .map_err(|error| error.to_string())?;
    view(&workspace, &store, profile)
}

#[tauri::command]
fn launch_profile(id: String) -> Result<ProfileView, String> {
    let (workspace, store) = context()?;
    let profile = store.get(&id).map_err(|error| error.to_string())?;
    profile_runtime::launch(&workspace, &store, &profile)
        .map_err(|error| error.to_string())?;
    view(&workspace, &store, profile)
}

#[tauri::command]
fn stop_profile(id: String) -> Result<ProfileView, String> {
    let (workspace, store) = context()?;
    let profile = store.get(&id).map_err(|error| error.to_string())?;
    profile_runtime::stop(&workspace, &store, &profile)
        .map_err(|error| error.to_string())?;
    view(&workspace, &store, profile)
}

#[tauri::command]
fn reset_profile(id: String) -> Result<ProfileView, String> {
    let (workspace, store) = context()?;
    let profile = store.get(&id).map_err(|error| error.to_string())?;
    let runtime = profile_runtime::status(&workspace, &store, &profile)
        .map_err(|error| error.to_string())?;
    if runtime.running {
        return Err("stop the profile before resetting browser data".to_owned());
    }
    store
        .reset_user_data(&id)
        .map_err(|error| error.to_string())?;
    view(&workspace, &store, profile)
}

#[tauri::command]
fn delete_profile(id: String) -> Result<(), String> {
    let (workspace, store) = context()?;
    let profile = store.get(&id).map_err(|error| error.to_string())?;
    let runtime = profile_runtime::status(&workspace, &store, &profile)
        .map_err(|error| error.to_string())?;
    if runtime.running {
        return Err("stop the profile before deleting it".to_owned());
    }
    store.delete(&id).map_err(|error| error.to_string())
}

#[tauri::command]
fn network_probe(id: String) -> Result<NetworkProbe, String> {
    let (_, store) = context()?;
    let profile = store.get(&id).map_err(|error| error.to_string())?;

    if let Err(error) = profile.network.validate() {
        return Ok(NetworkProbe {
            mode: "invalid".to_owned(),
            endpoint: None,
            valid: false,
            reachable: None,
            latency_ms: None,
            message: error.to_string(),
        });
    }

    match profile.network.mode {
        NetworkMode::Direct => Ok(NetworkProbe {
            mode: "direct".to_owned(),
            endpoint: None,
            valid: true,
            reachable: None,
            latency_ms: None,
            message: "Direct connection is configured. No proxy endpoint is used.".to_owned(),
        }),
        NetworkMode::Proxy => {
            let proxy = profile
                .network
                .proxy
                .as_ref()
                .ok_or_else(|| "proxy settings are missing".to_owned())?;
            let endpoint = format!("{}://{}:{}", proxy.scheme.as_str(), proxy.host, proxy.port);
            let addresses = (proxy.host.as_str(), proxy.port)
                .to_socket_addrs()
                .map_err(|error| format!("failed to resolve proxy host: {error}"))?
                .take(8)
                .collect::<Vec<_>>();

            if addresses.is_empty() {
                return Ok(NetworkProbe {
                    mode: "proxy".to_owned(),
                    endpoint: Some(endpoint),
                    valid: true,
                    reachable: Some(false),
                    latency_ms: None,
                    message: "Proxy host resolved to no usable address.".to_owned(),
                });
            }

            let started = Instant::now();
            let timeout = Duration::from_millis(1_500);
            let reachable = addresses
                .iter()
                .any(|address| TcpStream::connect_timeout(address, timeout).is_ok());
            let elapsed = started.elapsed().as_millis().min(u64::MAX as u128) as u64;

            Ok(NetworkProbe {
                mode: "proxy".to_owned(),
                endpoint: Some(endpoint),
                valid: true,
                reachable: Some(reachable),
                latency_ms: Some(elapsed),
                message: if reachable {
                    "Proxy endpoint accepted a TCP connection. This checks reachability only; it does not validate credentials or anonymity.".to_owned()
                } else {
                    "Proxy endpoint could not be reached within the local timeout.".to_owned()
                },
            })
        }
    }
}

#[tauri::command]
fn system_diagnostics() -> Result<Vec<DiagnosticItem>, String> {
    let (workspace, store) = context()?;
    let detected = chromium::detect(&workspace);
    let mut items = Vec::new();

    items.push(DiagnosticItem {
        id: "chromium".to_owned(),
        label: "Dravyn Chromium".to_owned(),
        status: if detected.build_available { "ok" } else { "error" }.to_owned(),
        detail: if detected.build_available {
            format!("Ready at {}", detected.browser_binary.display())
        } else {
            format!(
                "Browser binary is not available at {}",
                detected.browser_binary.display()
            )
        },
    });

    let display_ready = std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var_os("DISPLAY").is_some();
    items.push(DiagnosticItem {
        id: "display".to_owned(),
        label: "Desktop display".to_owned(),
        status: if display_ready { "ok" } else { "error" }.to_owned(),
        detail: if display_ready {
            "WSLg/desktop display variables are available.".to_owned()
        } else {
            "No WAYLAND_DISPLAY or DISPLAY variable is available.".to_owned()
        },
    });

    let profiles_ready = store.ensure_layout().is_ok();
    items.push(DiagnosticItem {
        id: "profiles".to_owned(),
        label: "Profile storage".to_owned(),
        status: if profiles_ready { "ok" } else { "error" }.to_owned(),
        detail: workspace.profiles_dir().display().to_string(),
    });

    let runtime_ready = fs::create_dir_all(workspace.runtime_dir()).is_ok();
    items.push(DiagnosticItem {
        id: "runtime".to_owned(),
        label: "Runtime workspace".to_owned(),
        status: if runtime_ready { "ok" } else { "error" }.to_owned(),
        detail: workspace.runtime_dir().display().to_string(),
    });

    let audit_ready = ensure_privacy_audit_page(&workspace).is_ok();
    items.push(DiagnosticItem {
        id: "privacy-audit".to_owned(),
        label: "Local privacy audit".to_owned(),
        status: if audit_ready { "ok" } else { "warning" }.to_owned(),
        detail: "Runs entirely inside the selected Dravyn profile and does not transmit fingerprint data.".to_owned(),
    });

    Ok(items)
}

#[tauri::command]
fn open_privacy_audit(id: String) -> Result<ProfileView, String> {
    let (workspace, store) = context()?;
    let profile = store.get(&id).map_err(|error| error.to_string())?;
    let audit_path = ensure_privacy_audit_page(&workspace).map_err(|error| error.to_string())?;
    let audit_url = file_url(&audit_path);
    let runtime = profile_runtime::status(&workspace, &store, &profile)
        .map_err(|error| error.to_string())?;

    if runtime.running {
        let user_data = store
            .user_data_dir(&profile.id)
            .map_err(|error| error.to_string())?;
        Command::new(workspace.chrome_binary())
            .arg(format!("--user-data-dir={}", user_data.display()))
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg(audit_url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("failed to open privacy audit: {error}"))?;
    } else {
        let mut audit_profile = profile.clone();
        audit_profile.browser.start_url = Some(audit_url);
        profile_runtime::launch(&workspace, &store, &audit_profile)
            .map_err(|error| error.to_string())?;
    }

    view(&workspace, &store, profile)
}

fn file_url(path: &Path) -> String {
    let mut encoded = path.to_string_lossy().replace('%', "%25");
    encoded = encoded.replace(' ', "%20").replace('#', "%23");
    if !encoded.starts_with('/') {
        encoded.insert(0, '/');
    }
    format!("file://{encoded}")
}

fn ensure_privacy_audit_page(workspace: &Workspace) -> std::io::Result<PathBuf> {
    let directory = workspace.runtime_dir().join("privacy-audit");
    fs::create_dir_all(&directory)?;
    let path = directory.join("index.html");
    let should_write = fs::read_to_string(&path)
        .map(|current| current != PRIVACY_AUDIT_HTML)
        .unwrap_or(true);
    if should_write {
        fs::write(&path, PRIVACY_AUDIT_HTML)?;
    }
    Ok(path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_status,
            list_profiles,
            create_profile,
            update_profile,
            launch_profile,
            stop_profile,
            reset_profile,
            delete_profile,
            network_probe,
            system_diagnostics,
            open_privacy_audit
        ])
        .run(tauri::generate_context!())
        .expect("error while running Dravyn Desktop");
}
