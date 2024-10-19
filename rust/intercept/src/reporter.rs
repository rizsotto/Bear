/*  Copyright (C) 2012-2024 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

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
