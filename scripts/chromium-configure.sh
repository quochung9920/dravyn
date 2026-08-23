#!/usr/bin/env bash
set -euo pipefail

# shellcheck source=scripts/lib.sh
source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

usage() {
  cat <<'EOF'
Usage: scripts/chromium-configure.sh [-h|--help]

Generates the Dravyn build directory (out/Dravyn) inside the Chromium
checkout using the canonical GN arguments from browser/config/args.gn.

Requires a completed bootstrap (depot_tools and Chromium source).
Safe to re-run; re-configure after changing args.gn.
EOF
}

for arg in "$@"; do
  case "$arg" in
    -h | --help)
      usage
      exit 0
      ;;
    *)
      dravyn_die "Unknown option: $arg (see --help)" 2
      ;;
  esac
done

dravyn_init_paths

GN_ARGS_FILE="$DRAVYN_REPO_ROOT/browser/config/args.gn"

if [[ ! -f "$GN_ARGS_FILE" ]]; then
  dravyn_die "GN arguments file not found: $GN_ARGS_FILE. Run this script from a Dravyn repository checkout."
fi

if [[ ! -d "$DEPOT_TOOLS_DIR/.git" ]]; then
  dravyn_die "depot_tools not found at $DEPOT_TOOLS_DIR. Run scripts/chromium-bootstrap.sh first (or: dravyn chromium bootstrap)."
fi

if [[ ! -d "$CHROMIUM_SRC_DIR/.git" ]]; then
  dravyn_die "Chromium source not found at $CHROMIUM_SRC_DIR. Run scripts/chromium-bootstrap.sh first (or: dravyn chromium bootstrap)."
fi

export PATH="$DEPOT_TOOLS_DIR:$PATH"
dravyn_require_command gn "gn should come from depot_tools; make sure $DEPOT_TOOLS_DIR is readable."

cd "$CHROMIUM_SRC_DIR"
mkdir -p "$BUILD_OUTPUT_REL"

dravyn_info "Generating build directory with Dravyn GN arguments"
gn gen "$BUILD_OUTPUT_REL" --args="$(<"$GN_ARGS_FILE")"

echo
dravyn_info "Configure complete: $BUILD_OUTPUT_DIR"
dravyn_info "Next:     scripts/chromium-build.sh (or: dravyn chromium build)"
