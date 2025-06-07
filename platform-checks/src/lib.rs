// SPDX-License-Identifier: GPL-3.0-or-later

//! Build-time utilities for system capability detection
//!
//! This crate provides functions for checking system capabilities during build time.
//! It's designed to be used in build scripts to generate appropriate cfg flags.

use std::io::Write;

/// Check if a header file is available on the system
///
/// # Arguments
/// * `header` - The header file name (e.g., "dlfcn.h")
/// * `define` - The cfg flag suffix (e.g., "dlfcn_h")
///
/// # Output
/// Generates `cargo:rustc-cfg=has_header_{define}` if the header is found
pub fn check_include_file(header: &str, define: &str) {
    let result = cc::Build::new()
        .cargo_metadata(false)
        .cargo_output(false)
        .cargo_warnings(false)
        .inherit_rustflags(true)
        .file(
            tempfile::Builder::new()
                .prefix("check_include_")
                .suffix(".c")
                .tempfile_in(std::env::var("OUT_DIR").unwrap_or_else(|_| "target".to_string()))
                .expect("Failed to create temp file for include check")
                .keep() // Keep the file for cc to compile
                .expect("Failed to keep temp file")
                .1, // Get the PathBuf
        )
        .include(header)
        .try_compile(define);

    match result {
        Ok(_) => {
            println!("cargo:rustc-cfg=has_header_{}", define);
            println!("cargo:rustc-check-cfg=cfg(has_header_{})", define);
            println!(
                "cargo:warning=Checking for include file: {} ... found",
                header
            );
        }
        Err(_) => {
            println!(
                "cargo:warning=Checking for include file: {} ... missing",
                header
            );
        }
    }
}

/// Check if a symbol exists in a header file
///
/// # Arguments
/// * `symbol` - The symbol name (e.g., "dlopen")
/// * `header` - The header file that should contain the symbol (e.g., "dlfcn.h")
///
/// # Output
/// Generates `cargo:rustc-cfg=has_symbol_{symbol}` if the symbol is found
pub fn check_symbol_exists(symbol: &str, header: &str) {
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
        "#,
        symbol = symbol,
        header = header
    );

    let (mut file, path) = tempfile::Builder::new()
        .prefix(&format!("check_{}", symbol))
        .suffix(".c")
        .tempfile_in(std::env::var("OUT_DIR").unwrap_or_else(|_| "target".to_string()))
        .expect("Failed to create temp file for symbol check")
        .keep()
        .expect("Failed to keep temp file");

    file.write_all(check_code.as_bytes())
        .expect("Failed to write to temp file");
    file.flush().expect("Failed to flush temp file");

    let result = cc::Build::new()
        .cargo_metadata(false)
        .cargo_output(false)
        .cargo_warnings(false)
        .inherit_rustflags(true)
        .file(path)
        .try_compile(&format!("check_{}", symbol));

    match result {
        Ok(_) => {
            println!("cargo:rustc-cfg=has_symbol_{}", symbol);
            println!("cargo:rustc-check-cfg=cfg(has_symbol_{})", symbol);
            println!("cargo:warning=Checking for symbol: {} ... found", symbol);
        }
        Err(_) => {
            println!("cargo:warning=Checking for symbol: {} ... missing", symbol);
        }
    }
}

/// Perform all system checks for libexec.so and integration tests
///
/// This function runs all the header and symbol checks that are needed
/// by the Bear project components.
pub fn perform_system_checks() {
    check_include_file("dlfcn.h", "dlfcn_h");
    check_symbol_exists("dlopen", "dlfcn.h");
    check_symbol_exists("dlsym", "dlfcn.h");
    check_symbol_exists("dlerror", "dlfcn.h");
    check_symbol_exists("dlclose", "dlfcn.h");
    check_symbol_exists("RTLD_NEXT", "dlfcn.h");

    check_include_file("errno.h", "errno_h");
    check_symbol_exists("EACCES", "errno.h");
    check_symbol_exists("ENOENT", "errno.h");

    check_include_file("unistd.h", "unistd_h");
    check_symbol_exists("execve", "unistd.h");
    check_symbol_exists("execv", "unistd.h");
    check_symbol_exists("execvpe", "unistd.h");
    check_symbol_exists("execvp", "unistd.h");
    check_symbol_exists("execvP", "unistd.h");
    check_symbol_exists("exect", "unistd.h");
    check_symbol_exists("execl", "unistd.h");
    check_symbol_exists("execlp", "unistd.h");
    check_symbol_exists("execle", "unistd.h");
    check_symbol_exists("execveat", "unistd.h");
    check_symbol_exists("fexecve", "unistd.h");

    check_include_file("spawn.h", "spawn_h");
    check_symbol_exists("posix_spawn", "spawn.h");
    check_symbol_exists("posix_spawnp", "spawn.h");

    check_include_file("stdio.h", "stdio_h");
    check_symbol_exists("popen", "stdio.h");

    check_include_file("stdlib.h", "stdlib_h");
    check_symbol_exists("system", "stdlib.h");
}

/// Get all the cfg flags that should be added to check-cfg
///
/// Returns a vector of cfg flag names that should be included in the
/// `cargo:rustc-check-cfg` directives.
pub fn get_all_cfg_flags() -> Vec<&'static str> {
    vec![
        "has_header_dlfcn_h",
        "has_symbol_dlopen",
        "has_symbol_dlsym",
        "has_symbol_dlerror",
        "has_symbol_dlclose",
        "has_symbol_RTLD_NEXT",
        "has_header_errno_h",
        "has_symbol_EACCES",
        "has_symbol_ENOENT",
        "has_header_unistd_h",
        "has_symbol_execve",
        "has_symbol_execv",
        "has_symbol_execvpe",
        "has_symbol_execvp",
        "has_symbol_execvP",
        "has_symbol_exect",
        "has_symbol_execl",
        "has_symbol_execlp",
        "has_symbol_execle",
        "has_symbol_execveat",
        "has_symbol_fexecve",
        "has_header_spawn_h",
        "has_symbol_posix_spawn",
        "has_symbol_posix_spawnp",
        "has_header_stdio_h",
        "has_symbol_popen",
        "has_header_stdlib_h",
        "has_symbol_system",
    ]
}
