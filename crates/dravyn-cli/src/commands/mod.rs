pub mod chromium;
pub mod desktop;
pub mod doctor;
pub mod profile;

pub const SCRIPT_NOT_FOUND_HINT: &str = "Run this command from the Dravyn repository checkout, or set DRAVYN_REPO_ROOT to the repository path.";

pub fn locate_script(name: &str) -> anyhow::Result<std::path::PathBuf> {
    if let Some(root) = std::env::var_os("DRAVYN_REPO_ROOT") {
        let candidate = std::path::PathBuf::from(&root).join("scripts").join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
        anyhow::bail!(
            "DRAVYN_REPO_ROOT={} does not contain scripts/{name}.\n\n{SCRIPT_NOT_FOUND_HINT}",
            root.to_string_lossy()
        );
    }

    let current =
        std::env::current_dir().map_err(|_| anyhow::anyhow!("cannot read current directory"))?;
    for ancestor in current.ancestors() {
        let candidate = ancestor.join("scripts").join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    anyhow::bail!(
        "Could not locate scripts/{name} from {}.\n\n{SCRIPT_NOT_FOUND_HINT}",
        current.display()
    );
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn locates_script_in_ancestor_directory() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("dravyn-cli-locate-{unique}"));
        let scripts = root.join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        fs::write(scripts.join("chromium-bootstrap.sh"), "#!/bin/sh\n").unwrap();

        assert_eq!(
            find_from(root.join("a/b/c"), "chromium-bootstrap.sh").unwrap(),
            scripts.join("chromium-bootstrap.sh")
        );

        let _ = fs::remove_dir_all(root);
    }

    fn find_from(start: std::path::PathBuf, name: &str) -> anyhow::Result<std::path::PathBuf> {
        for ancestor in start.ancestors() {
            let candidate = ancestor.join("scripts").join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
        anyhow::bail!("not found")
    }
}
