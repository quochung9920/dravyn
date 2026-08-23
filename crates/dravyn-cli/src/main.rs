use anyhow::Result;
use clap::{Parser, Subcommand};
use dravyn_core::doctor::run_doctor;

#[derive(Parser)]
#[command(name = "dravyn", version = "0.0.1-dev", about = "Dravyn Browser Core")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Check the Dravyn development environment
    Doctor,
}

fn status(ok: bool) -> &'static str {
    if ok { "PASS" } else { "MISSING" }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Doctor) => {
            let report = run_doctor();

            println!("🐉 Dravyn Doctor\n");
            println!("Environment");
            println!("--------------------------------");
            println!("WSL2             {}", status(report.is_wsl));
            println!("WSLg             {}", status(report.wslg_available));

            println!("\nToolchain");
            println!("--------------------------------");
            for tool in &report.tools {
                match &tool.version {
                    Some(version) => println!("{:<16} PASS    {}", tool.name, version),
                    None => println!("{:<16} MISSING", tool.name),
                }
            }

            println!("\nResources");
            println!("--------------------------------");
            match report.memory_total_gib {
                Some(value) => println!("Memory total     {:.1} GiB", value),
                None => println!("Memory total     UNKNOWN"),
            }
            match report.memory_available_gib {
                Some(value) => println!("Memory available {:.1} GiB", value),
                None => println!("Memory available UNKNOWN"),
            }
            match report.disk_free_gib {
                Some(value) => println!("Disk free        {:.1} GiB", value),
                None => println!("Disk free        UNKNOWN"),
            }

            println!("\nChromium");
            println!("--------------------------------");
            println!("Source           NOT SETUP");
            println!("Build            NOT AVAILABLE");

            let tools_ready = report.tools.iter().all(|tool| tool.available);
            println!("\nOverall");
            println!("--------------------------------");
            println!("M0 environment   {}", status(report.is_wsl && report.wslg_available && tools_ready));
            println!("M1 Chromium      NOT STARTED");
        }
        None => {
            println!("🐉 Dravyn");
            println!("Browser Core Development Environment\n");
            println!("Version: 0.0.1-dev\n");
            println!("Commands:");
            println!("  dravyn doctor");
        }
    }

    Ok(())
}
