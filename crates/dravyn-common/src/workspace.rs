use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub const HOME_ENV: &str = "HOME";
pub const WORKSPACE_ENV: &str = "DRAVYN_HOME";

const DEFAULT_WORKSPACE_SUBPATH: &str = ".cache/dravyn";
const DEPOT_TOOLS_COMPONENT: &str = "depot_tools";
const CHROMIUM_COMPONENT: &str = "chromium";
const CHROMIUM_SRC_COMPONENT: &str = "src";
const BUILD_OUTPUT_SUBPATH: &str = "out/Dravyn";
const CHROME_BINARY_NAME: &str = "chrome";
const REVISION_FILE_NAME: &str = "revision.txt";
const RUNTIME_COMPONENT: &str = "runtime";
const DEV_PROFILE_COMPONENT: &str = "dev-profile";
const PROFILES_COMPONENT: &str = "profiles";
const PROFILE_RUNTIME_COMPONENT: &str = "profile-processes";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    pub fn from_root(root: PathBuf) -> Self {
        Self { root }
    }

    /// Resolves the Dravyn workspace from the environment.
    ///
    /// `DRAVYN_HOME` wins when set; otherwise the default is
    /// `$HOME/.cache/dravyn`. Returns an error when neither variable is set.
    pub fn from_env() -> Result<Self, WorkspaceError> {
        let dravyn_home = std::env::var_os(WORKSPACE_ENV);
        let home = std::env::var_os(HOME_ENV);
        resolve_workspace_root(dravyn_home, home)
            .map(Self::from_root)
            .map_err(WorkspaceError)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn depot_tools(&self) -> PathBuf {
        self.root.join(DEPOT_TOOLS_COMPONENT)
    }

    pub fn chromium_dir(&self) -> PathBuf {
        self.root.join(CHROMIUM_COMPONENT)
    }

    pub fn chromium_src(&self) -> PathBuf {
        self.chromium_dir().join(CHROMIUM_SRC_COMPONENT)
    }

    pub fn build_output(&self) -> PathBuf {
        self.chromium_src().join(BUILD_OUTPUT_SUBPATH)
    }

    pub fn chrome_binary(&self) -> PathBuf {
        self.build_output().join(CHROME_BINARY_NAME)
    }

    pub fn revision_file(&self) -> PathBuf {
        self.chromium_dir().join(REVISION_FILE_NAME)
    }

    pub fn runtime_dir(&self) -> PathBuf {
        self.root.join(RUNTIME_COMPONENT)
    }

    pub fn dev_profile(&self) -> PathBuf {
        self.runtime_dir().join(DEV_PROFILE_COMPONENT)
    }

    pub fn profiles_dir(&self) -> PathBuf {
        self.root.join(PROFILES_COMPONENT)
    }

    pub fn profile_runtime_dir(&self) -> PathBuf {
        self.runtime_dir().join(PROFILE_RUNTIME_COMPONENT)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceError(pub &'static str);

impl core::fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            "no-home" => write!(
                f,
                "neither {WORKSPACE_ENV} nor {HOME_ENV} is set; cannot resolve the Dravyn workspace"
            ),
            _ => write!(f, "{}", self.0),
        }
    }
}

impl std::error::Error for WorkspaceError {}

/// Pure resolver so callers can test every branch without touching process env.
pub fn resolve_workspace_root(
    dravyn_home: Option<OsString>,
    home: Option<OsString>,
) -> Result<PathBuf, &'static str> {
    fn trimmed(value: &OsString) -> String {
        value.to_string_lossy().trim().to_owned()
    }

    if let Some(value) = &dravyn_home {
        let value = trimmed(value);
        if value.is_empty() {
            return Err("DRAVYN_HOME is set but empty");
        }
        return Ok(PathBuf::from(value));
    }

    match home {
        Some(home) => {
            let home = trimmed(&home);
            if home.is_empty() {
                return Err("HOME is set but empty");
            }
            Ok(Path::new(&home).join(DEFAULT_WORKSPACE_SUBPATH))
        }
        None => Err("no-home"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    fn os(value: &str) -> Option<OsString> {
        Some(OsStr::new(value).to_os_string())
    }

    #[test]
    fn defaults_to_home_cache_dravyn() {
        let root = resolve_workspace_root(None, os("/home/dev")).expect("root resolves");
        assert_eq!(root, PathBuf::from("/home/dev/.cache/dravyn"));
    }

    #[test]
    fn dravyn_home_overrides_default() {
        let root =
            resolve_workspace_root(os("/mnt/big/dravyn"), os("/home/dev")).expect("root resolves");
        assert_eq!(root, PathBuf::from("/mnt/big/dravyn"));
    }

    #[test]
    fn missing_both_variables_is_an_error() {
        assert_eq!(resolve_workspace_root(None, None), Err("no-home"));
    }

    #[test]
    fn empty_override_is_rejected() {
        assert_eq!(
            resolve_workspace_root(Some(OsString::from("   ")), os("/home/dev")),
            Err("DRAVYN_HOME is set but empty")
        );
    }

    #[test]
    fn paths_are_derived_from_a_single_root() {
        let ws = Workspace::from_root(PathBuf::from("/data/dravyn"));
        assert_eq!(ws.depot_tools(), PathBuf::from("/data/dravyn/depot_tools"));
        assert_eq!(
            ws.chromium_src(),
            PathBuf::from("/data/dravyn/chromium/src")
        );
        assert_eq!(
            ws.build_output(),
            PathBuf::from("/data/dravyn/chromium/src/out/Dravyn")
        );
        assert_eq!(
            ws.chrome_binary(),
            PathBuf::from("/data/dravyn/chromium/src/out/Dravyn/chrome")
        );
        assert_eq!(
            ws.revision_file(),
            PathBuf::from("/data/dravyn/chromium/revision.txt")
        );
        assert_eq!(
            ws.dev_profile(),
            PathBuf::from("/data/dravyn/runtime/dev-profile")
        );
        assert_eq!(ws.profiles_dir(), PathBuf::from("/data/dravyn/profiles"));
        assert_eq!(
            ws.profile_runtime_dir(),
            PathBuf::from("/data/dravyn/runtime/profile-processes")
        );
    }

    #[test]
    fn error_message_mentions_environment_variables() {
        let err = WorkspaceError("no-home");
        assert!(err.to_string().contains("DRAVYN_HOME"));
        assert!(err.to_string().contains("HOME"));
    }
}
