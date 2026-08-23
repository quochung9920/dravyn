#!/usr/bin/env bash
set -euo pipefail

# shellcheck source=scripts/lib.sh
source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

usage() {
  cat <<'EOF'
Usage: scripts/chromium-run.sh [URL] [extra chromium flags...]

Launches the Chromium binary built in out/Dravyn through WSLg using a
clean development profile at $DRAVYN_HOME/runtime/dev-profile. The user's
own browser profiles are never touched.

Arguments:
  URL                     Optional startup URL (http:// or https://).
                          Can also be passed as --url <URL>.
  extra chromium flags    Any additional flags are passed to Chrome as-is.

Examples:
  scripts/chromium-run.sh
  scripts/chromium-run.sh https://example.com
  scripts/chromium-run.sh --url https://example.com
  scripts/chromium-run.sh --lang=de
EOF
}

URL=""
EXTRA_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    -h | --help)
      usage
      exit 0
      ;;
    --url)
      [[ $# -ge 2 ]] || dravyn_die "--url requires a value (see --help)" 2
      URL="$2"
      shift 2
      ;;
    --url=*)
      URL="${1#*=}"
      shift
      ;;
    *)
      if [[ -z "$URL" && "$1" != -* ]]; then
        URL="$1"
      else
        EXTRA_ARGS+=("$1")
      fi
      shift
      ;;
  esac
done

dravyn_init_paths

if [[ ! -x "$CHROME_BIN" ]]; then
  dravyn_die "Chromium binary not found: $CHROME_BIN

Build it first:
  dravyn chromium build"
fi

if [[ -z "${WAYLAND_DISPLAY:-}" && -z "${DISPLAY:-}" ]]; then
  dravyn_die "No GUI display detected (WAYLAND_DISPLAY and DISPLAY are both unset). Launch from a WSLg session."
fi

mkdir -p "$DEV_PROFILE_DIR"

dravyn_info "Launching Chromium (WSLg)"
dravyn_info "Profile: $DEV_PROFILE_DIR"

exec "$CHROME_BIN" \
  "--user-data-dir=$DEV_PROFILE_DIR" \
  --no-first-run \
  --no-default-browser-check \
  --ozone-platform-hint=auto \
  ${URL:+"$URL"} \
  "${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"}"
