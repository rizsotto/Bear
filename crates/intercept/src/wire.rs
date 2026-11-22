// SPDX-License-Identifier: GPL-3.0-or-later

//! The wire format of an execution report, the address it goes to and the reporter that sends it.

use crate::Execution;
use crate::reporter::{Reporter, ReporterError};
use std::fmt;
use std::io::{self, Write};
use std::net::{SocketAddr, TcpStream};
#[cfg(unix)]
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;

/// The wire format of an execution report. Pure byte work, no I/O.
///
/// A record is a 4-byte big-endian length followed by that many bytes of the
/// execution's JSON. A record of length zero ends a stream: no reporter sends
/// one. The reporter (`SocketReporter`, below) and the collector (in
/// `intercept-supervisor`) do the I/O around these functions.
pub struct ExecutionWireSerializer;

impl ExecutionWireSerializer {
    /// The size of a record header.
    pub const HEADER_SIZE: usize = 4;
    /// The end-of-stream record.
    pub const END_OF_STREAM: [u8; Self::HEADER_SIZE] = [0; Self::HEADER_SIZE];
    /// A record that announces more bytes than this is not from a reporter.
    const MAX_BODY: u64 = 16 * 1024 * 1024;

    /// Encodes one execution as a whole record, header and body.
    pub fn encode(execution: &Execution) -> Result<Vec<u8>, ReporterError> {
        let json = serde_json::to_vec(execution)?;
        if json.len() as u64 > Self::MAX_BODY {
            let message = format!("a record of {} bytes is too long", json.len());
            return Err(io::Error::new(io::ErrorKind::InvalidData, message).into());
        }
        let mut record = (json.len() as u32).to_be_bytes().to_vec();
        record.extend_from_slice(&json);
        Ok(record)
    }

    /// Decodes a record header. A length over the limit is an error, so the
    /// reader never allocates for it.
    pub fn decode_header(header: [u8; Self::HEADER_SIZE]) -> Result<RecordHeader, ReporterError> {
        match u64::from(u32::from_be_bytes(header)) {
            0 => Ok(RecordHeader::End),
            length if length > Self::MAX_BODY => {
                let message = format!("a record of {length} bytes is too long");
                Err(io::Error::new(io::ErrorKind::InvalidData, message).into())
            }
            length => Ok(RecordHeader::Body(length)),
        }
    }

    /// Decodes a record body.
    pub fn decode_body(body: &[u8]) -> Result<Execution, ReporterError> {
        Ok(serde_json::from_slice(body)?)
    }
}

/// What a record header announces.
#[derive(Debug, PartialEq, Eq)]
pub enum RecordHeader {
    /// The end-of-stream record.
    End,
    /// A body of this many bytes follows.
    Body(u64),
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
    /// The connection is opened and closed for each execution. The record is
    /// encoded before the connect, so a bad record opens no connection, and
    /// sent in one `write_all`, so the collector, which waits a short time
    /// for a silent peer, gets it at once. Never a vectored write:
    /// `write_vectored` on a UNIX socket does not suppress `SIGPIPE`.
    fn report(&self, execution: Execution) -> Result<(), ReporterError> {
        let execution = execution.trim();
        log::debug!("Execution report: {execution:?}");
        let record = ExecutionWireSerializer::encode(&execution)?;
        retry_on_reset(|| match &self.destination {
            Address::Tcp(address) => TcpStream::connect(address)?.write_all(&record),
            #[cfg(unix)]
            Address::Unix(path) => connect_unix(path)?.write_all(&record),
        })?;
        Ok(())
    }
}

/// Runs a send again while its connection is reset.
///
/// FreeBSD resets a TCP connection whose handshake completed when the
/// collector's accept queue is full, so the connect or the write fails with
/// `ECONNRESET`. A short backoff (1, 2, 4, 8 ms) rides out a burst. Any other
/// error returns at once.
pub fn retry_on_reset(mut send: impl FnMut() -> io::Result<()>) -> io::Result<()> {
    use std::time::Duration;

    let mut delay = Duration::from_millis(1);
    loop {
        match send() {
            // No duplicate: the collector keeps only a record it read whole, which a reset send is not.
            Err(err) if err.kind() == io::ErrorKind::ConnectionReset && delay <= Duration::from_millis(8) => {
                std::thread::sleep(delay);
                delay *= 2;
            }
            result => return result,
        }
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

        fn decode(record: &[u8]) -> Execution {
            let (header, body) = record.split_at(ExecutionWireSerializer::HEADER_SIZE);
            let length = ExecutionWireSerializer::decode_header(header.try_into().unwrap()).unwrap();
            assert_eq!(length, RecordHeader::Body(body.len() as u64));
            ExecutionWireSerializer::decode_body(body).unwrap()
        }

        #[test]
        fn encode_and_decode_round_trip() {
            for execution in sample_executions() {
                let record = ExecutionWireSerializer::encode(&execution).unwrap();

                let sut = decode(&record);

                assert_eq!(sut, execution);
            }
        }

        #[test]
        fn decode_header_reads_the_length() {
            let max = ExecutionWireSerializer::MAX_BODY as u32;
            let cases = [
                ("end of stream", ExecutionWireSerializer::END_OF_STREAM, Some(RecordHeader::End)),
                ("a body", 10u32.to_be_bytes(), Some(RecordHeader::Body(10))),
                ("at the limit", max.to_be_bytes(), Some(RecordHeader::Body(max as u64))),
                ("over the limit", (max + 1).to_be_bytes(), None),
                ("the largest length", u32::MAX.to_be_bytes(), None),
            ];

            for (case, header, expected) in cases {
                let sut = ExecutionWireSerializer::decode_header(header);

                assert_eq!(sut.ok(), expected, "case: {case}");
            }
        }

        #[test]
        fn decode_body_rejects_garbage() {
            let sut = ExecutionWireSerializer::decode_body(b"xxxxx");

            assert!(sut.is_err(), "{sut:?}");
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
        use std::io::Read;

        /// The record that the reporter sent before it closed the connection.
        fn received(connection: &mut impl Read) -> Execution {
            let mut record = Vec::new();
            connection.read_to_end(&mut record).unwrap();
            ExecutionWireSerializer::decode_body(&record[ExecutionWireSerializer::HEADER_SIZE..]).unwrap()
        }

        #[test]
        fn writes_a_readable_record_over_tcp() {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let sender = std::thread::spawn(move || {
                SocketReporter::new(Address::Tcp(address)).report(compiler_call())
            });

            let (mut connection, _) = listener.accept().unwrap();
            let sut = received(&mut connection);

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
            let sut = received(&mut connection);

            sender.join().unwrap().expect("report should succeed");
            assert_eq!(sut, compiler_call());
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
                // SAFETY: a zeroed sigaction is a valid starting value; the
                // handler only stores to an atomic. The old action is restored.
                let (sut, previous) = unsafe {
                    let mut action: libc::sigaction = std::mem::zeroed();
                    action.sa_sigaction = on_sigpipe as extern "C" fn(libc::c_int) as usize;
                    let mut previous: libc::sigaction = std::mem::zeroed();
                    libc::sigaction(libc::SIGPIPE, &action, &mut previous);
                    SIGPIPE_SEEN.store(false, Ordering::SeqCst);
                    let record = ExecutionWireSerializer::encode(&execution).unwrap();
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

    mod retry_on_reset {
        use super::*;
        use std::cell::Cell;
        use std::io::ErrorKind::{ConnectionRefused, ConnectionReset};
        use std::rc::Rc;

        /// A send that fails with `error` the first `failures` times, then succeeds.
        fn failing(
            failures: usize,
            error: io::ErrorKind,
        ) -> (impl FnMut() -> io::Result<()>, Rc<Cell<usize>>) {
            let calls = Rc::new(Cell::new(0));
            let counter = Rc::clone(&calls);
            let send = move || {
                counter.set(counter.get() + 1);
                if counter.get() <= failures { Err(io::Error::from(error)) } else { Ok(()) }
            };
            (send, calls)
        }

        // Requirements: interception-report-channel
        #[test]
        fn retries_a_reset_a_few_times() {
            let cases = [
                ("no failure", 0, ConnectionReset, Ok(()), 1),
                ("resets then succeeds", 4, ConnectionReset, Ok(()), 5),
                ("resets for good", 9, ConnectionReset, Err(ConnectionReset), 5),
                ("other error", 9, ConnectionRefused, Err(ConnectionRefused), 1),
            ];

            for (case, failures, error, expected, expected_calls) in cases {
                let (send, calls) = failing(failures, error);

                let sut = retry_on_reset(send);

                assert_eq!(sut.map_err(|err| err.kind()), expected, "case: {case}");
                assert_eq!(calls.get(), expected_calls, "case: {case}");
            }
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
