// SPDX-License-Identifier: GPL-3.0-or-later

//! Snapshot tests for all generated Rust source files.
//!
//! Each test generates one output file from the real YAML definitions and
//! compares it against a stored snapshot. Any change in YAML or codegen
//! logic is caught as a snapshot diff.

use bear_codegen::env_keys::generate_env_keys;
use bear_codegen::recognition::generate_recognition_patterns;
use bear_codegen::tables::TABLES;
use bear_codegen::{ResolvedTable, load_tables};

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
fn snapshot_recognition() {
    let raw_tables = load_tables().unwrap();
    insta::assert_snapshot!(generate_recognition_patterns(&raw_tables));
}

#[test]
fn snapshot_env_keys() {
    let raw_tables = load_tables().unwrap();
    insta::assert_snapshot!(generate_env_keys(&raw_tables));
}
