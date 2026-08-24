# Dravyn

Dravyn is a local-first browser-core research and development project focused on Chromium runtime management, isolated browser profiles, explicit network policy, privacy diagnostics, compatibility testing, and authorized browser automation.

## Current milestone

**M6 - Commercial Privacy Operations** (M0-M5 complete)

M6 turns the M4/M5 fingerprint and privacy foundations into a more product-like assurance workflow:

- redesigned commercial desktop UI with clearer Overview, Profiles, Privacy, Fingerprints, Verification, Network, Diagnostics and Settings workspaces
- command palette (`Ctrl+K`) and unified profile selection/actions
- per-profile privacy policy schema/version metadata
- policy version automatically increments when privacy semantics change
- local privacy policy still applies and verifies before a stopped profile launches
- Strict proxy Network Guard remains fail-closed on endpoint preflight failure
- per-profile fingerprint baseline/history/drift from M4 remains separate from privacy enforcement
- new per-profile Verification Journal with up to 100 local records
- Pass / Warning / Critical / Inconclusive result states for remote tests
- latest-result verification summary attached to every profile
- Privacy Center health now combines local policy state, route preflight, fingerprint state and verification journal state without pretending these are the same signal
- External Verification Center launches BrowserLeaks IP/WebRTC/DNS/IPv6/Canvas/WebGL, EFF Cover Your Tracks and AmIUnique inside the exact selected profile
- cross-profile stable-surface comparison helps privacy engineers understand similarity without claiming whether a third party will correlate profiles
- verification data is stored outside Git under `$DRAVYN_HOME/verifications`

Dravyn's scope is defensive privacy engineering, local profile isolation, network control, compatibility testing and authorized browser automation. It does not randomize or spoof device identity to impersonate another device, and it is not intended to bypass CAPTCHA, KYC, anti-fraud or abuse controls.

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
dravyn chromium bootstrap
dravyn chromium configure
dravyn chromium build
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

M6 changes the Dravyn application layer and reuses `$DRAVYN_HOME/chromium/src/out/Dravyn/chrome`; it does not require rebuilding Chromium.

## Commercial privacy workflow

A healthy profile is not determined by one score. Dravyn keeps four signals visibly separate:

```text
Profile isolation
      +
Privacy policy enforcement
      +
Fingerprint stability/consistency
      +
Remote verification journal
```

Recommended workflow:

1. Create or edit a profile and select its browser route and privacy policy.
2. Stop/relaunch after policy changes so policy is applied before browsing.
3. Open **Privacy** and run local preflight.
4. Open **Fingerprints** and establish/review the local baseline.
5. Open **Verification** and run the core external tests inside the exact selected profile.
6. Record the remote result as Pass, Warning, Critical or Inconclusive, including expected/observed values where useful.
7. Treat any unexpected real public address or other critical remote exposure as critical even when local policy/fingerprint scores are high.
8. Re-verify after meaningful browser, network, OS, display or privacy-policy changes.

A reachable proxy endpoint is not proof of no leak. A stable fingerprint is not proof of anonymity. A local policy match is not proof of what a remote website observes.

See [`docs/m6-commercial-privacy.md`](docs/m6-commercial-privacy.md), [`docs/m5-privacy-leak-guard.md`](docs/m5-privacy-leak-guard.md), and [`docs/m4-per-profile-fingerprints.md`](docs/m4-per-profile-fingerprints.md).

## Per-profile data model

```text
~/.cache/dravyn/
├── profiles/
│   └── <profile-id>/
│       ├── profile.json        # browser + network + versioned privacy policy
│       └── user-data/          # Chromium profile and applied Preferences
├── fingerprints/
│   └── <profile-id>/
│       ├── baseline.json
│       ├── latest.json
│       └── history/
├── verifications/
│   └── <profile-id>/
│       └── history/            # external verification journal, max 100
└── runtime/
```

Deleting a profile removes its browser data, fingerprint data and verification journal. Resetting browser data intentionally keeps fingerprint/verification history so changes can be audited.

## Repository layout

```text
apps/
  desktop/              Tauri 2 + React/TypeScript commercial operations console

crates/
  dravyn-cli/           Command-line interface
  dravyn-core/          Chromium detection and fail-closed runtime orchestration
  dravyn-common/        Shared types and DRAVYN_HOME workspace resolution
  dravyn-profile/       Persistent profile domain + policy version lifecycle
  dravyn-network/       Direct/proxy configuration + endpoint preflight
  dravyn-fingerprint/   Per-profile baseline, history and drift engine
  dravyn-privacy/       Defensive privacy policy + Chromium preference enforcement
  dravyn-verification/  Per-profile remote-verification journal and latest-result summary

browser/                Chromium configuration of record + reviewed privacy patch area
scripts/                Development helpers
docs/                   Architecture, roadmap, Chromium and desktop workflow docs
```

## Local/generated data and Git

Large or machine-generated data never belongs in the repository. The root `.gitignore` excludes Chromium source/build trees, Rust/Tauri `target/`, frontend `dist/`, Node dependencies, pnpm stores, local profiles/fingerprints/verifications/runtime/log/cache data, test reports and local desktop lockfiles.

Source-of-truth files such as `Cargo.toml`, application source, Tauri configuration and browser configuration remain tracked.

## Current boundary

M6 is a substantial commercial-product foundation, not a claim that Dravyn can prove anonymity or eliminate every browser fingerprint. Remote verification is operator-reviewed and stored locally; Dravyn does not yet run its own internet-facing verification service or OS-level egress firewall. Those capabilities require separate infrastructure and a deeper Chromium/network integration layer.

## Status

Early development (`0.1.0-dev`). APIs and schemas may still change while the product is hardened.
