use std::env;
use std::fs;
use std::process::Command;

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

    DoctorReport {
        is_wsl,
        wslg_available,
        memory_total_gib,
        memory_available_gib,
        disk_free_gib: disk_free_gib(),
        tools,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_status_is_constructible() {
        let status = ToolStatus {
            name: "Example",
            available: true,
            version: Some("1.0".into()),
        };
        assert!(status.available);
        assert_eq!(status.name, "Example");
    }
}
