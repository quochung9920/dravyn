#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
APP_DIR="$REPO_ROOT/apps/desktop"

if [[ -z "${WAYLAND_DISPLAY:-}" && -z "${DISPLAY:-}" ]]; then
  printf '%s\n' "[dravyn] No GUI display detected. Run Dravyn Desktop from a WSLg/desktop session." >&2
  exit 1
fi

if ! command -v pnpm >/dev/null 2>&1; then
  printf '%s\n' "[dravyn] pnpm is required. Install/activate pnpm before starting Dravyn Desktop." >&2
  exit 1
fi

if [[ ! -f "$APP_DIR/package.json" ]]; then
  printf '%s\n' "[dravyn] Desktop app not found at $APP_DIR" >&2
  exit 1
fi

cd "$APP_DIR"
if [[ ! -d node_modules ]]; then
  printf '%s\n' "[dravyn] Installing desktop frontend dependencies..."
  pnpm install
fi

exec pnpm tauri dev
