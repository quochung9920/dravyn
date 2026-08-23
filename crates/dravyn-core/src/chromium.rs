use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use dravyn_common::Workspace;

pub const BUILD_TARGET: &str = "chrome";

/// Lifecycle state of the local Chromium workspace, from nothing built up to a
/// usable browser binary. Detection never downloads or modifies anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChromiumState {
    NotBootstrapped,
    DepotToolsReady,
    SourceReady,
    Configured,
    Built,
}

impl ChromiumState {
    pub fn label(self) -> &'static str {
        match self {
            ChromiumState::NotBootstrapped => "NOT BOOTSTRAPPED",
            ChromiumState::DepotToolsReady => "DEPOT TOOLS READY",
            ChromiumState::SourceReady => "SOURCE READY",
            ChromiumState::Configured => "CONFIGURED",
            ChromiumState::Built => "BUILT",
        }
    }
}

/// Everything `dravyn chromium status` and the doctor need to know about the
/// current workspace without side effects.
#[derive(Debug, Clone)]
pub struct ChromiumDetection {
    pub state: ChromiumState,
    pub depot_tools_ready: bool,
    pub source_ready: bool,
    pub configured: bool,
    pub build_available: bool,
    pub depot_tools_root: PathBuf,
    pub source_root: PathBuf,
    pub build_output: PathBuf,
    pub browser_binary: PathBuf,
    pub revision: Option<String>,
}

pub fn detect(workspace: &Workspace) -> ChromiumDetection {
    let depot_tools_root = workspace.depot_tools();
    let source_root = workspace.chromium_src();
    let build_output = workspace.build_output();
    let browser_binary = workspace.chrome_binary();

    let depot_tools_ready = depot_tools_root.join("fetch").is_file();
    let source_ready = is_chromium_checkout(&source_root);
    let configured = build_output.join("args.gn").is_file();

    let build_available = fs::metadata(&browser_binary)
        .map(|meta| meta.is_file())
        .unwrap_or(false)
        && is_executable(&browser_binary);

    let state = if build_available {
        ChromiumState::Built
    } else if configured {
        ChromiumState::Configured
    } else if source_ready {
        ChromiumState::SourceReady
    } else if depot_tools_ready {
        ChromiumState::DepotToolsReady
    } else {
        ChromiumState::NotBootstrapped
    };

    let revision = fs::read_to_string(workspace.revision_file())
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());

    ChromiumDetection {
        state,
        depot_tools_ready,
        source_ready,
        configured,
        build_available,
        depot_tools_root,
        source_root,
        build_output,
        browser_binary,
        revision,
    }
}

fn is_chromium_checkout(source_root: &Path) -> bool {
    source_root.join("DEPS").is_file() && source_root.join(".git").exists()
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|meta| meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    true
}

/// Conservative parallel-job count for linking Chromium on memory-constrained
/// machines such as WSL guests.
///
/// A release link job can transiently need ~3 GiB of RAM, so jobs are capped by
/// available memory in 3 GiB slices as well as by CPU count. The result is at
/// least 1 so a build can always be attempted explicitly.
pub fn recommended_build_jobs(available_memory_kib: u64, cpu_count: u32) -> u32 {
    const KIB_PER_JOB: u64 = 3 * 1024 * 1024;
    let memory_jobs = (available_memory_kib / KIB_PER_JOB).min(u32::MAX as u64) as u32;
    cpu_count.min(memory_jobs).max(1)
}

/// Reads `MemAvailable` from `/proc/meminfo` in KiB, if the platform exposes it.
pub fn available_memory_kib() -> Option<u64> {
    let content = fs::read_to_string("/proc/meminfo").ok()?;
    let line = content
        .lines()
        .find(|line| line.starts_with("MemAvailable:"))?;
    line.split_whitespace().nth(1)?.parse::<u64>().ok()
}

/// Number of logical CPUs, with a conservative fallback of 2 when undetectable.
pub fn cpu_count() -> u32 {
    std::thread::available_parallelism()
        .map(|value| value.get() as u32)
        .unwrap_or(2)
}

/// Resolves the effective job count for a build: an explicit override always
/// wins; otherwise jobs are derived from machine resources.
pub fn resolve_build_jobs(explicit: Option<u32>) -> u32 {
    match explicit {
        Some(jobs) if jobs >= 1 => jobs,
        _ => {
            let memory = available_memory_kib().unwrap_or(u64::MAX);
            recommended_build_jobs(memory, cpu_count())
        }
    }
}

pub fn script_exit_hint(state: ChromiumState) -> Option<&'static str> {
    match state {
        ChromiumState::NotBootstrapped | ChromiumState::DepotToolsReady => {
            Some("dravyn chromium bootstrap")
        }
        ChromiumState::SourceReady => Some("dravyn chromium configure"),
        ChromiumState::Configured => Some("dravyn chromium build"),
        ChromiumState::Built => None,
    }
}

pub fn run_script(script: &Path, args: &[String]) -> Result<i32, ScriptError> {
    if !script.is_file() {
        return Err(ScriptError::NotFound(script.to_path_buf()));
    }

    let status = Command::new(script).args(args).status()?;
    Ok(status.code().unwrap_or(1))
}

#[derive(Debug)]
pub enum ScriptError {
    NotFound(PathBuf),
    Io(std::io::Error),
}

impl From<std::io::Error> for ScriptError {
    fn from(value: std::io::Error) -> Self {
        ScriptError::Io(value)
    }
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptError::NotFound(path) => write!(f, "script not found: {}", path.display()),
            ScriptError::Io(error) => write!(f, "failed to run script: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_workspace(tag: &str) -> Workspace {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock works")
            .as_nanos();
        let dir =
            env::temp_dir().join(format!("dravyn-test-{tag}-{}-{unique}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp dir");
        Workspace::from_root(dir)
    }

    #[test]
    fn empty_workspace_is_not_bootstrapped() {
        let ws = temp_workspace("empty");
        let detection = detect(&ws);
        assert_eq!(detection.state, ChromiumState::NotBootstrapped);
        assert!(!detection.depot_tools_ready);
        assert!(!detection.source_ready);
        assert!(!detection.configured);
        assert!(!detection.build_available);
        let _ = fs::remove_dir_all(ws.root());
    }

    #[test]
    fn depot_tools_alone_reports_depot_tools_ready() {
        let ws = temp_workspace("depot");
        fs::create_dir_all(ws.depot_tools()).expect("mkdir");
        fs::write(ws.depot_tools().join("fetch"), "#!/bin/sh\n").expect("write fetch");

        let detection = detect(&ws);
        assert_eq!(detection.state, ChromiumState::DepotToolsReady);
        assert!(detection.depot_tools_ready);
        assert!(!detection.source_ready);
        let _ = fs::remove_dir_all(ws.root());
    }

    #[test]
    fn deps_file_and_git_report_source_ready() {
        let ws = temp_workspace("source");
        fs::create_dir_all(ws.depot_tools()).expect("mkdir");
        fs::write(ws.depot_tools().join("fetch"), "x").expect("fetch");
        let src = ws.chromium_src();
        fs::create_dir_all(src.join(".git")).expect("git dir");
        fs::write(src.join("DEPS"), "{}").expect("DEPS");

        let detection = detect(&ws);
        assert_eq!(detection.state, ChromiumState::SourceReady);
        assert!(detection.source_ready);
        let _ = fs::remove_dir_all(ws.root());
    }

    #[test]
    fn args_gn_marks_configuration() {
        let ws = temp_workspace("configured");
        seed_source(&ws);
        fs::create_dir_all(ws.build_output()).expect("out dir");
        fs::write(ws.build_output().join("args.gn"), "is_debug = false").expect("args.gn");

        let detection = detect(&ws);
        assert_eq!(detection.state, ChromiumState::Configured);
        assert!(detection.configured);
        assert!(!detection.build_available);
        let _ = fs::remove_dir_all(ws.root());
    }

    #[cfg(unix)]
    #[test]
    fn executable_chrome_marks_build_complete() {
        use std::os::unix::fs::PermissionsExt;

        let ws = temp_workspace("built");
        seed_source(&ws);
        fs::create_dir_all(ws.build_output()).expect("out dir");
        fs::write(ws.build_output().join("args.gn"), "is_debug = false").expect("args.gn");
        fs::write(ws.chrome_binary(), "#!/bin/sh\n").expect("chrome stub");
        let mut perms = fs::metadata(ws.chrome_binary())
            .expect("stat")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(ws.chrome_binary(), perms).expect("chmod");

        let detection = detect(&ws);
        assert_eq!(detection.state, ChromiumState::Built);
        assert!(detection.build_available);
        let _ = fs::remove_dir_all(ws.root());
    }

    #[cfg(unix)]
    #[test]
    fn non_executable_chrome_is_not_a_build() {
        let ws = temp_workspace("not-exec");
        seed_source(&ws);
        fs::create_dir_all(ws.build_output()).expect("out dir");
        fs::write(ws.build_output().join("args.gn"), "x").expect("args.gn");
        fs::write(ws.chrome_binary(), "").expect("chrome stub");

        let detection = detect(&ws);
        assert_eq!(detection.state, ChromiumState::Configured);
        assert!(!detection.build_available);
        let _ = fs::remove_dir_all(ws.root());
    }

    #[test]
    fn revision_is_read_and_trimmed() {
        let ws = temp_workspace("revision");
        fs::create_dir_all(ws.chromium_dir()).expect("chromium dir");
        fs::write(ws.revision_file(), " abc123\n").expect("revision");

        let detection = detect(&ws);
        assert_eq!(detection.revision.as_deref(), Some("abc123"));
        let _ = fs::remove_dir_all(ws.root());
    }

    #[test]
    fn missing_revision_is_none() {
        let ws = temp_workspace("no-revision");
        let detection = detect(&ws);
        assert_eq!(detection.revision, None);
        let _ = fs::remove_dir_all(ws.root());
    }

    fn seed_source(ws: &Workspace) {
        fs::create_dir_all(ws.depot_tools()).expect("mkdir depot");
        fs::write(ws.depot_tools().join("fetch"), "x").expect("fetch");
        let src = ws.chromium_src();
        fs::create_dir_all(src.join(".git")).expect("git dir");
        fs::write(src.join("DEPS"), "{}").expect("DEPS");
    }

    #[test]
    fn job_count_is_capped_by_three_gib_slices() {
        assert_eq!(recommended_build_jobs(15 * 1024 * 1024, 16), 5);
        assert_eq!(recommended_build_jobs(12 * 1024 * 1024, 8), 4);
    }

    #[test]
    fn job_count_is_capped_by_cpu_count() {
        assert_eq!(recommended_build_jobs(64 * 1024 * 1024, 4), 4);
    }

    #[test]
    fn job_count_never_drops_below_one() {
        assert_eq!(recommended_build_jobs(0, 0), 1);
        assert_eq!(recommended_build_jobs(1, 1), 1);
    }

    #[test]
    fn explicit_job_override_wins() {
        assert_eq!(resolve_build_jobs(Some(7)), 7);
    }

    #[test]
    fn zero_override_falls_back_to_detection() {
        assert_eq!(resolve_build_jobs(Some(0)), resolve_build_jobs(None));
    }

    #[test]
    fn next_step_hints_follow_lifecycle() {
        assert_eq!(
            script_exit_hint(ChromiumState::NotBootstrapped),
            Some("dravyn chromium bootstrap")
        );
        assert_eq!(
            script_exit_hint(ChromiumState::DepotToolsReady),
            Some("dravyn chromium bootstrap")
        );
        assert_eq!(
            script_exit_hint(ChromiumState::SourceReady),
            Some("dravyn chromium configure")
        );
        assert_eq!(
            script_exit_hint(ChromiumState::Configured),
            Some("dravyn chromium build")
        );
        assert_eq!(script_exit_hint(ChromiumState::Built), None);
    }

    #[test]
    fn states_are_ordered_from_earliest_to_latest() {
        assert!(ChromiumState::NotBootstrapped < ChromiumState::Built);
    }
}
