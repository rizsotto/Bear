// SPDX-License-Identifier: GPL-3.0-or-later

//! The module contains the driver-side collector.
//!
//! The collector is the receiver half of the interception wire protocol. It
//! accepts one connection per reported execution and reads each record in the
//! [`intercept::wire::ExecutionWireSerializer`] wire format that the reporter
//! (`intercept::wire::SocketReporter`) writes.
//!
//! On Unix the collector listens on a UNIX domain socket in a private
//! directory, or on a loopback TCP port when no socket can be made and the
//! caller allows it. On Windows it listens on a loopback TCP port. Both are
//! served by one blocking loop: accept a connection, read one record, close.

#[cfg(unix)]
mod unix;

use intercept::Execution;
use intercept::reporter::ReporterError;
use intercept::wire::{Address, ExecutionWireSerializer, RecordHeader};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// How long a read waits for a silent peer. It must stay below the 511 ms that
/// a reporter retries a refused UNIX connect, so a stuck peer cannot outlast
/// the reporters waiting behind it.
#[cfg(unix)]
const UNIX_READ_TIMEOUT: Duration = Duration::from_millis(200);
/// When the accept queue is full, Linux drops the client's last handshake
/// packet, and the record comes on a resend 200 ms or more later. A shorter
/// wait lost reports in a burst. TCP has no reporter retry budget to stay under.
const TCP_READ_TIMEOUT: Duration = Duration::from_secs(1);
/// The pause after an `accept` error, so a lasting one does not spin.
const ACCEPT_PAUSE: Duration = Duration::from_millis(10);
/// After `shutdown`, how long `accept` may keep failing before the loop ends.
const ACCEPT_GIVE_UP: Duration = Duration::from_secs(1);

/// Receives the executions that the interception agents report over a socket.
pub struct Collector {
    listener: Listener,
    address: Address,
    stop_requested: AtomicBool,
}

impl Collector {
    /// Creates a new execution collector and returns it with its address.
    ///
    /// On Unix the collector tries each base directory in order. In each it
    /// makes a private `bear-<random>` directory and binds a socket there.
    /// A base that is relative, not UTF-8, too long or not writable is
    /// skipped. When no base works, the collector falls back to a loopback
    /// TCP port if `fallback` is set, and fails otherwise. Windows ignores
    /// all three arguments and uses a loopback TCP port.
    pub fn new(bases: &[PathBuf], access: SocketAccess, fallback: bool) -> io::Result<(Self, Address)> {
        #[cfg(unix)]
        let listener = unix::bind(bases, access, fallback)?;
        #[cfg(not(unix))]
        let listener = {
            let _ = (bases, access, fallback);
            Listener::Tcp(bind_loopback()?)
        };
        let address = listener.address()?;
        let collector = Self { listener, address: address.clone(), stop_requested: AtomicBool::new(false) };
        Ok((collector, address))
    }

    /// Yields the reported executions until `shutdown` is called and every
    /// report that was complete before that call has been read.
    ///
    /// # Graceful shutdown
    ///
    /// Reporters write the whole record before the intercepted process
    /// execs, so when `shutdown` is called every report is in the accept
    /// queue with its bytes, even when the reporter has closed (issue #704).
    /// `shutdown` then connects and sends a zero length, and the loop ends
    /// when it reads that marker. On the UNIX socket the queue is in connect
    /// order, so every earlier report has been read by then. On TCP that
    /// holds only while the accept queue has not overflowed: Linux then drops
    /// the client's last handshake packet, and that connection is queued on a
    /// resend, possibly after the marker. The loop never leaves blocking mode,
    /// so no accepted socket inherits `O_NONBLOCK` from the listener, as it
    /// would on macOS and the BSDs.
    pub fn executions(&self) -> impl Iterator<Item = Result<Execution, ReporterError>> + '_ {
        let mut failing_since: Option<Instant> = None;
        std::iter::from_fn(move || {
            loop {
                let connection = match self.listener.accept() {
                    Ok(connection) => connection,
                    Err(err) => {
                        // A lasting error (out of descriptors) would keep the marker out for good.
                        let since = *failing_since.get_or_insert_with(Instant::now);
                        if self.stop_requested.load(Ordering::Acquire) && since.elapsed() > ACCEPT_GIVE_UP {
                            return None;
                        }
                        std::thread::sleep(ACCEPT_PAUSE);
                        return Some(Err(ReporterError::Network(err)));
                    }
                };
                failing_since = None;
                let started = Instant::now();
                match read_record(connection) {
                    Ok(Some(execution)) => return Some(Ok(execution)),
                    Ok(None) if self.stop_requested.load(Ordering::Acquire) => return None,
                    Ok(None) => log::debug!("skipping an empty record"),
                    Err(err) => log::warn!("lost a report after {} ms: {err}", started.elapsed().as_millis()),
                }
            }
        })
    }

    /// Stops the collector. The `executions` iterator drains and ends.
    ///
    /// Fails when the collector cannot connect to its own address, for
    /// example when the build deleted the socket file. The iterator does not
    /// end then.
    pub fn shutdown(&self) -> Result<(), ReporterError> {
        self.stop_requested.store(true, Ordering::Release);
        let wake = || -> io::Result<()> {
            let mut stream: Box<dyn Write> = match &self.address {
                Address::Tcp(address) => Box::new(TcpStream::connect(address)?),
                #[cfg(unix)]
                Address::Unix(path) => Box::new(intercept::wire::connect_unix(path)?),
            };
            stream.write_all(&ExecutionWireSerializer::END_OF_STREAM)
        };
        wake().map_err(|err| {
            io::Error::new(err.kind(), format!("cannot reach the report socket {}: {err}", self.address))
                .into()
        })
    }
}

/// Who may connect to the report socket.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SocketAccess {
    /// The user who runs Bear.
    Owner,
    /// Every user.
    All,
}

enum Listener {
    Tcp(TcpListener),
    /// The directory is removed when the listener is dropped.
    #[cfg(unix)]
    Unix(UnixListener, unix::SocketDir),
}

impl Listener {
    fn address(&self) -> io::Result<Address> {
        match self {
            Listener::Tcp(listener) => Ok(Address::Tcp(listener.local_addr()?)),
            #[cfg(unix)]
            Listener::Unix(_, dir) => Ok(Address::Unix(dir.socket.clone())),
        }
    }

    /// Accepts a blocking connection whose reads give up on a silent peer.
    fn accept(&self) -> io::Result<Box<dyn Read>> {
        match self {
            Listener::Tcp(listener) => {
                let (stream, _) = listener.accept()?;
                allow_closed_peer(stream.set_read_timeout(Some(TCP_READ_TIMEOUT)))?;
                Ok(Box::new(stream))
            }
            #[cfg(unix)]
            Listener::Unix(listener, _) => {
                let (stream, _) = listener.accept()?;
                allow_closed_peer(stream.set_read_timeout(Some(UNIX_READ_TIMEOUT)))?;
                Ok(Box::new(stream))
            }
        }
    }
}

/// Reads one record from a connection. `None` is the end-of-stream record.
fn read_record(mut connection: impl Read) -> Result<Option<Execution>, ReporterError> {
    let header = read_part(&mut connection, ExecutionWireSerializer::HEADER_SIZE as u64, "header")?;
    let header = header.try_into().expect("read_part returns exactly the bytes asked for");
    let length = match ExecutionWireSerializer::decode_header(header)? {
        RecordHeader::End => return Ok(None),
        RecordHeader::Body(length) => length,
    };
    let body = read_part(&mut connection, length, "body")?;
    Ok(Some(ExecutionWireSerializer::decode_body(&body)?))
}

/// Reads exactly `length` bytes. The error says how many bytes came.
///
/// The buffer grows with the bytes that arrive, so a bad length does not
/// allocate up front. A connection that closes early is an error.
fn read_part(connection: impl Read, length: u64, part: &str) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let err = match connection.take(length).read_to_end(&mut bytes) {
        Ok(_) if bytes.len() as u64 == length => return Ok(bytes),
        Ok(_) => io::Error::from(io::ErrorKind::UnexpectedEof),
        Err(err) => err,
    };
    Err(io::Error::new(err.kind(), format!("{} of {length} {part} bytes received: {err}", bytes.len())))
}

/// macOS refuses `setsockopt` with EINVAL when the peer already closed. Its
/// bytes can still be read, then EOF, so the read cannot hang.
fn allow_closed_peer(result: io::Result<()>) -> io::Result<()> {
    match result {
        Err(err) if err.kind() == io::ErrorKind::InvalidInput => Ok(()),
        other => other,
    }
}

/// Binds an ephemeral loopback port: IPv4 first, IPv6 when IPv4 is unavailable.
fn bind_loopback() -> io::Result<TcpListener> {
    TcpListener::bind("127.0.0.1:0").or_else(|_| TcpListener::bind("[::1]:0"))
}

#[cfg(test)]
mod tests {
    use self::fixture::*;
    use intercept::Execution;
    use intercept::reporter::Reporter;
    use intercept::wire::SocketReporter;
    use std::collections::HashMap;
    use std::io::Write;
    use std::time::Duration;

    pub(crate) mod fixture {
        use super::super::*;
        use intercept::reporter::Reporter;
        use intercept::wire::SocketReporter;
        use std::collections::HashMap;
        use std::sync::Arc;
        use std::time::Duration;

        pub(crate) fn sample_executions() -> Vec<Execution> {
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

        pub(crate) fn numbered(index: usize) -> Execution {
            Execution::from_strings(
                "/usr/bin/cc",
                vec!["cc", &format!("file_{index}.c")],
                "/tmp",
                HashMap::new(),
            )
        }

        /// A collector and where it listens. Owns the directory that holds the socket.
        pub(crate) struct Fixture {
            pub(crate) name: &'static str,
            pub(crate) collector: Arc<Collector>,
            pub(crate) address: Address,
            pub(crate) _base: Option<tempfile::TempDir>,
        }

        impl Fixture {
            pub(crate) fn tcp() -> Self {
                let (collector, address) = Collector::new(&[], SocketAccess::Owner, true).unwrap();
                Self { name: "tcp", collector: Arc::new(collector), address, _base: None }
            }

            #[cfg(unix)]
            pub(crate) fn unix() -> Self {
                let base = short_base();
                let (collector, address) = open_in(base.path(), SocketAccess::Owner);
                Self { name: "unix", collector: Arc::new(collector), address, _base: Some(base) }
            }

            pub(crate) fn all() -> Vec<Self> {
                #[cfg(unix)]
                return vec![Self::tcp(), Self::unix()];
                #[cfg(not(unix))]
                vec![Self::tcp()]
            }

            pub(crate) fn report(&self, execution: &Execution) {
                SocketReporter::new(self.address.clone())
                    .report(execution.clone())
                    .expect("report should succeed");
            }

            /// Collects every execution until the iterator ends.
            pub(crate) fn collect_in_thread(&self) -> std::thread::JoinHandle<Vec<Execution>> {
                collect_in_thread(&self.collector)
            }
        }

        /// Collects every execution until the iterator ends. An error from the collector fails the test.
        pub(crate) fn collect_in_thread(
            collector: &Arc<Collector>,
        ) -> std::thread::JoinHandle<Vec<Execution>> {
            let results = collect_results_in_thread(collector);
            std::thread::spawn(move || {
                let results = results.join().unwrap();
                results.into_iter().map(|item| item.expect("the collector reported an error")).collect()
            })
        }

        /// Collects every item until the iterator ends, errors included.
        pub(crate) fn collect_results_in_thread(
            collector: &Arc<Collector>,
        ) -> std::thread::JoinHandle<Vec<Result<Execution, ReporterError>>> {
            let collector = Arc::clone(collector);
            std::thread::spawn(move || collector.executions().collect())
        }

        /// Joins a worker, failing instead of hanging when it does not end.
        pub(crate) fn join_within<T: Send + 'static>(
            handle: std::thread::JoinHandle<T>,
            limit: Duration,
        ) -> T {
            let (done_tx, done_rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = done_tx.send(handle.join().unwrap());
            });
            done_rx.recv_timeout(limit).expect("timed out waiting for the collector to end")
        }

        pub(crate) fn sorted_commands(executions: &[Execution]) -> Vec<Vec<String>> {
            let mut commands: Vec<_> =
                executions.iter().map(|execution| execution.arguments.clone()).collect();
            commands.sort();
            commands
        }

        pub(crate) fn connect(address: &Address) -> Box<dyn std::io::Write + Send> {
            match address {
                Address::Tcp(address) => Box::new(std::net::TcpStream::connect(address).unwrap()),
                #[cfg(unix)]
                Address::Unix(path) => Box::new(std::os::unix::net::UnixStream::connect(path).unwrap()),
            }
        }

        pub(crate) fn pause() {
            std::thread::sleep(Duration::from_millis(5));
        }

        #[cfg(unix)]
        pub(crate) use unix::*;

        #[cfg(unix)]
        mod unix {
            use super::*;
            use std::os::unix::fs::PermissionsExt;
            use std::path::Path;

            pub(crate) fn open_in(base: &Path, access: SocketAccess) -> (Collector, Address) {
                Collector::new(&[base.to_path_buf()], access, false).unwrap()
            }

            pub(crate) fn socket_path(address: &Address) -> PathBuf {
                match address {
                    Address::Unix(path) => path.clone(),
                    other => panic!("expected a UNIX address, got {other}"),
                }
            }

            pub(crate) fn mode(path: &Path) -> u32 {
                std::fs::metadata(path).unwrap().permissions().mode() & 0o7777
            }

            pub(crate) fn short_base() -> tempfile::TempDir {
                let base = tempfile::Builder::new().prefix("bc").tempdir_in("/tmp").unwrap();
                set_mode(base.path(), 0o755);
                base
            }

            /// The socket directory is made under the canonical base (`/tmp` is `/private/tmp` on macOS).
            pub(crate) fn real(path: &Path) -> PathBuf {
                path.canonicalize().unwrap()
            }

            pub(crate) fn set_mode(path: &Path, mode: u32) {
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
            }

            pub(crate) fn entries(base: &Path) -> usize {
                std::fs::read_dir(base).unwrap().count()
            }
        }
    }

    #[test]
    fn reporter_and_collector_work() {
        for fixture in Fixture::all() {
            let expected = sample_executions();
            let worker = fixture.collect_in_thread();

            for execution in expected.iter() {
                fixture.report(execution);
            }
            fixture.collector.shutdown().unwrap();
            let sut = join_within(worker, Duration::from_secs(5));

            assert_eq!(sorted_commands(&sut), sorted_commands(&expected), "case: {}", fixture.name);
        }
    }

    // Regression test for issue #704: reports already queued in the accept
    // backlog must not be dropped when shutdown is requested.
    //
    // All reports are sent *before* `shutdown` is called and *before* the
    // collector starts accepting, so they sit in the listener's queue ahead
    // of the shutdown marker, as the last compiler's report does at the end
    // of a real build.
    //
    // Requirements: interception-preload-mechanism, interception-report-channel
    #[test]
    fn drains_backlog_on_shutdown() {
        for fixture in Fixture::all() {
            let expected = sample_executions();
            for execution in expected.iter() {
                fixture.report(execution);
            }
            fixture.collector.shutdown().unwrap();

            let worker = fixture.collect_in_thread();
            let sut = join_within(worker, Duration::from_secs(5));

            assert_eq!(sorted_commands(&sut), sorted_commands(&expected), "case: {}", fixture.name);
        }
    }

    // The marker that `shutdown` sends wakes an idle collector.
    #[test]
    fn shutdown_stops_iterator() {
        for fixture in Fixture::all() {
            let worker = fixture.collect_in_thread();

            fixture.collector.shutdown().unwrap();

            let sut = join_within(worker, Duration::from_secs(5));
            assert!(sut.is_empty(), "case: {}", fixture.name);
        }
    }

    // Each transport is checked on its own and every one runs, so a CI log
    // from a platform we cannot run says which transport failed and how.
    //
    // Requirements: interception-mechanism-selection, interception-report-channel
    #[test]
    fn loses_no_report_in_a_burst() {
        const THREADS: usize = 16;
        const PER_THREAD: usize = 500;
        const REPORTS: usize = THREADS * PER_THREAD;
        let mut failures = Vec::new();
        for fixture in Fixture::all() {
            // A known limitation of the loopback fallback: FreeBSD loses reports in a burst.
            if cfg!(target_os = "freebsd") && fixture.name == "tcp" {
                eprintln!("tcp: skipped on FreeBSD, see interception-report-channel");
                continue;
            }
            eprintln!("{}: sending {REPORTS} reports", fixture.name);
            let worker = collect_results_in_thread(&fixture.collector);

            let senders: Vec<_> = (0..THREADS)
                .map(|thread| {
                    let address = fixture.address.clone();
                    std::thread::spawn(move || {
                        let mut failed = Failures::default();
                        for index in 0..PER_THREAD {
                            let reporter = SocketReporter::new(address.clone());
                            if let Err(err) = reporter.report(numbered(thread * PER_THREAD + index)) {
                                failed.add(err);
                            }
                        }
                        failed
                    })
                })
                .collect();
            let mut sent = Failures::default();
            for sender in senders {
                sent.merge(sender.join().unwrap());
            }
            fixture.collector.shutdown().unwrap();
            let sut = join_within(worker, Duration::from_secs(120));

            let mut collector = Failures::default();
            let mut seen = Vec::new();
            for result in sut {
                match result {
                    Ok(execution) => seen.push(execution.arguments[1].clone()),
                    Err(err) => collector.add(err),
                }
            }
            let collected = seen.len();
            seen.sort();
            seen.dedup();
            let mut problems = Vec::new();
            if let Some(error) = sent.first {
                problems.push(format!("{} of {REPORTS} reports failed, first error: {error}", sent.count));
            }
            if let Some(error) = collector.first {
                problems
                    .push(format!("the collector reported {} errors, first error: {error}", collector.count));
            }
            if collected != REPORTS || seen.len() != REPORTS {
                problems.push(format!("collected {collected} of {REPORTS} reports, {} distinct", seen.len()));
            }
            if !problems.is_empty() {
                failures.push(format!("{}: {}", fixture.name, problems.join("; ")));
            }
        }

        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    /// How many calls failed, and the text of the first failure.
    #[derive(Default)]
    struct Failures {
        count: usize,
        first: Option<String>,
    }

    impl Failures {
        fn add(&mut self, err: intercept::reporter::ReporterError) {
            self.count += 1;
            self.first.get_or_insert_with(|| err.to_string());
        }

        fn merge(&mut self, other: Failures) {
            self.count += other.count;
            self.first = self.first.take().or(other.first);
        }
    }

    // A record larger than the socket buffer (8 KiB for a UNIX socket on macOS
    // and the BSDs, about 208 KiB on Linux) takes several sends and reads.
    //
    // Requirements: interception-report-channel
    #[test]
    fn reads_a_record_larger_than_the_socket_buffer() {
        for fixture in Fixture::all() {
            let long = "x".repeat(1024 * 1024);
            let expected = Execution::from_strings("/usr/bin/cc", vec!["cc", &long], "/tmp", HashMap::new());
            let worker = fixture.collect_in_thread();

            fixture.report(&expected);
            fixture.collector.shutdown().unwrap();
            let sut = join_within(worker, Duration::from_secs(5));

            assert_eq!(sut, vec![expected], "case: {}", fixture.name);
        }
    }

    // Requirements: interception-report-channel
    #[test]
    fn ends_with_a_stuck_connection_open() {
        for fixture in Fixture::all() {
            let expected = sample_executions();
            let _stuck = connect(&fixture.address);
            for execution in expected.iter() {
                fixture.report(execution);
            }
            fixture.collector.shutdown().unwrap();

            let worker = fixture.collect_in_thread();
            let sut = join_within(worker, Duration::from_secs(10));

            assert_eq!(sorted_commands(&sut), sorted_commands(&expected), "case: {}", fixture.name);
        }
    }

    // Requirements: interception-report-channel
    #[test]
    fn drops_a_connection_with_a_huge_length() {
        for fixture in Fixture::all() {
            let worker = fixture.collect_in_thread();
            let mut bad = connect(&fixture.address);
            bad.write_all(&u32::MAX.to_be_bytes()).unwrap();
            bad.flush().unwrap();
            pause();

            fixture.report(&numbered(7));
            fixture.collector.shutdown().unwrap();
            let sut = join_within(worker, Duration::from_secs(5));

            assert_eq!(sut, vec![numbered(7)], "case: {}", fixture.name);
            drop(bad);
        }
    }

    mod read_record {
        use super::super::{ExecutionWireSerializer, read_record};
        use super::fixture::numbered;
        use std::io::{Cursor, Read};

        /// Hands out one byte per call, as a socket may when a record
        /// arrives in pieces.
        struct OneByteAtATime(Cursor<Vec<u8>>);

        impl Read for OneByteAtATime {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                let end = buffer.len().min(1);
                self.0.read(&mut buffer[..end])
            }
        }

        #[test]
        fn takes_a_record_that_arrives_in_pieces() {
            let record = ExecutionWireSerializer::encode(&numbered(1)).unwrap();

            let sut = read_record(OneByteAtATime(Cursor::new(record)));

            assert_eq!(sut.unwrap(), Some(numbered(1)));
        }

        #[test]
        fn returns_none_for_the_end_of_stream() {
            let sut = read_record(Cursor::new(ExecutionWireSerializer::END_OF_STREAM));

            assert_eq!(sut.unwrap(), None);
        }

        #[test]
        fn rejects_a_connection_that_closes_early() {
            let record = ExecutionWireSerializer::encode(&numbered(1)).unwrap();
            let body = record.len() - ExecutionWireSerializer::HEADER_SIZE;
            let cases = [
                ("in the header", &record[..2], "2 of 4 header bytes received".to_string()),
                (
                    "in the body",
                    &record[..record.len() - 1],
                    format!("{} of {body} body bytes received", body - 1),
                ),
            ];

            for (case, bytes, expected) in cases {
                let sut = read_record(Cursor::new(bytes.to_vec()));

                let message = sut.unwrap_err().to_string();
                assert!(message.contains(&expected), "case: {case}: {message}");
            }
        }
    }

    // Requirements: interception-report-channel
    #[test]
    fn skips_a_bad_connection_and_reads_the_next() {
        let cases: [(&str, Vec<u8>); 2] = [
            ("a zero length before shutdown", vec![0; 4]),
            ("a garbage payload", vec![0, 0, 0, 5, b'x', b'x', b'x', b'x', b'x']),
        ];

        for (case, bytes) in cases {
            for fixture in Fixture::all() {
                let mut sut = fixture.collector.executions();
                let mut bad = connect(&fixture.address);
                bad.write_all(&bytes).unwrap();
                drop(bad);
                fixture.report(&numbered(2));

                // Taken before `shutdown`, so a zero length cannot pass for the marker.
                assert_eq!(sut.next().unwrap().unwrap(), numbered(2), "case: {case}, {}", fixture.name);
                fixture.collector.shutdown().unwrap();
                assert!(sut.next().is_none(), "case: {case}, {}", fixture.name);
            }
        }
    }
}
