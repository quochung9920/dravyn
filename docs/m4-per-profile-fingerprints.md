# M4 - Per-Profile Fingerprint Engine

M4 makes fingerprint state a first-class part of each Dravyn profile. The goal is stable, inspectable privacy diagnostics for authorized QA and browser compatibility work, not fingerprint impersonation.

## What changed

Every profile now has an independent fingerprint record with:

- first-capture baseline
- latest snapshot
- local history (up to 50 snapshots)
- consistency score
- stable-surface drift count
- current review issues
- surface count
- last capture time

The desktop Profile cards expose the current score/state and the new **Fingerprints** page provides a dedicated profile selector, timeline, latest drift comparison and baseline controls.

## Storage

Fingerprint data lives outside the Git repository under `$DRAVYN_HOME` (default `~/.cache/dravyn`):

```text
$DRAVYN_HOME/
├── profiles/
│   └── <profile-id>/
│       ├── profile.json
│       └── user-data/
└── fingerprints/
    └── <profile-id>/
        ├── baseline.json
        ├── latest.json
        └── history/
            └── <snapshot-id>.json
```

Deleting a profile also deletes its fingerprint directory. Resetting browser cookies/cache/site data intentionally preserves fingerprint history so environment changes can still be audited.

## Capture flow

When Dravyn Desktop starts, it binds an HTTP listener only to `127.0.0.1` on an ephemeral port. The process creates an ephemeral session token.

When the user chooses **Run profile audit**:

1. Dravyn resolves the selected profile and its exact Chromium `--user-data-dir`.
2. The browser opens a local audit page served by the loopback listener.
3. The page observes browser-visible surfaces inside that profile.
4. The page posts the result back to the same loopback process using the session token.
5. The Rust fingerprint engine validates and stores the snapshot under that profile ID.
6. The first successful capture automatically creates the baseline.
7. Later captures compare stable surfaces with that profile's baseline and report drift.

No audit result is sent to an external service.

## Stable vs dynamic observations

Not every observed value should be treated as identity-stable. M4 separates stable surfaces from dynamic privacy state.

Baseline-compared examples include:

- User-Agent
- navigator.platform
- User-Agent Client Hints
- language/languages
- timezone
- screen dimensions / color depth / DPR
- hardware concurrency
- device memory
- touch capability
- Canvas render hash
- WebGL vendor/renderer
- audio sample rate

Recorded but not baseline-compared examples include:

- current color scheme
- reduced-motion preference
- cookie availability
- current permission state
- local storage availability
- WebRTC candidate types
- AudioContext runtime state

This avoids treating normal runtime changes as fingerprint drift.

## Consistency score

The snapshot starts at 100. M4 currently applies deterministic penalties for:

- local consistency/privacy issues reported by the audit
- stable surfaces that differ from the profile baseline

The score is a Dravyn diagnostic indicator, not a claim that a profile is anonymous or undetectable.

## Baseline lifecycle

The first successful audit creates the baseline automatically.

If a legitimate environment change is intentional (for example a display/GPU/OS update), the user can review the new snapshot and choose **Set latest as baseline**. This is explicit because silently changing a baseline would make drift monitoring meaningless.

After replacing a baseline, run another audit to create a fresh post-change comparison snapshot.

## Safety boundary

M4 deliberately does not:

- randomize Canvas/WebGL/audio values
- inject fake hardware characteristics
- impersonate another device or operating system
- provide CAPTCHA/KYC bypass
- provide anti-fraud evasion
- claim that proxy reachability equals anonymity

The supported purpose is per-profile isolation, privacy diagnostics, stability monitoring, leak review, compatibility testing and authorized browser automation.

## Run

No Chromium rebuild is required for M4 because these changes are in the Dravyn application layer.

```bash
cd ~/projects/dravyn
git pull origin main

cd apps/desktop
pnpm install
pnpm tauri dev
```

Then open **Fingerprints**, select a profile and choose **Run profile audit**.
