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
use std::process::Stdio;

const SYMBOL_PROBES: &[&str] = &[
    "execve",
    "execv",
    "execvpe",
    "execvp",
    "execvP",
    "exect",
    "execl",
    "execlp",
    "execle",
    "posix_spawn",
    "pclose",
    "posix_spawnp",
    "popen",
    "system",
];

/// Feature macros that autoconf's `AC_USE_SYSTEM_EXTENSIONS` defines
/// unconditionally on every system, as (name, value). Consumers pass them to
/// the C compiler so their C sources need no `#define` of their own.
pub const SYSTEM_EXTENSIONS: &[(&str, &str)] = &[
    ("_ALL_SOURCE", "1"),
    ("_COSMO_SOURCE", "1"),
    ("_DARWIN_C_SOURCE", "1"),
    ("_GNU_SOURCE", "1"),
    ("_HPUX_ALT_XOPEN_SOCKET_API", "1"),
    ("_NETBSD_SOURCE", "1"),
    ("_OPENBSD_SOURCE", "1"),
    ("_POSIX_PTHREAD_SEMANTICS", "1"),
    ("__STDC_WANT_IEC_60559_ATTRIBS_EXT__", "1"),
    ("__STDC_WANT_IEC_60559_BFP_EXT__", "1"),
    ("__STDC_WANT_IEC_60559_DFP_EXT__", "1"),
    ("__STDC_WANT_IEC_60559_EXT__", "1"),
    ("__STDC_WANT_IEC_60559_FUNCS_EXT__", "1"),
    ("__STDC_WANT_IEC_60559_TYPES_EXT__", "1"),
    ("__STDC_WANT_LIB_EXT2__", "1"),
    ("__STDC_WANT_MATH_SPEC_FUNCS__", "1"),
    ("_TANDEM_SOURCE", "1"),
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
        .copied()
        .filter(|symbol| check_symbol_exists(symbol, probe_dir.path()))
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
    for symbol in SYMBOL_PROBES {
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

/// Autoconf's `AC_LANG_FUNC_LINK_TRY`: declare the function ourselves and
/// link, so the probe passes only if the target libc exports the symbol.
fn check_symbol_exists(symbol: &str, probe_dir: &Path) -> bool {
    let check_code = format!(
        r#"
        #define {symbol} innocuous_{symbol}
        #include <limits.h>
        #undef {symbol}
        char {symbol}(void);
        #if defined __stub_{symbol} || defined __stub___{symbol}
        choke me
        #endif
        int main(void) {{ return {symbol}(); }}
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

    let mut build = cc::Build::new();
    for (name, value) in SYSTEM_EXTENSIONS {
        build.define(name, *value);
    }
    let compiler = build
        .cargo_metadata(false)
        .cargo_output(false)
        .cargo_warnings(false)
        .inherit_rustflags(true)
        .get_compiler();

    let found = compiler
        .to_command()
        .arg("-fno-builtin")
        .arg(&path)
        .arg("-o")
        .arg(probe_dir.join(format!("check_{symbol}")))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());

    let outcome = if found { "found" } else { "missing" };
    println!("cargo:warning=Checking for symbol: {symbol} ... {outcome}");
    found
}
