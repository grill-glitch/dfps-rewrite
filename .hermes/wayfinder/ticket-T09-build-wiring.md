# T13 — Build wiring: build.sh for dfps-rs

## Question

How does dfps-rs integrate into build.sh?

1. Rust workspace: new crate uperf-dfps at rust/uperf-dfps/, builds as
   libuperf_dfps.a, linked into the existing binary — OR — dfps-rs as a
   sub-binary of uperf-cli style? Or as a separate binary?
2. C++: deleting cpp/dfps means changing CMakeLists.txt and removing the
   static lib link from uperf.
3. build.sh: build_webui already verifies the WebUI bundle — same gate for
   the new daemon?
4. NOTICE deletion: drop spdlog + scnlib + dfps entries.

## Required output

A short list of build.sh patches + CMakeLists patches + the workspace
file structure.

## Resolution (2026-10-06)

### 1. Cargo workspace: dfps code lives in `uperf-core`, not a new crate

Reason: T07 says single binary. dfps-rs is just one more `pub mod` in
`uperf-core`. Adding `uperf-dfps` as a separate crate means a 2nd
staticlib link and a 2nd `extern "C"` surface in the C++ supervisor —
friction for zero gain. Cost of inlining: dfps_task is ~400 LoC plus
a `dfps-config` parser (~150 LoC) for the upstream text format. Total
~600 LoC added to `uperf-core/src/dfps_task.rs` + `uperf-core/src/dfps_config.rs`.

### 2. workspace layout (after T09)

```
rust/
│   ├── Cargo.toml                  (workspace: uperf-config, uperf-core, uperf-cli — unchanged)
│   ├── uperf-config/
│   ├── uperf-core/
│   │   ├── Cargo.toml            (add `parser` feature? — see below)
│   │   └── src/
│   │       ├── lib.rs            (existing — wires dfps_task into uperf_rs_start)
│   │       ├── dfps_task.rs      (NEW — business logic per T04-T05)
│   │       └── dfps_config.rs    (NEW — text parser; pure, no I/O)
│   └── uperf-cli/                (unchanged)
```

The parser stays pure (no I/O, no clock) so it can be unit-tested without
device fakes (M2 acceptance: 36 configs × ≥5 rules).

### 3. `uperf-core/Cargo.toml` diff

No new dependencies. The only deps dfps-rs needs are `std::collections::HashMap`,
`std::fs`, `std::cell::Cell`, `std::process::Command` — all already pulled
by `uperf-core` (libc for FFI, parking_lot for mutexes).

### 4. CMakeLists diff

* `cpp/CMakeLists.txt`: drop the `add_subdirectory(dfps/thirdparty thirdparty)`
  line. The vendored spdlog+scnlib tree goes too (`cpp/dfps/thirdparty/`).
* `cpp/uperf/Cargo.toml`: drop `dfps` sources from `target_sources(uperf PRIVATE ...)`.
* `cpp/uperf/app_main.cpp`: drop the lines that call into dfps supervisor's
  CLI parser (`-o log -n notify`). The C++ binary keeps `-c config_path`
  only.

### 5. build.sh diff

`build_rust()` already does:

```sh
(cd $BASEDIR/rust && cargo build -p uperf-core --release --target $ARM64_TARGET)
```

No change. Adding `uperf-dfx` (separate crate) would have required a
second `cargo build -p` line — but we inlined, so build.sh is unchanged.

The `make_uperf`, `make_dfps`, `make_pack`, `make_check` pipeline does
NOT change shape. The `pack` step's file inclusion changes (see patch
list below).

### 6. `make_pack` file list diff

Current pack: cpp → binary, magisk/scripts → tar. After T09:

* Same binary (`bin/uperf`).
* Same magisk tree.
* **Add**: `magisk/config/dfps.default.txt` (the upstream default config
  that customize.sh seeds when `dfps.txt` is missing).
* **Add**: `magisk/script/dfps.sh` (T08).
* **No change** to `magisk/bin/`.

### 7. `make_check` diff

The current `check` task compiles the binary, then runs `cargo test` for
each crate. After T09:

```sh
cargo test -p uperf-core --target $HOST_TARGET dfps_config::tests
cargo test -p uperf-core --target $HOST_TARGET dfps_task::tests
```

`dfps_config::tests` are the M2 acceptance gate ("36 configs × ≥5 rules
→ output bytes-equal to upstream"). `dfps_task::tests` are the M1 gate
("fake input event → orchestrator sees the event"). Both run on the
**host** (x86_64), not the target — they're pure logic, no syscalls.

### 8. NOTICE deletion

After cpp/dfps/ is deleted, the vendored spdlog + scnlib + dfps entries
in `NOTICE` are also deleted. dfps-rs is in-repo (same Apache-2.0 from
upstream yc9559/dfps is preserved; the attribution lives in
`rust/uperf-core/src/dfps_task.rs` as a file header).

### 9. Patch list

```
modify:  rust/Cargo.toml                          (no change — dfps in uperf-core)
modify:  rust/uperf-core/src/lib.rs               (wire dfps_task into uperf_rs_start)
add:     rust/uperf-core/src/dfps_task.rs         (NEW — T04+T05 code)
add:     rust/uperf-core/src/dfps_config.rs       (NEW — T02 parser)
delete:  cpp/dfps/                                (T07)
modify:  cpp/CMakeLists.txt                       (drop add_subdirectory(dfps/...))
modify:  cpp/uperf/CMakeLists.txt                 (drop dfps sources)
modify:  cpp/uperf/app_main.cpp                   (drop -o/-n argv parsing)
modify:  build.sh                                 (cargo test step for dfps_*)
modify:  NOTICE                                   (drop 3 vendor entries)
```

## Outcome

T09 resolved. dfps-rs is one new module in `uperf-core`; no workspace
restructure, no CMake twist. Code lands at M1 (dfps_task.rs skeleton +
dfps_config.rs skeleton) and M2 (full + tests).
