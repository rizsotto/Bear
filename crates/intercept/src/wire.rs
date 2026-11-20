// SPDX-License-Identifier: GPL-3.0-or-later

//! The wire format of an execution report, the address it goes to and the reporter that sends it.

use crate::Execution;
use crate::reporter::{Reporter, ReporterError};
use std::fmt;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
#[cfg(unix)]
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;

/// The serializer for executions to transmit over the network.
///
/// The executions are serialized using LV (Length-Value) format.
/// The length is a 4-byte big-endian integer, and the value is the JSON
/// representation of the execution.
///
/// Both the reporter (sender, in this crate) and the collector (receiver, in
/// `intercept-supervisor`) share this wire format, so the type is `pub`.
pub struct ExecutionWireSerializer;

impl ExecutionWireSerializer {
    /// Read an execution from a reader using LV format.
    pub fn read(reader: &mut impl std::io::Read) -> Result<Execution, ReporterError> {
        let mut length_bytes = [0; 4];
        reader.read_exact(&mut length_bytes)?;
        let length = u32::from_be_bytes(length_bytes) as usize;

        let mut buffer = vec![0; length];
        reader.read_exact(&mut buffer)?;
        let execution = serde_json::from_slice(buffer.as_ref())?;

        Ok(execution)
    }

    /// Encode an execution as one LV record, to send with a single write.
    pub fn encode(execution: Execution) -> Result<Vec<u8>, ReporterError> {
        let json = serde_json::to_vec(&execution)?;
        let mut record = (json.len() as u32).to_be_bytes().to_vec();
        record.extend_from_slice(&json);
        Ok(record)
    }
}

/// Where the collector listens: a loopback TCP address, or a UNIX domain
/// socket path.
///
/// The text form is also the JSON form. TCP is what `SocketAddr` prints
/// (`127.0.0.1:8080`, `[::1]:8080`). UNIX is `unix:` followed by an
/// absolute path (`unix:/run/user/1000/bear-1a2b3c4d/sock`).
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub enum Address {
    Tcp(SocketAddr),
    #[cfg(unix)]
    Unix(PathBuf),
}

const UNIX_PREFIX: &str = "unix:";

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.strip_prefix(UNIX_PREFIX) {
            #[cfg(unix)]
            Some(path) if path.starts_with('/') => Ok(Address::Unix(PathBuf::from(path))),
            #[cfg(unix)]
            Some(_) => Err(AddressError::RelativePath(text.to_string())),
            #[cfg(not(unix))]
            Some(_) => Err(AddressError::UnixUnsupported(text.to_string())),
            None => text.parse().map(Address::Tcp).map_err(|_| AddressError::Invalid(text.to_string())),
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Address::Tcp(address) => write!(f, "{address}"),
            #[cfg(unix)]
            Address::Unix(path) => write!(f, "{UNIX_PREFIX}{}", path.display()),
        }
    }
}

impl TryFrom<String> for Address {
    type Error = AddressError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Address> for String {
    fn from(address: Address) -> Self {
        address.to_string()
    }
}

/// The text is not a valid report address.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum AddressError {
    #[error("the UNIX socket path is not absolute: {0}")]
    RelativePath(String),
    #[error("UNIX sockets are not supported on this platform: {0}")]
    UnixUnsupported(String),
    #[error("not a socket address: {0}")]
    Invalid(String),
}

/// An execution reporter that sends to a collector at an [`Address`].
pub struct SocketReporter {
    destination: Address,
}

impl SocketReporter {
    pub fn new(destination: Address) -> Self {
        Self { destination }
    }
}

impl Reporter for SocketReporter {
    /// Sends an execution to the remote collector.
    ///
    /// The execution's environment is trimmed to the variables relevant for
    /// compilation database generation before serialization.
    /// The connection is opened and closed for each execution. The record
    /// is encoded before the connect and sent in one write, so the collector,
    /// which waits a short time for a silent peer, gets it at once.
    fn report(&self, execution: Execution) -> Result<(), ReporterError> {
        let execution = execution.trim();
        log::debug!("Execution report: {execution:?}");
        let record = ExecutionWireSerializer::encode(execution)?;

        match &self.destination {
            Address::Tcp(address) => TcpStream::connect(address)?.write_all(&record)?,
            #[cfg(unix)]
            Address::Unix(path) => connect_unix(path)?.write_all(&record)?,
        }

        Ok(())
    }
}

/// Connects to a UNIX socket, retrying while the collector's queue is full.
///
/// macOS and the BSDs refuse a connect with `ECONNREFUSED` when the listen
/// queue is full, where TCP would have resent the SYN. A short backoff
/// (1, 2, 4 ... 128 ms) rides out a burst. A stale socket file gives the same
/// error, so the wait is bounded at 255 ms. Any other error returns at once.
#[cfg(unix)]
pub fn connect_unix(path: &std::path::Path) -> std::io::Result<std::os::unix::net::UnixStream> {
    use std::io::ErrorKind;
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    let mut delay = Duration::from_millis(1);
    loop {
        match UnixStream::connect(path) {
            Err(err) if err.kind() == ErrorKind::ConnectionRefused && delay <= Duration::from_millis(128) => {
                std::thread::sleep(delay);
                delay *= 2;
            }
            result => return result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_executions() -> Vec<Execution> {
        vec![
            Execution::from_strings("/usr/bin/ls", vec!["ls", "-l"], "/tmp", HashMap::new()),
            Execution::from_strings(
                "/usr/bin/cc",
                vec!["cc", "-c", "./file_a.c", "-o", "./file_a.o"],
                "/home/user",
                HashMap::from([("PATH", "/usr/bin:/bin"), ("CC", "gcc")]),
            ),
            Execution::from_strings(
                "/usr/bin/ld",
                vec!["ld", "-o", "./file_a", "./file_a.o"],
                "/opt/project",
                HashMap::from([("PATH", "/usr/bin:/bin"), ("LD_PRELOAD", "/usr/lib:/lib")]),
            ),
        ]
    }

    fn compiler_call() -> Execution {
        Execution::from_strings("/usr/bin/cc", vec!["cc", "-c", "file.c"], "/tmp", HashMap::new())
    }

    mod serializer {
        use super::*;
        use std::io::Cursor;

        // The executions are written to a buffer and read back.
        #[test]
        fn read_write_works() {
            let expected = sample_executions();
            let mut written = Vec::new();
            for execution in expected.iter() {
                written.extend(ExecutionWireSerializer::encode(execution.clone()).unwrap());
            }

            let mut reader = Cursor::new(written);
            for execution in expected.iter() {
                let sut = ExecutionWireSerializer::read(&mut reader);

                assert_eq!(sut.unwrap(), execution.clone());
            }
        }

        #[cfg(unix)]
        mod sigpipe {
            use super::*;
            use std::io::ErrorKind;
            use std::os::unix::net::{UnixListener, UnixStream};
            use std::sync::atomic::{AtomicBool, Ordering};

            static SIGPIPE_SEEN: AtomicBool = AtomicBool::new(false);

            extern "C" fn on_sigpipe(_: libc::c_int) {
                SIGPIPE_SEEN.store(true, Ordering::SeqCst);
            }

            // Guard, not a "fails first" test: with the MSRV at 1.90 std's write
            // is `send` with MSG_NOSIGNAL, so this passes from the start. It
            // catches a later switch to `write_vectored` or a raw `write`.
            //
            // Requirements: interception-report-channel
            #[test]
            fn write_to_a_vanished_collector_raises_no_sigpipe() {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("s");
                let listener = UnixListener::bind(&path).unwrap();
                let mut stream = UnixStream::connect(&path).unwrap();
                drop(listener);
                let execution = Execution::from_strings("/usr/bin/cc", vec!["cc"], "/tmp", HashMap::new());
                let record = ExecutionWireSerializer::encode(execution).unwrap();
                // SAFETY: a zeroed sigaction is a valid starting value; the
                // handler only stores to an atomic. The old action is restored.
                let (sut, previous) = unsafe {
                    let mut action: libc::sigaction = std::mem::zeroed();
                    action.sa_sigaction = on_sigpipe as extern "C" fn(libc::c_int) as usize;
                    let mut previous: libc::sigaction = std::mem::zeroed();
                    libc::sigaction(libc::SIGPIPE, &action, &mut previous);
                    SIGPIPE_SEEN.store(false, Ordering::SeqCst);
                    let result = stream.write_all(&record);
                    (result, previous)
                };
                let seen = SIGPIPE_SEEN.load(Ordering::SeqCst);
                // SAFETY: restores the action saved above.
                unsafe { libc::sigaction(libc::SIGPIPE, &previous, std::ptr::null_mut()) };

                assert!(!seen, "SIGPIPE was raised");
                match sut {
                    Err(err) => assert_eq!(err.kind(), ErrorKind::BrokenPipe),
                    other => panic!("expected BrokenPipe, got {other:?}"),
                }
            }
        }
    }

    mod address {
        use super::*;

        #[test]
        fn text_round_trips() {
            let cases = [
                "127.0.0.1:8080",
                "[::1]:8080",
                #[cfg(unix)]
                "unix:/run/user/1000/bear-1a2b3c4d/sock",
            ];

            for text in cases {
                let sut: Address = text.parse().unwrap();

                assert_eq!(sut.to_string(), text, "case: {text}");
                let json = serde_json::to_string(&sut).unwrap();
                assert_eq!(json, format!("\"{text}\""), "case: {text}");
                assert_eq!(serde_json::from_str::<Address>(&json).unwrap(), sut, "case: {text}");
            }
        }

        #[test]
        fn rejects_bad_text() {
            let cases = ["", "unix:", "unix:relative/sock", "localhost", "/tmp/sock", "tcp:127.0.0.1:1"];

            for text in cases {
                let sut = text.parse::<Address>();

                assert!(sut.is_err(), "case: {text:?}");
            }
        }
    }

    mod reporter {
        use super::*;

        #[test]
        fn writes_a_readable_record_over_tcp() {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let sender = std::thread::spawn(move || {
                SocketReporter::new(Address::Tcp(address)).report(compiler_call())
            });

            let (mut connection, _) = listener.accept().unwrap();
            let sut = ExecutionWireSerializer::read(&mut connection).unwrap();

            sender.join().unwrap().expect("report should succeed");
            assert_eq!(sut, compiler_call());
        }

        #[cfg(unix)]
        #[test]
        fn writes_a_readable_record_over_a_unix_socket() {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("s");
            let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
            let sender =
                std::thread::spawn(move || SocketReporter::new(Address::Unix(path)).report(compiler_call()));

            let (mut connection, _) = listener.accept().unwrap();
            let sut = ExecutionWireSerializer::read(&mut connection).unwrap();

            sender.join().unwrap().expect("report should succeed");
            assert_eq!(sut, compiler_call());
        }
    }

    #[cfg(unix)]
    mod connect {
        use super::*;
        use std::io::ErrorKind;
        use std::os::unix::net::UnixListener;
        use std::time::{Duration, Instant};

        fn stale_socket(directory: &std::path::Path) -> PathBuf {
            let path = directory.join("s");
            drop(UnixListener::bind(&path).unwrap());
            path
        }

        // Requirements: interception-report-channel
        #[test]
        fn retries_while_the_connection_is_refused() {
            let directory = tempfile::tempdir().unwrap();
            let path = stale_socket(directory.path());
            let replacement = directory.path().join("r");
            let listener = UnixListener::bind(&replacement).unwrap();
            let target = path.clone();
            let swapper = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(20));
                std::fs::rename(&replacement, &target).unwrap();
            });

            let sut = connect_unix(&path);

            swapper.join().unwrap();
            assert!(sut.is_ok(), "{sut:?}");
            drop(listener);
        }

        #[test]
        fn gives_up_on_a_stale_socket() {
            let directory = tempfile::tempdir().unwrap();
            let path = stale_socket(directory.path());

            let sut = connect_unix(&path);

            assert_eq!(sut.unwrap_err().kind(), ErrorKind::ConnectionRefused);
        }

        #[test]
        fn fails_fast_on_a_missing_path() {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("missing");
            let started = Instant::now();

            let sut = connect_unix(&path);

            assert_eq!(sut.unwrap_err().kind(), ErrorKind::NotFound);
            assert!(started.elapsed() < Duration::from_millis(100));
        }
    }
}
