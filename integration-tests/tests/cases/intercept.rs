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
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
#[cfg(has_executable_compiler_c)]
fn basic_command_interception() -> Result<()> {
    let env = TestEnvironment::new("basic_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Run intercept mode to capture commands
    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        COMPILER_C_PATH,
        "-c",
        "test.c",
    ])?;

    // Load and verify events file
    let events_content = std::fs::read_to_string(env.temp_dir().join("events.json"))?;
    let events: Vec<Value> = events_content
        .lines()
        .map(|line| serde_json::from_str(line))
        .collect::<Result<Vec<_>, _>>()?;

    assert!(!events.is_empty());

    // Should contain command execution event
    let has_exec_event = events.iter().any(|event| {
        event.get("execution").is_some()
            && event
                .get("execution")
                .and_then(|e| e.get("executable"))
                .and_then(|exe| exe.as_str())
                .map(|exe| exe.contains("gcc") || exe.contains("cc") || exe == COMPILER_C_PATH)
                .unwrap_or(false)
    });

    assert!(
        has_exec_event,
        "No compiler execution event found in intercept output"
    );

    Ok(())
}

/// Test shell command interception
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
#[cfg(has_executable_compiler_c)]
fn shell_command_interception() -> Result<()> {
    let env = TestEnvironment::new("shell_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Create shell script that runs compiler
    let build_commands = format!(
        "echo \"Starting build...\"\n{} -c test.c -o test.o\necho \"Build complete\"",
        COMPILER_C_PATH
    );

    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Intercept shell script execution
    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Verify intercepted events
    let events_content = std::fs::read_to_string(env.temp_dir().join("events.json"))?;
    let events: Vec<Value> = events_content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    assert!(!events.is_empty());

    // Should capture both shell and compiler execution
    let shell_events = events
        .iter()
        .filter(|event| {
            event
                .get("execution")
                .and_then(|e| e.get("executable"))
                .and_then(|exe| exe.as_str())
                .map(|exe| exe.contains("sh"))
                .unwrap_or(false)
        })
        .count();

    let compiler_events = events
        .iter()
        .filter(|event| {
            event
                .get("execution")
                .and_then(|e| e.get("executable"))
                .and_then(|exe| exe.as_str())
                .map(|exe| exe.contains("cc") || exe == COMPILER_C_PATH)
                .unwrap_or(false)
        })
        .count();

    assert!(shell_events >= 1, "Should capture shell execution");
    assert!(compiler_events >= 1, "Should capture compiler execution");

    Ok(())
}

/// Test shell commands without shebang
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
#[cfg(has_executable_compiler_c)]
fn shell_commands_without_shebang() -> Result<()> {
    let env = TestEnvironment::new("shell_no_shebang")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Create shell script WITHOUT shebang
    let shell_script = format!(
        r#"echo "Building without shebang..."
{cc} -c test.c
echo "Done"
"#,
        cc = COMPILER_C_PATH
    );

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
    let events: Vec<Value> = events_content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    assert!(!events.is_empty());

    Ok(())
}

/// Test parallel command interception
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
#[cfg(has_executable_compiler_c)]
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
    let build_commands = format!(
        "{} -c test_1.c &\n{} -c test_2.c &\n{} -c test_3.c &\n{} -c test_4.c &\nwait",
        COMPILER_C_PATH, COMPILER_C_PATH, COMPILER_C_PATH, COMPILER_C_PATH
    );

    let script_path = env.create_shell_script("parallel_build.sh", &build_commands)?;

    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Should capture all parallel executions
    let events_content = std::fs::read_to_string(env.temp_dir().join("events.json"))?;
    let events: Vec<Value> = events_content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    // Should have events for all 4 compiler invocations
    let compiler_events = events
        .iter()
        .filter(|event| {
            event
                .get("execution")
                .and_then(|e| e.get("executable"))
                .and_then(|exe| exe.as_str())
                .map(|exe| exe.contains("cc") || exe == COMPILER_C_PATH)
                .unwrap_or(false)
        })
        .count();

    assert!(
        compiler_events >= 4,
        "Should capture all 4 parallel compiler invocations"
    );

    Ok(())
}

/// Test build stdout capture during interception
#[test]
fn build_stdout_capture() -> Result<()> {
    let env = TestEnvironment::new("stdout_capture")?;

    // Create script that outputs to stdout
    let script_commands = r#"echo "This goes to stdout"
echo "This also goes to stdout"
true"#;

    let script_path = env.create_shell_script("stdout_test.sh", script_commands)?;

    let output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        "sh",
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
fn build_stderr_capture() -> Result<()> {
    let env = TestEnvironment::new("stderr_capture")?;

    // Create script that outputs to stderr
    let script_commands = r#"echo "This goes to stderr" >&2
echo "This also goes to stderr" >&2
true"#;

    let script_path = env.create_shell_script("stderr_test.sh", script_commands)?;

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
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
#[cfg(has_executable_compiler_c)]
fn intercept_empty_environment() -> Result<()> {
    let env = TestEnvironment::new("empty_env_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Run with minimal environment
    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        "env",
        "-i",
        "PATH=/usr/bin:/bin",
        COMPILER_C_PATH,
        "-c",
        "test.c",
    ])?;

    // Should still capture execution even with empty environment
    let events_content = std::fs::read_to_string(env.temp_dir().join("events.json"))?;
    let events: Vec<Value> = events_content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    assert!(!events.is_empty());

    Ok(())
}

/// Test libtool command interception
///
/// Note: This test might be fragile test, because libtool versions are different.
/// eg.: MacOS CI was failing to complain about "unknown option character `-' in: --mode=compile".
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
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
    let events: Vec<Value> = events_content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

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
#[cfg(has_executable_compiler_c)]
fn wrapper_based_interception() -> Result<()> {
    let env = TestEnvironment::new("wrapper_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Create a wrapper script
    let wrapper_commands = format!(
        r#"echo "Wrapper called with: $@"
exec {} "$@""#,
        COMPILER_C_PATH
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
    let events: Vec<Value> = events_content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    assert!(!events.is_empty());

    Ok(())
}

/// Test Unicode handling in shell commands
#[test]
#[cfg(has_executable_shell)]
fn unicode_shell_commands() -> Result<()> {
    let env = TestEnvironment::new("unicode_intercept")?;

    // Create script with Unicode content
    let unicode_commands = r#"echo "Testing Unicode: αβγδε 中文 🚀"
echo "Файл с русскими именами"
true"#;

    let script_path = env.create_shell_script("unicode_test.sh", unicode_commands)?;

    let output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        "sh",
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
#[cfg(has_executable_shell)]
fn iso8859_2_encoding() -> Result<()> {
    let env = TestEnvironment::new("iso8859_2")?;

    // Create script with ISO-8859-2 characters
    let script_commands = "echo 'Testing ISO-8859-2: ąęłńóśźż'\ntrue";
    let script_path = env.create_shell_script("iso_test.sh", script_commands)?;

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
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
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
    let events: Vec<Value> = events_content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    assert!(!events.is_empty());

    Ok(())
}

/// Test fakeroot integration
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
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
    let events: Vec<Value> = events_content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    assert!(!events.is_empty());

    Ok(())
}
