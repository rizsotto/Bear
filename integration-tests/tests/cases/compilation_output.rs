// SPDX-License-Identifier: GPL-3.0-or-later

//! Compilation database output tests for Bear integration
//!
//! These tests verify that Bear generates correct compilation databases
//! for various build scenarios, ported from the Python/lit test suite.

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::{
    CompilationEntryMatcher, TestEnvironment, compilation_entry, filename_of,
};
use anyhow::Result;
#[cfg(target_family = "unix")]
use serde_json::Value;

/// Test compilation with build script that calls compiler
/// This generates events that the semantic analyzer can process
// Requirements: output-json-compilation-database, output-compilation-entries, output-atomic-write, output-path-format
#[test]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn simple_single_file_compilation() -> Result<()> {
    let env = TestEnvironment::new("simple_single_file_compilation")?;

    // Create a simple source file
    env.create_source_files(&[("simple.c", "int main() { return 0; }")])?;

    // Create a shell script that calls the compiler
    let build_commands = format!("{} -c simple.c -o simple.o", filename_of(COMPILER_C_PATH));
    let build_script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Step 1: Run intercept command to capture events from the build script
    let result = env.run_bear(&[
        "intercept",
        "-o",
        "events.json",
        "--",
        SHELL_PATH,
        build_script_path.to_str().unwrap(),
    ])?;
    result.assert_success()?;

    // Check if events file was created
    assert!(env.file_exists("events.json"));

    // Step 2: Run semantic command to convert events to compilation database
    let result = env.run_bear(&["semantic", "-i", "events.json", "-o", "compile_commands.json"])?;
    result.assert_success()?;

    // Verify compilation database was created
    assert!(env.file_exists("compile_commands.json"));

    // Load and verify the compilation database
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify the entry contains expected information
    db.assert_contains(&compilation_entry!(
        file: "simple.c".to_string(),
        directory: env.test_dir().to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "simple.c".to_string(),
            "-o".to_string(),
            "simple.o".to_string(),
        ]
    ))?;

    Ok(())
}

/// Test successful build with multiple sources (C and C++)
/// Verifies Bear handles mixed compilation units
// Requirements: output-json-compilation-database, output-compilation-entries
#[test]
#[cfg(all(has_executable_compiler_c, has_executable_compiler_cxx, has_executable_shell))]
fn successful_build_multiple_sources() -> Result<()> {
    let env = TestEnvironment::new("successful_build_multiple_sources")?;

    // Create multiple source files
    env.create_source_files(&[
        ("test1.c", "int func1() { return 1; }"),
        ("test2.c", "int func2() { return 2; }"),
        ("test3.cpp", "extern \"C\" int func3() { return 3; }"),
        ("test4.cpp", "extern \"C\" int func4() { return 4; }"),
    ])?;

    // Create build script that compiles all files
    let build_commands = [
        format!("{} -c -o test1.o test1.c", filename_of(COMPILER_C_PATH)),
        format!("{} -c -o test2.o test2.c", filename_of(COMPILER_C_PATH)),
        format!("{} -c -o test3.o test3.cpp", filename_of(COMPILER_CXX_PATH)),
        format!("{} -c -o test4.o test4.cpp", filename_of(COMPILER_CXX_PATH)),
    ]
    .join("\n");
    let build_script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Run bear
    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        build_script_path.to_str().unwrap(),
    ])?;

    // Verify compilation database
    assert!(env.file_exists("compile_commands.json"));
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(4)?;

    // Verify entries for each source file
    let temp_dir = env.test_dir().to_str().unwrap();

    db.assert_contains(&compilation_entry!(
        file: "test1.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "-o".to_string(),
            "test1.o".to_string(),
            "test1.c".to_string(),
        ]
    ))?;

    db.assert_contains(&compilation_entry!(
        file: "test2.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "-o".to_string(),
            "test2.o".to_string(),
            "test2.c".to_string(),
        ]
    ))?;

    db.assert_contains(&compilation_entry!(
        file: "test3.cpp".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            COMPILER_CXX_PATH.to_string(),
            "-c".to_string(),
            "-o".to_string(),
            "test3.o".to_string(),
            "test3.cpp".to_string(),
        ]
    ))?;

    db.assert_contains(&compilation_entry!(
        file: "test4.cpp".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            COMPILER_CXX_PATH.to_string(),
            "-c".to_string(),
            "-o".to_string(),
            "test4.o".to_string(),
            "test4.cpp".to_string(),
        ]
    ))?;

    Ok(())
}

/// Test output is overwritten when no append flag
// Requirements: output-append
#[test]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn without_append_output_is_overwritten() -> Result<()> {
    let env = TestEnvironment::new("without_append_output_is_overwritten")?;

    // Create multiple source files
    env.create_source_files(&[
        ("test1.c", "int func1() { return 1; }"),
        ("test2.c", "int func2() { return 2; }"),
    ])?;

    // Create build script that compiles all files
    let build_command1 = format!("{} -c -o test1.o test1.c", filename_of(COMPILER_C_PATH));
    let build_script1_path = env.create_shell_script("build1.sh", &build_command1)?;

    let build_command2 = format!("{} -c -o test2.o test2.c", filename_of(COMPILER_C_PATH));
    let build_script2_path = env.create_shell_script("build2.sh", &build_command2)?;

    // Run bear once
    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        build_script1_path.to_str().unwrap(),
    ])?;

    // Verify compilation database
    assert!(env.file_exists("compile_commands.json"));
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Run bear again with append
    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        build_script2_path.to_str().unwrap(),
    ])?;

    // Verify compilation database
    assert!(env.file_exists("compile_commands.json"));
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    Ok(())
}

/// Test append functionality
// Requirements: output-append
#[test]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn append_works_as_expected() -> Result<()> {
    let env = TestEnvironment::new("append_works_as_expected")?;

    // Create multiple source files
    env.create_source_files(&[
        ("test1.c", "int func1() { return 1; }"),
        ("test2.c", "int func2() { return 2; }"),
    ])?;

    // Create build script that compiles all files
    let build_command1 = format!("{} -c -o test1.o test1.c", filename_of(COMPILER_C_PATH));
    let build_script1_path = env.create_shell_script("build1.sh", &build_command1)?;

    let build_command2 = format!("{} -c -o test2.o test2.c", filename_of(COMPILER_C_PATH));
    let build_script2_path = env.create_shell_script("build2.sh", &build_command2)?;

    // Run bear once
    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        build_script1_path.to_str().unwrap(),
    ])?;

    // Verify compilation database
    assert!(env.file_exists("compile_commands.json"));
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Run bear again with append
    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--append",
        "--",
        SHELL_PATH,
        build_script2_path.to_str().unwrap(),
    ])?;

    // Verify compilation database
    assert!(env.file_exists("compile_commands.json"));
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(2)?;

    Ok(())
}

/// Test build with compilation failures - should still generate partial database
/// Verifies Bear can handle partial build failures
// Requirements: output-json-compilation-database
#[test]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn broken_build_partial_success() -> Result<()> {
    let env = TestEnvironment::new("broken_build_partial_success")?;

    // Create one valid and one invalid source file
    env.create_source_files(&[
        ("valid.c", "int main() { return 0; }"),
        ("invalid.c", "this is not valid C code #@!%"),
    ])?;

    // Create build script that tries to compile both (one will fail)
    let build_commands = [
        format!("{} -c -o valid.o valid.c", filename_of(COMPILER_C_PATH)),
        format!("{} -c -o invalid.o invalid.c", filename_of(COMPILER_C_PATH)),
    ]
    .join("\n");
    let build_script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Run bear - should fail due to compilation error but generate partial DB
    let result = env.run_bear(&[
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        build_script_path.to_str().unwrap(),
    ])?;
    result.assert_failure()?; // Build should fail

    // Compilation database should still be created with successful entries
    assert!(env.file_exists("compile_commands.json"));
    let db = env.load_compilation_database("compile_commands.json")?;

    // Should have entries for both attempted compilations
    // NOTE: commented out because of compiler wrappers produces extra entries for
    //  the failed one. probably re-running the command with `-fdiagnostics-color`.
    // db.assert_count(2)?;

    // Should contain entry for valid compilation
    db.assert_contains(&compilation_entry!(
        file: "valid.c".to_string(),
        directory: env.test_dir().to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "-o".to_string(),
            "valid.o".to_string(),
            "valid.c".to_string(),
        ]
    ))?;

    // Should also contain entry for failed compilation attempt
    db.assert_contains(&compilation_entry!(
        file: "invalid.c".to_string(),
        directory: env.test_dir().to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "-o".to_string(),
            "invalid.o".to_string(),
            "invalid.c".to_string(),
        ]
    ))?;

    Ok(())
}

/// Test empty build - should generate empty compilation database
/// Verifies Bear handles builds with no compilation commands
// Requirements: output-json-compilation-database
#[test]
#[cfg(all(has_executable_true, has_executable_shell, has_executable_echo))]
fn empty_build_generates_empty_database() -> Result<()> {
    let env = TestEnvironment::new("empty_build_generates_empty_database")?;

    // Create shell script that doesn't compile anything
    let build_commands = format!("\"{}\" && \"{}\" 'Build completed'", TRUE_PATH, ECHO_PATH);
    let build_script_path = env.create_shell_script("build.sh", &build_commands)?;

    let result = env.run_bear(&[
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        build_script_path.to_str().unwrap(),
    ])?;
    result.assert_success()?;

    // Should create empty compilation database
    assert!(env.file_exists("compile_commands.json"));
    let content = env.read_file("compile_commands.json")?;
    assert_eq!(content.trim(), "[]");

    Ok(())
}

/// Test compilation with multiple source files using single command
/// Verifies Bear handles batch compilation commands
// Requirements: output-json-compilation-database, output-compilation-entries
#[test]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn multiple_sources_single_command() -> Result<()> {
    let env = TestEnvironment::new("multiple_sources_single_command")?;

    // Create multiple source files
    env.create_source_files(&[
        ("src1.c", "int func1() { return 1; }"),
        ("src2.c", "int func2() { return 2; }"),
        ("src3.c", "int func3() { return 3; }"),
    ])?;

    // Create build script with single command compiling multiple files
    let build_commands = format!("{} -c src1.c src2.c src3.c", filename_of(COMPILER_C_PATH));
    let build_script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Run bear with build script
    let result = env.run_bear(&[
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        build_script_path.to_str().unwrap(),
    ])?;
    result.assert_success()?;

    // Verify compilation database was created
    assert!(env.file_exists("compile_commands.json"));
    let db = env.load_compilation_database("compile_commands.json")?;

    // Verify entries exist for each source file
    // NOTE: exact count not asserted because ccache may split multi-file
    // commands and produce additional entries via the underlying compiler.
    db.assert_contains(&compilation_entry!(
        file: "src1.c".to_string(),
        directory: env.test_dir().to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "src1.c".to_string(),
        ]
    ))?;
    db.assert_contains(&compilation_entry!(
        file: "src2.c".to_string(),
        directory: env.test_dir().to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "src2.c".to_string(),
        ]
    ))?;
    db.assert_contains(&compilation_entry!(
        file: "src3.c".to_string(),
        directory: env.test_dir().to_str().unwrap().to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "src3.c".to_string(),
        ]
    ))?;

    Ok(())
}

/// Helper to extract the arguments array from a compilation database entry.
#[cfg(target_family = "unix")]
fn get_arguments(entry: &Value) -> Vec<String> {
    entry
        .get("arguments")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .map(String::from)
        .collect()
}

/// Verifies that CPATH environment variable survives interception and appears
/// as -I flags in the compilation database.
///
/// This exercises the full pipeline: shell sets CPATH, compiler is intercepted,
/// environment is trimmed (must keep CPATH), event is sent over TCP, semantic
/// analyzer converts CPATH to -I flags, and the compilation database is written.
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn env_cpath_produces_include_flags() -> Result<()> {
    let env = TestEnvironment::new("env_cpath_produces_include_flags")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let build_commands = format!(
        "export CPATH=/test/include_a:/test/include_b\n{} -c test.c -o test.o",
        filename_of(COMPILER_C_PATH)
    );
    let build_script = env.create_shell_script("build.sh", &build_commands)?;

    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        build_script.to_str().unwrap(),
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;

    // Find the entry for test.c (ccache may produce extra entries)
    let entry = db
        .entries()
        .iter()
        .find(|e| e.get("file").and_then(Value::as_str) == Some("test.c"))
        .expect("Expected a compilation entry for test.c");

    let args = get_arguments(entry);
    assert!(
        args.windows(2).any(|w| w[0] == "-I" && w[1] == "/test/include_a"),
        "Expected '-I /test/include_a' from CPATH in: {:?}",
        args
    );
    assert!(
        args.windows(2).any(|w| w[0] == "-I" && w[1] == "/test/include_b"),
        "Expected '-I /test/include_b' from CPATH in: {:?}",
        args
    );

    Ok(())
}

/// Verifies that C_INCLUDE_PATH environment variable survives interception
/// and appears as -isystem flags in the compilation database.
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn env_c_include_path_produces_isystem_flags() -> Result<()> {
    let env = TestEnvironment::new("env_c_include_path_produces_isystem_flags")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let build_commands = format!(
        "export C_INCLUDE_PATH=/test/sys_include\n{} -c test.c -o test.o",
        filename_of(COMPILER_C_PATH)
    );
    let build_script = env.create_shell_script("build.sh", &build_commands)?;

    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        build_script.to_str().unwrap(),
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;

    let entry = db
        .entries()
        .iter()
        .find(|e| e.get("file").and_then(Value::as_str) == Some("test.c"))
        .expect("Expected a compilation entry for test.c");

    let args = get_arguments(entry);
    assert!(
        args.windows(2).any(|w| w[0] == "-isystem" && w[1] == "/test/sys_include"),
        "Expected '-isystem /test/sys_include' from C_INCLUDE_PATH in: {:?}",
        args
    );

    Ok(())
}

/// Given cc -o a.out src1.c src2.c src3.c (compile-and-link in one invocation),
/// one compile entry is produced per source and no entry describes the link
/// output a.out.
// Requirements: output-compilation-entries
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn compile_and_link_split_produces_compile_entries() -> Result<()> {
    let env = TestEnvironment::new("compile_and_link_split")?;

    env.create_source_files(&[
        ("src1.c", "int f1(void) { return 1; }"),
        ("src2.c", "int f2(void) { return 2; }"),
        ("src3.c", "int main(void) { return 0; }"),
    ])?;

    let build = format!("{} -o a.out src1.c src2.c src3.c", filename_of(COMPILER_C_PATH));
    let script = env.create_shell_script("build.sh", &build)?;

    env.run_bear(&["--output", "compile_commands.json", "--", SHELL_PATH, script.to_str().unwrap()])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_contains(&CompilationEntryMatcher::new().file("src1.c"))?;
    db.assert_contains(&CompilationEntryMatcher::new().file("src2.c"))?;
    db.assert_contains(&CompilationEntryMatcher::new().file("src3.c"))?;

    let has_linker_output =
        db.entries().iter().any(|e| e.get("file").and_then(Value::as_str) == Some("a.out"));
    assert!(!has_linker_output, "link output a.out must not appear as a source entry");

    Ok(())
}

/// Given cc -o a.out src.o (pure link of a pre-built object file), no entry
/// is produced for this invocation.
// Requirements: output-compilation-entries
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn pure_link_invocation_produces_no_entries() -> Result<()> {
    let env = TestEnvironment::new("pure_link_no_entries")?;

    env.create_source_files(&[("src.c", "int main(void) { return 0; }")])?;

    // Pre-build src.o outside of Bear so only the link step is captured.
    let prep = format!("{} -c src.c -o src.o", filename_of(COMPILER_C_PATH));
    let prep_script = env.create_shell_script("prep.sh", &prep)?;
    let status = std::process::Command::new(SHELL_PATH)
        .arg(prep_script.to_str().unwrap())
        .current_dir(env.test_dir())
        .status()?;
    assert!(status.success(), "setup compile step failed");

    // Now capture only the pure-link invocation.
    let link = format!("{} -o a.out src.o", filename_of(COMPILER_C_PATH));
    let link_script = env.create_shell_script("link.sh", &link)?;

    env.run_bear(&["--output", "compile_commands.json", "--", SHELL_PATH, link_script.to_str().unwrap()])?;

    let content = env.read_file("compile_commands.json")?;
    assert_eq!(content.trim(), "[]", "pure-link invocation must produce zero entries");

    Ok(())
}

/// Given cc -o a.out -lm -O2 src.c, the resulting compile entry preserves -O2
/// but drops the link-only -lm flag.
// Requirements: output-compilation-entries
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn link_only_flags_are_stripped_from_entries() -> Result<()> {
    let env = TestEnvironment::new("link_only_flags_stripped")?;

    env.create_source_files(&[("src.c", "int main(void) { return 0; }")])?;

    let build = format!("{} -o a.out -lm -O2 src.c", filename_of(COMPILER_C_PATH));
    let script = env.create_shell_script("build.sh", &build)?;

    env.run_bear(&["--output", "compile_commands.json", "--", SHELL_PATH, script.to_str().unwrap()])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    let entry = db
        .entries()
        .iter()
        .find(|e| e.get("file").and_then(Value::as_str) == Some("src.c"))
        .expect("expected a compile entry for src.c");
    let args = get_arguments(entry);

    assert!(args.iter().any(|a| a == "-O2"), "compile flag -O2 must be preserved, got {:?}", args);
    assert!(!args.iter().any(|a| a == "-lm"), "link-only flag -lm must be stripped, got {:?}", args);

    Ok(())
}

/// Given cc -I first -I second -DFOO -DBAR -c src.c, the entry preserves the
/// original relative order of flags (consumers depend on it for include search
/// order and macro overrides).
// Requirements: output-compilation-entries
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn argument_order_is_preserved_in_entries() -> Result<()> {
    let env = TestEnvironment::new("argument_order_preserved")?;

    env.create_source_files(&[("src.c", "int main(void) { return 0; }")])?;
    std::fs::create_dir_all(env.test_dir().join("first"))?;
    std::fs::create_dir_all(env.test_dir().join("second"))?;

    let build = format!("{} -I first -I second -DFOO -DBAR -c src.c", filename_of(COMPILER_C_PATH));
    let script = env.create_shell_script("build.sh", &build)?;

    env.run_bear(&["--output", "compile_commands.json", "--", SHELL_PATH, script.to_str().unwrap()])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    let entry = db
        .entries()
        .iter()
        .find(|e| e.get("file").and_then(Value::as_str) == Some("src.c"))
        .expect("expected a compile entry for src.c");
    let args = get_arguments(entry);

    let pos = |needle: &str| args.iter().position(|a| a == needle);
    let first = pos("first").expect("missing include path 'first'");
    let second = pos("second").expect("missing include path 'second'");
    let foo = pos("-DFOO").expect("missing -DFOO");
    let bar = pos("-DBAR").expect("missing -DBAR");

    assert!(first < second, "'-I first' must precede '-I second', got {:?}", args);
    assert!(foo < bar, "'-DFOO' must precede '-DBAR', got {:?}", args);

    Ok(())
}

/// Given cc --version (info-only invocation), no entry is produced.
// Requirements: output-compilation-entries
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn info_only_invocation_produces_no_entries() -> Result<()> {
    let env = TestEnvironment::new("info_only_no_entries")?;

    let build = format!("{} --version", filename_of(COMPILER_C_PATH));
    let script = env.create_shell_script("build.sh", &build)?;

    env.run_bear(&["--output", "compile_commands.json", "--", SHELL_PATH, script.to_str().unwrap()])?;

    let content = env.read_file("compile_commands.json")?;
    assert_eq!(content.trim(), "[]", "info-only invocation must produce zero entries");

    Ok(())
}

/// Given cc -o a.out src1.c src2.c with the output field enabled via
/// configuration, every resulting entry records output = a.out (the known
/// limitation: the single -o value is copied verbatim, not inferred per
/// source).
// Requirements: output-compilation-entries
#[test]
#[cfg(target_family = "unix")]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn output_field_is_recorded_when_enabled() -> Result<()> {
    let env = TestEnvironment::new("output_field_enabled")?;

    env.create_source_files(&[
        ("src1.c", "int f1(void) { return 1; }"),
        ("src2.c", "int main(void) { return 0; }"),
    ])?;

    let build = format!("{} -o a.out src1.c src2.c", filename_of(COMPILER_C_PATH));
    let script = env.create_shell_script("build.sh", &build)?;

    let config = format!(
        r#"
schema: "4.1"

intercept:
  mode: preload
  path: "{preload}"

format:
  paths:
    directory: as-is
    file: as-is
  entries:
    use_array_format: true
    include_output_field: true
"#,
        preload = PRELOAD_LIBRARY_PATH
    );
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    env.run_bear(&[
        "--output",
        "compile_commands.json",
        "--config",
        config_path.to_str().unwrap(),
        "--",
        SHELL_PATH,
        script.to_str().unwrap(),
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;

    // Every entry from this invocation must record output = a.out.
    let compile_entries: Vec<_> = db
        .entries()
        .iter()
        .filter(|e| {
            let file = e.get("file").and_then(Value::as_str).unwrap_or("");
            file == "src1.c" || file == "src2.c"
        })
        .collect();
    assert!(!compile_entries.is_empty(), "expected compile entries for src1.c / src2.c");
    for entry in compile_entries {
        let output = entry.get("output").and_then(Value::as_str);
        assert_eq!(
            output,
            Some("a.out"),
            "entry {:?} must have output = a.out, got {:?}",
            entry.get("file"),
            output
        );
    }

    Ok(())
}
