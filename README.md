# Dravyn

Dravyn is a local-first browser-core research and development project focused on Chromium runtime management, isolated browser profiles, explicit network policy, privacy diagnostics, compatibility testing, and authorized browser automation.

## Current milestone

**M3 - Professional Desktop, Network & Privacy Audit** (M0/M1/M2 complete)

M1 established the reproducible Chromium checkout/build/run path. M2 introduced persistent browser profiles and the first Tauri desktop app. M3 turns that foundation into a more complete browser operations console:

- professional Dashboard, Profiles, Network, Privacy, Diagnostics and Settings views
- searchable/sortable isolated profile management
- profile create/edit/clone/launch/stop/reset/delete workflows
- direct or explicit HTTP/HTTPS/SOCKS5 proxy configuration
- local proxy endpoint reachability tests
- local-only fingerprint/privacy inspector opened inside the selected Dravyn Chromium profile
- consistency checks across browser-exposed surfaces such as User-Agent/platform, language, timezone, screen, Canvas, WebGL, AudioContext, WebRTC and hardware hints
- system diagnostics for Chromium, WSLg/display, profile storage and runtime state
- stronger Git ignore rules for Chromium workspaces, Rust/Tauri builds, frontend builds, dependencies, runtime profiles, cache, logs and locally generated desktop lockfiles

Dravyn's scope is privacy engineering, local profile isolation, network control, compatibility testing, and authorized automation. The privacy inspector observes and reports browser surfaces; it does not modify or spoof identity signals. Dravyn is not intended to bypass identity verification, CAPTCHA systems, KYC, anti-fraud controls, or third-party abuse protections.

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

For direct desktop development:

```bash
cd apps/desktop
pnpm install
pnpm tauri dev
```

The desktop app reuses `$DRAVYN_HOME/chromium/src/out/Dravyn/chrome`; UI/profile-manager changes do not require rebuilding Chromium.

### Profile CLI

```bash
dravyn profile list
dravyn profile create "QA profile" --start-url https://example.com
dravyn profile launch <id>
dravyn profile stop <id>
```

## M3 privacy audit

Open **Privacy** in Dravyn Desktop and choose **Open local audit** for a profile. Dravyn writes a self-contained inspector to `$DRAVYN_HOME/runtime/privacy-audit/index.html` and opens it inside that exact Chromium profile. The inspector is local-only and does not transmit its observations.

See [`docs/m3-professional-desktop.md`](docs/m3-professional-desktop.md) for the M3 details and safety boundary.

## Repository layout

```text
apps/
  desktop/          Tauri 2 + React/TypeScript operations console

crates/
  dravyn-cli/       Command-line interface
  dravyn-core/      Diagnostics, Chromium state detection, runtime orchestration
  dravyn-common/    Shared types and DRAVYN_HOME workspace resolution
  dravyn-profile/   Persistent profile domain + storage
  dravyn-network/   Explicit direct/proxy network configuration

browser/            Chromium configuration of record + future reviewed privacy patch sets
scripts/            Development helpers
docs/               Architecture, roadmap, Chromium and desktop workflow docs
```

## Local/generated data and Git

Large or machine-generated data never belongs in the repository. The root `.gitignore` excludes Chromium source/build trees, nested Rust/Tauri `target/` trees, frontend `dist/`, Node dependencies, pnpm stores, Dravyn runtime/profile/log/cache data, test reports, and local desktop lockfiles.

Source-of-truth files such as `Cargo.toml`, `package.json`, Tauri configuration, Chromium configuration and application source/assets remain tracked.

## Chromium workspace

Large Chromium sources/build outputs live outside the repository by default at `~/.cache/dravyn`:

```text
~/.cache/dravyn/
├── depot_tools/
├── chromium/
│   └── src/out/Dravyn/chrome
├── profiles/
└── runtime/
```

Override this with `DRAVYN_HOME` when needed.

## Status

Early development (`0.1.0-dev`). APIs, profile schema, and desktop UI may change without notice.
