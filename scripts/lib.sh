#!/usr/bin/env bash
# Shared helpers for Dravyn Chromium scripts.
# This file is meant to be sourced, never executed directly.

DRAVYN_SCRIPTS_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC2034  # used by the scripts that source this file
DRAVYN_REPO_ROOT="$(dirname -- "$DRAVYN_SCRIPTS_DIR")"

dravyn_info() { printf '[dravyn] %s\n' "$*"; }
dravyn_warn() { printf '[dravyn] WARNING: %s\n' "$*" >&2; }

dravyn_die() {
  local message="$1"
  local code="${2:-1}"
  printf '[dravyn] ERROR: %s\n' "$message" >&2
  exit "$code"
}

dravyn_require_command() {
  local name="$1"
  local hint="${2:-Install it and re-run this script.}"
  if ! command -v "$name" >/dev/null 2>&1; then
    dravyn_die "'$name' is required but was not found in PATH. $hint"
  fi
}

# Resolves the Dravyn workspace root: DRAVYN_HOME wins, otherwise
# $HOME/.cache/dravyn. Prints the path on stdout.
dravyn_resolve_home() {
  if [[ -n "${DRAVYN_HOME:-}" ]]; then
    printf '%s\n' "${DRAVYN_HOME%/}"
  elif [[ -n "${HOME:-}" ]]; then
    printf '%s/.cache/dravyn\n' "$HOME"
  else
    return 1
  fi
}

# Populates DRAVYN_ROOT plus all derived Chromium workspace paths.
# Values mirror browser/config/chromium.toml and crates/dravyn-common.
# shellcheck disable=SC2034  # consumers source these variables
dravyn_init_paths() {
  DRAVYN_ROOT="$(dravyn_resolve_home)" || dravyn_die \
    "Neither DRAVYN_HOME nor HOME is set. Export DRAVYN_HOME=/path/to/workspace and retry."
  DEPOT_TOOLS_DIR="$DRAVYN_ROOT/depot_tools"
  CHROMIUM_SRC_DIR="$DRAVYN_ROOT/chromium/src"
  BUILD_OUTPUT_REL="out/Dravyn"
  BUILD_OUTPUT_DIR="$CHROMIUM_SRC_DIR/$BUILD_OUTPUT_REL"
  CHROME_BIN="$BUILD_OUTPUT_DIR/chrome"
  REVISION_FILE="$DRAVYN_ROOT/chromium/revision.txt"
  DEV_PROFILE_DIR="$DRAVYN_ROOT/runtime/dev-profile"
}

# Free space in KiB on the filesystem containing $1.
dravyn_available_disk_kib() {
  df -Pk -- "$1" 2>/dev/null | awk 'NR == 2 { print $4 }'
}

dravyn_total_memory_kib() {
  awk '/^MemTotal:/ { print $2 }' /proc/meminfo 2>/dev/null || true
}

dravyn_available_memory_kib() {
  awk '/^MemAvailable:/ { print $2 }' /proc/meminfo 2>/dev/null || true
}

dravyn_check_architecture() {
  local machine
  machine="$(uname -m)"
  case "$machine" in
    x86_64 | aarch64) ;;
    *)
      dravyn_die "Unsupported architecture '$machine'. Upstream Chromium Linux builds target x86_64 (or arm64)."
      ;;
  esac
}

# Fails below min_gib, warns below warn_gib. Usage: dravyn_check_disk_space PATH WARN_GIB MIN_GIB PURPOSE
dravyn_check_disk_space() {
  local path="$1"
  local warn_gib="$2"
  local min_gib="$3"
  local purpose="$4"

  local free_kib
  free_kib="$(dravyn_available_disk_kib "$path")"
  if [[ -z "$free_kib" ]]; then
    dravyn_warn "Could not determine free disk space for '$path'. Continuing."
    return 0
  fi

  local free_gib=$((free_kib / (1024 * 1024)))
  if ((free_gib < min_gib)); then
    dravyn_die "Only ${free_gib} GiB free on the filesystem of '$path'; $purpose needs about ${min_gib} GiB. Free up space or point DRAVYN_HOME at a larger disk."
  fi
  if ((free_gib < warn_gib)); then
    dravyn_warn "${free_gib} GiB free is enough to start but may be tight for '$purpose'."
  fi
  dravyn_info "Disk space OK: ${free_gib} GiB free for $purpose"
}
