# Chromium Development (M1)

M1 establishes a reproducible upstream Chromium checkout, build, and launch
path before any Dravyn-specific Chromium patches exist.

## Where Chromium lives, and why it is outside the repo

Chromium source, build outputs, and `depot_tools` are large (tens of GiB
checked out; 50-100+ GiB built). They must never enter the Dravyn Git
repository. Everything lives in a dedicated workspace:

```text
$DRAVYN_HOME (default ~/.cache/dravyn)
├── depot_tools/                 Google's Chromium tooling (gclient, gn, ninja)
├── chromium/
│   ├── revision.txt             resolved upstream revision, written at bootstrap
│   └── src/
│       ├── out/Dravyn/          Dravyn build directory
│       │   ├── args.gn          generated GN arguments
│       │   └── chrome           browser binary
│       └── ...
└── runtime/
    └── dev-profile/             throwaway development profile used by run
```

The repository only tracks configuration (`browser/config/`), scripts
(`scripts/`), patch infrastructure (`browser/patches/`), runtime logic,
tests, and documentation.

## DRAVYN_HOME

All tools resolve the workspace the same way:

1. `$DRAVYN_HOME` when set
2. otherwise `$HOME/.cache/dravyn`

Example:

```bash
export DRAVYN_HOME=/mnt/bigdisk/dravyn
dravyn chromium status   # now inspects /mnt/bigdisk/dravyn
```

The same resolution is implemented in Rust (`crates/dravyn-common/src/workspace.rs`)
and in shell (`scripts/lib.sh`). Do not hardcode absolute paths anywhere else.

## How depot_tools works

`depot_tools` is Google's collection of build/checkout tooling for Chromium:
`fetch` bootstraps a checkout from a `.gclient` manifest, `gclient sync`
updates sources and runs hooks, `gn` generates Ninja files, `autoninja`
drives compilation. Dravyn clones it once into `$DRAVYN_HOME/depot_tools`
and prepends it to `PATH` inside its own processes only; your shell config
files are never modified.

## Workflow

```bash
# 1. Bootstrap: depot_tools + source + deps + hooks
dravyn chromium bootstrap        # or scripts/chromium-bootstrap.sh

# 2. Configure: generate out/Dravyn from browser/config/args.gn
dravyn chromium configure        # or scripts/chromium-configure.sh

# 3. Build: compile the chrome target (resource-aware jobs)
dravyn chromium build            # or scripts/chromium-build.sh
dravyn chromium build --jobs 4

# 4. Run: launch via WSLg with a clean dev profile
dravyn chromium run
dravyn chromium run https://example.com

# Inspect state at any time (never downloads anything)
dravyn chromium status
dravyn doctor
```

### Bootstrap details

Validates git, python3, architecture, RAM, and free disk space first.
Then:

- clones or fast-forward updates `depot_tools`
- runs `fetch --nohooks chromium` on first run, `gclient sync -D` afterwards
- installs Linux build dependencies using Chromium's official
  `build/install-build-deps.sh` (announces the sudo requirement first;
  skip with `--no-deps` if deps are already installed)
- records the resolved revision to `$DRAVYN_HOME/chromium/revision.txt`

### GN arguments and why they are chosen

Canonical args live in `browser/config/args.gn`:

| Arg | Value | Reason |
| --- | --- | --- |
| `is_debug` | `false` | release-speed baseline; debug builds link/run far slower |
| `symbol_level`, `blink_symbol_level`, `v8_symbol_level` | `0` | no debug info: tens of GiB less disk and much lower linker memory |
| `is_component_build` | `true` | many small libs instead of one huge binary: seconds-fast incremental relinks and low peak RAM — the standard dev setup |
| `enable_nacl` | `false` | Native Client is unused by Dravyn |
| `use_remoteexec` | `false` | no remote execution backend locally |

This profile targets iteration speed on the ~15 GiB WSL guest, not
shipping performance. A future release-quality profile can be added as an
additional documented configuration.

### Resource-aware builds

A Chromium link job can transiently need ~3 GiB of RAM. Job counts resolve
in this order:

1. `--jobs N`
2. `DRAVYN_BUILD_JOBS`
3. auto: available RAM / 3 GiB per job, capped by CPU count, minimum 1

On a 15 GiB WSL guest this typically selects 4 jobs. Never run
`autoninja -j$(nproc)` on memory-constrained machines; it invites OOM kills.

### Running through WSLg

`chromium-run.sh` requires `WAYLAND_DISPLAY` or `DISPLAY` to be set
(WSLg provides both). It launches with:

```text
--user-data-dir=$DRAVYN_HOME/runtime/dev-profile
--no-first-run
--no-default-browser-check
--ozone-platform-hint=auto
```

Your personal Chrome profiles are never touched.

## Clean / reset

```bash
rm -rf "$DRAVYN_HOME/chromium/src/out/Dravyn"   # build outputs only
rm -rf "$DRAVYN_HOME/runtime/dev-profile"       # dev profile
rm -rf "$DRAVYN_HOME"                           # everything (re-bootstrap after)
```

## Updating Chromium

Re-run bootstrap:

```bash
dravyn chromium bootstrap
```

It updates depot_tools, performs `gclient sync -D`, refreshes dependencies
and hooks, then records the new revision in `revision.txt`. Rebuild
afterwards. There is no magic pinned revision: reproducibility comes from
the recorded revision plus the tracked configuration in `browser/config/`.

## Environment variables summary

| Variable | Purpose |
| --- | --- |
| `DRAVYN_HOME` | workspace root override (default `~/.cache/dravyn`) |
| `DRAVYN_BUILD_JOBS` | default parallel job count for builds |
| `DRAVYN_REPO_ROOT` | helps the CLI find `scripts/` outside the repo checkout |

## Scope reminder

M1 builds a clean upstream Chromium. No fingerprint spoofing, no
anti-fraud/CAPTCHA/KYC bypass work belongs in this milestone.
