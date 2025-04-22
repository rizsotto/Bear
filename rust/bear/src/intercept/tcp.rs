// SPDX-License-Identifier: GPL-3.0-or-later

//! The module contains the implementation of the TCP collector and reporter.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use super::{Collector, Event, Reporter};

/// Implements convenient methods for the `Event` type.
impl Event {
    /// Read an event from a reader using TLV format.
    ///
    /// The event is serialized using JSON, and the length of the JSON
    /// is written as a 4-byte big-endian integer before the JSON.
    fn read_from(reader: &mut impl Read) -> Result<Self, anyhow::Error> {
        let mut length_bytes = [0; 4];
        reader.read_exact(&mut length_bytes)?;
        let length = u32::from_be_bytes(length_bytes) as usize;

        let mut buffer = vec![0; length];
        reader.read_exact(&mut buffer)?;
        let event = serde_json::from_slice(buffer.as_ref())?;

        Ok(event)
    }

    /// Write an event to a writer using TLV format.
    ///
    /// The event is serialized using JSON, and the length of the JSON
    /// is written as a 4-byte big-endian integer before the JSON.
    fn write_into(&self, writer: &mut impl Write) -> Result<u32, anyhow::Error> {
        let serialized_event = serde_json::to_string(&self)?;
        let bytes = serialized_event.into_bytes();
        let length = bytes.len() as u32;

        writer.write_all(&length.to_be_bytes())?;
        writer.write_all(&bytes)?;

        Ok(length)
    }
}

/// Represents a TCP event collector.
pub struct CollectorOnTcp {
    shutdown: Arc<AtomicBool>,
    listener: TcpListener,
    address: SocketAddr,
}

impl CollectorOnTcp {
    /// Creates a new TCP event collector.
    ///
    /// The collector listens to a random port on the loopback interface.
    /// The address of the collector can be obtained by the `address` method.
    pub fn new() -> Result<Self, anyhow::Error> {
        let shutdown = Arc::new(AtomicBool::new(false));
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;

        let result = CollectorOnTcp {
            shutdown,
            listener,
            address,
        };

        Ok(result)
    }

    fn send(&self, mut socket: TcpStream, destination: Sender<Event>) -> Result<(), anyhow::Error> {
        let event = Event::read_from(&mut socket)?;
        destination.send(event)?;

        Ok(())
    }
}

impl Collector for CollectorOnTcp {
    fn address(&self) -> String {
        self.address.to_string()
    }

    /// Single-threaded implementation of the collector.
    ///
    /// The collector listens to the TCP port and accepts incoming connections.
    /// When a connection is accepted, the collector reads the events from the
    /// connection and sends them to the destination channel.
    fn collect(&self, destination: Sender<Event>) -> Result<(), anyhow::Error> {
        for stream in self.listener.incoming() {
            // This has to be the first thing to do, to implement the stop method!
            if self.shutdown.load(Ordering::Relaxed) {
                break;
            }

            match stream {
                Ok(connection) => {
                    // ... (process the connection in a separate thread or task)
                    self.send(connection, destination.clone())?;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No new connection available, continue checking for shutdown
                    continue;
                }
                Err(e) => {
                    println!("Error: {}", e);
                    break;
                }
            }
        }
        Ok(())
    }

    /// Stops the collector by flipping the shutdown flag and connecting to the collector.
    ///
    /// The collector is stopped when the `collect` method sees the shutdown flag.
    /// To signal the collector to stop, we connect to the collector to unblock the
    /// `accept` call to check the shutdown flag.
    fn stop(&self) -> Result<(), anyhow::Error> {
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address)?;
        Ok(())
    }
}

/// Represents a TCP event reporter.
pub struct ReporterOnTcp {
    destination: String,
}

impl ReporterOnTcp {
    /// Creates a new TCP reporter instance.
    ///
    /// It does not open the TCP connection yet. Stores the destination
    /// address and creates a unique reporter id.
    pub fn new(destination: String) -> Result<Self, anyhow::Error> {
        let result = ReporterOnTcp { destination };
        Ok(result)
    }
}

impl Reporter for ReporterOnTcp {
    /// Sends an event to the remote collector.
    ///
    /// The event is wrapped in an envelope and sent to the remote collector.
    /// The TCP connection is opened and closed for each event.
    fn report(&self, event: Event) -> Result<(), anyhow::Error> {
        let mut socket = TcpStream::connect(self.destination.clone())?;
        event.write_into(&mut socket)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::mpsc::channel;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    // Test that the serialization and deserialization of the Envelope works.
    // We write the Envelope to a buffer and read it back to check if the
    // deserialized Envelope is the same as the original one.
    #[test]
    fn read_write_works() {
        let mut writer = Cursor::new(vec![0; 1024]);
        for event in fixtures::EVENTS.iter() {
            let result = Event::write_into(event, &mut writer);
            assert!(result.is_ok());
        }

        let mut reader = Cursor::new(writer.get_ref());
        for event in fixtures::EVENTS.iter() {
            let result = Event::read_from(&mut reader);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), event.clone());
        }
    }

    // Test that the TCP reporter and the TCP collector work together.
    // We create a TCP collector and a TCP reporter, then we send events
    // to the reporter and check if the collector receives them.
    //
    // We use a bounded channel to send the events from the reporter to the
    // collector. The collector reads the events from the channel and checks
    // if they are the same as the original events.
    #[test]
    fn tcp_reporter_and_collectors_work() {
        let collector = CollectorOnTcp::new().unwrap();
        let reporter = ReporterOnTcp::new(collector.address()).unwrap();

        // Create wrapper to share the collector across threads.
        let thread_collector = Arc::new(collector);
        let main_collector = thread_collector.clone();

        // Start the collector in a separate thread.
        let (input, output) = channel();
        let receiver_thread = thread::spawn(move || {
            thread_collector.collect(input).unwrap();
        });
        // Send events to the reporter.
        for event in fixtures::EVENTS.iter() {
            let result = reporter.report(event.clone());
            assert!(result.is_ok());
        }

        // Call the stop method to stop the collector. This will close the
        // channel and the collector will stop reading from it.
        thread::sleep(Duration::from_secs(1));
        main_collector.stop().unwrap();

        // Empty the channel and assert that we received all the events.
        let mut count = 0;
        for event in output.iter() {
            assert!(fixtures::EVENTS.contains(&event));
            count += 1;
        }
        assert_eq!(count, fixtures::EVENTS.len());
        // shutdown the receiver thread
        receiver_thread.join().unwrap();
    }

    mod fixtures {
        use super::*;
        use crate::intercept::{Execution, ProcessId};
        use crate::{map_of_strings, vec_of_strings};
        use std::collections::HashMap;
        use std::path::PathBuf;

        pub(super) static EVENTS: std::sync::LazyLock<Vec<Event>> =
            std::sync::LazyLock::new(|| {
                vec![
                    Event {
                        pid: ProcessId(3425),
                        execution: Execution {
                            executable: PathBuf::from("/usr/bin/ls"),
                            arguments: vec_of_strings!["ls", "-l"],
                            working_dir: PathBuf::from("/tmp"),
                            environment: HashMap::new(),
                        },
                    },
                    Event {
                        pid: ProcessId(3492),
                        execution: Execution {
                            executable: PathBuf::from("/usr/bin/cc"),
                            arguments: vec_of_strings![
                                "cc",
                                "-c",
                                "./file_a.c",
                                "-o",
                                "./file_a.o"
                            ],
                            working_dir: PathBuf::from("/home/user"),
                            environment: map_of_strings! {
                                "PATH" => "/usr/bin:/bin",
                                "HOME" => "/home/user",
                            },
                        },
                    },
                    Event {
                        pid: ProcessId(3522),
                        execution: Execution {
                            executable: PathBuf::from("/usr/bin/ld"),
                            arguments: vec_of_strings!["ld", "-o", "./file_a", "./file_a.o"],
                            working_dir: PathBuf::from("/opt/project"),
                            environment: map_of_strings! {
                                "PATH" => "/usr/bin:/bin",
                                "LD_LIBRARY_PATH" => "/usr/lib:/lib",
                            },
                        },
                    },
                ]
            });
    }
}
