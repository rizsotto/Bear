// SPDX-License-Identifier: GPL-3.0-or-later

//! The wire format of an execution report, the address it goes to and the reporter that sends it.

use crate::Execution;
use crate::reporter::{Reporter, ReporterError};
use std::fmt;
use std::io::{Read, Write};
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
    pub fn read(reader: &mut impl Read) -> Result<Execution, ReporterError> {
        let mut length_bytes = [0; 4];
        reader.read_exact(&mut length_bytes)?;
        let length = u32::from_be_bytes(length_bytes) as usize;

        let mut buffer = vec![0; length];
        reader.read_exact(&mut buffer)?;
        let execution = serde_json::from_slice(buffer.as_ref())?;

        Ok(execution)
    }

    /// Write an execution to a writer using LV format.
    pub fn write(writer: &mut impl Write, execution: Execution) -> Result<u32, ReporterError> {
        let serialized = serde_json::to_string(&execution)?;
        let bytes = serialized.into_bytes();
        let length = bytes.len() as u32;

        writer.write_all(&length.to_be_bytes())?;
        writer.write_all(&bytes)?;

        Ok(length)
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
    /// The connection is opened and closed for each execution.
    fn report(&self, execution: Execution) -> Result<(), ReporterError> {
        let execution = execution.trim();
        log::debug!("Execution report: {execution:?}");

        match &self.destination {
            Address::Tcp(address) => {
                ExecutionWireSerializer::write(&mut TcpStream::connect(address)?, execution)?;
            }
            #[cfg(unix)]
            Address::Unix(path) => {
                ExecutionWireSerializer::write(
                    &mut std::os::unix::net::UnixStream::connect(path)?,
                    execution,
                )?;
            }
        }

        Ok(())
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
                ExecutionWireSerializer::write(&mut written, execution.clone()).unwrap();
            }

            let mut reader = Cursor::new(written);
            for execution in expected.iter() {
                let sut = ExecutionWireSerializer::read(&mut reader);

                assert_eq!(sut.unwrap(), execution.clone());
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
}
