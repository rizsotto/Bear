// SPDX-License-Identifier: GPL-3.0-or-later

//! Bear Build Configuration
//!
//! This build script configures paths for Bear's runtime components based on the build context.
//!
//! ## Available Controls
//!
//! - `DEFAULT_WRAPPER_PATH`: Full path to wrapper executable
//! - `DEFAULT_PRELOAD_PATH`: Full path to preload library (with $LIB placeholder)
//!
//! To customize installation paths, use sed or similar tools to replace these path constants:
//!
//! ```bash
//! # Example: Change installation to /opt/bear
//! sed -i 's|/usr/local/libexec/bear|/opt/bear/lib|g' bear/build.rs
//!
//! # Example: Use system lib directory
//! sed -i 's|/usr/local/libexec/bear|/usr/lib/bear|g' bear/build.rs
//! ```

// =============================================================================
// CONFIGURABLE INSTALLATION PATHS
// =============================================================================

/// Default wrapper executable path
/// Package creators: modify this entire path to change wrapper location
/// Note for Windows: a single backslash in rust strings starts an escape
///                   sequence ("C:\Users\..." breaks); use forward slashes
///                   ("C:/Users/..."), escaped backslashes ("C:\\Users\\..."),
///                   or a raw string literal (r"C:\Users\...")
const DEFAULT_WRAPPER_PATH: &str = "/usr/local/libexec/bear";

/// Default preload library path
/// Package creators: modify this entire path to change preload library location
/// Note: $LIB will be expanded at runtime to the appropriate architecture subdirectory
/// Note for Windows: preload isn't supported; feel free to ignore this path
const DEFAULT_PRELOAD_PATH: &str = "/usr/local/libexec/bear/$LIB";

// =============================================================================
// PLATFORM-SPECIFIC EXECUTABLE AND LIBRARY NAMES (DO NOT CHANGE THESE)
// =============================================================================

/// Wrapper executable name (platform-dependent)
#[cfg(windows)]
const WRAPPER_NAME: &str = "wrapper.exe";
#[cfg(not(windows))]
const WRAPPER_NAME: &str = "wrapper";

/// Preload library name (platform-dependent)
#[cfg(target_os = "macos")]
const PRELOAD_NAME: &str = "libexec.dylib";
#[cfg(not(target_os = "macos"))]
const PRELOAD_NAME: &str = "libexec.so";

fn main() {
    // Check if the allow-integration-tests feature is enabled
    let feature_enabled = std::env::var("CARGO_FEATURE_ALLOW_INTEGRATION_TESTS").is_ok();

    if !feature_enabled {
        // =========================================================================
        // STANDARD PRODUCTION BUILD CONFIGURATION
        // =========================================================================
        // This section configures paths for normal builds (packages, manual installs)

        let wrapper_path = format!("{}/{}", DEFAULT_WRAPPER_PATH, WRAPPER_NAME);
        let preload_path = format!("{}/{}", DEFAULT_PRELOAD_PATH, PRELOAD_NAME);

        println!("cargo:rustc-env=WRAPPER_EXECUTABLE_PATH={}", wrapper_path);
        println!("cargo:rustc-env=PRELOAD_LIBRARY_PATH={}", preload_path);
    } else {
        // =========================================================================
        // INTEGRATION TEST OVERRIDE CONFIGURATION
        // =========================================================================
        // EVERYTHING IN THIS BLOCK IS FOR INTEGRATION TESTING ONLY
        // When the allow-integration-tests feature is enabled, we override the
        // default production paths to use paths within the cargo target directory

        configure_integration_test_paths();
    }

    // Re-run build script if environment changes
    println!("cargo:rerun-if-env-changed=PATH");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_ALLOW_INTEGRATION_TESTS");
}

/// Configure paths for integration testing
///
/// This function overrides the default production paths to use locations
/// within the cargo target directory, allowing integration tests to find
/// the wrapper and preload library components.
fn configure_integration_test_paths() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR environment variable not set during build");

    // Navigate from OUT_DIR to the target directory
    // OUT_DIR typically looks like: target/debug/build/bear-<hash>/out
    // We need to go up to: target/debug or target/release
    let target_dir = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3) // Go up from out_dir to target/debug or target/release
        .expect("Could not determine target directory from OUT_DIR");

    let wrapper_path = target_dir.join(WRAPPER_NAME);
    let preload_path = target_dir.join(PRELOAD_NAME);

    // Set integration test paths (this is the only place paths are printed when feature is enabled)
    println!("cargo:rustc-env=WRAPPER_EXECUTABLE_PATH={}", wrapper_path.display());
    println!("cargo:rustc-env=PRELOAD_LIBRARY_PATH={}", preload_path.display());

    // Debug output for integration test builds
    println!("cargo:warning=Integration test paths configured:");
    println!("cargo:warning=  Wrapper: {}", wrapper_path.display());
    println!("cargo:warning=  Preload: {}", preload_path.display());
}
