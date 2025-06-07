/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * This build script is responsible for setting up environment variables and
 * cfg flags required by the integration tests, mirroring the logic from the
 * original bear/build.rs.
 */

fn main() {
    // Set up paths for wrapper and preload artifacts
    let (wrapper_path, preload_path) = find_intercept_artifacts();
    println!("cargo:rustc-env=WRAPPER_EXECUTABLE_PATH={}", wrapper_path);
    println!("cargo:rustc-env=PRELOAD_LIBRARY_PATH={}", preload_path);

    // Re-run build script if env changes
    println!("cargo:rerun-if-env-changed=PATH");
    println!("cargo:rerun-if-env-changed=CARGO_TARGET_DIR");
    println!("cargo:rerun-if-env-changed=PROFILE");

    // Perform system checks for headers and symbols
    platform_checks::perform_system_checks();

    // Perform additional checks for executables
    check_executable_exists("true");
    check_executable_exists("false");
    check_executable_exists("echo");
    check_executable_exists("sleep");
    check_one_executable_exists("shell", &["sh", "zsh", "bash"]);
    check_one_executable_exists("make", &["make", "gmake", "mingw32-make"]);
    check_one_executable_exists("compiler_c", &["cc", "gcc", "clang"]);
    check_one_executable_exists("compiler_cxx", &["c++", "g++", "clang++"]);
    check_one_executable_exists("compiler_fortran", &["gfortran", "flang"]);
    check_one_executable_exists("compiler_cuda", &["nvcc"]);
    check_executable_exists("libtool");
    check_executable_exists("fakeroot");
    check_executable_exists("valgrind");
    check_executable_exists("ar");
}

fn find_intercept_artifacts() -> (String, String) {
    use std::path::PathBuf;

    // Get the target directory from CARGO_TARGET_DIR or default to target/
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // Find workspace root by looking for Cargo.toml with [workspace]
            let mut current = std::env::current_dir().expect("Failed to get current directory");
            loop {
                let cargo_toml = current.join("Cargo.toml");
                if cargo_toml.exists() {
                    if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                        if content.contains("[workspace]") {
                            return current.join("target");
                        }
                    }
                }
                if !current.pop() {
                    break;
                }
            }
            PathBuf::from("target")
        });

    // Determine build profile (debug or release)
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let profile_dir = target_dir.join(&profile);

    // Construct paths for the intercept artifacts
    let wrapper_path = profile_dir.join("wrapper");

    // For the preload library, we need to handle the platform-specific naming
    let preload_path = if cfg!(target_os = "macos") {
        profile_dir.join("deps").join("libexec.dylib")
    } else if cfg!(target_os = "windows") {
        profile_dir.join("deps").join("exec.dll")
    } else {
        profile_dir.join("deps").join("libexec.so")
    };

    // If artifacts don't exist yet, fall back to system paths for installed version
    let wrapper_final = if wrapper_path.exists() {
        wrapper_path.to_string_lossy().to_string()
    } else {
        "/usr/libexec/bear/wrapper".to_string()
    };

    let preload_final = if preload_path.exists() {
        preload_path.to_string_lossy().to_string()
    } else {
        "/usr/libexec/bear/$LIB/libexec.so".to_string()
    };

    println!("cargo:warning=Using wrapper path: {}", wrapper_final);
    println!("cargo:warning=Using preload path: {}", preload_final);

    (wrapper_final, preload_final)
}

fn check_executable_exists(executable: &str) {
    match which::which(executable) {
        Ok(path) => {
            println!("cargo:rustc-cfg=has_executable_{}", executable);
            println!("cargo:rustc-check-cfg=cfg(has_executable_{})", executable);
            println!(
                "cargo:rustc-env={}_PATH={}",
                executable.to_uppercase(),
                path.display()
            );
            println!(
                "cargo:warning=Checking for executable: {} ... {}",
                executable,
                path.display()
            );
        }
        Err(_) => {
            println!(
                "cargo:warning=Checking for executable: {} ... missing",
                executable
            );
        }
    }
}

fn check_one_executable_exists(define: &str, executables: &[&str]) {
    for executable in executables {
        if let Ok(path) = which::which(executable) {
            println!("cargo:rustc-cfg=has_executable_{}", define);
            println!("cargo:rustc-check-cfg=cfg(has_executable_{})", define);
            println!(
                "cargo:rustc-env={}_PATH={}",
                define.to_uppercase(),
                path.display()
            );
            println!(
                "cargo:warning=Checking for executable: {} ... {}",
                define,
                path.display()
            );
            return;
        }
    }
    println!(
        "cargo:warning=Checking for executable: {} ... missing",
        define
    );
}
