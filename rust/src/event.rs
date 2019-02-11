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
use serde_json;
use std::path;

pub type DateTime = chrono::DateTime<chrono::Utc>;
pub type ProcessId = u32;
pub type ExitCode = i32;
pub type SignalId = i32;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessStarted {
    pub pid: ProcessId,
    pub ppid: ProcessId,
    pub cwd: path::PathBuf,
    pub cmd: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessStartFailed {
    pub cwd: path::PathBuf,
    pub cmd: Vec<String>,
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessStopped {
    pub pid: ProcessId,
    pub exit_code: ExitCode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessSignaled {
    pub pid: ProcessId,
    pub signal: SignalId,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    Started(ProcessStarted, DateTime),
    Failed(ProcessStartFailed, DateTime),
    Stopped(ProcessStopped, DateTime),
    Signaled(ProcessSignaled, DateTime),
}
