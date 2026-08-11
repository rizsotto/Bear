// SPDX-License-Identifier: GPL-3.0-or-later

fn main() {
    // The compiler-specific environment variable names (`COMPILER_ENV_KEYS`)
    // are generated from the same `compilers/*.yaml` descriptions the
    // `semantic` crate uses. The agent-side environment filtering in
    // `src/environment.rs` includes the result via
    // `include!(OUT_DIR/env_keys.rs)`.
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let flags_dir = manifest_dir.join("../semantic/compilers");
    let out_dir: std::path::PathBuf = std::env::var("OUT_DIR").unwrap().into();

    if let Err(e) = compilers_codegen::generate_env_keys_only(&flags_dir, &out_dir) {
        eprintln!("error: env-key codegen failed");
        for cause in e.chain() {
            eprintln!("  caused by: {}", cause);
        }
        std::process::exit(1);
    }
}
