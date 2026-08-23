#!/usr/bin/env bash
set -euo pipefail

DEPOT_TOOLS="${DRAVYN_DEPOT_TOOLS:-$HOME/.local/share/dravyn/depot_tools}"
CHROMIUM_WORKSPACE="${DRAVYN_CHROMIUM_WORKSPACE:-$HOME/.cache/dravyn/chromium}"
CHROMIUM_ROOT="${DRAVYN_CHROMIUM_ROOT:-$CHROMIUM_WORKSPACE/src}"
BUILD_DIR="${DRAVYN_CHROMIUM_BUILD_DIR:-out/Dravyn}"
JOBS="${DRAVYN_BUILD_JOBS:-2}"

if [[ ! -d "$DEPOT_TOOLS" ]]; then
  echo "depot_tools not found. Run ./scripts/chromium-bootstrap.sh first." >&2
  exit 1
fi

if [[ ! -d "$CHROMIUM_ROOT/.git" ]]; then
  echo "Chromium source not found. Run ./scripts/chromium-bootstrap.sh first." >&2
  exit 1
fi

export PATH="$DEPOT_TOOLS:$PATH"
cd "$CHROMIUM_ROOT"

GN_ARGS='is_debug=false symbol_level=0 blink_symbol_level=0 v8_symbol_level=0'

echo "Generating $BUILD_DIR"
gn gen "$BUILD_DIR" --args="$GN_ARGS"

echo "Building Chromium with $JOBS parallel jobs"
autoninja -C "$BUILD_DIR" -j "$JOBS" chrome

echo
echo "Build complete: $CHROMIUM_ROOT/$BUILD_DIR/chrome"
