# M8 - Network Shield & Continuous Assurance

M8 extends the M7 production-readiness foundation with continuous runtime route-health supervision for proxy profiles and freshness-aware assurance UX.

## Goal

M7 could fail closed before launch when a Strict proxy endpoint was unavailable. M8 adds an additional runtime layer:

```text
Profile launch
    ↓
Strict preflight
    ↓
Chromium starts with profile policy
    ↓
Network Shield arms
    ↓
proxy endpoint health checked continuously
    ↓
3 consecutive failures?
    ├── no  → continue monitoring
    └── yes → terminate the profile process
```

This is deliberately described as a **process-level kill-switch**. It is not represented as an OS firewall and it does not prove what a remote website observes.

## Network Shield modes

The existing per-profile `network_guard` policy drives the supervisor:

- `off` - no proxy health supervisor is armed.
- `monitor` - proxy endpoint health is checked while the profile runs; failures are reported but the browser is not terminated.
- `strict` - the same continuous checks run, and three consecutive failures trip the shield and terminate the profile.

Only proxy profiles require this supervisor. Direct profiles report the shield as off/not required.

## Why three failures

A single failed TCP probe can be caused by transient scheduling, DNS, proxy load or a short network transition. M8 therefore requires three consecutive failed endpoint checks before Strict mode trips.

Current defaults:

```text
probe interval       3 seconds
probe timeout        900 ms
failure threshold    3 consecutive failures
```

A successful check resets the consecutive failure counter to zero.

## Bounded preflight

`dravyn-network` now limits a proxy probe to at most four resolved addresses and divides a single timeout budget across attempts. This prevents a multi-address hostname from turning a nominal 1.5 second preflight into a long sequence of independent 1.5 second waits.

The probe still proves only that a TCP endpoint accepted a connection.

It does not prove:

- the website-visible public IPv4 address;
- whether a native IPv6 path is exposed;
- which resolver a remote DNS leak test observes;
- WebRTC candidate behavior;
- anonymity or unlinkability.

Those remain external-verification evidence.

## Strict runtime hardening

Strict manual-proxy profiles add `--disable-quic` when Dravyn launches Chromium. QUIC is UDP-based; disabling it narrows the transport surface for the strict manual-proxy mode. Dravyn continues to apply the existing defensive WebRTC policy separately.

This launch hardening is not treated as proof of no leak. Public IP, DNS, IPv6 and WebRTC remain independent verification checks.

## Supervisor lifecycle

`dravyn-core::network_shield` owns an in-process supervisor registry.

For every running proxy profile with Monitor/Strict guard it tracks:

- profile ID;
- guard mode;
- endpoint label;
- policy version;
- current shield state;
- latest check time;
- consecutive failures;
- configured trip threshold;
- operator-facing status message.

States:

```text
off
standby
monitoring
healthy
degraded
tripped
```

A tripped state remains visible after automatic termination until the profile is explicitly launched again or the state is otherwise reconciled by a new session.

## Restart/reconciliation behavior

`list_profiles` and `network_shield_status` reconcile running profiles with the supervisor. Therefore, if the Tauri application restarts while a previously launched Dravyn Chromium profile remains alive, opening/refreshing the desktop re-arms the applicable proxy supervisor.

If Strict mode cannot arm after a new launch, Dravyn stops that profile rather than silently continuing without the requested runtime guard.

## Runtime edit safety

M8 prevents browser/network/privacy configuration from being changed while a profile is running. Name, notes and tags remain editable.

This avoids the misleading state where the saved profile says one route/policy while the live Chromium process is still using the old launch configuration.

## Verification freshness

Every `ProfileView` now exposes whether its current verification evidence is inside the profile's configured freshness window.

A verification result can therefore be historically healthy but still require review because it is too old for the active privacy policy.

The M8 Assurance Center treats missing, review, critical **or expired** verification as requiring attention.

## UX

The global Assurance Center now includes a Network Shield section for proxy profiles with:

- current shield state;
- Monitor/Strict mode;
- endpoint;
- most recent check time;
- consecutive failure counter;
- detailed status message.

The activity timeline records shield state transitions and verification freshness changes alongside runtime, fingerprint and diagnostic events.

## Safety boundary

M8 is defensive privacy engineering and authorized QA. It does not:

- create fake hardware/browser identities;
- randomize fingerprint surfaces for anti-fraud evasion;
- claim that a reachable proxy means no leak;
- claim that the process kill-switch is an OS firewall;
- claim anonymity or undetectability.

## Remaining production work

M8 still does not provide:

- kernel/OS-level egress firewalling bound to Chromium processes;
- a deployed Dravyn-owned internet verification service;
- deterministic remote DNS-path proof from local state alone;
- OS-keychain-backed proxy credentials;
- signed installer/updater infrastructure;
- a multi-version Chromium privacy regression farm.

Those are separate infrastructure/platform milestones and must be implemented and verified before Dravyn claims those capabilities.

## Validation

Run the existing full local validation:

```bash
cd ~/projects/dravyn
bash scripts/validate.sh
```

For manual M8 behavior testing, use a disposable proxy profile:

1. configure Proxy + Strict Network Guard;
2. launch the profile and confirm Network Shield becomes healthy;
3. intentionally stop the test proxy endpoint;
4. observe degraded checks increment from 1/3 to 2/3;
5. after the third consecutive failure, confirm the profile is terminated and shield state becomes `tripped`;
6. restore the proxy, relaunch, and repeat Public IP/WebRTC/DNS/IPv6 external verification.

Do not use a production profile for destructive route-failure testing.
