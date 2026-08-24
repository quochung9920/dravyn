use dravyn_common::Workspace;
use dravyn_core::{chromium, profile_runtime};
use dravyn_fingerprint::{
    AuditSubmission, FingerprintHistoryEntry, FingerprintSnapshot, FingerprintStore,
    FingerprintSummary,
};
use dravyn_network::{NetworkMode, NetworkProbeResult, probe_network};
use dravyn_privacy::{NetworkGuardMode, PrivacyAppliedStatus, inspect_user_data};
use dravyn_profile::{Profile, ProfileDraft, ProfileStore};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const PRIVACY_AUDIT_HTML: &str = include_str!("../privacy_audit.html");
const MAX_HTTP_REQUEST_BYTES: usize = 256 * 1024;
const NETWORK_PROBE_TIMEOUT: Duration = Duration::from_millis(1_500);

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
    fingerprint: FingerprintSummary,
}

#[derive(Debug, Serialize)]
struct AppStatus {
    chromium_ready: bool,
    chromium_state: String,
    browser_binary: String,
    workspace: String,
    version: String,
    fingerprint_capture_origin: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticItem {
    id: String,
    label: String,
    status: String,
    detail: String,
}

#[derive(Debug, Serialize)]
struct PrivacyStatusView {
    profile_id: String,
    preset: String,
    network_guard: String,
    webrtc_policy: String,
    policy_applied: PrivacyAppliedStatus,
    network_probe: NetworkProbeResult,
    overall_status: String,
    external_verification_required: bool,
    message: String,
}

#[derive(Debug)]
struct AuditServer {
    address: SocketAddr,
    token: String,
}

impl AuditServer {
    fn start(workspace: Workspace) -> Result<Self, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|error| format!("failed to bind local fingerprint capture server: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("failed to inspect local capture address: {error}"))?;
        let token = capture_token();
        let thread_token = token.clone();

        thread::Builder::new()
            .name("dravyn-fingerprint-capture".to_owned())
            .spawn(move || {
                for connection in listener.incoming() {
                    match connection {
                        Ok(stream) => {
                            if let Err(error) =
                                handle_audit_connection(stream, &workspace, &thread_token)
                            {
                                eprintln!("[dravyn] fingerprint capture request failed: {error}");
                            }
                        }
                        Err(error) => {
                            eprintln!("[dravyn] fingerprint capture listener error: {error}");
                            break;
                        }
                    }
                }
            })
            .map_err(|error| format!("failed to start fingerprint capture thread: {error}"))?;

        Ok(Self { address, token })
    }

    fn origin(&self) -> String {
        format!("http://{}", self.address)
    }

    fn audit_url(&self, profile_id: &str) -> String {
        format!(
            "{}/audit/{}?token={}",
            self.origin(),
            profile_id,
            self.token
        )
    }
}

fn context() -> Result<(Workspace, ProfileStore, FingerprintStore), String> {
    let workspace = Workspace::from_env().map_err(|error| error.to_string())?;
    let profiles = ProfileStore::new(workspace.clone());
    let fingerprints = FingerprintStore::new(workspace.clone());
    Ok((workspace, profiles, fingerprints))
}

fn view(
    workspace: &Workspace,
    profiles: &ProfileStore,
    fingerprints: &FingerprintStore,
    profile: Profile,
) -> Result<ProfileView, String> {
    let runtime = profile_runtime::status(workspace, profiles, &profile)
        .map_err(|error| error.to_string())?;
    let fingerprint = fingerprints
        .summary(&profile.id)
        .map_err(|error| error.to_string())?;
    Ok(ProfileView {
        profile,
        runtime: RuntimeView {
            running: runtime.running,
            pid: runtime.pid,
            started_at: runtime.started_at,
        },
        fingerprint,
    })
}

fn enum_label<T: std::fmt::Debug>(value: T) -> String {
    format!("{value:?}").to_lowercase()
}

#[tauri::command]
fn app_status(server: tauri::State<'_, AuditServer>) -> Result<AppStatus, String> {
    let workspace = Workspace::from_env().map_err(|error| error.to_string())?;
    let detected = chromium::detect(&workspace);
    Ok(AppStatus {
        chromium_ready: detected.build_available,
        chromium_state: detected.state.label().to_owned(),
        browser_binary: detected.browser_binary.display().to_string(),
        workspace: workspace.root().display().to_string(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        fingerprint_capture_origin: server.origin(),
    })
}

#[tauri::command]
fn list_profiles() -> Result<Vec<ProfileView>, String> {
    let (workspace, profiles, fingerprints) = context()?;
    profiles
        .list()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|profile| view(&workspace, &profiles, &fingerprints, profile))
        .collect()
}

#[tauri::command]
fn create_profile(draft: ProfileDraft) -> Result<ProfileView, String> {
    let (workspace, profiles, fingerprints) = context()?;
    let profile = profiles.create(draft).map_err(|error| error.to_string())?;
    view(&workspace, &profiles, &fingerprints, profile)
}

#[tauri::command]
fn update_profile(id: String, draft: ProfileDraft) -> Result<ProfileView, String> {
    let (workspace, profiles, fingerprints) = context()?;
    let profile = profiles
        .update(&id, draft)
        .map_err(|error| error.to_string())?;
    view(&workspace, &profiles, &fingerprints, profile)
}

#[tauri::command]
fn launch_profile(id: String) -> Result<ProfileView, String> {
    let (workspace, profiles, fingerprints) = context()?;
    let profile = profiles.get(&id).map_err(|error| error.to_string())?;
    profile_runtime::launch(&workspace, &profiles, &profile)
        .map_err(|error| error.to_string())?;
    view(&workspace, &profiles, &fingerprints, profile)
}

#[tauri::command]
fn stop_profile(id: String) -> Result<ProfileView, String> {
    let (workspace, profiles, fingerprints) = context()?;
    let profile = profiles.get(&id).map_err(|error| error.to_string())?;
    profile_runtime::stop(&workspace, &profiles, &profile)
        .map_err(|error| error.to_string())?;
    view(&workspace, &profiles, &fingerprints, profile)
}

#[tauri::command]
fn reset_profile(id: String) -> Result<ProfileView, String> {
    let (workspace, profiles, fingerprints) = context()?;
    let profile = profiles.get(&id).map_err(|error| error.to_string())?;
    let runtime = profile_runtime::status(&workspace, &profiles, &profile)
        .map_err(|error| error.to_string())?;
    if runtime.running {
        return Err("stop the profile before resetting browser data".to_owned());
    }
    profiles
        .reset_user_data(&id)
        .map_err(|error| error.to_string())?;
    view(&workspace, &profiles, &fingerprints, profile)
}

#[tauri::command]
fn delete_profile(id: String) -> Result<(), String> {
    let (workspace, profiles, fingerprints) = context()?;
    let profile = profiles.get(&id).map_err(|error| error.to_string())?;
    let runtime = profile_runtime::status(&workspace, &profiles, &profile)
        .map_err(|error| error.to_string())?;
    if runtime.running {
        return Err("stop the profile before deleting it".to_owned());
    }
    profiles.delete(&id).map_err(|error| error.to_string())?;
    fingerprints
        .clear_profile(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn fingerprint_history(id: String) -> Result<Vec<FingerprintHistoryEntry>, String> {
    let (_, profiles, fingerprints) = context()?;
    profiles.get(&id).map_err(|error| error.to_string())?;
    fingerprints
        .history(&id, 20)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn fingerprint_latest(id: String) -> Result<Option<FingerprintSnapshot>, String> {
    let (_, profiles, fingerprints) = context()?;
    profiles.get(&id).map_err(|error| error.to_string())?;
    fingerprints.latest(&id).map_err(|error| error.to_string())
}

#[tauri::command]
fn set_fingerprint_baseline(id: String) -> Result<ProfileView, String> {
    let (workspace, profiles, fingerprints) = context()?;
    let profile = profiles.get(&id).map_err(|error| error.to_string())?;
    fingerprints
        .set_baseline_from_latest(&id)
        .map_err(|error| error.to_string())?;
    view(&workspace, &profiles, &fingerprints, profile)
}

#[tauri::command]
fn network_probe(id: String) -> Result<NetworkProbeResult, String> {
    let (_, profiles, _) = context()?;
    let profile = profiles.get(&id).map_err(|error| error.to_string())?;
    Ok(probe_network(&profile.network, NETWORK_PROBE_TIMEOUT))
}

#[tauri::command]
fn privacy_status(id: String) -> Result<PrivacyStatusView, String> {
    let (_, profiles, _) = context()?;
    let profile = profiles.get(&id).map_err(|error| error.to_string())?;
    let user_data = profiles
        .user_data_dir(&profile.id)
        .map_err(|error| error.to_string())?;
    let policy_applied = inspect_user_data(&user_data, &profile.privacy)
        .map_err(|error| error.to_string())?;
    let network_probe = probe_network(&profile.network, NETWORK_PROBE_TIMEOUT);

    let strict_proxy_failure = profile.privacy.network_guard == NetworkGuardMode::Strict
        && profile.network.mode == NetworkMode::Proxy
        && network_probe.reachable != Some(true);
    let (overall_status, message) = if strict_proxy_failure {
        (
            "critical".to_owned(),
            "Strict Network Guard would block launch because the configured proxy endpoint did not pass preflight.".to_owned(),
        )
    } else if !policy_applied.applied {
        (
            "restart_required".to_owned(),
            "Stored Chromium preferences do not yet match this profile's privacy policy. Stop and relaunch the profile to apply the policy before browsing.".to_owned(),
        )
    } else {
        (
            "verify_external".to_owned(),
            "Local privacy policy is applied. Use External Verification in this exact profile to confirm public IP, WebRTC, DNS and IPv6 exposure as seen by real websites.".to_owned(),
        )
    };

    Ok(PrivacyStatusView {
        profile_id: profile.id,
        preset: enum_label(profile.privacy.preset),
        network_guard: enum_label(profile.privacy.network_guard),
        webrtc_policy: enum_label(profile.privacy.webrtc),
        policy_applied,
        network_probe,
        overall_status,
        external_verification_required: true,
        message,
    })
}

#[tauri::command]
fn open_external_verification(id: String, test: String) -> Result<ProfileView, String> {
    let (workspace, profiles, fingerprints) = context()?;
    let profile = profiles.get(&id).map_err(|error| error.to_string())?;
    let url = external_test_url(&test)
        .ok_or_else(|| format!("unsupported external verification test: {test}"))?;
    open_url_in_profile(&workspace, &profiles, &profile, url)?;
    view(&workspace, &profiles, &fingerprints, profile)
}

fn external_test_url(test: &str) -> Option<&'static str> {
    match test {
        "browserleaks_ip" => Some("https://browserleaks.com/ip"),
        "browserleaks_webrtc" => Some("https://browserleaks.com/webrtc"),
        "browserleaks_dns" => Some("https://browserleaks.com/dns"),
        "browserleaks_canvas" => Some("https://browserleaks.com/canvas"),
        "browserleaks_webgl" => Some("https://browserleaks.com/webgl"),
        "eff" => Some("https://coveryourtracks.eff.org/"),
        "amiunique" => Some("https://amiunique.org/fingerprint"),
        _ => None,
    }
}

fn open_url_in_profile(
    workspace: &Workspace,
    profiles: &ProfileStore,
    profile: &Profile,
    url: &str,
) -> Result<(), String> {
    let runtime = profile_runtime::status(workspace, profiles, profile)
        .map_err(|error| error.to_string())?;
    if runtime.running {
        let user_data = profiles
            .user_data_dir(&profile.id)
            .map_err(|error| error.to_string())?;
        Command::new(workspace.chrome_binary())
            .arg(format!("--user-data-dir={}", user_data.display()))
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("failed to open URL in profile: {error}"))?;
    } else {
        let mut launch_profile = profile.clone();
        launch_profile.browser.start_url = Some(url.to_owned());
        profile_runtime::launch(workspace, profiles, &launch_profile)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn system_diagnostics(server: tauri::State<'_, AuditServer>) -> Result<Vec<DiagnosticItem>, String> {
    let (workspace, profiles, fingerprints) = context()?;
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

    let profiles_ready = profiles.ensure_layout().is_ok();
    items.push(DiagnosticItem {
        id: "profiles".to_owned(),
        label: "Profile storage".to_owned(),
        status: if profiles_ready { "ok" } else { "error" }.to_owned(),
        detail: workspace.profiles_dir().display().to_string(),
    });

    let fingerprints_ready = fingerprints.ensure_layout().is_ok();
    items.push(DiagnosticItem {
        id: "fingerprints".to_owned(),
        label: "Per-profile fingerprint store".to_owned(),
        status: if fingerprints_ready { "ok" } else { "error" }.to_owned(),
        detail: workspace.fingerprints_dir().display().to_string(),
    });

    let runtime_ready = fs::create_dir_all(workspace.runtime_dir()).is_ok();
    items.push(DiagnosticItem {
        id: "runtime".to_owned(),
        label: "Runtime workspace".to_owned(),
        status: if runtime_ready { "ok" } else { "error" }.to_owned(),
        detail: workspace.runtime_dir().display().to_string(),
    });

    items.push(DiagnosticItem {
        id: "privacy-preflight".to_owned(),
        label: "Privacy preflight".to_owned(),
        status: "ok".to_owned(),
        detail: "Privacy preferences are applied and verified before profile launch; strict proxy profiles fail closed when endpoint preflight fails.".to_owned(),
    });

    items.push(DiagnosticItem {
        id: "fingerprint-capture".to_owned(),
        label: "Local fingerprint capture".to_owned(),
        status: "ok".to_owned(),
        detail: format!(
            "Listening only on {} with an ephemeral session token. Audit values stay on this device.",
            server.origin()
        ),
    });

    Ok(items)
}

#[tauri::command]
fn open_privacy_audit(
    id: String,
    server: tauri::State<'_, AuditServer>,
) -> Result<ProfileView, String> {
    let (workspace, profiles, fingerprints) = context()?;
    let profile = profiles.get(&id).map_err(|error| error.to_string())?;
    let audit_url = server.audit_url(&profile.id);
    open_url_in_profile(&workspace, &profiles, &profile, &audit_url)?;
    view(&workspace, &profiles, &fingerprints, profile)
}

fn handle_audit_connection(
    mut stream: TcpStream,
    workspace: &Workspace,
    expected_token: &str,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .map_err(|error| error.to_string())?;
    let request = read_http_request(&mut stream)?;
    let (path, query) = split_target(&request.target);
    let token = query.get("token").map(String::as_str).unwrap_or_default();
    if token != expected_token {
        return write_http_response(
            &mut stream,
            "403 Forbidden",
            "text/plain; charset=utf-8",
            b"Forbidden",
        );
    }

    let profiles = ProfileStore::new(workspace.clone());
    let fingerprints = FingerprintStore::new(workspace.clone());

    if request.method == "GET" && path.starts_with("/audit/") {
        let profile_id = path.trim_start_matches("/audit/");
        let profile = match profiles.get(profile_id) {
            Ok(profile) => profile,
            Err(_) => {
                return write_http_response(
                    &mut stream,
                    "404 Not Found",
                    "text/plain; charset=utf-8",
                    b"Profile not found",
                );
            }
        };
        let html = render_audit_page(&profile, expected_token);
        return write_http_response(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            html.as_bytes(),
        );
    }

    if request.method == "POST" && path.starts_with("/capture/") {
        let profile_id = path.trim_start_matches("/capture/");
        if profiles.get(profile_id).is_err() {
            return write_http_response(
                &mut stream,
                "404 Not Found",
                "application/json; charset=utf-8",
                br#"{"error":"profile not found"}"#,
            );
        }
        let submission: AuditSubmission = serde_json::from_slice(&request.body)
            .map_err(|error| format!("invalid fingerprint capture JSON: {error}"))?;
        let snapshot = fingerprints
            .capture(profile_id, submission)
            .map_err(|error| error.to_string())?;
        let body = serde_json::to_vec(&snapshot).map_err(|error| error.to_string())?;
        return write_http_response(
            &mut stream,
            "200 OK",
            "application/json; charset=utf-8",
            &body,
        );
    }

    write_http_response(
        &mut stream,
        "404 Not Found",
        "text/plain; charset=utf-8",
        b"Not found",
    )
}

struct HttpRequest {
    method: String,
    target: String,
    body: Vec<u8>,
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    let header_end;
    loop {
        let read = stream.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            return Err("connection closed before HTTP headers were complete".to_owned());
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > MAX_HTTP_REQUEST_BYTES {
            return Err("fingerprint capture request is too large".to_owned());
        }
        if let Some(position) = find_bytes(&bytes, b"\r\n\r\n") {
            header_end = position + 4;
            break;
        }
    }

    let header_text = String::from_utf8_lossy(&bytes[..header_end]);
    let mut lines = header_text.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "missing HTTP request line".to_owned())?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| "missing HTTP method".to_owned())?
        .to_owned();
    let target = request_parts
        .next()
        .ok_or_else(|| "missing HTTP target".to_owned())?
        .to_owned();
    let mut content_length = 0usize;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| "invalid HTTP content-length".to_owned())?;
            }
        }
    }
    if header_end.saturating_add(content_length) > MAX_HTTP_REQUEST_BYTES {
        return Err("fingerprint capture request body is too large".to_owned());
    }

    while bytes.len() < header_end + content_length {
        let read = stream.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            return Err("connection closed before HTTP body was complete".to_owned());
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > MAX_HTTP_REQUEST_BYTES {
            return Err("fingerprint capture request is too large".to_owned());
        }
    }

    Ok(HttpRequest {
        method,
        target,
        body: bytes[header_end..header_end + content_length].to_vec(),
    })
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn split_target(target: &str) -> (&str, HashMap<String, String>) {
    let (path, raw_query) = target.split_once('?').unwrap_or((target, ""));
    let query = raw_query
        .split('&')
        .filter_map(|item| item.split_once('='))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect();
    (path, query)
}

fn render_audit_page(profile: &Profile, token: &str) -> String {
    PRIVACY_AUDIT_HTML
        .replace("__PROFILE_ID__", &profile.id)
        .replace("__PROFILE_NAME__", &html_escape(&profile.name))
        .replace("__CAPTURE_TOKEN__", token)
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn write_http_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> Result<(), String> {
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(headers.as_bytes())
        .and_then(|_| stream.write_all(body))
        .and_then(|_| stream.flush())
        .map_err(|error| error.to_string())
}

fn capture_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{nanos:032x}-{:08x}", std::process::id())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let workspace =
        Workspace::from_env().expect("Dravyn workspace must resolve before desktop startup");
    let audit_server = AuditServer::start(workspace)
        .expect("local per-profile fingerprint capture server must start");

    tauri::Builder::default()
        .manage(audit_server)
        .invoke_handler(tauri::generate_handler![
            app_status,
            list_profiles,
            create_profile,
            update_profile,
            launch_profile,
            stop_profile,
            reset_profile,
            delete_profile,
            fingerprint_history,
            fingerprint_latest,
            set_fingerprint_baseline,
            network_probe,
            privacy_status,
            open_external_verification,
            system_diagnostics,
            open_privacy_audit
        ])
        .run(tauri::generate_context!())
        .expect("error while running Dravyn Desktop");
}
