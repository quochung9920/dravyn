mod commands;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

use crate::commands::chromium::{self, ChromiumCommand};
use crate::commands::doctor;

#[derive(Parser)]
#[command(name = "dravyn", version, about = "Dravyn Browser Core")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Check the Dravyn development environment
    Doctor,
    /// Manage the local Chromium workspace and build
    Chromium {
        #[command(subcommand)]
        command: ChromiumCommand,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Doctor) => doctor::run(),
        Some(Commands::Chromium { command }) => chromium::run(command),
        None => {
            print_banner();
            Ok(())
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

fn print_banner() {
    println!("🐉 Dravyn");
    println!("Browser Core Development Environment\n");
    println!("Version: {}\n", env!("CARGO_PKG_VERSION"));
    println!("Commands:");
    println!("  dravyn doctor");
    println!("  dravyn chromium status");
    println!("  dravyn chromium bootstrap");
    println!("  dravyn chromium configure");
    println!("  dravyn chromium build");
    println!("  dravyn chromium run");
}
