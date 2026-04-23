// SPDX-License-Identifier: GPL-3.0-or-later

//! Configuration tests for Bear integration
//!
//! These tests verify that Bear correctly handles configuration files
//! and applies filtering rules, adapted to Bear's actual configuration format.

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::*;
use anyhow::Result;

/// Test basic configuration file loading
/// Verifies Bear can load a valid configuration file
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn basic_config_loading() -> Result<()> {
    let env = TestEnvironment::new("basic_config")?;

    // Create source file
    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let build_commands = format!("{} -c test.c", COMPILER_C_PATH);
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Create basic valid config
    let config = format!(
        r#"
schema: "4.1"

intercept:
  mode: preload
  path: "{}"

sources:
  only_existing_files: true

format:
  paths:
    directory: as-is
    file: as-is
"#,
        PRELOAD_LIBRARY_PATH
    );

    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    // Run bear with config
    let _output = env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--config",
        config_path.to_str().unwrap(),
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Verify the compilation database was created
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify the entry contains the expected compilation command
    let expected_args = vec![COMPILER_C_PATH.to_string(), "-c".to_string(), "test.c".to_string()];

    let matcher = CompilationEntryMatcher::new()
        .file("test.c")
        .directory(env.test_dir().to_str().unwrap())
        .arguments(expected_args);

    db.assert_contains(&matcher)?;

    Ok(())
}

/// Test compiler ignore functionality
/// Verifies that compilers marked with ignore: true are excluded
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_compiler_cxx, has_executable_shell))]
fn compiler_ignore_config() -> Result<()> {
    let env = TestEnvironment::new("compiler_ignore")?;

    // Create source files for both C and C++
    env.create_source_files(&[
        ("source.c", "int main() { return 0; }"),
        ("source.cpp", "int main() { return 0; }"),
    ])?;

    let build_commands = format!("{} -c source.c\n{} -c source.cpp", COMPILER_C_PATH, COMPILER_CXX_PATH);
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Create config that ignores the C++ compiler
    let config = format!(
        r#"
schema: "4.1"

intercept:
  mode: preload
  path: "{preload_path}"

compilers:
  - path: "{cxx}"
    ignore: true

sources:
  only_existing_files: true
"#,
        preload_path = PRELOAD_LIBRARY_PATH,
        cxx = COMPILER_CXX_PATH
    );

    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    // Run bear with config
    let _output = env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--config",
        config_path.to_str().unwrap(),
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Should only capture C compiler invocation, not C++
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify the entry contains the expected C compilation command
    let expected_args = vec![COMPILER_C_PATH.to_string(), "-c".to_string(), "source.c".to_string()];

    let matcher = CompilationEntryMatcher::new()
        .file("source.c")
        .directory(env.test_dir().to_str().unwrap())
        .arguments(expected_args);

    db.assert_contains(&matcher)?;

    Ok(())
}

/// Test source file filtering
/// Verifies only_existing_files configuration option
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn source_file_filtering() -> Result<()> {
    let env = TestEnvironment::new("source_filtering")?;

    // Create only one source file (the other will be missing)
    env.create_source_files(&[("existing.c", "int main() { return 0; }")])?;

    let build_commands = format!(
        "{} -c existing.c\n{} -c nonexistent.c 2>/dev/null || true",
        COMPILER_C_PATH, COMPILER_C_PATH
    );
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Config to include only existing source files
    let config = format!(
        r#"
schema: "4.1"

intercept:
  mode: preload
  path: "{}"

sources:
  only_existing_files: true
"#,
        PRELOAD_LIBRARY_PATH
    );

    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    let _output = env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--config",
        config_path.to_str().unwrap(),
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Only existing.c should be in the output
    let db = env.load_compilation_database("compile_commands.json")?;

    // Verify that we have entries and at least one is for existing.c
    let expected_args = vec![COMPILER_C_PATH.to_string(), "-c".to_string(), "existing.c".to_string()];

    let matcher = CompilationEntryMatcher::new()
        .file("existing.c")
        .directory(env.test_dir().to_str().unwrap())
        .arguments(expected_args);

    db.assert_contains(&matcher)?;

    Ok(())
}

/// Test source directory filter with include/exclude rules.
/// Verifies last-match-wins semantics and default-include behavior end-to-end
/// through the YAML config -> output pipeline path.
// Requirements: output-source-directory-filter
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn source_directory_filter_config() -> Result<()> {
    let env = TestEnvironment::new("source_directory_filter")?;

    // Four files across three directories: one top-level matched by `include src`,
    // one excluded by `exclude src/test`, one re-included by a more specific
    // `include src/test/integration` (last-match-wins), and one outside any rule
    // (default include).
    env.create_source_files(&[
        ("src/main.c", "int main() { return 0; }"),
        ("src/test/unit.c", "int unit() { return 0; }"),
        ("src/test/integration/api.c", "int api() { return 0; }"),
        ("lib/util.c", "int util() { return 0; }"),
    ])?;

    let build_commands = [
        format!("{} -c src/main.c -o src/main.o", COMPILER_C_PATH),
        format!("{} -c src/test/unit.c -o src/test/unit.o", COMPILER_C_PATH),
        format!("{} -c src/test/integration/api.c -o src/test/integration/api.o", COMPILER_C_PATH),
        format!("{} -c lib/util.c -o lib/util.o", COMPILER_C_PATH),
    ]
    .join("\n");
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    let config = format!(
        r#"
schema: "4.1"

intercept:
  mode: preload
  path: "{preload}"

sources:
  directories:
    - path: src
      action: include
    - path: src/test
      action: exclude
    - path: src/test/integration
      action: include

format:
  paths:
    directory: as-is
    file: as-is
"#,
        preload = PRELOAD_LIBRARY_PATH
    );
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--config",
        config_path.to_str().unwrap(),
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;

    // Should contain: src/main.c (include), src/test/integration/api.c (re-included
    // by the more specific rule), lib/util.c (default include).
    db.assert_contains(&CompilationEntryMatcher::new().file("src/main.c"))?;
    db.assert_contains(&CompilationEntryMatcher::new().file("src/test/integration/api.c"))?;
    db.assert_contains(&CompilationEntryMatcher::new().file("lib/util.c"))?;

    // Must NOT contain src/test/unit.c - excluded by `exclude src/test`.
    let excluded = db
        .entries()
        .iter()
        .any(|entry| entry.get("file").and_then(|v| v.as_str()) == Some("src/test/unit.c"));
    assert!(!excluded, "src/test/unit.c should have been excluded by `exclude src/test` rule");

    Ok(())
}

/// Test path format configuration
/// Verifies different path formatting options
// Requirements: output-path-format
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn path_format_config() -> Result<()> {
    let env = TestEnvironment::new("path_format")?;

    env.create_source_files(&[("src/main.c", "int main() { return 0; }")])?;

    let build_commands = format!("cd src && {} -c main.c", COMPILER_C_PATH);
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Test absolute path format
    let config = format!(
        r#"
schema: "4.1"

intercept:
  mode: preload
  path: "{}"

format:
  paths:
    directory: absolute
    file: absolute

sources:
  only_existing_files: true
"#,
        PRELOAD_LIBRARY_PATH
    );

    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    let _output = env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--config",
        config_path.to_str().unwrap(),
        "--",
        "sh",
        script_path.to_str().unwrap(),
    ])?;

    // Verify the format is applied
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify the entry contains the expected compilation command
    // When absolute path format is used, the source file argument is also absolute
    let src_dir = env.test_dir().join("src");
    let absolute_src_dir = src_dir.canonicalize().unwrap_or_else(|_| src_dir.clone());
    let absolute_file_path = absolute_src_dir.join("main.c");

    let expected_args =
        vec![COMPILER_C_PATH.to_string(), "-c".to_string(), absolute_file_path.to_str().unwrap().to_string()];

    // For absolute path format, we expect the file and directory to be absolute paths

    let matcher = CompilationEntryMatcher::new()
        .file(absolute_file_path.to_str().unwrap())
        .directory(absolute_src_dir.to_str().unwrap())
        .arguments(expected_args);

    db.assert_contains(&matcher)?;

    Ok(())
}

/// With `file: canonical`, symlinked source paths are written as the resolved
/// real path (symlinks followed).
// Requirements: output-path-format
#[test]
#[cfg(target_family = "unix")]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn canonical_path_format_resolves_symlinks() -> Result<()> {
    use std::os::unix::fs::symlink;

    let env = TestEnvironment::new("canonical_symlinks")?;

    // Real source under real/; src/ is a symlink pointing at real/. Compiling
    // via src/main.c records "src/main.c" in the event, which canonical must
    // resolve back to .../real/main.c.
    env.create_source_files(&[("real/main.c", "int main() { return 0; }")])?;
    symlink(env.test_dir().join("real"), env.test_dir().join("src"))?;

    let build = format!("{} -c src/main.c -o src/main.o", COMPILER_C_PATH);
    let script = env.create_shell_script("build.sh", &build)?;

    let config = format!(
        r#"
schema: "4.1"

intercept:
  mode: preload
  path: "{preload}"

format:
  paths:
    directory: canonical
    file: canonical
"#,
        preload = PRELOAD_LIBRARY_PATH
    );
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--config",
        config_path.to_str().unwrap(),
        "--",
        SHELL_PATH,
        script.to_str().unwrap(),
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    let file = db
        .entries()
        .first()
        .and_then(|e| e.get("file").and_then(|v| v.as_str()))
        .expect("expected at least one entry")
        .to_string();

    assert!(
        file.ends_with("/real/main.c"),
        "canonical file field must resolve symlinks; got {file}, expected path ending with /real/main.c"
    );
    assert!(
        !file.contains("/src/"),
        "canonical file field must not contain the symlink segment /src/; got {file}"
    );

    Ok(())
}

/// With `directory: absolute` and `file: relative`, an absolute source path
/// observed at interception is rewritten relative to the (formatted) directory.
// Requirements: output-path-format
#[test]
#[cfg(target_family = "unix")]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn relative_file_format_is_relative_to_directory() -> Result<()> {
    let env = TestEnvironment::new("relative_file_format")?;

    env.create_source_files(&[("src/main.c", "int main() { return 0; }")])?;

    // Compile using an absolute source path so the intercepted event records
    // the absolute form; `file: relative` must then rewrite it to src/main.c.
    let build = format!(r#"{} -c "$(pwd)/src/main.c" -o src/main.o"#, COMPILER_C_PATH);
    let script = env.create_shell_script("build.sh", &build)?;

    let config = format!(
        r#"
schema: "4.1"

intercept:
  mode: preload
  path: "{preload}"

format:
  paths:
    directory: absolute
    file: relative
"#,
        preload = PRELOAD_LIBRARY_PATH
    );
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--config",
        config_path.to_str().unwrap(),
        "--",
        SHELL_PATH,
        script.to_str().unwrap(),
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    let entry = db.entries().first().expect("expected at least one entry");
    let file = entry.get("file").and_then(|v| v.as_str()).unwrap_or("");
    let directory = entry.get("directory").and_then(|v| v.as_str()).unwrap_or("");

    assert_eq!(file, "src/main.c", "file must be relative to directory, got {file:?}");
    assert!(std::path::Path::new(directory).is_absolute(), "directory must be absolute, got {directory:?}");

    Ok(())
}

/// With `file: canonical` and a source that does not exist at output-write
/// time, Bear must not drop the entry: it falls back to the unformatted path
/// (and logs a warning). Exercised via the `semantic` subcommand so we can
/// feed a hand-crafted event referencing a ghost source.
// Requirements: output-path-format
#[test]
#[cfg(target_family = "unix")]
fn canonical_file_format_falls_back_for_missing_source() -> Result<()> {
    use serde_json::json;

    let env = TestEnvironment::new("canonical_missing_fallback")?;

    // Working dir exists (so directory canonicalization succeeds and the entry
    // is kept); the source file does not exist, so file canonicalization must
    // fall back to the unformatted path.
    let temp_dir = env.test_dir().to_str().unwrap().to_string();
    let event = json!({
        "pid": 12345,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "ghost.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });
    env.create_source_files(&[("events.json", &event.to_string())])?;

    let config = r#"
schema: "4.1"

format:
  paths:
    directory: canonical
    file: canonical
"#;
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    env.run_bear_success(&[
        "--config",
        config_path.to_str().unwrap(),
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    let file = db
        .entries()
        .first()
        .and_then(|e| e.get("file").and_then(|v| v.as_str()))
        .expect("entry must survive the canonical-file fallback");
    assert_eq!(file, "ghost.c", "file field must fall back to the unformatted path; got {file:?}");

    Ok(())
}

/// Reproduces GitHub issue #692: with `directory: relative`, the working
/// directory resolves relative to itself and produces an empty path, which
/// fails entry validation and aborts the output pipeline. The symptom the
/// reporter saw was repeated "sending on a disconnected channel" errors and
/// no compilation database being written.
// Requirements: output-path-format
#[test]
#[cfg(target_family = "unix")]
fn relative_directory_format_does_not_break_pipeline() -> Result<()> {
    use serde_json::json;

    let env = TestEnvironment::new("relative_directory_format")?;
    let temp_dir = env.test_dir().to_str().unwrap().to_string();

    let event = json!({
        "pid": 9001,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "main.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });
    env.create_source_files(&[("events.json", &event.to_string()), ("main.c", "int main() { return 0; }")])?;

    let config = r#"
schema: "4.1"

format:
  paths:
    directory: relative
    file: relative
"#;
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    env.run_bear_success(&[
        "--config",
        config_path.to_str().unwrap(),
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    let entry = db.entries().first().expect("expected at least one entry");
    let directory = entry.get("directory").and_then(|v| v.as_str()).unwrap_or("");
    assert!(!directory.is_empty(), "directory field must not be empty; got {directory:?}");

    Ok(())
}

/// Companion to `relative_directory_format_does_not_break_pipeline`: with
/// only `directory: relative` (and `file: as-is`) the output pipeline
/// succeeds too. Isolates the regression guard to the `directory` axis --
/// when issue #692 was open, this config reproduced the bug independently
/// of the `file` setting.
// Requirements: output-path-format
#[test]
#[cfg(target_family = "unix")]
fn relative_directory_alone_does_not_break_pipeline() -> Result<()> {
    use serde_json::json;

    let env = TestEnvironment::new("relative_directory_alone")?;
    let temp_dir = env.test_dir().to_str().unwrap().to_string();

    let event = json!({
        "pid": 9101,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "main.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });
    env.create_source_files(&[("events.json", &event.to_string()), ("main.c", "int main() { return 0; }")])?;

    let config = r#"
schema: "4.1"

format:
  paths:
    directory: relative
    file: as-is
"#;
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    env.run_bear_success(&[
        "--config",
        config_path.to_str().unwrap(),
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    let entry = db.entries().first().expect("expected at least one entry");
    let directory = entry.get("directory").and_then(|v| v.as_str()).unwrap_or("");
    assert!(!directory.is_empty(), "directory field must not be empty; got {directory:?}");

    Ok(())
}

/// Test invalid configuration handling
/// Verifies Bear handles invalid config gracefully
#[test]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn invalid_config_handling() -> Result<()> {
    let env = TestEnvironment::new("invalid_config")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Create invalid YAML config
    let invalid_config = "{ invalid yaml content }";
    let config_path = env.test_dir().join("invalid_config.yaml");
    std::fs::write(&config_path, invalid_config)?;

    let build_commands = format!("{} -c test.c", COMPILER_C_PATH);
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Bear should handle invalid config gracefully (likely with error)
    let output = env.run_bear(&[
        "--config",
        config_path.to_str().unwrap(),
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Should fail with non-zero exit code
    assert!(output.exit_code() != Some(0));

    Ok(())
}

/// Test unsupported schema version
/// Verifies Bear rejects unsupported schema versions
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn unsupported_schema_version() -> Result<()> {
    let env = TestEnvironment::new("unsupported_schema")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Create config with unsupported schema version
    let config = format!(
        r#"
schema: "3.0"

intercept:
  mode: preload
  path: "{}"

sources:
  only_existing_files: true
"#,
        PRELOAD_LIBRARY_PATH
    );

    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    let build_commands = format!("{} -c test.c", COMPILER_C_PATH);
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Bear should reject unsupported schema version
    let output = env.run_bear(&[
        "--config",
        config_path.to_str().unwrap(),
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Should fail with non-zero exit code and mention schema
    assert!(output.exit_code() != Some(0));

    // Error message should mention schema version issue
    assert!(output.stderr().contains("schema"));

    Ok(())
}

/// Test duplicate filter configuration
/// Verifies duplicate filtering options work
// Requirements: output-duplicate-detection
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn duplicate_filter_config() -> Result<()> {
    let env = TestEnvironment::new("duplicate_filter")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Build script that might generate duplicate entries
    let build_commands = format!("{} -c test.c\n{} -c test.c", COMPILER_C_PATH, COMPILER_C_PATH);
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Config with duplicate filtering
    let config = format!(
        r#"
schema: "4.1"

intercept:
  mode: preload
  path: "{}"

duplicates:
  match_on: ["file", "directory"]

sources:
  only_existing_files: true
"#,
        PRELOAD_LIBRARY_PATH
    );

    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    let _output = env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--config",
        config_path.to_str().unwrap(),
        "--",
        "sh",
        script_path.to_str().unwrap(),
    ])?;

    // Verify duplicate filtering worked
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    Ok(())
}

/// With `match_on: [file]`, two entries for the same file with different flags
/// collapse to one (the first).
// Requirements: output-duplicate-detection
#[test]
#[cfg(target_family = "unix")]
fn duplicate_match_on_file_alone_collapses_flag_variants() -> Result<()> {
    use serde_json::json;

    let env = TestEnvironment::new("dup_match_file")?;
    let temp_dir = env.test_dir().to_str().unwrap().to_string();

    let event1 = json!({
        "pid": 1001,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "-O2", "test.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });
    let event2 = json!({
        "pid": 1002,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "-O3", "test.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });
    let events_content = format!("{}\n{}", event1, event2);
    env.create_source_files(&[("events.json", &events_content), ("test.c", "int main() { return 0; }")])?;

    let config = r#"
schema: "4.1"

duplicates:
  match_on: [file]
"#;
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    env.run_bear_success(&[
        "--config",
        config_path.to_str().unwrap(),
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    // First-occurrence wins: -O2 is kept, -O3 is dropped.
    let entry = db.entries().first().expect("one entry expected");
    let args: Vec<String> = entry
        .get("arguments")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .map(String::from)
        .collect();
    assert!(args.iter().any(|a| a == "-O2"), "first-occurrence entry must carry -O2, got {:?}", args);
    assert!(!args.iter().any(|a| a == "-O3"), "second-occurrence -O3 entry must be dropped, got {:?}", args);

    Ok(())
}

/// With `match_on: [file, output]`, the same source compiled to different
/// output paths yields two entries (different output means not a duplicate).
// Requirements: output-duplicate-detection
#[test]
#[cfg(target_family = "unix")]
fn duplicate_match_on_file_and_output_preserves_differing_outputs() -> Result<()> {
    use serde_json::json;

    let env = TestEnvironment::new("dup_match_file_output")?;
    let temp_dir = env.test_dir().to_str().unwrap().to_string();

    let event1 = json!({
        "pid": 2001,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "test.c", "-o", "debug/test.o"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });
    let event2 = json!({
        "pid": 2002,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "test.c", "-o", "release/test.o"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });
    let events_content = format!("{}\n{}", event1, event2);
    env.create_source_files(&[("events.json", &events_content), ("test.c", "int main() { return 0; }")])?;

    let config = r#"
schema: "4.1"

duplicates:
  match_on: [file, output]

format:
  entries:
    use_array_format: true
    include_output_field: true
"#;
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    env.run_bear_success(&[
        "--config",
        config_path.to_str().unwrap(),
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(2)?;

    Ok(())
}

/// Configuration validation must reject `match_on` that contains both
/// `command` and `arguments` (they are alternative representations of the
/// same data).
// Requirements: output-duplicate-detection
#[test]
fn duplicate_match_on_command_and_arguments_is_rejected() -> Result<()> {
    let env = TestEnvironment::new("dup_match_conflict")?;

    let config = r#"
schema: "4.1"

duplicates:
  match_on: [command, arguments]
"#;
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    let output = env.run_bear(&[
        "--config",
        config_path.to_str().unwrap(),
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;
    assert!(output.exit_code() != Some(0), "config with conflicting match_on must be rejected");

    Ok(())
}

/// Configuration validation must reject an empty `match_on` list.
// Requirements: output-duplicate-detection
#[test]
fn duplicate_match_on_empty_is_rejected() -> Result<()> {
    let env = TestEnvironment::new("dup_match_empty")?;

    let config = r#"
schema: "4.1"

duplicates:
  match_on: []
"#;
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    let output = env.run_bear(&[
        "--config",
        config_path.to_str().unwrap(),
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;
    assert!(output.exit_code() != Some(0), "config with empty match_on must be rejected");

    Ok(())
}

/// In append mode, the original entry from the existing compilation database
/// wins over a new entry that matches on the configured fields: the original
/// arguments survive, the new ones are dropped.
// Requirements: output-duplicate-detection, output-append
#[test]
#[cfg(target_family = "unix")]
fn duplicate_append_mode_preserves_original_entry() -> Result<()> {
    use serde_json::json;

    let env = TestEnvironment::new("dup_append_priority")?;
    let temp_dir = env.test_dir().to_str().unwrap().to_string();

    // First run builds the existing database with -O2.
    let first_event = json!({
        "pid": 3001,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "-O2", "test.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });
    env.create_source_files(&[
        ("events1.json", &first_event.to_string()),
        ("test.c", "int main() { return 0; }"),
    ])?;

    // match_on deliberately excludes arguments: same file+directory = duplicate.
    let config = r#"
schema: "4.1"

duplicates:
  match_on: [file, directory]
"#;
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    env.run_bear_success(&[
        "--config",
        config_path.to_str().unwrap(),
        "semantic",
        "--input",
        "events1.json",
        "--output",
        "compile_commands.json",
    ])?;

    // Second run tries to add a new entry for the same file with -O3. The
    // append-mode guarantee: the original -O2 entry wins.
    let second_event = json!({
        "pid": 3002,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "-O3", "test.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });
    std::fs::write(env.test_dir().join("events2.json"), second_event.to_string())?;

    env.run_bear_success(&[
        "--config",
        config_path.to_str().unwrap(),
        "semantic",
        "--append",
        "--input",
        "events2.json",
        "--output",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    let args: Vec<String> = db
        .entries()
        .first()
        .and_then(|e| e.get("arguments"))
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .map(String::from)
        .collect();
    assert!(args.iter().any(|a| a == "-O2"), "original -O2 entry must survive, got {:?}", args);
    assert!(!args.iter().any(|a| a == "-O3"), "new -O3 entry must be dropped, got {:?}", args);

    Ok(())
}
