#!/usr/bin/env bash
set -euo pipefail

# shellcheck source=scripts/lib.sh
source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

usage() {
  cat <<'EOF'
Usage: scripts/chromium-bootstrap.sh [options]

Installs depot_tools and fetches the upstream Chromium source into the
Dravyn workspace ($DRAVYN_HOME, default ~/.cache/dravyn). Safe to re-run:
existing depot_tools is updated in place and an existing checkout is synced.

Options:
  --no-deps    Skip Chromium's install-build-deps.sh (no sudo required).
               Use this if you already installed build dependencies.
  -h, --help   Show this help.

Steps performed:
  1. validate git, python3, architecture and disk space
  2. clone/update depot_tools at $DRAVYN_HOME/depot_tools
  3. fetch the Chromium source (first run) or sync it (later runs)
  4. install Linux build dependencies via the official upstream script
     (invokes sudo; skipped with --no-deps)
  5. run gclient sync/hooks
  6. record the resolved revision to $DRAVYN_HOME/chromium/revision.txt

The workspace never lives inside the Dravyn Git repository.
EOF
}

SKIP_DEPS=false
for arg in "$@"; do
  case "$arg" in
    -h | --help)
      usage
      exit 0
      ;;
    --no-deps)
      SKIP_DEPS=true
      ;;
    *)
      dravyn_die "Unknown option: $arg (see --help)" 2
      ;;
  esac
done

dravyn_init_paths

dravyn_info "Workspace root: $DRAVYN_ROOT"

dravyn_require_command git "Install git (sudo apt install git) and retry."
dravyn_require_command python3 "Chromium tooling needs python3. Install it (sudo apt install python3) and retry."
dravyn_check_architecture

TOTAL_MEM_KIB="$(dravyn_total_memory_kib)"
if [[ -n "$TOTAL_MEM_KIB" && $((TOTAL_MEM_KIB / (1024 * 1024))) -lt 8 ]]; then
  dravyn_warn "Less than 8 GiB RAM detected. Building Chromium will be slow; consider raising WSL memory."
fi

mkdir -p "$DRAVYN_ROOT/chromium"
dravyn_check_disk_space "$DRAVYN_ROOT/chromium" 150 80 "a Chromium checkout and later builds"

if [[ -d "$DEPOT_TOOLS_DIR/.git" ]]; then
  dravyn_info "Updating existing depot_tools at $DEPOT_TOOLS_DIR"
  git -C "$DEPOT_TOOLS_DIR" pull --ff-only
elif [[ -e "$DEPOT_TOOLS_DIR" && ! -d "$DEPOT_TOOLS_DIR/.git" ]]; then
  dravyn_die "$DEPOT_TOOLS_DIR exists but is not a git checkout. Remove it manually and re-run."
else
  dravyn_info "Cloning depot_tools into $DEPOT_TOOLS_DIR"
  git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git "$DEPOT_TOOLS_DIR"
fi

export PATH="$DEPOT_TOOLS_DIR:$PATH"

if [[ ! -d "$CHROMIUM_SRC_DIR/.git" ]]; then
  if [[ -e "$CHROMIUM_SRC_DIR" ]]; then
    dravyn_die "$CHROMIUM_SRC_DIR exists but has no .git directory. A previous fetch probably failed. Remove '$DRAVYN_ROOT/chromium/src' and re-run this script."
  fi
  dravyn_info "Fetching Chromium source into $DRAVYN_ROOT/chromium (this downloads tens of GiB)"
  cd "$DRAVYN_ROOT/chromium"
  fetch --nohooks chromium
else
  dravyn_info "Existing Chromium checkout found: $CHROMIUM_SRC_DIR"
fi

cd "$CHROMIUM_SRC_DIR"

if [[ "$SKIP_DEPS" == false ]]; then
  if [[ ! -f "./build/install-build-deps.sh" ]]; then
    dravyn_die "./build/install-build-deps.sh not found. The checkout looks incomplete; re-run this script."
  fi
  echo
  dravyn_info "About to run Chromium's official dependency installer:"
  dravyn_info "  ./build/install-build-deps.sh --no-prompt"
  dravyn_info "It invokes 'sudo apt-get install' and may ask for your password."
  ./build/install-build-deps.sh --no-prompt
else
  dravyn_info "Skipping build dependencies (--no-deps)."
fi

dravyn_info "Syncing checkout and running hooks (gclient sync -D)"
gclient sync -D

REVISION="$(git rev-parse HEAD)"
printf '%s\n' "$REVISION" >"$REVISION_FILE"

echo
dravyn_info "Bootstrap complete"
dravyn_info "Source:   $CHROMIUM_SRC_DIR"
dravyn_info "Revision: $REVISION"
dravyn_info "Next:     scripts/chromium-configure.sh (or: dravyn chromium configure)"
