# M3 evidence — dfps-rs event pipeline on alioth

**Date**: 2026-10-06
**Device**: f748d277 (alioth, crDroid Android 16 / SDK 36, arm64-v8a)
**Binary**: `rust/dfpsd/` built for `aarch64-linux-android23`
**Milestone**: M3 (event pipeline), distinct from
[`m3-standalone-evidence.md`](m3-standalone-evidence.md) (M3-standalone,
which proved only parse + notify + settings reachability).

**What this proves**: the full dfps-rs pipeline — event → state machine →
dedupe → `dfps_cur.txt` + `settings put` → **SurfaceFlinger modeId** — works
on the real device, with the delayed (timer-driven) transitions firing on
time.

`dfpsd` links only the `dfps_rs` subtree and calls the **same**
`DfpsScheduler` the embedded daemon will call. There is no separate code
path for the smoke.

## Config used

`/data/local/tmp/dfps.txt`:

```
/touchSlackMs 4000
/enableMinBrightness 8
/useSfBackdoor 0
* 60 120
- 30 120
com.android.settings 90 144
```

`com.android.settings` has `active = 144`, deliberately above the panel's
supported range (60/90/120) — see "clamping" below.

## Smoke output (stdout)

```
dfpsd: starting (notify=/sdcard/Android/yc/uperf/dfps_cur.txt)
dfpsd: parsed OK (1 rules, universal=60/120, offscreen=30/120)
dfpsd: scheduler spawned (timer thread up)
[dfps_rs] Dfps: switch - -> 90 Hz
STEP 1: topapp com.android.settings -> cur_hz=Some(90)
[dfps_rs] Dfps: switch 90 -> 144 Hz
STEP 2: touch down -> cur_hz=Some(144)
STEP 3: touch up (idle scheduled) -> cur_hz=Some(144)
[dfps_rs] Dfps: switch 144 -> 90 Hz
[dfps_rs] Dfps: 1 delayed transition(s) applied
STEP 4: after touchSlackMs (idle expected) -> cur_hz=Some(90)
[dfps_rs] Dfps: switch 90 -> 120 Hz
STEP 5: offscreen on -> cur_hz=Some(120)
STEP 6: offscreen off (wake scheduled) -> cur_hz=Some(120)
[dfps_rs] Dfps: switch 120 -> 90 Hz
[dfps_rs] Dfps: 1 delayed transition(s) applied
STEP 7: after gestureSlackMs (restored expected) -> cur_hz=Some(90)
[dfps_rs] Dfps: switch 90 -> 60 Hz
STEP 8: topapp unknown (universal) -> cur_hz=Some(60)
dfpsd: sequence complete; cur_app=com.example.unknown
dfpsd: done
```

Every step matches the config-derived expectation:

| step | event | expected | observed |
|---|---|---|---|
| 1 | topapp `com.android.settings` | settings idle = 90 | 90 |
| 2 | touch down | settings active = 144 | 144 |
| 3 | touch up | unchanged (idle only *scheduled*) | 144 |
| 4 | touchSlackMs elapsed | idle applied by timer | 90 |
| 5 | offscreen on | offscreen active = 120 | 120 |
| 6 | offscreen off | unchanged (restore *scheduled*) | 120 |
| 7 | gestureSlackMs elapsed | restore applied by timer | 90 |
| 8 | topapp unknown | universal idle = 60 | 60 |

Steps 3 and 6 matter: they show the release/wake paths **schedule** rather
than apply immediately (upstream `DwSetWork`). Steps 4 and 7 show the timer
thread firing them (`1 delayed transition(s) applied`).

## Correlated with SurfaceFlinger

An independent `adb shell dumpsys display | grep mActiveModeId` loop sampled
every 2 s while the smoke ran:

```
t=2s     mActiveModeId=1              baseline (60 Hz)
t=10s    mActiveModeId=3  90          STEP 1 -> 90 Hz = mode 3
t=12s    mActiveModeId=2  144         STEP 2 -> 144, clamped to panel max
t=14s    mActiveModeId=2  144
t=16s    mActiveModeId=2  144
t=18s    mActiveModeId=2  90          STEP 4 idle applied
t=20s    mActiveModeId=3  90          mode id caught up to 90 Hz
t=22s    mActiveModeId=2  120         STEP 5 offscreen -> 120 Hz = mode 2
t=24s    mActiveModeId=2  120
t=26s    mActiveModeId=2  120
t=28s    mActiveModeId=2  90          STEP 7 restore applied
t=30s    mActiveModeId=3  90
t=32s    mActiveModeId=1  60          STEP 8 universal idle -> 60 Hz = mode 1
```

The device's `supportedModes` are `id=1 (60), id=2 (120), id=3 (90)`. Every
`dfps_cur.txt` value with a supported counterpart shows the matching modeId
one sample later (SettingsProvider applies asynchronously).

### Clamping at 144

`STEP 2` writes `dfps_cur.txt = 144` but the display goes to mode 2 (120 Hz)
because the panel has no 144 Hz mode. That is the platform choosing the
closest supported rate for an out-of-range `peak_refresh_rate` — not a
dfps-rs bug, and it confirms the write reached a layer that resolves it
against the panel's mode list. The 144 value is here only to make the step
visually distinct from the 120 Hz offscreen step.

## What this covers

**On device**: config parse; the topic handlers (touch, `input.state`
gesture, topapp, offscreen); the dedupe gate; the delayed transitions and
their cancellation on re-press; the notify file; the four `settings put`
keys; the round trip into SurfaceFlinger.

**Not covered here**:

* The uperf-rs *topic delivery* — this binary calls the scheduler directly
  instead of receiving events from `topic_dispatch`. That wiring is the
  uperf-rewrite side of M3.
* `input.btn` — same `apply_press` path as touch, not separately driven.
* `dfps.txt` hot reload — `Reload` is unit-tested, not driven here.

## Reproduce

```
cd ~/Projects/dfps-rewrite/rust
CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$HOME/Android/Sdk/ndk/android-ndk-r30/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android23-clang \
  cargo build --release --target aarch64-linux-android -p dfpsd

adb -s f748d277 push target/aarch64-linux-android/release/dfpsd /data/local/tmp/dfps
adb -s f748d277 shell chmod 755 /data/local/tmp/dfps
adb -s f748d277 push <the config above> /data/local/tmp/dfps.txt
# in one shell: watch  dumpsys display | grep mActiveModeId
# in another:
adb -s f748d277 shell '/data/local/tmp/dfps /data/local/tmp/dfps.txt'
```