# T09 — SwitchRefreshRate call-frequency control

## Question

dynamic_fps.cpp's SwitchRefreshRate(force=false) is called from many event
handlers (input.touch every finger-frame, topapp.switch, etc.). The sf backdoor
is `service call SurfaceFlinger 1035 i32 N` — a binder transaction that is
nontrivial. Decide:

1. Should dfps-rs call the backdoor every event, or dedupe by (target_hz,
   in-flight token)? dfps does dedupe by comparing new vs current rule; only
   switches when changed.
2. force=true means full flexibility-token dance (1036 1 → 1035 -1 → 1035 idx →
   1036 0). Is force=true actually used anywhere upstream, or vestigial?
   (grep dynamic_fps.cpp + main.cpp for `force=true` callers.)

## Evidence

- source/modules/dynamic_fps.cpp:97-99 (useSfBackdoor path)
- source/modules/dynamic_fps.cpp:245-260 (SyncCallSurfaceflingerBackdoor)
- source/modules/dynamic_fps.cpp:302-321 (SyncRefreshRate overloads)

## Required output

One paragraph: dedupe strategy + whether force=true survives.

## Resolution (2026-10-06)

### Dedupe strategy: keep upstream's `(hz != curHz_) || force` gate

dynamic_fps.cpp:308-310:

```cpp
if (force == false && hz == curHz_) { return; }
```

That's the only dedupe in upstream. dfps-rs keeps it verbatim with T04's
`Option<i32>` for `cur_hz` so the first call (after init) goes through
without a special case.

### force=true callers (3 sites, all genuine)

```
dynamic_fps.cpp:259  OnTopAppSwitch     -> SwitchRefreshRate(true)
dynamic_fps.cpp:272  OnOffscreen (off)  -> SwitchRefreshRate(true)
dynamic_fps.cpp:277  exitOffscreen wake -> SwitchRefreshRate(true)
```

In all three, the caller's intent is "I just observed a state change
(app switch / screen off), make sure the rate actually moved regardless of
whether `curHz_` already equals the target." `OnInput()` and `OnInputScene()`
call `SwitchRefreshRate(false)` — they're high-frequency events where dedupe
matters.

**Decision: keep `force=true` and the bool parameter.** Remove `forceSwitch_`
module-level scratch, fold it into a closure capture (see code sketch in the
M1 stub, `rust/uperf-core/src/dfps_task.rs`).

### settings-put path needs no extra dedupe

`settings put system peak_refresh_rate N` is cheap (~ms) and Android's
SettingsProvider debounces internally. We **don't** add a `time::Instant`
system target or per-event guard on top of upstream's. If M3 evidence shows
audit log spam (SettingsProvider log per call), revisit.

### sf-backdoor path keeps upstream's 4-call force dance

Even though T01 said sf backdoor is dead on alioth, dfps-rs keeps the
multi-step `force` path (`:250-260`) so the binary works on devices where
the backdoor is alive. Users with `useSfBackdoor=true` on a non-MIUI device
get the same semantics as upstream; users on alioth keep `useSfBackdoor` at
its default `false` and the dead path never runs.

## Outcome

T05 resolved. No new files. M1 code sketch follows ticket-T05 + ticket-T06.
