// SPDX-License-Identifier: GPL-3.0-or-later

//! Requirements: cli-signal-forwarding
//!
//! Runs a build (or a single compiler call) as a child process, waits for
//! it, and relays every termination signal Bear gets to it. Bear never
//! kills the build: it waits as long as the build takes. The steps that
//! differ between the outermost supervisor and a nested one live in
//! [`Leader`] and [`Inherit`], the two implementations of [`GroupPolicy`],
//! picked at the call site as a type parameter.
//!
//! The process registers the signals once, with [`signals`], and passes the
//! source to every supervision. On Unix the source blocks on `SIGCHLD` and
//! the termination signals; on Windows it polls, and both policies relay
//! nothing.

use std::ffi::c_int;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus};
use thiserror::Error;

#[cfg(unix)]
pub use platform::die_by;

/// How a supervised child ended.
pub struct Outcome {
    pub status: ExitStatus,
    /// A termination signal reached Bear while the child ran.
    pub interrupted: bool,
}

/// This method supervises the execution of a command.
///
/// It starts the command and waits for its completion. While waiting it
/// relays every termination signal from `signals` to the build, and
/// reports the build's exit status and whether it was interrupted back to the caller. Where the signal
/// goes is up to the policy `P`: [`Leader`] or [`Inherit`].
pub fn supervise<P: GroupPolicy>(
    command: &mut Command,
    signals: impl IntoIterator<Item = c_int>,
) -> Result<Outcome, SuperviseError> {
    let executable = PathBuf::from(command.get_program());
    let mut signals = signals.into_iter();

    let policy = P::prepare(command);

    let process = command
        .spawn()
        .map_err(|err| SuperviseError::ProcessSpawn { executable: executable.clone(), source: err })?;
    let mut child = Supervised { process, executable };

    // Close the child-exited-early race: the child may already be gone
    // before we block on the signal source.
    if let Some(status) = child.try_wait()? {
        return Ok(Outcome { status, interrupted: false });
    }

    // Any item that is not a termination signal (`SIGCHLD` on Unix, a poll
    // tick on Windows) means the child may have exited.
    let mut interrupted = false;
    loop {
        match signals.next() {
            Some(signal) if is_termination(signal) => {
                interrupted = true;
                policy.relay(&child, signal);
            }
            Some(_) => {
                if let Some(status) = child.try_wait()? {
                    return Ok(Outcome { status, interrupted });
                }
            }
            // The source ended: nothing is left to forward, so block until
            // the child is reaped.
            None => return child.wait().map(|status| Outcome { status, interrupted }),
        }
    }
}

/// The registered signals, as a blocking iterator. Every item that is not
/// a termination signal means the child may have exited.
pub struct SignalSource(Box<dyn Iterator<Item = c_int> + Send>);

impl Iterator for SignalSource {
    type Item = c_int;

    fn next(&mut self) -> Option<c_int> {
        self.0.next()
    }
}

/// Registers the signals a supervision reacts to and returns them as a
/// source for [`supervise`] or [`abort_on_termination`].
///
/// Call it once per process, before anything creates a temporary artifact:
/// a signal that arrives after the registration waits in the source, so
/// none is lost. The handlers stay installed for the life of the process.
pub fn signals() -> std::io::Result<SignalSource> {
    platform::signals()
}

/// Starts the watcher thread for a process that runs no build: the first
/// termination signal read from `signals` aborts the process.
pub fn abort_on_termination(signals: impl Iterator<Item = c_int> + Send + 'static) {
    std::thread::spawn(move || {
        for signal in signals {
            if is_termination(signal) {
                abort(signal);
            }
        }
    });
}

/// Whether `signal` is one the build or the watcher reacts to: `SIGHUP`,
/// `SIGINT`, `SIGQUIT` and `SIGTERM` on Unix, `SIGINT` and `SIGTERM` on
/// Windows.
fn is_termination(signal: c_int) -> bool {
    platform::TERMINATION.contains(&signal)
}

/// Removes the tracked temporary artifacts, then ends the process by `signal`.
pub fn abort(signal: c_int) -> ! {
    crate::cleanup::remove_tracked();
    platform::die(signal)
}

/// Ends the process like `status`: by the same signal on Unix, with the same
/// code otherwise. Call it last, after every cleanup.
pub fn exit_like(status: ExitStatus) -> ! {
    platform::exit_like(status)
}

/// How a supervisor relates to the build's process group: it sets up the
/// group before the spawn, and decides where a termination signal goes.
///
/// In wrapper mode the supervision chain nests
/// (`bear-driver` -> `make` -> `bear-wrapper` -> real `cc`); only the
/// outermost supervisor may create a process group. If every level created
/// its own group, a signal sent to the top-level group would miss the deeper
/// processes.
///
/// Implemented by [`Leader`] and [`Inherit`]; the policy is chosen at the
/// call site as a type parameter of [`supervise`].
pub trait GroupPolicy: Sized {
    /// Runs before the spawn; the returned value lives until the
    /// supervision ends.
    fn prepare(command: &mut Command) -> Self;

    /// Runs for every termination signal, while the child runs.
    fn relay(&self, child: &Supervised, signal: c_int);
}

/// The outermost supervisor (the driver). Creates a new process group with
/// the child as leader and relays each signal to the whole group.
pub struct Leader;

/// A nested supervisor (the wrapper). Stays in the inherited group and
/// relays each signal to its direct child only.
pub struct Inherit;

/// Errors that can occur during process supervision.
#[derive(Error, Debug)]
pub enum SuperviseError {
    #[error("cannot run '{executable}': {source}", executable = executable.display())]
    ProcessSpawn {
        executable: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot wait for '{executable}': {source}", executable = executable.display())]
    ProcessWait {
        executable: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// The child [`supervise`] spawned, with its executable for the error
/// messages. Only `supervise` builds one.
pub struct Supervised {
    process: Child,
    executable: PathBuf,
}

impl Supervised {
    fn try_wait(&mut self) -> Result<Option<ExitStatus>, SuperviseError> {
        match self.process.try_wait() {
            Ok(Some(status)) => {
                log::debug!("Child process exited: {status:?}");
                Ok(Some(status))
            }
            Ok(None) => Ok(None),
            Err(err) => {
                log::error!("Error waiting for child process: {err}");
                Err(SuperviseError::ProcessWait { executable: self.executable.clone(), source: err })
            }
        }
    }

    fn wait(&mut self) -> Result<ExitStatus, SuperviseError> {
        self.process
            .wait()
            .map_err(|err| SuperviseError::ProcessWait { executable: self.executable.clone(), source: err })
    }
}

#[cfg(unix)]
mod platform {
    use super::{GroupPolicy, Inherit, Leader, SignalSource, Supervised};
    use signal_hook::consts::signal::{SIGCHLD, SIGHUP, SIGINT, SIGQUIT, SIGTERM};
    use signal_hook::iterator::Signals;
    use std::ffi::c_int;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Command, ExitStatus};

    pub(super) const TERMINATION: [c_int; 4] = [SIGHUP, SIGINT, SIGQUIT, SIGTERM];

    /// The termination signals that are not ignored at entry, plus
    /// `SIGCHLD`, so a wait on the source wakes on a termination signal to
    /// relay or when the child exits. A signal that is ignored at entry is
    /// left alone, so it stays ignored in the build too. signal-hook's
    /// self-pipe keeps a signal delivered after the registration until it
    /// is read.
    pub(super) fn signals() -> std::io::Result<SignalSource> {
        let mut watched: Vec<c_int> =
            TERMINATION.into_iter().filter(|signal| !ignored_at_entry(*signal)).collect();
        watched.push(SIGCHLD);
        let mut signals = Signals::new(&watched)?;
        Ok(SignalSource(Box::new(std::iter::from_fn(move || signals.forever().next()))))
    }

    fn ignored_at_entry(signal: c_int) -> bool {
        // SAFETY: with a null new action, `sigaction` only writes the
        // current action into `old`, a zeroed local that outlives the call.
        unsafe {
            let mut old: libc::sigaction = std::mem::zeroed();
            libc::sigaction(signal, std::ptr::null(), &mut old) == 0 && old.sa_sigaction == libc::SIG_IGN
        }
    }

    pub(super) fn die(signal: c_int) -> ! {
        die_by(signal)
    }

    pub(super) fn exit_like(status: ExitStatus) -> ! {
        match status.signal() {
            Some(signal) => die_by(signal),
            None => std::process::exit(status.code().unwrap_or(1)),
        }
    }

    /// Ends the process as if `signal` had its default action, with core
    /// dumps off. Falls back to `128 + signal` when the signal cannot be
    /// raised.
    pub fn die_by(signal: c_int) -> ! {
        disable_core_dumps();
        let _ = signal_hook::low_level::emulate_default_handler(signal);
        std::process::exit(128 + signal)
    }

    /// `setrlimit` alone leaves the core flag in the wait status when the
    /// core pattern is a pipe, so `prctl` is used where it exists.
    fn disable_core_dumps() {
        // SAFETY: both calls take plain integers or a pointer to a local
        // that outlives the call, and only lower this process's own limits.
        #[cfg(has_symbol_prctl)]
        unsafe {
            libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
        }
        #[cfg(not(has_symbol_prctl))]
        unsafe {
            let limit = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            libc::setrlimit(libc::RLIMIT_CORE, &limit);
        }
    }

    impl GroupPolicy for Leader {
        fn prepare(command: &mut Command) -> Self {
            use std::os::unix::process::CommandExt;

            // The leader owns the build's process group so a single killpg
            // reaches the whole tree. process_group(0) is safe std: the child
            // becomes a new group leader (pgid == child pid) before exec.
            command.process_group(0);
            Leader
        }

        fn relay(&self, child: &Supervised, signal: c_int) {
            let pgid = child.process.id() as libc::pid_t;
            log::debug!("Received signal {signal}, forwarding to process group {pgid}");
            send(Target::Group(pgid), signal);
        }
    }

    impl GroupPolicy for Inherit {
        fn prepare(_command: &mut Command) -> Self {
            Inherit
        }

        fn relay(&self, child: &Supervised, signal: c_int) {
            let child_pid = child.process.id() as libc::pid_t;
            log::debug!("Received signal {signal}, forwarding to child {child_pid}");
            send(Target::Process(child_pid), signal);
        }
    }

    enum Target {
        Group(libc::pid_t),
        Process(libc::pid_t),
    }

    /// Best-effort delivery of a signal to a process or process group.
    ///
    /// Signal forwarding is best effort: a failure to deliver must not turn
    /// into a hard supervision error (only spawn/wait failures are fatal).
    /// `ESRCH` - the target already exited, the normal exit-then-signal race -
    /// is benign and logged at debug; any other errno is logged at error.
    fn send(target: Target, signal: libc::c_int) {
        // SAFETY: kill()/killpg() are async-signal-safe libc calls taking
        // plain integers; they cannot violate memory safety. We inspect errno
        // only on the documented -1 return.
        let rc = unsafe {
            match target {
                Target::Group(pgid) => libc::killpg(pgid, signal),
                Target::Process(pid) => libc::kill(pid, signal),
            }
        };
        if rc == -1 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ESRCH) {
                log::debug!("Signal target already gone (ESRCH), nothing to forward");
            } else {
                log::error!("Failed to forward signal {signal}: {err}");
            }
        }
    }
}

#[cfg(not(unix))]
mod platform {
    use super::{GroupPolicy, Inherit, Leader, SignalSource, Supervised};
    use std::ffi::c_int;
    use std::process::{Command, ExitStatus};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    pub(super) const TERMINATION: [c_int; 2] = [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM];

    /// There is no `SIGCHLD` to wake the wait when the child exits, so the
    /// source wakes it on this interval instead.
    const POLL: Duration = Duration::from_millis(100);

    /// A termination signal is recorded in a flag. The source sleeps one
    /// `POLL`, then yields the recorded signal, or `0` when none arrived,
    /// which the wait reads as "check the child".
    pub(super) fn signals() -> std::io::Result<SignalSource> {
        let signaled = Arc::new(AtomicUsize::new(0));
        for signal in TERMINATION {
            signal_hook::flag::register_usize(signal, Arc::clone(&signaled), signal as usize)?;
        }
        Ok(SignalSource(Box::new(std::iter::from_fn(move || {
            thread::sleep(POLL);
            Some(signaled.swap(0, Ordering::SeqCst) as c_int)
        }))))
    }

    /// What a console program stopped by Ctrl-C returns.
    const STATUS_CONTROL_C_EXIT: u32 = 0xC000013A;

    pub(super) fn die(_signal: c_int) -> ! {
        std::process::exit(STATUS_CONTROL_C_EXIT as i32)
    }

    pub(super) fn exit_like(status: ExitStatus) -> ! {
        std::process::exit(status.code().unwrap_or(1))
    }

    impl GroupPolicy for Leader {
        fn prepare(_command: &mut Command) -> Self {
            Leader
        }

        fn relay(&self, _child: &Supervised, _signal: c_int) {}
    }

    impl GroupPolicy for Inherit {
        fn prepare(_command: &mut Command) -> Self {
            Inherit
        }

        fn relay(&self, _child: &Supervised, _signal: c_int) {}
    }
}

#[cfg(all(test, unix))]
mod tests {
    // Requirements: cli-signal-forwarding
    //
    // The signal source is a plain iterator, so these tests feed fake
    // signals and leave the process-global handlers alone. The child is
    // real; supervise sends the real signal to it, never to the test
    // process. No test waits on a timer: the child reports its pid once its
    // traps are set, and the stand-in for `SIGCHLD` blocks until the child
    // has exited. Whole-tree teardown is verified end to end against the
    // real driver in the integration suite (tests/integration, exit_codes.rs).
    use super::*;
    use fixture::shell;
    use std::os::unix::process::ExitStatusExt;

    mod fixture {
        use super::*;
        use std::io::{BufRead, BufReader, PipeReader, PipeWriter, Write};
        use std::iter;
        use std::os::unix::process::CommandExt;

        /// The test's side of a child's pipes: the child's stdin, held open
        /// so a blocking `cat` or `read` waits for the test, and the child's
        /// stdout, where the script prints `$$` once it is ready.
        pub(super) struct Probe {
            stdin: PipeWriter,
            stdout: BufReader<PipeReader>,
        }

        /// A `sh -c script` command wired to a [`Probe`]. The termination
        /// signals are reset to their default action in the child, so a
        /// signal ignored by the test runner (`cargo test &`) does not leak
        /// into the build and change what the test sees.
        pub(super) fn shell(script: &str) -> (Command, Probe) {
            let (stdin_reader, stdin_writer) = std::io::pipe().expect("stdin pipe");
            let (stdout_reader, stdout_writer) = std::io::pipe().expect("stdout pipe");
            let mut command = Command::new("sh");
            command.arg("-c").arg(script).stdin(stdin_reader).stdout(stdout_writer);
            // SAFETY: runs in the child between fork and exec, and calls
            // only `signal`, which is async-signal-safe.
            unsafe {
                command.pre_exec(|| {
                    for signal in signal_hook::consts::TERM_SIGNALS {
                        libc::signal(*signal, libc::SIG_DFL);
                    }
                    Ok(())
                });
            }
            (command, Probe { stdin: stdin_writer, stdout: BufReader::new(stdout_reader) })
        }

        impl Probe {
            /// Stands in for `SIGCHLD` only: every item comes once the
            /// child has exited.
            pub(super) fn exits(self) -> impl Iterator<Item = c_int> {
                self.signal_then_exits(None)
            }

            /// `signal` once the child is ready, then the `SIGCHLD`
            /// stand-in.
            pub(super) fn signal(self, signal: c_int) -> impl Iterator<Item = c_int> {
                self.signal_then_exits(Some(signal))
            }

            /// Lets the child past its `read`, then ends, so supervise
            /// is left with nothing to wait on but the child itself.
            pub(super) fn release(mut self) -> impl Iterator<Item = c_int> {
                iter::from_fn(move || {
                    self.stdin.write_all(b"go\n").expect("write to the child");
                    None
                })
            }

            /// `signal` once the child is ready. Asked for the next item,
            /// lets the child past its `read` and ends.
            pub(super) fn signal_then_release(mut self, signal: c_int) -> impl Iterator<Item = c_int> {
                let mut ready = false;
                iter::from_fn(move || {
                    if !ready {
                        self.ready();
                        ready = true;
                        return Some(signal);
                    }
                    self.stdin.write_all(b"go\n").expect("write to the child");
                    None
                })
            }

            fn signal_then_exits(mut self, signal: Option<c_int>) -> impl Iterator<Item = c_int> {
                let mut pid = None;
                iter::from_fn(move || match pid {
                    None => {
                        let ready = self.ready();
                        pid = Some(ready);
                        match signal {
                            Some(signal) => Some(signal),
                            None => wait_for_exit(ready),
                        }
                    }
                    Some(pid) => wait_for_exit(pid),
                })
            }

            /// Blocks until the script prints its pid.
            fn ready(&mut self) -> libc::pid_t {
                let mut line = String::new();
                self.stdout.read_line(&mut line).expect("read the child's pid");
                line.trim().parse().expect("the child prints its pid")
            }
        }

        /// Blocks until `pid` has exited, and leaves it unreaped
        /// (`WNOWAIT`), so supervise still collects its status.
        fn wait_for_exit(pid: libc::pid_t) -> Option<c_int> {
            // SAFETY: an all-zero `siginfo_t` is a valid value; waitid only
            // writes into it.
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            loop {
                // SAFETY: plain syscall on a pid this test spawned; `info`
                // outlives the call.
                let rc = unsafe {
                    libc::waitid(libc::P_PID, pid as libc::id_t, &mut info, libc::WEXITED | libc::WNOWAIT)
                };
                if rc == 0 {
                    return Some(libc::SIGCHLD);
                }
                let err = std::io::Error::last_os_error();
                assert_eq!(err.raw_os_error(), Some(libc::EINTR), "waitid failed: {err}");
            }
        }
    }

    #[test]
    fn normal_exit_propagates_status() {
        // arrange / act / assert per case: (exit code, expected success)
        let cases = [(0, true), (7, false)];
        for (code, expect_success) in cases {
            let (mut command, probe) = shell(&format!("echo $$; exit {code}"));

            let sut = supervise::<Leader>(&mut command, probe.exits()).expect("supervise failed").status;

            assert_eq!(sut.success(), expect_success, "exit {code} success mismatch");
            assert_eq!(sut.code(), Some(code), "exit {code} code mismatch");
        }
    }

    #[test]
    fn ended_source_waits_for_the_child() {
        let (mut command, probe) = shell("read _; exit 3");

        let sut = supervise::<Leader>(&mut command, probe.release()).expect("supervise failed").status;

        assert_eq!(sut.code(), Some(3));
    }

    #[test]
    fn leader_forwards_the_received_signal() {
        // arrange / act / assert per case: the signal Bear got reaches the child
        let cases = [libc::SIGTERM, libc::SIGINT];
        for signal in cases {
            let (mut command, probe) = shell("echo $$; exec cat");

            let sut =
                supervise::<Leader>(&mut command, probe.signal(signal)).expect("supervise failed").status;

            assert_eq!(sut.signal(), Some(signal), "signal {signal}");
        }
    }

    #[test]
    fn inherit_relays_the_received_signal() {
        // arrange / act / assert per case: the signal Bear got reaches the child
        let cases = [libc::SIGTERM, libc::SIGINT];
        for signal in cases {
            let (mut command, probe) = shell("echo $$; exec cat");

            let sut =
                supervise::<Inherit>(&mut command, probe.signal(signal)).expect("supervise failed").status;

            assert_eq!(sut.signal(), Some(signal), "signal {signal}");
        }
    }

    #[test]
    fn leader_lets_a_trap_finish() {
        // A shell runs a trap only once its foreground command ends, so a
        // foreground `cat` forked after the signal would never end. The
        // background `cat` exists before the pid is printed, so the group
        // signal reaches it. It reads fd 3 because a background job's stdin
        // is /dev/null.
        let (mut command, probe) = shell("exec 3<&0; trap 'exit 5' TERM; cat <&3 & echo $$; wait; exit 0");

        let sut = supervise::<Leader>(&mut command, probe.signal(libc::SIGTERM)).expect("supervise failed");

        assert!(sut.interrupted);
        assert_eq!(sut.status.code(), Some(5));
    }

    #[test]
    fn leader_waits_for_a_build_that_ignores_the_signal() {
        let (mut command, probe) = shell("trap '' TERM; echo $$; read _; exit 3");

        let sut = supervise::<Leader>(&mut command, probe.signal_then_release(libc::SIGTERM))
            .expect("supervise failed")
            .status;

        assert_eq!(sut.code(), Some(3));
    }
}
