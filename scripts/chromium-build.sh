#!/usr/bin/env bash
set -euo pipefail

# shellcheck source=scripts/lib.sh
source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

usage() {
  cat <<'EOF'
Usage: scripts/chromium-build.sh [--jobs N] [-h|--help]

Builds the 'chrome' target in out/Dravyn. The directory is configured
automatically (using browser/config/args.gn) when missing.

Job count resolution, most specific wins:
  1. --jobs N
  2. DRAVYN_BUILD_JOBS environment variable
  3. auto: available RAM / 3 GiB per link job, capped by CPU count

The auto default keeps memory-constrained WSL guests from OOM-killing
linker processes.

Options:
  --jobs N    Run at most N parallel ninja jobs.
  -h, --help  Show this help.
EOF
}

JOBS=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    -h | --help)
      usage
      exit 0
      ;;
    --jobs)
      [[ $# -ge 2 ]] || dravyn_die "--jobs requires a value (see --help)" 2
      JOBS="$2"
      shift 2
      ;;
    --jobs=*)
      JOBS="${1#*=}"
      shift
      ;;
    *)
      dravyn_die "Unknown option: $1 (see --help)" 2
      ;;
  esac
done

if [[ -n "$JOBS" && ! "$JOBS" =~ ^[1-9][0-9]*$ ]]; then
  dravyn_die "--jobs must be a positive integer (got '$JOBS')" 2
fi

dravyn_init_paths

if [[ ! -d "$DEPOT_TOOLS_DIR/.git" ]]; then
  dravyn_die "depot_tools not found at $DEPOT_TOOLS_DIR. Run scripts/chromium-bootstrap.sh first (or: dravyn chromium bootstrap)."
fi

if [[ ! -d "$CHROMIUM_SRC_DIR/.git" ]]; then
  dravyn_die "Chromium source not found at $CHROMIUM_SRC_DIR. Run scripts/chromium-bootstrap.sh first (or: dravyn chromium bootstrap)."
fi

export PATH="$DEPOT_TOOLS_DIR:$PATH"

if [[ ! -f "$BUILD_OUTPUT_DIR/args.gn" ]]; then
  dravyn_info "No build configuration found; running configure step first"
  "$DRAVYN_SCRIPTS_DIR/chromium-configure.sh"
fi

if [[ -z "$JOBS" ]]; then
  JOBS="${DRAVYN_BUILD_JOBS:-}"
fi

if [[ -n "$JOBS" ]]; then
  dravyn_info "Using requested job count: $JOBS"
else
  AVAIL_KIB="$(dravyn_available_memory_kib)"
  CPUS="$(nproc)"
  if [[ -z "$AVAIL_KIB" ]]; then
    JOBS=$((CPUS / 2))
    ((JOBS < 1)) && JOBS=1
    dravyn_warn "Could not read MemAvailable; falling back to half the CPU count."
    dravyn_info "Auto-selected jobs: $JOBS (CPUs: $CPUS)"
  else
    JOBS=$((AVAIL_KIB / (3 * 1024 * 1024)))
    ((JOBS > CPUS)) && JOBS=$CPUS
    ((JOBS < 1)) && JOBS=1
    dravyn_info "Auto-selected jobs: $JOBS (CPUs: $CPUS, available RAM: $((AVAIL_KIB / (1024 * 1024))) GiB)"
  fi
  dravyn_info "Override with --jobs N or DRAVYN_BUILD_JOBS."
fi

cd "$CHROMIUM_SRC_DIR"

dravyn_info "Building 'chrome' with $JOBS parallel jobs (this can take hours on a laptop/WSL)"
autoninja -C "$BUILD_OUTPUT_REL" chrome

echo
dravyn_info "Build complete: $CHROME_BIN"
dravyn_info "Next:     scripts/chromium-run.sh (or: dravyn chromium run)"
