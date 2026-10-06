# M3 embedded evidence — dfps-rs inside the uperf daemon on alioth

**Date**: 2026-10-06
**Device**: f748d277 (alioth, crDroid Android 16 / SDK 36, arm64-v8a)
**Binary**: `build/aarch64-linux-android23/runnable/uperf` (2 250 240 B, 2.14 MiB,
NEEDED libc/libdl/libm, dynamic PIE, stripped, RELRO + BIND_NOW)
**Milestone**: M3 embedded — the event pipeline **inside the real daemon**, as
opposed to [`m3-evidence.md`](m3-evidence.md) (the same scheduler driven by a
standalone harness).

## How it was run

1. Built the module: `build.sh Release make pack check` → all gates green,
   `magisk/webroot` (516 K) and `uperf-magisk.zip` (2 430 610 B) produced.
2. Confirmed the new code is in the binary:
   `strings ... | grep -E "Dfps: switch|dfps loaded|uperf-dfps|screen_brightness"`
   → all present.
3. Backed up the installed binary (`/data/local/tmp/uperf.bak`, md5
   `725a42dc…`) then replaced `/data/adb/modules/uperf/bin/uperf` with the
   new build (md5 `95e92424…`).
4. Wrote `/sdcard/Android/yc/uperf/dfps.txt` (the embedded path, AGENT.md §7.1):
   ```
   /touchSlackMs 4000
   /enableMinBrightness 8
   /useSfBackdoor 0
   * 60 120
   - 30 90
   com.android.settings 90 120
   ```
5. Restarted through the module's own control entry:
   `su -c 'sh /data/adb/modules/uperf/script/webui.sh restart'` → `restart.ok=1`.

## Boot-time evidence

```
07:  08:10:42 I Rust: dfps loaded (1 rules, universal=60/120, offscreen=30/90)
08:  08:10:42 I Rust: dfps timer thread started
536: 08:10:42 I Rust: sched pid=24644 "uperf" tid=24650 "uperf-dfps" ...
```

So the embedded daemon parsed the config, installed the scheduler, and the
`uperf-dfps` timer thread is a real thread in the daemon's process — visible
in uperf's own per-thread scheduler log.

## Event → switch chain (from the daemon's log)

```
554: 08:10:45 I EventTap: topapp.pkgName = com.example.piliplus
555: 08:10:45 I Rust:     topapp.pkgName = com.example.piliplus
556: 08:10:45 I Dfps:     switch - -> 60 Hz
...
1172: 08:11:19 I EventTap: topapp.pkgName = com.android.settings
1173: 08:11:19 I Rust:     topapp.pkgName = com.android.settings
1174: 08:11:19 I Dfps:     switch 60 -> 90 Hz
...
1735: 08:11:21 I EventTap: offscreen.state = true
1736: 08:11:21 I Rust:     offscreen.state = true
1755: 08:11:21 I Dfps:     switch 90 -> 90 Hz
...
1810: 08:11:24 I EventTap: offscreen.state = false
1811: 08:11:24 I Rust:     offscreen.state = false
1853: 08:11:28 I Dfps:     switch 90 -> 90 Hz
```

Every line is the **real topic path** — `EventTap` (C++) → `topic_dispatch`
(Rust) → `route_dfps` → `DfpsScheduler`. There is no test harness in this
chain.

Reading it:

* `topapp com.example.piliplus` → universal idle `60`. No per-app rule, so the
  universal rule applies.
* `topapp com.android.settings` → its idle `90` (`com.android.settings 90 120`).
* `offscreen.state = true` → offscreen rule `- 30 90` → its **active** `90`.
  The log says `90 -> 90` because the previous value was already 90: the
  switch is recorded because `force=true` bypasses the dedupe gate, exactly as
  upstream does on the offscreen edge (`dynamic_fps.cpp:272`).
* `offscreen.state = false` → `08:11:24` event, `08:11:28` switch. The four
  seconds between them is the `gestureSlackMs = 4000` delayed transition
  firing **from the `uperf-dfps` timer thread**, i.e. the wake path restores
  the rule only after the slack, not immediately.

## Refresh rate actually changed

Sampled through `dumpsys display` while launching Settings:

| moment | `dfps_cur.txt` | `mActiveModeId` |
|---|---|---|
| baseline | 60 | 1 (60 Hz) |
| after `am start com.android.settings` | **90** | **3 (90 Hz)** |
| screen off (`input keyevent 26`) | 90 | 1 |
| screen on | **90** | **3 (90 Hz)** |

So the settings write reaches SurfaceFlinger from inside the real daemon, and
the panel mode tracks `dfps_cur.txt`.

## Health

* `ps -A -o PID,STAT,NAME | grep -w uperf` → two processes (daemon + worker),
  both alive after the restart.
* `grep -c ' E '` over the fresh daemon log → **0** errors.
* The daemon's other subsystems (context scheduler, governors, preset watcher)
  keep logging normally across the restart — no uperf regression observed at
  this scale.

## `dfps.txt` hot reload — verified

`dfps.txt` is re-read **every watcher tick (1 s)** and installed only when the
bytes changed (content dedup), in `poll_dfps_txt`
(`rust/uperf-core/src/watch_task.rs`). Tick-based rather than event-driven on
purpose: `/sdcard` is a FUSE view, and a rename-into-place (`adb push`, `mv`,
most editors) surfaces on the *directory* watch — the reported path is the
directory, so a `Written(dfps.txt)` match misses exactly the writes that
matter. This mirrors the reasoning already in the file for the hint byte
("read on every tick, not only when inotify speaks — measured on /sdcard").

The keep-on-error rule lives in `DfpsScheduler::reload_from_text` (the
dfps-rs subtree, unit-tested by
`reload_from_text_keeps_the_old_table_on_a_bad_edit`).

Device log, in order:

```
08:21:31  EventTap: topapp.pkgName = com.android.launcher3
08:21:31  Dfps: switch - -> 60 Hz                          (boot table, universal idle 60)
08:21:39  Dfps: keeping the previous rules, dfps.txt does not parse:
          dfps.txt: default rule ('*') not specified        <- partial read mid-push
08:21:40  Dfps: dfps.txt reloaded (1 rules, universal=30/144)  <- valid edit accepted
08:21:44  Dfps: keeping the previous rules, ... default rule ('*') not specified
08:21:45  Dfps: keeping the previous rules, ... offscreen rule ('-') not specified
08:21:59  Dfps: switch 60 -> 90 Hz                          (topapp settings)
08:22:03  Dfps: switch 90 -> 30 Hz                          <- NEW universal idle
08:22:12  Dfps: dfps.txt reloaded (1 rules, universal=60/120)   <- config restored
```

Three things this proves, all on the real device through the real daemon:

1. **A valid edit takes effect without a restart.** The `30` at 08:22:03 is the
   new universal idle; the old table would have given `60`.
2. **A bad edit is rejected and the previous table stays in force.** Two
   malformed writes logged once each and changed nothing.
3. **A partial write is absorbed.** `08:21:39` is the poll catching the file
   after the push truncated it but before the content was complete. The daemon
   logged and kept running; the next tick read the finished file and installed
   it. Upstream throws here and the daemon dies — this is the case the
   keep-on-error rule exists for, and it happened unprompted on the first try.

## Not covered

### `input.touch` / `input.btn` / `input.state` could not be driven

The listener reads `/dev/input/event*` directly. Every synthetic path available
over adb is rejected or invisible to it:

* `input tap` / `input swipe` inject at the InputManager layer, above
  `/dev/input` — the log shows **zero** `input.*` events after them.
* `sendevent /dev/input/event2 …` (the real touchscreen, `fts_ts`) is written
  but the kernel does not deliver it to the listener — again zero events.

The `.bak` daemon log (the previous session) contains 11 816 real input events
(`input.touch` 5176, `input.state` 6512, `input.btn` 128), so the listener
itself works; it needs a **real touch**.

What is covered instead: the touch/gesture/button logic is unit-tested
(`press_marks_active_and_release_schedules_idle`, `repress_cancels_pending_idle`,
`gesture_overrides_to_universal_then_restores`), and the *routing* that the
device would exercise — `topic_dispatch` → `route_dfps` — is the same function
that carried the topapp and offscreen events above. The one untested link is
the listener's delivery of a physical touch into that already-proven path.

**To close it**: on the device, touch and hold the screen for ~2 s while
watching `/sdcard/Android/yc/uperf/uperf_log.txt`; a `Dfps: switch 60 -> 120`
followed ~4 s later by `Dfps: switch 120 -> 60` is the pass condition (120 is
the universal rule's active Hz).

## Rollback

```
su -c 'cp -f /data/local/tmp/uperf.bak /data/adb/modules/uperf/bin/uperf'
su -c 'sh /data/adb/modules/uperf/script/webui.sh restart'
rm -f /sdcard/Android/yc/uperf/dfps.txt      # leaves uperf unchanged without dfps
```
