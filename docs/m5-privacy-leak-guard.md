# M5 - Per-Profile Privacy Policy & Leak Guard

M5 separates three concepts that should not be conflated in a commercial privacy browser:

1. **profile isolation** - separate Chromium user-data directories and session state;
2. **fingerprint observability** - local per-profile baseline/history/drift from M4;
3. **privacy enforcement and verification** - defensive browser preferences, network preflight and real-website checks added in M5.

The goal is a transparent privacy/QA platform. M5 does not spoof a device identity and does not claim that an endpoint-reachable proxy makes a profile anonymous.

## Per-profile privacy policy

Each `Profile` and `ProfileDraft` now contains a `PrivacyPolicy`:

- `preset`: standard / balanced / strict / custom
- `network_guard`: off / monitor / strict
- `webrtc`: default / proxied_only
- block third-party cookies
- block notifications
- block geolocation
- block camera
- block microphone

Existing profiles deserialize safely through the policy default.

## Launch lifecycle

For a stopped profile the runtime follows this order:

```text
resolve profile
    ↓
validate network configuration
    ↓
validate privacy policy
    ↓
Strict proxy profile?
    ├─ yes → proxy TCP preflight → fail closed if unreachable
    └─ no  → continue
    ↓
write Chromium profile Preferences
    ↓
read Preferences back and verify expected policy
    ↓
apply supported Chromium command-line privacy switches
    ↓
spawn Chromium with exact profile user-data-dir
    ↓
open start URL
```

The privacy policy is therefore applied before the first requested website is opened for a stopped profile.

If a running profile is edited, the Privacy Center reports that a restart is required. The application deliberately does not pretend that a policy edit retroactively changes an already-running Chromium process.

## Strict Network Guard

Strict mode is fail-closed for configured proxy profiles. Before Chromium starts, Dravyn resolves the proxy hostname and attempts a short TCP connection to one of its resolved addresses.

If the endpoint cannot be reached, launch is rejected.

This prevents the product from silently presenting an unusable configured proxy as healthy. It is still only an endpoint preflight. It does **not** prove:

- the public browser IP;
- proxy credentials are accepted by a destination;
- DNS is routed as expected;
- IPv6 is not exposed;
- WebRTC cannot reveal an unexpected address;
- the remote service considers the route anonymous.

Those require external observation.

## WebRTC defensive mode

`proxied_only` applies the Chromium WebRTC IP handling policy `disable_non_proxied_udp` through the profile Preferences and the matching command-line policy switch. This is a defensive privacy control intended to reduce non-proxied UDP exposure.

Because WebRTC behavior can affect legitimate real-time applications, it is explicit per profile rather than globally forced.

## Chromium Preferences

M5 updates only the selected profile's `Default/Preferences` file and preserves unrelated JSON properties. The policy currently controls:

- `profile.block_third_party_cookies`
- `webrtc.ip_handling_policy`
- `webrtc.multiple_routes_enabled`
- `webrtc.nonproxied_udp_enabled`
- default content settings for notifications, geolocation, camera and microphone

After writing, Dravyn reads the Preferences file again and checks that the requested values are present. A stopped profile is not launched if local policy verification fails.

## Privacy Center

The desktop Privacy Center is intentionally separate from Fingerprint Center.

For the selected profile it shows:

- selected preset;
- Network Guard mode;
- WebRTC policy;
- whether stored Chromium preferences match policy;
- proxy endpoint preflight result;
- restart-required state;
- External Verification Lab.

## External Verification Lab

M5 whitelists a small set of third-party privacy/fingerprint test pages and always opens them inside the selected Dravyn profile:

- BrowserLeaks IP
- BrowserLeaks WebRTC
- BrowserLeaks DNS
- BrowserLeaks Canvas
- BrowserLeaks WebGL
- EFF Cover Your Tracks
- AmIUnique

If the profile is stopped, the normal launch lifecycle runs first, including privacy policy verification and strict network preflight. If the profile is already running, the URL is opened in that exact Chromium user-data directory.

Third-party tests receive the browser/network characteristics necessary to perform their tests. Users should understand that these are external services, unlike Dravyn's local fingerprint audit.

## Interpreting results

Use the following distinction:

```text
Endpoint reachable
≠ no network leak

Fingerprint stable
≠ anonymous

No local consistency warning
≠ remote website cannot identify the browser
```

A proxy profile should be considered healthy only after both local policy/preflight checks and a recent external verification of the network surfaces that matter to the user's policy.

An unexpected real public address in a WebRTC/IP/IPv6 test should be treated as critical even if the local fingerprint score is high.

## Safety boundary

M5 is defensive privacy engineering. It does not:

- generate fake device identities;
- randomize Canvas/WebGL/audio fingerprints to imitate other machines;
- bypass CAPTCHA, KYC or anti-fraud controls;
- claim that any profile is undetectable;
- automatically interpret a third-party site's anti-bot score as a target to evade.

## Run

M5 does not change the Chromium source tree, so no Chromium rebuild is required.

```bash
cd ~/projects/dravyn
git pull origin main

cd apps/desktop
pnpm install
pnpm tauri dev
```

After launch, edit a profile's Privacy Policy, then open **Privacy** and run preflight/external verification for that profile.
