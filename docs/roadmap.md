# Roadmap

## M0 - Foundation

Status: complete.

- Rust workspace
- Dravyn CLI
- `dravyn doctor`
- WSL2 / WSLg diagnostics
- toolchain/resource diagnostics
- CI validation
- local install helper

## M1 - Upstream Chromium

Status: in progress.

- install/update `depot_tools`
- fetch Chromium outside the repository
- install Linux build dependencies
- generate a low-symbol build configuration
- compile `chrome`
- launch through WSLg
- detect source/build from `dravyn doctor`

Success condition: an upstream Chromium build produced from the Dravyn workflow opens through WSLg and is recognized by `dravyn doctor`.

## M2 - Profile Engine

- create/list/remove profiles
- isolated user-data directories
- start/stop lifecycle
- persistent session tests
- process locking and crash recovery baseline

## M3 - Network Engine

- explicit proxy configuration
- HTTP/HTTPS/SOCKS support
- connection health checks
- fail-closed network policy
- leak regression tests

## M4 - Automation

- controlled local CDP endpoint
- Playwright connection
- smoke workflow
- automation regression tests

## M5 - Privacy and Environment Controls

- explicit locale/timezone/language policy
- permission policy
- WebRTC privacy controls
- consistency tests
- maintained Chromium patch series where upstream configuration is insufficient
