// SPDX-License-Identifier: GPL-3.0-or-later

//! Generates shell completion scripts for the `bear` command.
//!
//! Usage:
//!   generate-completions <output-directory>
//!
//! Writes completion files for bash, zsh, fish, and elvish into the
//! given directory.

use bear::args::cli;
use clap_complete::Shell;
use clap_complete::generate_to;
use std::path::PathBuf;

fn main() {
    let outdir = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        eprintln!("usage: generate-completions <output-directory>");
        std::process::exit(1);
    });

    std::fs::create_dir_all(&outdir).expect("failed to create output directory");

    let mut cmd = cli();
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::Elvish] {
        generate_to(shell, &mut cmd, "bear", &outdir)
            .unwrap_or_else(|e| panic!("failed to generate {shell} completions: {e}"));
    }
}
