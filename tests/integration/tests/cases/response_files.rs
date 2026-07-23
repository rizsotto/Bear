// SPDX-License-Identifier: GPL-3.0-or-later

//! Response-file (`@file`) inlining integration tests.
//!
//! These exercise the opt-in `format.arguments.from_response_files` behaviour
//! end to end: a build that passes flags via an `@file` reference, analysed by
//! Bear with inlining enabled or disabled.
//!
//! The whole module is gated (at its declaration in `cases/mod.rs`) on a
//! preload library plus a C compiler and shell, since every test needs all
//! three; that keeps the shared helpers below from tripping the dead-code lint
//! where the tests are compiled out.

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::*;
use anyhow::Result;

const SRC: &str = "int main() { return 0; }";

/// Minimal config that toggles response-file inlining.
fn config(from_response_files: bool) -> String {
    format!(
        r#"
schema: "4.2"
intercept:
  mode: preload
  path: "{}"
format:
  paths:
    directory: as-is
    file: as-is
  arguments:
    from_response_files: {}
"#,
        PRELOAD_LIBRARY_PATH, from_response_files
    )
}

fn run(env: &TestEnvironment, config_yaml: &str, script: &std::path::Path) -> Result<BearOutput> {
    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--config",
        env.create_config(config_yaml)?.to_str().unwrap(),
        "--",
        SHELL_PATH,
        script.to_str().unwrap(),
    ])
}

// Requirements: output-response-file-inlining
#[test]
fn inlines_response_file_when_enabled() -> Result<()> {
    let env = TestEnvironment::new("rsp_enabled")?;
    env.create_source_files(&[("src.c", SRC), ("flags.resp", "-I/opt/include -DEXTRA=2")])?;
    let build = format!("{} -DBASE=1 @flags.resp -c src.c -o src.o", filename_of(COMPILER_C_PATH));
    let script = env.create_shell_script("build.sh", &build)?;

    run(&env, &config(true), &script)?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    let expected = vec![
        COMPILER_C_PATH.to_string(),
        "-DBASE=1".to_string(),
        "-I/opt/include".to_string(),
        "-DEXTRA=2".to_string(),
        "-c".to_string(),
        "src.c".to_string(),
        "-o".to_string(),
        "src.o".to_string(),
    ];
    db.assert_contains(&CompilationEntryMatcher::new().file("src.c").arguments(expected))?;
    Ok(())
}

// Requirements: output-response-file-inlining
#[test]
fn keeps_response_file_literal_when_disabled() -> Result<()> {
    let env = TestEnvironment::new("rsp_disabled")?;
    env.create_source_files(&[("src.c", SRC), ("flags.resp", "-I/opt/include -DEXTRA=2")])?;
    let build = format!("{} -DBASE=1 @flags.resp -c src.c -o src.o", filename_of(COMPILER_C_PATH));
    let script = env.create_shell_script("build.sh", &build)?;

    run(&env, &config(false), &script)?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    let expected = vec![
        COMPILER_C_PATH.to_string(),
        "-DBASE=1".to_string(),
        "@flags.resp".to_string(),
        "-c".to_string(),
        "src.c".to_string(),
        "-o".to_string(),
        "src.o".to_string(),
    ];
    db.assert_contains(&CompilationEntryMatcher::new().file("src.c").arguments(expected))?;
    Ok(())
}

// Requirements: output-response-file-inlining
#[test]
fn inlining_splits_quoted_tokens() -> Result<()> {
    let env = TestEnvironment::new("rsp_quoted")?;
    env.create_source_files(&[("src.c", SRC), ("flags.resp", "'-DGREETING=hello' -DCOUNT=3")])?;
    let build = format!("{} @flags.resp -c src.c -o src.o", filename_of(COMPILER_C_PATH));
    let script = env.create_shell_script("build.sh", &build)?;

    run(&env, &config(true), &script)?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    let expected = vec![
        COMPILER_C_PATH.to_string(),
        "-DGREETING=hello".to_string(),
        "-DCOUNT=3".to_string(),
        "-c".to_string(),
        "src.c".to_string(),
        "-o".to_string(),
        "src.o".to_string(),
    ];
    db.assert_contains(&CompilationEntryMatcher::new().file("src.c").arguments(expected))?;
    Ok(())
}

// Requirements: output-response-file-inlining
#[test]
fn inlining_expands_nested_references() -> Result<()> {
    let env = TestEnvironment::new("rsp_nested")?;
    env.create_source_files(&[
        ("src.c", SRC),
        ("outer.resp", "-DOUTER @inner.resp"),
        ("inner.resp", "-DINNER"),
    ])?;
    let build = format!("{} @outer.resp -c src.c -o src.o", filename_of(COMPILER_C_PATH));
    let script = env.create_shell_script("build.sh", &build)?;

    run(&env, &config(true), &script)?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    let expected = vec![
        COMPILER_C_PATH.to_string(),
        "-DOUTER".to_string(),
        "-DINNER".to_string(),
        "-c".to_string(),
        "src.c".to_string(),
        "-o".to_string(),
        "src.o".to_string(),
    ];
    db.assert_contains(&CompilationEntryMatcher::new().file("src.c").arguments(expected))?;
    Ok(())
}

// Requirements: output-response-file-inlining
//
// Inlined tokens participate in entry shaping: a response file that names an
// additional source makes the database gain an entry for that source, exactly
// as if it had been written on the command line.
#[test]
fn inlining_adds_entry_for_source_named_in_response_file() -> Result<()> {
    let env = TestEnvironment::new("rsp_extra_source")?;
    env.create_source_files(&[("src.c", SRC), ("extra.c", SRC), ("srcs.resp", "extra.c")])?;
    let build = format!("{} -c src.c @srcs.resp", filename_of(COMPILER_C_PATH));
    let script = env.create_shell_script("build.sh", &build)?;

    run(&env, &config(true), &script)?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(2)?;
    db.assert_contains(&CompilationEntryMatcher::new().file("src.c").arguments(vec![
        COMPILER_C_PATH.to_string(),
        "-c".to_string(),
        "src.c".to_string(),
    ]))?;
    db.assert_contains(&CompilationEntryMatcher::new().file("extra.c").arguments(vec![
        COMPILER_C_PATH.to_string(),
        "-c".to_string(),
        "extra.c".to_string(),
    ]))?;
    Ok(())
}

// Requirements: output-response-file-inlining
#[test]
fn missing_response_file_at_analysis_kept_literal_with_warning() -> Result<()> {
    // Two-phase flow: intercept while the response file is present (the build
    // succeeds), then remove it before the separate semantic analysis -- the
    // stale-build-artefact scenario the requirement describes.
    let env = TestEnvironment::new("rsp_missing")?;
    env.create_source_files(&[("src.c", SRC), ("flags.resp", "-DPRESENT=1")])?;
    let build = format!("{} @flags.resp -c src.c -o src.o", filename_of(COMPILER_C_PATH));
    let script = env.create_shell_script("build.sh", &build)?;
    let config_path = env.create_config(&config(true))?;

    env.run_bear_success(&["intercept", "-o", "events.json", "--", SHELL_PATH, script.to_str().unwrap()])?;
    std::fs::remove_file(env.test_dir().join("flags.resp"))?;

    let output = env.run_bear_success(&[
        "--config",
        config_path.to_str().unwrap(),
        "semantic",
        "-i",
        "events.json",
        "-o",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    let expected = vec![
        COMPILER_C_PATH.to_string(),
        "@flags.resp".to_string(),
        "-c".to_string(),
        "src.c".to_string(),
        "-o".to_string(),
        "src.o".to_string(),
    ];
    db.assert_contains(&CompilationEntryMatcher::new().file("src.c").arguments(expected))?;

    let stderr = output.stderr();
    anyhow::ensure!(
        stderr.contains("flags.resp"),
        "expected a warning naming the missing response file, got stderr: {stderr}"
    );
    Ok(())
}
