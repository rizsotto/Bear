// SPDX-License-Identifier: GPL-3.0-or-later

//! Build-time utilities for system capability detection.
//!
//! Consumer `build.rs` scripts call [`detect`] to probe the C toolchain of
//! their own target for symbols, replay the result as `cargo:rustc-cfg=`
//! directives by calling [`emit_cfg`], and register the full `check-cfg`
//! allowlist by calling [`emit_check_cfg`]. The probes run inside the
//! consumer's build script, so `TARGET`, `CC_<target>` and `CFLAGS` are
//! the consumer's, not the host's.

use std::io::Write;
use std::path::Path;

const SYMBOL_PROBES: &[(&str, &str)] = &[
    ("execve", "unistd.h"),
    ("execv", "unistd.h"),
    ("execvpe", "unistd.h"),
    ("execvp", "unistd.h"),
    ("execvP", "unistd.h"),
    ("exect", "unistd.h"),
    ("execl", "unistd.h"),
    ("execlp", "unistd.h"),
    ("execle", "unistd.h"),
    ("posix_spawn", "spawn.h"),
    ("pclose", "stdio.h"),
    ("posix_spawnp", "spawn.h"),
    ("popen", "stdio.h"),
    ("system", "stdlib.h"),
];

/// Probe the target C toolchain and return the symbols it provides. Call
/// this once from a consumer's `build.rs`.
///
/// Returns an empty list on non-unix targets without probing. Panics on a
/// unix target when `execve` is not found, because that means the target
/// toolchain is broken and every other probe would be wrong too.
pub fn detect() -> Vec<&'static str> {
    emit_rerun_if_env_changed();

    let family = std::env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default();
    if !family.split(',').any(|f| f == "unix") {
        return Vec::new();
    }

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let probe_dir = tempfile::tempdir_in(out_dir).expect("Failed to create probe directory");

    let detected: Vec<&'static str> = SYMBOL_PROBES
        .iter()
        .filter(|(symbol, header)| check_symbol_exists(symbol, header, probe_dir.path()))
        .map(|(symbol, _)| *symbol)
        .collect();

    if !detected.contains(&"execve") {
        let target = std::env::var("TARGET").unwrap_or_default();
        let compiler = cc::Build::new().get_compiler().path().display().to_string();
        panic!(
            "symbol `execve` not found for target {target} with compiler {compiler}: \
             the target C toolchain is broken or missing"
        );
    }
    detected
}

/// Emit `cargo:rustc-cfg=has_symbol_*` directives
/// for every detected symbol. Call this from a
/// consumer's `build.rs` so the directives apply to that crate.
pub fn emit_cfg(symbols: &[&str]) {
    for symbol in symbols {
        println!("cargo:rustc-cfg=has_symbol_{symbol}");
    }
}

/// Emit `cargo:rustc-check-cfg=` directives for every probed flag, so
/// rustc accepts them in `cfg(...)` attributes regardless of detection
/// outcome. Call this from a consumer's `build.rs`.
pub fn emit_check_cfg() {
    for (symbol, _) in SYMBOL_PROBES {
        println!("cargo:rustc-check-cfg=cfg(has_symbol_{symbol})");
    }
}

/// `cc` is used with `cargo_metadata(false)`, so it does not tell Cargo
/// which environment variables select the compiler. Do it here.
fn emit_rerun_if_env_changed() {
    for var in ["CC", "CFLAGS", "TARGET_CC", "TARGET_CFLAGS"] {
        println!("cargo:rerun-if-env-changed={var}");
    }
    if let Ok(target) = std::env::var("TARGET") {
        let underscored = target.replace('-', "_");
        for var in ["CC", "CFLAGS"] {
            println!("cargo:rerun-if-env-changed={var}_{target}");
            println!("cargo:rerun-if-env-changed={var}_{underscored}");
        }
    }
}

fn check_symbol_exists(symbol: &str, header: &str, probe_dir: &Path) -> bool {
    let check_code = format!(
        r#"
        #include <stddef.h>
        #include <{header}>

        // Use a function pointer to avoid unused function warnings,
        // and ensure the linker must find the symbol.
        int main() {{
            void *ptr = (void*){symbol};
            (void)ptr; // Suppress unused variable warning
            return 0;
        }}
        "#
    );

    let (mut file, path) = tempfile::Builder::new()
        .prefix(&format!("check_{symbol}"))
        .suffix(".c")
        .tempfile_in(probe_dir)
        .expect("Failed to create temp file for symbol check")
        .keep()
        .expect("Failed to keep temp file");

    file.write_all(check_code.as_bytes()).expect("Failed to write to temp file");
    file.flush().expect("Failed to flush temp file");

    let result = cc::Build::new()
        .cargo_metadata(false)
        .cargo_output(false)
        .cargo_warnings(false)
        .inherit_rustflags(true)
        .define("_GNU_SOURCE", "1")
        .file(path)
        .out_dir(probe_dir)
        .try_compile(&format!("check_{symbol}"));

    match result {
        Ok(_) => {
            println!("cargo:warning=Checking for symbol: {symbol} ... found");
            true
        }
        Err(_) => {
            println!("cargo:warning=Checking for symbol: {symbol} ... missing");
            false
        }
    }
}
