# Dravyn Browser Configuration

M1 tracks an upstream Chromium checkout, build, and launch path. No
Dravyn-specific Chromium patches are applied in this milestone.

## Layout

```text
browser/
├── README.md            this file
├── config/
│   ├── chromium.toml    configuration of record (paths, channel strategy)
│   └── args.gn          canonical GN arguments used by the configure step
└── patches/
    ├── network/         future network-related patch series (empty in M1)
    ├── privacy/         future environment/privacy patch series (empty in M1)
    └── runtime/         future runtime/lifecycle patch series (empty in M1)
```

## Rules

1. Chromium source and build outputs never live inside this repository.
   They live under `$DRAVYN_HOME` (default `~/.cache/dravyn`).
2. `config/args.gn` is the single source of truth for GN arguments.
3. Patch directories are infrastructure placeholders. Any future change to
   Chromium behavior must land here as a reviewable patch series, never as
   undocumented source edits.

See `docs/chromium.md` for the full workflow.
