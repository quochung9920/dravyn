#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

printf '\n[dravyn] Rust format\n'
cargo fmt --all -- --check

printf '\n[dravyn] Rust workspace check\n'
cargo check --workspace

printf '\n[dravyn] Rust workspace tests\n'
cargo test --workspace

printf '\n[dravyn] Desktop dependencies\n'
cd "$ROOT/apps/desktop"
pnpm install --no-frozen-lockfile

printf '\n[dravyn] Desktop typecheck\n'
pnpm check

printf '\n[dravyn] Desktop production build\n'
pnpm build

printf '\n[dravyn] Tauri backend check\n'
cd "$ROOT"
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml

printf '\n[dravyn] Validation complete\n'
