# Dravyn

Dravyn is a local-first browser-core research and development project focused on Chromium runtime management, isolated browser profiles, explicit network policy, privacy-oriented environment controls, and developer automation.

## Current milestone

**M0 - Foundation**

- Rust workspace
- `dravyn` CLI
- `dravyn doctor` environment diagnostics
- WSL2 / WSLg detection
- Toolchain checks
- Memory and disk diagnostics
- Unit tests and CI

Chromium checkout/build work starts in M1 after M0 is stable.

## Quick start

```bash
git clone https://github.com/quochung9920/dravyn.git
cd dravyn
cargo test --workspace
cargo run -p dravyn-cli -- doctor
```

To install the CLI locally:

```bash
./scripts/install-local.sh

dravyn doctor
```

## Repository layout

```text
crates/
  dravyn-cli/       Command-line interface
  dravyn-core/      Runtime and diagnostics core
  dravyn-common/    Shared types and utilities
  dravyn-profile/   Profile-domain foundation
  dravyn-network/   Network-policy foundation

browser/            Chromium configuration and future patch sets
automation/         Future Playwright/CDP integration
tests/              Cross-component test area
scripts/            Development helpers
docs/               Architecture and roadmap documentation
```

## Scope

Dravyn is being built around local profile isolation, network control, privacy, compatibility testing, and authorized browser automation. It is not intended to provide mechanisms for bypassing identity verification, CAPTCHA systems, or third-party anti-fraud controls.

## Status

Early development (`0.0.1-dev`). APIs and file formats may change without notice.
