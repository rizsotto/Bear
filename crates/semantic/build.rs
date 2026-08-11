// SPDX-License-Identifier: GPL-3.0-or-later

fn main() {
    let flags_dir = std::path::Path::new("compilers");
    let out_dir: std::path::PathBuf = std::env::var("OUT_DIR").unwrap().into();
    if let Err(e) = compilers_codegen::generate(flags_dir, &out_dir) {
        eprintln!("error: flag codegen failed");
        for cause in e.chain() {
            eprintln!("  caused by: {}", cause);
        }
        std::process::exit(1);
    }
}
