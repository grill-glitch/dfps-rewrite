# T11 — Merge dfps-rs into uperf magisk module

## Question

The endpoint is "embedded into uperf module" (one module.prop, one service.sh,
one webroot, one USER_PATH). What specifically changes in the uperf module to
ship dfps-rs?

1. bin/uperf — does it become the dfps-rs daemon too (single binary, two
   supervisors)? Or do we add bin/dfps?
2. service.sh — does it run both, or just uperf which then forks dfps?
3. module.prop description — does the versionCode bump?
4. customize.sh — anything to seed for dfps (no config to seed if dfps is
   zero-config, but check).
5. NOTICE — vendored cpp/dfps, spdlog, scnlib all deleted. New NOTICE entries:
   none (dfps-rs is in-repo Rust, covered by the existing uperf-rs entry).

## Required output

A short patch list for build.sh + magisk/. No code, just the diff outline.

## Resolution (2026-10-06)

### 1. Binary: one (`bin/uperf`)

uperf's C++ supervisor becomes the entry; Rust orchestrator + dfps_task +
cpu_task + sched_task + watch_task all run in the same `uperf` process.
**No** `bin/dfps` is added. The upstream `-n <path>` argv path is replaced
by a const in `dfps_task.rs` (T06).

### 2. service.sh: unchanged

`magisk/script/service.sh` already calls `uperf_start` (in
`libuperf.sh`) which spawns one daemon. No dfps-specific service.sh entry —
dfps starts when uperf starts (M1 wires `dfps_task` into
`uperf_rs_start()`).

### 3. module.prop: bump versionCode + description

* `versionCode` bumps by 1 (per release, no semantic change in the
  numbering for the dfps-rewrite merge).
* `description` gains a trailing " + dfps (Rust)" suffix so users
  installing from the merged tree know dfps is in.

### 4. customize.sh: no change

`SKIPUNZIP=0` + `sh $MODPATH/script/setup.sh` is unchanged. dfps has no
config to seed; if a fresh install wants a starter `dfps.txt`, copy the
upstream one (`/tmp/dfps-repo/magisk/config/dfps.txt`) into
`magisk/config/dfps.default.txt` and add a 2-line copy in customize.sh,
gated on `[ ! -e $USER_PATH/dfps.txt ]`. **Decision: do this** — without
   it, dfps_task.rs fails to load at boot and logs an error.

### 5. NOTICE: **delete** spdlog + scnlib + dfps entries; add no new ones

After M5:
* `NOTICE` removes:
    * The `<Distributor>` / `<License>` paragraphs whose names are "spdlog",
        "dfps project name", "scnlib" (3 entries).
* Adds: dfps-rs is the same repository and is covered by the
  existing uperf-rs entry. No new attribution.

### 6. magisk/bin: unchanged

`bin/uperf` (single ELF) is the only entry. dfps-rs is statically linked
into it (see T09).

### 7. magisk/webroot: no change

Existing 3-tab WebUI gains a 4th tab (T08).

## Patch list (no code yet)

```
modify:  magisk/module.prop                       (versionCode, description)
modify:  magisk/customize.sh                      (2-line seed for dfps.txt)
delete:  cpp/dfps/                                (3977 lines C++, with thirdparty)
modify:  cpp/CMakeLists.txt                       (drop add_subdirectory(dfps/...))
modify:  cpp/uperf/CMakeLists.txt                 (drop dfps sources from link)
modify:  magisk/script/libuperf.sh                (daemon name unchanged — see AGENT.md §8)
modify:  magisk/initsvc.sh                        (no change to uperf_start call)
delete:  magisk/webroot/pages/dfps.html           (N/A — see T08)
modify:  webui/index.html, webui/index.js         (add 4th tab — T08)
add:     magisk/script/dfps.sh                    (control entry for the new tab)
modify:  NOTICE                                   (delete 3 entries)
```

## Outcome

T07 resolved. Implementation blocked only on T08 (which files the new tab
reads/writes) and T09 (Cargo workspace placement). Both ticket resolutions
follow.
