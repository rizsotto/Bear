// SPDX-License-Identifier: GPL-3.0-or-later

//! Integration tests for the `bear parse-sh` subcommand: the wiring of the
//! pure lex/interpret stages to real I/O (stdin/file input, stdout/file
//! output, the `--directory` override, and the exit-code policy).
//!
//! See `docs/requirements/interception-events-from-shell-text.md` for the
//! contract; the crate-level unit tests in
//! `crates/bear/src/parse_sh/interpreter.rs` cover the parsing rules
//! themselves, so these tests focus on the mode's I/O behavior.

use crate::fixtures::infrastructure::{CompilationEntryMatcher, compilation_entry};
use crate::fixtures::*;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::path::Path;

/// A real `make -n` capture of zlib 1.3.1's build (77 lines; 34 compile
/// commands among the gcc/ar/mv/mkdir/ln/subshell/redirect noise). See
/// `tests/integration/tests/fixtures/data/zlib.make-n.sh`.
const ZLIB_SH: &str = include_str!("../fixtures/data/zlib.make-n.sh");

/// The same zlib 1.3.1 build, captured with `make -n -w`: byte-identical to
/// `ZLIB_SH` except for a leading `make: Entering directory '/tmp/build'`
/// and a trailing `make: Leaving directory '/tmp/build'` marker.
const ZLIB_SH_W: &str = include_str!("../fixtures/data/zlib.make-n-w.sh");

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
//
// `--config` shapes semantic analysis, which parse-sh does not run: the mode
// emits a raw event stream and never consults config. The invocation is
// rejected at argument-parse time, before any config is loaded, so a valid
// config file present here still fails -- proving the rejection is about the
// mode, not the file's contents.
#[test]
fn parse_sh_rejects_config_option() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_rejects_config")?;
    let config = env.create_config("schema: \"4.1\"\n")?;

    let result =
        env.run_bear_with_stdin(&["--config", config.to_str().unwrap(), "parse-sh"], b"gcc -c foo.c\n")?;

    result.assert_failure()?;
    assert!(result.stdout().trim().is_empty(), "no event stream must be produced: {}", result.stdout());
    assert!(
        result.stderr().contains("parse-sh"),
        "stderr must explain that config does not apply to parse-sh: {}",
        result.stderr()
    );

    Ok(())
}

// Requirements: interception-events-from-shell-text
//
// Skip reports must reach stderr by default, without the caller having to
// opt in via `RUST_LOG`. Runs with `RUST_LOG` explicitly unset (the harness's
// other helpers default it to `info`, which would hide a regression in
// bear-driver's own built-in default log level).
#[test]
fn parse_sh_default_log_level_reports_skips_without_rust_log() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_default_log_level")?;

    let script = b"(subshell command) >/dev/null\ngcc -c foo.c\n";
    let result = env.run_bear_with_stdin_default_log(&["parse-sh"], script)?;
    result.assert_success()?;

    let stdout = result.stdout();
    let lines: Vec<&str> = stdout.lines().filter(|line| !line.is_empty()).collect();
    assert_eq!(lines.len(), 1, "expected exactly one event line, got: {stdout:?}");

    let stderr = result.stderr();
    assert!(stderr.contains("skipped"), "stderr must report the skip without RUST_LOG set: {stderr}");
    assert!(stderr.contains("line 1"), "stderr must cite the skipped line number: {stderr}");
    assert!(stderr.contains("subshell"), "stderr must name the subshell as the skip reason: {stderr}");

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

// Requirements: interception-events-from-shell-text, interception-events-format
//
// Same pipeline as `parse_sh_output_piped_into_semantic_yields_compilation_database`,
// but the consumer end also streams its output: `bear parse-sh | bear semantic
// --output -` must produce the compilation database on standard output, with
// no intermediate file anywhere in the chain.
#[test]
#[cfg(has_executable_compiler_c)]
fn parse_sh_output_piped_into_semantic_stdout_yields_compilation_database() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_pipe_to_semantic_stdout")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    env.create_source_files(&[("foo.c", "int main() { return 0; }")])?;

    let script = format!("{COMPILER_C_PATH} -c foo.c\n");
    let parse_result = env.run_bear_with_stdin(&["parse-sh"], script.as_bytes())?;
    parse_result.assert_success()?;

    let semantic_result = env.run_bear_with_stdin(
        &["semantic", "--input", "-", "--output", "-"],
        parse_result.stdout().as_bytes(),
    )?;
    semantic_result.assert_success()?;

    let entries: Vec<Value> = serde_json::from_str(&semantic_result.stdout())
        .context("stdout must be a valid JSON compilation database")?;
    assert_eq!(entries.len(), 1, "expected exactly one compilation entry: {entries:?}");
    assert_eq!(entries[0]["file"], "foo.c");
    // Compare canonically: on macOS the process cwd resolves the
    // `/var` -> `/private/var` symlink, so the recorded directory differs
    // from the temp dir's symlinked path even though they are the same dir.
    let recorded_dir = entries[0]["directory"].as_str().context("directory must be a string")?;
    assert_eq!(std::fs::canonicalize(recorded_dir)?, std::fs::canonicalize(temp_dir)?);
    assert_eq!(entries[0]["arguments"], json!([COMPILER_C_PATH, "-c", "foo.c"]));

    assert!(!env.file_exists("-"), "must not create a file literally named `-`");

    Ok(())
}

/// The 17 distinct source files the zlib fixture compiles, in the order
/// `make -n` first mentions them (the default duplicate key collapses each
/// source's plain and `-fPIC` compile into one entry, keeping the first
/// occurrence).
const ZLIB_EXPECTED_FILES: [&str; 17] = [
    "../zlib-1.3.1/test/example.c",
    "../zlib-1.3.1/adler32.c",
    "../zlib-1.3.1/crc32.c",
    "../zlib-1.3.1/deflate.c",
    "../zlib-1.3.1/infback.c",
    "../zlib-1.3.1/inffast.c",
    "../zlib-1.3.1/inflate.c",
    "../zlib-1.3.1/inftrees.c",
    "../zlib-1.3.1/trees.c",
    "../zlib-1.3.1/zutil.c",
    "../zlib-1.3.1/compress.c",
    "../zlib-1.3.1/uncompr.c",
    "../zlib-1.3.1/gzclose.c",
    "../zlib-1.3.1/gzlib.c",
    "../zlib-1.3.1/gzread.c",
    "../zlib-1.3.1/gzwrite.c",
    "../zlib-1.3.1/test/minigzip.c",
];

/// Asserts that every entry in `db` records a bare `gcc` invocation: its
/// `arguments[0]` basename is `gcc`, never a literal path. `bear semantic`'s
/// shared `ResolveExecutable` rewrites the fixture's bare `gcc` token to an
/// absolute host path (`/usr/bin/gcc`, or a ccache masquerade path), so the
/// path itself is host-specific and only the basename is portable.
fn assert_all_entries_use_gcc(db: &CompilationDatabase) -> Result<()> {
    for entry in db.entries() {
        let arg0 =
            entry["arguments"][0].as_str().with_context(|| format!("entry missing arguments[0]: {entry}"))?;
        let basename = Path::new(arg0).file_name().and_then(|name| name.to_str()).unwrap_or(arg0);
        assert_eq!(basename, "gcc", "arguments[0] basename must be gcc, got: {arg0} (entry: {entry})");
    }
    Ok(())
}

// Requirements: interception-events-from-shell-text, interception-events-format
//
// End-to-end coverage over a REAL `make -n` capture (zlib 1.3.1): pipes
// `bear parse-sh` into `bear semantic` with the default config, proving the
// whole producer (parse-sh) and consumer (semantic) agree on the event
// format. The default duplicate key (directory+file) collapses each
// source's plain and `-fPIC` compile into one entry (first occurrence -
// the plain compile - wins), so 34 compile commands over 17 distinct
// sources yield 17 database entries.
#[test]
#[cfg(has_executable_compiler_c)]
fn parse_sh_zlib_capture_piped_into_semantic_yields_default_deduped_database() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_zlib_default")?;

    let parse_result = env.run_bear_with_stdin(&["parse-sh"], ZLIB_SH.as_bytes())?;
    parse_result.assert_success()?;
    let parse_stderr = parse_result.stderr();
    assert!(parse_stderr.contains("skipped"), "stderr must report the skip: {parse_stderr}");
    assert!(parse_stderr.contains("line 18"), "stderr must cite the skipped line number: {parse_stderr}");
    assert!(
        parse_stderr.contains("subshell"),
        "stderr must name the subshell as the skip reason: {parse_stderr}"
    );

    let semantic_result = env.run_bear_with_stdin(
        &["semantic", "--input", "-", "--output", "compile_commands.json"],
        parse_result.stdout().as_bytes(),
    )?;
    semantic_result.assert_success()?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(17)?;
    for file in ZLIB_EXPECTED_FILES {
        db.assert_contains(&CompilationEntryMatcher::new().file(file.to_string()))
            .with_context(|| format!("missing expected file entry: {file}"))?;
    }
    assert_all_entries_use_gcc(&db)?;

    Ok(())
}

// Requirements: interception-events-from-shell-text, interception-events-format
//
// Same real zlib capture as above, but recorded with `make -n -w`, which
// wraps it in a top-level `make: Entering directory '/tmp/build'` /
// `Leaving directory '/tmp/build'` pair. Proves real GNU make `-w` directory
// markers drive the recorded `directory` end to end, on a real capture: the
// markers add no entries of their own (still 17, via the default dedup), but
// every entry's `directory` must come from the announced `/tmp/build`.
#[test]
#[cfg(has_executable_compiler_c)]
fn parse_sh_zlib_make_n_w_capture_records_marker_directory() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_zlib_make_n_w")?;

    let parse_result = env.run_bear_with_stdin(&["parse-sh"], ZLIB_SH_W.as_bytes())?;
    parse_result.assert_success()?;

    let semantic_result = env.run_bear_with_stdin(
        &["semantic", "--input", "-", "--output", "compile_commands.json"],
        parse_result.stdout().as_bytes(),
    )?;
    semantic_result.assert_success()?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(17)?;
    for entry in db.entries() {
        assert_eq!(
            entry["directory"], "/tmp/build",
            "entry directory must come from the 'Entering directory' marker: {entry}"
        );
    }
    assert_all_entries_use_gcc(&db)?;

    Ok(())
}

// Requirements: interception-events-from-shell-text, interception-events-format
//
// Same real zlib capture as above, but with a config that adds `arguments`
// to the duplicate key: the plain and `-fPIC` compiles of each source now
// differ (different flags), so none collapse and all 34 compile commands
// survive as distinct entries. Confirms parse-sh's event stream carries
// enough detail (distinct argv per invocation) for `bear semantic`'s
// arguments-aware deduplication to tell them apart.
#[test]
#[cfg(has_executable_compiler_c)]
fn parse_sh_zlib_capture_with_arguments_dedup_yields_all_compile_entries() -> Result<()> {
    const CONFIG: &str = r#"schema: '4.1'

duplicates:
  match_on:
    - directory
    - file
    - arguments
"#;

    let env = TestEnvironment::new("parse_sh_zlib_arguments_dedup")?;
    let config_path = env.create_config(CONFIG)?;
    let config_path = config_path.to_str().context("config path is not valid UTF-8")?;

    let parse_result = env.run_bear_with_stdin(&["parse-sh"], ZLIB_SH.as_bytes())?;
    parse_result.assert_success()?;

    let semantic_result = env.run_bear_with_stdin(
        &["-c", config_path, "semantic", "--input", "-", "--output", "compile_commands.json"],
        parse_result.stdout().as_bytes(),
    )?;
    semantic_result.assert_success()?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(34)?;
    assert_all_entries_use_gcc(&db)?;

    Ok(())
}

// Requirements: interception-events-from-shell-text
//
// Synthetic recursive-make directory tracking, end to end: `make[1]:
// Entering directory` / `Leaving directory` markers must drive parse-sh's
// working directory, and that working directory must flow through
// `bear semantic` into each entry's recorded `directory`. `a.c` is compiled
// inside the announced subdirectory; `b.c` is compiled after the matching
// `Leaving directory`, back in parse-sh's own cwd (the test dir).
#[test]
#[cfg(has_executable_compiler_c)]
fn parse_sh_recursive_make_markers_drive_semantic_directory() -> Result<()> {
    const SCRIPT: &str = "\
make[1]: Entering directory '/build/sub'
gcc -c a.c
make[1]: Leaving directory '/build/sub'
gcc -c b.c
";

    let env = TestEnvironment::new("parse_sh_recursive_make_directory")?;

    let parse_result = env.run_bear_with_stdin(&["parse-sh"], SCRIPT.as_bytes())?;
    parse_result.assert_success()?;

    let semantic_result = env.run_bear_with_stdin(
        &["semantic", "--input", "-", "--output", "compile_commands.json"],
        parse_result.stdout().as_bytes(),
    )?;
    semantic_result.assert_success()?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(2)?;

    let a_entry =
        db.entries().iter().find(|entry| entry["file"] == "a.c").context("expected an entry for a.c")?;
    assert_eq!(a_entry["directory"], "/build/sub");

    let b_entry =
        db.entries().iter().find(|entry| entry["file"] == "b.c").context("expected an entry for b.c")?;
    assert_ne!(
        b_entry["directory"], "/build/sub",
        "b.c must be recorded in parse-sh's own cwd, not the entered subdirectory"
    );

    Ok(())
}
