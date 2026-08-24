# Dravyn

Dravyn is a local-first browser-core research and development project focused on Chromium runtime management, isolated browser profiles, explicit network policy, privacy-oriented environment controls, and developer automation.

## Current milestone

**M2 - Desktop Profile Manager** (M0/M1 complete)

M1 established a reproducible Chromium checkout/build/run path. M2 adds the first usable Dravyn application layer:

- persistent isolated profiles under `$DRAVYN_HOME/profiles`
- one Chromium `--user-data-dir` per profile
- profile launch/stop/status with guarded PID tracking on Linux/WSLg
- direct or explicit HTTP/HTTPS/SOCKS5 proxy configuration
- profile reset/delete safeguards
- a Tauri 2 + React/TypeScript desktop control panel
- CLI commands that share the same Rust profile/runtime implementation

Dravyn's scope is privacy, local profile isolation, network control, compatibility testing, and authorized browser automation. It is not intended to bypass identity verification, CAPTCHA systems, KYC, anti-fraud controls, or third-party abuse protections.

## Quick start

```bash
git clone https://github.com/quochung9920/dravyn.git
cd dravyn

cargo test --workspace
cargo install --path crates/dravyn-cli --force

dravyn doctor
dravyn chromium status
```

### Chromium foundation

```bash
# First-time Chromium source/dependency setup
dravyn chromium bootstrap

# Generate out/Dravyn
dravyn chromium configure

# Build the chrome target
dravyn chromium build

# Launch the development profile
dravyn chromium run
```

### Dravyn Desktop

After Chromium has been built successfully, install the Tauri Linux dependencies described in [`docs/m2-desktop.md`](docs/m2-desktop.md), then:

```bash
dravyn desktop
```

The desktop app reuses `$DRAVYN_HOME/chromium/src/out/Dravyn/chrome`; frontend/profile-manager changes do not require rebuilding Chromium.

### Profile CLI

```bash
dravyn profile list
dravyn profile create "QA profile" --start-url https://example.com
dravyn profile launch <id>
dravyn profile stop <id>
```

## Repository layout

```text
apps/
  desktop/          Tauri 2 + React/TypeScript Dravyn control panel

crates/
  dravyn-cli/       Command-line interface
  dravyn-core/      Diagnostics, Chromium state detection, runtime orchestration
  dravyn-common/    Shared types and DRAVYN_HOME workspace resolution
  dravyn-profile/   Persistent profile domain + storage
  dravyn-network/   Explicit direct/proxy network configuration

browser/            Chromium configuration of record + future reviewed patch sets
scripts/            Development helpers
docs/               Architecture, roadmap, Chromium and desktop workflow docs
```

## Chromium workspace

Large Chromium sources/build outputs never enter this repository. By default they live at `~/.cache/dravyn`:

```text
~/.cache/dravyn/
├── depot_tools/
├── chromium/
│   └── src/out/Dravyn/chrome
├── profiles/
└── runtime/
```

Override this with `DRAVYN_HOME` when needed.

See [`docs/chromium.md`](docs/chromium.md) for the Chromium workflow and [`docs/m2-desktop.md`](docs/m2-desktop.md) for the desktop/profile-manager workflow.

## Status

Early development (`0.1.0-dev`). APIs, profile schema, and desktop UI may change without notice.
