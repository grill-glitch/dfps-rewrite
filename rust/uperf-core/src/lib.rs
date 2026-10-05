//! Stub uperf-core lib in dfps-rewrite, exists ONLY to compile dfps-rs
//! standalone for testing. This file is NOT mounted into uperf-rewrite
//! via subtree — it's the dfps-rewrite side that verifies the dfps-rs
//! source compiles with `cargo check -p uperf-core-stub`.

pub mod dfps_rs;

/// Stub of uperf-rs's crate-root log helper. In uperf-rewrite (where
/// dfps-rs is mounted as a subtree), `crate::log_msg` resolves to the
/// real C-bridge log writer defined in lib.rs.
#[doc(hidden)]
pub fn log_msg(msg: &str) {
    eprintln!("[dfps_rs] {msg}");
}

#[doc(hidden)]
pub fn dfps_rs_only_compile_check() -> &'static str {
    "stub"
}
