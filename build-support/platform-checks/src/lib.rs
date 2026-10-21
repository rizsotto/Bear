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
    "_NSGetEnviron",
];

const HEADER_PROBES: &[&str] = &["crt_externs.h"];

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

/// What the target C toolchain provides.
pub struct Detected {
    pub symbols: Vec<&'static str>,
    pub headers: Vec<&'static str>,
}

/// The `cfg` key, and the C macro name, for a detected header.
pub fn header_cfg_name(header: &str) -> String {
    let mangled: String = header.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    format!("has_header_{mangled}")
}

/// Probe the target C toolchain and return the symbols and headers it
/// provides. Call this once from a consumer's `build.rs`.
///
/// Returns an empty list on non-unix targets without probing. Panics on a
/// unix target when `execve` is not found, because that means the target
/// toolchain is broken and every other probe would be wrong too.
pub fn detect() -> Detected {
    emit_rerun_if_env_changed();

    let family = std::env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default();
    if !family.split(',').any(|f| f == "unix") {
        return Detected { symbols: Vec::new(), headers: Vec::new() };
    }

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let probe_dir = tempfile::tempdir_in(out_dir).expect("Failed to create probe directory");

    let mut symbols: Vec<&'static str> = Vec::new();
    for symbol in SYMBOL_PROBES {
        match check_symbol_exists(symbol, probe_dir.path()) {
            Ok(()) => symbols.push(symbol),
            Err(error) if *symbol == "execve" => {
                let target = std::env::var("TARGET").unwrap_or_default();
                let compiler = cc::Build::new().get_compiler().path().display().to_string();
                panic!(
                    "symbol `execve` not found for target {target} with compiler {compiler}: \
                     the target C toolchain is broken or missing\n{error}"
                );
            }
            Err(_) => {}
        }
    }

    let headers: Vec<&'static str> = HEADER_PROBES
        .iter()
        .copied()
        .filter(|header| check_header_exists(header, probe_dir.path()).is_ok())
        .collect();
    Detected { symbols, headers }
}

/// Emit `cargo:rustc-cfg=has_symbol_*` and `has_header_*` directives
/// for everything detected. Call this from a
/// consumer's `build.rs` so the directives apply to that crate.
pub fn emit_cfg(detected: &Detected) {
    for symbol in &detected.symbols {
        println!("cargo:rustc-cfg=has_symbol_{symbol}");
    }
    for header in &detected.headers {
        println!("cargo:rustc-cfg={}", header_cfg_name(header));
    }
}

/// Emit `cargo:rustc-check-cfg=` directives for every probed flag, so
/// rustc accepts them in `cfg(...)` attributes regardless of detection
/// outcome. Call this from a consumer's `build.rs`.
pub fn emit_check_cfg() {
    for symbol in SYMBOL_PROBES {
        println!("cargo:rustc-check-cfg=cfg(has_symbol_{symbol})");
    }
    for header in HEADER_PROBES {
        println!("cargo:rustc-check-cfg=cfg({})", header_cfg_name(header));
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
fn check_symbol_exists(symbol: &str, probe_dir: &Path) -> Result<(), String> {
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

    let result = run_probe(&format!("check_{symbol}"), &check_code, probe_dir, true);
    let outcome = if result.is_ok() { "found" } else { "missing" };
    println!("cargo:warning=Checking for symbol: {symbol} ... {outcome}");
    result
}

/// Compile a source that includes the header. Compile only: the header may
/// exist while its functions are not linkable, and that is the symbol probe's job.
fn check_header_exists(header: &str, probe_dir: &Path) -> Result<(), String> {
    let check_code = format!(
        r#"
        #include <{header}>
        int main(void) {{ return 0; }}
        "#
    );

    let name = format!("check_{}", header_cfg_name(header));
    let result = run_probe(&name, &check_code, probe_dir, false);
    let outcome = if result.is_ok() { "found" } else { "missing" };
    println!("cargo:warning=Checking for header: {header} ... {outcome}");
    result
}

/// Run the compiler on `code`. On failure, the error holds the compiler's
/// output, so a broken toolchain can be reported.
fn run_probe(name: &str, code: &str, probe_dir: &Path, link: bool) -> Result<(), String> {
    let (mut file, path) = tempfile::Builder::new()
        .prefix(name)
        .suffix(".c")
        .tempfile_in(probe_dir)
        .expect("Failed to create temp file for probe")
        .keep()
        .expect("Failed to keep temp file");

    file.write_all(code.as_bytes()).expect("Failed to write to temp file");
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

    let mut command = compiler.to_command();
    command.arg("-fno-builtin");
    if !link {
        command.arg("-c");
    }
    let output = command
        .arg(&path)
        .arg("-o")
        .arg(probe_dir.join(name))
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("failed to run {}: {error}", command_line(&command)))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{}\n{}{}",
            command_line(&command),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// The program and its arguments, without the environment: `Command`'s
/// `Debug` output lists every variable, secrets included.
fn command_line(command: &std::process::Command) -> String {
    std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}
