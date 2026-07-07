// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::Write;

// Symbols that the C shim (src/c/shim.c) actually exports for LD_PRELOAD
// interception. The cc -D defines and the version script / exports list
// are restricted to this family, even though `platform_checks::DETECTED_SYMBOLS`
// also includes auxiliary probes (dlopen, RTLD_NEXT, EACCES, ...).
const INTERCEPT_FAMILY: &[&str] = &[
    "execve",
    "execv",
    "execvpe",
    "execvp",
    "execvP",
    "exect",
    "execl",
    "execlp",
    "execle",
    "pclose",
    "posix_spawn",
    "posix_spawnp",
    "popen",
    "system",
];

fn main() {
    // Tell cargo to invalidate the built crate whenever source changes
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/c/shim.c");

    // Replay platform-checks results as cfg directives for this crate
    platform_checks::emit_cfg();
    platform_checks::emit_check_cfg();

    if cfg!(target_family = "unix") {
        let out_dir = std::env::var("OUT_DIR").unwrap();

        let mut intercept_symbols: Vec<&str> = platform_checks::DETECTED_SYMBOLS
            .iter()
            .copied()
            .filter(|s| INTERCEPT_FAMILY.contains(s))
            .collect();

        // musl exposes the execvpe implementation as the namespace-reserved
        // __execvpe and makes execvpe a weak alias. On some arches (e.g. s390x)
        // callers bind to __execvpe directly, so we must export and intercept it
        // too. It is handled here rather than via the platform-checks probe /
        // INTERCEPT_FAMILY pattern because no public header declares __execvpe
        // (the host probe cannot reference it) and it is musl-specific. glibc
        // does not need this: its execvpe export is the real symbol.
        let is_musl = std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("musl");
        if is_musl && intercept_symbols.contains(&"execvpe") {
            intercept_symbols.push("__execvpe");
        }

        // Compile the C shim for all intercepted functions
        // This handles variadic arguments properly (execl family) and provides
        // a clean separation between C exports and Rust implementation.
        //
        // We use cargo_metadata(false) to prevent cc from emitting its own
        // cargo:rustc-link-lib directive, which would link without --whole-archive
        let mut shim = cc::Build::new();
        for symbol in &intercept_symbols {
            let flag = format!("has_symbol_{}", symbol);
            shim.define(flag.as_str(), None);
        }
        shim.file("src/c/shim.c")
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
            generate_macos_exports(&exports_path, &intercept_symbols);

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
            generate_linux_exports(&exports_path, &intercept_symbols);

            // Linux and other ELF platforms use --whole-archive
            println!("cargo:rustc-cdylib-link-arg=-Wl,--whole-archive");
            println!("cargo:rustc-cdylib-link-arg={}/libshim.a", out_dir);
            println!("cargo:rustc-cdylib-link-arg=-Wl,--no-whole-archive");

            // Use a dynamically generated version script to control symbol visibility
            // This ensures all intercepted functions are exported as GLOBAL symbols
            // The version script's "local: *" hides all other symbols
            println!("cargo:rustc-cdylib-link-arg=-Wl,--version-script={}", exports_path);
            // Set rpath to look for dependencies in the same directory as the library
            println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
            emit_elf_linker_selection();
        }
    } else {
        // We don't build on other platforms
        println!("cargo:warning=libexec is not supported on this platform");
    }
}

/// Pick a linker that can combine rustc's own cdylib version script with
/// the exports.map above; GNU ld refuses that combination outright. The
/// preference order and its reasoning are recorded in
/// docs/rationale/preload-linker-selection.md.
fn emit_elf_linker_selection() {
    if let Some(gcc_ld_dir) = bundled_rust_lld_dir() {
        println!("cargo:rustc-cdylib-link-arg=-B{}", gcc_ld_dir.display());
        println!("cargo:rustc-cdylib-link-arg=-fuse-ld=lld");
    } else if find_in_path("ld.lld") {
        println!("cargo:rustc-cdylib-link-arg=-fuse-ld=lld");
    } else if find_in_path("mold") {
        println!("cargo:rustc-cdylib-link-arg=-fuse-ld=mold");
    } else {
        // On hosts whose default linker already is lld (e.g. FreeBSD) the
        // link succeeds anyway, so phrase this as a hint, not a verdict.
        println!(
            "cargo:warning=could not confirm an lld or mold linker; relying on \
             the toolchain default. If linking libexec fails with 'anonymous \
             version tag cannot be combined with other version tags', install \
             lld or mold (GNU ld cannot link this library)"
        );
    }
}

/// rustup toolchains ship rust-lld with cc-compatible shims under
/// <sysroot>/lib/rustlib/<host>/bin/gcc-ld; preferring it keeps the build
/// free of system linker dependencies. The shims run on the build host,
/// hence HOST rather than TARGET.
fn bundled_rust_lld_dir() -> Option<std::path::PathBuf> {
    let rustc = std::env::var_os("RUSTC")?;
    let output = std::process::Command::new(rustc).args(["--print", "sysroot"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let sysroot = String::from_utf8(output.stdout).ok()?;
    let host = std::env::var("HOST").ok()?;
    let dir = std::path::Path::new(sysroot.trim()).join("lib/rustlib").join(host).join("bin/gcc-ld");
    dir.join("ld.lld").is_file().then_some(dir)
}

fn find_in_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

/// Generate the Linux ELF version script based on intercept symbols
fn generate_linux_exports(path: &str, intercept_symbols: &[&str]) {
    let mut file = std::fs::File::create(path).expect("Failed to create exports.map");

    writeln!(file, "/* Generated version script for libexec library */").unwrap();
    writeln!(file, "{{").unwrap();
    writeln!(file, "    global:").unwrap();

    // Export symbols that were detected on this platform
    for symbol in intercept_symbols {
        writeln!(file, "        {};", symbol).unwrap();
    }

    writeln!(file).unwrap();
    writeln!(file, "        /* Library version info */").unwrap();
    writeln!(file, "        LIBEXEC_VERSION;").unwrap();
    writeln!(file).unwrap();
    writeln!(file, "    local:").unwrap();
    writeln!(file, "        *;").unwrap();
    writeln!(file, "}};").unwrap();
}

/// Generate the macOS exported symbols list based on intercept symbols
fn generate_macos_exports(path: &str, intercept_symbols: &[&str]) {
    let mut file = std::fs::File::create(path).expect("Failed to create exports.txt");

    // macOS exported_symbols_list format: one symbol per line, prefixed with underscore
    for symbol in intercept_symbols {
        writeln!(file, "_{}", symbol).unwrap();
    }

    // Library version info
    writeln!(file, "_LIBEXEC_VERSION").unwrap();
}
