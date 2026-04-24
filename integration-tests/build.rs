/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * This build script is responsible for setting up environment variables and
 * cfg flags required by the integration tests, mirroring the logic from the
 * original bear/build.rs.
 */

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
    // Re-run build script if env changes
    println!("cargo:rerun-if-env-changed=PATH");
    println!("cargo:rerun-if-env-changed=CARGO_TARGET_DIR");
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-env-changed=INTERCEPT_LIBDIR");

    // Forward INTERCEPT_LIBDIR so integration tests use the same value as bear-driver
    let intercept_libdir = std::env::var("INTERCEPT_LIBDIR").unwrap_or_else(|_| "lib".to_string());
    validate_intercept_libdir(&intercept_libdir);
    println!("cargo:rustc-env=INTERCEPT_LIBDIR={}", intercept_libdir);

    // Re-run if bear or intercept-preload artifacts change
    println!("cargo:rerun-if-changed=../bear/src");
    println!("cargo:rerun-if-changed=../intercept-preload/src");

    // Locate install script and repo root for integration tests
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let repo_root = std::path::Path::new(&manifest_dir).parent().unwrap();
    let install_script = repo_root.join("scripts").join("install.sh");
    println!("cargo:rerun-if-changed={}", install_script.display());
    println!("cargo:rustc-env=REPO_ROOT={}", repo_root.display());
    println!("cargo:rustc-env=INSTALL_SCRIPT_PATH={}", install_script.display());

    // Set up paths for driver, wrapper and preload artifacts
    let (driver_path, wrapper_path, preload_path) = find_intercept_artifacts();
    println!("cargo:rustc-env=DRIVER_EXECUTABLE={}", DRIVER_NAME);
    println!("cargo:rustc-env=DRIVER_EXECUTABLE_PATH={}", driver_path);
    println!("cargo:rustc-env=WRAPPER_EXECUTABLE={}", WRAPPER_NAME);
    println!("cargo:rustc-env=WRAPPER_EXECUTABLE_PATH={}", wrapper_path);
    println!("cargo:rustc-env=PRELOAD_LIBRARY={}", PRELOAD_NAME);
    println!("cargo:rustc-env=PRELOAD_LIBRARY_PATH={}", preload_path);

    // Perform system checks for headers and symbols
    platform_checks::perform_system_checks();

    // Perform additional checks for executables
    check_executable_exists("true");
    check_executable_exists("false");
    check_executable_exists("echo");
    check_executable_exists("sleep");
    check_executable_exists("cat");
    check_executable_exists("ls");
    check_executable_exists("mkdir");
    check_executable_exists("rm");
    check_one_executable_exists("shell", &["sh", "zsh", "bash"]);
    check_one_executable_exists("make", &["make", "gmake", "mingw32-make"]);
    check_one_executable_exists("compiler_c", &["gcc", "clang", "cc"]);
    check_one_executable_exists("compiler_cxx", &["g++", "clang++", "c++"]);
    check_ccache_masquerade_dir();
    check_one_executable_exists("compiler_fortran", &["gfortran", "flang"]);
    check_one_executable_exists("compiler_cuda", &["nvcc"]);
    check_executable_exists("libtool");
    check_executable_exists("fakeroot");
    check_executable_exists("valgrind");
    check_executable_exists("ar");
    check_executable_exists("env");

    // Check for preload library availability
    check_preload_library_availability(&preload_path);
}

fn find_intercept_artifacts() -> (String, String, String) {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let target_dir = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3) // Go up from out_dir to target/debug or target/release
        .unwrap();

    let driver_path = target_dir.join(DRIVER_NAME);
    let wrapper_path = target_dir.join(WRAPPER_NAME);
    let preload_path = target_dir.join(PRELOAD_NAME);

    (
        format!("{}", driver_path.display()),
        format!("{}", wrapper_path.display()),
        format!("{}", preload_path.display()),
    )
}

fn check_executable_exists(executable: &str) {
    match which::which(executable) {
        Ok(path) => {
            println!("cargo:rustc-cfg=has_executable_{}", executable);
            println!("cargo:rustc-check-cfg=cfg(has_executable_{})", executable);
            println!("cargo:rustc-env={}_PATH={}", executable.to_uppercase(), path.display());
            println!("cargo:warning=Checking for executable: {} ... {}", executable, path.display());
        }
        Err(_) => {
            println!("cargo:warning=Checking for executable: {} ... missing", executable);
        }
    }
}

fn check_one_executable_exists(define: &str, executables: &[&str]) {
    for executable in executables {
        if let Ok(path) = which::which(executable) {
            println!("cargo:rustc-cfg=has_executable_{}", define);
            println!("cargo:rustc-check-cfg=cfg(has_executable_{})", define);
            println!("cargo:rustc-env={}_PATH={}", define.to_uppercase(), path.display());
            println!("cargo:warning=Checking for executable: {} ... {}", define, path.display());
            return;
        }
    }
    println!("cargo:warning=Checking for executable: {} ... missing", define);
}

/// Locate a ccache masquerade directory on this host, independent of PATH.
/// When one is found, expose it via the `CCACHE_MASQUERADE_DIR` env var and
/// set `cfg(host_has_ccache_masquerade)`. The dedicated recursion test (see
/// `interception-wrapper-recursion`) prepends that dir to its own PATH at
/// runtime so the masquerade setup is exercised regardless of whether the
/// host's default PATH already includes it. CI installs ccache so the dir
/// exists on the Ubuntu matrix entry.
fn check_ccache_masquerade_dir() {
    println!("cargo:rustc-check-cfg=cfg(host_has_ccache_masquerade)");
    let candidates = [
        // Debian/Ubuntu
        "/usr/lib/ccache",
        // Fedora/Arch/Gentoo (lib64 multilib)
        "/usr/lib64/ccache",
        // Some BSDs / older distros
        "/usr/libexec/ccache",
        // Homebrew on Apple Silicon
        "/opt/homebrew/opt/ccache/libexec",
        // Homebrew on Intel macOS / Linuxbrew
        "/usr/local/opt/ccache/libexec",
    ];
    for dir in candidates {
        if let Some(path) = detect_ccache_masquerade_dir(dir) {
            println!("cargo:rustc-cfg=host_has_ccache_masquerade");
            println!("cargo:rustc-env=CCACHE_MASQUERADE_DIR={}", path);
            println!("cargo:warning=ccache masquerade directory found at {}", path);
            return;
        }
    }
    println!("cargo:warning=no ccache masquerade directory detected");
}

/// A directory qualifies as a ccache masquerade dir if it contains a `gcc`,
/// `cc`, `g++`, `c++`, `clang`, or `clang++` entry whose ultimate target's
/// file name is `ccache`.
fn detect_ccache_masquerade_dir(dir: &str) -> Option<String> {
    let path = std::path::Path::new(dir);
    if !path.is_dir() {
        return None;
    }
    for name in ["gcc", "cc", "g++", "c++", "clang", "clang++"] {
        let candidate = path.join(name);
        if !candidate.exists() {
            continue;
        }
        if let Ok(target) = std::fs::canonicalize(&candidate)
            && target.file_name().and_then(|n| n.to_str()) == Some("ccache")
        {
            return Some(dir.to_string());
        }
    }
    None
}

fn check_preload_library_availability(preload_path: &str) {
    // Check if we're on a platform that supports LD_PRELOAD (Unix-like systems)
    let platform_supports_preload = !cfg!(windows);

    // Check if the preload library file exists
    let preload_file_exists = std::path::Path::new(preload_path).exists();

    // Check for platform-specific restrictions
    let runtime_supports_preload = is_preload_supported_at_runtime();

    if platform_supports_preload && preload_file_exists && runtime_supports_preload {
        println!("cargo:rustc-cfg=has_preload_library");
        println!("cargo:rustc-check-cfg=cfg(has_preload_library)");
        println!("cargo:warning=Preload library available at: {}", preload_path);
    } else {
        println!(
            "cargo:warning=Preload library not available (platform_supports: {}, file_exists: {}, runtime_supports: {})",
            platform_supports_preload, preload_file_exists, runtime_supports_preload
        );
    }
}

/// Check if preload is supported at runtime, considering platform-specific restrictions.
fn is_preload_supported_at_runtime() -> bool {
    #[cfg(windows)]
    {
        // Windows doesn't support LD_PRELOAD
        false
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        // Disable integration test on macOS, because could not find out how to compile
        // the libexec.dylib that works on arm64e, and that makes the CI build broken.
        false
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios", windows)))]
    {
        // Other Unix-like systems should support preload
        true
    }
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
