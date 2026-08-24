# Dravyn

Dravyn is a local-first browser-core research and development project focused on Chromium runtime management, isolated browser profiles, explicit network policy, privacy diagnostics, compatibility testing, and authorized browser automation.

## Current milestone

**M4 - Per-Profile Fingerprint Engine** (M0/M1/M2/M3 complete)

M1 established the reproducible Chromium checkout/build/run path. M2 introduced persistent browser profiles and the first Tauri desktop app. M3 added the professional operations console, network checks and local privacy inspection. M4 makes fingerprint state a first-class property of every individual profile:

- one fingerprint baseline per profile
- local snapshot history per profile, capped to the latest 50 captures
- current consistency score, surface count, review items and drift count on profile cards
- stable-surface drift detection against the selected profile's own baseline
- dedicated Fingerprint Center with timeline, latest comparison and baseline controls
- automatic baseline creation on the first successful audit
- explicit **Set latest as baseline** workflow for intentional environment changes
- a loopback-only fingerprint capture service bound to `127.0.0.1` with an ephemeral desktop-session token
- local audit coverage for User-Agent/platform, Client Hints, language/timezone, screen/DPR, Canvas, WebGL, AudioContext, WebRTC, hardware hints, storage and permissions
- fingerprint data removal when the owning profile is deleted

Dravyn's scope is privacy engineering, local profile isolation, network control, compatibility testing, and authorized automation. The fingerprint engine observes, stores and compares browser-visible surfaces for the owning profile; it does not randomize or spoof identity signals. Dravyn is not intended to bypass identity verification, CAPTCHA systems, KYC, anti-fraud controls, or third-party abuse protections.

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

The desktop app reuses `$DRAVYN_HOME/chromium/src/out/Dravyn/chrome`; UI, profile-manager and fingerprint-engine changes do not require rebuilding Chromium.

### Profile CLI

```bash
dravyn profile list
dravyn profile create "QA profile" --start-url https://example.com
dravyn profile launch <id>
dravyn profile stop <id>
```

## M4 per-profile fingerprints

Open **Fingerprints** in Dravyn Desktop, select a profile and choose **Run profile audit**. Dravyn opens a local inspector inside that exact Chromium `--user-data-dir`. The inspector posts its observations only to the desktop's loopback capture service, which stores them under that profile's fingerprint directory.

The first successful capture becomes the profile baseline. Later captures compare stable surfaces with that baseline and report drift. Dynamic observations such as permission state, color scheme and WebRTC candidate type can still be recorded without being treated as baseline identity.

Default storage:

```text
~/.cache/dravyn/
├── profiles/
│   └── <profile-id>/
│       ├── profile.json
│       └── user-data/
├── fingerprints/
│   └── <profile-id>/
│       ├── baseline.json
│       ├── latest.json
│       └── history/
│           └── <snapshot-id>.json
└── runtime/
```

See [`docs/m4-per-profile-fingerprints.md`](docs/m4-per-profile-fingerprints.md) for the M4 architecture, storage model and safety boundary.

## Repository layout

```text
apps/
  desktop/              Tauri 2 + React/TypeScript operations console

crates/
  dravyn-cli/           Command-line interface
  dravyn-core/          Diagnostics, Chromium state detection, runtime orchestration
  dravyn-common/        Shared types and DRAVYN_HOME workspace resolution
  dravyn-profile/       Persistent profile domain + storage
  dravyn-network/       Explicit direct/proxy network configuration
  dravyn-fingerprint/   Per-profile baseline, history and drift engine

browser/                Chromium configuration of record + future reviewed privacy patch sets
scripts/                Development helpers
docs/                   Architecture, roadmap, Chromium and desktop workflow docs
```

## Local/generated data and Git

Large or machine-generated data never belongs in the repository. The root `.gitignore` excludes Chromium source/build trees, nested Rust/Tauri `target/` trees, frontend `dist/`, Node dependencies, pnpm stores, local Dravyn profile/fingerprint/runtime/log/cache data, test reports, and local desktop lockfiles.

Source-of-truth files such as `Cargo.toml`, `package.json`, Tauri configuration, Chromium configuration and application source/assets remain tracked.

## Chromium workspace

Large Chromium sources/build outputs live outside the repository by default at `~/.cache/dravyn`:

```text
~/.cache/dravyn/
├── depot_tools/
├── chromium/
│   └── src/out/Dravyn/chrome
├── profiles/
├── fingerprints/
└── runtime/
```

Override this with `DRAVYN_HOME` when needed.

## Status

Early development (`0.1.0-dev`). APIs, profile schema, fingerprint schema and desktop UI may change without notice.
