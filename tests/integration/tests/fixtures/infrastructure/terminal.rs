// SPDX-License-Identifier: GPL-3.0-or-later

//! A pseudo-terminal that a test process runs on, as a user's terminal would.
//!
//! The command becomes the session leader of the terminal (`login_tty`), so
//! bytes typed with [`Terminal::type_bytes`] go through the line discipline:
//! `0x03` becomes SIGINT for the foreground process group, and
//! [`Terminal::hang_up`] is the hang-up of a closed terminal window.
//!
//! Distro build sandboxes often have no `/dev/pts`. [`Terminal::spawn_or_skip`]
//! turns that into a printed skip, unless `BEAR_TEST_REQUIRE_PTY` is set.

use anyhow::{Result, bail};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Ends a hung test: after this long the whole terminal session is killed.
const HANG_DEADLINE: Duration = Duration::from_secs(60);

pub enum SpawnError {
    NoPty(std::io::Error),
    Spawn(std::io::Error),
}

pub struct Terminal {
    child: Child,
    writer: Option<File>,
    output: Arc<Mutex<Vec<u8>>>,
    closed: Arc<AtomicBool>,
    drain: Option<JoinHandle<()>>,
    deadline: Option<mpsc::Sender<()>>,
}

/// Opens a pty pair as (master, slave), both close-on-exec.
///
/// The flag is set after `openpty`, so a test that spawns a process in that
/// short window on another thread can inherit the descriptors. This race is
/// accepted: `posix_openpt` with `O_CLOEXEC` is not portable to macOS.
pub fn open_pty() -> std::io::Result<(OwnedFd, OwnedFd)> {
    let (mut master, mut slave) = (0, 0);
    // SAFETY: the out-pointers are valid and the optional arguments are null.
    let rc = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: openpty returned two fresh descriptors that nothing else owns.
    let (master, slave) = unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };
    for fd in [&master, &slave] {
        // SAFETY: the descriptor is open for the duration of the call.
        if unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok((master, slave))
}

impl Terminal {
    pub fn spawn(mut command: Command) -> Result<Terminal, SpawnError> {
        let (master, slave) = open_pty().map_err(SpawnError::NoPty)?;
        let slave_fd = slave.as_raw_fd();
        // SAFETY: runs in the child between fork and exec and calls only
        // `login_tty`, which makes the child a session leader and moves the
        // slave onto stdin, stdout and stderr.
        unsafe {
            command.pre_exec(move || match libc::login_tty(slave_fd) {
                0 => Ok(()),
                _ => Err(std::io::Error::last_os_error()),
            });
        }
        let writer = File::from(master);
        let reader = writer.try_clone().map_err(SpawnError::Spawn)?;
        let child = command.spawn().map_err(SpawnError::Spawn)?;
        drop(slave);

        let output = Arc::new(Mutex::new(Vec::new()));
        let closed = Arc::new(AtomicBool::new(false));
        let drain = std::thread::spawn({
            let (output, closed) = (output.clone(), closed.clone());
            move || drain(reader, &output, &closed)
        });

        let (deadline, cancel) = mpsc::channel::<()>();
        let session = child.id() as libc::pid_t;
        std::thread::spawn(move || {
            if let Err(RecvTimeoutError::Timeout) = cancel.recv_timeout(HANG_DEADLINE) {
                kill_session(session);
            }
        });

        Ok(Terminal {
            child,
            writer: Some(writer),
            output,
            closed,
            drain: Some(drain),
            deadline: Some(deadline),
        })
    }

    /// Spawns on a pty, or prints a skip and returns `None` when the host
    /// cannot open one. With `BEAR_TEST_REQUIRE_PTY` set a missing pty fails.
    pub fn spawn_or_skip(command: Command) -> Result<Option<Terminal>> {
        match Terminal::spawn(command) {
            Ok(terminal) => Ok(Some(terminal)),
            Err(SpawnError::NoPty(error)) if std::env::var_os("BEAR_TEST_REQUIRE_PTY").is_none() => {
                println!("skipped: no pty available ({error})");
                Ok(None)
            }
            Err(SpawnError::NoPty(error)) => bail!("BEAR_TEST_REQUIRE_PTY is set, but no pty: {error}"),
            Err(SpawnError::Spawn(error)) => bail!("failed to spawn on the pty: {error}"),
        }
    }

    pub fn output(&self) -> String {
        String::from_utf8_lossy(&self.output.lock().unwrap()).into_owned()
    }

    /// Waits until the terminal output contains `text`.
    pub fn wait_for(&self, text: &str, deadline: Duration) -> Result<()> {
        let start = Instant::now();
        while !self.output().contains(text) {
            if start.elapsed() > deadline {
                bail!("timed out waiting for {text:?}; output so far:\n{}", self.output());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }

    /// Types the bytes at the terminal, as a user at the keyboard would.
    pub fn type_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        match self.writer.as_mut() {
            Some(writer) => Ok(writer.write_all(bytes)?),
            None => bail!("the terminal was hung up"),
        }
    }

    /// Closes the master side. The kernel then hangs up the session.
    pub fn hang_up(&mut self) {
        self.closed.store(true, Ordering::SeqCst);
        self.writer = None;
        if let Some(drain) = self.drain.take() {
            let _ = drain.join();
        }
    }

    fn cancel_deadline(&mut self) {
        self.deadline = None;
    }

    /// Waits for the command to end. The hang deadline bounds the wait.
    pub fn wait(&mut self) -> Result<ExitStatus> {
        let status = self.child.wait()?;
        self.cancel_deadline();
        Ok(status)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            kill_session(self.child.id() as libc::pid_t);
            let _ = self.child.wait();
        }
        self.cancel_deadline();
        self.hang_up();
    }
}

fn kill_session(leader: libc::pid_t) {
    // SAFETY: plain syscalls with no memory arguments. The leader's pid is
    // also its process group id.
    unsafe {
        libc::kill(-leader, libc::SIGKILL);
        libc::kill(leader, libc::SIGKILL);
    }
}

/// Reads the master into `output` until the slave side is gone or the
/// terminal is hung up. It polls, because closing a descriptor does not wake
/// a `read` that is blocked on it. The descriptor closes when this returns.
fn drain(mut reader: File, output: &Mutex<Vec<u8>>, closed: &AtomicBool) {
    let mut buffer = [0u8; 4096];
    while !closed.load(Ordering::SeqCst) {
        let mut poll = libc::pollfd { fd: reader.as_fd().as_raw_fd(), events: libc::POLLIN, revents: 0 };
        // SAFETY: `poll` is a valid pollfd and the count is one.
        let ready = unsafe { libc::poll(&mut poll, 1, 50) };
        if ready <= 0 {
            continue;
        }
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(n) => output.lock().unwrap().extend_from_slice(&buffer[..n]),
        }
    }
}
