//! dfpsd — standalone smoke binary for M3-standalone (real-device
//! verification before M5 module-merge).
//!
//! Usage:
//!   dfpsd <dfps.txt> -o <dfps_log.txt> -n <dfps_cur.txt>
//!
//! This binary deliberately lives **only in dfps-rewrite**, not in
//! uperf-rewrite: it does NOT pull in the uperf-rs bridge, the C++
//! platform layer, or any of the cross-process IPC. It exercises only
//! what dfps-rs (the subtree-mounted Rust) actually does:
//!
//! * load dfps.txt from disk (RuleTable::parse)
//! * construct a DfpsTask
//! * write a few Hz values to dfps_cur.txt via notifier::write_cur_hz
//! * sleep then exit
//!
//! It is **not** a real dfps daemon — it does not subscribe to input
//! events (the orchestrator does that in uperf-rs). It exists so we
//! can confirm on-device that:
//!
//!   (a) the parser loads a real-world dfps.txt without error;
//!   (b) `notifier::write_cur_hz` produces a file visible from adb;
//!   (c) the binary links and runs on aarch64-linux-android23.
//!
//! If (a)+(b) hold, the parser + notifier surface is right; M3 then
//! only needs to wire topic events into the DfpsTask, not redo the
//! I/O plumbing. That is the whole point of M3-standalone.

use std::env;
use std::process::ExitCode;
use std::time::Duration;

use uperf_core::dfps_rs::{config::RuleTable, DfpsTask, DFPS_NOTIFY_PATH};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let config_path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: dfpsd <dfps.txt> [-o <log>] [-n <cur>]");
            return ExitCode::from(2);
        }
    };

    // -o / -n are accepted (and printed back) so the smoke script's
    // invocation matches what a real daemon would see. The actual
    // notify path always lives at DFPS_NOTIFY_PATH — the upstream
    // /sdcard/Android/yc/uperf/dfps_cur.txt — to keep this identical
    // to the embedded path.
    let mut log_path: Option<String> = None;
    let mut _notify_arg: Option<String> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "-o" => log_path = args.next(),
            "-n" => _notify_arg = args.next(),
            _ => eprintln!("dfpsd: ignoring unknown arg: {a}"),
        }
    }

    eprintln!("dfpsd: starting (notify={DFPS_NOTIFY_PATH})");
    eprintln!("dfpsd: config={config_path}");
    eprintln!("dfpsd: log={}", log_path.as_deref().unwrap_or("(none)"));

    // (a) parse the config — fail loud if dfps.txt is missing or invalid.
    let text = match std::fs::read_to_string(&config_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("dfpsd: cannot read '{config_path}': {e}");
            return ExitCode::from(1);
        }
    };
    let table = match RuleTable::parse(&text) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("dfpsd: cannot parse dfps.txt: {e}");
            return ExitCode::from(1);
        }
    };
    let rules_n = table.rule_count();
    let universal = table.universal_rule();
    let offscreen = table.offscreen_rule();
    eprintln!(
        "dfpsd: dfps.txt parsed OK ({} rules, universal={}/{}, offscreen={}/{})",
        rules_n, universal.idle, universal.active, offscreen.idle, offscreen.active
    );

    // Construct the task (proves DfpsTask::new compiles and runs on the
    // device; we don't drive it with real events here).
    let mut task = DfpsTask::new(table.clone());

    // (b) emit three Hz values to dfps_cur.txt so we can verify the
    // notifier surface works.
    let hz_active = table.universal_rule().active;
    let hz_idle = table.universal_rule().idle;
    let seq: [i32; 3] = [hz_active, hz_idle, hz_active];
    for hz in seq {
        // tick() picks hz_idle when active=false; to force a specific
        // value we hit the notifier directly.
        if let Err(e) = uperf_core::dfps_rs::write_cur_hz(hz) {
            eprintln!("dfpsd: write_cur_hz({hz}) failed: {e}");
            return ExitCode::from(1);
        }
        eprintln!("dfpsd: wrote {hz} to {DFPS_NOTIFY_PATH}");
        std::thread::sleep(Duration::from_millis(200));
    }

    // One more: read the task's cur_app() accessor to prove the getter
    // works end-to-end through the subtree mount. (We can't write
    // task.cur_app because the field is private; the smoke only needs
    // to prove the type is constructible and readable.)
    let _ = task.cur_app();
    eprintln!("dfpsd: task is alive");

    // Stay alive long enough for adb to read dfps_cur.txt + log.
    eprintln!("dfpsd: sleeping 5s, then exiting");
    std::thread::sleep(Duration::from_secs(5));
    eprintln!("dfpsd: done");
    ExitCode::SUCCESS
}