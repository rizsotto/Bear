// SPDX-License-Identifier: GPL-3.0-or-later

//! Build-time utilities for system capability detection.
//!
//! The `build.rs` of this crate runs once per workspace build and probes
//! the host for symbols. Results are baked into the library
//! as the `DETECTED_SYMBOLS` constant. Consumer
//! `build.rs` scripts replay the probe results as `cargo:rustc-cfg=`
//! directives for their own crate by calling [`emit_cfg`], and register
//! the full `check-cfg` allowlist by calling [`emit_check_cfg`].

include!(concat!(env!("OUT_DIR"), "/detected.rs"));

/// Emit `cargo:rustc-cfg=has_symbol_*` directives
/// for every detected symbol. Call this from a
/// consumer's `build.rs` so the directives apply to that crate.
pub fn emit_cfg() {
    for symbol in DETECTED_SYMBOLS {
        println!("cargo:rustc-cfg=has_symbol_{symbol}");
    }
}

/// Emit `cargo:rustc-check-cfg=` directives for every probed flag, so
/// rustc accepts them in `cfg(...)` attributes regardless of detection
/// outcome. Call this from a consumer's `build.rs`.
pub fn emit_check_cfg() {
    for symbol in KNOWN_SYMBOLS {
        println!("cargo:rustc-check-cfg=cfg(has_symbol_{symbol})");
    }
}
