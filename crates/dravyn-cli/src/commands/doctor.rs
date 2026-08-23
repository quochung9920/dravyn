use anyhow::Result;
use dravyn_core::doctor::run_doctor;

fn status(ok: bool) -> &'static str {
    if ok { "PASS" } else { "MISSING" }
}

pub fn run() -> Result<()> {
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
    println!(
        "depot_tools      {}",
        setup_status(report.chromium.depot_tools_ready)
    );
    println!(
        "Source           {}",
        setup_status(report.chromium.source_ready)
    );
    println!(
        "Configured       {}",
        setup_status(report.chromium.configured)
    );
    println!(
        "Build            {}",
        if report.chromium.build_available {
            "PASS"
        } else {
            "NOT AVAILABLE"
        }
    );
    println!("Workspace root   {}", report.workspace_root.display());
    println!("Source root      {}", report.chromium.source_root.display());
    println!(
        "Browser binary   {}",
        report.chromium.browser_binary.display()
    );

    let tools_ready = report.tools.iter().all(|tool| tool.available);
    let m0_ready = report.is_wsl && report.wslg_available && tools_ready;
    let m1_ready = report.chromium.state == dravyn_core::chromium::ChromiumState::Built;

    println!("\nOverall");
    println!("--------------------------------");
    println!("M0 environment   {}", status(m0_ready));
    println!(
        "M1 Chromium      {}",
        if m1_ready { "PASS" } else { "NOT READY" }
    );

    Ok(())
}

fn setup_status(ok: bool) -> &'static str {
    if ok { "READY" } else { "NOT SETUP" }
}
