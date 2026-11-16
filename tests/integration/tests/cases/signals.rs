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
    use crate::fixtures::infrastructure::signal_group;

    let env = TestEnvironment::new("exit_code_mid_compile_signal")?;

    // Named pipe as the compiler's input. With no writer, the compiler
    // blocks in read() and stays mid-compile until a signal arrives.
    let fifo = env.test_dir().join("source.c");
    let mkfifo_status = std::process::Command::new("mkfifo").arg(&fifo).status()?;
    assert!(mkfifo_status.success(), "mkfifo failed -- this test needs a POSIX environment");

    let mut child = env.spawn_bear_in_group_with_stderr(
        &[
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
        ],
        Stdio::null(),
    )?;

    // Give the compiler time to start and block on opening/reading the FIFO.
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Send SIGTERM to the group, as a CI runner would. With a signal to
    // Bear's pid alone the compiler would stay blocked on the FIFO.
    let signal_time = Instant::now();
    signal_group(&child, "TERM");

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

/// The build is a plain child in Bear's process group. A signal to Bear's
/// pid reaches the build's top process through the relay and leaves a
/// background grandchild running, as it would without Bear. A signal to the
/// group reaches the grandchild too.
// Requirements: cli-signal-forwarding
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_shell, has_executable_sleep))]
fn signal_reaches_the_build_tree_only_through_the_group() -> Result<()> {
    use crate::fixtures::infrastructure::{signal_group, signal_process};

    // (name, whether the signal goes to the group)
    let cases = [("pid", false), ("group", true)];
    for (name, to_group) in cases {
        let env = TestEnvironment::new(&format!("signal_build_tree_{name}"))?;
        let gpid_file = env.test_dir().join("grandchild.pid");
        let ready = env.test_dir().join("ready");
        let script = format!(
            "{sleep} 60 & echo $! > {pid}; echo ready > {ready}; wait",
            sleep = SLEEP_PATH,
            pid = gpid_file.display(),
            ready = ready.display()
        );
        let mut child = env.spawn_bear_in_group_with_stderr(
            &["--output", "compile_commands.json", "--", SHELL_PATH, "-c", &script],
            Stdio::null(),
        )?;
        wait_for_file(&ready);
        let gpid: i32 = std::fs::read_to_string(&gpid_file)?.trim().parse()?;
        assert!(is_alive(gpid), "{name}: the grandchild should run before the signal");

        if to_group {
            signal_group(&child, "TERM");
        } else {
            signal_process(&child, "TERM");
        }
        let status = child.wait()?;

        assert!(!status.success(), "{name}: bear must report non-success after the signal");
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        while is_alive(gpid) && Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let survived = is_alive(gpid);
        if survived {
            let _ = std::process::Command::new("kill").args(["-KILL", &gpid.to_string()]).status();
        }
        assert_eq!(survived, !to_group, "{name}: grandchild alive after the signal: {survived}");
    }
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
/// the wrapper's supervision when the signal arrives. Signaling Bear's process
/// group must still tear the whole nested tree down within the budget -- not
/// leave Bear hanging on the blocked compiler -- so the contract holds in
/// wrapper mode just as in preload mode.
// Requirements: cli-signal-forwarding, interception-wrapper-mechanism
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn signal_tears_down_nested_wrapper_supervision() -> Result<()> {
    use crate::fixtures::infrastructure::signal_group;

    let env = TestEnvironment::new("signal_wrapper_teardown")?;

    // Named pipe as the compiler's input: with no writer the compiler blocks
    // in read() and stays mid-compile until a signal arrives.
    let fifo = env.test_dir().join("source.c");
    let mkfifo_status = std::process::Command::new("mkfifo").arg(&fifo).status()?;
    assert!(mkfifo_status.success(), "mkfifo failed -- this test needs a POSIX environment");

    // Invoke the compiler via $CC so wrapper mode actually supervises it.
    let build = format!(
        "CC={cc}\n$CC -x c -c {fifo} -o out.o",
        cc = filename_of(COMPILER_C_PATH),
        fifo = fifo.display()
    );
    let script = env.create_shell_script("build.sh", &build)?;

    let config = "schema: \"4.2\"\n\nintercept:\n  mode: wrapper\n";
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    let mut child = env.spawn_bear_in_group_with_stderr(
        &[
            "--config",
            config_path.to_str().unwrap(),
            "--output",
            "compile_commands.json",
            "--",
            SHELL_PATH,
            script.to_str().unwrap(),
        ],
        Stdio::null(),
    )?;

    // Let the build reach the compiler and the compiler block on the FIFO.
    std::thread::sleep(std::time::Duration::from_millis(800));

    let signal_time = Instant::now();
    signal_group(&child, "TERM");

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
        .stderr(Stdio::null())
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
/// installs `traps` (signal name, marker text, exit code) and blocks in
/// `wait` on a background sleep, wait until the script reports ready, send `signal` to bear,
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
        script.push_str(&format!(
            "trap 'kill $! ; echo {text} > {m} ; exit {code}' {sig} ; ",
            m = marker.display()
        ));
    }
    script.push_str(&format!(
        "{sleep} 60 & echo ready > {r} ; wait",
        r = ready.display(),
        sleep = SLEEP_PATH
    ));

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

/// The build reads the terminal. Bear and the build share a process group,
/// so the build is not stopped for reading from it.
// Requirements: cli-signal-forwarding
#[test]
#[cfg(all(unix, has_symbol_openpty, has_symbol_login_tty))]
#[cfg(has_executable_shell)]
fn the_build_reads_a_line_typed_at_the_terminal() -> Result<()> {
    use crate::fixtures::infrastructure::Terminal;
    use std::time::Duration;

    let env = TestEnvironment::new("the_build_reads_the_terminal")?;
    let got = env.test_dir().join("got");
    let script = env.create_shell_script(
        "build.sh",
        &format!("echo ready\nread line\necho \"$line\" > {got}\n", got = got.display()),
    )?;
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
    terminal.wait_for("ready", Duration::from_secs(30))?;

    terminal.type_bytes(b"hello\n")?;
    let sut = terminal.wait()?;

    assert!(sut.success(), "{sut:?}; output:\n{}", terminal.output());
    assert_eq!(std::fs::read_to_string(&got)?.trim(), "hello");
    Ok(())
}

/// What a build that traps `SIGINT` and reads a line leaves after Bear got
/// an interrupt: the terminal output (which holds Bear's debug log) and the
/// `log` file the trap appended to. `interrupt` delivers the interrupt, with
/// the pid of Bear, which the build prints after `ready`.
#[cfg(all(unix, has_symbol_openpty, has_symbol_login_tty))]
#[cfg(has_executable_shell)]
fn interrupt_a_trapping_build_at_the_terminal(
    name: &str,
    interrupt: impl FnOnce(&mut crate::fixtures::infrastructure::Terminal, i32) -> Result<()>,
) -> Result<Option<(String, String)>> {
    use crate::fixtures::infrastructure::Terminal;
    use std::time::Duration;

    let env = TestEnvironment::new(name)?;
    let log = env.test_dir().join("log");
    let script = env.create_shell_script(
        "build.sh",
        &format!(
            "trap 'echo x >> {log}; echo trapped' INT\necho ready $PPID\nread line\n",
            log = log.display()
        ),
    )?;
    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir()).env("RUST_LOG", "debug").args([
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        script.to_str().unwrap(),
    ]);
    let Some(mut terminal) = Terminal::spawn_or_skip(cmd)? else {
        return Ok(None);
    };
    terminal.wait_for("ready ", Duration::from_secs(30))?;
    let output = terminal.output();
    let bear_pid: i32 = output
        .lines()
        .find_map(|line| line.trim().strip_prefix("ready "))
        .and_then(|pid| pid.trim().parse().ok())
        .unwrap_or_else(|| panic!("no pid after ready; output:\n{output}"));

    interrupt(&mut terminal, bear_pid)?;
    terminal.wait_for("trapped", Duration::from_secs(30))?;
    // The shell may have left `read` already, so the newline can go nowhere.
    let _ = terminal.type_bytes(b"\n");
    terminal.wait()?;

    let decision = "SIGINT";
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !terminal.output().contains(&format!("skipped {decision}"))
        && !terminal.output().contains(&format!("relayed {decision}"))
    {
        assert!(
            std::time::Instant::now() < deadline,
            "Bear logged no decision; output:\n{}",
            terminal.output()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(Some((terminal.output(), std::fs::read_to_string(&log).unwrap_or_default())))
}

/// Whether one log line says the interrupt was relayed with cause `other`.
#[cfg(all(unix, has_symbol_openpty, has_symbol_login_tty, has_executable_shell))]
fn relayed_as_other(output: &str) -> bool {
    output.lines().any(|line| line.contains("relayed SIGINT to pid ") && line.contains("(cause: other)"))
}

/// Ctrl-C reaches the build from the terminal. Bear does not relay it a
/// second time. The check is on Bear's debug log, not on the count of trap
/// runs: two pending `SIGINT` merge into one.
// Requirements: cli-signal-forwarding
#[test]
#[cfg(all(unix, has_symbol_openpty, has_symbol_login_tty))]
#[cfg(has_executable_shell)]
fn ctrl_c_at_the_terminal_is_not_relayed_again() -> Result<()> {
    let Some((output, log)) =
        interrupt_a_trapping_build_at_the_terminal("ctrl_c_not_relayed_again", |terminal, _| {
            terminal.type_bytes(&[0x03])
        })?
    else {
        return Ok(());
    };

    assert!(log.contains('x'), "the trap did not run; output:\n{output}");
    assert!(!output.contains("(cause: kernel)"), "a kernel signal was relayed; output:\n{output}");
    let skipped = output.contains("skipped SIGINT, the child got it from the terminal");
    let relayed = relayed_as_other(&output);
    assert!(skipped || relayed, "no decision line; output:\n{output}");
    #[cfg(target_os = "macos")]
    assert!(relayed, "macOS reports no kernel cause, so it relays; output:\n{output}");
    Ok(())
}

/// A `kill -INT` sent to Bear's pid from outside is not a terminal signal:
/// the build has not got it, so Bear relays it.
// Requirements: cli-signal-forwarding
#[test]
#[cfg(all(unix, has_symbol_openpty, has_symbol_login_tty))]
#[cfg(has_executable_shell)]
fn interrupt_sent_to_bear_at_the_terminal_is_relayed() -> Result<()> {
    let Some((output, log)) =
        interrupt_a_trapping_build_at_the_terminal("interrupt_sent_to_bear_is_relayed", |_, bear_pid| {
            let status = std::process::Command::new("kill").args(["-INT", &bear_pid.to_string()]).status()?;
            assert!(status.success(), "kill -INT failed");
            Ok(())
        })?
    else {
        return Ok(());
    };

    assert!(log.contains('x'), "the trap did not run; output:\n{output}");
    assert!(relayed_as_other(&output), "no relay line; output:\n{output}");
    assert!(!output.contains("skipped SIGINT"), "the interrupt was skipped; output:\n{output}");
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

/// Runs Bear with `args`. With `signal`, waits for the ready file, then sends
/// the signal to Bear's group. Returns Bear's status and stderr.
#[cfg(all(target_family = "unix", has_executable_shell, has_executable_sleep, has_executable_compiler_c))]
fn run_until_signalled(
    env: &TestEnvironment,
    name: &str,
    args: &[&str],
    signal: Option<(&std::path::Path, &str)>,
) -> Result<(std::process::ExitStatus, String)> {
    use crate::fixtures::infrastructure::signal_group;
    use std::time::Duration;

    let mut child = env.spawn_bear_in_group_with_stderr(args, Stdio::piped())?;
    let mut stderr = child.stderr.take().expect("bear stderr was piped");
    let drain = std::thread::spawn(move || {
        let mut captured = String::new();
        let _ = stderr.read_to_string(&mut captured);
        captured
    });

    if let Some((ready, signal)) = signal {
        wait_for_file(ready);
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
    Ok((status, drain.join().unwrap_or_default()))
}

/// Runs `build` under Bear with an existing output. With `signal`, waits for
/// the build to write its ready file, then sends the signal to Bear's group.
#[cfg(all(target_family = "unix", has_executable_shell, has_executable_sleep, has_executable_compiler_c))]
fn run_interrupted(
    name: &str,
    mode: &str,
    build: &str,
    signal: Option<&str>,
) -> Result<(InterruptedRun, String)> {
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
        .replace("COMPILE", &format!("{} -c new.c", filename_of(COMPILER_C_PATH)));
    let args = ["--output", "compile_commands.json", mode, "--", SHELL_PATH, "-c", &script];
    let (status, stderr) =
        run_until_signalled(&env, name, &args, signal.map(|signal| (ready.as_path(), signal)))?;

    let run = InterruptedRun {
        status,
        stderr,
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

/// Intercept mode always replaces the events file, so an interrupted build
/// still leaves the events captured so far.
// Requirements: interception-events-format
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_shell, has_executable_sleep, has_executable_compiler_c))]
fn interrupted_build_in_intercept_mode_writes_the_captured_events() -> Result<()> {
    let env = TestEnvironment::new("interrupted_intercept")?;
    std::fs::write(env.test_dir().join(NEW_ENTRY), "int main() { return 0; }\n")?;
    let ready = env.test_dir().join("ready");
    let script = format!(
        "{} -c {NEW_ENTRY}; {SLEEP_PATH} 60 & echo ready > {}; wait",
        filename_of(COMPILER_C_PATH),
        ready.display()
    );
    let args = ["intercept", "--output", "events.json", "--", SHELL_PATH, "-c", &script];
    let (status, stderr) = run_until_signalled(&env, "interrupted_intercept", &args, Some((&ready, "TERM")))?;

    let sut = std::fs::read_to_string(env.test_dir().join("events.json"))?;
    assert!(!status.success(), "Bear ended with {status:?}; stderr:\n{stderr}");
    assert!(sut.contains(NEW_ENTRY), "the compile event is missing: {sut}; stderr:\n{stderr}");
    assert!(
        stderr.contains(
            "bear: warning: the build was interrupted; events.json holds the events captured so far"
        ),
        "no warning; stderr:\n{stderr}"
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

/// A signal that is ignored when Bear starts stays ignored for the build,
/// as with a script that starts Bear with `&`. A shell cannot trap a signal
/// that was ignored when it started, so the marker says `survived`.
// Requirements: cli-signal-forwarding
#[test]
#[cfg(target_family = "unix")]
#[cfg(has_executable_shell)]
fn ignored_signal_stays_ignored_in_the_build() -> Result<()> {
    use std::os::unix::process::CommandExt;

    let env = TestEnvironment::new("ignored_signal_stays_ignored")?;
    let marker = env.test_dir().join("marker");
    let script = format!(
        "trap 'echo trapped > {m}; exit 3' INT; kill -INT $$; echo survived > {m}",
        m = marker.display()
    );
    let mut command = env.command_bear();
    // SAFETY: runs in the child between fork and exec, and calls only
    // `signal`, which is async-signal-safe. It runs after the harness reset.
    unsafe {
        command.pre_exec(|| {
            libc::signal(libc::SIGINT, libc::SIG_IGN);
            Ok(())
        });
    }

    let sut = command
        .current_dir(env.test_dir())
        .args(["--output", "compile_commands.json", "--", SHELL_PATH, "-c", &script])
        .stdin(std::process::Stdio::null())
        .output()?;

    assert_eq!(
        std::fs::read_to_string(&marker)?.trim(),
        "survived",
        "stderr:\n{}",
        String::from_utf8_lossy(&sut.stderr)
    );
    Ok(())
}

/// SIGHUP is a termination signal: Bear relays it, the build's trap runs and
/// its exit code passes through, and the wrapper directory is removed.
// Requirements: cli-signal-forwarding, interception-wrapper-mechanism
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_shell, has_executable_sleep))]
fn hangup_runs_the_build_trap_and_cleans_up() -> Result<()> {
    use crate::fixtures::infrastructure::signal_group;

    let env = TestEnvironment::new("hangup_runs_the_build_trap")?;
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, "schema: \"4.2\"\n\nintercept:\n  mode: wrapper\n")?;
    let marker = env.test_dir().join("marker");
    let ready = env.test_dir().join("ready");
    let script = format!(
        "trap 'kill $!; echo trapped > {m}; exit 7' HUP; {sleep} 60 & echo ready > {r}; wait",
        m = marker.display(),
        r = ready.display(),
        sleep = SLEEP_PATH
    );
    let mut child = env.spawn_bear_in_group_with_stderr(
        &[
            "--config",
            config_path.to_str().unwrap(),
            "--output",
            "compile_commands.json",
            "--",
            SHELL_PATH,
            "-c",
            &script,
        ],
        Stdio::null(),
    )?;

    wait_for_file(&ready);
    assert!(env.test_dir().join(".bear").is_dir(), "the wrapper mechanism is not active");
    signal_group(&child, "HUP");
    let deadline = Instant::now() + std::time::Duration::from_secs(30);
    let sut = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill()?;
            panic!("Bear did not end after SIGHUP");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };

    assert_eq!(std::fs::read_to_string(&marker).unwrap_or_default().trim(), "trapped");
    assert_eq!(sut.code(), Some(7), "{sut:?}");
    assert!(!env.test_dir().join(".bear").exists(), ".bear is left behind");
    Ok(())
}

/// Ctrl-Z at a job-control shell stops Bear and the build together, and
/// they continue together. A stop only works under a job-control shell, so
/// the terminal runs `bash` with `set -m`. The test reads the states with
/// `ps` itself: macOS `/bin/bash` 3.2 never reports the stopped job, so a
/// check in the script after the stop would never run there.
// Requirements: cli-signal-forwarding
#[test]
#[cfg(all(unix, has_symbol_openpty, has_symbol_login_tty))]
#[cfg(all(has_executable_shell, has_executable_bash, has_executable_ps, has_executable_sleep))]
fn ctrl_z_at_the_terminal_stops_and_continues_bear_and_the_build() -> Result<()> {
    use crate::fixtures::infrastructure::Terminal;
    use std::time::Duration;

    let env = TestEnvironment::new("ctrl_z_at_the_terminal")?;
    let dir = env.test_dir();
    let (build_pid, bear_pid, marker, go) =
        (dir.join("build.pid"), dir.join("bear.pid"), dir.join("marker"), dir.join("go"));
    let build = env.create_shell_script(
        "build.sh",
        &format!(
            "echo $$ > {b}\necho $PPID > {p}\necho build-ready\nread line\necho done > {m}\n",
            b = build_pid.display(),
            p = bear_pid.display(),
            m = marker.display()
        ),
    )?;
    // Keeps bash alive, so the session survives, until the job is done.
    let script = format!(
        "set -m\n{bear} --output compile_commands.json -- {shell} {build}\n\
         while [ ! -e {go} ]; do {sleep} 0.1; done\n\
         for ((i=0; i<50; i++)); do jobs -p > {jobs}; [ -s {jobs} ] || break; fg; done\n",
        bear = env.bear_path().display(),
        shell = SHELL_PATH,
        build = build.display(),
        go = go.display(),
        jobs = dir.join("jobs").display(),
        sleep = SLEEP_PATH,
    );
    let mut cmd = std::process::Command::new(BASH_PATH);
    crate::fixtures::infrastructure::reset_signals(&mut cmd);
    cmd.current_dir(dir)
        .env("XDG_CONFIG_HOME", env.user_config_dir())
        .env_remove("RUST_LOG")
        .args(["-c", &script]);
    let Some(mut terminal) = Terminal::spawn_or_skip(cmd)? else {
        return Ok(());
    };
    terminal.wait_for("build-ready", Duration::from_secs(30))?;
    let read_pid = |file: &std::path::Path| -> Result<libc::pid_t> {
        Ok(std::fs::read_to_string(file)?.trim().parse()?)
    };
    let (build, bear) = (read_pid(&build_pid)?, read_pid(&bear_pid)?);
    let state_of = |pid: libc::pid_t| -> String {
        let output = std::process::Command::new(PS_PATH)
            .args(["-o", "state=", "-p", &pid.to_string()])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
            .unwrap_or_default();
        output.trim().chars().take(1).collect()
    };

    terminal.type_bytes(&[0x1a])?;
    let deadline = Instant::now() + Duration::from_secs(20);
    let states = loop {
        let states = (state_of(build), state_of(bear));
        if states == ("T".to_string(), "T".to_string()) || Instant::now() >= deadline {
            break states;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(
        states,
        ("T".to_string(), "T".to_string()),
        "Bear and the build must both be stopped (build, bear); output:\n{}",
        terminal.output()
    );
    assert!(!marker.exists(), "the build ran on while stopped");

    // bash 5 saw the stop and took the terminal back, so only its `fg`
    // gives the job the terminal again. bash 3.2 saw nothing and is still
    // waiting, so the job needs the `SIGCONT` from here. If the `SIGCONT`
    // comes first under bash 5, the build stops again on its read and the
    // script runs `fg` once more.
    std::fs::write(&go, "")?;
    // SAFETY: `kill` takes plain integers. Bear leads the job's group.
    let rc = unsafe { libc::kill(-bear, libc::SIGCONT) };
    assert_eq!(rc, 0, "SIGCONT failed: {}", std::io::Error::last_os_error());
    terminal.type_bytes(b"\n")?;
    terminal.wait()?;

    assert!(
        marker.exists(),
        "the build did not finish after it was continued; output:\n{}",
        terminal.output()
    );
    assert!(dir.join("compile_commands.json").is_file(), "the output was not published");
    assert!(!has_temp_file(dir), "a temp file is left behind");
    Ok(())
}

/// A closed terminal hangs up the session. Bear is the session leader, so
/// the kernel signals only Bear: it must relay SIGHUP. The build's trap
/// runs, its exit code passes through, and nothing is left behind. The logger
/// must also survive a dead stderr.
// Requirements: cli-signal-forwarding, interception-wrapper-mechanism, output-atomic-write
#[test]
#[cfg(all(unix, has_symbol_openpty, has_symbol_login_tty))]
#[cfg(all(has_executable_shell, has_executable_sleep))]
fn hang_up_at_the_terminal_runs_the_build_trap_and_cleans_up() -> Result<()> {
    use crate::fixtures::infrastructure::Terminal;

    let env = TestEnvironment::new("hang_up_at_the_terminal")?;
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, "schema: \"4.2\"\n\nintercept:\n  mode: wrapper\n")?;
    let marker = env.test_dir().join("marker");
    let ready = env.test_dir().join("ready");
    let script = format!(
        "trap 'kill $!; echo trapped > {m}; exit 7' HUP; {sleep} 60 & echo ready > {r}; wait",
        m = marker.display(),
        r = ready.display(),
        sleep = SLEEP_PATH
    );
    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir()).env_remove("RUST_LOG").args([
        "--config",
        config_path.to_str().unwrap(),
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        "-c",
        &script,
    ]);
    let Some(mut terminal) = Terminal::spawn_or_skip(cmd)? else {
        return Ok(());
    };
    wait_for_file(&ready);
    assert!(env.test_dir().join(".bear").is_dir(), "the wrapper mechanism is not active");

    terminal.hang_up();
    let sut = terminal.wait()?;

    assert_eq!(std::fs::read_to_string(&marker).unwrap_or_default().trim(), "trapped");
    assert_eq!(sut.code(), Some(7), "{sut:?}");
    assert!(!env.test_dir().join(".bear").exists(), ".bear is left behind");
    assert!(!has_temp_file(env.test_dir()), "a temp file is left behind");
    Ok(())
}

/// `bear -- build &` in a script: the shell has no job control, so it starts
/// the background job with SIGINT ignored. Bear keeps that for the build,
/// as without Bear, so the marker says `survived`.
// Requirements: cli-signal-forwarding
#[test]
#[cfg(target_family = "unix")]
#[cfg(has_executable_shell)]
fn background_from_a_script_keeps_sigint_ignored() -> Result<()> {
    let env = TestEnvironment::new("background_from_a_script")?;
    let marker = env.test_dir().join("marker");
    let build = env.create_shell_script(
        "build.sh",
        &format!(
            "trap 'echo trapped > {m}; exit 3' INT; kill -INT $$; echo survived > {m}\n",
            m = marker.display()
        ),
    )?;
    let script = format!(
        "{bear} --output compile_commands.json -- {shell} {build} & wait $!",
        bear = env.bear_path().display(),
        shell = SHELL_PATH,
        build = build.display()
    );

    let mut command = std::process::Command::new(SHELL_PATH);
    crate::fixtures::infrastructure::reset_signals(&mut command);
    let sut = command
        .current_dir(env.test_dir())
        .env("XDG_CONFIG_HOME", env.user_config_dir())
        .env_remove("RUST_LOG")
        .args(["-c", &script])
        .stdin(std::process::Stdio::null())
        .output()?;

    assert_eq!(
        std::fs::read_to_string(&marker).unwrap_or_default().trim(),
        "survived",
        "stderr:\n{}",
        String::from_utf8_lossy(&sut.stderr)
    );
    assert!(sut.status.success(), "{:?}\nstderr:\n{}", sut.status, String::from_utf8_lossy(&sut.stderr));
    assert!(env.test_dir().join("compile_commands.json").is_file(), "the output was not published");
    Ok(())
}
