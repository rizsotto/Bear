pub mod ipc {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct SessionLocator(String);

    // Reporter id is a unique identifier for a reporter.
    //
    // It is used to identify the process that sends the execution report.
    // Because the OS PID is not unique across a single build (PIDs are
    // recycled), we need to use a new unique identifier to identify the process.
    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    pub struct ReporterId(pub u64);

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct ProcessId(u32);

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
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
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
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

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct Envelope {
        pub rid: ReporterId,
        pub timestamp: u64,
        pub event: Event,
    }
}

mod client {
    use std::net::UdpSocket;

    use super::ipc::{Envelope, Event, ReporterId};

    use rand::Rng;

    impl ReporterId {
        pub fn new() -> Self {
            let id = rand::thread_rng().gen::<u64>();
            ReporterId(id)
        }
    }

    impl Envelope {
        pub fn new(rid: &ReporterId, event: super::ipc::Event) -> Self {
            let timestamp = chrono::Utc::now().timestamp_millis() as u64;
            Envelope { rid: rid.clone(), timestamp, event }
        }
    }

    // Represents the remote sink of supervised process events.
    //
    // Events from a process execution can be sent from many actors (mostly
    // supervisor processes). The events are collected in a common place
    // in order to reconstruct of final report of a build process.
    trait Report {
        fn report(&self, event: Event);
    }

    struct UdpReporter {
        socket: UdpSocket,
        destination: String,
        reporter_id: ReporterId,
    }

    impl Report for UdpReporter {
        fn report(&self, event: Event) {
            let envelope = Envelope::new(&self.reporter_id, event);
            let serialized_envelope = match serde_json::to_string(&envelope) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to serialize envelope: {}", e);
                    return;
                }
            };

            match self.socket.send_to(serialized_envelope.as_bytes(), &self.destination) {
                Ok(_) => (),
                Err(e) => eprintln!("Failed to send envelope: {}", e),
            };
        }
    }
}

mod server {
    use std::net::UdpSocket;

    use crossbeam::channel::Sender;
    use serde_json::Result;

    use super::ipc::Envelope;

    struct UdpServer {
        socket: UdpSocket,
        sender: Sender<Envelope>,
    }

    impl UdpServer {
        fn listen(&self) {
            let mut buf = [0; 4096];

            loop {
                match self.socket.recv_from(&mut buf) {
                    Ok((amt, _src)) => {
                        let data = &mut buf[..amt];
                        let envelope: Result<Envelope> = serde_json::from_slice(data);

                        match envelope {
                            Ok(envelope) => {
                                if let Err(e) = self.sender.send(envelope) {
                                    eprintln!("Failed to send envelope to channel: {}", e);
                                }
                            }
                            Err(e) => eprintln!("Failed to deserialize envelope: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Failed to receive data: {}", e),
                }
            }
        }
    }
}