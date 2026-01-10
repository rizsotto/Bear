// SPDX-License-Identifier: GPL-3.0-or-later

fn main() {
    // Tell cargo to invalidate the built crate whenever source changes
    println!("cargo:rerun-if-changed=src/lib.rs");

    if cfg!(target_family = "unix") {
        // Perform system capability checks
        platform_checks::perform_system_checks();

        // Force building cdylib even in debug mode
        println!("cargo:rustc-cfg=build_cdylib");

        if cfg!(target_os = "macos") {
            // On macOS, symbols in dylibs are exported by default, so we don't need --export-dynamic
            // Use -export_dynamic for the Apple linker if explicit export is needed
            println!("cargo:rustc-cdylib-link-arg=-Wl,-export_dynamic");

            // Set rpath to look for dependencies in the same directory as the library
            // macOS uses @loader_path instead of $ORIGIN
            println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
        } else {
            // Let the linker know about symbols we want to export
            println!("cargo:rustc-cdylib-link-arg=-Wl,--export-dynamic");

            // Set rpath to look for dependencies in the same directory as the library
            println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
        }
    } else {
        // We don't build on other platforms
        println!("cargo:warning=libexec is not supported on this platform");
    }
}
