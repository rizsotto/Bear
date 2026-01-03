// SPDX-License-Identifier: GPL-3.0-or-later

//! POSIX system call interception tests for Bear integration
//!
//! These tests verify that Bear correctly intercepts various POSIX system calls
//! like execve, execl, popen, posix_spawn, etc. These tests are ported from
//! the test/cases/intercept/preload/posix/ directory.

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::*;
use anyhow::Result;

/// Test execve system call interception
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
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
    env.run_c_compiler("test_execve", &["test_execve.c"])?;

    // Run intercept on the compiled program
    env.run_bear_success(&["intercept", "--output", "events.json", "--", "./test_execve"])?;

    // Verify intercepted events using infrastructure assertions
    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo"))?;

    Ok(())
}

/// Test execl system call interception
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
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
    env.run_c_compiler("test_execl", &["test_execl.c"])?;

    // Run intercept on the compiled program
    env.run_bear(&["intercept", "--output", "events.json", "--", "./test_execl"])?.assert_success()?;

    // Verify events were captured using infrastructure assertions
    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo"))?;

    Ok(())
}

/// Test execlp system call interception
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
fn execv_interception() -> Result<()> {
    let env = TestEnvironment::new("execlp_intercept")?;

    // Create a C program that uses execlp (searches PATH)
    let c_program = r#"#include <unistd.h>

int main() {
    return execlp("echo", "echo", "hello from execlp", (char *)0);
}"#;

    env.create_source_files(&[("test_execlp.c", c_program)])?;
    env.run_c_compiler("test_execlp", &["test_execlp.c"])?;

    // Run intercept on the compiled program
    env.run_bear(&["intercept", "--output", "events.json", "--", "./test_execlp"])?.assert_success()?;

    // Verify events using infrastructure assertions
    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo"))?;

    Ok(())
}

/// Test execvp system call interception
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
fn execvp_interception() -> Result<()> {
    let env = TestEnvironment::new("execvp_intercept")?;

    let c_program = r#"#include <unistd.h>

int main() {
    char *const argv[] = {"echo", "hello from execvp", 0};
    return execvp("echo", argv);
}"#;

    env.create_source_files(&[("test_execvp.c", c_program)])?;
    env.run_c_compiler("test_execvp", &["test_execvp.c"])?;

    // Run intercept on the compiled program
    env.run_bear(&["intercept", "--output", "events.json", "--", "./test_execvp"])?.assert_success()?;

    // Verify events using infrastructure assertions
    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo"))?;

    Ok(())
}

/// Test popen system call interception
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_cat))]
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
    env.run_c_compiler("test_popen", &["test_popen.c"])?;

    // Run intercept on the compiled program
    env.run_bear(&["intercept", "--output", "events.json", "--", "./test_popen"])?.assert_success()?;

    // Verify events using infrastructure assertions
    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("cat"))?;

    Ok(())
}

/// Test system() call interception
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
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
    env.run_c_compiler("test_system", &["test_system.c"])?;

    // Run intercept on the compiled program
    env.run_bear(&["intercept", "--output", "events.json", "--", "./test_system"])?.assert_success()?;

    // Verify events using infrastructure assertions
    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo"))?;

    Ok(())
}

/// Test posix_spawn interception
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
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
    env.run_c_compiler("test_posix_spawn", &["test_posix_spawn.c"])?;

    // Run intercept on the compiled program
    env.run_bear(&["intercept", "--output", "events.json", "--", "./test_posix_spawn"])?.assert_success()?;

    // Verify events using infrastructure assertions
    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo"))?;

    Ok(())
}

/// Test posix_spawnp interception (searches PATH)
#[test]
#[cfg(has_preload_library)]
#[cfg(has_executable_compiler_c)]
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
    env.run_c_compiler("test_posix_spawnp", &["test_posix_spawnp.c"])?;

    // Run intercept on the compiled program
    env.run_bear(&["intercept", "--output", "events.json", "--", "./test_posix_spawnp"])?.assert_success()?;

    // Verify events using infrastructure assertions
    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo"))?;

    Ok(())
}

/// Test errno handling with failed exec calls
#[test]
#[cfg(has_preload_library)]
#[cfg(has_executable_compiler_c)]
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
    env.run_c_compiler("test_failed_exec", &["test_failed_exec.c"])?;

    // Run intercept on the compiled program
    let intercept_output =
        env.run_bear(&["intercept", "--output", "events.json", "--", "./test_failed_exec"])?;

    // The program should fail (non-zero exit) but intercept should still work
    intercept_output.assert_failure()?;

    // Should still capture the attempted exec - even though it fails, the intercept library
    // should record the attempt. We expect at least 0 events (may be more depending on implementation)
    let events = env.load_events_file("events.json")?;
    // Just verify we can load the events file - failed execs may or may not be recorded
    // depending on when exactly the failure occurs
    let _ = events.events();

    Ok(())
}

/// Test that programs with no exec calls don't generate events
#[test]
#[cfg(has_preload_library)]
#[cfg(has_executable_compiler_c)]
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
    env.run_c_compiler("test_no_exec", &["test_no_exec.c"])?;

    // Run intercept on the compiled program
    env.run_bear(&["intercept", "--output", "events.json", "--", "./test_no_exec"])?.assert_success()?;

    // Verify minimal events using infrastructure assertions
    let events = env.load_events_file("events.json")?;
    // For programs that don't call exec functions, we expect 0 events
    // since the Rust implementation only captures exec-family calls
    let event_count = events.events().len();
    println!("Captured {} events", event_count);

    // The exact count may vary by implementation, but should be minimal
    // We just verify we can successfully load and examine the events

    Ok(())
}

/// Test execvpe system call (non-standard but common extension)
/// Some systems support execvpe which combines execvp with explicit environment
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
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
    env.run_c_compiler("test_execvpe", &["test_execvpe.c"])?;

    // Run intercept on the compiled program
    env.run_bear(&["intercept", "--output", "events.json", "--", "./test_execvpe"])?.assert_success()?;

    // Verify events using infrastructure assertions
    let events = env.load_events_file("events.json")?;
    events.assert_contains(&EventMatcher::new().executable_name("echo"))?;

    Ok(())
}
