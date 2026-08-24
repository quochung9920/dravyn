# M3 - Professional Desktop, Network and Privacy Audit

M3 upgrades Dravyn from the initial profile-manager UI into a more complete local browser operations console.

## Delivered in M3

- redesigned Dashboard and navigation
- searchable/sortable profile management
- profile clone, launch, stop, reset and delete workflows
- dedicated Network page with local proxy endpoint reachability checks
- dedicated Privacy page with a local-only fingerprint/privacy inspector
- local consistency checks for browser-exposed surfaces
- Diagnostics page for Chromium, WSLg/display, profile storage and runtime health
- Settings page showing the effective workspace and Chromium binary
- stronger ignore rules for build output, caches, runtime state and large Chromium workspaces

## Local privacy inspector

The Privacy page can open a generated audit document inside the selected Dravyn Chromium profile. The document is written under:

```text
$DRAVYN_HOME/runtime/privacy-audit/index.html
```

It observes browser-exposed values such as User-Agent, platform, Client Hints, language, timezone, screen, Canvas rendering, WebGL, AudioContext, local storage, permissions, WebRTC candidate types, CPU concurrency and touch capability.

The page performs a small set of consistency checks and displays a score. It does not upload results and it does not modify, randomize, or spoof browser values.

This is intentionally a privacy/QA diagnostic feature, not a bypass feature. Dravyn does not provide fingerprint impersonation, CAPTCHA/KYC bypass, anti-fraud evasion or identity impersonation.

## Network probe

For profiles configured with an explicit proxy, M3 can resolve the proxy hostname and attempt a TCP connection to the configured host/port with a short timeout. This verifies endpoint reachability only. It does not prove that proxy credentials are valid and it does not make anonymity claims.

## Generated files and Git

The root `.gitignore` explicitly excludes:

- all Rust/Tauri `target/` trees
- frontend `dist/`, Vite cache and TypeScript build metadata
- Node `node_modules/` and pnpm stores
- Dravyn runtime/profile/log/cache state
- Chromium source/build workspaces
- test reports and temporary files
- `apps/desktop/pnpm-lock.yaml` and `apps/desktop/src-tauri/Cargo.lock`, which are generated locally under the current desktop development policy

Source-of-truth configuration remains tracked, including `Cargo.toml`, `package.json`, Tauri configuration, browser configuration and application source/assets.

## Run M3

```bash
cd ~/projects/dravyn
git pull origin main
cargo install --path crates/dravyn-cli --force
dravyn desktop
```

Or during direct desktop development:

```bash
cd ~/projects/dravyn/apps/desktop
pnpm install
pnpm tauri dev
```
