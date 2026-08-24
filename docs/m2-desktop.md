# M2 - Dravyn Desktop and Profile Manager

M2 turns the M1 Chromium foundation into an application that can create, persist, launch, stop, reset, and delete isolated browser profiles.

## Scope

M2 intentionally focuses on browser profile isolation and explicit network configuration for development, QA, privacy testing, and authorized automation.

It does **not** implement fingerprint spoofing, CAPTCHA/KYC bypass, anti-fraud evasion, or identity impersonation.

## Storage model

All mutable profile data remains outside the Git repository under `$DRAVYN_HOME` (default `~/.cache/dravyn`):

```text
$DRAVYN_HOME/
├── chromium/src/out/Dravyn/chrome
├── profiles/
│   └── <profile-id>/
│       ├── profile.json
│       └── user-data/
└── runtime/
    └── profile-processes/
        └── <profile-id>.pid
```

Every profile gets a dedicated Chromium `--user-data-dir`. Cookies, local storage, history, site permissions, and extension state therefore remain physically separated between profiles.

## Profile settings in M2

- name, notes, tags
- optional `http://` or `https://` start URL
- window size
- direct connection or an explicit HTTP/HTTPS/SOCKS5 proxy host and port
- running/stopped state and PID
- reset local browser data while keeping profile metadata

Proxy credentials are deliberately not stored or passed on the Chromium command line in M2.

## Desktop development prerequisites on Ubuntu 24.04 / WSLg

The frontend uses React + TypeScript + Vite and the desktop shell uses Tauri 2.

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  file \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libssl-dev \
  libwebkit2gtk-4.1-dev \
  libxdo-dev \
  wget
```

Node/pnpm and Rust must also be available. The current Dravyn development environment already manages Rust for the CLI and the user's workstation can use the existing Node/pnpm toolchain.

## Run the app

From the repository root:

```bash
cargo install --path crates/dravyn-cli --force
dravyn desktop
```

On the first run `scripts/desktop-dev.sh` installs frontend dependencies with pnpm, then starts `pnpm tauri dev`.

You can also start it directly:

```bash
cd apps/desktop
pnpm install
pnpm tauri dev
```

The already-built browser binary at `$DRAVYN_HOME/chromium/src/out/Dravyn/chrome` is reused. Editing the desktop UI does not rebuild Chromium.

## CLI profile commands

The desktop app and CLI share the same Rust profile store/runtime logic.

```bash
dravyn profile list
dravyn profile create "QA profile" --start-url https://example.com
dravyn profile show <id>
dravyn profile launch <id>
dravyn profile status <id>
dravyn profile stop <id>
dravyn profile reset <id>
dravyn profile delete <id>
```

## Runtime safety

The runtime PID file is treated as a hint, not authority. On Linux/WSLg Dravyn verifies `/proc/<pid>/cmdline` contains both the expected Dravyn Chromium binary and the expected profile `--user-data-dir` before reporting the process as running. A stale PID file is deleted rather than used to signal an unrelated process.

Profiles must be stopped before reset or deletion.
