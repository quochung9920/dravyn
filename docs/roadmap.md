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

## M1 - Chromium Foundation

Status: implementation complete; full local Chromium build not yet verified.

- install/update `depot_tools` (`dravyn chromium bootstrap`)
- fetch Chromium outside the repository under `$DRAVYN_HOME`
- official Linux dependency installation with explicit sudo announcement
- documented GN configuration (`browser/config/args.gn`)
- resource-aware `chrome` build (RAM-based job limiting)
- launch through WSLg with a clean dev profile
- real state detection surfaced through `dravyn doctor` and
  `dravyn chromium status`

Success condition: an upstream Chromium build produced from the Dravyn
workflow opens through WSLg and is reported as `Build PASS / M1 PASS` by
`dravyn doctor`.

## M2 - Profile Engine

- create/list/remove profiles
- isolated user-data directories
- start/stop lifecycle
- persistent session tests
- process locking and crash recovery baseline

The launcher interface keeps future compatibility for: proxy configuration,
locale, timezone policy, CDP port, extensions, startup URLs, and runtime
flags.

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
- maintained Chromium patch series where upstream configuration is
  insufficient (`browser/patches/`)
