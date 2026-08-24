# Dravyn

Dravyn is a local-first browser-core research and development project focused on Chromium runtime management, isolated browser profiles, explicit network policy, privacy diagnostics, compatibility testing, and authorized browser automation.

## Current milestone

**M8 - Network Shield & Continuous Assurance** (M0-M7 complete)

M8 extends the M7 production-readiness foundation with continuous per-profile route-health supervision and fresher assurance semantics:

- proxy preflight now uses a bounded total timeout budget across a limited set of resolved addresses
- Monitor and Strict proxy profiles get a continuous Network Shield supervisor while Dravyn Desktop is running
- Strict mode trips after three consecutive proxy endpoint failures and terminates the affected Chromium profile
- strict manual-proxy launches disable QUIC to narrow the UDP transport surface while the route guard is active
- running profiles reject browser/network/privacy configuration changes until stopped; name, notes and tags remain editable
- profiles expose verification freshness independently from historical verification state
- global Assurance Center shows shield state, last check, consecutive failures, fingerprint drift, verification due state and system readiness
- activity timeline records Network Shield and verification-freshness transitions
- existing M7 profile schema/recovery, M6 verification journal and M4 fingerprint baseline/history remain intact

Earlier commercial-assurance capabilities remain available:

- professional Overview, Profiles, Privacy, Fingerprints, Verification, Network, Diagnostics and Settings workspaces
- per-profile privacy policy schema/version lifecycle
- Strict proxy preflight fail-closed before launch
- per-profile fingerprint baseline/history/drift
- per-profile Verification Journal with Pass / Warning / Critical / Inconclusive results
- core external verification coverage for Public IP, WebRTC, DNS and IPv6
- BrowserLeaks/EFF/AmIUnique launch inside the exact selected Dravyn profile
- cross-profile stable-surface comparison for defensive privacy diagnostics
- recoverable versioned profile metadata with last-known-good `profile.json.bak`

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

M8 changes the application/runtime assurance layer and reuses `$DRAVYN_HOME/chromium/src/out/Dravyn/chrome`; it does not require rebuilding Chromium.

## Recommended validation

Before considering a local checkout ready:

```bash
cd ~/projects/dravyn
bash scripts/validate.sh
```

This checks Rust formatting, workspace compilation/tests, desktop dependencies, TypeScript, the Vite production build and the Tauri backend.

## Continuous-assurance workflow

A healthy profile is not determined by one synthetic score. Dravyn keeps the evidence layers separate:

```text
Profile metadata integrity
      +
Profile isolation
      +
Privacy policy enforcement
      +
Network Shield route health
      +
Fingerprint stability/consistency
      +
Fresh remote verification evidence
```

Recommended operating order:

1. Confirm Dravyn Chromium and system diagnostics are ready.
2. Create or edit a profile and choose its browser route + privacy policy.
3. For proxy profiles, choose Off / Monitor / Strict Network Guard.
4. Launch the profile; Strict mode still performs fail-closed preflight before Chromium starts.
5. While the desktop app remains running, Network Shield continuously watches Monitor/Strict proxy endpoint health.
6. Open **Privacy** and review local policy/preflight state.
7. Open **Fingerprints** and establish/review the local baseline.
8. Open **Verification** and run the core external tests in the exact selected profile.
9. Record Public IP, WebRTC, DNS and IPv6 results as Pass, Warning, Critical or Inconclusive.
10. Re-verify when the configured freshness window expires or after meaningful browser, network, OS, display or policy changes.

A reachable proxy endpoint is not proof of no leak. Network Shield endpoint health is not an OS firewall. A stable fingerprint is not proof of anonymity. A local policy match is not proof of what a remote website observes.

See [`docs/m8-network-shield.md`](docs/m8-network-shield.md), [`docs/m7-production-readiness.md`](docs/m7-production-readiness.md), [`docs/m6-commercial-privacy.md`](docs/m6-commercial-privacy.md), [`docs/m5-privacy-leak-guard.md`](docs/m5-privacy-leak-guard.md), and [`docs/m4-per-profile-fingerprints.md`](docs/m4-per-profile-fingerprints.md).

## Network Shield behavior

For a running proxy profile:

```text
Monitor
  proxy health failure -> report Degraded
  browser keeps running

Strict
  proxy health failure 1/3 -> Degraded
  proxy health failure 2/3 -> Degraded
  proxy health failure 3/3 -> Tripped -> terminate profile process
```

A successful probe resets the consecutive failure count.

The current defaults are a 3 second check interval, 900 ms health-check timeout and three consecutive failures before Strict mode trips. See the M8 document for the exact boundary and manual test procedure.

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
    └── profile-processes/      # local process records
```

Network Shield supervisor state is intentionally in-memory runtime state. A tripped state is visible during the desktop session; durable external evidence remains in the Verification Journal.

The metadata backup is intentionally not a full Chromium user-data backup. Resetting browser data keeps fingerprint/verification history; deleting a profile removes its browser data, fingerprint data and verification journal.

## Repository layout

```text
apps/
  desktop/              Tauri 2 + React/TypeScript commercial operations console

crates/
  dravyn-cli/           Command-line interface
  dravyn-core/          Chromium runtime + continuous Network Shield orchestration
  dravyn-common/        Shared types and DRAVYN_HOME workspace resolution
  dravyn-profile/       Persistent profile domain + schema/recovery + policy lifecycle
  dravyn-network/       Direct/proxy configuration + bounded endpoint preflight
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

M8 adds a meaningful continuous process kill-switch for strict proxy health, but it still does not claim anonymity or eliminate every browser fingerprint. Dravyn does not yet provide an internet-facing Dravyn-owned verification endpoint, kernel/OS-level process egress firewall, production signing/updater infrastructure, OS-keychain-backed proxy credentials or a Chromium regression farm.

Those capabilities require deployed infrastructure, platform-specific security work and deeper reviewed Chromium/network integration. They should not be represented as complete until they are actually implemented and verified.

## Status

Early development (`0.1.0-dev`). M8 improves runtime route assurance substantially, but release qualification, remote automated verification and OS/Chromium-level hardening remain future milestones.
