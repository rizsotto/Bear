// SPDX-License-Identifier: GPL-3.0-or-later

//! Integration tests for the `bear parse-sh` subcommand: the wiring of the
//! pure lex/interpret stages to real I/O (stdin/file input, stdout/file
//! output, the `--directory` override, and the exit-code policy).
//!
//! See `docs/requirements/interception-events-from-shell-text.md` for the
//! contract; the crate-level unit tests in
//! `crates/bear/src/parse_sh/interpreter.rs` cover the parsing rules
//! themselves, so these tests focus on the mode's I/O behavior.

use crate::fixtures::infrastructure::compilation_entry;
use crate::fixtures::*;
use anyhow::{Context, Result};
use serde_json::{Value, json};

// Requirements: interception-events-from-shell-text
#[test]
fn parse_sh_stdin_default_emits_one_event_to_stdout() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_stdin_default")?;

    let result = env.run_bear_with_stdin(&["parse-sh"], b"gcc -c foo.c\n")?;
    result.assert_success()?;

    let stdout = result.stdout();
    let lines: Vec<&str> = stdout.lines().filter(|line| !line.is_empty()).collect();
    assert_eq!(lines.len(), 1, "expected exactly one event line, got: {stdout:?}");

    let event: Value = serde_json::from_str(lines[0])
        .with_context(|| format!("failed to parse event line as JSON: {}", lines[0]))?;
    assert_eq!(event["executable"], "gcc");
    assert_eq!(event["arguments"], json!(["gcc", "-c", "foo.c"]));
    assert!(event["working_dir"].is_string(), "working_dir must be a string: {event}");

    Ok(())
}

// Requirements: interception-events-from-shell-text
#[test]
fn parse_sh_directory_flag_sets_event_working_dir() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_directory_flag")?;

    let result = env.run_bear_with_stdin(&["parse-sh", "--directory", "/custom/build"], b"gcc -c foo.c\n")?;
    result.assert_success()?;

    let stdout = result.stdout();
    let line = stdout.lines().next().context("expected one event line on stdout")?;
    let event: Value = serde_json::from_str(line)?;
    assert_eq!(event["working_dir"], "/custom/build");

    Ok(())
}

// Requirements: interception-events-from-shell-text
#[test]
fn parse_sh_all_skipped_input_exits_non_zero_with_no_events() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_all_skipped")?;

    // A subshell group is outside the supported shell subset: the whole
    // line must be skipped, not guessed at.
    let result = env.run_bear_with_stdin(&["parse-sh"], b"(ranlib libz.a || true) >/dev/null 2>&1\n")?;
    result.assert_failure()?;
    assert!(result.stdout().trim().is_empty(), "stdout must carry no event lines: {}", result.stdout());
    assert!(result.stderr().contains("skipped"), "stderr must report the skip: {}", result.stderr());

    Ok(())
}

// Requirements: interception-events-from-shell-text
#[test]
fn parse_sh_empty_input_exits_zero_with_warning() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_empty_input")?;

    let result = env.run_bear_with_stdin(&["parse-sh"], b"")?;
    result.assert_success()?;
    assert!(result.stdout().trim().is_empty(), "stdout must carry no event lines: {}", result.stdout());
    assert!(
        result.stderr().contains("no commands found"),
        "stderr must warn that no commands were found: {}",
        result.stderr()
    );

    Ok(())
}

// Requirements: interception-events-from-shell-text
#[test]
fn parse_sh_file_input_and_output() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_file_io")?;

    env.create_source_files(&[("build.sh", "gcc -c foo.c\n")])?;

    let result = env.run_bear(&["parse-sh", "-i", "build.sh", "-o", "out.jsonl"])?;
    result.assert_success()?;

    let content = env.read_file("out.jsonl")?;
    let lines: Vec<&str> = content.lines().filter(|line| !line.is_empty()).collect();
    assert_eq!(lines.len(), 1, "expected exactly one event line in the file, got: {content:?}");

    let event: Value = serde_json::from_str(lines[0])?;
    assert_eq!(event["executable"], "gcc");
    assert_eq!(event["arguments"], json!(["gcc", "-c", "foo.c"]));

    Ok(())
}

// Requirements: interception-events-from-shell-text
//
// End-to-end pipeline: `bear parse-sh`'s event stream, fed into
// `bear semantic --input -`, must yield a compilation database entry for
// the compile line -- proving the two subcommands agree on the event
// format documented in `interception-events-format`.
#[test]
#[cfg(has_executable_compiler_c)]
fn parse_sh_output_piped_into_semantic_yields_compilation_database() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_pipe_to_semantic")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    env.create_source_files(&[("foo.c", "int main() { return 0; }")])?;

    let script = format!("{COMPILER_C_PATH} -c foo.c\n");
    let parse_result = env.run_bear_with_stdin(&["parse-sh"], script.as_bytes())?;
    parse_result.assert_success()?;

    let semantic_result = env.run_bear_with_stdin(
        &["semantic", "--input", "-", "--output", "compile_commands.json"],
        parse_result.stdout().as_bytes(),
    )?;
    semantic_result.assert_success()?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "foo.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![COMPILER_C_PATH.to_string(), "-c".to_string(), "foo.c".to_string()]
    ))?;

    Ok(())
}
