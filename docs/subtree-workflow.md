# dfps-rs ↔ uperf-rewrite subtree workflow

dfps-rs lives **here**, in `grill-glitch/dfps-rewrite`. The host repo
`grill-glitch/uperf-rewrite` mounts it as a git **subtree** at
`rust/uperf-core/src/dfps_rs/`. There is no submodule: uperf-rewrite's
history contains the real files.

## Why subtree (not submodule)

* `git clone uperf-rewrite` works with no `--recurse-submodules`, and CI
  needs no extra checkout step.
* uperf-rewrite's build sees `rust/uperf-core/src/dfps_rs/` as ordinary
  source, so `cargo build -p uperf-core` just works.
* Reverse sync is available (`git subtree push`) when a fix must originate
  from the uperf side (it should not — see the rule below).

## The one rule

**dfps-rs source is edited here, never in uperf-rewrite.** The subtree
mount is one-directional in practice; edits made directly in
`uperf-rewrite/rust/uperf-core/src/dfps_rs/` are lost on the next pull.

## Layout

```
dfps-rewrite/                              uperf-rewrite/
├── rust/uperf-core/src/dfps_rs/  ──────▶  rust/uperf-core/src/dfps_rs/
│   ├── mod.rs                              (same 4 files, subtree-mounted
│   ├── config.rs                            from the split branch below)
│   ├── task.rs
│   └── notifier.rs
├── magisk/script/dfps.sh          ─── copy ─▶  magisk/script/dfps.sh
├── magisk/config/dfps.default.txt ─── copy ─▶  magisk/config/dfps.default.txt
└── webui/pages/dfps.js            ─── copy ─▶  webui/pages/dfps.js
```

The Rust tree is subtree-mounted. The shell/JS artifacts are **not** —
they sit outside the subtree prefix, so they are copied across manually
when they change (a `cp` in the M5 cleanup commit). If they churn often,
promote them to their own subtree prefix.

## Sync: dfps-rewrite → uperf-rewrite

Run in uperf-rewrite, on a clean working tree:

```bash
git fetch dfps-rs main                       # grill-glitch/dfps-rewrite
# Split the dfps-rs subtree out of main into a synthetic branch:
git subtree split --prefix=rust/uperf-core/src/dfps_rs \
    --branch=dfps-rs-split dfps-rs/main      # (run once; or in dfps-rewrite and push)
git fetch dfps-rs dfps-rs-split
git subtree pull --prefix=rust/uperf-core/src/dfps_rs dfps-rs dfps-rs-split
```

The split step is what lets uperf-rewrite mount **only** the dfps-rs
subdirectory — a plain `git subtree add` of `dfps-rs/main` would mount the
whole dfps-rewrite tree (AGENT.md, docs, LICENSE, …) under the prefix.

Publish the split branch from dfps-rewrite so every consumer sees the same
synthetic history:

```bash
# in dfps-rewrite
git subtree split --prefix=rust/uperf-core/src/dfps_rs --branch=dfps-rs-split
git push origin dfps-rs-split
```

## Sync: uperf-rewrite → dfps-rewrite (reverse)

Only for a fix that genuinely must originate from the uperf side
(e.g. an API change driven by a uperf-rewrite refactor). Prefer editing
here and pulling forward.

```bash
# in uperf-rewrite
git subtree push --prefix=rust/uperf-core/src/dfps_rs dfps-rs dfps-rs-src
```

## Verification

The round trip is exercised, not assumed:

1. Edit a file under `dfps-rewrite/rust/uperf-core/src/dfps_rs/`, commit.
2. In dfps-rewrite: `git subtree split --prefix=rust/uperf-core/src/dfps_rs --branch=dfps-rs-split && git push origin dfps-rs-split`.
3. In uperf-rewrite: `git subtree pull --prefix=rust/uperf-core/src/dfps_rs dfps-rs dfps-rs-split`.
4. The change appears in `uperf-rewrite/rust/uperf-core/src/dfps_rs/` and
   `cargo test -p uperf-core --lib dfps_rs` still passes (15/15).

## Configuration

* Remote in uperf-rewrite: `dfps-rs` → `https://github.com/grill-glitch/dfps-rewrite.git`
* Subtree prefix: `rust/uperf-core/src/dfps_rs`
* Split branch: `dfps-rs-split` (in dfps-rewrite)
* Source branch: `main` (dfps-rewrite) — the dfps-rs source lives on `main`
  after the move lands; the `dfps-rs-src` branch was the staging branch for
  the initial move.

## A dirty working tree blocks subtree operations

`git subtree pull/add` refuses with `fatal: working tree has modifications.
Cannot add.` when *any* tracked file is dirty — even one unrelated to the
subtree. In this repo `magisk/bin/uperf` (a checked-in build artifact) is
frequently dirty; stash it first:

```bash
git stash push -m "temp: subtree op" -- magisk/bin/uperf
git subtree pull ...
git stash pop
```
