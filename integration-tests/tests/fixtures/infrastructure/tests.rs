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
        "execution": {
            "executable": "/usr/bin/gcc",
            "arguments": ["gcc", "-c", "test.c"],
            "working_directory": "/tmp"
        }
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
        "execution": {
            "executable": "/usr/bin/gcc",
            "arguments": ["gcc", "-c", "test.c"]
        }
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
        "execution": {
            "executable": "/usr/bin/gcc",
            "arguments": ["gcc", "-c", "test.c", "-o", "test.o"]
        }
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
fn event_matcher_no_execution() {
    let event = serde_json::json!({
        "other_field": "value"
    });

    let matcher = EventMatcher::new().executable_path("/usr/bin/gcc");

    // Should not match if looking for execution fields but no execution present
    assert!(!matcher.matches(&event));

    // Should match if not looking for execution-specific fields
    let empty_matcher = EventMatcher::new();
    assert!(empty_matcher.matches(&event));
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
        json!({"execution": {"executable": "/usr/bin/gcc", "arguments": ["gcc", "-c", "test.c"]}}),
        json!({"execution": {"executable": "/usr/bin/gcc", "arguments": ["gcc", "-c", "test2.c"]}}),
    ];

    let intercept_events = InterceptEvents { events, verbose: false, bear_output: None };

    // Should pass when actual >= min_expected
    assert!(intercept_events.assert_min_count(1).is_ok());
    assert!(intercept_events.assert_min_count(2).is_ok());

    // Should fail when actual < min_expected
    assert!(intercept_events.assert_min_count(3).is_err());

    Ok(())
}
