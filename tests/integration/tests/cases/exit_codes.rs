// SPDX-License-Identifier: GPL-3.0-or-later

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::TestEnvironment;
use anyhow::Result;
#[cfg(has_executable_sleep)]
use std::process::Stdio;
#[cfg(has_executable_sleep)]
use std::time::Instant;

// Requirements: cli-exit-codes
#[test]
fn exit_code_for_empty_arguments() -> Result<()> {
    // Executing Bear with no arguments should return a non-zero exit code,
    // and print usage information.
    let env = TestEnvironment::new("exit_code_for_empty_arguments")?;

    let result = env.run_bear(&[])?;
    result.assert_failure()?;
    assert!(result.stderr().contains("Usage: bear"));
    Ok(())
}

// Requirements: cli-exit-codes
#[test]
fn exit_code_for_help() -> Result<()> {
    // Executing help and subcommand help should always has zero exit code,
    // and print out usage information
    let env = TestEnvironment::new("exit_code_for_help")?;

    // Test main help
    let result = env.run_bear(&["--help"])?;
    result.assert_success()?;
    assert!(result.stdout().contains("Usage: bear"));

    // Test intercept help
    let result = env.run_bear(&["intercept", "--help"])?;
    result.assert_success()?;
    assert!(result.stdout().contains("Usage: bear"));

    // Test semantic help
    let result = env.run_bear(&["semantic", "--help"])?;
    result.assert_success()?;
    assert!(result.stdout().contains("Usage: bear"));

    Ok(())
}

// Requirements: cli-exit-codes
#[test]
fn exit_code_for_invalid_argument() -> Result<()> {
    // Executing Bear with an invalid argument should always has non-zero exit code,
    // and print relevant information about the reason about the failure.
    let env = TestEnvironment::new("exit_code_for_invalid_argument")?;

    let result = env.run_bear(&["invalid_argument"])?;
    result.assert_failure()?;
    assert!(result.stderr().contains("error: unexpected argument"));
    Ok(())
}

// Requirements: cli-exit-codes
#[test]
fn exit_code_for_non_existing_command() -> Result<()> {
    // Executing a non-existing command should always has non-zero exit code,
    // and print relevant information about the reason about the failure.
    let env = TestEnvironment::new("exit_code_for_non_existing_command")?;

    let result = env.run_bear(&["--", "invalid_command"])?;
    result.assert_failure()?;
    assert!(result.stderr().contains("cannot run 'invalid_command': "));
    Ok(())
}

// Requirements: cli-exit-codes
#[test]
#[cfg(has_executable_true)]
fn exit_code_for_true() -> Result<()> {
    // When the executed command returns successfully, Bear exit code should be zero.
    let env = TestEnvironment::new("exit_code_for_true")?;

    let result = env.run_bear(&["--", TRUE_PATH])?;
    result.assert_success()?;
    Ok(())
}

// Requirements: cli-exit-codes
#[test]
#[cfg(has_executable_false)]
fn exit_code_for_false() -> Result<()> {
    // When the executed command returns unsuccessfully, Bear exit code should be non-zero.
    let env = TestEnvironment::new("exit_code_for_false")?;

    let result = env.run_bear(&["--", FALSE_PATH])?;
    result.assert_failure()?;
    Ok(())
}

/// The build's exit code is propagated byte-for-byte, not collapsed to a
/// generic failure: a build that exits 42 makes Bear exit exactly 42, in
/// both the combined and the intercept build-running modes.
// Requirements: cli-exit-codes
#[test]
#[cfg(has_executable_shell)]
fn exit_code_propagates_exact_build_code() -> Result<()> {
    let env = TestEnvironment::new("exit_code_exact_42")?;

    let result = env.run_bear(&["--output", "compile_commands.json", "--", SHELL_PATH, "-c", "exit 42"])?;
    assert_eq!(result.exit_code(), Some(42), "combined mode must propagate the build's exact exit code");

    let result =
        env.run_bear(&["intercept", "--output", "events.json", "--", SHELL_PATH, "-c", "exit 42"])?;
    assert_eq!(result.exit_code(), Some(42), "intercept mode must propagate the build's exact exit code");

    Ok(())
}

/// Windows exit codes are 32 bits wide, so Bear passes the build's code on whole
/// instead of cutting it to 8 bits.
// Requirements: cli-exit-codes
#[test]
#[cfg(windows)]
fn exit_code_above_255_passes_through_on_windows() -> Result<()> {
    let env = TestEnvironment::new("exit_code_above_255")?;

    let combined = env.run_bear(&["--output", "compile_commands.json", "--", "cmd", "/c", "exit 300"])?;
    let intercept = env.run_bear(&["intercept", "--output", "events.json", "--", "cmd", "/c", "exit 300"])?;

    assert_eq!(combined.exit_code(), Some(300), "combined mode");
    assert_eq!(intercept.exit_code(), Some(300), "intercept mode");
    Ok(())
}

/// A build-running mode whose every candidate entry is dropped at output
/// validation still exits with the build's exit code: an `--append` run
/// feeds the existing database's entries back through output validation, so
/// a pre-seeded entry with an empty directory (modeled on the all-dropped
/// semantic test in config.rs) is the sole candidate and is rejected -- yet
/// the build succeeded, so Bear exits 0 and the database is written empty.
/// Data-quality outcomes must never change the exit code.
// Requirements: cli-exit-codes
#[test]
#[cfg(has_executable_true)]
fn build_exit_code_is_preserved_when_every_entry_is_dropped_at_output() -> Result<()> {
    let env = TestEnvironment::new("exit_code_all_entries_dropped")?;

    // A well-formed database whose only entry fails validation (empty
    // directory field), so it is rejected when --append reads it back.
    let invalid_entry = r#"[{"directory": "", "file": "main.c", "arguments": ["gcc", "-c", "main.c"]}]"#;
    env.create_source_files(&[("compile_commands.json", invalid_entry)])?;

    let output = env.run_bear(&["--append", "--output", "compile_commands.json", "--", TRUE_PATH])?;

    // The build exited 0, so Bear must exit 0 despite the empty database.
    output.assert_success()?;

    // The candidate entry existed and was rejected at output validation.
    assert!(
        output.stderr().contains("Entry has an empty directory field"),
        "expected the validation rejection to be visible in stderr: {}",
        output.stderr()
    );

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(0)?;

    Ok(())
}

// Requirements: cli-signal-forwarding, cli-exit-codes
#[test]
#[cfg(has_executable_sleep)]
fn exit_code_when_signaled() -> Result<()> {
    // When the bear process is signaled, Bear exit code should be non-zero.
    // And should terminate the child process and return immediately.
    let env = TestEnvironment::new("exit_code_when_signaled")?;

    // SIGKILL leaves the socket directory behind; keep it inside the test directory.
    let runtime = env.test_dir().join("runtime");
    std::fs::create_dir(&runtime)?;
    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir())
        .env("XDG_RUNTIME_DIR", &runtime)
        .arg("--")
        .arg(SLEEP_PATH)
        .arg("10")
        .env("RUST_LOG", "debug")
        .env("RUST_BACKTRACE", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().expect("Failed to spawn command");

    // Wait 200ms to ensure that the sleep command was also executed
    std::thread::sleep(std::time::Duration::from_millis(200));

    let kill_time = Instant::now();
    child.kill().expect("Failed to signal the process");
    let status = child.wait().expect("Failed to wait for command");
    let wait_end = Instant::now();

    assert!(!status.success());
    assert!(wait_end.duration_since(kill_time).as_secs() < 1, "Process took too long to terminate.",);
    Ok(())
}

/// The combined mode (no subcommand) runs the build too, so it must reject
/// `--output -` for the same reason `bear intercept --output -` does: the
/// build's own stdout shares the stream and a non-atomic write could split
/// a JSON line.
// Requirements: output-json-compilation-database
#[test]
#[cfg(has_executable_true)]
fn combined_output_dash_is_rejected() -> Result<()> {
    let env = TestEnvironment::new("combined_output_dash_is_rejected")?;

    let result = env.run_bear(&["--output", "-", "--", TRUE_PATH])?;
    result.assert_failure()?;
    assert!(
        result.stderr().contains("cannot write the compilation database to stdout"),
        "stderr did not explain the rejection: {}",
        result.stderr()
    );
    assert!(!env.file_exists("-"), "must not create a file literally named `-`");
    Ok(())
}

// Intercept mode exit code tests

/// Test that intercept command returns 0 for successful interception
// Requirements: cli-exit-codes
#[test]
#[cfg(has_executable_true)]
fn intercept_exit_code_for_success() -> Result<()> {
    let env = TestEnvironment::new("intercept_exit_code_for_success")?;

    let result = env.run_bear(&["intercept", "--output", "events.json", "--", TRUE_PATH])?;
    result.assert_success()?;
    Ok(())
}

/// Interception has no default event-file name: omitting the output
/// destination is a usage error, reported before the build runs.
// Requirements: interception-events-format
#[test]
#[cfg(has_executable_true)]
fn intercept_exit_code_without_output() -> Result<()> {
    let env = TestEnvironment::new("intercept_exit_code_without_output")?;

    let result = env.run_bear(&["intercept", "--", TRUE_PATH])?;
    result.assert_failure()?;
    assert!(
        result.stderr().contains("--output"),
        "the usage error must name the missing option: {}",
        result.stderr()
    );
    Ok(())
}

/// Test that intercept command propagates command failure exit codes
// Requirements: cli-exit-codes
#[test]
#[cfg(has_executable_false)]
fn intercept_exit_code_for_failure() -> Result<()> {
    let env = TestEnvironment::new("intercept_exit_code_for_failure")?;

    let result = env.run_bear(&["intercept", "--output", "events.json", "--", FALSE_PATH])?;
    result.assert_failure()?;
    Ok(())
}

// Semantic mode exit code tests (note: this is now called 'semantic' not 'citnames')

/// Test that semantic command returns 0 for valid input
// Requirements: cli-exit-codes
#[test]
fn semantic_exit_code_for_success() -> Result<()> {
    let env = TestEnvironment::new("semantic_exit_code_for_success")?;

    // Create a sample events file
    let events_content =
        r#"{"executable":"/usr/bin/gcc","arguments":["-c","test.c"],"working_dir":"/tmp","environment":{}}"#;
    env.create_source_files(&[("events.json", events_content)])?;

    let result =
        env.run_bear(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;
    result.assert_success()?;
    Ok(())
}

/// `bear semantic` on input whose events are all understood but name only
/// non-compiler commands exits 0 and writes an empty database: the filter
/// did its job, and an empty result is not a failure.
// Requirements: cli-exit-codes
#[test]
fn semantic_exit_code_for_understood_non_compiler_events() -> Result<()> {
    let env = TestEnvironment::new("semantic_exit_code_non_compiler")?;

    // Both events conform to the format; neither names a compiler.
    let events_content = [
        r#"{"executable":"ls","arguments":["ls","-la"],"working_dir":"/tmp","environment":{}}"#,
        r#"{"executable":"echo","arguments":["echo","done"],"working_dir":"/tmp","environment":{}}"#,
    ]
    .join("\n");
    env.create_source_files(&[("events.json", &events_content)])?;

    let result =
        env.run_bear(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;
    result.assert_success()?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(0)?;

    Ok(())
}

/// Test that semantic command with missing input file returns non-zero
// Requirements: cli-exit-codes
#[test]
fn semantic_exit_code_for_missing_input() -> Result<()> {
    let env = TestEnvironment::new("semantic_exit_code_for_missing_input")?;

    let result =
        env.run_bear(&["semantic", "--input", "nonexistent.json", "--output", "compile_commands.json"])?;
    result.assert_failure()?;
    Ok(())
}

// Requirements: cli-input-from-another-machine, cli-exit-codes
#[test]
fn no_local_lookups_is_an_unknown_argument_where_the_build_runs() -> Result<()> {
    let cases: [&[&str]; 2] = [
        &["--no-local-lookups", "--", "true"],
        &["intercept", "--no-local-lookups", "-o", "events.json", "--", "true"],
    ];

    for args in cases {
        let env = TestEnvironment::new("no_local_lookups_unknown")?;

        let sut = env.run_bear(args)?;

        sut.assert_failure()?;
        assert!(sut.stderr().contains("unexpected argument"), "case {args:?}: {}", sut.stderr());
    }

    Ok(())
}

/// A build that kills itself by a signal ends Bear by the same signal. The
/// shell that ran Bear then sees the signal the plain build would show.
// Requirements: cli-exit-codes
#[test]
#[cfg(target_family = "unix")]
#[cfg(has_executable_shell)]
fn bear_ends_by_the_signal_that_killed_the_build() -> Result<()> {
    use std::os::unix::process::ExitStatusExt;

    for (name, signal) in [("TERM", libc::SIGTERM), ("INT", libc::SIGINT), ("KILL", libc::SIGKILL)] {
        let env = TestEnvironment::new(&format!("exit_by_signal_{name}"))?;
        let script = format!("kill -{name} $$");
        // SIGKILL leaves the socket directory behind; keep it inside the test directory.
        let runtime = env.test_dir().join("runtime");
        std::fs::create_dir(&runtime)?;

        let sut = env
            .command_bear()
            .current_dir(env.test_dir())
            .env("XDG_RUNTIME_DIR", &runtime)
            .args(["--output", "compile_commands.json", "--", SHELL_PATH, "-c", &script])
            .status()?;

        assert_eq!(sut.signal(), Some(signal), "{name}: wait status {sut:?}");
    }
    Ok(())
}

/// SEGV dumps core for the plain build. Bear turns its own core dump off,
/// so the wait status has no core flag.
// Requirements: cli-exit-codes
#[test]
#[cfg(target_family = "unix")]
#[cfg(has_executable_shell)]
fn bear_ends_by_segv_without_a_core_dump() -> Result<()> {
    use std::os::unix::process::ExitStatusExt;

    let env = TestEnvironment::new("exit_by_segv")?;

    let sut = env
        .command_bear()
        .current_dir(env.test_dir())
        .args(["--output", "compile_commands.json", "--", SHELL_PATH, "-c", "kill -SEGV $$"])
        .status()?;

    assert_eq!(sut.signal(), Some(libc::SIGSEGV), "wait status {sut:?}");
    assert!(!sut.core_dumped(), "Bear must not dump core: {sut:?}");
    Ok(())
}

/// In wrapper mode the wrapper ends by the compiler's signal, so the build
/// script sees the shell's `128 + N` code.
// Requirements: cli-exit-codes, interception-wrapper-mechanism
#[test]
#[cfg(target_family = "unix")]
#[cfg(has_executable_shell)]
fn wrapper_ends_by_the_signal_that_killed_the_compiler() -> Result<()> {
    let env = TestEnvironment::new("wrapper_exit_by_signal")?;
    let compiler = env.create_shell_script("fake-cc", "kill -TERM $$\n")?;
    let script = env.create_shell_script("build.sh", "\"$CC\" -c test.c\necho $? > code.txt\n")?;
    let config = env.create_config("schema: \"4.2\"\n\nintercept:\n  mode: wrapper\n")?;

    let sut = env
        .command_bear()
        .current_dir(env.test_dir())
        .env("CC", &compiler)
        .args([
            "--config",
            config.to_str().unwrap(),
            "--output",
            "compile_commands.json",
            "--",
            SHELL_PATH,
            script.to_str().unwrap(),
        ])
        .status()?;

    assert!(sut.success(), "the build script ends with echo: {sut:?}");
    assert_eq!(env.read_file("code.txt")?.trim(), "143");
    Ok(())
}
