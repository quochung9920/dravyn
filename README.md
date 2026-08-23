# Dravyn

Dravyn is a local-first browser-core research and development project focused on Chromium runtime management, isolated browser profiles, explicit network policy, privacy-oriented environment controls, and developer automation.

## Current milestone

**M1 - Chromium Foundation** (M0 complete)

- `dravyn` CLI with environment diagnostics
- Chromium workspace management under `$DRAVYN_HOME`
- depot_tools bootstrap, upstream Chromium checkout, GN configure,
  resource-aware build, WSLg launch - all via CLI or scripts

## Quick start

```bash
git clone https://github.com/quochung9920/dravyn.git
cd dravyn

cargo test --workspace

cargo install --path crates/dravyn-cli --force

dravyn doctor

dravyn chromium status
```

## M1 workflow

```bash
# 1. Install depot_tools and fetch the Chromium source (~tens of GiB)
dravyn chromium bootstrap

# 2. Generate the build configuration from browser/config/args.gn
dravyn chromium configure

# 3. Build the chrome target (RAM-aware parallelism; takes hours)
dravyn chromium build

# 4. Launch through WSLg with a clean development profile
dravyn chromium run
```

Every step is idempotent and also available as a standalone script:
`scripts/chromium-{bootstrap,configure,build,run}.sh --help`.

Details: [docs/chromium.md](docs/chromium.md).

## Repository layout

```text
crates/
  dravyn-cli/       Command-line interface
  dravyn-core/      Diagnostics, Chromium state detection, build orchestration
  dravyn-common/    Shared types; DRAVYN_HOME workspace resolution
  dravyn-profile/   Profile-domain foundation
  dravyn-network/   Network-policy foundation

browser/            Chromium configuration of record + future patch sets
scripts/            Development helpers (bootstrap/configure/build/run)
docs/               Architecture, roadmap, Chromium workflow documentation
```

## Scope

Dravyn is being built around local profile isolation, network control, privacy, compatibility testing, and authorized browser automation. It is not intended to provide mechanisms for bypassing identity verification, CAPTCHA systems, or third-party anti-fraud controls.

## Status

Early development (`0.1.0-dev`). APIs and file formats may change without notice.
