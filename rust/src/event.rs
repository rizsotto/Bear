/*  Copyright (C) 2012-2018 by László Nagy
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

use chrono;
use std::path;

pub type DateTime = chrono::DateTime<chrono::Utc>;
pub type ProcessId = u32;
pub type ExitCode = i32;
pub type SignalId = String;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Event {
    Created {
        when: DateTime,
        pid: ProcessId,
        ppid: ProcessId,
        cwd: path::PathBuf,
        cmd: Vec<String>,
    },
    TerminatedNormally {
        when: DateTime,
        pid: ProcessId,
        code: ExitCode,
    },
    TerminatedAbnormally {
        when: DateTime,
        pid: ProcessId,
        signal: SignalId,
    },
    Stopped {
        when: DateTime,
        pid: ProcessId,
        signal: SignalId,
    },
    Continued {
        when: DateTime,
        pid: ProcessId,
    },
}

impl Event {
    pub fn pid(&self) -> ProcessId {
        match self {
            Event::Created { pid, .. } => *pid,
            Event::TerminatedNormally { pid, .. } => *pid,
            Event::TerminatedAbnormally { pid, .. } => *pid,
            Event::Stopped { pid, .. } => *pid,
            Event::Continued { pid, .. } => *pid,
        }
    }

    pub fn when(&self) -> DateTime {
        match self {
            Event::Created { when, .. } => *when,
            Event::TerminatedNormally { when, .. } => *when,
            Event::TerminatedAbnormally { when, .. } => *when,
            Event::Stopped { when, .. } => *when,
            Event::Continued { when, .. } => *when,
        }
    }
}