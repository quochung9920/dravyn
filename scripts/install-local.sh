#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR"
echo "Installing Dravyn CLI from $ROOT_DIR"
cargo install --path crates/dravyn-cli --force

echo
if command -v dravyn >/dev/null 2>&1; then
  echo "Installed: $(command -v dravyn)"
  dravyn --version
else
  echo "Dravyn was installed by Cargo but is not on PATH." >&2
  echo 'Ensure $HOME/.cargo/bin is present in PATH.' >&2
  exit 1
fi
