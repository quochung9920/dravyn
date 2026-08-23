# Dravyn Architecture

Dravyn is intentionally local-first during the browser-core phase.

```text
Dravyn CLI (Rust)
        |
        v
Dravyn Core
  |     |     |
  |     |     +-- diagnostics / runtime lifecycle
  |     +-------- profile domain
  +-------------- network policy domain
        |
        v
Chromium workspace at $DRAVYN_HOME (default ~/.cache/dravyn)
  depot_tools / chromium src / out/Dravyn build
        |
        v
Chromium build + isolated user-data directories
```

## Principles

1. Chromium source and build output stay outside the Dravyn Git repository.
2. Dravyn tracks scripts, configuration, tests, and later patch sets.
3. Browser profiles must have explicit, separate user-data directories.
4. Network policy will be fail-closed when a profile requires a proxy.
5. Automation will connect to a Dravyn-managed browser rather than silently
   launching an unrelated browser instance.
6. Changes to Chromium will be maintained as a reviewable patch series
   (`browser/patches/`) instead of undocumented edits.

## Current components

- `dravyn-cli`: user-facing command-line interface (`doctor`, `chromium ...`).
- `dravyn-core`: environment diagnostics, Chromium state detection,
  workspace/job orchestration helpers.
- `dravyn-common`: shared types; owns the single source of truth for
  workspace path resolution (`DRAVYN_HOME`).
- `dravyn-profile`: profile-domain foundation (M2).
- `dravyn-network`: network-policy foundation (M3).

## Layering rules

- The CLI routes and renders; it does not reimplement workflows.
- Core exposes typed, testable services (state detection, job calculation).
- Shell scripts remain the canonical implementation of bootstrap/configure/
  build/run so they stay usable without Rust and mirror upstream tooling.
- The CLI invokes those scripts with explicit argument vectors via
  `std::process::Command`; no `sh -c` string concatenation with user input.

The desktop GUI, SaaS control plane, billing, team management, and cloud
browser infrastructure are intentionally out of scope during the core phase.
