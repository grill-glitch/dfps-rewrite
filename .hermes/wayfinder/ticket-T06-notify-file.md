# T10 — Notify file: same path or new?

## Question

dynamic_fps writes the active refresh rate index to a file (notifyPath_), opened
O_CREAT|O_TRUNC, written, closed. uperf-rs already watches a single-byte file
(sfanalysis.hint) under the same USER_PATH. Decide:

1. Reuse uperf-rs's USER_PATH for dfps notify file? Pro: one path for the
   user to inspect. Con: dfps's current users (vendored) may have an existing
   path on devices already running upstream dfps — must not break.
2. New path /data/dynamic_refresh_rate (vendored default)? Pro: no migration
   risk for upstream dfps users. Con: lives outside USER_PATH, harder to
   inspect via WebUI.

## Evidence required before deciding

The vendored dfps binary's compile-time default for notifyPath_ (grep source
for `dynamic_refresh_rate` or `notifyPath_` defaults). If vendored ships no
default and the path is config-driven, that's a different conversation.

## Resolution (2026-10-06)

### Default in vendored upstream: **config-driven, no compile-time default**

`source/modules/dynamic_fps.h:64` has `std::string notifyPath_;`
`source/modules/dynamic_fps.cpp:52` initialises from the constructor arg.
`source/dfps.cpp:42` forwards from `Dfps::Load(configPath, notifyPath)`.
`source/main.cpp:80` reads `notifyFile` (a global), which `main.cpp:195`
populates from `-n <path>` on argv.

`magisk/initsvc.sh:44` of the vendored module ships:

```sh
$BASEDIR/bin/dfps $DFPS_DIR/dfps.txt -o $DFPS_DIR/dfps_log.txt \
                 -n $DFPS_DIR/dfps_cur.txt
```

where `DFPS_DIR="/sdcard/Android/yc/dfps"`. So upstream's **shipped** path
is `/sdcard/Android/yc/dfps/dfps_cur.txt`. There is no `-n` default if argv
omits the flag (NotifyRefreshRate writes to whatever was constructed; if
`-n` was missing `NotifyRefreshRate` would `open("", ...)` and silently
no-op — file just never created).

### Embedded-mode path: reuse uperf USER_PATH

Map destination §1 + AGENT.md §7.1 already locked this: we ship
`dfps.txt / dfps_log.txt / dfps_cur.txt` under
`/sdcard/Android/yc/uperf/` (uperf-rs's `USER_PATH`). The upstream
`/sdcard/Android/yc/dfps/` tree is left alone on device and is **not**
read.

### Concrete `notifyPath_` value in dfps-rs

Hardcoded const at the top of `dfps_task.rs`:

```rust
pub(super) const DFPS_NOTIFY_PATH: &str = "/sdcard/Android/yc/uperf/dfps_cur.txt";
```

Path is also derivable from uperf-rs's existing `USER_PATH` constant (same
constant string in `pathinfo.sh`); if M3 evidence reveals a user-edited
USER_PATH override, dfps_task reads the same env var rather than hardcode.
This is a M1-only choice; refactor at M3 if needed.

### Rationale

* **No migration risk for the agent's own workflow** — we own the module
  directory.
* **Single inspection path for users.** One `ls /sdcard/Android/yc/uperf/`
  shows all runtime state (current powermode, hint, dfps rule, current hz).
* **No conflict with leftover `/sdcard/Android/yc/dfps/`** — that path's
  `dfps_cur.txt` is stale on device but nobody reads it.

## Outcome

T06 resolved. M1 code reads `/sdcard/Android/yc/uperf/dfps_cur.txt` (matches
AGENT.md §7.1). No new files.
