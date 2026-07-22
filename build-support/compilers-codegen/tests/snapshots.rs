// SPDX-License-Identifier: GPL-3.0-or-later

//! Snapshot tests for all generated Rust source files.
//!
//! Each test generates one output file from the real YAML definitions and
//! compares it against a stored snapshot. Any change in YAML or codegen
//! logic is caught as a snapshot diff.

use compilers_codegen::env_keys::generate_env_keys;
use compilers_codegen::families::generate_families;
use compilers_codegen::recognition::{generate_compiler_ids, generate_recognition_patterns};
use compilers_codegen::{ResolvedTable, load_compiler_files, load_tables, load_wrapper_tables};

/// One snapshot per discovered compiler flag table, named
/// `snapshot_flags_<stem>` so it lands in the same file the former
/// hand-written per-family `snapshot_flags_*` functions produced. A new
/// compiler YAML auto-gains a snapshot on the next run; accept the diff.
#[test]
fn snapshot_flags() {
    let raw_tables = load_tables().unwrap();
    for compiler_file in load_compiler_files().unwrap() {
        let generated = ResolvedTable::new(&compiler_file, &raw_tables).unwrap().generate().unwrap();
        insta::assert_snapshot!(format!("snapshot_flags_{}", compiler_file.stem), generated);
    }
}

#[test]
fn snapshot_recognition() {
    let raw_tables = load_tables().unwrap();
    let compiler_files = load_compiler_files().unwrap();
    let wrapper_tables = load_wrapper_tables(&flags_dir(), false).unwrap();
    insta::assert_snapshot!(
        generate_recognition_patterns(&raw_tables, &compiler_files, &wrapper_tables).unwrap()
    );
}

/// Path to the real compiler-definitions directory. Mirrors the library's
/// private `flags_dir()` helper (`pub(crate)`, not visible to this
/// integration-test binary); `CARGO_MANIFEST_DIR` here is the same
/// `build-support/compilers-codegen` directory, so the walk-up is identical.
fn flags_dir() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .ancestors()
        .nth(2)
        .expect("CARGO_MANIFEST_DIR is build-support/compilers-codegen, two levels below the workspace root");
    workspace_root.join("crates/bear/compilers")
}

#[test]
fn snapshot_env_keys() {
    let raw_tables = load_tables().unwrap();
    insta::assert_snapshot!(generate_env_keys(&raw_tables));
}

#[test]
fn snapshot_compiler_ids() {
    let compiler_files = load_compiler_files().unwrap();
    let wrapper_tables = load_wrapper_tables(&flags_dir(), false).unwrap();
    insta::assert_snapshot!(generate_compiler_ids(&compiler_files, &wrapper_tables).unwrap());
}

#[test]
fn snapshot_families() {
    let raw_tables = load_tables().unwrap();
    let compiler_files = load_compiler_files().unwrap();
    insta::assert_snapshot!(generate_families(&compiler_files, &raw_tables));
}
