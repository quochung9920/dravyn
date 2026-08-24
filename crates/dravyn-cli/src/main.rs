mod commands;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

use crate::commands::chromium::{self, ChromiumCommand};
use crate::commands::profile::{self, ProfileCommand};
use crate::commands::{desktop, doctor};

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
    /// Open the Dravyn Desktop profile manager in development mode
    Desktop,
    /// Manage isolated browser profiles
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
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
        Some(Commands::Desktop) => desktop::run(),
        Some(Commands::Profile { command }) => profile::run(command),
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
    println!("Local-first Browser Development Environment\n");
    println!("Version: {}\n", env!("CARGO_PKG_VERSION"));
    println!("Commands:");
    println!("  dravyn desktop");
    println!("  dravyn profile list");
    println!("  dravyn profile create <name>");
    println!("  dravyn profile launch <id>");
    println!("  dravyn doctor");
    println!("  dravyn chromium status");
    println!("  dravyn chromium bootstrap");
    println!("  dravyn chromium configure");
    println!("  dravyn chromium build");
    println!("  dravyn chromium run");
}
