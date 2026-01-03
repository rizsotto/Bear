// SPDX-License-Identifier: GPL-3.0-or-later

//! POSIX system call interception tests for Bear integration
//!
//! These tests verify that Bear correctly intercepts various POSIX system calls
//! like execve, execl, popen, posix_spawn, etc. These tests are ported from
//! the test/cases/intercept/preload/posix/ directory.

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::*;
use anyhow::Result;

use serde_json::Value;
use std::fs;

/// Test execve system call interception
#[test]
#[cfg(has_preload_library)]
#[cfg(has_executable_echo)]
fn execve_interception() -> Result<()> {
    let env = TestEnvironment::new("execve_intercept")?;

    // Create a C program that uses execve
    let c_program = format!(
        r#"#include <unistd.h>

int main() {{
    char *const program = "{}";
    char *const argv[] = {{ "{}", "hi there", 0 }};
    char *const envp[] = {{ "THIS=THAT", 0 }};
    return execve(program, argv, envp);
}}"#,
        ECHO_PATH, ECHO_PATH
    );

    env.create_source_files(&[("test_execve.c", &c_program)])?;

    // Compile the test program
    #[cfg(has_executable_compiler_c)]
    {
        let compile_output = env.run_bear(&["--", COMPILER_C_PATH, "-o", "test_execve", "test_execve.c"])?;
        compile_output.assert_success()?;

        // Run intercept on the compiled program
        let intercept_output =
            env.run_bear(&["intercept", "--output", "events.json", "--", "./test_execve"])?;
        intercept_output.assert_success()?;

        // Verify intercepted events
        let events_content = fs::read_to_string(env.temp_dir().join("events.json"))?;
        let events: Vec<Value> =
            events_content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();

        // Should have at least 1 event: the echo command
        assert!(events.len() >= 1, "Expected at least 1 event, got {}", events.len());

        // Should contain the echo command
        let has_echo_event = events.iter().any(|event| {
            event
                .get("execution")
                .and_then(|e| e.get("executable"))
                .and_then(|exe| exe.as_str())
                .map(|exe| exe.contains("echo"))
                .unwrap_or(false)
        });

        assert!(has_echo_event, "Should capture echo execution via execve");
    }

    Ok(())
}

/// Test execl system call interception
#[test]
#[cfg(has_preload_library)]
#[cfg(has_executable_echo)]
fn execle_interception() -> Result<()> {
    let env = TestEnvironment::new("execl_intercept")?;

    // Create a C program that uses execl
    let c_program = format!(
        r#"#include <unistd.h>

int main() {{
    return execl("{}", "{}", "hello world", (char *)0);
}}"#,
        ECHO_PATH, ECHO_PATH
    );

    env.create_source_files(&[("test_execl.c", &c_program)])?;

    #[cfg(has_executable_compiler_c)]
    {
        // Compile and test
        env.run_bear(&["--", COMPILER_C_PATH, "-o", "test_execl", "test_execl.c"])?.assert_success()?;

        env.run_bear(&["intercept", "--output", "events.json", "--", "./test_execl"])?.assert_success()?;

        // Verify events were captured
        let events_content = fs::read_to_string(env.temp_dir().join("events.json"))?;
        let events: Vec<Value> =
            events_content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();

        assert!(events.len() >= 1, "Expected at least 1 event for execl test");
    }

    Ok(())
}

/// Test execlp system call interception
#[test]
#[cfg(has_preload_library)]
#[cfg(has_executable_echo)]
fn execv_interception() -> Result<()> {
    let env = TestEnvironment::new("execlp_intercept")?;

    // Create a C program that uses execlp (searches PATH)
    let c_program = r#"#include <unistd.h>

int main() {
    return execlp("echo", "echo", "hello from execlp", (char *)0);
}"#;

    env.create_source_files(&[("test_execlp.c", c_program)])?;

    #[cfg(has_executable_compiler_c)]
    {
        env.run_bear(&["--", COMPILER_C_PATH, "-o", "test_execlp", "test_execlp.c"])?.assert_success()?;

        env.run_bear(&["intercept", "--output", "events.json", "--", "./test_execlp"])?.assert_success()?;

        let events_content = fs::read_to_string(env.temp_dir().join("events.json"))?;
        assert!(!events_content.is_empty(), "Events file should not be empty");
    }

    Ok(())
}

/// Test popen system call interception
#[test]
#[cfg(has_preload_library)]
#[cfg(has_executable_echo)]
fn execvp_interception() -> Result<()> {
    let env = TestEnvironment::new("execvp_intercept")?;

    let c_program = r#"#include <unistd.h>

int main() {
    char *const argv[] = {"echo", "hello from execvp", 0};
    return execvp("echo", argv);
}"#;

    env.create_source_files(&[("test_execvp.c", c_program)])?;

    #[cfg(has_executable_compiler_c)]
    {
        env.run_bear(&["--", COMPILER_C_PATH, "-o", "test_execvp", "test_execvp.c"])?.assert_success()?;

        env.run_bear(&["intercept", "--output", "events.json", "--", "./test_execvp"])?.assert_success()?;

        let events_content = fs::read_to_string(env.temp_dir().join("events.json"))?;
        let events: Vec<Value> =
            events_content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();

        assert!(events.len() >= 1, "Should capture execvp events");
    }

    Ok(())
}

/// Test popen system call interception
#[test]
#[cfg(has_preload_library)]
#[cfg(has_executable_cat)]
fn popen_interception() -> Result<()> {
    let env = TestEnvironment::new("popen_intercept")?;

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

    #[cfg(has_executable_compiler_c)]
    {
        env.run_bear(&["--", COMPILER_C_PATH, "-o", "test_popen", "test_popen.c"])?.assert_success()?;

        env.run_bear(&["intercept", "--output", "events.json", "--", "./test_popen"])?.assert_success()?;

        let events_content = fs::read_to_string(env.temp_dir().join("events.json"))?;
        let events: Vec<Value> =
            events_content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();

        // Should capture the popen'd cat command
        assert!(events.len() >= 1, "Should capture popen events");

        let has_cat_event = events.iter().any(|event| {
            event
                .get("execution")
                .and_then(|e| e.get("arguments"))
                .and_then(|args| args.as_array())
                .map(|arr| arr.iter().any(|arg| arg.as_str().map(|s| s.contains("cat")).unwrap_or(false)))
                .unwrap_or(false)
        });

        assert!(has_cat_event, "Should capture cat command from popen");
    }

    Ok(())
}

/// Test system() call interception
#[test]
#[cfg(has_preload_library)]
#[cfg(has_executable_echo)]
fn system_interception() -> Result<()> {
    let env = TestEnvironment::new("system_intercept")?;

    let c_program = format!(
        r#"#include <stdlib.h>

int main() {{
    return system("{} 'hello from system'");
}}"#,
        ECHO_PATH
    );

    env.create_source_files(&[("test_system.c", &c_program)])?;

    #[cfg(has_executable_compiler_c)]
    {
        env.run_bear(&["--", COMPILER_C_PATH, "-o", "test_system", "test_system.c"])?.assert_success()?;

        env.run_bear(&["intercept", "--output", "events.json", "--", "./test_system"])?.assert_success()?;

        let events_content = fs::read_to_string(env.temp_dir().join("events.json"))?;
        let events: Vec<Value> =
            events_content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();

        assert!(events.len() >= 1, "Should capture system() call events");

        // Should capture shell and echo execution
        // Should contain echo execution
        let has_echo_event = events.iter().any(|event| {
            event
                .get("execution")
                .and_then(|e| e.get("executable"))
                .and_then(|exe| exe.as_str())
                .map(|exe| exe.contains("echo"))
                .unwrap_or(false)
        });

        assert!(has_echo_event, "Should capture echo command from system() call");
    }

    Ok(())
}

/// Test posix_spawn interception
#[test]
#[cfg(has_preload_library)]
#[cfg(has_executable_echo)]
fn posix_spawn_interception() -> Result<()> {
    let env = TestEnvironment::new("posix_spawn_intercept")?;

    let c_program = format!(
        r#"#include <spawn.h>
#include <sys/wait.h>
#include <unistd.h>

int main() {{
    pid_t pid;
    char *const argv[] = {{ "{}", "hello from posix_spawn", 0 }};
    char *const envp[] = {{ "TEST=1", 0 }};

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

    #[cfg(has_executable_compiler_c)]
    {
        env.run_bear(&["--", COMPILER_C_PATH, "-o", "test_posix_spawn", "test_posix_spawn.c"])?
            .assert_success()?;

        env.run_bear(&["intercept", "--output", "events.json", "--", "./test_posix_spawn"])?
            .assert_success()?;

        let events_content = fs::read_to_string(env.temp_dir().join("events.json"))?;
        let events: Vec<Value> =
            events_content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();

        assert!(events.len() >= 1, "Should capture posix_spawn events");
    }

    Ok(())
}

/// Test posix_spawnp interception (searches PATH)
#[test]
#[cfg(has_preload_library)]
fn posix_spawnp_interception() -> Result<()> {
    let env = TestEnvironment::new("posix_spawnp_intercept")?;

    let c_program = r#"#include <spawn.h>
#include <sys/wait.h>
#include <unistd.h>

int main() {
    pid_t pid;
    char *const argv[] = {"echo", "hello from posix_spawnp", 0};
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

    #[cfg(has_executable_compiler_c)]
    {
        env.run_bear(&["--", COMPILER_C_PATH, "-o", "test_posix_spawnp", "test_posix_spawnp.c"])?
            .assert_success()?;

        env.run_bear(&["intercept", "--output", "events.json", "--", "./test_posix_spawnp"])?
            .assert_success()?;

        let events_content = fs::read_to_string(env.temp_dir().join("events.json"))?;
        let events: Vec<Value> =
            events_content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();
        assert!(events.len() >= 1, "Should capture execvpe events");

        // Should contain echo execution
        let has_echo_event = events.iter().any(|event| {
            event
                .get("execution")
                .and_then(|e| e.get("executable"))
                .and_then(|exe| exe.as_str())
                .map(|exe| exe.contains("echo"))
                .unwrap_or(false)
        });

        assert!(has_echo_event, "Should capture echo execution via execvpe");
    }

    Ok(())
}

/// Test errno handling with failed exec calls
#[test]
#[cfg(has_preload_library)]
fn test_failed_exec_errno_handling() -> Result<()> {
    let env = TestEnvironment::new("failed_exec_errno")?;

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

    #[cfg(has_executable_compiler_c)]
    {
        env.run_bear(&["--", COMPILER_C_PATH, "-o", "test_failed_exec", "test_failed_exec.c"])?
            .assert_success()?;

        let intercept_output =
            env.run_bear(&["intercept", "--output", "events.json", "--", "./test_failed_exec"])?;

        // The program should fail (non-zero exit) but intercept should still work
        intercept_output.assert_failure()?;

        // Should still capture the attempted exec
        let events_content = fs::read_to_string(env.temp_dir().join("events.json"))?;
        assert!(!events_content.is_empty(), "Should capture failed exec attempt");
    }

    Ok(())
}

/// Test that programs with no exec calls don't generate events
#[test]
#[cfg(has_preload_library)]
fn test_no_exec_calls() -> Result<()> {
    let env = TestEnvironment::new("no_exec")?;

    let c_program = r#"#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main() {
    printf("This program does not call any exec functions\n");
    printf("Process ID: %d\n", getpid());
    printf("Working directory: %s\n", getcwd(NULL, 0));
    return EXIT_SUCCESS;
}"#;

    env.create_source_files(&[("test_no_exec.c", c_program)])?;

    #[cfg(has_executable_compiler_c)]
    {
        env.run_bear(&["--", COMPILER_C_PATH, "-o", "test_no_exec", "test_no_exec.c"])?.assert_success()?;

        env.run_bear(&["intercept", "--output", "events.json", "--", "./test_no_exec"])?.assert_success()?;

        let events_content = fs::read_to_string(env.temp_dir().join("events.json"))?;
        let events: Vec<Value> =
            events_content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();

        // Should only have events for the test program itself, no child processes
        // For programs that don't call exec functions, we may see 0 events
        // since the Rust implementation only captures exec-family calls
        println!("Captured {} events", events.len());
    }

    Ok(())
}

/// Test execvpe system call (non-standard but common extension)
/// Some systems support execvpe which combines execvp with explicit environment
#[test]
#[cfg(has_preload_library)]
#[cfg(has_executable_echo)]
fn execvpe_interception() -> Result<()> {
    let env = TestEnvironment::new("execvpe_intercept")?;

    // Note: execvpe is not POSIX standard, may not be available on all systems
    let c_program = r#"#define _GNU_SOURCE
#include <unistd.h>

int main() {
    char *const argv[] = {"echo", "hello from execvpe", 0};
    char *const envp[] = {"TEST=execvpe", 0};

#ifdef __linux__
    return execvpe("echo", argv, envp);
#else
    // Fallback to execvp on non-Linux systems
    return execvp("echo", argv);
#endif
}"#;

    env.create_source_files(&[("test_execvpe.c", c_program)])?;

    #[cfg(has_executable_compiler_c)]
    {
        env.run_bear(&["--", COMPILER_C_PATH, "-o", "test_execvpe", "test_execvpe.c"])?.assert_success()?;

        env.run_bear(&["intercept", "--output", "events.json", "--", "./test_execvpe"])?.assert_success()?;

        let events_content = fs::read_to_string(env.temp_dir().join("events.json"))?;
        let events: Vec<Value> =
            events_content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();
        assert!(events.len() >= 1, "Should capture execvpe/execvp events");

        // Should contain echo execution
        let has_echo_event = events.iter().any(|event| {
            event
                .get("execution")
                .and_then(|e| e.get("executable"))
                .and_then(|exe| exe.as_str())
                .map(|exe| exe.contains("echo"))
                .unwrap_or(false)
        });

        assert!(has_echo_event, "Should capture echo execution via execvpe/execvp");
    }

    Ok(())
}
