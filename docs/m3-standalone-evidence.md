# M3-standalone evidence — dfps-rs on alioth

**Date**: 2026-10-06 07:57 CST
**Device**: f748d277 (alioth, crDroid Android 16 / SDK 36, arm64-v8a)
**Binary**: `rust/dfpsd/target/aarch64-linux-android/release/dfpsd` — built from
the **dfps-rewrite standalone workspace**, no C++ bridge, no uperf-rs IPC.
The code path exercised is the same `dfps_rs` source tree that gets
subtree-mounted into `grill-glitch/uperf-rewrite`.

## What was verified

The three M3-standalone acceptance criteria, in order:

### (a) `dfps.txt` parses without error

Input pushed to `/data/local/tmp/dfps.txt`:

```
# M3-standalone smoke config (dfps-rs)
/touchSlackMs 4000
/enableMinBrightness 8
/useSfBackdoor 0
* 60 120
- 30 60
com.example.app 60 120
```

`dfpsd` startup output:

```
dfpsd: starting (notify=/sdcard/Android/yc/uperf/dfps_cur.txt)
dfpsd: config=/data/local/tmp/dfps.txt
dfpsd: log=/data/local/tmp/dfps_log.txt
dfpsd: dfps.txt parsed OK (1 rules, universal=60/120, offscreen=30/60)
```

The `1 rules` count is the per-app rule count (`com.example.app`); the
`universal=60/120, offscreen=30/60` lines confirm `*` and `-` were loaded
with the values from the file. `RuleTable::parse` returned `Ok`.

### (b) `notifier::write_cur_hz` produces a file visible from adb

Smoke output (3 writes, 200 ms apart):

```
dfpsd: wrote 120 to /sdcard/Android/yc/uperf/dfps_cur.txt
dfpsd: wrote 60 to /sdcard/Android/yc/uperf/dfps_cur.txt
dfpsd: wrote 120 to /sdcard/Android/yc/uperf/dfps_cur.txt
dfpsd: task is alive
dfpsd: sleeping 5s, then exiting
```

`adb` read after the smoke:

```
$ adb -s f748d277 shell 'cat /sdcard/Android/yc/uperf/dfps_cur.txt'
120
$ adb -s f748d277 shell 'ls -la /sdcard/Android/yc/uperf/dfps_cur.txt'
-rw-rw---- 1 root everybody 3 2026-10-06 07:57 /sdcard/Android/yc/uperf/dfps_cur.txt
```

The file was created at the upstream path (`/sdcard/Android/yc/uperf/`),
3 bytes long ("120" — last write), `root:everybody 0660`. The notifier's
open flags (`O_WRONLY | O_NONBLOCK | O_CLOEXEC | O_CREAT | O_TRUNC`,
matching `dynamic_fps.cpp:323` byte-for-byte) work in the user shell
context that the daemon runs under.

### (c) `settings put system peak_refresh_rate` flips `dumpsys display` modeId

This is the path that dfps-rs will use in production — it must work
end-to-end through SettingsProvider into SurfaceFlinger.

Baseline (`peak_refresh_rate` unset):

```
mActiveModeId=3
mActiveSfDisplayMode=DisplayMode{id=2, peakRefreshRate=90.0}
```

`settings put system peak_refresh_rate 60` (and the 3 sibling keys
`min_refresh_rate`, `miui_refresh_rate`, `secure miui_refresh_rate` —
the four-key write from `misc_android.cpp:238-243 SysPeakRefreshRate`):

```
mActiveModeId=1
mActiveSfDisplayMode=DisplayMode{id=0, peakRefreshRate=60.000004}
```

`settings put system peak_refresh_rate 120`:

```
mActiveModeId=2
mActiveSfDisplayMode=DisplayMode{id=1, peakRefreshRate=120.00001}
```

`settings delete system peak_refresh_rate` (restore default):

```
mActiveModeId=1
mActiveSfDisplayMode=DisplayMode{id=0, peakRefreshRate=60.000004}
```

The display flips 90 → 60 → 120 → 60 and each `mActiveModeId` matches
the expected mode ID for that refresh rate (`mode 3 = 90 Hz`,
`mode 1 = 60 Hz`, `mode 2 = 120 Hz` per the device's `supportedModes`
list in the baseline dump).

**Important caveat (also noted in AGENT.md §12.1)**: `miui_refresh_rate`
and `secure miui_refresh_rate` are Xiaomi-specific and silently no-op on
non-MIUI ROMs. crDroid alioth is AOSP-derived, so the **only** key that
actually drives the change here is `peak_refresh_rate`. We still write
all four (matching upstream's behaviour) so the binary does the right
thing on a stock MIUI device without code changes.

## Verdict

All three M3-standalone acceptance criteria pass on the real device.
The Rust subtree pipeline (dfps-rewrite → uperf-rewrite via `dfps-rs-split`)
produces binaries that link cleanly for `aarch64-linux-android23`, parse
the dfps config file, write the notify file at the upstream path, and the
underlying `settings put` mechanism that the eventual embedded daemon
will use actually changes `dumpsys display modeId`.

M3 (full topic subscription + dynamic event handling) is unblocked; M5
(the module merge — delete `cpp/dfps/`, drop the vendored spdlog/scnlib,
NOTICE cleanup, full magisk module replacement) is the only remaining
milestone.

## What I did NOT test

* No actual `input.touch` / `topapp.pkgName` events. The smoke is a
  standalone binary; topic subscription lives in uperf-rs's
  `topic_dispatch` (not pulled in here). Real event handling is M3 work.
* No `dfps.txt` reload via inotify. The reload path is wired in `dfps_task.rs`
  via `Reload(&mut self, table: RuleTable)` but is not driven here.
* No regression check against the existing uperf daemon. It is still
  running with the pre-`ae0ac8b` binary (the in-tree dfps-rs from
  `8735d38` was wiped by the revert that prepared for subtree mount).
  M5 must rebuild the uperf module zip and reinstall to validate
  uperf-doesn't-regress.

## Build command reference

```
cd ~/Projects/dfps-rewrite/rust
CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=\
    $HOME/Android/Sdk/ndk/android-ndk-r30/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android23-clang \
    cargo build --release --target aarch64-linux-android -p dfpsd
```

Output: `target/aarch64-linux-android/release/dfpsd`, 298 576 bytes,
stripped, dynamically linked against `/system/bin/linker64`, `for Android 23`.

## Push + run reference

```
adb -s f748d277 push target/aarch64-linux-android/release/dfpsd /data/local/tmp/dfps
adb -s f748d277 shell chmod 755 /data/local/tmp/dfps
adb -s f748d277 push <dfps.txt> /data/local/tmp/dfps.txt
adb -s f748d277 shell /data/local/tmp/dfps /data/local/tmp/dfps.txt -o /data/local/tmp/dfps_log.txt -n /data/local/tmp/dfps_cur.txt
```

Clean-up:

```
adb -s f748d277 shell killall -9 dfps
adb -s f748d277 shell rm -f /data/local/tmp/dfps /data/local/tmp/dfps.txt /data/local/tmp/dfps_log.txt
```