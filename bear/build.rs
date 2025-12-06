// SPDX-License-Identifier: GPL-3.0-or-later

fn main() {
    // Check if the allow-integration-tests feature is enabled
    if std::env::var("CARGO_FEATURE_ALLOW_INTEGRATION_TESTS").is_ok() {
        // For integration tests, use paths in the target directory
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let target_dir = std::path::Path::new(&out_dir)
            .ancestors()
            .nth(3) // Go up from out_dir to target/debug or target/release
            .unwrap();

        let wrapper_path = target_dir.join("wrapper");
        let preload_path = target_dir.join("libexec.so");

        println!(
            "cargo:rustc-env=WRAPPER_EXECUTABLE_PATH={}",
            wrapper_path.display()
        );
        println!(
            "cargo:rustc-env=PRELOAD_LIBRARY_PATH={}",
            preload_path.display()
        );
    } else {
        // Use default system paths for production
        println!("cargo:rustc-env=WRAPPER_EXECUTABLE_PATH=/usr/libexec/bear/wrapper");
        println!("cargo:rustc-env=PRELOAD_LIBRARY_PATH=/usr/libexec/bear/$LIB/libexec.so");
    }

    // Re-run build script if env changes
    println!("cargo:rerun-if-env-changed=PATH");
}
