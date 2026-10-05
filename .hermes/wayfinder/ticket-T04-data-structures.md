# T08 — dfps-rs data structures (dynamic_fps → Rust)

## Question

Map dynamic_fps.cpp's rule table (std::map<std::string, FpsRule> with special
pkg names `*` and `-`) to Rust idioms. Decide:

1. HashMap<String, FpsRule> vs BTreeMap? The match is `pkg -> rule` lookups,
   no ordering needed. HashMap is what dfps uses (std::map is just the closest
   no-hashmap std option in C++). Use HashMap.
2. FpsRule is `(int idle, int active)` plus implicit `isUniversal` / `isOffscreen`
   derived from the pkg name. In Rust, make those a single enum or just two
   special match arms? dfps uses string match on `pkgName == UNIVERSIAL_PKG_NAME`
   etc. Decide: enum vs string key.

## Evidence

- source/modules/dynamic_fps.cpp:137-145 (AddRule, isUniversal, isOffscreen)
- source/modules/dynamic_fps.h:21-25 (FpsRule struct)

## Resolution (2026-10-06)

### 1. Container: `HashMap<String, FpsRule>`

`std::map` is the closest no-hashmap std option in C++; the lookup is unordered
pkg→rule. `HashMap<String, FpsRule>` from `std::collections` matches semantics
with O(1) average. `BTreeMap` rejected: no sorted-iteration requirement in
`FindInvalidRule` (it's a `for`-loop reporting the first bad name; order doesn't
matter to the caller). `BTreeMap` adds ordering overhead for no win.

### 2. Special pkg names: separate `Option<FpsRule>` fields, not an enum

Decision: keep `*` and `-` as **two sentinel fields** (`universial:
Option<FpsRule>`, `offscreen: Option<FpsRule>`) on the `DynamicFps` task
struct, plus `HashMap<String, FpsRule>` for the rest. NOT an enum variant on
`FpsRule`, NOT a magic string key in the map.

Rationale: the hot path is `GetCurrentRule()` (dynamic_fps.cpp:186-200):

```cpp
if (pkgName == OFFSCREEN_PKG_NAME) { rule = offscreen_; }
else { auto it = rules_.find(pkgName);
       rule = (it != rules_.end()) ? it->second : universial_; }
```

That's three branches (offscreen hit / rules hit / fallback to universal). With
an enum, every lookup pays a discriminant check even when the pkg is a normal
app name. With separate `Option<FpsRule>` fields + HashMap, the same code is:

```rust
let rule = if pkg == OFFSCREEN_PKG { self.offscreen }
           else if let Some(r) = self.rules.get(&pkg) { *r }
           else { self.universial };
```

Branch-for-branch identical, no synthetic enum disciminant cost.

### 3. FpsRule shape: `struct FpsRule { idle: i32, active: i32 }`

Verbatim. Validation in `FindInvalidRule` (`:164-184`):
- `idle == -1 && active == -1` → "default rule" (rule off signal)
- `idle < 20 && active < 20` → "sf-backdoor rule" (only valid when
  `useSfBackdoor=true`)

We don't fold `is_default` / `is_backdoor` booleans into the struct because
they're computed predicates of `(idle, active, useSfBackdoor_)`, not stored
state. Keep them as functions on `FpsRule`.

### 4. `curHz_ == INT32_MAX` sentinel → `Option<i32>`

dynamic_fps.cpp:58 initializes `curHz_(INT32_MAX)` as "never switched" so the
first call (with `force=false`) isn't suppressed by the dedupe check at
`:308-310`. In Rust, `cur_hz: Option<i32> = None` expresses this without a
sentinel — `if hz == cur_hz` becomes `if Some(hz) == self.cur_hz`. Reads of
`cur_hz` for logging add an `unwrap_or(-1)` or `format!("{:?}", ...)`.

### 5. `forceSwitch_` is single-thread task state

It's mutated by `SwitchRefreshRate(bool)` at `:284-302`, read by the heavy-worker
lambda at `:304-320`, then reset to `false`. In uperf-rs the heavy worker
already runs synchronously per task (`HwSetWork` posts to a dedicated queue
drained on the same thread), so we can use `Cell<bool>` rather than
`AtomicBool`. Saves a fence on every input event.

### 6. State fields to translate verbatim

| C++ field | Rust equivalent |
|---|---|
| `touchPressed_`, `btnPressed_`, `active_`, `lowBrightness_` | `Cell<bool>` (intra-task, single thread) |
| `isOffscreen_`, `hasUniversial_`, `hasOffscreen_` | `bool` (set once at LoadConfig, never mutated after) |
| `curApp_`, `overridedApp_` | `RefCell<Option<String>>` or just `String` (the empty string is already "no override" in upstream — see `:188`: `overridedApp_.empty() ? curApp_ : overridedApp_`) |
| `touchSlackMs_`, `gestureSlackMs_`, `enableMinBrightness_` | `i64`, `i64`, `i32` (clamped at parse time) |
| `curHz_` | `Option<i32>` (None = sentinel) |
| `forceSwitch_` | `Cell<bool>` (heavy-worker only) |
| `dwInput_`, `dwGesture_`, `dwWakeup_`, `hw_` | uperf-core's `DelayedWorker` handles + `HeavyWorker::Handle` (already exist in uperf-rs) |

`brightnessTimer_` reuses uperf-rs's `TimeCounter` already imported
elsewhere — see `rust/uperf-core/src/hint.rs` for the existing pattern.

## Outcome

T04 resolved. No new files. Next ticket unblocked: T05 (SwitchRefreshRate
call-frequency control — `force=true` callers, dedupe).
