# M7 - Production Readiness & Assurance

M7 hardens the M6 commercial privacy workflow around data safety, startup clarity, operator awareness, and repeatable validation.

## Goals

M7 is intentionally not a fingerprint-spoofing milestone. Its purpose is to make Dravyn safer to operate and easier to trust as a local privacy engineering product:

```text
reliable metadata
      +
recoverable writes
      +
versioned schemas
      +
clear system health
      +
repeatable validation
      +
operator-visible state changes
```

## Profile metadata recovery

Every persisted profile now contains a `schema_version`.

Profile writes use this lifecycle:

```text
serialize validated profile
        ↓
write profile.json.tmp
        ↓
flush + fsync temporary file
        ↓
copy existing profile.json → profile.json.bak
        ↓
rename temporary → profile.json
        ↓
fsync profile directory
```

If the primary `profile.json` is syntactically corrupt and a valid last-known-good `profile.json.bak` exists, `dravyn-profile` automatically restores the backup before returning the profile.

This protects metadata such as profile identity, browser configuration, network route and privacy policy. It does not back up the full Chromium `user-data` directory.

### Recovery boundary

A backup is deliberately the previous valid profile metadata, not a transaction log. If the newest primary file becomes corrupt after an update, recovery can roll metadata back to the preceding valid version. That behavior is safer than silently inventing or partially reconstructing configuration.

Unsupported future profile schema versions fail clearly rather than being loaded as if compatible.

## M7 Assurance Center

The desktop now starts through `ProductionApp`, which wraps the M6 commercial interface with a lightweight production-assurance layer.

It provides:

- first-run assurance onboarding;
- global Healthy / Review / Critical state;
- Chromium/runtime readiness summary;
- counts for critical verification, fingerprint drift and profiles needing verification;
- system diagnostic view without leaving the current workflow;
- a local activity timeline derived from runtime, fingerprint and verification state transitions;
- explicit wording about what local checks can and cannot prove.

The activity timeline is UI-local operational history. It is not a security audit log and is not presented as tamper-proof evidence.

## Health semantics

The M7 shell deliberately does not average unrelated signals into a misleading score.

`Critical` is raised when, for example:

- Dravyn Chromium is unavailable;
- a system diagnostic reports an error;
- a profile has a current critical remote-verification result.

`Review` is used when there are warnings, fingerprint drift, or profiles that do not yet have healthy current verification evidence.

`Healthy` requires no current shell-level critical/review condition. It still does not mean anonymity or guaranteed non-trackability.

## Onboarding workflow

The first-run overlay teaches the recommended order:

1. validate Dravyn Chromium/runtime readiness;
2. create a profile with browser route + privacy policy;
3. establish the local fingerprint baseline;
4. complete Public IP, WebRTC, DNS and IPv6 remote verification.

This keeps the product usable for operators who do not need to understand the Rust/Tauri/Chromium architecture.

## Repeatable validation

M7 adds:

```bash
bash scripts/validate.sh
```

The script runs:

```text
cargo fmt --check
cargo check --workspace
cargo test --workspace
pnpm install
pnpm typecheck
pnpm build
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
```

The desktop package also exposes:

```bash
pnpm check
pnpm build
```

GitHub Actions now has explicit timeouts, a separate TypeScript check, Rust backtraces for tests, and the existing workspace/Tauri/shell stages.

## What M7 does not claim

M7 still does not provide:

- an internet-facing Dravyn-owned verification service;
- OS-level firewall or process egress enforcement;
- cryptographic release signing using production keys;
- automatic updater infrastructure;
- OS-keychain-backed proxy credentials;
- a Chromium regression farm;
- fake or randomized device identity signals;
- CAPTCHA/KYC/anti-fraud bypass capability.

These require infrastructure, platform-specific security integration, or a deeper reviewed Chromium/network layer. They should not be represented as complete until they are actually deployed and verified.

## Run

M7 remains an application/reliability milestone and does not require rebuilding the existing Dravyn Chromium binary:

```bash
cd ~/projects/dravyn
git pull origin main

bash scripts/validate.sh

cd apps/desktop
pnpm tauri dev
```

For a faster development start after dependencies are already installed:

```bash
cd ~/projects/dravyn/apps/desktop
pnpm tauri dev
```
