# Dravyn

Dravyn is a local-first browser-core research and development project focused on Chromium runtime management, isolated browser profiles, explicit network policy, privacy diagnostics, compatibility testing, and authorized browser automation.

## Current milestone

**M5 - Per-Profile Privacy Policy & Leak Guard** (M0/M1/M2/M3/M4 complete)

M4 made fingerprint baseline, history and drift first-class properties of every profile. M5 adds a separate privacy enforcement layer so a profile has both an observed fingerprint history and an explicit browser/network privacy policy:

- per-profile Standard, Balanced, Strict and Custom privacy policies
- privacy policy persisted beside each profile's browser and network configuration
- Chromium privacy preferences written and verified before a stopped profile launches
- WebRTC `disable_non_proxied_udp` mode available per profile
- per-profile third-party-cookie and permission defaults for notifications, geolocation, camera and microphone
- Off, Monitor and Strict Network Guard modes
- Strict proxy profiles fail closed before launch when the configured proxy endpoint cannot pass TCP preflight
- centralized network preflight results shared by runtime and desktop UI
- dedicated Privacy Center with applied-policy state, network preflight and external verification launchers
- BrowserLeaks IP/WebRTC/DNS/Canvas/WebGL, EFF Cover Your Tracks and AmIUnique launch inside the exact selected profile
- external verification is deliberately kept separate from local preflight: endpoint reachability is not presented as proof of anonymity or zero leakage

Dravyn's scope is privacy engineering, local profile isolation, network control, compatibility testing, and authorized automation. The fingerprint engine observes/stores browser-visible surfaces and the privacy engine applies defensive browser policy. Dravyn does not randomize or spoof identity signals, impersonate devices, or provide CAPTCHA/KYC/anti-fraud evasion.

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

The desktop app reuses `$DRAVYN_HOME/chromium/src/out/Dravyn/chrome`; M5 application-layer changes do not require rebuilding Chromium.

## M5 privacy workflow

Open **Profiles** and edit a profile's Privacy Policy. A stopped profile applies and verifies that policy before Chromium is spawned. With **Strict Network Guard** plus an explicit proxy, an unreachable proxy blocks launch instead of silently falling back to browsing.

Then open **Privacy**:

1. choose the profile;
2. run local preflight to inspect stored Chromium policy and proxy endpoint reachability;
3. launch the profile with policy enforcement if it is stopped;
4. use **External Verification Lab** to open IP, WebRTC, DNS/IPv6 and fingerprint tests inside that exact profile;
5. treat unexpected real-network exposure reported by those remote tests as a critical privacy issue.

Local checks intentionally do not claim that a proxy is anonymous or that no leak exists. Public IP, DNS, IPv6 and WebRTC exposure must be confirmed from a remote website's point of view.

See [`docs/m5-privacy-leak-guard.md`](docs/m5-privacy-leak-guard.md) for the M5 threat model and enforcement lifecycle. Fingerprint baseline/history details remain in [`docs/m4-per-profile-fingerprints.md`](docs/m4-per-profile-fingerprints.md).

## Per-profile data model

```text
~/.cache/dravyn/
├── profiles/
│   └── <profile-id>/
│       ├── profile.json        # browser + network + privacy policy
│       └── user-data/          # Chromium profile and applied Preferences
├── fingerprints/
│   └── <profile-id>/
│       ├── baseline.json
│       ├── latest.json
│       └── history/
└── runtime/
```

## Repository layout

```text
apps/
  desktop/              Tauri 2 + React/TypeScript operations console

crates/
  dravyn-cli/           Command-line interface
  dravyn-core/          Chromium detection and fail-closed runtime orchestration
  dravyn-common/        Shared types and DRAVYN_HOME workspace resolution
  dravyn-profile/       Persistent profile domain + storage
  dravyn-network/       Direct/proxy configuration + endpoint preflight
  dravyn-fingerprint/   Per-profile baseline, history and drift engine
  dravyn-privacy/       Per-profile defensive privacy policy + Chromium preference enforcement

browser/                Chromium configuration of record + future reviewed privacy patch sets
scripts/                Development helpers
docs/                   Architecture, roadmap, Chromium and desktop workflow docs
```

## Local/generated data and Git

Large or machine-generated data never belongs in the repository. The root `.gitignore` excludes Chromium source/build trees, nested Rust/Tauri `target/` trees, frontend `dist/`, Node dependencies, pnpm stores, local Dravyn profile/fingerprint/runtime/log/cache data, test reports, and local desktop lockfiles.

Source-of-truth files such as `Cargo.toml`, `package.json`, Tauri configuration, Chromium configuration and application source/assets remain tracked.

## Status

Early development (`0.1.0-dev`). APIs, profile/fingerprint/privacy schemas and desktop UI may change without notice.
