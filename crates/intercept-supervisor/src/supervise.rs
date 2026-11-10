// SPDX-License-Identifier: GPL-3.0-or-later

//! Requirements: cli-signal-forwarding
//!
//! Runs a build (or a single compiler call) as a child process, waits for
//! it, and tears it down when Bear gets a termination signal. The steps
//! that differ between the outermost supervisor and a nested one live in
//! [`Leader`] and [`Inherit`], the two implementations of [`GroupPolicy`],
//! picked at the call site as a type parameter.
//!
//! The process registers the signals once, with [`signals`], and passes the
//! source to every supervision. On Unix the source blocks on `SIGCHLD` and
//! the termination signals; on Windows it polls, and both policies stop
//! only the direct child.

use std::ffi::c_int;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus};
use thiserror::Error;

/// This method supervises the execution of a command.
///
/// It starts the command and waits for its completion. While waiting it
/// forwards termination signals from `signals` to the build so the build
/// stops too, and propagates the build's exit status back to the caller.
/// How the build is torn down is up to the policy `P`: [`Leader`] or
/// [`Inherit`].
pub fn supervise<P: GroupPolicy>(
    command: &mut Command,
    signals: impl IntoIterator<Item = c_int>,
) -> Result<ExitStatus, SuperviseError> {
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
        return Ok(status);
    }

    // Wait for a termination signal, the teardown trigger. Any other item
    // (`SIGCHLD` on Unix, a poll tick on Windows) means the child may have
    // exited.
    let signal = loop {
        match signals.next() {
            Some(signal) if signal_hook::consts::TERM_SIGNALS.contains(&signal) => break signal,
            Some(_) => {
                if let Some(status) = child.try_wait()? {
                    return Ok(status);
                }
            }
            // The source ended: nothing is left to forward, so block until
            // the child is reaped.
            None => return child.wait(),
        }
    };

    policy.teardown(&mut child, signal)
}

/// Registers the signals a supervision reacts to and returns them as a
/// source for [`supervise`].
///
/// Call it once per process, before the first spawn: a signal that arrives
/// after the registration waits in the source, so none is lost. The
/// handlers stay installed for the life of the process. Every item that is
/// not a termination signal means the child may have exited.
pub fn signals() -> std::io::Result<impl Iterator<Item = c_int>> {
    platform::signals()
}

/// How a supervisor relates to the build's process group: it sets up the
/// group before the spawn, and decides how the build is torn down after a
/// termination signal.
///
/// In wrapper mode the supervision chain nests
/// (`bear-driver` -> `make` -> `bear-wrapper` -> real `cc`); only the
/// outermost supervisor may create a process group. If every level created
/// its own group, a top-level group kill would miss the deeper processes.
///
/// Implemented by [`Leader`] and [`Inherit`]; the policy is chosen at the
/// call site as a type parameter of [`supervise`].
pub trait GroupPolicy: Sized {
    /// Runs before the spawn; the returned value lives until the
    /// supervision ends.
    fn prepare(command: &mut Command) -> Self;

    /// Runs once, after the first termination signal; returns when the
    /// direct child is reaped.
    fn teardown(self, child: &mut Supervised, signal: c_int) -> Result<ExitStatus, SuperviseError>;
}

/// The outermost supervisor (the driver). Creates a new process group with
/// the child as leader and owns the authoritative escalation: forward the
/// received signal to the whole group, grant a grace window, then escalate
/// to `SIGKILL`.
pub struct Leader {
    #[cfg(unix)]
    cgroup: Option<cgroup::Cgroup>,
}

/// A nested supervisor (the wrapper). Stays in the inherited group and only
/// relays the received signal to its direct child; the driver's group kill
/// is the authoritative teardown, so it runs no grace/`SIGKILL` timer.
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
    #[error("cannot kill '{executable}': {source}", executable = executable.display())]
    ProcessKill {
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
    use super::cgroup;
    use super::{GroupPolicy, Inherit, Leader, SuperviseError, Supervised};
    use signal_hook::consts::signal::SIGCHLD;
    use signal_hook::iterator::Signals;
    use std::ffi::c_int;
    use std::process::{Command, ExitStatus};
    use std::time::{Duration, Instant};

    /// How long the build's process tree is allowed to wind down on its own
    /// after the real termination signal before the `Leader` escalates to
    /// `SIGKILL`. A cooperating tree ends the window early by exiting, so
    /// only a build that ignores the signal pays the full wait; sized to
    /// the largest value that keeps that case inside the requirement's
    /// sub-one-second teardown budget, because the headroom is what keeps
    /// a trap on a starved CI runner from being SIGKILLed mid-flight
    /// (issue #713).
    const GRACE: Duration = Duration::from_millis(800);

    /// The termination signals plus `SIGCHLD`, so a wait on the source
    /// wakes when the build wants to be torn down or the child exits.
    /// signal-hook's self-pipe keeps a signal delivered after the
    /// registration until it is read.
    pub(super) fn signals() -> std::io::Result<impl Iterator<Item = c_int>> {
        let mut watched = signal_hook::consts::TERM_SIGNALS.to_vec();
        watched.push(SIGCHLD);
        let mut signals = Signals::new(&watched)?;
        Ok(std::iter::from_fn(move || signals.forever().next()))
    }

    impl GroupPolicy for Leader {
        fn prepare(command: &mut Command) -> Self {
            use std::os::unix::process::CommandExt;

            // The leader owns the build's process group so a single killpg
            // reaps the whole tree. process_group(0) is safe std: the child
            // becomes a new group leader (pgid == child pid) before exec.
            command.process_group(0);

            // On Linux, additionally place the build in a fresh cgroup so a
            // descendant that re-`setsid`s out of the process group is still
            // reaped on escalation. `create_and_attach` returns `None` (and
            // teardown falls back to the process-group `SIGKILL`) where
            // cgroup v2 is unavailable or its directory is not
            // writable/delegated. Nested wrappers inherit it through the child.
            Leader { cgroup: cgroup::create_and_attach(command) }
        }

        /// Deliver the real signal to the whole group, grant a grace window,
        /// then force whatever is still alive. The leader runs the single
        /// authoritative escalation timer for the supervision chain.
        fn teardown(self, child: &mut Supervised, signal: c_int) -> Result<ExitStatus, SuperviseError> {
            let pgid = child.process.id() as libc::pid_t;
            log::debug!("Received signal {signal}, forwarding to process group {pgid}");
            send(SendTarget::Group(pgid), signal);

            // Let the tree wind down on its own (run traps, drain in-flight work)
            // before forcing it. The direct child exiting ends the grace early,
            // but a cgroup is still swept below: a detached descendant can outlive
            // the child that exited cleanly.
            let deadline = Instant::now() + GRACE;
            let mut exited = None;
            loop {
                if let Some(status) = child.try_wait()? {
                    exited = Some(status);
                    break;
                }
                if Instant::now() >= deadline {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }

            if exited.is_none() {
                // One stable line across both escalation paths (cgroup and
                // process-group): tests and triage key on it to distinguish a
                // starved grace window from a build that exited in time.
                log::debug!("Build did not exit within the grace window ({GRACE:?})");
            }

            // Force anything still alive. A cgroup kill reaps the whole cgroup,
            // including a descendant that left the process group (a daemon that
            // called `setsid`), so it runs even when the direct child already
            // exited. Without a cgroup, fall back to forcing the process group -
            // but only when something is still there to force.
            match self.cgroup.as_ref() {
                Some(cgroup) => {
                    log::debug!("Killing build cgroup to reap the whole tree");
                    cgroup.kill();
                }
                None if exited.is_none() => {
                    log::debug!("Grace window elapsed, sending SIGKILL to process group {pgid}");
                    send(SendTarget::Group(pgid), libc::SIGKILL);
                }
                None => {}
            }

            if let Some(status) = exited {
                return Ok(status);
            }

            // Nothing left to forward; block until the direct child is reaped. A
            // blocking wait (not a poll loop) avoids burning CPU and does not hang
            // any worse than the kernel itself: if SIGKILL cannot collect the child
            // (e.g. uninterruptible sleep), no caller could have done better.
            child.wait()
        }
    }

    impl GroupPolicy for Inherit {
        fn prepare(_command: &mut Command) -> Self {
            Inherit
        }

        /// Relay the real signal to the direct child only and wait for it to
        /// be reaped. The leader's group kill is authoritative, so no grace
        /// or `SIGKILL` timer runs here.
        fn teardown(self, child: &mut Supervised, signal: c_int) -> Result<ExitStatus, SuperviseError> {
            let child_pid = child.process.id() as libc::pid_t;
            log::debug!("Received signal {signal}, forwarding to child {child_pid}");
            send(SendTarget::Process(child_pid), signal);

            // A repeated termination signal needs no second relay, and a
            // child that does not stop is reaped by the leader's group kill,
            // so a blocking wait is enough.
            child.wait()
        }
    }

    enum SendTarget {
        Group(libc::pid_t),
        Process(libc::pid_t),
    }

    /// Best-effort delivery of a signal to a process or process group.
    ///
    /// Signal forwarding is best effort: a failure to deliver must not turn
    /// into a hard supervision error (only spawn/wait failures are fatal).
    /// `ESRCH` - the target already exited, the normal exit-then-signal race -
    /// is benign and logged at debug; any other errno is logged at error.
    fn send(target: SendTarget, signal: libc::c_int) {
        // SAFETY: kill()/killpg() are async-signal-safe libc calls taking
        // plain integers; they cannot violate memory safety. We inspect errno
        // only on the documented -1 return.
        let rc = unsafe {
            match target {
                SendTarget::Group(pgid) => libc::killpg(pgid, signal),
                SendTarget::Process(pid) => libc::kill(pid, signal),
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
    use super::{GroupPolicy, Inherit, Leader, SuperviseError, Supervised};
    use std::ffi::c_int;
    use std::process::{Command, ExitStatus};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    /// There is no `SIGCHLD` to wake the wait when the child exits, so the
    /// source wakes it on this interval instead.
    const POLL: Duration = Duration::from_millis(100);

    /// A termination signal is recorded in a flag. The source sleeps one
    /// `POLL`, then yields the recorded signal, or `0` when none arrived,
    /// which the wait reads as "check the child".
    pub(super) fn signals() -> std::io::Result<impl Iterator<Item = c_int>> {
        let signaled = Arc::new(AtomicUsize::new(0));
        for signal in signal_hook::consts::TERM_SIGNALS {
            signal_hook::flag::register_usize(*signal, Arc::clone(&signaled), *signal as usize)?;
        }
        Ok(std::iter::from_fn(move || {
            thread::sleep(POLL);
            Some(signaled.swap(0, Ordering::SeqCst) as c_int)
        }))
    }

    impl GroupPolicy for Leader {
        fn prepare(_command: &mut Command) -> Self {
            Leader {}
        }

        fn teardown(self, child: &mut Supervised, _signal: c_int) -> Result<ExitStatus, SuperviseError> {
            kill(child)
        }
    }

    impl GroupPolicy for Inherit {
        fn prepare(_command: &mut Command) -> Self {
            Inherit
        }

        fn teardown(self, child: &mut Supervised, _signal: c_int) -> Result<ExitStatus, SuperviseError> {
            kill(child)
        }
    }

    /// Process groups are out of scope here, so both policies stop only the
    /// direct child.
    fn kill(child: &mut Supervised) -> Result<ExitStatus, SuperviseError> {
        log::debug!("Received signal, forwarding to child process");
        child.process.kill().map_err(|err| SuperviseError::ProcessKill {
            executable: child.executable.clone(),
            source: err,
        })?;
        child.wait()
    }
}

/// Linux cgroup v2 teardown: closes the one hole process groups leave open -
/// a descendant that calls `setsid` escapes the group but cannot leave the
/// cgroup unprivileged, so killing the cgroup reaps it too.
#[cfg(all(unix, target_os = "linux"))]
mod cgroup {
    use std::fs;
    use std::io;
    use std::os::fd::{AsRawFd, OwnedFd, RawFd};
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::Duration;

    /// A freshly created cgroup v2 directory owning one supervised build.
    /// Dropping it removes the directory, so a normal build leaves nothing
    /// behind.
    pub(crate) struct Cgroup {
        path: PathBuf,
    }

    /// Create a child cgroup for `command` and arrange the spawned process to
    /// join it before `exec`. Best effort: returns `None` (caller falls back
    /// to process-group teardown) when cgroup v2 is unavailable, not writable,
    /// or too old to expose `cgroup.kill`.
    pub(crate) fn create_and_attach(command: &mut Command) -> Option<Cgroup> {
        let cgroup = Cgroup::create()?;
        if let Err(err) = cgroup.attach(command) {
            log::debug!("cgroup attach failed ({err}); using process-group teardown");
            return None; // `cgroup` drops here and removes the directory
        }
        Some(cgroup)
    }

    impl Cgroup {
        fn create() -> Option<Cgroup> {
            let relative = own_v2_path()?;
            let path = Path::new("/sys/fs/cgroup")
                .join(relative.trim_start_matches('/'))
                .join(format!("bear-{:016x}", random_u64()));
            if let Err(err) = fs::create_dir(&path) {
                log::debug!("cgroup unavailable ({}: {err}); using process-group teardown", path.display());
                return None;
            }
            let cgroup = Cgroup { path };
            // `cgroup.kill` exists in every cgroup since kernel 5.14; its
            // absence means the kernel is too old for this path.
            if !cgroup.path.join("cgroup.kill").exists() {
                log::debug!("cgroup.kill missing (kernel too old); using process-group teardown");
                return None; // `cgroup` drops here and removes the directory
            }
            Some(cgroup)
        }

        /// Move the about-to-be-spawned child into this cgroup from a
        /// `pre_exec` hook, so its whole tree starts inside the cgroup with no
        /// race window. The `cgroup.procs` file is opened in the parent and
        /// only written (an async-signal-safe operation) in the child.
        fn attach(&self, command: &mut Command) -> io::Result<()> {
            let file = fs::OpenOptions::new()
                .write(true)
                .custom_flags(libc::O_CLOEXEC)
                .open(self.path.join("cgroup.procs"))?;
            let fd: OwnedFd = file.into();
            // SAFETY: the closure runs in the child between fork and exec and
            // calls only async-signal-safe libc (getpid, write).
            unsafe {
                command.pre_exec(move || {
                    let pid = libc::getpid();
                    write_pid(fd.as_raw_fd(), pid)
                });
            }
            Ok(())
        }

        /// Kill every process in the cgroup with a single write. Best effort,
        /// like signal forwarding: a failure is logged, not surfaced.
        pub(crate) fn kill(&self) {
            if let Err(err) = fs::write(self.path.join("cgroup.kill"), b"1") {
                log::error!("failed to kill cgroup {}: {err}", self.path.display());
            }
        }
    }

    impl Drop for Cgroup {
        fn drop(&mut self) {
            // A cgroup directory is removable only once empty; killed
            // processes leave it as init reaps them, which can lag under load,
            // so retry up to ~500ms before giving up and leaving the dir.
            for _ in 0..100 {
                match fs::remove_dir(&self.path) {
                    Ok(()) => return,
                    Err(err) if err.raw_os_error() == Some(libc::EBUSY) => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(err) => {
                        log::debug!("could not remove cgroup {}: {err}", self.path.display());
                        return;
                    }
                }
            }
            log::debug!("cgroup {} still busy after retries; leaving it", self.path.display());
        }
    }

    /// A random name part, so two supervisions in one process (parallel
    /// unit tests) never share a cgroup. std's `RandomState` is seeded
    /// randomly per thread and changes keys with every instance, so no
    /// dependency is needed.
    fn random_u64() -> u64 {
        use std::hash::{BuildHasher, Hasher};
        std::collections::hash_map::RandomState::new().build_hasher().finish()
    }

    /// Our own cgroup v2 path, read from the `0::<path>` line of
    /// `/proc/self/cgroup`. `None` on a host without a unified hierarchy.
    fn own_v2_path() -> Option<String> {
        let content = fs::read_to_string("/proc/self/cgroup").ok()?;
        content.lines().find_map(|line| line.strip_prefix("0::").map(str::to_string))
    }

    /// Write a pid as decimal bytes to `fd` without allocating, so it is safe
    /// to call from the post-fork child.
    fn write_pid(fd: RawFd, pid: libc::pid_t) -> io::Result<()> {
        let mut buf = [0u8; 20];
        let mut at = buf.len();
        let mut value = pid as u64;
        if value == 0 {
            at -= 1;
            buf[at] = b'0';
        }
        while value > 0 {
            at -= 1;
            buf[at] = b'0' + (value % 10) as u8;
            value /= 10;
        }
        let bytes = &buf[at..];
        let mut written = 0;
        while written < bytes.len() {
            // SAFETY: async-signal-safe write to a valid, inherited fd.
            let rc = unsafe { libc::write(fd, bytes[written..].as_ptr().cast(), bytes.len() - written) };
            if rc < 0 {
                let err = io::Error::last_os_error();
                // A signal interrupting the write must not fail the spawn:
                // returning Err here aborts the whole supervised run.
                if err.raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                return Err(err);
            }
            written += rc as usize;
        }
        Ok(())
    }
}

/// Non-Linux unix has no cgroups; teardown stays process-group only. The type
/// is uninhabited so the leader-teardown signature is uniform across unix.
#[cfg(all(unix, not(target_os = "linux")))]
mod cgroup {
    pub(crate) enum Cgroup {}

    impl Cgroup {
        pub(crate) fn kill(&self) {
            match *self {}
        }
    }

    pub(crate) fn create_and_attach(_command: &mut std::process::Command) -> Option<Cgroup> {
        None
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

            let sut = supervise::<Leader>(&mut command, probe.exits()).expect("supervise failed");

            assert_eq!(sut.success(), expect_success, "exit {code} success mismatch");
            assert_eq!(sut.code(), Some(code), "exit {code} code mismatch");
        }
    }

    #[test]
    fn ended_source_waits_for_the_child() {
        let (mut command, probe) = shell("read _; exit 3");

        let sut = supervise::<Leader>(&mut command, probe.release()).expect("supervise failed");

        assert_eq!(sut.code(), Some(3));
    }

    #[test]
    fn leader_forwards_the_received_signal() {
        // arrange / act / assert per case: the signal Bear got reaches the child
        let cases = [libc::SIGTERM, libc::SIGINT];
        for signal in cases {
            let (mut command, probe) = shell("echo $$; exec cat");

            let sut = supervise::<Leader>(&mut command, probe.signal(signal)).expect("supervise failed");

            assert_eq!(sut.signal(), Some(signal), "signal {signal}");
        }
    }

    #[test]
    fn inherit_relays_the_received_signal() {
        // arrange / act / assert per case: the signal Bear got reaches the child
        let cases = [libc::SIGTERM, libc::SIGINT];
        for signal in cases {
            let (mut command, probe) = shell("echo $$; exec cat");

            let sut = supervise::<Inherit>(&mut command, probe.signal(signal)).expect("supervise failed");

            assert_eq!(sut.signal(), Some(signal), "signal {signal}");
        }
    }

    #[test]
    fn leader_lets_a_trap_finish_within_the_grace_window() {
        // A shell runs a trap only once its foreground command ends, so a
        // foreground `cat` forked after the signal would never end. The
        // background `cat` exists before the pid is printed, so the group
        // signal reaches it. It reads fd 3 because a background job's stdin
        // is /dev/null.
        let (mut command, probe) = shell("exec 3<&0; trap 'exit 5' TERM; cat <&3 & echo $$; wait; exit 0");

        let sut = supervise::<Leader>(&mut command, probe.signal(libc::SIGTERM)).expect("supervise failed");

        assert_eq!(sut.code(), Some(5));
    }

    #[test]
    fn leader_kills_a_build_that_ignores_the_signal() {
        let (mut command, probe) = shell("trap '' TERM; echo $$; exec cat");

        let sut = supervise::<Leader>(&mut command, probe.signal(libc::SIGTERM)).expect("supervise failed");

        assert_eq!(sut.signal(), Some(libc::SIGKILL));
    }
}
