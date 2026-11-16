// SPDX-License-Identifier: GPL-3.0-or-later

//! Requirements: cli-signal-forwarding
//!
//! Runs a build (or a single compiler call) as a child process, waits for
//! it, and relays every termination signal Bear gets to it. Bear never
//! kills the build: it waits as long as the build takes. The child stays
//! in Bear's process group, so the terminal reaches it directly.
//!
//! The process registers the signals once, with [`signals`], and passes the
//! source to every supervision. On Unix the source blocks on `SIGCHLD` and
//! the termination signals; on Windows it polls, and nothing is relayed.

use std::collections::VecDeque;
use std::ffi::c_int;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus};
use thiserror::Error;

#[cfg(unix)]
pub use platform::die_by;

/// One received signal. `from_kernel` is true only when signal-hook
/// reports `Cause::Kernel` (Linux terminal signals). Unknown is false.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Received {
    pub signal: c_int,
    pub from_kernel: bool,
}

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
/// reports the build's exit status and whether it was interrupted back to
/// the caller. A signal goes to the direct child only, and not at all when
/// the terminal already delivered it.
pub fn supervise(
    command: &mut Command,
    signals: impl IntoIterator<Item = Received>,
) -> Result<Outcome, SuperviseError> {
    let executable = PathBuf::from(command.get_program());
    let mut signals = signals.into_iter();

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
            Some(received) if is_termination(received.signal) => {
                interrupted = true;
                platform::relay(&child, received);
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
///
/// The first item is read only after [`supervise`] has spawned the child.
/// Signals queued by then come first and are marked as not from the
/// kernel: a terminal key that arrived before the child existed never
/// reached it, so it must be relayed. A key that lands in the instant of
/// the spawn may reach the child twice.
pub struct SignalSource {
    inner: platform::Inner,
    queued: Option<VecDeque<Received>>,
}

impl Iterator for SignalSource {
    type Item = Received;

    fn next(&mut self) -> Option<Received> {
        let inner = &mut self.inner;
        let queued = self.queued.get_or_insert_with(|| queued(platform::pending(inner)).into());
        queued.pop_front().or_else(|| platform::next(inner))
    }
}

impl SignalSource {
    /// Without blocking, a termination signal that arrived since the
    /// registration, if any. Every other signal read here is dropped: on Unix
    /// those are `SIGCHLD`, which nothing waits on yet. A signal read here is
    /// consumed, so the caller must act on it.
    pub fn pending_termination(&mut self) -> Option<c_int> {
        first_termination(&platform::pending(&mut self.inner))
    }
}

/// Signals found queued as `supervise` starts reading, as [`Received`].
/// None is from the kernel: the child did not exist when it arrived, so
/// the terminal did not give it a copy.
fn queued(signals: Vec<c_int>) -> Vec<Received> {
    signals.into_iter().map(|signal| Received { signal, from_kernel: false }).collect()
}

/// The first termination signal in `signals`.
fn first_termination(signals: &[c_int]) -> Option<c_int> {
    signals.iter().copied().find(|signal| is_termination(*signal))
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
pub fn abort_on_termination(signals: impl Iterator<Item = Received> + Send + 'static) {
    std::thread::spawn(move || {
        for received in signals {
            if is_termination(received.signal) {
                abort(received.signal);
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

/// `from_kernel`: signal-hook reported `Cause::Kernel`.
/// `child_shares_group`: the child is still in Bear's process group.
#[cfg(unix)]
fn relays(signal: c_int, from_kernel: bool, child_shares_group: bool) -> bool {
    let terminal_key = signal == libc::SIGINT || signal == libc::SIGQUIT;
    !(terminal_key && from_kernel && child_shares_group)
}

/// Warns that `signal` aborted Bear, removes the tracked temporary artifacts,
/// then ends the process by `signal`.
///
/// The warning goes through the logger, which drops a write error, so a
/// closed stderr cannot stop the abort.
pub fn abort(signal: c_int) -> ! {
    // A stalled stderr reader blocks the abort here, before the cleanup.
    log::warn!("{}", abort_warning(signal));
    crate::cleanup::remove_tracked();
    platform::die(signal)
}

/// The warning for an abort. It claims nothing about the output: stdout and
/// special files are written in place, and the signal can land after the commit.
fn abort_warning(signal: c_int) -> String {
    format!("interrupted by {}; stopped before finishing", platform::signal_label(signal))
}

/// Ends the process like `status`: by the same signal on Unix, with the same
/// code otherwise. Call it last, after every cleanup.
pub fn exit_like(status: ExitStatus) -> ! {
    platform::exit_like(status)
}

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
    use super::{Received, SignalSource, Supervised, relays};
    use signal_hook::consts::signal::{SIGCHLD, SIGHUP, SIGINT, SIGQUIT, SIGTERM};
    use signal_hook::iterator::SignalsInfo;
    use signal_hook::iterator::exfiltrator::WithOrigin;
    use signal_hook::low_level::siginfo::Cause;
    use std::ffi::c_int;
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;

    pub(super) type Inner = SignalsInfo<WithOrigin>;

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
        Ok(SignalSource { inner: SignalsInfo::<WithOrigin>::new(&watched)?, queued: None })
    }

    pub(super) fn next(signals: &mut Inner) -> Option<Received> {
        signals
            .forever()
            .next()
            .map(|origin| Received { signal: origin.signal, from_kernel: origin.cause == Cause::Kernel })
    }

    pub(super) fn pending(signals: &mut Inner) -> Vec<c_int> {
        signals.pending().map(|origin| origin.signal).collect()
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

    pub(super) fn signal_label(signal: c_int) -> &'static str {
        signal_hook::low_level::signal_name(signal).unwrap_or("a signal")
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
        // SAFETY: the pointer is to a local that outlives the call.
        unsafe {
            let limit = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            libc::setrlimit(libc::RLIMIT_CORE, &limit);
        }
    }

    /// Relays `received` to the direct child, unless the terminal already
    /// delivered it.
    pub(super) fn relay(child: &Supervised, received: Received) {
        if decide(child, received) {
            send(child.process.id() as libc::pid_t, received.signal);
        }
    }

    /// Applies [`relays`] to `received` and logs the decision. Returns
    /// whether to send. When the child's group cannot be read (the child is
    /// gone), nothing is sent: the wait loop sees the exit next.
    fn decide(child: &Supervised, received: Received) -> bool {
        let pid = child.process.id() as libc::pid_t;
        let Some(shares_group) = shares_group(pid) else {
            return false;
        };
        let name = signal_hook::low_level::signal_name(received.signal).unwrap_or("unknown signal");
        if relays(received.signal, received.from_kernel, shares_group) {
            let cause = if received.from_kernel { "kernel" } else { "other" };
            log::debug!("relayed {name} to pid {pid} (cause: {cause})");
            true
        } else {
            log::debug!("skipped {name}, the child got it from the terminal");
            false
        }
    }

    /// Whether the child is still in Bear's process group. `None` when
    /// `getpgid` fails.
    fn shares_group(pid: libc::pid_t) -> Option<bool> {
        // SAFETY: both calls take no pointers and only read process state.
        let (child, own) = unsafe { (libc::getpgid(pid), libc::getpgrp()) };
        (child != -1).then_some(child == own)
    }

    /// Best-effort delivery of a signal to a process.
    ///
    /// Signal forwarding is best effort: a failure to deliver must not turn
    /// into a hard supervision error (only spawn/wait failures are fatal).
    /// `ESRCH` - the target already exited, the normal exit-then-signal race -
    /// is benign and logged at debug; any other errno is logged at error.
    fn send(pid: libc::pid_t, signal: libc::c_int) {
        // SAFETY: kill() is an async-signal-safe libc call taking a
        // plain integer; it cannot violate memory safety. We inspect errno
        // only on the documented -1 return.
        let rc = unsafe { libc::kill(pid, signal) };
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
    use super::{Received, SignalSource, Supervised};
    use std::ffi::c_int;
    use std::process::ExitStatus;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    pub(super) type Inner = Arc<AtomicUsize>;

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
        Ok(SignalSource { inner: signaled, queued: None })
    }

    pub(super) fn next(signaled: &mut Inner) -> Option<Received> {
        thread::sleep(POLL);
        Some(Received { signal: signaled.swap(0, Ordering::SeqCst) as c_int, from_kernel: false })
    }

    pub(super) fn pending(signaled: &mut Inner) -> Vec<c_int> {
        vec![signaled.swap(0, Ordering::SeqCst) as c_int]
    }

    /// What a console program stopped by Ctrl-C returns.
    const STATUS_CONTROL_C_EXIT: u32 = 0xC000013A;

    pub(super) fn die(_signal: c_int) -> ! {
        std::process::exit(STATUS_CONTROL_C_EXIT as i32)
    }

    pub(super) fn signal_label(signal: c_int) -> &'static str {
        if signal == signal_hook::consts::SIGINT { "Ctrl-C" } else { "SIGTERM" }
    }

    pub(super) fn exit_like(status: ExitStatus) -> ! {
        std::process::exit(status.code().unwrap_or(1))
    }

    /// The console gives Ctrl-C to every attached process, so nothing is
    /// relayed.
    pub(super) fn relay(_child: &Supervised, _received: Received) {}
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
    // has exited. The relay is verified end to end in tests/integration
    // (signals.rs).
    use super::*;
    use fixture::{shell, shell_in_own_group};
    use std::os::unix::process::ExitStatusExt;

    #[test]
    fn queued_signals_are_never_from_the_kernel() {
        // arrange / act / assert per case: what was queued, in order
        let cases: [&[c_int]; 3] = [&[libc::SIGINT], &[libc::SIGCHLD, libc::SIGQUIT, libc::SIGTERM], &[]];
        for signals in cases {
            let sut = queued(signals.to_vec());

            let expected: Vec<Received> =
                signals.iter().map(|signal| Received { signal: *signal, from_kernel: false }).collect();
            assert_eq!(sut, expected, "signals: {signals:?}");
            for received in sut {
                assert!(relays(received.signal, received.from_kernel, true), "{received:?} is skipped");
            }
        }
    }

    #[test]
    fn abort_warning_names_the_signal() {
        let cases = [
            (libc::SIGINT, "interrupted by SIGINT; stopped before finishing"),
            (libc::SIGTERM, "interrupted by SIGTERM; stopped before finishing"),
            (libc::SIGHUP, "interrupted by SIGHUP; stopped before finishing"),
            (libc::SIGQUIT, "interrupted by SIGQUIT; stopped before finishing"),
        ];
        for (signal, expected) in cases {
            let sut = abort_warning(signal);

            assert_eq!(sut, expected, "signal: {signal}");
        }
    }

    #[test]
    fn first_termination_picks_the_first_termination_signal() {
        let cases: [(&[c_int], Option<c_int>); 4] = [
            (&[libc::SIGCHLD, libc::SIGTERM, libc::SIGINT], Some(libc::SIGTERM)),
            (&[libc::SIGHUP], Some(libc::SIGHUP)),
            (&[libc::SIGCHLD, libc::SIGCHLD], None),
            (&[], None),
        ];
        for (signals, expected) in cases {
            let sut = first_termination(signals);

            assert_eq!(sut, expected, "signals: {signals:?}");
        }
    }

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
            shell_with(script, false)
        }

        /// Like [`shell`], with the child in a process group of its own, as
        /// `timeout` leaves a build tool.
        pub(super) fn shell_in_own_group(script: &str) -> (Command, Probe) {
            shell_with(script, true)
        }

        fn shell_with(script: &str, own_group: bool) -> (Command, Probe) {
            let (stdin_reader, stdin_writer) = std::io::pipe().expect("stdin pipe");
            let (stdout_reader, stdout_writer) = std::io::pipe().expect("stdout pipe");
            let mut command = Command::new("sh");
            command.arg("-c").arg(script).stdin(stdin_reader).stdout(stdout_writer);
            if own_group {
                command.process_group(0);
            }
            // SAFETY: runs in the child between fork and exec, and calls
            // only `signal`, which is async-signal-safe.
            unsafe {
                command.pre_exec(|| {
                    for signal in super::super::platform::TERMINATION {
                        libc::signal(signal, libc::SIG_DFL);
                    }
                    Ok(())
                });
            }
            (command, Probe { stdin: stdin_writer, stdout: BufReader::new(stdout_reader) })
        }

        impl Probe {
            /// Stands in for `SIGCHLD` only: every item comes once the
            /// child has exited.
            pub(super) fn exits(self) -> impl Iterator<Item = Received> {
                self.signal_then_exits(None)
            }

            /// `signal` once the child is ready, then the `SIGCHLD`
            /// stand-in.
            pub(super) fn signal(self, signal: c_int) -> impl Iterator<Item = Received> {
                self.signal_then_exits(Some(Received { signal, from_kernel: false }))
            }

            /// Lets the child past its `read`, then ends, so supervise
            /// is left with nothing to wait on but the child itself.
            pub(super) fn release(mut self) -> impl Iterator<Item = Received> {
                iter::from_fn(move || {
                    self.stdin.write_all(b"go\n").expect("write to the child");
                    None
                })
            }

            /// `signal` once the child is ready. Asked for the next item,
            /// lets the child past its `read` and ends.
            pub(super) fn signal_then_release(
                mut self,
                received: Received,
            ) -> impl Iterator<Item = Received> {
                let mut ready = false;
                iter::from_fn(move || {
                    if !ready {
                        self.ready();
                        ready = true;
                        return Some(received);
                    }
                    self.stdin.write_all(b"go\n").expect("write to the child");
                    None
                })
            }

            fn signal_then_exits(mut self, signal: Option<Received>) -> impl Iterator<Item = Received> {
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
        fn wait_for_exit(pid: libc::pid_t) -> Option<Received> {
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
                    return Some(Received { signal: libc::SIGCHLD, from_kernel: false });
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

            let sut = supervise(&mut command, probe.exits()).expect("supervise failed").status;

            assert_eq!(sut.success(), expect_success, "exit {code} success mismatch");
            assert_eq!(sut.code(), Some(code), "exit {code} code mismatch");
        }
    }

    #[test]
    fn ended_source_waits_for_the_child() {
        let (mut command, probe) = shell("read _; exit 3");

        let sut = supervise(&mut command, probe.release()).expect("supervise failed").status;

        assert_eq!(sut.code(), Some(3));
    }

    #[test]
    fn relays_the_received_signal() {
        // arrange / act / assert per case: the signal Bear got reaches the child
        let cases = [libc::SIGTERM, libc::SIGINT];
        for signal in cases {
            let (mut command, probe) = shell("echo $$; exec cat");

            let sut = supervise(&mut command, probe.signal(signal)).expect("supervise failed").status;

            assert_eq!(sut.signal(), Some(signal), "signal {signal}");
        }
    }

    #[test]
    fn a_trap_can_finish() {
        // A shell runs a trap only once its foreground command ends. The
        // background `sleep` is what the shell waits on, and the trap
        // stops it, so no orphan is left.
        let (mut command, probe) = shell("trap 'kill $!; exit 5' TERM; sleep 60 & echo $$; wait; exit 0");

        let sut = supervise(&mut command, probe.signal(libc::SIGTERM)).expect("supervise failed");

        assert!(sut.interrupted);
        assert_eq!(sut.status.code(), Some(5));
    }

    #[test]
    fn waits_for_a_build_that_ignores_the_signal() {
        let (mut command, probe) = shell("trap '' TERM; echo $$; read _; exit 3");

        let sut = supervise(
            &mut command,
            probe.signal_then_release(Received { signal: libc::SIGTERM, from_kernel: false }),
        )
        .expect("supervise failed")
        .status;

        assert_eq!(sut.code(), Some(3));
    }

    #[test]
    fn relay_rule_skips_only_a_terminal_key_the_child_got() {
        // arrange / act / assert per case: (signal, from kernel, shares group, relayed)
        let cases = [
            (libc::SIGINT, true, true, false),
            (libc::SIGQUIT, true, true, false),
            (libc::SIGINT, true, false, true),
            (libc::SIGQUIT, true, false, true),
            (libc::SIGINT, false, true, true),
            (libc::SIGQUIT, false, false, true),
            (libc::SIGTERM, true, true, true),
            (libc::SIGTERM, false, true, true),
            (libc::SIGTERM, true, false, true),
            (libc::SIGHUP, true, true, true),
            (libc::SIGHUP, false, true, true),
            (libc::SIGHUP, true, false, true),
        ];
        for (signal, from_kernel, shares_group, expected) in cases {
            let sut = relays(signal, from_kernel, shares_group);

            assert_eq!(sut, expected, "signal {signal}, kernel {from_kernel}, shares group {shares_group}");
        }
    }

    /// The child ends with 3 when the test releases it, and with 4 when a
    /// relayed signal reaches its trap. A trap, not the default action: a
    /// shell may ignore `SIGQUIT` until it has exec'd.
    const WAITS_FOR_RELEASE: &str = "trap 'exit 4' HUP INT QUIT; echo $$; read _; exit 3";

    #[test]
    fn kernel_signal_is_skipped_for_a_child_in_the_same_group() {
        // arrange / act / assert per case: the terminal key reached the child already
        let cases = [libc::SIGINT, libc::SIGQUIT];
        for signal in cases {
            let (mut command, probe) = shell(WAITS_FOR_RELEASE);
            let received = Received { signal, from_kernel: true };

            let sut = supervise(&mut command, probe.signal_then_release(received)).expect("supervise failed");

            assert!(sut.interrupted);
            assert_eq!(sut.status.code(), Some(3), "signal {signal} was relayed");
        }
    }

    #[test]
    fn kernel_signal_is_relayed_to_a_child_in_its_own_group() {
        // arrange / act / assert per case: the terminal did not reach the child
        let cases = [libc::SIGINT, libc::SIGQUIT];
        for signal in cases {
            let (mut command, probe) = shell_in_own_group(WAITS_FOR_RELEASE);
            let received = Received { signal, from_kernel: true };

            let sut = supervise(&mut command, probe.signal_then_release(received)).expect("supervise failed");

            assert_eq!(sut.status.code(), Some(4), "signal {signal}");
        }
    }

    #[test]
    fn process_signal_and_kernel_hangup_are_relayed_in_both_groups() {
        // arrange / act / assert per case: (signal, from kernel)
        let cases = [(libc::SIGINT, false), (libc::SIGHUP, true)];
        for (signal, from_kernel) in cases {
            let received = Received { signal, from_kernel };

            let (mut command, probe) = shell(WAITS_FOR_RELEASE);
            let shared =
                supervise(&mut command, probe.signal_then_release(received)).expect("supervise failed");
            let (mut command, probe) = shell_in_own_group(WAITS_FOR_RELEASE);
            let own = supervise(&mut command, probe.signal_then_release(received)).expect("supervise failed");

            assert_eq!(shared.status.code(), Some(4), "same group, {received:?}");
            assert_eq!(own.status.code(), Some(4), "own group, {received:?}");
        }
    }
}
