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

pub mod config;
pub mod builder;
pub mod file;

use crate::Result;
use crate::output::clang::config::Format;

/// Represents a compilation database.
pub trait CompilationDatabase {

    fn load(&self, empty_if_not_exists: bool) -> Result<Entries>;

    fn save(&self, format: &Format, entries: Entries) -> Result<()>;
}

/// Represents an entry of the compilation database.
#[derive(Hash, Debug)]
pub struct Entry {
    pub directory: std::path::PathBuf,
    pub file: std::path::PathBuf,
    pub command: Vec<String>,
    pub output: Option<std::path::PathBuf>,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Entry) -> bool {
        self.directory == other.directory
            && self.file == other.file
            && self.command == other.command
    }
}

pub type Entries = Vec<Entry>;
