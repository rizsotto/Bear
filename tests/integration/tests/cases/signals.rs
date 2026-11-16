// SPDX-License-Identifier: GPL-3.0-or-later

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::TestEnvironment;
#[cfg(all(target_family = "unix", has_executable_compiler_c, has_executable_shell))]
use crate::fixtures::infrastructure::filename_of;
use anyhow::Result;
#[cfg(all(target_family = "unix", has_executable_shell, has_executable_sleep))]
use std::io::Read;
#[cfg(all(target_family = "unix", has_executable_shell, has_executable_sleep))]
use std::process::Stdio;
#[cfg(all(target_family = "unix", has_executable_shell, has_executable_sleep))]
use std::time::Instant;

/// A compiler that is blocked reading from a FIFO with no writer is in the
/// mid-compile state. Signaling Bear with SIGTERM must stop both Bear and
/// the compiler quickly, with Bear reporting non-success.
// Requirements: cli-signal-forwarding
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
// Requirements: cli-signal-forwarding
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

/// Bear relays the exact signal it received rather than substituting one of
/// its own: a build that traps `SIGINT` and `SIGTERM` differently sees the
/// real one. Each case sends a distinct signal and asserts the matching trap
/// ran (via its marker) and that Bear propagated that trap's exit code.
// Requirements: cli-signal-forwarding
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_shell, has_executable_sleep))]
fn forwards_the_exact_signal_received() -> Result<()> {
    // (signal for `kill`, marker its trap writes, exit code its trap uses)
    let cases = [("INT", "INT", 10), ("TERM", "TERM", 20)];
    // Distinct INT and TERM traps: whichever marker appears reveals which
    // signal Bear actually forwarded. If Bear substituted a signal, the
    // wrong trap would fire.
    let traps = [("INT", "INT", 10), ("TERM", "TERM", 20)];

    for (signal, expected_marker, expected_code) in cases {
        let (caught, code, bear_stderr) =
            run_trapping_build(&format!("signal_fidelity_{signal}"), &traps, signal)?;

        assert_eq!(
            caught, expected_marker,
            "{signal}: build trapped the wrong signal; bear stderr:\n{bear_stderr}"
        );
        assert_eq!(
            code,
            Some(expected_code),
            "{signal}: bear did not propagate the trap's exit code; bear stderr:\n{bear_stderr}"
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
// Requirements: cli-signal-forwarding, interception-wrapper-mechanism
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

    let config = "schema: \"4.2\"\n\nintercept:\n  mode: wrapper\n";
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

/// Whether `pid` is still running. `kill -0` probes without sending a
/// signal, but it also succeeds on a zombie. In a container whose PID 1
/// does not reap orphans (GitHub Actions container jobs), a killed
/// grandchild stays a zombie forever, so on Linux a zombie counts as dead.
#[cfg(all(target_family = "unix", has_executable_shell, has_executable_sleep))]
fn is_alive(pid: i32) -> bool {
    #[cfg(target_os = "linux")]
    if let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        && let Some((_, rest)) = stat.rsplit_once(')')
        && rest.trim_start().starts_with('Z')
    {
        return false;
    }
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
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

/// One attempt of a trapping-build signal scenario: run a shell script that
/// installs `traps` (signal name, marker text, exit code) and blocks in a
/// long sleep, wait until the script reports ready, send `signal` to bear,
/// and return the trap marker's content, bear's exit code, and bear's
/// captured stderr (debug level, so the supervisor's trace is in it).
#[cfg(all(target_family = "unix", has_executable_shell, has_executable_sleep))]
fn run_trapping_build(
    env_name: &str,
    traps: &[(&str, &str, i32)],
    signal: &str,
) -> Result<(String, Option<i32>, String)> {
    let env = TestEnvironment::new(env_name)?;

    let marker = env.test_dir().join("trap.marker");
    let ready = env.test_dir().join("ready");
    let mut script = String::new();
    for (sig, text, code) in traps {
        script.push_str(&format!("trap 'echo {text} > {m} ; exit {code}' {sig} ; ", m = marker.display()));
    }
    script.push_str(&format!("echo ready > {r} ; {sleep} 60", r = ready.display(), sleep = SLEEP_PATH));

    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir())
        .env("RUST_BACKTRACE", "1")
        .env("RUST_LOG", "debug")
        .args(["--output", "compile_commands.json", "--", SHELL_PATH, "-c", &script])
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("failed to spawn bear");

    // Drain stderr on a thread: with debug logging a full pipe would
    // otherwise block the very build we are trying to tear down. The write
    // ends close when bear and its build tree exit, so the read returns EOF.
    let stderr = child.stderr.take().expect("bear stderr was piped");
    let drain = std::thread::spawn(move || {
        let mut captured = String::new();
        let mut stderr = stderr;
        let _ = stderr.read_to_string(&mut captured);
        captured
    });

    wait_for_file(&ready);

    let pid = child.id().to_string();
    let kill_status = std::process::Command::new("kill")
        .arg(format!("-{signal}"))
        .arg(&pid)
        .status()
        .expect("kill failed to run");
    assert!(kill_status.success(), "kill -{signal} reported failure");

    let status = child.wait().expect("failed to wait for bear");
    let bear_stderr = drain.join().unwrap_or_default();

    let caught = std::fs::read_to_string(&marker).unwrap_or_default();
    Ok((caught.trim().to_string(), status.code(), bear_stderr))
}

/// Bear forwards the real signal, so a build that traps it runs its trap and
/// Bear's exit code reflects whatever the build ultimately exited with.
// Requirements: cli-signal-forwarding
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_shell, has_executable_sleep))]
fn signal_lets_a_trapping_build_run_its_trap() -> Result<()> {
    // The build traps TERM, records that the trap ran, and exits 42.
    let (caught, code, bear_stderr) =
        run_trapping_build("signal_trapping_build", &[("TERM", "caught", 42)], "TERM")?;

    assert_eq!(caught, "caught", "build's TERM trap did not run; bear stderr:\n{bear_stderr}");
    assert_eq!(
        code,
        Some(42),
        "bear did not propagate the build's trap exit code; bear stderr:\n{bear_stderr}"
    );
    Ok(())
}

/// Ctrl-C typed at the terminal reaches the foreground process group. Bear
/// and the build both end, and Bear reports a non-success status.
// Requirements: cli-signal-forwarding
#[test]
#[cfg(all(unix, has_symbol_openpty, has_symbol_login_tty))]
#[cfg(all(has_executable_shell, has_executable_sleep))]
fn ctrl_c_at_the_terminal_ends_bear_and_the_build() -> Result<()> {
    use crate::fixtures::infrastructure::Terminal;
    use std::time::Duration;

    let env = TestEnvironment::new("ctrl_c_at_the_terminal")?;
    let script = env.create_shell_script("build.sh", &format!("echo build-started\n{SLEEP_PATH} 60\n"))?;
    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir()).args([
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        script.to_str().unwrap(),
    ]);
    let Some(mut terminal) = Terminal::spawn_or_skip(cmd)? else {
        return Ok(());
    };
    terminal.wait_for("build-started", Duration::from_secs(30))?;

    terminal.type_bytes(&[0x03])?;
    let sut = terminal.wait()?;

    assert!(!sut.success(), "Bear must not report success after Ctrl-C: {sut:?}");
    Ok(())
}

/// With no build running, a termination signal ends Bear by that signal and
/// leaves no temporary file next to the output, which stays as it was.
// Requirements: cli-signal-forwarding, output-atomic-write, cli-exit-codes
#[test]
#[cfg(target_family = "unix")]
fn signal_without_a_build_removes_the_temp_file() -> Result<()> {
    use crate::fixtures::infrastructure::signal_process;
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::time::Duration;

    let cases: [(&str, &[&str]); 2] = [
        ("semantic", &["semantic", "--output", "out.json"]),
        ("parse_sh", &["parse-sh", "--output", "out.json"]),
    ];
    for (name, args) in cases {
        let env = TestEnvironment::new(&format!("signal_without_build_{name}"))?;
        let output = env.test_dir().join("out.json");
        let known = "[]\n";
        std::fs::write(&output, known)?;

        // The test keeps the write end of stdin open, so Bear stays blocked reading it.
        let mut child = env
            .command_bear()
            .current_dir(env.test_dir())
            .args(args)
            .process_group(0)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        let stdin = child.stdin.take();

        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        while !has_temp_file(env.test_dir()) {
            assert!(std::time::Instant::now() < deadline, "{name}: the temp file never showed up");
            std::thread::sleep(Duration::from_millis(20));
        }

        signal_process(&child, "TERM");
        let sut = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if std::time::Instant::now() >= deadline {
                child.kill()?;
                panic!("{name}: Bear did not stop after SIGTERM");
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        drop(stdin);

        assert_eq!(sut.signal(), Some(libc::SIGTERM), "{name}: wait status {sut:?}");
        assert_eq!(std::fs::read_to_string(&output)?, known, "{name}: the output changed");
        assert!(!has_temp_file(env.test_dir()), "{name}: a temp file is left behind");
    }
    Ok(())
}

/// Whether the directory holds one of Bear's output temp files
/// (`.{pid}.{random}.tmp`).
#[cfg(target_family = "unix")]
fn has_temp_file(dir: &std::path::Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with('.') && name.ends_with(".tmp")
        })
    })
}

/// What one run of an interrupted-build scenario left behind.
#[cfg(all(target_family = "unix", has_executable_shell, has_executable_sleep, has_executable_compiler_c))]
struct InterruptedRun {
    status: std::process::ExitStatus,
    stderr: String,
    output: String,
    has_temp_file: bool,
}

/// The entry the output holds before the run. A run that publishes drops it
/// (overwrite) or keeps it (append). The build compiles `new.c`, so a
/// published output also holds `NEW_ENTRY`.
#[cfg(all(target_family = "unix", has_executable_shell, has_executable_sleep, has_executable_compiler_c))]
const KNOWN_ENTRY: &str = "old.c";

#[cfg(all(target_family = "unix", has_executable_shell, has_executable_sleep, has_executable_compiler_c))]
const NEW_ENTRY: &str = "new.c";

/// Runs `build` under Bear with an existing output. With `signal`, waits for
/// the build to write its ready file, then sends the signal to Bear's group.
#[cfg(all(target_family = "unix", has_executable_shell, has_executable_sleep, has_executable_compiler_c))]
fn run_interrupted(
    name: &str,
    mode: &str,
    build: &str,
    signal: Option<&str>,
) -> Result<(InterruptedRun, String)> {
    use crate::fixtures::infrastructure::signal_group;
    use std::time::Duration;

    let env = TestEnvironment::new(name)?;
    let output_path = env.test_dir().join("compile_commands.json");
    std::fs::write(env.test_dir().join(KNOWN_ENTRY), "int main() { return 0; }\n")?;
    let known = format!(
        "[{{\"file\":\"{KNOWN_ENTRY}\",\"directory\":{dir:?},\"arguments\":[\"cc\",\"-c\",\"{KNOWN_ENTRY}\"]}}]\n",
        dir = env.test_dir().to_str().unwrap()
    );
    std::fs::write(&output_path, &known)?;

    let ready = env.test_dir().join("ready");
    std::fs::write(env.test_dir().join("new.c"), "int main() { return 0; }\n")?;
    let script = build
        .replace("READY", &ready.display().to_string())
        .replace("COMPILE", &format!("{COMPILER_C_PATH} -c new.c"));
    let mut child = env.spawn_bear_in_group_with_stderr(
        &["--output", "compile_commands.json", mode, "--", SHELL_PATH, "-c", &script],
        Stdio::piped(),
    )?;
    let mut stderr = child.stderr.take().expect("bear stderr was piped");
    let drain = std::thread::spawn(move || {
        let mut captured = String::new();
        let _ = stderr.read_to_string(&mut captured);
        captured
    });

    if let Some(signal) = signal {
        wait_for_file(&ready);
        signal_group(&child, signal);
    }
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill()?;
            panic!("{name}: Bear did not end");
        }
        std::thread::sleep(Duration::from_millis(20));
    };

    let run = InterruptedRun {
        status,
        stderr: drain.join().unwrap_or_default(),
        output: std::fs::read_to_string(&output_path)?,
        has_temp_file: has_temp_file(env.test_dir()),
    };
    Ok((run, known))
}

/// An interrupted build publishes only when the run overwrites the output.
// Requirements: output-append, output-atomic-write
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_shell, has_executable_sleep, has_executable_compiler_c))]
fn interrupted_build_with_append_keeps_the_output() -> Result<()> {
    let build = format!("COMPILE; {SLEEP_PATH} 60 & echo ready > READY; wait");

    let (sut, known) = run_interrupted("interrupted_append", "--append", &build, Some("TERM"))?;

    assert_eq!(sut.output, known, "the output changed; stderr:\n{}", sut.stderr);
    assert!(!sut.output.contains(NEW_ENTRY), "the new entry was published: {}", sut.output);
    assert!(!sut.has_temp_file, "a temp file is left behind");
    assert!(
        sut.stderr.contains("bear: warning: the build was interrupted; compile_commands.json is unchanged"),
        "no warning; stderr:\n{}",
        sut.stderr
    );
    Ok(())
}

// Requirements: output-append, output-atomic-write
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_shell, has_executable_sleep, has_executable_compiler_c))]
fn interrupted_build_with_overwrite_replaces_the_output() -> Result<()> {
    let build = format!("COMPILE; {SLEEP_PATH} 60 & echo ready > READY; wait");

    let (sut, known) = run_interrupted("interrupted_overwrite", "--overwrite", &build, Some("TERM"))?;

    assert_ne!(sut.output, known, "the output was not replaced; stderr:\n{}", sut.stderr);
    assert!(!sut.output.contains(KNOWN_ENTRY), "the old entry is still there: {}", sut.output);
    assert!(sut.output.contains(NEW_ENTRY), "the new entry is missing: {}", sut.output);
    assert!(!sut.has_temp_file, "a temp file is left behind");
    assert!(
        sut.stderr.contains(
            "bear: warning: the build was interrupted; compile_commands.json was replaced with a partial result (--overwrite)"
        ),
        "no warning; stderr:\n{}",
        sut.stderr
    );
    Ok(())
}

/// A build that survives the signal and exits 0 publishes in both modes, and
/// Bear says nothing about an interruption.
// Requirements: output-append, output-atomic-write
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_shell, has_executable_sleep, has_executable_compiler_c))]
fn interrupted_build_that_exits_zero_publishes() -> Result<()> {
    let build = format!("trap 'kill $!; exit 0' TERM; COMPILE; {SLEEP_PATH} 60 & echo ready > READY; wait");

    for mode in ["--append", "--overwrite"] {
        let (sut, known) =
            run_interrupted(&format!("interrupted_exit_zero{mode}"), mode, &build, Some("TERM"))?;

        assert_eq!(sut.status.code(), Some(0), "{mode}: {:?}", sut.status);
        assert!(!sut.has_temp_file, "{mode}: a temp file is left behind");
        if mode == "--append" {
            assert!(sut.output.contains(KNOWN_ENTRY), "{mode}: the old entry is gone: {}", sut.output);
        } else {
            assert_ne!(sut.output, known, "{mode}: the output was not published");
        }
        assert!(sut.output.contains(NEW_ENTRY), "{mode}: the new entry is missing: {}", sut.output);
        assert!(!sut.stderr.contains("was interrupted"), "{mode}: unexpected warning:\n{}", sut.stderr);
    }
    Ok(())
}

/// An interrupted build that exits non-zero leaves an appended output as it
/// was, and Bear passes the build's exit code on.
// Requirements: output-append
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_shell, has_executable_sleep, has_executable_compiler_c))]
fn interrupted_build_that_fails_keeps_the_appended_output() -> Result<()> {
    let build = format!("trap 'kill $!; exit 3' TERM; COMPILE; {SLEEP_PATH} 60 & echo ready > READY; wait");

    let (sut, known) = run_interrupted("interrupted_exit_three", "--append", &build, Some("TERM"))?;

    assert_eq!(sut.status.code(), Some(3), "{:?}; stderr:\n{}", sut.status, sut.stderr);
    assert_eq!(sut.output, known, "the output changed");
    assert!(!sut.output.contains(NEW_ENTRY), "the new entry was published");
    assert!(!sut.has_temp_file, "a temp file is left behind");
    Ok(())
}

/// A build that ends badly with no signal to Bear is a failed build, and a
/// failed build publishes, whatever the mode.
// Requirements: output-append, output-atomic-write
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_shell, has_executable_sleep, has_executable_compiler_c))]
fn failed_build_without_a_signal_publishes() -> Result<()> {
    let cases = [("killed", "COMPILE; kill -KILL $$", None), ("exit_two", "COMPILE; exit 2", Some(2))];
    for (name, build, code) in cases {
        for mode in ["--append", "--overwrite"] {
            let (sut, known) = run_interrupted(&format!("failed_build_{name}{mode}"), mode, build, None)?;

            if let Some(code) = code {
                assert_eq!(sut.status.code(), Some(code), "{name} {mode}: {:?}", sut.status);
            }
            assert!(!sut.has_temp_file, "{name} {mode}: a temp file is left behind");
            assert!(!sut.stderr.contains("was interrupted"), "{name} {mode}: unexpected warning");
            if mode == "--append" {
                assert!(sut.output.contains(KNOWN_ENTRY), "{name} {mode}: the old entry is gone");
            } else {
                assert_ne!(sut.output, known, "{name} {mode}: the output was not published");
            }
            assert!(
                sut.output.contains(NEW_ENTRY),
                "{name} {mode}: the new entry is missing: {}",
                sut.output
            );
        }
    }
    Ok(())
}
