// SPDX-License-Identifier: GPL-3.0-or-later

fn main() {
    // Only build on Linux
    if cfg!(target_os = "linux") {
        // Tell cargo to invalidate the built crate whenever source changes
        println!("cargo:rerun-if-changed=src/lib.rs");

        // Force building cdylib even in debug mode
        println!("cargo:rustc-cfg=build_cdylib");

        // Let the linker know about symbols we want to export
        println!("cargo:rustc-cdylib-link-arg=-Wl,--export-dynamic");

        // Set rpath to look for dependencies in the same directory as the library
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");

        // Perform system capability checks
        platform_checks::perform_system_checks();
    } else {
        // We don't build on non-Linux platforms
        println!("cargo:warning=libexec is only supported on Linux platforms");
    }
}
