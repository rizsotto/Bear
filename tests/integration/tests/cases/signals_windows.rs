// SPDX-License-Identifier: GPL-3.0-or-later

//! Ctrl-C on Windows.
//!
//! `GenerateConsoleCtrlEvent` reaches every process on the console, so the
//! tests never send it from the test runner. The runner starts a copy of
//! this test binary in a console of its own (the driver). The driver starts
//! Bear in that console, ignores Ctrl-C itself, sends the event, and writes
//! what Bear did to a result file. The copies are told what to do by
//! environment variables and select themselves through the `role` test,
//! which does nothing in a normal run.

use crate::fixtures::infrastructure::TestEnvironment;
use anyhow::{Context, Result, bail};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use windows_sys::Win32::System::Console::{CTRL_C_EVENT, GenerateConsoleCtrlEvent, SetConsoleCtrlHandler};
use windows_sys::Win32::System::Threading::CREATE_NEW_CONSOLE;
use windows_sys::core::BOOL;

const ROLE: &str = "BEAR_TEST_CTRL_C_ROLE";
const DIR: &str = "BEAR_TEST_CTRL_C_DIR";
const BEAR: &str = "BEAR_TEST_CTRL_C_BEAR";
const MODE: &str = "BEAR_TEST_CTRL_C_MODE";
const READY: &str = "BEAR_TEST_CTRL_C_READY";

const BUILD_EXIT_CODE: u32 = 42;
const STATUS_CONTROL_C_EXIT: u32 = 0xC000013A;
const DEADLINE: Duration = Duration::from_secs(30);

/// While a build runs, Ctrl-C relays nothing: Bear waits for the build,
/// which handles Ctrl-C and exits with its own code, and passes that code on.
// Requirements: cli-signal-forwarding, cli-exit-codes
#[test]
fn ctrl_c_during_a_build_waits_for_it_and_passes_its_exit_code() -> Result<()> {
    let env = TestEnvironment::new("ctrl_c_during_build")?;

    let sut = run_in_own_console(&env, "build")?;

    assert_eq!(sut.exit_code, BUILD_EXIT_CODE, "bear stderr:\n{}", sut.bear_stderr);
    Ok(())
}

/// With no build running, Ctrl-C ends Bear with the Ctrl-C status and
/// removes the temporary output file.
// Requirements: cli-signal-forwarding, cli-exit-codes, output-atomic-write
#[test]
fn ctrl_c_without_a_build_ends_bear_and_removes_the_temp_file() -> Result<()> {
    let env = TestEnvironment::new("ctrl_c_without_build")?;
    let output = env.test_dir().join("out.json");
    let known = "[]\n";
    std::fs::write(&output, known)?;

    let sut = run_in_own_console(&env, "idle")?;

    assert_eq!(sut.exit_code, STATUS_CONTROL_C_EXIT, "bear stderr:\n{}", sut.bear_stderr);
    assert!(sut.bear_stderr.contains("interrupted by Ctrl-C"), "bear stderr:\n{}", sut.bear_stderr);
    assert_eq!(std::fs::read_to_string(&output)?, known, "the output changed");
    assert!(!has_temp_file(env.test_dir()), "a temp file is left behind");
    Ok(())
}

struct Outcome {
    exit_code: u32,
    bear_stderr: String,
}

fn run_in_own_console(env: &TestEnvironment, mode: &str) -> Result<Outcome> {
    let mut driver = Command::new(std::env::current_exe()?)
        .args([&role_test_name(), "--exact", "--nocapture"])
        .env(ROLE, "driver")
        .env(DIR, env.test_dir())
        .env(BEAR, env.bear_path())
        .env(MODE, mode)
        .env("XDG_CONFIG_HOME", env.user_config_dir())
        .env("RUST_LOG", "info")
        .creation_flags(CREATE_NEW_CONSOLE)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(env.test_dir().join("driver.stderr"))?)
        .spawn()
        .context("cannot start the driver")?;

    let dir = env.test_dir();
    let deadline = Instant::now() + 3 * DEADLINE;
    while driver.try_wait()?.is_none() {
        if Instant::now() >= deadline {
            driver.kill()?;
            bail!("{mode}: the driver did not finish\n{}", diagnostics(dir));
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let bear_stderr = std::fs::read_to_string(dir.join("bear.stderr")).unwrap_or_default();
    let result = std::fs::read_to_string(dir.join("result.txt")).unwrap_or_default();
    let Some(exit_code) = result.trim().strip_prefix("code=").and_then(|code| code.parse().ok()) else {
        bail!("{mode}: driver result: {result:?}\n{}", diagnostics(dir));
    };
    Ok(Outcome { exit_code, bear_stderr })
}

/// The directory listing and the stderr files of the driver and Bear.
fn diagnostics(dir: &Path) -> String {
    let names: Vec<_> = std::fs::read_dir(dir)
        .map(|entries| entries.flatten().map(|entry| entry.file_name()).collect())
        .unwrap_or_default();
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap_or_default();
    format!(
        "files: {names:?}\ndriver stderr:\n{}\nbear stderr:\n{}",
        read("driver.stderr"),
        read("bear.stderr")
    )
}

/// libtest names the test without the crate name.
fn role_test_name() -> String {
    let module = module_path!();
    let module = module.split_once("::").map_or(module, |(_, rest)| rest);
    format!("{module}::role")
}

/// Not a test. The driver and the build run as this test in a copy of the
/// binary; in a normal run no role is set and it does nothing.
#[test]
fn role() {
    match std::env::var(ROLE).as_deref() {
        Ok("driver") => drive(),
        Ok("build") => build(),
        _ => {}
    }
}

fn path_from_env(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("{name} is not set")))
}

fn wait_until(what: &str, mut ready: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + DEADLINE;
    while !ready() {
        if Instant::now() >= deadline {
            eprintln!("gave up waiting for {what}");
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    true
}

/// Runs in a console of its own. Bear is started while this process still
/// takes Ctrl-C, and the "ignore" attribute is set only after, because
/// children inherit it.
fn drive() {
    let dir = path_from_env(DIR);
    let building = std::env::var(MODE).is_ok_and(|mode| mode == "build");
    let ready = dir.join("build.ready");

    // The runner of the tests may have been started with Ctrl-C ignored,
    // which this console would inherit.
    set_ctrl_c_ignored(false);

    let stderr = std::fs::File::create(dir.join("bear.stderr")).expect("cannot create bear.stderr");
    let mut bear = Command::new(path_from_env(BEAR));
    bear.current_dir(&dir).stdout(Stdio::null()).stderr(stderr).env_remove(ROLE);
    if building {
        let exe = std::env::current_exe().expect("no current exe");
        bear.args(["--output", "compile_commands.json", "--"])
            .arg(exe)
            .args([&role_test_name(), "--exact", "--nocapture"])
            .env(ROLE, "build")
            .env(READY, &ready)
            .stdin(Stdio::null());
    } else {
        bear.args(["semantic", "--output", "out.json"]).stdin(Stdio::piped());
    }
    let mut child = bear.spawn().expect("cannot start bear");
    // Keeps Bear blocked reading stdin in the idle mode.
    let _stdin = child.stdin.take();

    let started = if building {
        wait_until("the build", || ready.exists())
    } else {
        wait_until("the temp file", || has_temp_file(&dir))
    };

    set_ctrl_c_ignored(true);
    // SAFETY: plain FFI call with constant arguments. Group 0 means every process on this console.
    let sent = started && unsafe { GenerateConsoleCtrlEvent(CTRL_C_EVENT, 0) } != 0;
    let result = if sent {
        wait_for_bear(&mut child)
    } else {
        let _ = child.kill();
        let _ = child.wait();
        "readiness never came, or Ctrl-C could not be sent".to_string()
    };
    std::fs::write(dir.join("result.txt"), result).expect("cannot write the result");
}

fn wait_for_bear(child: &mut std::process::Child) -> String {
    let deadline = Instant::now() + DEADLINE;
    loop {
        if let Some(status) = child.try_wait().expect("cannot wait for bear") {
            return match status.code() {
                Some(code) => format!("code={}", code as u32),
                None => "no exit code".to_string(),
            };
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            return "bear did not stop".to_string();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

static CTRL_C_SEEN: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn on_ctrl(kind: u32) -> BOOL {
    if kind == CTRL_C_EVENT {
        CTRL_C_SEEN.store(true, Ordering::SeqCst);
        1
    } else {
        0
    }
}

/// The build handles Ctrl-C, keeps running a little, and exits with a code
/// of its choice. Bear passes that code on only if it waited.
fn build() {
    // SAFETY: `on_ctrl` has the handler signature and only touches an atomic.
    let added = unsafe { SetConsoleCtrlHandler(Some(on_ctrl), 1) };
    if added == 0 {
        std::process::exit(98);
    }
    std::fs::write(path_from_env(READY), "ready").expect("cannot write the ready file");

    if !wait_until("Ctrl-C", || CTRL_C_SEEN.load(Ordering::SeqCst)) {
        std::process::exit(99);
    }
    std::thread::sleep(Duration::from_millis(500));
    std::process::exit(BUILD_EXIT_CODE as i32);
}

fn set_ctrl_c_ignored(ignored: bool) {
    // SAFETY: a null handler with TRUE or FALSE only toggles the "ignore Ctrl-C" attribute of this process.
    let ok = unsafe { SetConsoleCtrlHandler(None, BOOL::from(ignored)) };
    assert!(ok != 0, "SetConsoleCtrlHandler failed");
}

/// Whether the directory holds one of Bear's output temp files
/// (`.{pid}.{random}.tmp`).
fn has_temp_file(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with('.') && name.ends_with(".tmp")
        })
    })
}
