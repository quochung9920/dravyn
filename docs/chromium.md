# Chromium Development

M1 establishes a reproducible upstream Chromium checkout, build, and launch path before any Dravyn-specific Chromium patches are introduced.

## Default paths

```text
depot_tools:  ~/.local/share/dravyn/depot_tools
workspace:    ~/.cache/dravyn/chromium
source:       ~/.cache/dravyn/chromium/src
build:        ~/.cache/dravyn/chromium/src/out/Dravyn
binary:       ~/.cache/dravyn/chromium/src/out/Dravyn/chrome
```

The large Chromium checkout is deliberately not stored inside this repository.

## Bootstrap

```bash
./scripts/chromium-bootstrap.sh
```

This clones/updates `depot_tools`, fetches upstream Chromium, installs Linux build dependencies, and runs Chromium hooks.

## Build

The current WSL environment has limited memory, so the default build concurrency is conservative:

```bash
DRAVYN_BUILD_JOBS=2 ./scripts/chromium-build.sh
```

You can increase `DRAVYN_BUILD_JOBS` later if more memory is assigned to WSL.

## Launch smoke profile

```bash
./scripts/chromium-run.sh m1-smoke
```

This is only the M1 smoke launcher. The dedicated Dravyn profile engine will own profile lifecycle in a later milestone.

## Overrides

- `DRAVYN_DEPOT_TOOLS`
- `DRAVYN_CHROMIUM_WORKSPACE`
- `DRAVYN_CHROMIUM_ROOT`
- `DRAVYN_CHROMIUM_BUILD_DIR`
- `DRAVYN_CHROMIUM_BINARY`
- `DRAVYN_BUILD_JOBS`
- `DRAVYN_PROFILE_ROOT`

After bootstrap/build, run `dravyn doctor` to see what Dravyn detects.
