// SPDX-License-Identifier: GPL-3.0-or-later

//
// Build-time system capability detection. Runs once per workspace build
// (since this crate is a [build-dependencies] of the consumers, Cargo
// schedules it before any consumer's build.rs).
//
// Output: writes OUT_DIR/detected.rs containing four constants
// (DETECTED_HEADERS, DETECTED_SYMBOLS, KNOWN_HEADERS, KNOWN_SYMBOLS)
// that lib.rs pulls in via include!(). Consumer build scripts call
// platform_checks::emit_cfg() / emit_check_cfg() to apply the
// cargo:rustc-cfg / cargo:rustc-check-cfg directives to their own crate.
//

use std::collections::BTreeSet;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

const HEADER_PROBES: &[(&str, &str)] = &[
    ("dlfcn.h", "dlfcn_h"),
    ("errno.h", "errno_h"),
    ("unistd.h", "unistd_h"),
    ("spawn.h", "spawn_h"),
    ("stdio.h", "stdio_h"),
    ("stdlib.h", "stdlib_h"),
];

const SYMBOL_PROBES: &[(&str, &str)] = &[
    ("dlopen", "dlfcn.h"),
    ("dlsym", "dlfcn.h"),
    ("dlerror", "dlfcn.h"),
    ("dlclose", "dlfcn.h"),
    ("RTLD_NEXT", "dlfcn.h"),
    ("EACCES", "errno.h"),
    ("ENOENT", "errno.h"),
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
    ("posix_spawnp", "spawn.h"),
    ("popen", "stdio.h"),
    ("system", "stdlib.h"),
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir: PathBuf = std::env::var("OUT_DIR").expect("OUT_DIR not set").into();

    let mut detected_headers: BTreeSet<&str> = BTreeSet::new();
    for (header, define) in HEADER_PROBES {
        if check_include_file(header, define, &out_dir) {
            detected_headers.insert(define);
        }
    }

    let mut detected_symbols: BTreeSet<&str> = BTreeSet::new();
    for (symbol, header) in SYMBOL_PROBES {
        if check_symbol_exists(symbol, header, &out_dir) {
            detected_symbols.insert(symbol);
        }
    }

    let path = out_dir.join("detected.rs");
    let mut file = File::create(&path).expect("Failed to create detected.rs");
    write_const(&mut file, "DETECTED_HEADERS", detected_headers.iter().copied());
    write_const(&mut file, "DETECTED_SYMBOLS", detected_symbols.iter().copied());
    write_const(&mut file, "KNOWN_HEADERS", HEADER_PROBES.iter().map(|(_, d)| *d));
    write_const(&mut file, "KNOWN_SYMBOLS", SYMBOL_PROBES.iter().map(|(s, _)| *s));
}

fn write_const<'a, I: Iterator<Item = &'a str>>(file: &mut File, name: &str, items: I) {
    write!(file, "pub const {name}: &[&str] = &[").expect("write failed");
    for item in items {
        write!(file, "\"{item}\", ").expect("write failed");
    }
    writeln!(file, "];").expect("write failed");
}

fn check_include_file(header: &str, define: &str, out_dir: &std::path::Path) -> bool {
    let result = cc::Build::new()
        .cargo_metadata(false)
        .cargo_output(false)
        .cargo_warnings(false)
        .inherit_rustflags(true)
        .file(
            tempfile::Builder::new()
                .prefix("check_include_")
                .suffix(".c")
                .tempfile_in(out_dir)
                .expect("Failed to create temp file for include check")
                .keep()
                .expect("Failed to keep temp file")
                .1,
        )
        .include(header)
        .try_compile(define);

    match result {
        Ok(_) => {
            println!("cargo:warning=Checking for include file: {header} ... found");
            true
        }
        Err(_) => {
            println!("cargo:warning=Checking for include file: {header} ... missing");
            false
        }
    }
}

fn check_symbol_exists(symbol: &str, header: &str, out_dir: &std::path::Path) -> bool {
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
        .tempfile_in(out_dir)
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
