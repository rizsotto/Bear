// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;
use anyhow::Result;

#[test]
fn environment_creation() -> Result<()> {
    let env = TestEnvironment::new("test_creation")?;
    assert!(env.test_dir().exists());
    Ok(())
}

#[test]
fn file_creation() -> Result<()> {
    let env = TestEnvironment::new("test_files")?;
    env.create_source_files(&[("test.c", "int main() { return 0; }"), ("subdir/test.h", "#pragma once")])?;

    assert!(env.test_dir().join("test.c").exists());
    assert!(env.test_dir().join("subdir/test.h").exists());
    Ok(())
}

#[test]
fn compilation_matcher() {
    let entry = serde_json::json!({
        "file": "/path/to/test.c",
        "directory": "/path/to",
        "arguments": ["gcc", "-c", "test.c"]
    });

    let matcher = CompilationEntryMatcher::new()
        .file("/path/to/test.c")
        .directory("/path/to")
        .arguments(vec!["gcc".to_string(), "-c".to_string(), "test.c".to_string()]);

    assert!(matcher.matches(&entry));
}

#[test]
fn event_matcher_executable_path() {
    let event = serde_json::json!({
        "executable": "/usr/bin/gcc",
        "arguments": ["gcc", "-c", "test.c"],
        "working_directory": "/tmp"
    });

    let matcher = EventMatcher::new().executable_path("/usr/bin/gcc");

    assert!(matcher.matches(&event));

    // Test non-matching path
    let matcher_no_match = EventMatcher::new().executable_path("/usr/bin/clang");

    assert!(!matcher_no_match.matches(&event));
}

#[test]
fn event_matcher_executable_name() {
    let event = serde_json::json!({
        "executable": "/usr/bin/gcc",
        "arguments": ["gcc", "-c", "test.c"]
    });

    let matcher = EventMatcher::new().executable_name("gcc");

    assert!(matcher.matches(&event));

    // Test partial name matching
    let matcher_partial = EventMatcher::new().executable_name("gc");

    assert!(matcher_partial.matches(&event));

    // Test non-matching name
    let matcher_no_match = EventMatcher::new().executable_name("clang");

    assert!(!matcher_no_match.matches(&event));
}

#[test]
fn event_matcher_arguments() {
    let event = serde_json::json!({
        "executable": "/usr/bin/gcc",
        "arguments": ["gcc", "-c", "test.c", "-o", "test.o"]
    });

    let matcher = EventMatcher::new().arguments(vec![
        "gcc".to_string(),
        "-c".to_string(),
        "test.c".to_string(),
        "-o".to_string(),
        "test.o".to_string(),
    ]);

    assert!(matcher.matches(&event));

    // Test non-matching arguments
    let matcher_no_match =
        EventMatcher::new().arguments(vec!["gcc".to_string(), "-c".to_string(), "other.c".to_string()]);

    assert!(!matcher_no_match.matches(&event));
}

#[test]
#[cfg(has_executable_compiler_c)]
fn run_c_compiler_basic() -> Result<()> {
    let env = TestEnvironment::new("test_c_compiler")?;

    // Create a simple C program
    env.create_source_files(&[(
        "hello.c",
        r#"
#include <stdio.h>
int main() {
    printf("Hello, World!\n");
    return 0;
}
"#,
    )])?;

    // Compile it using our new method
    let executable_path = env.run_c_compiler("hello", &["hello.c"])?;

    // Verify the executable exists at the returned path
    assert!(executable_path.exists());

    // Verify the executable actually works by running it
    let output = std::process::Command::new(&executable_path)
        .current_dir(env.test_dir())
        .output()
        .expect("Failed to run compiled executable");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello, World!"));

    Ok(())
}

#[test]
#[cfg(has_executable_compiler_c)]
fn run_c_compiler_error_handling() -> Result<()> {
    let env = TestEnvironment::new("test_c_compiler_error")?;

    // Create a C program with syntax errors
    env.create_source_files(&[(
        "broken.c",
        r#"
#include <stdio.h>
int main() {
    printf("Hello, World!\n"  // Missing closing parenthesis and semicolon
    return 0;
}
"#,
    )])?;

    // Compilation should fail and return an error
    let compile_result = env.run_c_compiler("broken", &["broken.c"]);
    assert!(compile_result.is_err());

    Ok(())
}

#[test]
fn assert_min_count_test() -> Result<()> {
    use serde_json::json;

    // Create mock events
    let events = vec![
        json!({"executable": "/usr/bin/gcc", "arguments": ["gcc", "-c", "test.c"]}),
        json!({"executable": "/usr/bin/gcc", "arguments": ["gcc", "-c", "test2.c"]}),
    ];

    let intercept_events = InterceptEvents { events };

    // Should pass when actual >= min_expected
    assert!(intercept_events.assert_min_count(1).is_ok());
    assert!(intercept_events.assert_min_count(2).is_ok());

    // Should fail when actual < min_expected
    assert!(intercept_events.assert_min_count(3).is_err());

    Ok(())
}

#[cfg(all(unix, has_executable_shell))]
mod reset_signals {
    use super::*;
    use crate::fixtures::constants::SHELL_PATH;
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    fn shell_that_trusts_its_trap() -> Command {
        let mut command = Command::new(SHELL_PATH);
        command.arg("-c").arg("trap 'echo trapped' INT; kill -INT $$; echo survived");
        command
    }

    fn ignore_sigint(command: &mut Command) {
        // SAFETY: runs between fork and exec and calls only `signal`, which is async-signal-safe.
        unsafe {
            command.pre_exec(|| {
                libc::signal(libc::SIGINT, libc::SIG_IGN);
                Ok(())
            });
        }
    }

    #[test]
    fn restores_an_ignored_signal() {
        let mut command = shell_that_trusts_its_trap();
        ignore_sigint(&mut command);
        reset_signals(&mut command);

        let sut = command.output().unwrap();

        let stdout = String::from_utf8_lossy(&sut.stdout);
        assert!(stdout.contains("trapped"), "stdout: {stdout}");
    }

    #[test]
    fn is_needed_when_the_signal_is_ignored() {
        let mut command = shell_that_trusts_its_trap();
        ignore_sigint(&mut command);

        let sut = command.output().unwrap();

        let stdout = String::from_utf8_lossy(&sut.stdout);
        assert!(!stdout.contains("trapped"), "stdout: {stdout}");
    }
}

/// CI sets `BEAR_TEST_REQUIRE_PTY`, so a host without a pty cannot skip every
/// terminal test unnoticed.
#[test]
#[cfg(all(unix, has_symbol_openpty, has_symbol_login_tty))]
fn pty_opens_when_required() {
    if std::env::var_os("BEAR_TEST_REQUIRE_PTY").is_none() {
        return;
    }

    let sut = open_pty();

    assert!(sut.is_ok(), "BEAR_TEST_REQUIRE_PTY is set, but openpty failed: {:?}", sut.err());
}
