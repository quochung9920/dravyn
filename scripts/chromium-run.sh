#!/usr/bin/env bash
set -euo pipefail

CHROMIUM_WORKSPACE="${DRAVYN_CHROMIUM_WORKSPACE:-$HOME/.cache/dravyn/chromium}"
CHROMIUM_ROOT="${DRAVYN_CHROMIUM_ROOT:-$CHROMIUM_WORKSPACE/src}"
BUILD_DIR="${DRAVYN_CHROMIUM_BUILD_DIR:-out/Dravyn}"
BROWSER_BINARY="${DRAVYN_CHROMIUM_BINARY:-$CHROMIUM_ROOT/$BUILD_DIR/chrome}"
PROFILE_NAME="${1:-m1-smoke}"
PROFILE_ROOT="${DRAVYN_PROFILE_ROOT:-$HOME/.local/share/dravyn/profiles}"
USER_DATA_DIR="$PROFILE_ROOT/$PROFILE_NAME/chromium-data"

if [[ ! -x "$BROWSER_BINARY" ]]; then
  echo "Chromium binary not found: $BROWSER_BINARY" >&2
  echo "Run ./scripts/chromium-build.sh first." >&2
  exit 1
fi

if [[ -z "${WAYLAND_DISPLAY:-}" && -z "${DISPLAY:-}" ]]; then
  echo "No WSLg/GUI display detected." >&2
  exit 1
fi

mkdir -p "$USER_DATA_DIR"

echo "Launching Chromium"
echo "Profile: $PROFILE_NAME"
echo "Data:    $USER_DATA_DIR"

exec "$BROWSER_BINARY" \
  --user-data-dir="$USER_DATA_DIR" \
  --no-first-run \
  --no-default-browser-check
