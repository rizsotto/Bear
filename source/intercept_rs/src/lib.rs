pub mod ipc {
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::path::PathBuf;

    use chrono::Utc;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    pub struct SessionLocator(pub String);

    // Reporter id is a unique identifier for a reporter.
    //
    // It is used to identify the process that sends the execution report.
    // Because the OS PID is not unique across a single build (PIDs are
    // recycled), we need to use a new unique identifier to identify the process.
    #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
    pub struct ReporterId(pub u64);

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    pub struct ProcessId(pub u32);

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    pub struct Execution {
        pub executable: PathBuf,
        pub arguments: Vec<String>,
        pub working_dir: PathBuf,
        pub environment: HashMap<String, String>,
    }

    // Represent a relevant life cycle event of a process.
    //
    // Currently, it's only the process life cycle events (start, signal,
    // terminate), but can be extended later with performance related
    // events like monitoring the CPU usage or the memory allocation if
    // this information is available.
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    pub enum Event {
        Started {
            pid: ProcessId,
            ppid: ProcessId,
            execution: Execution,
        },
        Terminated {
            status: i64
        },
        Signaled {
            signal: i32,
        },
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    pub struct Envelope {
        pub rid: ReporterId,
        pub timestamp: u64,
        pub event: Event,
    }

    impl Envelope {
        pub fn new(rid: &ReporterId, event: Event) -> Self {
            let timestamp = Utc::now().timestamp_millis() as u64;
            Envelope { rid: rid.clone(), timestamp, event }
        }

        pub fn read_from(mut reader: impl Read) -> Result<Self, anyhow::Error> {
            let mut length_bytes = [0; 4];
            reader.read_exact(&mut length_bytes)?;
            let length = u32::from_be_bytes(length_bytes) as usize;

            let mut buffer = vec![0; length];
            reader.read_exact(&mut buffer)?;
            let envelope = serde_json::from_slice(buffer.as_ref())?;

            Ok(envelope)
        }

        pub fn write_into(&self, mut writer: impl Write) -> Result<(), anyhow::Error> {
            let serialized_envelope = serde_json::to_string(&self)?;
            let bytes = serialized_envelope.into_bytes();
            let length = bytes.len() as u32;

            writer.write_all(&length.to_be_bytes())?;
            writer.write_all(&bytes)?;

            Ok(())
        }
    }
}

mod client {
    use std::net::TcpStream;

    use rand::random;

    use super::ipc::{Envelope, Event, ReporterId};

    impl ReporterId {
        pub fn new() -> Self {
            let id = random::<u64>();
            ReporterId(id)
        }
    }

    // Represents the remote sink of supervised process events.
    //
    // Events from a process execution can be sent from many actors (mostly
    // supervisor processes). The events are collected in a common place
    // in order to reconstruct of final report of a build process.
    trait Reporter {
        fn report(&mut self, event: Event) -> Result<(), anyhow::Error>;
    }

    struct TcpReporter {
        socket: TcpStream,
        destination: String,
        reporter_id: ReporterId,
    }

    impl TcpReporter {
        pub fn new(destination: String) -> Result<Self, anyhow::Error> {
            let socket = TcpStream::connect(destination.clone())?;
            let reporter_id = ReporterId::new();
            let result = TcpReporter { socket, destination, reporter_id };
            Ok(result)
        }
    }

    impl Reporter for TcpReporter {
        fn report(&mut self, event: Event) -> Result<(), anyhow::Error> {
            let envelope = Envelope::new(&self.reporter_id, event);
            envelope.write_into(&mut self.socket)?;

            Ok(())
        }
    }
}

mod server {
    use std::net::{TcpListener, TcpStream};

    use crossbeam::channel::{Receiver, Sender};
    use crossbeam_channel::bounded;

    use super::ipc::{Envelope, SessionLocator};

    trait EventCollector {
        fn address(&self) -> Result<SessionLocator, anyhow::Error>;
        fn collect(&self, destination: Sender<Envelope>) -> Result<(), anyhow::Error>;
        fn stop(&self) -> Result<(), anyhow::Error>;
    }

    struct EventCollectorOnTcp {
        control_input: Sender<bool>,
        control_output: Receiver<bool>,
        listener: TcpListener,
    }

    impl EventCollectorOnTcp {
        pub fn new() -> Result<Self, anyhow::Error> {
            let (control_input, control_output) = bounded(0);
            let listener = TcpListener::bind("127.0.0.1:0")?;

            let result = EventCollectorOnTcp { control_input, control_output, listener };

            Ok(result)
        }

        pub fn send(
            &self,
            mut socket: TcpStream,
            destination: Sender<Envelope>,
        ) -> Result<(), anyhow::Error> {
            let envelope = Envelope::read_from(&mut socket)?;
            destination.send(envelope)?;

            Ok(())
        }
    }

    impl EventCollector for EventCollectorOnTcp {
        fn address(&self) -> Result<SessionLocator, anyhow::Error> {
            let local_addr = self.listener.local_addr()?;
            let locator = SessionLocator(local_addr.to_string());
            Ok(locator)
        }

        fn collect(&self, destination: Sender<Envelope>) -> Result<(), anyhow::Error> {
            loop {
                if let Ok(shutdown) = self.control_output.try_recv() {
                    if shutdown {
                        break;
                    }
                }

                match self.listener.accept() {
                    Ok((stream, _)) => {
                        println!("Got a connection");
                        // ... (process the connection in a separate thread or task)
                        self.send(stream, destination.clone())?;
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

            println!("Server shutting down");
            Ok(())
        }

        fn stop(&self) -> Result<(), anyhow::Error> {
            self.control_input.send(true)?;
            Ok(())
        }
    }
}