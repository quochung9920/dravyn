# M6 - Commercial Privacy Operations

M6 builds a product-oriented assurance layer on top of M4 fingerprint observability and M5 privacy enforcement.

## Design principle

Dravyn deliberately keeps these concepts separate:

```text
Isolation != privacy enforcement
Privacy enforcement != remote verification
Fingerprint stability != anonymity
Proxy reachability != no leak
```

The UI can combine them into a health view, but it does not erase their individual evidence or claim more than each layer can prove.

## Per-profile assurance model

Each profile owns:

```text
Profile
├── browser configuration
├── network configuration
├── privacy policy
│   ├── schema version
│   ├── policy version
│   └── verification freshness metadata
├── Chromium user-data
├── fingerprint baseline/history
└── verification journal
```

### Policy version lifecycle

New profiles start at policy version 1.

When privacy semantics change, `dravyn-profile` increments the profile's privacy policy version. Name, notes, tags and unrelated browser edits do not increment it.

This provides a stable basis for later release qualification and for associating remote verification records with the policy that was in effect when they were reviewed.

## Verification Journal

`dravyn-verification` stores external verification observations outside the repository:

```text
$DRAVYN_HOME/verifications/<profile-id>/history/<record-id>.json
```

Each record contains:

- test identifier
- Pass / Warning / Critical / Inconclusive result
- optional expected value
- optional observed value
- operator notes
- source URL
- optional Chromium version metadata
- privacy policy version
- timestamp

The store keeps up to the latest 100 records per profile.

A summary is calculated from the newest record for each test rather than all historical failures. This allows a critical issue to become healthy after it is intentionally fixed and re-verified while retaining the full audit trail.

## Verification tests

The commercial desktop exposes these external tests inside the exact selected Dravyn profile:

### Core network verification

- BrowserLeaks Public IP
- BrowserLeaks WebRTC
- BrowserLeaks DNS
- BrowserLeaks IPv6 view

### Fingerprint perspective

- BrowserLeaks Canvas
- BrowserLeaks WebGL
- EFF Cover Your Tracks
- AmIUnique

Third-party tests receive the browser/network characteristics needed to produce their results. They are external services and are separate from Dravyn's local-only fingerprint capture endpoint.

## Health state

The desktop distinguishes:

- **Healthy** - no current critical/warning verification result and local signals are in an acceptable state
- **Review** - warning, inconclusive fingerprint/privacy state, or drift needs review
- **Critical** - a current verification result or strict local preflight indicates a critical problem
- **Unverified / pending** - insufficient external evidence exists yet

A critical result is never averaged away by a high fingerprint score.

## Cross-profile comparison

Fingerprint Center can compare stable observed surfaces from two profile snapshots and show the percentage that are equal.

This is intentionally a privacy diagnostic only. It helps answer:

> "How similar are the browser-visible stable surfaces I observed for these profiles?"

It does not answer or claim:

> "Will a third-party anti-abuse system correlate these profiles?"

Dravyn does not use this feature to generate fake identities or tune profiles for anti-fraud evasion.

## Commercial desktop UX

M6 replaces the previous prototype shell with a new operations experience:

- grouped navigation: Operate / Assure / System
- overview health cards
- profile health cards with separate Network / Privacy / Fingerprint / Verification facts
- dedicated Privacy Center
- dedicated Fingerprint Center
- dedicated Verification Center
- network preflight workspace
- runtime diagnostics
- `Ctrl+K` command palette
- responsive layouts for common desktop widths
- clearer Critical / Review / Healthy states
- unified create/edit profile modal with browser, network and privacy configuration in one workflow

## Safety boundary

M6 supports defensive privacy engineering and authorized QA. It does not:

- generate fake hardware identities;
- randomize Canvas/WebGL/audio signals to impersonate devices;
- promise an undetectable browser;
- bypass CAPTCHA/KYC/anti-fraud controls;
- convert third-party anti-bot scores into an evasion target.

## Current limitations

M6 does not yet provide:

- an internet-facing Dravyn-owned verification endpoint;
- OS-level firewall/egress enforcement around the Chromium process;
- deterministic DNS-path proof from the local desktop alone;
- signed release/update infrastructure;
- OS keychain-backed proxy credentials;
- a full browser-integration regression farm across supported Chromium versions.

Those require infrastructure and deeper system/Chromium integration beyond this application-layer milestone.

## Run

No Chromium rebuild is required for M6:

```bash
cd ~/projects/dravyn
git pull origin main

cd apps/desktop
pnpm install
pnpm tauri dev
```

Then use:

```text
Profiles -> configure workspace
Privacy -> run policy/preflight review
Fingerprints -> run local audit and review drift
Verification -> open remote tests and record results
```
