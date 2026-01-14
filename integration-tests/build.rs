/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * This build script is responsible for setting up environment variables and
 * cfg flags required by the integration tests, mirroring the logic from the
 * original bear/build.rs.
 */

/// Bear executable name (platform-dependent)
#[cfg(windows)]
const BEAR_NAME: &str = "bear.exe";
#[cfg(not(windows))]
const BEAR_NAME: &str = "bear";

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
    // Set up paths for bear, wrapper and preload artifacts
    let (bear_path, wrapper_path, preload_path) = find_intercept_artifacts();
    println!("cargo:rustc-env=BEAR_EXECUTABLE_PATH={}", bear_path);
    println!("cargo:rustc-env=WRAPPER_EXECUTABLE_PATH={}", wrapper_path);
    println!("cargo:rustc-env=PRELOAD_LIBRARY_PATH={}", preload_path);

    // Re-run build script if env changes
    println!("cargo:rerun-if-env-changed=PATH");
    println!("cargo:rerun-if-env-changed=CARGO_TARGET_DIR");
    println!("cargo:rerun-if-env-changed=PROFILE");

    // Re-run if bear or intercept-preload artifacts change
    println!("cargo:rerun-if-changed=../bear/src");
    println!("cargo:rerun-if-changed=../intercept-preload/src");

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

    let bear_path = target_dir.join(BEAR_NAME);
    let wrapper_path = target_dir.join(WRAPPER_NAME);
    let preload_path = target_dir.join(PRELOAD_NAME);

    (
        format!("{}", bear_path.display()),
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
            // For integration tests, prefer real compiler paths over ccache wrappers
            let final_path = if path.to_string_lossy().contains("ccache") {
                // Try to find the real compiler in /usr/bin
                let real_path = std::path::Path::new("/usr/bin").join(executable);
                if real_path.exists() {
                    println!(
                        "cargo:warning=Preferring real compiler {} over ccache wrapper {}",
                        real_path.display(),
                        path.display()
                    );
                    real_path
                } else {
                    path
                }
            } else {
                path
            };

            println!("cargo:rustc-cfg=has_executable_{}", define);
            println!("cargo:rustc-check-cfg=cfg(has_executable_{})", define);
            println!("cargo:rustc-env={}_PATH={}", define.to_uppercase(), final_path.display());
            println!("cargo:warning=Checking for executable: {} ... {}", define, final_path.display());
            return;
        }
    }
    println!("cargo:warning=Checking for executable: {} ... missing", define);
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
    #[cfg(all(target_os = "macos", not(windows)))]
    {
        // Disable integration test on macOS, because could not find out how to compile
        // the libexec.dylib that works on arm64e, and that makes the CI build broken.
        false
    }
    #[cfg(all(not(target_os = "macos"), not(windows)))]
    {
        // Other Unix-like systems should support preload
        true
    }
}
