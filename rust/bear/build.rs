// SPDX-License-Identifier: GPL-3.0-or-later

fn main() {
    println!("cargo:rustc-env=WRAPPER_EXECUTABLE_PATH=/usr/libexec/bear/wrapper");
    println!("cargo:rustc-env=PRELOAD_LIBRARY_PATH=/usr/libexec/bear/$LIB/libexec.so");
}
