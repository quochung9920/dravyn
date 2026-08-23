#!/usr/bin/env bash
set -euo pipefail

if ! grep -qi microsoft /proc/version 2>/dev/null; then
  echo "This setup script is intended for WSL2." >&2
  exit 1
fi

sudo apt update
sudo apt install -y \
  build-essential \
  clang \
  lld \
  ninja-build \
  cmake \
  pkg-config \
  curl \
  ca-certificates \
  git \
  git-lfs \
  jq \
  unzip \
  zip \
  xz-utils

echo
for cmd in git rustc cargo node pnpm python3 clang ninja cmake; do
  if command -v "$cmd" >/dev/null 2>&1; then
    printf '%-10s OK\n' "$cmd"
  else
    printf '%-10s MISSING\n' "$cmd"
  fi
done

echo
printf 'WSLg: '
if [[ -n "${WAYLAND_DISPLAY:-}" ]]; then
  echo "OK (${WAYLAND_DISPLAY})"
else
  echo "MISSING"
fi
