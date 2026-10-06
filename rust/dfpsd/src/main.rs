//! dfpsd — standalone smoke binary for the dfps-rs event pipeline.
//!
//! Usage:
//!   dfpsd <dfps.txt> [-o <log>] [-n <cur>]
//!
//! This binary lives **only in dfps-rewrite** and pulls in only the
//! `dfps_rs` subtree — no C++ bridge, no uperf-rs IPC. It drives the real
//! `DfpsScheduler` (the same type the embedded daemon uses) with a scripted
//! event sequence and the real `RealSink`, so a run on the device proves:
//!
//!   (a) the config parses;
//!   (b) the event -> state -> switch pipeline produces the expected Hz
//!       sequence (printed as `STEP n: <event> -> cur_hz=...`);
//!   (c) `RealSink` actually reaches SettingsProvider — observable as a
//!       change in `dumpsys display` `mActiveModeId` while the smoke runs;
//!   (d) the delayed transitions fire on time: the release -> idle step
//!       waits out `touchSlackMs` and the timer thread applies it.
//!
//! Steps are paced 3 s apart so an adb-side `dumpsys display` loop can
//! correlate each one with a display state change.

use std::env;
use std::process::ExitCode;
use std::time::Duration;

use uperf_core::dfps_rs::{config::RuleTable, DfpsScheduler, DFPS_NOTIFY_PATH};

/// Pause after each step so a watcher can observe it.
const STEP_PAUSE: Duration = Duration::from_secs(3);

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let config_path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: dfpsd <dfps.txt> [-o <log>] [-n <cur>]");
            return ExitCode::from(2);
        }
    };
    let mut log_path: Option<String> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "-o" => log_path = args.next(),
            "-n" => {
                let _ = args.next();
            }
            _ => eprintln!("dfpsd: ignoring unknown arg: {a}"),
        }
    }

    // stdout is line-buffered when piped through adb; println! keeps STEP
    // lines interleaved with the log in the right order.
    println!("dfpsd: starting (notify={DFPS_NOTIFY_PATH})");
    println!("dfpsd: config={config_path}");
    println!("dfpsd: log={}", log_path.as_deref().unwrap_or("(none)"));

    // (a) parse
    let text = match std::fs::read_to_string(&config_path) {
        Ok(t) => t,
        Err(e) => {
            println!("dfpsd: cannot read '{config_path}': {e}");
            return ExitCode::from(1);
        }
    };
    let table = match RuleTable::parse(&text) {
        Ok(t) => t,
        Err(e) => {
            println!("dfpsd: cannot parse dfps.txt: {e}");
            return ExitCode::from(1);
        }
    };
    let u = table.universal_rule();
    let minus = table.offscreen_rule();
    println!(
        "dfpsd: parsed OK ({} rules, universal={}/{}, offscreen={}/{})",
        table.rule_count(),
        u.idle,
        u.active,
        minus.idle,
        minus.active
    );

    // The scheduler owns the production sink: every effective change writes
    // dfps_cur.txt and issues the four `settings put` calls.
    let sched = DfpsScheduler::new(table);
    let timer = sched.spawn();
    println!("dfpsd: scheduler spawned (timer thread up)");

    let step = std::cell::Cell::new(0u32);
    macro_rules! report {
        ($ev:expr) => {{
            step.set(step.get() + 1);
            println!("STEP {}: {} -> cur_hz={:?}", step.get(), $ev, sched.cur_hz());
        }};
    }

    // --- scripted sequence, mirroring what the real topics deliver ---

    std::thread::sleep(STEP_PAUSE);
    sched.on_top_app("com.android.settings");
    report!("topapp com.android.settings");

    std::thread::sleep(STEP_PAUSE);
    sched.on_touch(true);
    report!("touch down");

    std::thread::sleep(STEP_PAUSE);
    sched.on_touch(false);
    report!("touch up (idle scheduled)");

    // touchSlackMs from the config (4000 in the shipped default). Wait it
    // out plus a margin so the timer thread applies the idle transition.
    std::thread::sleep(Duration::from_millis(4500));
    report!("after touchSlackMs (idle expected)");

    std::thread::sleep(STEP_PAUSE);
    sched.on_offscreen(true);
    report!("offscreen on");

    std::thread::sleep(STEP_PAUSE);
    sched.on_offscreen(false);
    report!("offscreen off (wake scheduled)");

    std::thread::sleep(Duration::from_millis(4500));
    report!("after gestureSlackMs (restored expected)");

    std::thread::sleep(STEP_PAUSE);
    sched.on_top_app("com.example.unknown");
    report!("topapp unknown (universal)");

    println!("dfpsd: sequence complete; cur_app={}", sched.cur_app());
    sched.stop();
    let _ = timer.join();
    println!("dfpsd: done");
    ExitCode::SUCCESS
}