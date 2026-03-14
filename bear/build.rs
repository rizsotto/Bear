// SPDX-License-Identifier: GPL-3.0-or-later

/// Driver executable name (platform-dependent)
#[cfg(windows)]
const DRIVER_NAME: &str = "bear-driver.exe";
#[cfg(not(windows))]
const DRIVER_NAME: &str = "bear-driver";

/// Wrapper executable name (platform-dependent)
#[cfg(windows)]
const WRAPPER_NAME: &str = "bear-wrapper.exe";
#[cfg(not(windows))]
const WRAPPER_NAME: &str = "bear-wrapper";

/// Preload library name (platform-dependent)
#[cfg(target_os = "macos")]
const PRELOAD_NAME: &str = "libexec.dylib";
#[cfg(not(target_os = "macos"))]
const PRELOAD_NAME: &str = "libexec.so";

fn main() {
    let intercept_libdir = std::env::var("INTERCEPT_LIBDIR").unwrap_or_else(|_| "lib".to_string());
    validate_intercept_libdir(&intercept_libdir);

    println!("cargo:rustc-env=DRIVER_NAME={}", DRIVER_NAME);
    println!("cargo:rustc-env=WRAPPER_NAME={}", WRAPPER_NAME);
    println!("cargo:rustc-env=PRELOAD_NAME={}", PRELOAD_NAME);
    println!("cargo:rustc-env=INTERCEPT_LIBDIR={}", intercept_libdir);
    println!("cargo:rerun-if-env-changed=INTERCEPT_LIBDIR");
}

fn validate_intercept_libdir(value: &str) {
    if value.trim().is_empty() {
        panic!("INTERCEPT_LIBDIR must not be empty or whitespace-only");
    }
    let path = std::path::Path::new(value);
    if path.is_absolute() {
        panic!("INTERCEPT_LIBDIR must be a relative path, got: {}", value);
    }
}
