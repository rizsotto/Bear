// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::Write;

fn main() {
    println!("cargo:rustc-env=WRAPPER_EXECUTABLE_PATH=/usr/libexec/bear/wrapper");
    println!("cargo:rustc-env=PRELOAD_LIBRARY_PATH=/usr/libexec/bear/$LIB/libexec.so");

    // Re-run build script if env changes
    println!("cargo:rerun-if-env-changed=PATH");

    // check things for the libexec.so
    check_include_file("dlfcn.h", "dlfcn_h");
    check_symbol_exists("dlopen", "dlfcn.h");
    check_symbol_exists("dlsym", "dlfcn.h");
    check_symbol_exists("dlerror", "dlfcn.h");
    check_symbol_exists("dlclose", "dlfcn.h");
    check_symbol_exists("RTLD_NEXT", "dlfcn.h");

    check_include_file("errno.h", "errno_h");
    check_symbol_exists("EACCES", "errno.h");
    check_symbol_exists("ENOENT", "errno.h");

    // check things for the integration tests
    check_include_file("unistd.h", "unistd_h");
    check_symbol_exists("execve", "unistd.h");
    check_symbol_exists("execv", "unistd.h");
    check_symbol_exists("execvpe", "unistd.h");
    check_symbol_exists("execvp", "unistd.h");
    check_symbol_exists("execvP", "unistd.h");
    check_symbol_exists("exect", "unistd.h");
    check_symbol_exists("execl", "unistd.h");
    check_symbol_exists("execlp", "unistd.h");
    check_symbol_exists("execle", "unistd.h");
    check_symbol_exists("execveat", "unistd.h");
    check_symbol_exists("fexecve", "unistd.h");

    check_include_file("spawn.h", "spawn_h");
    check_symbol_exists("posix_spawn", "spawn.h");
    check_symbol_exists("posix_spawnp", "spawn.h");

    check_include_file("stdio.h", "stdio_h");
    check_symbol_exists("popen", "stdio.h");

    check_include_file("stdlib.h", "stdlib_h");
    check_symbol_exists("system", "stdlib.h");

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

fn check_include_file(header: &str, define: &str) {
    let result = cc::Build::new()
        .cargo_metadata(false)
        .cargo_output(false)
        .cargo_warnings(false)
        .inherit_rustflags(true)
        .file(
            tempfile::Builder::new()
                .prefix("check_include_")
                .suffix(".c")
                .tempfile_in(std::env::var("OUT_DIR").unwrap_or_else(|_| "target".to_string()))
                .expect("Failed to create temp file for include check")
                .keep() // Keep the file for cc to compile
                .expect("Failed to keep temp file")
                .1, // Get the PathBuf
        )
        .include(header)
        .try_compile(define);

    match result {
        Ok(_) => {
            println!("cargo:rustc-cfg=has_header_{}", define);
            println!("cargo:rustc-check-cfg=cfg(has_header_{})", define);
            println!(
                "cargo:warning=Checking for include file: {} ... found",
                header
            );
        }
        Err(_) => {
            println!(
                "cargo:warning=Checking for include file: {} ... missing",
                header
            );
        }
    }
}

fn check_symbol_exists(symbol: &str, header: &str) {
    let check_code = format!(
        r#"
        #include <stddef.h>
        #include <{header}>

        // Use a function pointer to avoid unused function warnings,
        // and ensure the linker must find the symbol.
        int main() {{
            void *ptr = (void*){symbol};
            (void)ptr; // Suppress unused variable warning
            return 0;
        }}
        "#,
        symbol = symbol,
        header = header
    );

    let (mut file, path) = tempfile::Builder::new()
        .prefix(&format!("check_{}", symbol))
        .suffix(".c")
        .tempfile_in(std::env::var("OUT_DIR").unwrap_or_else(|_| "target".to_string()))
        .expect("Failed to create temp file for symbol check")
        .keep()
        .expect("Failed to keep temp file");

    file.write_all(check_code.as_bytes())
        .expect("Failed to write to temp file");
    file.flush().expect("Failed to flush temp file");

    let result = cc::Build::new()
        .cargo_metadata(false)
        .cargo_output(false)
        .cargo_warnings(false)
        .inherit_rustflags(true)
        .file(path)
        .try_compile(&format!("check_{}", symbol));

    match result {
        Ok(_) => {
            println!("cargo:rustc-cfg=has_symbol_{}", symbol);
            println!("cargo:rustc-check-cfg=cfg(has_symbol_{})", symbol);
            println!("cargo:warning=Checking for symbol: {} ... found", symbol);
        }
        Err(_) => {
            println!("cargo:warning=Checking for symbol: {} ... missing", symbol);
        }
    }
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
