# Development

## Primary environment

Dravyn is currently developed from Ubuntu 24.04 under WSL2 with WSLg.

## Validate the Rust workspace

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
```

## Install the CLI

```bash
./scripts/install-local.sh
dravyn doctor
```

## Local directories

Dravyn intentionally keeps large/generated assets outside the Git repository:

```text
~/.local/share/dravyn/depot_tools
~/.cache/dravyn/chromium/src
~/.local/share/dravyn/profiles
```

These defaults may be overridden with environment variables documented in `docs/chromium.md`.
