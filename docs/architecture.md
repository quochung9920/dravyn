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
Chromium checkout outside the Git repository
        |
        v
Chromium build + isolated user-data directories
```

## Principles

1. Chromium source and build output stay outside the Dravyn Git repository.
2. Dravyn tracks scripts, configuration, tests, and later patch sets.
3. Browser profiles must have explicit, separate user-data directories.
4. Network policy will be fail-closed when a profile requires a proxy.
5. Automation will connect to a Dravyn-managed browser rather than silently launching an unrelated browser instance.
6. Changes to Chromium will be maintained as a reviewable patch series instead of undocumented edits.

## Current components

- `dravyn-cli`: user-facing command-line interface.
- `dravyn-core`: diagnostics and future process lifecycle.
- `dravyn-profile`: profile-domain foundation.
- `dravyn-network`: network-policy foundation.
- `dravyn-common`: shared types and utilities.

The desktop GUI, SaaS control plane, billing, team management, and cloud browser infrastructure are intentionally out of scope during the core phase.
