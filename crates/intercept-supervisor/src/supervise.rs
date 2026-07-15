// SPDX-License-Identifier: GPL-3.0-or-later

//! Requirements: cli-signal-forwarding

use intercept::Execution;
use std::path::PathBuf;
use std::process::ExitStatus;
use thiserror::Error;

/// Whether this supervisor owns the build's process group or merely inherits it.
///
/// In wrapper mode the supervision chain nests
/// (`bear-driver` -> `make` -> `bear-wrapper` -> real `cc`); only the
/// outermost supervisor may create a process group. If every level created
/// its own group, a top-level group kill would miss the deeper processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupPolicy {
    /// The outermost supervisor (the driver). Creates a new process group with
    /// the child as leader and owns the authoritative escalation: forward the
    /// received signal to the whole group, grant a grace window, then escalate
    /// to `SIGKILL`.
    Leader,
    /// A nested supervisor (the wrapper). Stays in the inherited group and only
    /// relays the received signal to its direct child; the driver's group kill
    /// is the authoritative teardown, so it runs no grace/`SIGKILL` timer.
    Inherit,
}

/// This method supervises the execution of a command.
///
/// It starts the command and waits for its completion. While waiting it
/// forwards termination signals it receives to the build so the build stops
/// too, and propagates the build's exit status back to the caller.
///
/// On Unix a `Leader` supervisor places the child in a new process group and,
/// on a termination signal, forwards the real signal to the whole group, grants
/// a short grace window, then escalates to `SIGKILL` - so the entire process
/// tree is torn down, not just the immediate child. An `Inherit` supervisor
/// relays the signal to its direct child only and lets the leader's group kill
/// reap the rest.
pub fn supervise(
    command: &mut std::process::Command,
    policy: GroupPolicy,
) -> Result<ExitStatus, SuperviseError> {
    platform::supervise(command, policy)
}

/// This function supervises the execution of a command represented by the `Execution` struct.
pub fn supervise_execution(execution: Execution, policy: GroupPolicy) -> Result<ExitStatus, SuperviseError> {
    let mut child = command_from_execution(execution)?;
    supervise(&mut child, policy)
}

/// Builds a [`std::process::Command`] from an [`Execution`].
///
/// This is a free function rather than a `TryFrom` impl because the orphan
/// rule forbids implementing the foreign `TryFrom`/`Command` for the foreign
/// `Execution` from this crate.
fn command_from_execution(val: Execution) -> Result<std::process::Command, SuperviseError> {
    let mut command = match val.arguments.as_slice() {
        [] => return Err(SuperviseError::EmptyArguments),
        [_] => std::process::Command::new(val.executable),
        [_, arguments @ ..] => {
            let mut cmd = std::process::Command::new(val.executable);
            cmd.args(arguments);
            cmd
        }
    };

    command.envs(val.environment);
    command.current_dir(val.working_dir);
    Ok(command)
}

#[cfg(unix)]
mod platform {
    use super::cgroup;
    use super::{GroupPolicy, SuperviseError};
    use signal_hook::consts::signal::SIGCHLD;
    use signal_hook::iterator::Signals;
    use std::path::{Path, PathBuf};
    use std::process::{Child, ExitStatus};
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

    pub(super) fn supervise(
        command: &mut std::process::Command,
        policy: GroupPolicy,
    ) -> Result<ExitStatus, SuperviseError> {
        use std::os::unix::process::CommandExt;

        let executable = PathBuf::from(command.get_program());

        // The leader owns the build's process group so a single killpg reaps
        // the whole tree. process_group(0) is safe std: the child becomes a
        // new group leader (pgid == child pid) before exec.
        if policy == GroupPolicy::Leader {
            command.process_group(0);
        }

        // On Linux, additionally place the build in a fresh cgroup so a
        // descendant that re-`setsid`s out of the process group is still
        // reaped on escalation. `create_and_attach` returns `None` (and
        // teardown falls back to the process-group `SIGKILL`) where cgroup v2
        // is unavailable or its directory is not writable/delegated. Only the
        // leader owns a cgroup; nested wrappers inherit it through the child.
        let cgroup = if policy == GroupPolicy::Leader { cgroup::create_and_attach(command) } else { None };

        // Watch the termination signals plus SIGCHLD so the wait below blocks
        // until either the build wants to be torn down or the child exits.
        let mut watched: Vec<libc::c_int> = signal_hook::consts::TERM_SIGNALS.to_vec();
        watched.push(SIGCHLD);
        let mut signals = Signals::new(&watched).map_err(|err| SuperviseError::SignalRegistration {
            executable: executable.clone(),
            source: err,
        })?;

        let mut child = command
            .spawn()
            .map_err(|err| SuperviseError::ProcessSpawn { executable: executable.clone(), source: err })?;
        let child_pid = child.id() as libc::pid_t;

        // Close the child-exited-early race: the child may already be gone
        // before we block on the signal iterator.
        if let Some(status) = try_wait(&mut child, &executable)? {
            return Ok(status);
        }

        // Phase 1: wait for either SIGCHLD (child reaped) or a term signal
        // (the teardown trigger). signal-hook's self-pipe persists a signal
        // delivered after registration, so a signal racing this loop is not
        // lost.
        let mut received: Option<libc::c_int> = None;
        for info in &mut signals {
            if info == SIGCHLD {
                if let Some(status) = try_wait(&mut child, &executable)? {
                    return Ok(status);
                }
            } else {
                received = Some(info);
                break;
            }
        }
        // The loop exits only via the `break` above: we never close the signal
        // handle, and `Signals::forever()` ends only on a closed handle. If
        // signal-hook ever closes it out from under us, there is nothing left to
        // forward, so fall back to blocking until the child is reaped.
        let Some(signal) = received else {
            return child.wait().map_err(|err| SuperviseError::ProcessWait { executable, source: err });
        };

        match policy {
            GroupPolicy::Leader => {
                leader_teardown(&mut child, child_pid, signal, &executable, cgroup.as_ref())
            }
            GroupPolicy::Inherit => inherit_forward(&mut child, child_pid, signal, &executable, &mut signals),
        }
    }

    /// Leader: deliver the real signal to the whole group, grant a grace
    /// window, then force whatever is still alive. The leader runs the single
    /// authoritative escalation timer for the supervision chain.
    fn leader_teardown(
        child: &mut Child,
        pgid: libc::pid_t,
        signal: libc::c_int,
        executable: &Path,
        cgroup: Option<&cgroup::Cgroup>,
    ) -> Result<ExitStatus, SuperviseError> {
        log::debug!("Received signal {signal}, forwarding to process group {pgid}");
        send(SendTarget::Group(pgid), signal);

        // Let the tree wind down on its own (run traps, drain in-flight work)
        // before forcing it. The direct child exiting ends the grace early,
        // but a cgroup is still swept below: a detached descendant can outlive
        // the child that exited cleanly.
        let deadline = Instant::now() + GRACE;
        let mut exited = None;
        loop {
            if let Some(status) = try_wait(child, executable)? {
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
        match cgroup {
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
        child
            .wait()
            .map_err(|err| SuperviseError::ProcessWait { executable: executable.to_path_buf(), source: err })
    }

    /// Inherit: relay the real signal to the direct child only and wait for it
    /// to be reaped. The leader's group kill is authoritative, so no grace or
    /// `SIGKILL` timer runs here.
    fn inherit_forward(
        child: &mut Child,
        child_pid: libc::pid_t,
        signal: libc::c_int,
        executable: &Path,
        signals: &mut Signals,
    ) -> Result<ExitStatus, SuperviseError> {
        log::debug!("Received signal {signal}, forwarding to child {child_pid}");
        send(SendTarget::Process(child_pid), signal);

        if let Some(status) = try_wait(child, executable)? {
            return Ok(status);
        }
        for info in signals {
            if info == SIGCHLD
                && let Some(status) = try_wait(child, executable)?
            {
                return Ok(status);
            }
        }
        // We never close the signal handle, so the loop above only ends if
        // signal-hook closes it itself; in that case block until the child is
        // reaped (the leader's group kill remains the authoritative teardown).
        child
            .wait()
            .map_err(|err| SuperviseError::ProcessWait { executable: executable.to_path_buf(), source: err })
    }

    fn try_wait(child: &mut Child, executable: &Path) -> Result<Option<ExitStatus>, SuperviseError> {
        match child.try_wait() {
            Ok(Some(status)) => {
                log::debug!("Child process exited: {status:?}");
                Ok(Some(status))
            }
            Ok(None) => Ok(None),
            Err(err) => {
                log::error!("Error waiting for child process: {err}");
                Err(SuperviseError::ProcessWait { executable: executable.to_path_buf(), source: err })
            }
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
                .join(format!("bear-{}", std::process::id()));
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

#[cfg(not(unix))]
mod platform {
    use super::{GroupPolicy, SuperviseError};
    use std::path::PathBuf;
    use std::process::ExitStatus;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    /// A blocking `wait()` cannot be interrupted by a signal on this platform
    /// (no `SIGCHLD`), so the term-signal flag is observed by polling. Process
    /// groups are out of scope here, so `GroupPolicy` is ignored and only the
    /// direct child is signalled.
    pub(super) fn supervise(
        command: &mut std::process::Command,
        _policy: GroupPolicy,
    ) -> Result<ExitStatus, SuperviseError> {
        let executable = PathBuf::from(command.get_program());
        let signaled = Arc::new(AtomicUsize::new(0));
        for signal in signal_hook::consts::TERM_SIGNALS {
            signal_hook::flag::register_usize(*signal, Arc::clone(&signaled), *signal as usize).map_err(
                |err| SuperviseError::SignalRegistration { executable: executable.clone(), source: err },
            )?;
        }

        let mut child = command
            .spawn()
            .map_err(|err| SuperviseError::ProcessSpawn { executable: executable.clone(), source: err })?;

        loop {
            if signaled.swap(0usize, Ordering::SeqCst) != 0 {
                log::debug!("Received signal, forwarding to child process");
                child.kill().map_err(|err| SuperviseError::ProcessKill {
                    executable: executable.clone(),
                    source: err,
                })?;
            }

            match child.try_wait() {
                Ok(Some(exit_status)) => {
                    log::debug!("Child process exited: {exit_status:?}");
                    return Ok(exit_status);
                }
                Ok(None) => {
                    thread::sleep(Duration::from_millis(100));
                }
                Err(err) => {
                    log::error!("Error waiting for child process: {err}");
                    return Err(SuperviseError::ProcessWait { executable: executable.clone(), source: err });
                }
            }
        }
    }
}

/// Errors that can occur during process supervision.
#[derive(Error, Debug)]
pub enum SuperviseError {
    #[error("Failed to register signal handler for '{executable}': {source}", executable = executable.display())]
    SignalRegistration {
        executable: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to execute '{executable}': {source}", executable = executable.display())]
    ProcessSpawn {
        executable: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to kill process '{executable}': {source}", executable = executable.display())]
    ProcessKill {
        executable: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to wait for process '{executable}': {source}", executable = executable.display())]
    ProcessWait {
        executable: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Execution arguments cannot be empty")]
    EmptyArguments,
}

#[cfg(all(test, unix))]
mod tests {
    // Requirements: cli-signal-forwarding
    //
    // Only the normal-exit path is unit-tested here. The signal-forwarding
    // paths (real-signal forward, grace window, SIGKILL escalation, whole-tree
    // teardown) are verified end to end against the real driver in the
    // integration suite (tests/integration, exit_codes.rs): driving supervise()
    // in-process is unsafe, because signal-hook handlers are process-global and
    // would bleed across parallel unit tests.
    use super::*;
    use std::process::Command;

    #[test]
    fn normal_exit_propagates_status() {
        // arrange / act / assert per case: (exit code, expected success)
        let cases = [(0, true), (7, false)];
        for (code, expect_success) in cases {
            let mut command = Command::new("sh");
            command.arg("-c").arg(format!("exit {code}"));

            let sut = supervise(&mut command, GroupPolicy::Leader).expect("supervise failed");

            assert_eq!(sut.success(), expect_success, "exit {code} success mismatch");
            assert_eq!(sut.code(), Some(code), "exit {code} code mismatch");
        }
    }
}
