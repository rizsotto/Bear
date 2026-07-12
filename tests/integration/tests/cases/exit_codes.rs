// SPDX-License-Identifier: GPL-3.0-or-later

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::TestEnvironment;
#[cfg(all(target_family = "unix", has_executable_compiler_c, has_executable_shell))]
use crate::fixtures::infrastructure::filename_of;
use anyhow::Result;
#[cfg(has_executable_sleep)]
use std::io::Read;
#[cfg(has_executable_sleep)]
use std::process::Stdio;
#[cfg(has_executable_sleep)]
use std::time::Instant;

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

#[test]
fn exit_code_for_non_existing_command() -> Result<()> {
    // Executing a non-existing command should always has non-zero exit code,
    // and print relevant information about the reason about the failure.
    let env = TestEnvironment::new("exit_code_for_non_existing_command")?;

    let result = env.run_bear(&["--", "invalid_command"])?;
    result.assert_failure()?;
    assert!(result.stderr().contains("Build execution failed: Failed to execute"));
    Ok(())
}

// Requirements: interception-signal-forwarding
#[test]
#[cfg(has_executable_true)]
fn exit_code_for_true() -> Result<()> {
    // When the executed command returns successfully, Bear exit code should be zero.
    let env = TestEnvironment::new("exit_code_for_true")?;

    let result = env.run_bear(&["--", TRUE_PATH])?;
    result.assert_success()?;
    Ok(())
}

// Requirements: interception-signal-forwarding
#[test]
#[cfg(has_executable_false)]
fn exit_code_for_false() -> Result<()> {
    // When the executed command returns unsuccessfully, Bear exit code should be non-zero.
    let env = TestEnvironment::new("exit_code_for_false")?;

    let result = env.run_bear(&["--", FALSE_PATH])?;
    result.assert_failure()?;
    Ok(())
}

// Requirements: interception-signal-forwarding
#[test]
#[cfg(has_executable_sleep)]
fn exit_code_when_signaled() -> Result<()> {
    // When the bear process is signaled, Bear exit code should be non-zero.
    // And should terminate the child process and return immediately.
    let env = TestEnvironment::new("exit_code_when_signaled")?;

    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir())
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
// Requirements: interception-events-format
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
// Requirements: interception-signal-forwarding
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
// Requirements: interception-signal-forwarding
#[test]
#[cfg(has_executable_false)]
fn intercept_exit_code_for_failure() -> Result<()> {
    let env = TestEnvironment::new("intercept_exit_code_for_failure")?;

    let result = env.run_bear(&["intercept", "--output", "events.json", "--", FALSE_PATH])?;
    result.assert_failure()?;
    Ok(())
}

/// A compiler that is blocked reading from a FIFO with no writer is in the
/// mid-compile state. Signaling Bear with SIGTERM must stop both Bear and
/// the compiler quickly, with Bear reporting non-success.
// Requirements: interception-signal-forwarding
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn exit_code_when_compiler_is_interrupted_mid_compile() -> Result<()> {
    let env = TestEnvironment::new("exit_code_mid_compile_signal")?;

    // Named pipe as the compiler's input. With no writer, the compiler
    // blocks in read() and stays mid-compile until a signal arrives.
    let fifo = env.test_dir().join("source.c");
    let mkfifo_status = std::process::Command::new("mkfifo").arg(&fifo).status()?;
    assert!(mkfifo_status.success(), "mkfifo failed -- this test needs a POSIX environment");

    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir())
        .args([
            "--output",
            "compile_commands.json",
            "--",
            COMPILER_C_PATH,
            "-x",
            "c",
            "-c",
            fifo.to_str().unwrap(),
            "-o",
            "out.o",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = cmd.spawn().expect("failed to spawn bear");

    // Give the compiler time to start and block on opening/reading the FIFO.
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Send SIGTERM so Bear's signal forwarding path is exercised (unlike
    // Child::kill() which sends SIGKILL and bypasses handlers).
    let signal_time = Instant::now();
    let pid = child.id().to_string();
    let kill_status = std::process::Command::new("kill")
        .arg("-TERM")
        .arg(&pid)
        .status()
        .expect("kill -TERM command failed to run");
    assert!(kill_status.success(), "kill -TERM reported failure");

    let status = child.wait().expect("failed to wait for bear");
    let elapsed = signal_time.elapsed();

    assert!(!status.success(), "bear must report non-success after signal");
    assert!(
        elapsed.as_secs() < 2,
        "bear must exit within ~1s of signal while the compiler was mid-compile, took {:?}",
        elapsed
    );

    Ok(())
}

/// A build whose process tree includes a detached grandchild must be torn
/// down whole: signaling Bear stops not just the direct child but the
/// grandchild the build spawned. This proves process-group teardown end to
/// end through the real driver, not just the killpg mechanism in isolation.
// Requirements: interception-signal-forwarding
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_shell, has_executable_sleep))]
fn signal_tears_down_build_process_tree() -> Result<()> {
    let env = TestEnvironment::new("signal_tears_down_build_tree")?;

    let gpid_file = env.test_dir().join("grandchild.pid");
    // The build forks a long-lived grandchild, records its pid, then blocks.
    let script =
        format!("{sleep} 60 & echo $! > {pid} ; {sleep} 60", sleep = SLEEP_PATH, pid = gpid_file.display());

    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir())
        .args(["--output", "compile_commands.json", "--", SHELL_PATH, "-c", &script])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = cmd.spawn().expect("failed to spawn bear");

    // `kill -0` probes liveness without sending a signal.
    let is_alive = |pid: i32| {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };

    // Wait for the build to start and record the grandchild pid.
    let gpid = {
        let deadline = Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if let Ok(text) = std::fs::read_to_string(&gpid_file)
                && let Ok(pid) = text.trim().parse::<i32>()
            {
                break pid;
            }
            assert!(Instant::now() < deadline, "build never recorded its grandchild pid");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    };
    assert!(is_alive(gpid), "grandchild should be running before the signal");

    // Send SIGTERM to Bear (forwarding path, unlike Child::kill()/SIGKILL).
    let pid = child.id().to_string();
    let kill_status = std::process::Command::new("kill")
        .arg("-TERM")
        .arg(&pid)
        .status()
        .expect("kill -TERM command failed to run");
    assert!(kill_status.success(), "kill -TERM reported failure");

    let status = child.wait().expect("failed to wait for bear");
    assert!(!status.success(), "bear must report non-success after signal");

    // The grandchild must be gone: process-group teardown reached it.
    let deadline = Instant::now() + std::time::Duration::from_secs(2);
    while is_alive(gpid) && Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(!is_alive(gpid), "grandchild survived -- only the direct child was reaped");

    Ok(())
}

/// A grandchild that calls `setsid` starts its own session and process
/// group, leaving the build's process group entirely -- so `killpg` can no
/// longer reach it. On Linux with a usable cgroup, Bear tears down the
/// build's cgroup, which a child cannot leave unprivileged, so the detached
/// grandchild dies too. Where no writable cgroup is available the documented
/// process-group fallback applies and this test skips (the
/// `signal_tears_down_build_process_tree` test covers that path).
// Requirements: interception-signal-forwarding
#[test]
#[cfg(target_os = "linux")]
#[cfg(all(has_executable_shell, has_executable_sleep, has_executable_setsid))]
fn signal_tears_down_cgroup_even_when_grandchild_detaches() -> Result<()> {
    if !cgroup_v2_writable() {
        eprintln!("skipping: no writable cgroup v2 subtree; process-group fallback applies");
        return Ok(());
    }

    let env = TestEnvironment::new("signal_tears_down_cgroup")?;

    let gpid_file = env.test_dir().join("grandchild.pid");
    // `setsid` runs the sleep in a brand-new session/group, so it escapes the
    // build's process group -- only a cgroup kill can still reach it.
    let script = format!(
        "{setsid} {sleep} 60 & echo $! > {pid} ; {sleep} 60",
        setsid = SETSID_PATH,
        sleep = SLEEP_PATH,
        pid = gpid_file.display(),
    );

    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir())
        .args(["--output", "compile_commands.json", "--", SHELL_PATH, "-c", &script])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = cmd.spawn().expect("failed to spawn bear");

    // `kill -0` probes liveness without sending a signal.
    let is_alive = |pid: i32| {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };

    // Wait for the build to start and record the detached grandchild pid.
    let gpid = {
        let deadline = Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if let Ok(text) = std::fs::read_to_string(&gpid_file)
                && let Ok(pid) = text.trim().parse::<i32>()
            {
                break pid;
            }
            assert!(Instant::now() < deadline, "build never recorded its grandchild pid");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    };
    assert!(is_alive(gpid), "detached grandchild should be running before the signal");

    // Send SIGTERM to Bear (forwarding path, unlike Child::kill()/SIGKILL).
    let pid = child.id().to_string();
    let kill_status = std::process::Command::new("kill")
        .arg("-TERM")
        .arg(&pid)
        .status()
        .expect("kill -TERM command failed to run");
    assert!(kill_status.success(), "kill -TERM reported failure");

    let status = child.wait().expect("failed to wait for bear");
    assert!(!status.success(), "bear must report non-success after signal");

    // The detached grandchild must be gone: cgroup teardown reached it even
    // though it left the process group.
    let deadline = Instant::now() + std::time::Duration::from_secs(2);
    while is_alive(gpid) && Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(!is_alive(gpid), "detached grandchild survived -- cgroup teardown did not reach it");

    Ok(())
}

/// Whether a child cgroup can be created under our own cgroup v2 -- the
/// runtime precondition for cgroup-based teardown. Mirrors what `supervise`
/// attempts, so the test runs only where the cgroup path is actually taken.
#[cfg(target_os = "linux")]
fn cgroup_v2_writable() -> bool {
    let Ok(own) = std::fs::read_to_string("/proc/self/cgroup") else {
        return false;
    };
    let Some(rel) = own.lines().find_map(|l| l.strip_prefix("0::")) else {
        return false;
    };
    let dir = std::path::Path::new("/sys/fs/cgroup")
        .join(rel.trim_start_matches('/'))
        .join(format!("bear-test-probe-{}", std::process::id()));
    if std::fs::create_dir(&dir).is_err() {
        return false;
    }
    let ok = dir.join("cgroup.kill").exists();
    let _ = std::fs::remove_dir(&dir);
    ok
}

/// Bear relays the exact signal it received rather than substituting one of
/// its own: a build that traps `SIGINT` and `SIGTERM` differently sees the
/// real one. Each case sends a distinct signal and asserts the matching trap
/// ran (via its marker) and that Bear propagated that trap's exit code.
// Requirements: interception-signal-forwarding
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_shell, has_executable_sleep))]
fn forwards_the_exact_signal_received() -> Result<()> {
    // (signal for `kill`, marker its trap writes, exit code its trap uses)
    let cases = [("INT", "INT", 10), ("TERM", "TERM", 20)];

    for (signal, expected_marker, expected_code) in cases {
        let env = TestEnvironment::new(&format!("signal_fidelity_{signal}"))?;

        let marker = env.test_dir().join("caught.marker");
        let ready = env.test_dir().join("ready");
        // Distinct INT and TERM traps: whichever marker appears reveals which
        // signal Bear actually forwarded. If Bear substituted a signal, the
        // wrong trap would fire.
        let script = format!(
            "trap 'echo INT > {m} ; exit 10' INT ; trap 'echo TERM > {m} ; exit 20' TERM ; echo ready > {r} ; {sleep} 60",
            m = marker.display(),
            r = ready.display(),
            sleep = SLEEP_PATH,
        );

        let mut cmd = env.command_bear();
        cmd.current_dir(env.test_dir())
            .args(["--output", "compile_commands.json", "--", SHELL_PATH, "-c", &script])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = cmd.spawn().expect("failed to spawn bear");

        wait_for_file(&ready);

        let pid = child.id().to_string();
        let kill_status = std::process::Command::new("kill")
            .arg(format!("-{signal}"))
            .arg(&pid)
            .status()
            .expect("kill failed to run");
        assert!(kill_status.success(), "kill -{signal} reported failure");

        let status = child.wait().expect("failed to wait for bear");

        let caught = std::fs::read_to_string(&marker).unwrap_or_default();
        assert_eq!(caught.trim(), expected_marker, "{signal}: build trapped the wrong signal");
        assert_eq!(
            status.code(),
            Some(expected_code),
            "{signal}: bear did not propagate the trap's exit code"
        );
    }

    Ok(())
}

/// Wrapper mode nests supervision: bear-driver -> sh -> bear-wrapper -> real
/// cc. A compiler blocked mid-compile (reading a writer-less FIFO) is under
/// the wrapper's `Inherit` supervision when the signal arrives. Signaling the
/// driver must still tear the whole nested tree down within the budget -- not
/// leave Bear hanging on the blocked compiler -- so the contract holds in
/// wrapper mode just as in preload mode.
// Requirements: interception-signal-forwarding, interception-wrapper-mechanism
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn signal_tears_down_nested_wrapper_supervision() -> Result<()> {
    let env = TestEnvironment::new("signal_wrapper_teardown")?;

    // Named pipe as the compiler's input: with no writer the compiler blocks
    // in read() and stays mid-compile until a signal arrives.
    let fifo = env.test_dir().join("source.c");
    let mkfifo_status = std::process::Command::new("mkfifo").arg(&fifo).status()?;
    assert!(mkfifo_status.success(), "mkfifo failed -- this test needs a POSIX environment");

    // Invoke the compiler via $CC so wrapper mode actually supervises it.
    let build = format!("$CC -x c -c {fifo} -o out.o", fifo = fifo.display());
    let script = env.create_shell_script("build.sh", &build)?;

    let config = "schema: \"4.1\"\n\nintercept:\n  mode: wrapper\n";
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir()).env("CC", filename_of(COMPILER_C_PATH)).args([
        "--config",
        config_path.to_str().unwrap(),
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        script.to_str().unwrap(),
    ]);
    cmd.stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = cmd.spawn().expect("failed to spawn bear");

    // Let the build reach the compiler and the compiler block on the FIFO.
    std::thread::sleep(std::time::Duration::from_millis(800));

    let signal_time = Instant::now();
    let pid = child.id().to_string();
    let kill_status =
        std::process::Command::new("kill").arg("-TERM").arg(&pid).status().expect("kill -TERM failed to run");
    assert!(kill_status.success(), "kill -TERM reported failure");

    let status = child.wait().expect("failed to wait for bear");
    let elapsed = signal_time.elapsed();

    assert!(!status.success(), "bear must report non-success after signaling a wrapper-mode build");
    assert!(
        elapsed.as_secs() < 2,
        "wrapper-mode teardown must reach the blocked compiler within budget, took {elapsed:?}"
    );

    Ok(())
}

/// Block until the supervised build reports it is ready (it creates `path`),
/// so the test signals Bear only once the build is actually running.
#[cfg(all(target_family = "unix", has_executable_shell, has_executable_sleep))]
fn wait_for_file(path: &std::path::Path) {
    let deadline = Instant::now() + std::time::Duration::from_secs(5);
    while !path.exists() {
        assert!(Instant::now() < deadline, "build never created {}", path.display());
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// Bear forwards the real signal (not SIGKILL) and grants a grace window, so
/// a build that traps the signal runs its trap and Bear's exit code reflects
/// whatever the build ultimately exited with.
// Requirements: interception-signal-forwarding
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_shell, has_executable_sleep))]
fn signal_lets_a_trapping_build_run_its_trap() -> Result<()> {
    let env = TestEnvironment::new("signal_trapping_build")?;

    let marker = env.test_dir().join("trap.marker");
    let ready = env.test_dir().join("ready");
    // The build traps TERM, records that the trap ran, and exits 42.
    let script = format!(
        "trap 'echo caught > {marker} ; exit 42' TERM ; echo ready > {ready} ; {sleep} 60",
        marker = marker.display(),
        ready = ready.display(),
        sleep = SLEEP_PATH,
    );

    // Capture bear's stderr (RUST_LOG=debug on CI) so a teardown-timing
    // failure carries the supervisor's own trace - which signal it forwarded
    // and whether it escalated to SIGKILL - instead of a bare assertion.
    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir())
        .env("RUST_BACKTRACE", "1")
        .args(["--output", "compile_commands.json", "--", SHELL_PATH, "-c", &script])
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if std::env::var_os("RUST_LOG").is_none() {
        cmd.env("RUST_LOG", "info");
    }
    let mut child = cmd.spawn().expect("failed to spawn bear");

    // Drain stderr on a thread: with debug logging a full pipe would otherwise
    // block the very build we are trying to tear down. The write ends close
    // when bear and its build tree exit, so the read returns EOF.
    let stderr = child.stderr.take().expect("bear stderr was piped");
    let drain = std::thread::spawn(move || {
        let mut captured = String::new();
        let mut stderr = stderr;
        let _ = stderr.read_to_string(&mut captured);
        captured
    });

    wait_for_file(&ready);

    let pid = child.id().to_string();
    let kill_status =
        std::process::Command::new("kill").arg("-TERM").arg(&pid).status().expect("kill -TERM failed to run");
    assert!(kill_status.success(), "kill -TERM reported failure");

    let status = child.wait().expect("failed to wait for bear");
    let bear_stderr = drain.join().unwrap_or_default();

    // The trap ran (real signal forwarded with grace, not an immediate
    // SIGKILL) and Bear reflected the build's own exit code.
    let caught = std::fs::read_to_string(&marker).unwrap_or_default();
    assert_eq!(caught.trim(), "caught", "build's TERM trap did not run; bear stderr:\n{bear_stderr}");
    assert_eq!(
        status.code(),
        Some(42),
        "bear did not propagate the build's trap exit code; bear stderr:\n{bear_stderr}"
    );
    Ok(())
}

/// A build that ignores the termination signal is still stopped: after the
/// grace window Bear escalates to SIGKILL, so Bear and the build both end
/// within the time budget and Bear reports non-success.
// Requirements: interception-signal-forwarding
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_shell, has_executable_sleep))]
fn signal_escalates_when_build_ignores_it() -> Result<()> {
    let env = TestEnvironment::new("signal_escalation")?;

    let ready = env.test_dir().join("ready");
    // The build ignores TERM and keeps running, forcing the SIGKILL escalation.
    let script = format!(
        "trap '' TERM ; echo ready > {ready} ; while true ; do {sleep} 1 ; done",
        ready = ready.display(),
        sleep = SLEEP_PATH,
    );

    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir())
        .args(["--output", "compile_commands.json", "--", SHELL_PATH, "-c", &script])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = cmd.spawn().expect("failed to spawn bear");

    wait_for_file(&ready);

    let signal_time = Instant::now();
    let pid = child.id().to_string();
    let kill_status =
        std::process::Command::new("kill").arg("-TERM").arg(&pid).status().expect("kill -TERM failed to run");
    assert!(kill_status.success(), "kill -TERM reported failure");

    let status = child.wait().expect("failed to wait for bear");
    let elapsed = signal_time.elapsed();

    assert!(!status.success(), "bear must report non-success after escalating");
    assert!(
        elapsed.as_secs() < 2,
        "bear must stop an unresponsive build within the budget, took {elapsed:?}"
    );
    Ok(())
}

// Semantic mode exit code tests (note: this is now called 'semantic' not 'citnames')

/// Test that semantic command returns 0 for valid input
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

/// Test that semantic command with missing input file returns non-zero
#[test]
fn semantic_exit_code_for_missing_input() -> Result<()> {
    let env = TestEnvironment::new("semantic_exit_code_for_missing_input")?;

    let result =
        env.run_bear(&["semantic", "--input", "nonexistent.json", "--output", "compile_commands.json"])?;
    result.assert_failure()?;
    Ok(())
}
