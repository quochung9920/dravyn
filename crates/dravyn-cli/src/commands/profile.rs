use anyhow::{Result, bail};
use clap::Subcommand;
use dravyn_common::Workspace;
use dravyn_core::profile_runtime;
use dravyn_network::NetworkMode;
use dravyn_profile::{Profile, ProfileDraft, ProfileStore};

#[derive(Debug, Subcommand)]
pub enum ProfileCommand {
    /// List saved browser profiles
    List,
    /// Create a profile with isolated browser storage
    Create {
        name: String,
        #[arg(long)]
        start_url: Option<String>,
        #[arg(long, default_value = "")]
        notes: String,
    },
    /// Show one profile
    Show { id: String },
    /// Launch the Dravyn Chromium build with this profile
    Launch { id: String },
    /// Show the runtime state of one profile
    Status { id: String },
    /// Stop a running profile
    Stop { id: String },
    /// Reset cookies/cache/site storage by recreating this profile's user-data directory
    Reset { id: String },
    /// Permanently delete a stopped profile and its browser data
    Delete { id: String },
}

pub fn run(command: ProfileCommand) -> Result<()> {
    let workspace = Workspace::from_env()?;
    let store = ProfileStore::new(workspace.clone());

    match command {
        ProfileCommand::List => {
            let profiles = store.list()?;
            if profiles.is_empty() {
                println!("No profiles yet. Create one in Dravyn Desktop or with `dravyn profile create`. ");
                return Ok(());
            }
            println!("{:<50}  {:<24}  STATUS", "ID", "NAME");
            for profile in profiles {
                let state = profile_runtime::status(&workspace, &store, &profile)?;
                let label = if state.running {
                    format!("running (pid {})", state.pid.unwrap_or_default())
                } else {
                    "stopped".to_owned()
                };
                println!("{:<50}  {:<24}  {label}", profile.id, profile.name);
            }
        }
        ProfileCommand::Create {
            name,
            start_url,
            notes,
        } => {
            let mut draft = ProfileDraft {
                name,
                notes,
                ..ProfileDraft::default()
            };
            draft.browser.start_url = start_url;
            let profile = store.create(draft)?;
            println!("Created profile {} ({})", profile.name, profile.id);
        }
        ProfileCommand::Show { id } => print_profile(&store.get(&id)?),
        ProfileCommand::Launch { id } => {
            let profile = store.get(&id)?;
            let state = profile_runtime::launch(&workspace, &store, &profile)?;
            println!(
                "Launched {} (pid {})",
                profile.name,
                state.pid.unwrap_or_default()
            );
        }
        ProfileCommand::Status { id } => {
            let profile = store.get(&id)?;
            let state = profile_runtime::status(&workspace, &store, &profile)?;
            if state.running {
                println!("{} is running (pid {})", profile.name, state.pid.unwrap_or_default());
            } else {
                println!("{} is stopped", profile.name);
            }
        }
        ProfileCommand::Stop { id } => {
            let profile = store.get(&id)?;
            profile_runtime::stop(&workspace, &store, &profile)?;
            println!("Stopped {}", profile.name);
        }
        ProfileCommand::Reset { id } => {
            let profile = store.get(&id)?;
            ensure_stopped(&workspace, &store, &profile)?;
            store.reset_user_data(&id)?;
            println!("Reset browser data for {}", profile.name);
        }
        ProfileCommand::Delete { id } => {
            let profile = store.get(&id)?;
            ensure_stopped(&workspace, &store, &profile)?;
            store.delete(&id)?;
            println!("Deleted {}", profile.name);
        }
    }

    Ok(())
}

fn ensure_stopped(workspace: &Workspace, store: &ProfileStore, profile: &Profile) -> Result<()> {
    let state = profile_runtime::status(workspace, store, profile)?;
    if state.running {
        bail!(
            "profile {} is running; stop it before modifying its browser data",
            profile.name
        );
    }
    Ok(())
}

fn print_profile(profile: &Profile) {
    println!("Name:       {}", profile.name);
    println!("ID:         {}", profile.id);
    println!("Notes:      {}", profile.notes);
    println!("Tags:       {}", profile.tags.join(", "));
    println!(
        "Start URL:  {}",
        profile.browser.start_url.as_deref().unwrap_or("(none)")
    );
    println!(
        "Window:     {} x {}",
        profile.browser.window_width.unwrap_or_default(),
        profile.browser.window_height.unwrap_or_default()
    );
    match profile.network.mode {
        NetworkMode::Direct => println!("Network:    direct"),
        NetworkMode::Proxy => {
            if let Some(proxy) = &profile.network.proxy {
                println!(
                    "Network:    {}://{}:{}",
                    proxy.scheme.as_str(),
                    proxy.host,
                    proxy.port
                );
            } else {
                println!("Network:    proxy (invalid configuration)");
            }
        }
    }
}
