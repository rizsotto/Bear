// SPDX-License-Identifier: GPL-3.0-or-later

//! Integration tests for the `bear parse-sh` subcommand: dry-run shell text
//! in, compilation database out. The wiring of the pure lex/interpret stages
//! to the semantic analysis and output pipeline (stdin/file input, the
//! default and named database destinations, stdout streaming, config,
//! append, the `--directory` override, and the exit-code policy).
//!
//! See `docs/requirements/interception-shell-text-parsing.md` for the
//! contract; the crate-level unit tests in
//! `crates/bear/src/parse_sh/interpreter.rs` cover the parsing rules
//! themselves, so these tests focus on the mode's observable behavior.

use crate::fixtures::infrastructure::{CompilationEntryMatcher, compilation_entry};
use crate::fixtures::*;
use anyhow::{Context, Result};
use serde_json::{Value, json};

/// A real `make -n` capture of zlib 1.3.1's build (77 lines; 34 compile
/// commands among the gcc/ar/mv/mkdir/ln/subshell/redirect noise). See
/// `tests/integration/tests/fixtures/data/zlib.make-n.sh`.
const ZLIB_SH: &str = include_str!("../fixtures/data/zlib.make-n.sh");

/// The same zlib 1.3.1 build, captured with `make -n -w`: byte-identical to
/// `ZLIB_SH` except for a leading `make: Entering directory '/tmp/build'`
/// and a trailing `make: Leaving directory '/tmp/build'` marker.
const ZLIB_SH_W: &str = include_str!("../fixtures/data/zlib.make-n-w.sh");

// Requirements: interception-shell-text-parsing
//
// The headline usage: shell text on standard input, nothing else named,
// and the database appears under the ecosystem-contracted default name.
#[test]
#[cfg(has_executable_compiler_c)]
fn parse_sh_stdin_default_writes_default_database() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_stdin_default")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let result = env.run_bear_with_stdin(&["parse-sh"], b"gcc -c foo.c\n")?;
    result.assert_success()?;
    assert!(
        result.stdout().trim().is_empty(),
        "the database goes to the file, not stdout: {}",
        result.stdout()
    );

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "foo.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec!["gcc".to_string(), "-c".to_string(), "foo.c".to_string()]
    ))?;

    Ok(())
}

// Requirements: interception-shell-text-parsing
#[test]
#[cfg(has_executable_compiler_c)]
fn parse_sh_file_input_and_output() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_file_io")?;

    env.create_source_files(&[("build.sh", "gcc -c foo.c\n")])?;

    let result = env.run_bear(&["parse-sh", "-i", "build.sh", "-o", "db.json"])?;
    result.assert_success()?;
    assert!(!env.file_exists("compile_commands.json"), "a named output must not also write the default");

    let db = env.load_compilation_database("db.json")?;
    db.assert_count(1)?;
    db.assert_contains(&CompilationEntryMatcher::new().file("foo.c".to_string()))?;

    Ok(())
}

// Requirements: interception-shell-text-parsing
//
// The database itself may stream to standard output: parse-sh runs no
// build, so stdout is free. Diagnostics stay on stderr, and no file is
// created anywhere.
#[test]
#[cfg(has_executable_compiler_c)]
fn parse_sh_stdout_streaming_emits_database() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_stdout_streaming")?;

    let result = env.run_bear_with_stdin(&["parse-sh", "--output", "-"], b"gcc -c foo.c\n")?;
    result.assert_success()?;

    let entries: Vec<Value> =
        serde_json::from_str(&result.stdout()).context("stdout must be a valid JSON compilation database")?;
    assert_eq!(entries.len(), 1, "expected exactly one compilation entry: {entries:?}");
    assert_eq!(entries[0]["file"], "foo.c");
    assert_eq!(entries[0]["arguments"], json!(["gcc", "-c", "foo.c"]));

    assert!(!env.file_exists("-"), "must not create a file literally named `-`");
    assert!(!env.file_exists("compile_commands.json"), "streaming must not also write the default file");

    Ok(())
}

// Requirements: interception-shell-text-parsing
//
// A streamed database cannot be combined with appending; the combination
// is rejected before any parsing happens.
#[test]
fn parse_sh_stdout_streaming_rejects_append() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_stdout_append")?;

    let result = env.run_bear_with_stdin(&["parse-sh", "--output", "-", "--append"], b"gcc -c foo.c\n")?;

    result.assert_failure()?;
    assert!(
        result.stderr().contains("append"),
        "stderr must explain the append/stdout conflict: {}",
        result.stderr()
    );

    Ok(())
}

// Requirements: interception-shell-text-parsing, output-append
#[test]
#[cfg(has_executable_compiler_c)]
fn parse_sh_append_merges_databases() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_append")?;

    let first = env.run_bear_with_stdin(&["parse-sh"], b"gcc -c a.c\n")?;
    first.assert_success()?;

    let second = env.run_bear_with_stdin(&["parse-sh", "--append"], b"gcc -c b.c\n")?;
    second.assert_success()?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(2)?;
    db.assert_contains(&CompilationEntryMatcher::new().file("a.c".to_string()))?;
    db.assert_contains(&CompilationEntryMatcher::new().file("b.c".to_string()))?;

    Ok(())
}

// Requirements: interception-shell-text-parsing
//
// Configuration applies to parse-sh exactly as to the other
// database-producing modes: with `arguments` added to the duplicate key,
// the plain and `-fPIC` compiles of each zlib source no longer collapse,
// so all 34 compile commands survive as distinct entries (the default
// key yields 17; see `parse_sh_zlib_capture_writes_default_deduped_database`).
#[test]
#[cfg(has_executable_compiler_c)]
fn parse_sh_config_widens_duplicate_detection() -> Result<()> {
    const CONFIG: &str = r#"schema: '4.2'

duplicates:
  match_on:
    - directory
    - file
    - arguments
"#;

    let env = TestEnvironment::new("parse_sh_config_dedup")?;
    let config_path = env.create_config(CONFIG)?;
    let config_path = config_path.to_str().context("config path is not valid UTF-8")?;

    let result = env.run_bear_with_stdin(&["-c", config_path, "parse-sh"], ZLIB_SH.as_bytes())?;
    result.assert_success()?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(34)?;

    Ok(())
}

// Requirements: interception-shell-text-parsing
//
// parse-sh now consumes configuration, so a malformed config in a default
// search location (`bear.yml` in the working directory) aborts the run
// like it aborts every other config-consuming mode. This inverts the
// pre-4.2.0 filter design, where parse-sh loaded no config at all.
#[test]
fn parse_sh_malformed_default_location_config_fails() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_malformed_default_config")?;
    // An unclosed flow sequence is a hard YAML parse error.
    std::fs::write(env.test_dir().join("bear.yml"), "format:\n  paths: [unclosed\n")?;

    let result = env.run_bear_with_stdin(&["parse-sh"], b"gcc -c foo.c\n")?;

    result.assert_failure()?;
    assert!(
        !result.stderr().trim().is_empty(),
        "the config load failure must be reported on stderr, not silent"
    );

    Ok(())
}

// Requirements: interception-shell-text-parsing, cli-exit-codes
#[test]
fn parse_sh_all_skipped_input_exits_non_zero() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_all_skipped")?;

    // A subshell group is outside the supported shell subset: the whole
    // line must be skipped, not guessed at.
    let result = env.run_bear_with_stdin(&["parse-sh"], b"(ranlib libz.a || true) >/dev/null 2>&1\n")?;
    result.assert_failure()?;
    assert!(result.stderr().contains("skipped"), "stderr must report the skip: {}", result.stderr());

    Ok(())
}

// Requirements: interception-shell-text-parsing, cli-exit-codes
//
// A clean parse with no recognized compiler is not an error: non-compiler
// commands are valid candidates that simply produce no entries, silently.
// This is distinct from the all-skipped failure above, where the parser
// could not handle the input at all. Runs at the default log level, where
// a nothing-skipped run keeps stderr quiet (the harness's `RUST_LOG=info`
// default would surface the zero-skip summary line).
#[test]
fn parse_sh_non_compiler_only_input_writes_empty_database() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_non_compiler_only")?;

    let result = env.run_bear_with_stdin_default_log(&["parse-sh"], b"mv objs/a.o a.lo\n")?;

    result.assert_success()?;
    assert!(
        !result.stderr().contains("skipped"),
        "a parsed non-compiler line is not a skip and must not be reported as one: {}",
        result.stderr()
    );
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(0)?;

    Ok(())
}

// Requirements: interception-shell-text-parsing
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

    let stderr = result.stderr();
    assert!(stderr.contains("skipped"), "stderr must report the skip without RUST_LOG set: {stderr}");
    assert!(stderr.contains("line 1"), "stderr must cite the skipped line number: {stderr}");
    assert!(stderr.contains("subshell"), "stderr must name the subshell as the skip reason: {stderr}");

    Ok(())
}

// Requirements: cli-diagnostic-format
//
// Without `RUST_LOG`, diagnostics use the UNIX user format: each line is
// prefixed with the emitting process (`bear`) and warnings carry a
// `warning:` qualifier. The developer tag `bear[<pid>]` must be absent, so
// this pins the user view rather than merely "some text on stderr".
#[test]
fn parse_sh_user_format_prefixes_program_and_severity() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_user_format")?;

    let script = b"(subshell command) >/dev/null\ngcc -c foo.c\n";
    let result = env.run_bear_with_stdin_default_log(&["parse-sh"], script)?;
    result.assert_success()?;

    let stderr = result.stderr();
    assert!(
        stderr.contains("bear: warning:"),
        "user format prefixes the process and qualifies the severity: {stderr}"
    );
    assert!(
        !stderr.contains("bear["),
        "the developer process[pid] tag must not appear without RUST_LOG: {stderr}"
    );

    Ok(())
}

// Requirements: cli-diagnostic-format
//
// With `RUST_LOG` set (the harness defaults it to `info`), diagnostics use
// the developer format: every line carries the process identity and pid as
// `bear[<pid>]`, distinguishing Bear's processes when they interleave.
#[test]
fn parse_sh_developer_format_tags_process_identity() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_developer_format")?;

    let script = b"(subshell command) >/dev/null\ngcc -c foo.c\n";
    let result = env.run_bear_with_stdin(&["parse-sh"], script)?;
    result.assert_success()?;

    let stderr = result.stderr();
    assert!(
        stderr.contains("bear["),
        "developer format tags each line with the process identity and pid: {stderr}"
    );
    assert!(
        !stderr.contains("bear: warning:"),
        "the user-format prefix must not appear once RUST_LOG selects the developer view: {stderr}"
    );

    Ok(())
}

// Requirements: interception-shell-text-parsing
//
// When any line is skipped, a stderr summary reports both counts, and the
// skip count is physical lines, not command segments: the `for` line below
// holds three segments but is one skipped line. The per-line warnings also
// say "skipped", so this pins the summary text itself, which no other
// assertion covers -- the summary could vanish or regress to counting
// segments without any other test failing.
#[test]
fn parse_sh_skip_summary_counts_lines_not_segments() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_skip_summary")?;

    let script = b"gcc -c foo.c\nfor f in *.c; do gcc -c $f; done\n";
    let result = env.run_bear_with_stdin(&["parse-sh"], script)?;

    result.assert_success()?;
    let stderr = result.stderr();
    assert!(
        stderr.contains("1 command(s) parsed, 1 line(s) skipped"),
        "the summary must count lines, not segments: {stderr}"
    );

    Ok(())
}

// Requirements: interception-shell-text-parsing, output-env-derived-flags
//
// A leading `VAR=value` assignment is that command's environment, not its
// executable, and it participates in semantic analysis like an intercepted
// execution's environment: `CPATH` folds into the entry as explicit include
// flags (the default; see output-env-derived-flags). The redirection is
// stripped from the recorded arguments.
#[test]
#[cfg(has_executable_compiler_c)]
fn parse_sh_leading_assignment_reaches_semantic_as_environment() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_environment")?;

    let script = b"CPATH=/opt/include gcc -c foo.c -o foo.o >/dev/null 2>&1\n";
    let result = env.run_bear_with_stdin(&["parse-sh"], script)?;
    result.assert_success()?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    let entry = &db.entries()[0];
    let arguments: Vec<&str> = entry["arguments"]
        .as_array()
        .context("arguments must be an array")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(arguments[0], "gcc", "the assignment must not be taken as the executable: {arguments:?}");
    let include_at = arguments.iter().position(|arg| *arg == "-I");
    assert!(
        include_at.is_some_and(|at| arguments.get(at + 1) == Some(&"/opt/include")),
        "CPATH must fold into explicit include flags: {arguments:?}"
    );
    assert!(
        !arguments.iter().any(|arg| arg.contains('>') || *arg == "/dev/null" || *arg == "2>&1"),
        "redirection words must not reach the entry: {arguments:?}"
    );

    Ok(())
}

// Requirements: interception-shell-text-parsing, cli-exit-codes
//
// An assignment-only line sets shell state but executes nothing, so it is
// neither a command nor a skip; input made only of such lines is the empty
// case, not the all-skipped failure.
#[test]
fn parse_sh_assignment_only_input_exits_zero_as_no_commands() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_assignment_only")?;

    let result = env.run_bear_with_stdin(&["parse-sh"], b"FOO=bar\n")?;

    result.assert_success()?;
    assert!(
        result.stderr().contains("no commands found"),
        "stderr must carry the empty-input notice: {}",
        result.stderr()
    );
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(0)?;

    Ok(())
}

// Requirements: interception-shell-text-parsing, cli-exit-codes
#[test]
fn parse_sh_non_utf8_input_fails_with_clear_error() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_non_utf8")?;

    let result = env.run_bear_with_stdin(&["parse-sh"], b"gcc \xff\xfe -c foo.c\n")?;

    result.assert_failure()?;
    assert!(result.stderr().contains("UTF-8"), "stderr must name the encoding problem: {}", result.stderr());

    Ok(())
}

// Requirements: interception-shell-text-parsing, cli-exit-codes
#[test]
fn parse_sh_missing_input_file_fails() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_missing_input")?;

    let result = env.run_bear(&["parse-sh", "--input", "missing.sh"])?;

    result.assert_failure()?;
    assert!(
        result.stderr().contains("Shell text file not found"),
        "stderr must name the missing input file: {}",
        result.stderr()
    );

    Ok(())
}

// Requirements: interception-shell-text-parsing, cli-exit-codes
#[test]
fn parse_sh_unwritable_output_file_fails() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_unwritable_output")?;

    let result =
        env.run_bear_with_stdin(&["parse-sh", "--output", "no_such_dir/db.json"], b"gcc -c foo.c\n")?;

    result.assert_failure()?;
    assert!(
        !result.stderr().trim().is_empty(),
        "the unwritable output destination must be reported on stderr, not silent"
    );

    Ok(())
}

// Requirements: interception-shell-text-parsing, cli-exit-codes
#[test]
fn parse_sh_empty_input_exits_zero_with_warning() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_empty_input")?;

    let result = env.run_bear_with_stdin(&["parse-sh"], b"")?;
    result.assert_success()?;
    assert!(
        result.stderr().contains("no commands found"),
        "stderr must warn that no commands were found: {}",
        result.stderr()
    );
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(0)?;

    Ok(())
}

// Requirements: interception-shell-text-parsing
#[test]
#[cfg(has_executable_compiler_c)]
fn parse_sh_directory_flag_sets_entry_directory() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_directory_flag")?;

    // The default path format records paths as-is without touching the
    // filesystem, so a directory that only existed where the log was
    // captured is fine.
    let result = env.run_bear_with_stdin(&["parse-sh", "--directory", "/custom/build"], b"gcc -c foo.c\n")?;
    result.assert_success()?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    assert_eq!(db.entries()[0]["directory"], "/custom/build");

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

/// Asserts that every entry in `db` records the fixture's bare `gcc` token
/// verbatim in `arguments[0]`: semantic analysis never rewrites the
/// executable, so the spelling from the `make -n` capture survives as-is.
fn assert_all_entries_use_gcc(db: &CompilationDatabase) -> Result<()> {
    for entry in db.entries() {
        let arg0 =
            entry["arguments"][0].as_str().with_context(|| format!("entry missing arguments[0]: {entry}"))?;
        assert_eq!(arg0, "gcc", "arguments[0] must be the observed bare gcc (entry: {entry})");
    }
    Ok(())
}

// Requirements: interception-shell-text-parsing
//
// End-to-end coverage over a REAL `make -n` capture (zlib 1.3.1), in one
// step. The default duplicate key (directory+file) collapses each source's
// plain and `-fPIC` compile into one entry (first occurrence - the plain
// compile - wins), so 34 compile commands over 17 distinct sources yield
// 17 database entries; the ar/mv/mkdir/ln noise contributes nothing, and
// the unsupported lines are reported as skips without failing the run.
#[test]
#[cfg(has_executable_compiler_c)]
fn parse_sh_zlib_capture_writes_default_deduped_database() -> Result<()> {
    let env = TestEnvironment::new("parse_sh_zlib_default")?;

    let result = env.run_bear_with_stdin(&["parse-sh"], ZLIB_SH.as_bytes())?;
    result.assert_success()?;
    let stderr = result.stderr();
    assert!(stderr.contains("skipped"), "stderr must report the skip: {stderr}");
    assert!(stderr.contains("line 18"), "stderr must cite the skipped line number: {stderr}");
    assert!(stderr.contains("subshell"), "stderr must name the subshell as the skip reason: {stderr}");

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(17)?;
    for file in ZLIB_EXPECTED_FILES {
        db.assert_contains(&CompilationEntryMatcher::new().file(file.to_string()))
            .with_context(|| format!("missing expected file entry: {file}"))?;
    }
    assert_all_entries_use_gcc(&db)?;

    Ok(())
}

// Requirements: interception-shell-text-parsing
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

    let result = env.run_bear_with_stdin(&["parse-sh"], ZLIB_SH_W.as_bytes())?;
    result.assert_success()?;

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

// Requirements: interception-shell-text-parsing
//
// Synthetic recursive-make directory tracking, end to end: `make[1]:
// Entering directory` / `Leaving directory` markers must drive the working
// directory, and that working directory must reach each entry's recorded
// `directory`. `a.c` is compiled inside the announced subdirectory; `b.c`
// is compiled after the matching `Leaving directory`, back in parse-sh's
// own cwd (the test dir).
#[test]
#[cfg(has_executable_compiler_c)]
fn parse_sh_recursive_make_markers_drive_entry_directory() -> Result<()> {
    const SCRIPT: &str = "\
make[1]: Entering directory '/build/sub'
gcc -c a.c
make[1]: Leaving directory '/build/sub'
gcc -c b.c
";

    let env = TestEnvironment::new("parse_sh_recursive_make_directory")?;

    let result = env.run_bear_with_stdin(&["parse-sh"], SCRIPT.as_bytes())?;
    result.assert_success()?;

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
