// SPDX-License-Identifier: GPL-3.0-or-later

//! POSIX system call interception tests for Bear integration
//!
//! The idea is that we write C programs which are calling the specific function
//! and verify if the interception get these calls recorded. This only works with
//! the preload mode, since it does not involve any compiler.

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::*;
use anyhow::Result;

/// Create a minimal config to enforce the preload mode.
///
/// This might be the default mode on many platform, but that's not always the case.
/// The preload might work on MacOS, but not set to default. Here we can enforce it.
const CONFIG: &str = concat!(
    r#"schema: '4.0'

intercept:
  mode: preload
  path: "#,
    env!("PRELOAD_LIBRARY_PATH"),
    r#"
"#
);

/// Test execl system call interception with multiple arguments
///
/// execl(path, arg0, arg1, ..., NULL) - variadic, no PATH search, inherited env
#[test]
#[cfg(has_symbol_execl)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
fn execl_interception() -> Result<()> {
    let env = TestEnvironment::new("execl_intercept")?;
    env.create_config(CONFIG)?;

    // Test with multiple variadic arguments to catch truncation bugs
    let c_program = format!(
        r#"#include <unistd.h>

int main() {{
    return execl("{}", "{}", "arg1", "arg2", "arg3", (char *)0);
}}"#,
        ECHO_PATH, ECHO_PATH
    );
    env.create_source_files(&[("test_execl.c", &c_program)])?;
    env.run_c_compiler("test_execl", &["test_execl.c"])?;

    // Run intercept on the compiled program
    env.run_bear_success(&[
        "--config",
        "config.yml",
        "intercept",
        "--output",
        "events.json",
        "--",
        "./test_execl",
    ])?;

    // Verify intercepted events - check BOTH executable AND arguments
    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo").arguments(vec![
        ECHO_PATH.to_string(),
        "arg1".to_string(),
        "arg2".to_string(),
        "arg3".to_string(),
    ]))?;

    Ok(())
}

/// Test execlp system call interception (searches PATH)
///
/// execlp(file, arg0, arg1, ..., NULL) - variadic, PATH search, inherited env
#[test]
#[cfg(has_symbol_execlp)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
fn execlp_interception() -> Result<()> {
    let env = TestEnvironment::new("execlp_intercept")?;
    env.create_config(CONFIG)?;

    // execlp searches PATH, so we use just "echo" instead of full path
    let c_program = r#"#include <unistd.h>

int main() {
    return execlp("echo", "echo", "arg1", "arg2", "arg3", (char *)0);
}"#;

    env.create_source_files(&[("test_execlp.c", c_program)])?;
    env.run_c_compiler("test_execlp", &["test_execlp.c"])?;

    env.run_bear_success(&[
        "--config",
        "config.yml",
        "intercept",
        "--output",
        "events.json",
        "--",
        "./test_execlp",
    ])?;

    // For execlp, we verify the arguments but not the full path since it's resolved via PATH
    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo").arguments(vec![
        "echo".to_string(),
        "arg1".to_string(),
        "arg2".to_string(),
        "arg3".to_string(),
    ]))?;

    Ok(())
}

/// Test execle system call interception (with explicit environment)
///
/// execle(path, arg0, arg1, ..., NULL, envp) - variadic, no PATH search, explicit env
#[test]
#[cfg(has_symbol_execle)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
fn execle_interception() -> Result<()> {
    let env = TestEnvironment::new("execle_intercept")?;
    env.create_config(CONFIG)?;

    // execle takes environment as final argument after NULL
    let c_program = format!(
        r#"#include <unistd.h>

int main() {{
    char *const envp[] = {{ "MY_VAR=test_value", "ANOTHER=123", 0 }};
    return execle("{}", "{}", "arg1", "arg2", "arg3", (char *)0, envp);
}}"#,
        ECHO_PATH, ECHO_PATH
    );
    env.create_source_files(&[("test_execle.c", &c_program)])?;
    env.run_c_compiler("test_execle", &["test_execle.c"])?;

    env.run_bear_success(&[
        "--config",
        "config.yml",
        "intercept",
        "--output",
        "events.json",
        "--",
        "./test_execle",
    ])?;

    // Verify arguments were captured correctly
    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo").arguments(vec![
        ECHO_PATH.to_string(),
        "arg1".to_string(),
        "arg2".to_string(),
        "arg3".to_string(),
    ]))?;

    Ok(())
}

/// Test execv system call interception
///
/// execv(path, argv) - array, no PATH search, inherited env
#[test]
#[cfg(has_symbol_execv)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
fn execv_interception() -> Result<()> {
    let env = TestEnvironment::new("execv_intercept")?;
    env.create_config(CONFIG)?;

    let c_program = format!(
        r#"#include <unistd.h>

int main() {{
    char *const argv[] = {{ "{}", "arg1", "arg2", "arg3", 0 }};
    return execv("{}", argv);
}}"#,
        ECHO_PATH, ECHO_PATH
    );
    env.create_source_files(&[("test_execv.c", &c_program)])?;
    env.run_c_compiler("test_execv", &["test_execv.c"])?;

    env.run_bear_success(&[
        "--config",
        "config.yml",
        "intercept",
        "--output",
        "events.json",
        "--",
        "./test_execv",
    ])?;

    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo").arguments(vec![
        ECHO_PATH.to_string(),
        "arg1".to_string(),
        "arg2".to_string(),
        "arg3".to_string(),
    ]))?;

    Ok(())
}

/// Test execve system call interception
///
/// execve(path, argv, envp) - array, no PATH search, explicit env
#[test]
#[cfg(has_symbol_execve)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
fn execve_interception() -> Result<()> {
    let env = TestEnvironment::new("execve_intercept")?;
    env.create_config(CONFIG)?;

    let c_program = format!(
        r#"#include <unistd.h>

int main() {{
    char *const argv[] = {{ "{}", "arg1", "arg2", "arg3", 0 }};
    char *const envp[] = {{ "TEST_VAR=test_value", 0 }};
    return execve("{}", argv, envp);
}}"#,
        ECHO_PATH, ECHO_PATH
    );
    env.create_source_files(&[("test_execve.c", &c_program)])?;
    env.run_c_compiler("test_execve", &["test_execve.c"])?;

    env.run_bear_success(&[
        "--config",
        "config.yml",
        "intercept",
        "--output",
        "events.json",
        "--",
        "./test_execve",
    ])?;

    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo").arguments(vec![
        ECHO_PATH.to_string(),
        "arg1".to_string(),
        "arg2".to_string(),
        "arg3".to_string(),
    ]))?;

    Ok(())
}

/// Test execvp system call interception (searches PATH)
///
/// execvp(file, argv) - array, PATH search, inherited env
#[test]
#[cfg(has_symbol_execvp)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
fn execvp_interception() -> Result<()> {
    let env = TestEnvironment::new("execvp_intercept")?;
    env.create_config(CONFIG)?;

    let c_program = r#"#include <unistd.h>

int main() {
    char *const argv[] = {"echo", "arg1", "arg2", "arg3", 0};
    return execvp("echo", argv);
}"#;

    env.create_source_files(&[("test_execvp.c", c_program)])?;
    env.run_c_compiler("test_execvp", &["test_execvp.c"])?;

    env.run_bear_success(&[
        "--config",
        "config.yml",
        "intercept",
        "--output",
        "events.json",
        "--",
        "./test_execvp",
    ])?;

    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo").arguments(vec![
        "echo".to_string(),
        "arg1".to_string(),
        "arg2".to_string(),
        "arg3".to_string(),
    ]))?;

    Ok(())
}

/// Test execvpe system call (GNU extension - searches PATH with explicit env)
///
/// execvpe(file, argv, envp) - array, PATH search, explicit env
#[test]
#[cfg(has_symbol_execvpe)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
fn execvpe_interception() -> Result<()> {
    let env = TestEnvironment::new("execvpe_intercept")?;
    env.create_config(CONFIG)?;

    // execvpe is a GNU extension, may not be available on all systems
    let c_program = r#"#define _GNU_SOURCE
#include <unistd.h>

int main() {
    char *const argv[] = {"echo", "arg1", "arg2", "arg3", 0};
    char *const envp[] = {"TEST=execvpe", 0};

    return execvpe("echo", argv, envp);
}"#;
    env.create_source_files(&[("test_execvpe.c", c_program)])?;
    env.run_c_compiler("test_execvpe", &["test_execvpe.c"])?;

    env.run_bear_success(&[
        "--config",
        "config.yml",
        "intercept",
        "--output",
        "events.json",
        "--",
        "./test_execvpe",
    ])?;

    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo").arguments(vec![
        "echo".to_string(),
        "arg1".to_string(),
        "arg2".to_string(),
        "arg3".to_string(),
    ]))?;

    Ok(())
}

/// Test posix_spawn interception
#[test]
#[cfg(has_symbol_posix_spawn)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
fn posix_spawn_interception() -> Result<()> {
    let env = TestEnvironment::new("posix_spawn_intercept")?;
    env.create_config(CONFIG)?;

    let c_program = format!(
        r#"#include <spawn.h>
#include <sys/wait.h>
#include <unistd.h>

int main() {{
    pid_t pid;
    char *const argv[] = {{ "{}", "arg1", "arg2", "arg3", 0 }};
    char *const envp[] = {{ "SPAWN_TEST=1", 0 }};

    int result = posix_spawn(&pid, "{}", NULL, NULL, argv, envp);
    if (result == 0) {{
        int status;
        waitpid(pid, &status, 0);
        return WEXITSTATUS(status);
    }}
    return result;
}}"#,
        ECHO_PATH, ECHO_PATH
    );

    env.create_source_files(&[("test_posix_spawn.c", &c_program)])?;
    env.run_c_compiler("test_posix_spawn", &["test_posix_spawn.c"])?;

    env.run_bear_success(&[
        "--config",
        "config.yml",
        "intercept",
        "--output",
        "events.json",
        "--",
        "./test_posix_spawn",
    ])?;

    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo").arguments(vec![
        ECHO_PATH.to_string(),
        "arg1".to_string(),
        "arg2".to_string(),
        "arg3".to_string(),
    ]))?;

    Ok(())
}

/// Test posix_spawnp interception (searches PATH)
#[test]
#[cfg(has_symbol_posix_spawnp)]
#[cfg(has_executable_compiler_c)]
fn posix_spawnp_interception() -> Result<()> {
    let env = TestEnvironment::new("posix_spawnp_intercept")?;
    env.create_config(CONFIG)?;

    let c_program = r#"#include <spawn.h>
#include <sys/wait.h>
#include <unistd.h>

int main() {
    pid_t pid;
    char *const argv[] = {"echo", "arg1", "arg2", "arg3", 0};
    char *const envp[] = {"TEST=1", 0};

    int result = posix_spawnp(&pid, "echo", NULL, NULL, argv, envp);
    if (result == 0) {
        int status;
        waitpid(pid, &status, 0);
        return WEXITSTATUS(status);
    }
    return result;
}"#;

    env.create_source_files(&[("test_posix_spawnp.c", c_program)])?;
    env.run_c_compiler("test_posix_spawnp", &["test_posix_spawnp.c"])?;

    env.run_bear_success(&[
        "--config",
        "config.yml",
        "intercept",
        "--output",
        "events.json",
        "--",
        "./test_posix_spawnp",
    ])?;

    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo").arguments(vec![
        "echo".to_string(),
        "arg1".to_string(),
        "arg2".to_string(),
        "arg3".to_string(),
    ]))?;

    Ok(())
}

/// Test popen system call interception
#[test]
#[cfg(has_symbol_popen)]
#[cfg(all(has_executable_compiler_c, has_executable_cat))]
fn popen_interception() -> Result<()> {
    let env = TestEnvironment::new("popen_intercept")?;
    env.create_config(CONFIG)?;

    let c_program = format!(
        r#"#include <stdio.h>
#include <stdlib.h>

void write_data(FILE *stream) {{
    int i;
    for (i = 0; i < 10; i++) {{
        fprintf(stream, "%d\n", i);
    }}
    if (ferror(stream)) {{
        fprintf(stderr, "Output to stream failed.\n");
        exit(EXIT_FAILURE);
    }}
}}

int main(void) {{
    FILE *output;

    output = popen("{}", "w");
    if (!output) {{
        fprintf(stderr, "Could not run cat.\n");
        return EXIT_FAILURE;
    }}
    write_data(output);
    if (pclose(output) != 0) {{
        fprintf(stderr, "Could not run cat or other error.\n");
    }}
    return EXIT_SUCCESS;
}}"#,
        CAT_PATH
    );

    env.create_source_files(&[("test_popen.c", &c_program)])?;
    env.run_c_compiler("test_popen", &["test_popen.c"])?;

    env.run_bear_success(&[
        "--config",
        "config.yml",
        "intercept",
        "--output",
        "events.json",
        "--",
        "./test_popen",
    ])?;

    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("cat"))?;

    Ok(())
}

/// Test system() call interception
#[test]
#[cfg(has_symbol_system)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
fn system_interception() -> Result<()> {
    let env = TestEnvironment::new("system_intercept")?;
    env.create_config(CONFIG)?;

    let c_program = format!(
        r#"#include <stdlib.h>

int main() {{
    return system("{} arg1 arg2 arg3");
}}"#,
        ECHO_PATH
    );

    env.create_source_files(&[("test_system.c", &c_program)])?;
    env.run_c_compiler("test_system", &["test_system.c"])?;

    env.run_bear_success(&[
        "--config",
        "config.yml",
        "intercept",
        "--output",
        "events.json",
        "--",
        "./test_system",
    ])?;

    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo").arguments(vec![
        ECHO_PATH.to_string(),
        "arg1".to_string(),
        "arg2".to_string(),
        "arg3".to_string(),
    ]))?;

    Ok(())
}

/// Test errno handling with failed exec calls
#[test]
#[cfg(has_symbol_execve)]
#[cfg(has_executable_compiler_c)]
fn test_failed_exec_errno_handling() -> Result<()> {
    let env = TestEnvironment::new("failed_exec_errno")?;
    env.create_config(CONFIG)?;

    let c_program = r#"#include <unistd.h>
#include <stdio.h>
#include <errno.h>
#include <string.h>

int main() {
    char *const argv[] = {"nonexistent_program", 0};
    char *const envp[] = {0};

    int result = execve("/nonexistent/path/program", argv, envp);

    // This should only execute if execve fails
    printf("execve failed: %s\n", strerror(errno));
    return result;
}"#;

    env.create_source_files(&[("test_failed_exec.c", c_program)])?;
    env.run_c_compiler("test_failed_exec", &["test_failed_exec.c"])?;

    // Run intercept on the compiled program
    let intercept_output = env.run_bear(&[
        "--config",
        "config.yml",
        "intercept",
        "--output",
        "events.json",
        "--",
        "./test_failed_exec",
    ])?;

    // The program should fail (non-zero exit) but intercept should still work
    intercept_output.assert_failure()?;

    // Should still be able to load events file
    let events = env.load_events_file("events.json")?;
    let _ = events.events();

    Ok(())
}

/// Test that programs with no exec calls don't generate spurious events
#[test]
#[cfg(has_executable_compiler_c)]
fn test_no_exec_calls() -> Result<()> {
    let env = TestEnvironment::new("no_exec")?;
    env.create_config(CONFIG)?;

    let c_program = r#"#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main() {
    printf("This program does not call any exec functions\n");
    printf("Process ID: %d\n", getpid());
    return EXIT_SUCCESS;
}"#;

    env.create_source_files(&[("test_no_exec.c", c_program)])?;
    env.run_c_compiler("test_no_exec", &["test_no_exec.c"])?;

    env.run_bear_success(&[
        "--config",
        "config.yml",
        "intercept",
        "--output",
        "events.json",
        "--",
        "./test_no_exec",
    ])?;

    // For programs that don't call exec functions, we expect minimal events.
    // Note: The test program itself may be captured during startup, so we
    // just verify we don't see unexpected child process executions.
    let events = env.load_events_file("events.json")?;
    let event_count = events.events().len();

    // We should have at most 1 event (the test program itself)
    // and no events for echo, sh, or other child processes
    assert!(
        event_count <= 1,
        "Programs without exec calls should generate at most 1 event (got {})",
        event_count
    );

    Ok(())
}
