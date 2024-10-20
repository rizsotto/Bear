// SPDX-License-Identifier: GPL-3.0-or-later

use std::net::TcpStream;

use super::{Envelope, Event, ReporterId};

/// Represents the remote sink of supervised process events.
///
/// This allows the reporters to send events to a remote collector.
pub trait Reporter {
    fn report(&self, event: Event) -> Result<(), anyhow::Error>;
}

pub struct TcpReporter {
    destination: String,
    reporter_id: ReporterId,
}

impl TcpReporter {
    /// Creates a new TCP reporter instance.
    ///
    /// It does not open the TCP connection yet. Stores the destination
    /// address and creates a unique reporter id.
    pub fn new(destination: String) -> Result<Self, anyhow::Error> {
        let reporter_id = ReporterId::new();
        let result = TcpReporter {
            destination,
            reporter_id,
        };
        Ok(result)
    }
}

impl Reporter for TcpReporter {
    /// Sends an event to the remote collector.
    ///
    /// The event is wrapped in an envelope and sent to the remote collector.
    /// The TCP connection is opened and closed for each event.
    fn report(&self, event: Event) -> Result<(), anyhow::Error> {
        let envelope = Envelope::new(&self.reporter_id, event);
        let mut socket = TcpStream::connect(self.destination.clone())?;
        envelope.write_into(&mut socket)?;

        Ok(())
    }
}
