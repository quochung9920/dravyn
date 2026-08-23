#!/usr/bin/env bash
set -euo pipefail

DEPOT_TOOLS="${DRAVYN_DEPOT_TOOLS:-$HOME/.local/share/dravyn/depot_tools}"
CHROMIUM_WORKSPACE="${DRAVYN_CHROMIUM_WORKSPACE:-$HOME/.cache/dravyn/chromium}"
CHROMIUM_ROOT="${DRAVYN_CHROMIUM_ROOT:-$CHROMIUM_WORKSPACE/src}"

mkdir -p "$(dirname "$DEPOT_TOOLS")" "$CHROMIUM_WORKSPACE"

if [[ ! -d "$DEPOT_TOOLS/.git" ]]; then
  echo "Cloning depot_tools into $DEPOT_TOOLS"
  git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git "$DEPOT_TOOLS"
else
  echo "Updating depot_tools"
  git -C "$DEPOT_TOOLS" pull --ff-only
fi

export PATH="$DEPOT_TOOLS:$PATH"

if [[ ! -d "$CHROMIUM_ROOT/.git" ]]; then
  echo "Fetching Chromium into $CHROMIUM_WORKSPACE"
  cd "$CHROMIUM_WORKSPACE"
  fetch --nohooks chromium
else
  echo "Chromium checkout already exists: $CHROMIUM_ROOT"
  cd "$CHROMIUM_ROOT"
  gclient sync
fi

cd "$CHROMIUM_ROOT"

echo "Installing Chromium Linux build dependencies"
./build/install-build-deps.sh --no-prompt

echo "Running Chromium hooks"
gclient runhooks

REVISION="$(git rev-parse HEAD)"
printf '%s\n' "$REVISION" > "$CHROMIUM_WORKSPACE/revision.txt"

echo
echo "Chromium bootstrap complete"
echo "Source:   $CHROMIUM_ROOT"
echo "Revision: $REVISION"
echo "Next:     ./scripts/chromium-build.sh"
