use anyhow::{Context, Result};
use clap::Subcommand;
use dravyn_common::Workspace;

use crate::commands::locate_script;
use dravyn_core::chromium::{
    self, ChromiumState, ScriptError, detect, resolve_build_jobs, run_script,
};

#[derive(Subcommand)]
pub enum ChromiumCommand {
    /// Inspect the local Chromium workspace (no downloads, no changes)
    Status,
    /// Install depot_tools and fetch the Chromium source checkout
    Bootstrap,
    /// Generate out/Dravyn with Dravyn GN arguments
    Configure,
    /// Build the chrome target with resource-aware parallelism
    Build {
        /// Explicit number of parallel jobs; defaults to a RAM-aware value
        #[arg(long)]
        jobs: Option<u32>,
    },
    /// Launch the built Chromium through WSLg with a clean dev profile
    Run {
        /// Optional startup URL
        url: Option<String>,
    },
}

pub fn run(command: ChromiumCommand) -> Result<()> {
    match command {
        ChromiumCommand::Status => status(),
        ChromiumCommand::Bootstrap => execute("chromium-bootstrap.sh", &[]),
        ChromiumCommand::Configure => execute("chromium-configure.sh", &[]),
        ChromiumCommand::Build { jobs } => {
            let jobs = resolve_build_jobs(jobs);
            println!("Selected build jobs: {jobs} (override with --jobs N or DRAVYN_BUILD_JOBS)");
            let jobs = jobs.to_string();
            execute("chromium-build.sh", &["--jobs".to_owned(), jobs])
        }
        ChromiumCommand::Run { url } => {
            let args: Vec<String> = url.into_iter().collect();
            execute("chromium-run.sh", &args)
        }
    }
}

fn status() -> Result<()> {
    let workspace = Workspace::from_env().context(chromium_workspace_hint())?;
    let detection = detect(&workspace);

    println!("🐉 Dravyn Chromium\n");
    println!("Workspace root   {}", workspace.root().display());
    println!("State            {}", detection.state.label());
    println!();
    println!(
        "depot_tools      {:<13} {}",
        readiness(detection.depot_tools_ready),
        detection.depot_tools_root.display()
    );
    println!(
        "Source           {:<13} {}",
        readiness(detection.source_ready),
        detection.source_root.display()
    );
    println!(
        "Configured       {:<13} {}",
        readiness(detection.configured),
        detection.build_output.display()
    );
    println!(
        "Build            {:<13} {}",
        if detection.build_available {
            "READY"
        } else {
            "NOT BUILT"
        },
        detection.browser_binary.display()
    );
    match &detection.revision {
        Some(revision) => println!("Revision         {revision}"),
        None => println!("Revision         UNKNOWN"),
    }

    match detection.state {
        ChromiumState::Built => {}
        state => {
            if let Some(next) = chromium::script_exit_hint(state) {
                println!("\nNext step:");
                println!("  {next}");
            }
        }
    }

    Ok(())
}

fn readiness(ok: bool) -> &'static str {
    if ok { "READY" } else { "NOT SETUP" }
}

fn execute(script_name: &str, args: &[String]) -> Result<()> {
    let script = locate_script(script_name)?;
    Workspace::from_env().context(chromium_workspace_hint())?;

    println!("Running: {}", script.display());
    for arg in args {
        println!("  arg: {arg}");
    }
    println!();

    match run_script(&script, args) {
        Ok(0) => Ok(()),
        Ok(code) => anyhow::bail!("{script_name} failed with exit code {code}"),
        Err(ScriptError::NotFound(path)) => anyhow::bail!(
            "{}\n\n{}",
            ScriptError::NotFound(path),
            crate::commands::SCRIPT_NOT_FOUND_HINT
        ),
        Err(ScriptError::Io(error)) => {
            anyhow::bail!("{script_name} could not be started: {error}")
        }
    }
}

fn chromium_workspace_hint() -> String {
    "set the DRAVYN_HOME environment variable or $HOME to choose where the Chromium workspace lives"
        .to_owned()
}
