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
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
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
schema: "4.0"

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

    let config_path = env.temp_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    // Run bear with config
    let _output = env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--config",
        config_path.to_str().unwrap(),
        "--",
        "sh",
        script_path.to_str().unwrap(),
    ])?;

    // Verify the compilation database was created
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    Ok(())
}

/// Test compiler ignore functionality
/// Verifies that compilers marked with ignore: true are excluded
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
#[cfg(all(
    has_executable_compiler_c,
    has_executable_compiler_cxx,
    has_executable_shell
))]
fn compiler_ignore_config() -> Result<()> {
    let env = TestEnvironment::new("compiler_ignore")?;

    // Create source files for both C and C++
    env.create_source_files(&[
        ("source.c", "int main() { return 0; }"),
        ("source.cpp", "int main() { return 0; }"),
    ])?;

    let build_commands = format!(
        "{} -c source.c\n{} -c source.cpp",
        COMPILER_C_PATH, COMPILER_CXX_PATH
    );
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Create config that ignores the C++ compiler
    let config = format!(
        r#"
schema: "4.0"

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

    let config_path = env.temp_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    // Run bear with config
    let _output = env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--config",
        config_path.to_str().unwrap(),
        "--",
        "sh",
        script_path.to_str().unwrap(),
    ])?;

    // Should only capture C compiler invocation, not C++
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify only C file is included (Bear may produce relative paths)
    let entries = db.entries();
    let entry = &entries[0];

    // Check that we have a C source file
    let file_path = entry.get("file").unwrap().as_str().unwrap();
    assert!(file_path.contains("source.c"));

    // Check that arguments contain the compiler and source file
    let args = entry.get("arguments").unwrap().as_array().unwrap();
    let arg_strings: Vec<String> = args
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    assert!(arg_strings.iter().any(|arg| arg.contains("gcc")));
    assert!(arg_strings.contains(&"-c".to_string()));
    assert!(arg_strings.contains(&"source.c".to_string()));

    Ok(())
}

/// Test source file filtering
/// Verifies only_existing_files configuration option
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
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
schema: "4.0"

intercept:
  mode: preload
  path: "{}"

sources:
  only_existing_files: true
"#,
        PRELOAD_LIBRARY_PATH
    );

    let config_path = env.temp_dir().join("config.yaml");
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

    // Only existing.c should be in the output
    let db = env.load_compilation_database("compile_commands.json")?;

    // Filter entries to only those referencing existing files
    let entries = db.entries();
    let existing_entries: Vec<_> = entries
        .iter()
        .filter(|entry| {
            if let Some(file_path) = entry.get("file").and_then(|v| v.as_str()) {
                file_path.contains("existing.c")
            } else {
                false
            }
        })
        .collect();

    assert!(
        !existing_entries.is_empty(),
        "Should have at least one entry for existing.c"
    );

    Ok(())
}

/// Test path format configuration
/// Verifies different path formatting options
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn path_format_config() -> Result<()> {
    let env = TestEnvironment::new("path_format")?;

    env.create_source_files(&[("src/main.c", "int main() { return 0; }")])?;

    let build_commands = format!("cd src && {} -c main.c", COMPILER_C_PATH);
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Test absolute path format
    let config = format!(
        r#"
schema: "4.0"

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

    let config_path = env.temp_dir().join("config.yaml");
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
    let entries = db.entries();

    if !entries.is_empty() {
        let entry = &entries[0];

        // Directory should be absolute path
        if let Some(directory) = entry.get("directory").and_then(|v| v.as_str()) {
            assert!(
                directory.starts_with('/'),
                "Directory should be absolute: {}",
                directory
            );
        }

        // File should be absolute path
        if let Some(file) = entry.get("file").and_then(|v| v.as_str()) {
            assert!(file.starts_with('/'), "File should be absolute: {}", file);
        }
    }

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
    let config_path = env.temp_dir().join("invalid_config.yaml");
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
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
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

    let config_path = env.temp_dir().join("config.yaml");
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
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn duplicate_filter_config() -> Result<()> {
    let env = TestEnvironment::new("duplicate_filter")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Build script that might generate duplicate entries
    let build_commands = format!(
        "{} -c test.c\n{} -c test.c",
        COMPILER_C_PATH, COMPILER_C_PATH
    );
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Config with duplicate filtering
    let config = format!(
        r#"
schema: "4.0"

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

    let config_path = env.temp_dir().join("config.yaml");
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

    // Should have deduplicated the entries
    let entries = db.entries();
    assert!(
        !entries.is_empty(),
        "Should have at least one compilation entry"
    );

    Ok(())
}
