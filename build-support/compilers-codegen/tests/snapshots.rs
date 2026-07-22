// SPDX-License-Identifier: GPL-3.0-or-later

//! Snapshot tests for all generated Rust source files.
//!
//! Each test generates one output file from the real YAML definitions and
//! compares it against a stored snapshot. Any change in YAML or codegen
//! logic is caught as a snapshot diff.

use compilers_codegen::env_keys::generate_env_keys;
use compilers_codegen::recognition::{generate_compiler_ids, generate_recognition_patterns};
use compilers_codegen::tables::TABLES;
use compilers_codegen::{ResolvedTable, load_tables, load_wrapper_tables};

fn generate_flag_file(yaml_stem: &str) -> String {
    let raw_tables = load_tables().unwrap();
    let config = TABLES.iter().find(|c| c.yaml_file == format!("{}.yaml", yaml_stem)).unwrap();
    ResolvedTable::new(yaml_stem, config, &raw_tables).unwrap().generate().unwrap()
}

#[test]
fn snapshot_flags_gcc() {
    insta::assert_snapshot!(generate_flag_file("gcc"));
}

#[test]
fn snapshot_flags_clang() {
    insta::assert_snapshot!(generate_flag_file("clang"));
}

#[test]
fn snapshot_flags_clang_cl() {
    insta::assert_snapshot!(generate_flag_file("clang_cl"));
}

#[test]
fn snapshot_flags_ibm_xl() {
    insta::assert_snapshot!(generate_flag_file("ibm_xl"));
}

#[test]
fn snapshot_flags_flang() {
    insta::assert_snapshot!(generate_flag_file("flang"));
}

#[test]
fn snapshot_flags_cuda() {
    insta::assert_snapshot!(generate_flag_file("cuda"));
}

#[test]
fn snapshot_flags_intel_fortran() {
    insta::assert_snapshot!(generate_flag_file("intel_fortran"));
}

#[test]
fn snapshot_flags_cray_fortran() {
    insta::assert_snapshot!(generate_flag_file("cray_fortran"));
}

#[test]
fn snapshot_flags_msvc() {
    insta::assert_snapshot!(generate_flag_file("msvc"));
}

#[test]
fn snapshot_flags_intel_cc() {
    insta::assert_snapshot!(generate_flag_file("intel_cc"));
}

#[test]
fn snapshot_flags_nvidia_hpc() {
    insta::assert_snapshot!(generate_flag_file("nvidia_hpc"));
}

#[test]
fn snapshot_flags_armclang() {
    insta::assert_snapshot!(generate_flag_file("armclang"));
}

#[test]
fn snapshot_flags_vala() {
    insta::assert_snapshot!(generate_flag_file("vala"));
}

#[test]
fn snapshot_flags_mpi() {
    insta::assert_snapshot!(generate_flag_file("mpi"));
}

#[test]
fn snapshot_flags_cray_cc() {
    insta::assert_snapshot!(generate_flag_file("cray_cc"));
}

#[test]
fn snapshot_flags_qnx() {
    insta::assert_snapshot!(generate_flag_file("qnx"));
}

#[test]
fn snapshot_flags_nasm() {
    insta::assert_snapshot!(generate_flag_file("nasm"));
}

#[test]
fn snapshot_flags_fasm() {
    insta::assert_snapshot!(generate_flag_file("fasm"));
}

#[test]
fn snapshot_flags_swift() {
    insta::assert_snapshot!(generate_flag_file("swift"));
}

#[test]
fn snapshot_recognition() {
    let raw_tables = load_tables().unwrap();
    // load_tables() keys by compiler.id; every real YAML file's id equals
    // its stem (see the migration table in the schema-split commit), so
    // rebuilding that mapping here reproduces the real yaml_file -> id
    // index generate() builds from the same load loop.
    let file_to_id: std::collections::HashMap<&'static str, String> = TABLES
        .iter()
        .map(|c| (c.yaml_file, c.yaml_file.strip_suffix(".yaml").unwrap().to_string()))
        .collect();
    let wrapper_tables = load_wrapper_tables(&flags_dir(), false).unwrap();
    insta::assert_snapshot!(
        generate_recognition_patterns(&raw_tables, &file_to_id, &wrapper_tables).unwrap()
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
    // Mirror generate()'s yaml_file -> id index (see snapshot_recognition).
    let file_to_id: std::collections::HashMap<&'static str, String> = TABLES
        .iter()
        .map(|c| (c.yaml_file, c.yaml_file.strip_suffix(".yaml").unwrap().to_string()))
        .collect();
    let wrapper_tables = load_wrapper_tables(&flags_dir(), false).unwrap();
    insta::assert_snapshot!(generate_compiler_ids(&file_to_id, &wrapper_tables).unwrap());
}
