# Dravyn

Dravyn is a local-first browser-core research and development project focused on Chromium runtime management, isolated browser profiles, explicit network policy, privacy diagnostics, compatibility testing, and authorized browser automation.

## Current milestone

**M7 - Production Readiness & Assurance** (M0-M6 complete)

M7 hardens the M6 commercial privacy workflow around recoverability, schema safety, operator awareness and repeatable validation:

- profile metadata now carries an explicit profile schema version
- validated profile writes keep a last-known-good `profile.json.bak`
- profile metadata writes flush/sync before replacement
- syntactically corrupt primary profile metadata automatically recovers from a valid backup when possible
- existing privacy policy versioning remains separate from profile schema versioning
- new desktop production-assurance shell around the M6 UI
- first-run onboarding for runtime readiness, profile setup, fingerprint baseline and remote verification
- global Healthy / Review / Critical assurance state without collapsing unrelated evidence into one score
- Assurance Center with Chromium/system readiness, verification state, fingerprint drift and recent state transitions
- local UI activity timeline for runtime, fingerprint and verification changes
- explicit `pnpm check` desktop typecheck command
- one-command local validation with `bash scripts/validate.sh`
- CI stages now include explicit TypeScript checking, timeouts and Rust test backtraces

M6 capabilities remain intact:

- professional Overview, Profiles, Privacy, Fingerprints, Verification, Network, Diagnostics and Settings workspaces
- per-profile privacy policy schema/version lifecycle
- Strict proxy Network Guard fail-closed on endpoint preflight failure
- per-profile fingerprint baseline/history/drift
- per-profile Verification Journal with Pass / Warning / Critical / Inconclusive results
- core external verification coverage for Public IP, WebRTC, DNS and IPv6
- BrowserLeaks/EFF/AmIUnique launch inside the exact selected Dravyn profile
- cross-profile stable-surface comparison for defensive privacy diagnostics

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

M7 is an application/reliability milestone and reuses `$DRAVYN_HOME/chromium/src/out/Dravyn/chrome`; it does not require rebuilding Chromium.

## Recommended validation

Before considering a local checkout ready:

```bash
cd ~/projects/dravyn
bash scripts/validate.sh
```

This checks Rust formatting, workspace compilation/tests, desktop dependencies, TypeScript, the Vite production build and the Tauri backend.

## Production-assurance workflow

A healthy profile is not determined by one synthetic score. Dravyn keeps the evidence layers separate:

```text
Profile metadata integrity
      +
Profile isolation
      +
Privacy policy enforcement
      +
Fingerprint stability/consistency
      +
Remote verification journal
```

Recommended operating order:

1. Confirm Dravyn Chromium and system diagnostics are ready.
2. Create or edit a profile and choose its browser route + privacy policy.
3. Stop/relaunch after privacy-policy changes so policy is applied before browsing.
4. Open **Privacy** and review local policy/preflight state.
5. Open **Fingerprints** and establish/review the local baseline.
6. Open **Verification** and run the core external tests in the exact selected profile.
7. Record Public IP, WebRTC, DNS and IPv6 results as Pass, Warning, Critical or Inconclusive.
8. Treat any unexpected real public-network exposure as critical even when other indicators are green.
9. Re-verify after meaningful browser, network, OS, display or policy changes.

A reachable proxy endpoint is not proof of no leak. A stable fingerprint is not proof of anonymity. A local policy match is not proof of what a remote website observes.

See [`docs/m7-production-readiness.md`](docs/m7-production-readiness.md), [`docs/m6-commercial-privacy.md`](docs/m6-commercial-privacy.md), [`docs/m5-privacy-leak-guard.md`](docs/m5-privacy-leak-guard.md), and [`docs/m4-per-profile-fingerprints.md`](docs/m4-per-profile-fingerprints.md).

## Per-profile data model

```text
~/.cache/dravyn/
├── profiles/
│   └── <profile-id>/
│       ├── profile.json        # current versioned profile metadata
│       ├── profile.json.bak    # previous valid metadata, when available
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

The metadata backup is intentionally not a full Chromium user-data backup. Resetting browser data keeps fingerprint/verification history; deleting a profile removes its browser data, fingerprint data and verification journal.

## Repository layout

```text
apps/
  desktop/              Tauri 2 + React/TypeScript commercial operations console

crates/
  dravyn-cli/           Command-line interface
  dravyn-core/          Chromium detection and fail-closed runtime orchestration
  dravyn-common/        Shared types and DRAVYN_HOME workspace resolution
  dravyn-profile/       Persistent profile domain + schema/recovery + policy lifecycle
  dravyn-network/       Direct/proxy configuration + endpoint preflight
  dravyn-fingerprint/   Per-profile baseline, history and drift engine
  dravyn-privacy/       Defensive privacy policy + Chromium preference enforcement
  dravyn-verification/  Per-profile remote-verification journal and latest-result summary

browser/                Chromium configuration of record + reviewed privacy patch area
scripts/                Development and validation helpers
docs/                   Architecture, roadmap, Chromium and desktop workflow docs
```

## Local/generated data and Git

Large or machine-generated data never belongs in the repository. The root `.gitignore` excludes Chromium source/build trees, Rust/Tauri `target/`, frontend `dist/`, Node dependencies, pnpm stores, local profiles/fingerprints/verifications/runtime/log/cache data, test reports and local desktop lockfiles.

Source-of-truth files such as `Cargo.toml`, application source, Tauri configuration and browser configuration remain tracked.

## Current boundary

M7 substantially improves application reliability and product operation, but it still does not claim anonymity or eliminate every browser fingerprint. Dravyn does not yet provide an internet-facing Dravyn-owned verification endpoint, OS-level process egress firewall, production signing/updater infrastructure, OS-keychain-backed proxy credentials or a Chromium regression farm.

Those capabilities require deployed infrastructure, platform-specific security work and deeper reviewed Chromium/network integration. They should not be represented as complete until they are actually implemented and verified.

## Status

Early development (`0.1.0-dev`). M7 moves the product closer to production readiness, but release qualification and deeper network/Chromium hardening remain future milestones.
