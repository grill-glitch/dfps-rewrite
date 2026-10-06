# M5 evidence — module install, cleanup, and the M4 tab on alioth

**Date**: 2026-10-06
**Device**: f748d277 (alioth, crDroid Android 16 / SDK 36, arm64-v8a)
**Artifact**: `build/package/uperf-magisk.zip` (2 431 779 B), `bin/uperf`
2 251 536 B (2.14 MiB), md5 `8a88ff26…`
**Context**: this is the first install of the *real* module (not a hand-copied
binary), so it also closes M4 — the refresh-rate tab travels in `webroot/`.

## What M5 actually turned out to be

The plan said "delete `cpp/dfps/` + 3 CMakeLists edits + NOTICE −3". The tree
disagreed, and the tree won:

| Plan | Reality |
|---|---|
| delete `cpp/dfps/` | `cpp/dfps/` holds **live** code: `UPERF_SRCS` compiles `platform/*`, the four event sources, and `utils/*` from it. Deleting the directory breaks the build. Only the dfps *executable* is dead. |
| 3 CMakeLists edits | `cpp/CMakeLists.txt` only ever added `dfps/thirdparty` and `uperf` — nothing to unlink. The three `cpp/dfps/` CMakeLists existed solely to build the dfps executable. One comment in `cpp/uperf/CMakeLists.txt` was stale. |
| NOTICE −3 (spdlog, scnlib, dfps) | **No change.** `scnlib` is included by `source/modules/cgroup_listener.cpp` and `source/utils/misc.cpp`; `spdlog` by `cpp/uperf/{app_main,bridge,m0_event_tap}.cpp`. Both stay. dfps keeps its own `NOTICE` inside `cpp/dfps/`, which is still correct because the platform layer stays. |

So M5 = delete the dead dfps business layer, and **say why the rest stays**.

## Removed (all verified unreferenced)

```
cpp/dfps/source/main.cpp              229 lines   dfps' supervisor
cpp/dfps/source/dfps.{h,cpp}           91 lines   dfps' module assembly
cpp/dfps/source/modules/dynamic_fps.{h,cpp}  409 lines   the policy Rust replaced
cpp/dfps/magisk/**                     12 files   dfps' own module packaging
cpp/dfps/CMakeLists.txt, source/CMakeLists.txt      built the above from a GLOB
```

**Proof they were dead**: after the deletion, a from-scratch
`rm -rf build/aarch64-linux-android23 && build.sh Release make pack check`
configured, compiled, linked, and passed every gate — and the binary is the
same size as before (2 251 536 B). A file that mattered could not survive that.

The two CMakeLists were removed rather than left, because they build the
`dfps` target from `GLOB_RECURSE *.cpp` — left in place they would re-glob the
deleted sources (or the platform layer alone, which is not a valid target) the
moment anyone added the directory as a subdirectory.

`cpp/dfps/DFPS_VENDOR.md` records all of this, so the tree is documented as
**no longer a verbatim copy** of upstream dfps — previously it claimed exactly
two modified files and nothing removed.

## Kept, and why

* `platform/` (`cobridge`, `delayed_worker`, `heavy_worker`, `inotifier`,
  `module_base`) — in `UPERF_SRCS`.
* `modules/{cgroup_listener, input_listener, offscreen_monitor, topapp_monitor}` —
  in `UPERF_SRCS`; these are the four event sources whose events `dfps_rs` now
  consumes. The whole point of the embedded design is that these are shared.
* `utils/` — in `UPERF_SRCS`.
* `thirdparty/{spdlog, scnlib}` — see the NOTICE row above.
* `source/version.c.in` — `configure_file`d into `cpp/uperf`; it is what makes
  `version.h` / `GetGitCommitHash()` work, and the reason every build stamps a
  fresh commit hash into the binary.
* `LICENSE`, `NOTICE`, `README.md`, `build.sh`, `.clang-format`, `.gitignore` —
  harmless, and `build.sh` documents the flags `cpp/uperf/CMakeLists.txt` was
  derived from.

## Install

```
adb push build/package/uperf-magisk.zip /data/local/tmp/
adb shell su -c 'ksud module install /data/local/tmp/uperf-magisk.zip'
```

`ksud module install` **cannot complete non-interactively**: `setup.sh`'s
`check_asopt()` ends in

```sh
while [ "$key_click" = "" ]; do
    key_click="$(getevent -qlc 1 | awk '{ print $3 }' | grep 'KEY_')"
    sleep 0.2
done
```

waiting for a hardware volume key. `input keyevent` cannot supply it (it
injects above `/dev/input`, which is what `getevent` reads). `sendevent` on the
`gpio-keys` node (`/dev/input/event5`) **can**:

```
sendevent /dev/input/event5 1 115 1     # EV_KEY KEY_VOLUMEUP press
sendevent /dev/input/event5 0 0 0       # SYN_REPORT
sendevent /dev/input/event5 1 115 0     # release
sendevent /dev/input/event5 0 0 0
```

With that, the install ran to completion:
`❗您已确认更新` … `- Module installed successfully!`

KernelSU stages to `/data/adb/modules_update/uperf` and marks
`/data/adb/modules/uperf/update`; a reboot applies it.

### Staging verified before rebooting

The staged tree was compared file-by-file against the zip — **52 files, 0
content differences**. The 70 files present in the zip but not staged are
`META-INF/**` (the installer always strips it) and `config/*` (12 per-SoC JSONs
+ `perapp_powermode.txt` + `dfps.default.txt`): `install_uperf()` copies the
one matching SoC config out to `$USER_PATH/uperf.json` and then does
`rm -rf $MODULE_PATH/config`.

## Post-reboot

```
boot_completed after ~20s
/data/adb/modules/uperf/update   -> gone (applied)
/data/adb/modules/uperf/script/dfps.sh -> present, md5 4f5ae40d… (== host)
```

daemon log:

```
08:35:28 I Rust: dfps loaded (1 rules, universal=60/120, offscreen=30/90)
08:35:28 I Rust: dfps timer thread started
08:35:28 I Rust: preset/hint watcher started
```

Two `uperf` processes (daemon + worker, fresh PIDs). `/sdcard/Android/yc/uperf/`
intact with `dfps.txt`, `dfps_cur.txt`, `cur_powermode.txt`, `orig_governor.txt`.

**A false alarm worth recording.** Right after boot, `/sdcard` was not mounted
and `ls /sdcard/` returned "No such file or directory", and `0 uperf` processes
were running — while `boot_completed` was already `1`. That is file-based
encryption plus `wait_until_login` in the module's `initsvc.sh`, not a broken
module: `dumpsys user` showed `RUNNING_UNLOCKED`, unlock `+3s` before the check.
Waiting ~45 s brought up the FUSE view and the daemon.

## M4 — the refresh-rate tab's contract, verified on the device

The tab is a thin UI over `script/dfps.sh`, so the contract is checkable from
adb (which is how the module's own control entry is tested):

```
$ sh /data/adb/modules/uperf/script/dfps.sh status
cur=90
config.path=/sdcard/Android/yc/uperf/dfps.txt
config.exists=1

$ sh /data/adb/modules/uperf/script/dfps.sh info
lines=3
rules=* 60 120|- 30 90|com.android.settings 90 120|

$ sh /data/adb/modules/uperf/script/dfps.sh set-rule com.android.settings 60 120
rule.pkg=com.android.settings
rule.idle=60
rule.active=120
rule.got=com.android.settings 60 120
rule.ok=1
```

and the daemon picked the write up on its next tick:

```
08:36:46 I Dfps: dfps.txt reloaded (1 rules, universal=60/120)
```

Restoring `90 120` produced `rule.ok=1` and a second reload at `08:36:49`.

The bundle ships the tab: `webroot/assets/index-Y7T98bzY.js` contains
`tab_dfps` and a reference to `script/dfps.sh`.

### A device-found bug in `dfps.sh`

`info` first returned **7 lines** — comments and `/tunables` included — where
it should return 3. The filter used BRE alternation:

```sh
grep -c -v '^[[:space:]]*$\|^#\|^/' dfps.txt
```

Android's `grep` is toybox, whose BRE parser has no `\|`, so the pattern
matched nothing and `-v` filtered nothing. `-E` gives 3. Reproduced both ways
on the device before and after the fix. This is the class of bug that only a
real device shows — the host's GNU grep accepts `\|` happily.

**Not verified**: the tab's actual appearance in the KernelSU manager. That is
a visual check in the manager UI and is the only remaining item; everything the
tab *calls* is confirmed working above.

## Found, not fixed: the A-SOUL (asopt) companion install is Magisk-only

The install output contains a line that looks alarming and is worth explaining:

```
/data/adb/modules_update/uperf/script/setup.sh: line 156: magisk: not found
```

`check_asopt()` in `script/setup.sh` ends its first-install path with

```sh
magisk --install-module "$MODULE_PATH"/modules/asoulopt.zip
```

This device runs **KernelSU-Next**, which has no `magisk` binary (only `ksud`),
so the command fails, the `asoulopt.zip` is then deleted by the trailing
`rm -rf "$MODULE_PATH"/modules/asoulopt.zip`, and the companion module is never
installed — `ls /data/adb/modules/` shows no `asoul_affinity_opt`, only
`KPatch-Next`, `fixchinacarrier-ksu`, `mountify`, `rezygisk`, `uperf`,
`zygisk_vector`.

This is **pre-existing uperf Game-Turbo behaviour, not a dfps-rs regression**:
the same line fails on any KernelSU install of this module, before this work
too. The module itself installs and runs correctly regardless — the failure is
confined to the optional A-SOUL thread-placement companion.

Not fixed here because it is outside dfps-rs's scope (the map rules out adding
uperf-only features) and it belongs to a feature with its own interactive flow.
The one-line shape of a fix, should it be wanted, is to fall back to the
KernelSU CLI:

```sh
if command -v magisk >/dev/null 2>&1; then
    magisk --install-module "$MODULE_PATH"/modules/asoulopt.zip
else
    ksud module install "$MODULE_PATH"/modules/asoulopt.zip
fi
```

## Rollback

```
su -c 'ksud module uninstall uperf'    # or delete /data/adb/modules/uperf
```
The previous binary is still at `/data/local/tmp/uperf.bak` if only the
daemon needs reverting.
