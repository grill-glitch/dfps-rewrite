# dfps Rust rewrite

A full rewrite of [yc9559/dfps](https://github.com/yc9559/dfps) in Rust, **embedded
into** the
[grill-glitch/uperf-rewrite](https://github.com/grill-glitch/uperf-rewrite) KernelSU
module.

The rewrite is planned via wayfinder — see
[`AGENT.md`](AGENT.md) for the project guide and
[`.hermes/wayfinder/map.md`](.hermes/wayfinder/map.md) for the ticket map.

## Status

* Planning: complete (3 research tickets resolved, 9 execution tickets resolved)
* Code: M1+M2 landed in `rust/uperf-core/src/dfps_rs/` (15/15 unit tests pass);
  mounted into uperf-rewrite as a **git subtree**
* Real device: M3 (end-to-end on alioth) and M5 (module merge) pending
* Target device: alioth (`f748d277`, crDroid Android 16, KSU Next)
* License: Apache-2.0 (matches upstream)

## What changes vs upstream

| | Upstream dfps | dfps-rs |
|---|---|---|
| Implementation | C++ (`dynamic_fps.cpp`, 328 lines + platform) | Pure Rust (slot into uperf-rs binary) |
| Process model | Standalone `dfps` binary | Runs in the same Rust binary as uperf-rs |
| Vendor libs | spdlog + scnlib (~38 k lines vendored) | None (Rust standard ecosystem) |
| USER_PATH | `/sdcard/Android/yc/dfps/` | `/sdcard/Android/yc/uperf/` (shared) |
| Refresh-rate API | `service call SurfaceFlinger 1035` (sf backdoor) | `settings put system peak_refresh_rate` (verified path on alioth) |

The sf backdoor is **structurally refused** on alioth (root + SELinux Enforcing),
see [`docs/research/sf-backdoor-probe-verdict.md`](docs/research/sf-backdoor-probe-verdict.md).

## Why this rewrite exists

The upstream dfps is a clean, small C++ program (~4 k lines total) that watches the
top-app and input events and switches the screen refresh rate. The C++ platform
layer (`inotify`, `cobridge`, `delayed_worker`, `module_base`, …) is duplicated
between dfps and uperf; embedding dfps-rs in the uperf-rs binary removes the
duplication and lets both daemons share the same event bus.

For the full motivation see [`AGENT.md`](AGENT.md).

## Repository layout

```
dfps-rewrite/
├── AGENT.md                                    ← project guide
├── LICENSE                                     ← Apache-2.0
├── README.md                                   ← you are here
├── .wayfinder-fog.md                           ← superseded-fog archive
├── .hermes/wayfinder/
│   ├── map.md                                  ← ticket index
│   └── ticket-T0{4..9}-*.md                    ← ticket detail
├── docs/
│   ├── subtree-workflow.md                     ← dfps-rs ↔ uperf-rewrite subtree recipe
│   └── research/
│       ├── dfps-config-format.md               ← upstream dfps config schema
│       └── sf-backdoor-probe-verdict.md        ← why the sf backdoor is dead on alioth
├── scripts/
│   └── alioth-dfps-smoke.sh                    ← M3-standalone device smoke
├── rust/uperf-core/src/dfps_rs/                ← ★ dfps-rs source (source of truth)
├── magisk/                                     ← dfps.sh + dfps.default.txt (copied to host)
└── webui/pages/dfps.js                         ← refresh-rate tab (copied to host)
```

**This repo owns the dfps-rs code.** uperf-rewrite mounts
`rust/uperf-core/src/dfps_rs/` as a git subtree; the shell/JS artifacts are
copied across manually. Edit the source here, never in uperf-rewrite. See
[`docs/subtree-workflow.md`](docs/subtree-workflow.md) for the full recipe.

## Author

gril-glitch (rewrite). Original dfps by Matt Yang (yc9559), Apache-2.0.
