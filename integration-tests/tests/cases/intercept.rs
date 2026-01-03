// SPDX-License-Identifier: GPL-3.0-or-later

//! Intercept functionality tests for Bear integration
//!
//! These tests verify that Bear's command interception works correctly
//! across different scenarios, ported from the Python/lit test suite.

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::*;
use anyhow::Result;
use serde_json::{self, Value};

/// Test basic command interception with preload mechanism
#[test]
#[cfg(target_family = "unix")]
#[cfg(has_executable_compiler_c)]
fn basic_command_interception() -> Result<()> {
    let env = TestEnvironment::new("basic_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Run intercept mode to capture commands
    env.run_bear_success(&["intercept", "--output", "events.json", "--", COMPILER_C_PATH, "-c", "test.c"])?;

    // Load and verify events using the new abstraction
    let events = env.load_events_file("events.json")?;

    // Should have at least one event
    assert!(!events.events().is_empty());

    // Should contain compiler execution event
    let compiler_matcher = event_matcher!(executable_path: COMPILER_C_PATH.to_string());
    events.assert_contains(&compiler_matcher)?;

    Ok(())
}

/// Test shell command interception
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_echo, has_executable_shell))]
fn shell_command_interception() -> Result<()> {
    let env = TestEnvironment::new("shell_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Create shell script that runs compiler
    let build_commands = [
        format!("{} \"Starting build...\"", ECHO_PATH),
        format!("{} -c test.c -o test.o", COMPILER_C_PATH),
        format!("{} \"Build complete\"", ECHO_PATH),
    ]
    .join("\n");

    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Intercept shell script execution
    env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Load and verify events using the new abstraction
    let events = env.load_events_file("events.json")?;

    // Should have captured events
    assert!(!events.events().is_empty());

    // Should capture shell execution
    let shell_matcher = event_matcher!(executable_name: "sh".to_string());
    events.assert_contains(&shell_matcher)?;

    // Should capture compiler execution
    let compiler_matcher = event_matcher!(executable_path: COMPILER_C_PATH.to_string());
    events.assert_contains(&compiler_matcher)?;

    Ok(())
}

/// Test shell commands without shebang
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_echo, has_executable_shell))]
fn shell_commands_without_shebang() -> Result<()> {
    let env = TestEnvironment::new("shell_no_shebang")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Create shell script WITHOUT shebang
    let shell_script = [
        format!("{} \"Building without shebang...\"", ECHO_PATH),
        format!("{} -c test.c", COMPILER_C_PATH),
        format!("{} \"Done\"", ECHO_PATH),
    ]
    .join("\n");
    let script_path = env.create_build_script("build_no_shebang.sh", &shell_script)?;

    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Should still capture commands even without shebang
    let events_content = std::fs::read_to_string(env.temp_dir().join("events.json"))?;
    let events: Vec<Value> =
        events_content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();

    assert!(!events.is_empty());

    Ok(())
}

/// Test parallel command interception
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn parallel_command_interception() -> Result<()> {
    let env = TestEnvironment::new("parallel_intercept")?;

    // Create multiple source files
    for i in 1..=4 {
        env.create_source_files(&[(
            &format!("test_{}.c", i),
            &format!("int func_{}() {{ return {}; }}", i, i),
        )])?;
    }

    // Create parallel build script
    let build_commands = [
        format!("{} -c test_1.c &", filename_of(COMPILER_C_PATH)),
        format!("{} -c test_2.c &", filename_of(COMPILER_C_PATH)),
        format!("{} -c test_3.c &", filename_of(COMPILER_C_PATH)),
        format!("{} -c test_4.c &", filename_of(COMPILER_C_PATH)),
        format!("wait"),
    ]
    .join("\n");
    let script_path = env.create_shell_script("parallel_build.sh", &build_commands)?;

    env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Load and verify events using the new abstraction
    let events = env.load_events_file("events.json")?;

    // Should have captured all 4 parallel compiler invocations
    let compiler_matcher = event_matcher!(executable_name: filename_of(COMPILER_C_PATH));
    events.assert_count_matching(&compiler_matcher, 4)?;

    // Verify each individual compiler invocation was captured
    for i in 1..=4 {
        let specific_compiler_matcher = EventMatcher::new().arguments(vec![
            filename_of(COMPILER_C_PATH).to_string(),
            "-c".to_string(),
            format!("test_{}.c", i),
        ]);
        events.assert_contains(&specific_compiler_matcher)?;
    }

    Ok(())
}

/// Test build stdout capture during interception
#[test]
#[cfg(all(has_executable_shell, has_executable_echo, has_executable_true))]
fn build_stdout_capture() -> Result<()> {
    let env = TestEnvironment::new("stdout_capture")?;

    // Create script that outputs to stdout
    let script_commands = [
        format!("\"{}\" \"This goes to stdout\"", ECHO_PATH),
        format!("\"{}\" \"This also goes to stdout\"", ECHO_PATH),
        format!("\"{}\"", TRUE_PATH),
    ]
    .join("\n");

    let script_path = env.create_shell_script("stdout_test.sh", &script_commands)?;

    let output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Stdout should be preserved
    assert!(output.stdout().contains("This goes to stdout"));
    assert!(output.stdout().contains("This also goes to stdout"));

    // Events should still be captured
    let events_content = std::fs::read_to_string(env.temp_dir().join("events.json"))?;
    assert!(!events_content.is_empty());

    Ok(())
}

/// Test build stderr capture during interception
#[test]
#[cfg(all(has_executable_shell, has_executable_echo, has_executable_true))]
fn build_stderr_capture() -> Result<()> {
    let env = TestEnvironment::new("stderr_capture")?;

    // Create script that outputs to stderr
    let script_commands = [
        format!("\"{}\" \"This goes to stderr\" >&2", ECHO_PATH),
        format!("\"{}\" \"This also goes to stderr\" >&2", ECHO_PATH),
        format!("\"{}\"", TRUE_PATH),
    ]
    .join("\n");

    let script_path = env.create_shell_script("stderr_test.sh", &script_commands)?;

    let output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Stderr should be preserved in the stderr stream
    assert!(output.stderr().contains("This goes to stderr"));
    assert!(output.stderr().contains("This also goes to stderr"));

    // Events should still be captured
    let events_content = std::fs::read_to_string(env.temp_dir().join("events.json"))?;
    assert!(!events_content.is_empty());

    Ok(())
}

/// Test interception with empty environment
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_env))]
fn intercept_empty_environment() -> Result<()> {
    let env = TestEnvironment::new("empty_env_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Run with minimal environment
    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        ENV_PATH,
        "-i",
        "PATH=/usr/bin:/bin",
        COMPILER_C_PATH,
        "-c",
        "test.c",
    ])?;

    // Should still capture execution even with empty environment
    let events_content = std::fs::read_to_string(env.temp_dir().join("events.json"))?;
    let events: Vec<Value> =
        events_content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();

    assert!(!events.is_empty());

    Ok(())
}

/// Test libtool command interception
///
/// Note: This test might be fragile test, because libtool versions are different.
/// eg.: MacOS CI was failing to complain about "unknown option character `-' in: --mode=compile".
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_libtool, has_executable_compiler_c))]
fn libtool_command_interception() -> Result<()> {
    let env = TestEnvironment::new("libtool_intercept")?;

    env.create_source_files(&[("lib.c", "int lib_func() { return 42; }")])?;

    // Use libtool to compile a library
    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        LIBTOOL_PATH,
        "--mode=compile",
        "--tag=CC",
        COMPILER_C_PATH,
        "-c",
        "lib.c",
    ])?;

    // Should capture libtool and compiler invocations
    let events_content = std::fs::read_to_string(env.temp_dir().join("events.json"))?;
    let events: Vec<Value> =
        events_content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();

    assert!(!events.is_empty());

    // Should have captured libtool execution
    let libtool_events = events
        .iter()
        .filter(|event| {
            event
                .get("execution")
                .and_then(|e| e.get("executable"))
                .and_then(|exe| exe.as_str())
                .map(|exe| exe.contains("libtool"))
                .unwrap_or(false)
        })
        .count();

    assert!(libtool_events >= 1, "Should capture libtool execution");

    Ok(())
}

/// Test wrapper-based interception
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
fn wrapper_based_interception() -> Result<()> {
    let env = TestEnvironment::new("wrapper_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Create a wrapper script
    let wrapper_commands = format!(
        r#"{} "Wrapper called with: $@"
exec {} "$@""#,
        ECHO_PATH, COMPILER_C_PATH
    );

    let wrapper_path = env.create_shell_script("cc-wrapper", &wrapper_commands)?;

    // Test with wrapper-based interception (when preload isn't available)
    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        wrapper_path.to_str().unwrap(),
        "-c",
        "test.c",
    ])?;

    // Should capture wrapper execution
    let events_content = std::fs::read_to_string(env.temp_dir().join("events.json"))?;
    let events: Vec<Value> =
        events_content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();

    assert!(!events.is_empty());

    Ok(())
}

/// Test Unicode handling in shell commands
#[test]
#[cfg(all(has_executable_shell, has_executable_echo, has_executable_true))]
fn unicode_shell_commands() -> Result<()> {
    let env = TestEnvironment::new("unicode_intercept")?;

    // Create script with Unicode content
    let unicode_commands = [
        format!("\"{}\" \"Testing Unicode: αβγδε 中文 🚀\"", ECHO_PATH),
        format!("\"{}\" \"Файл с русскими именами\"", ECHO_PATH),
        format!("\"{}\"", TRUE_PATH),
    ]
    .join("\n");

    let script_path = env.create_shell_script("unicode_test.sh", &unicode_commands)?;

    let output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Should handle Unicode properly
    assert!(output.stdout().contains("αβγδε"));
    assert!(output.stdout().contains("中文"));
    assert!(output.stdout().contains("🚀"));

    // Events file should be created
    let events_path = env.temp_dir().join("events.json");
    assert!(events_path.exists());

    Ok(())
}

/// Test interception with ISO-8859-2 encoding
#[test]
#[cfg(all(has_executable_shell, has_executable_echo, has_executable_true))]
fn iso8859_2_encoding() -> Result<()> {
    let env = TestEnvironment::new("iso8859_2")?;

    // Create script with ISO-8859-2 characters
    let script_commands =
        [format!("\"{}\" 'Testing ISO-8859-2: ąęłńóśźż'", ECHO_PATH), format!("\"{}\"", TRUE_PATH)]
            .join("\n");
    let script_path = env.create_shell_script("iso_test.sh", &script_commands)?;

    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Should handle encoding properly
    let events_path = env.temp_dir().join("events.json");
    assert!(events_path.exists());

    Ok(())
}

/// Test Valgrind integration
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_valgrind, has_executable_compiler_c))]
fn valgrind_integration() -> Result<()> {
    let env = TestEnvironment::new("valgrind_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        VALGRIND_PATH,
        "--tool=memcheck",
        COMPILER_C_PATH,
        "-c",
        "test.c",
    ])?;

    // Should capture valgrind execution
    let events_content = std::fs::read_to_string(env.temp_dir().join("events.json"))?;
    let events: Vec<Value> =
        events_content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();

    assert!(!events.is_empty());

    Ok(())
}

/// Test fakeroot integration
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_fakeroot, has_executable_compiler_c))]
fn fakeroot_integration() -> Result<()> {
    let env = TestEnvironment::new("fakeroot_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        FAKEROOT_PATH,
        COMPILER_C_PATH,
        "-c",
        "test.c",
    ])?;

    // Should capture fakeroot execution
    let events_content = std::fs::read_to_string(env.temp_dir().join("events.json"))?;
    let events: Vec<Value> =
        events_content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();

    assert!(!events.is_empty());

    Ok(())
}
