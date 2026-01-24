// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;
use std::io::Write;

fn main() {
    // Tell cargo to invalidate the built crate whenever source changes
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/c/shim.c");

    if cfg!(target_family = "unix") {
        let out_dir = std::env::var("OUT_DIR").unwrap();

        // Perform system capability checks and get detected symbols
        let detected_symbols = platform_checks::perform_system_checks();

        // Compile the C shim for all intercepted functions
        // This handles variadic arguments properly (execl family) and provides
        // a clean separation between C exports and Rust implementation.
        //
        // We use cargo_metadata(false) to prevent cc from emitting its own
        // cargo:rustc-link-lib directive, which would link without --whole-archive
        let mut shim = cc::Build::new();
        for symbol in &detected_symbols {
            let flag = format!("has_symbol_{}", symbol);
            shim.define(flag.as_str(), None);
        }
        shim
            .file("src/c/shim.c")
            .warnings(true)
            .extra_warnings(true)
            .pic(true) // Position independent code for shared library
            .cargo_metadata(false) // Don't let cc emit link directives
            .out_dir(&out_dir)
            .compile("shim");

        // Manually specify linking with --whole-archive to ensure all C symbols
        // are included in the shared library, even if they're not referenced from Rust.
        // This is critical because the C shim exports need to be available for
        // LD_PRELOAD interception to work.
        if cfg!(target_os = "macos") {
            // Generate macOS export file
            let exports_path = format!("{}/exports.txt", out_dir);
            generate_macos_exports(&exports_path, &detected_symbols);

            // macOS uses -force_load instead of --whole-archive
            println!("cargo:rustc-cdylib-link-arg=-Wl,-force_load,{}/libshim.a", out_dir);
            // macOS uses -exported_symbols_list for symbol visibility
            println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbols_list,{}", exports_path);
            // Set rpath to look for dependencies in the same directory as the library
            // macOS uses @loader_path instead of $ORIGIN
            println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
        } else {
            // Generate Linux/ELF version script
            let exports_path = format!("{}/exports.map", out_dir);
            generate_linux_exports(&exports_path, &detected_symbols);

            // Linux and other ELF platforms use --whole-archive
            println!("cargo:rustc-cdylib-link-arg=-Wl,--whole-archive");
            println!("cargo:rustc-cdylib-link-arg={}/libshim.a", out_dir);
            println!("cargo:rustc-cdylib-link-arg=-Wl,--no-whole-archive");

            // Use a dynamically generated version script to control symbol visibility
            // This ensures all intercepted functions are exported as GLOBAL symbols
            // The version script's "local: *" hides all other symbols
            println!("cargo:rustc-cdylib-link-arg=-Wl,--version-script={}", exports_path);
            // Set rpath to look for dependencies in the same directory as the library
            println!("cargo:rustc-cdylib-link-arg=-fuse-ld=lld");
            println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
        }

        // Force building cdylib even in debug mode
        println!("cargo:rustc-cfg=build_cdylib");
    } else {
        // We don't build on other platforms
        println!("cargo:warning=libexec is not supported on this platform");
    }
}

/// Generate the Linux ELF version script based on detected symbols
fn generate_linux_exports(path: &str, detected_symbols: &HashSet<String>) {
    let mut file = std::fs::File::create(path).expect("Failed to create exports.map");

    writeln!(file, "/* Generated version script for libexec library */").unwrap();
    writeln!(file, "{{").unwrap();
    writeln!(file, "    global:").unwrap();

    // Export symbols that were detected on this platform
    for symbol in detected_symbols {
        writeln!(file, "        {};", symbol).unwrap();
    }

    writeln!(file).unwrap();
    writeln!(file, "        /* Library version info */").unwrap();
    writeln!(file, "        LIBEAR_VERSION;").unwrap();
    writeln!(file).unwrap();
    writeln!(file, "    local:").unwrap();
    writeln!(file, "        *;").unwrap();
    writeln!(file, "}};").unwrap();
}

/// Generate the macOS exported symbols list based on detected symbols
fn generate_macos_exports(path: &str, detected_symbols: &HashSet<String>) {
    let mut file = std::fs::File::create(path).expect("Failed to create exports.txt");

    // macOS exported_symbols_list format: one symbol per line, prefixed with underscore
    for symbol in detected_symbols {
        writeln!(file, "_{}", symbol).unwrap();
    }

    // Library version info
    writeln!(file, "_LIBEAR_VERSION").unwrap();
}
