use dravyn_common::Workspace;
use dravyn_core::{chromium, profile_runtime};
use dravyn_profile::{Profile, ProfileDraft, ProfileStore};
use serde::Serialize;

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
}

fn context() -> Result<(Workspace, ProfileStore), String> {
    let workspace = Workspace::from_env().map_err(|error| error.to_string())?;
    let store = ProfileStore::new(workspace.clone());
    Ok((workspace, store))
}

fn view(workspace: &Workspace, store: &ProfileStore, profile: Profile) -> Result<ProfileView, String> {
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
    let profile = store.update(&id, draft).map_err(|error| error.to_string())?;
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
    store.reset_user_data(&id).map_err(|error| error.to_string())?;
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
            delete_profile
        ])
        .run(tauri::generate_context!())
        .expect("error while running Dravyn Desktop");
}
