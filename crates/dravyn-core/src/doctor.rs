use std::env;
use std::fs;
use std::process::Command;

use dravyn_common::Workspace;

use crate::chromium::{self, ChromiumDetection};

#[derive(Debug, Clone)]
pub struct ToolStatus {
    pub name: &'static str,
    pub available: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DoctorReport {
    pub is_wsl: bool,
    pub wslg_available: bool,
    pub memory_total_gib: Option<f64>,
    pub memory_available_gib: Option<f64>,
    pub disk_free_gib: Option<f64>,
    pub tools: Vec<ToolStatus>,
    pub chromium: ChromiumDetection,
    pub workspace_root: std::path::PathBuf,
}

fn command_version(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let value = if stdout.trim().is_empty() {
        stderr.trim()
    } else {
        stdout.trim()
    };

    value.lines().next().map(str::to_owned)
}

fn check_tool(name: &'static str, command: &str, args: &[&str]) -> ToolStatus {
    let version = command_version(command, args);
    ToolStatus {
        name,
        available: version.is_some(),
        version,
    }
}

fn parse_meminfo() -> (Option<f64>, Option<f64>) {
    let content = match fs::read_to_string("/proc/meminfo") {
        Ok(value) => value,
        Err(_) => return (None, None),
    };

    fn value_gib(content: &str, key: &str) -> Option<f64> {
        let line = content.lines().find(|line| line.starts_with(key))?;
        let kib = line.split_whitespace().nth(1)?.parse::<f64>().ok()?;
        Some(kib / 1024.0 / 1024.0)
    }

    (
        value_gib(&content, "MemTotal:"),
        value_gib(&content, "MemAvailable:"),
    )
}

fn disk_free_gib() -> Option<f64> {
    let output = Command::new("df").args(["-Pk", "."]).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().nth(1)?;
    let available_kib = line.split_whitespace().nth(3)?.parse::<f64>().ok()?;
    Some(available_kib / 1024.0 / 1024.0)
}

pub fn run_doctor() -> DoctorReport {
    let workspace = Workspace::from_env().unwrap_or_else(|_| {
        Workspace::from_root(env::temp_dir().join("dravyn-unresolved-workspace"))
    });

    run_doctor_for_workspace(workspace)
}

pub fn run_doctor_for_workspace(workspace: Workspace) -> DoctorReport {
    let proc_version = fs::read_to_string("/proc/version").unwrap_or_default();
    let is_wsl = proc_version.to_lowercase().contains("microsoft");
    let wslg_available = env::var("WAYLAND_DISPLAY")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let (memory_total_gib, memory_available_gib) = parse_meminfo();

    let tools = vec![
        check_tool("Git", "git", &["--version"]),
        check_tool("Rust", "rustc", &["--version"]),
        check_tool("Cargo", "cargo", &["--version"]),
        check_tool("Node", "node", &["--version"]),
        check_tool("pnpm", "pnpm", &["--version"]),
        check_tool("Python", "python3", &["--version"]),
        check_tool("Clang", "clang", &["--version"]),
        check_tool("Ninja", "ninja", &["--version"]),
        check_tool("CMake", "cmake", &["--version"]),
    ];

    let chromium = chromium::detect(&workspace);

    DoctorReport {
        is_wsl,
        wslg_available,
        memory_total_gib,
        memory_available_gib,
        disk_free_gib: disk_free_gib(),
        tools,
        chromium,
        workspace_root: workspace.root().to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_workspace(tag: &str) -> Workspace {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock works")
            .as_nanos();
        let dir = env::temp_dir().join(format!(
            "dravyn-doctor-test-{tag}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        Workspace::from_root(dir)
    }

    #[test]
    fn doctor_report_aggregates_chromium_detection() {
        let ws = temp_workspace("aggregate");
        fs::create_dir_all(ws.depot_tools()).expect("mkdir");
        fs::write(ws.depot_tools().join("fetch"), "#!/bin/sh\n").expect("fetch");

        let report = run_doctor_for_workspace(ws.clone());
        assert_eq!(
            report.chromium.state,
            chromium::ChromiumState::DepotToolsReady
        );
        assert_eq!(report.workspace_root, ws.root());
        assert!(report.tools.iter().any(|tool| tool.name == "Git"));
        let _ = fs::remove_dir_all(ws.root());
    }

    #[test]
    fn m1_is_ready_only_when_chromium_is_built() {
        let ws = temp_workspace("m1");
        let report = run_doctor_for_workspace(ws.clone());
        assert_ne!(
            report.chromium.state,
            chromium::ChromiumState::Built,
            "fresh workspace must not claim a build"
        );
        let _ = fs::remove_dir_all(ws.root());
    }

    #[test]
    fn default_paths_stay_outside_the_repository() {
        let ws = Workspace::from_root(Path::new("/home/dev").join(".cache/dravyn"));
        assert!(ws.root().starts_with("/home/dev"));
        assert!(
            !ws.root()
                .starts_with(env::current_dir().unwrap_or_default())
        );
    }
}
