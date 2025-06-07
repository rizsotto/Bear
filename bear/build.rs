// SPDX-License-Identifier: GPL-3.0-or-later

fn main() {
    println!("cargo:rustc-env=WRAPPER_EXECUTABLE_PATH=/usr/libexec/bear/wrapper");
    println!("cargo:rustc-env=PRELOAD_LIBRARY_PATH=/usr/libexec/bear/$LIB/libexec.so");

    // Re-run build script if env changes
    println!("cargo:rerun-if-env-changed=PATH");

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
