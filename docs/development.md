# Development

## Primary environment

Dravyn is currently developed from Ubuntu 24.04 under WSL2 with WSLg.

## Validate the Rust workspace

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI enforces exactly these checks plus `shellcheck scripts/*.sh`.

## Install the CLI

```bash
./scripts/install-local.sh
dravyn doctor
```

The executable is always named `dravyn` (the Cargo package is
`dravyn-cli`).

## Chromium workspace

Large/generated assets live outside the repository:

```text
$DRAVYN_HOME/depot_tools          (default ~/.cache/dravyn/depot_tools)
$DRAVYN_HOME/chromium/src         upstream checkout
$DRAVYN_HOME/chromium/src/out/Dravyn  build directory
$DRAVYN_HOME/runtime/dev-profile  throwaway dev profile
```

Set `DRAVYN_HOME` to relocate the entire workspace. Full workflow,
GN argument rationale, and resource notes: see `docs/chromium.md`.

## Shell scripts

All scripts under `scripts/` support `--help`, are idempotent, and are
linted with shellcheck in CI:

```bash
scripts/chromium-bootstrap.sh --help
scripts/chromium-configure.sh --help
scripts/chromium-build.sh --help
scripts/chromium-run.sh --help
```
