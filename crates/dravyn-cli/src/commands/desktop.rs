use anyhow::{Context, Result, bail};
use std::process::Command;

use super::locate_script;

pub fn run() -> Result<()> {
    let script = locate_script("desktop-dev.sh")?;
    let status = Command::new(&script)
        .status()
        .with_context(|| format!("failed to run {}", script.display()))?;
    if !status.success() {
        bail!("Dravyn Desktop exited with status {status}");
    }
    Ok(())
}
