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

use std::collections;
use std::path;

use crate::{Result, ResultExt};
use crate::compilation::CompilerCall;
use crate::compilation::compiler::CompilerFilter;
use crate::compilation::flags::FlagFilter;
use crate::compilation::source::SourceFilter;
use crate::protocol::collector::Protocol;
use crate::database::file;
use compilation::pass::CompilerPass;


/// Represents a compilation database building strategy.
pub struct Builder {
    pub format: Format,
    pub append_to_existing: bool,
    pub include_headers: bool,      // TODO
    pub include_linking: bool,
    pub compilers: CompilerFilter,  // TODO
    pub sources: SourceFilter,      // TODO
    pub flags: FlagFilter,          // TODO
}

impl Builder {

    pub fn build(&self, path: &path::Path, collector: &Protocol) -> Result<()> {
        let previous = if self.append_to_existing {
            debug!("Reading from: {:?}", path);
            file::load(path)
                .chain_err(|| "Failed to load compilation database.")?
        } else {
            Entries::new()
        };

        let current: Entries = collector.events()
            .filter_map(|event| {
                debug!("Event from protocol: {:?}", event);
                event.to_execution()
            })
            .filter_map(|execution| {
                debug!("Execution: {:?} @ {:?}", execution.0, execution.1);
                CompilerCall::from(&execution.0, execution.1.as_ref()).ok()
            })
            .filter(|call| {
                let pass = call.pass();
                debug!("Compiler runs this pass: {:?}", pass);
                (self.include_linking && pass.is_compiling()) || (pass == CompilerPass::Compilation)
            })
            .flat_map(|call| {
                debug!("Compiler call: {:?}", call);
                Entry::from(&call, &self.format)
            })
            .inspect(|entry| {
                debug!("The output entry: {:?}", entry)
            })
            .collect();

        debug!("Writing into: {:?}", path);
        file::save(path, previous.union(&current), &self.format)
            .chain_err(|| "Failed to save compilation database.")
    }

    pub fn transform(&self, path: &path::Path) -> Result<()> {
        debug!("Reading from: {:?}", path);
        let previous = file::load(path)
            .chain_err(|| "Failed to load compilation database.")?;

        let current: Entries = previous.iter()
            .filter_map(|entry| {
                debug!("Entry from file {:?}", entry);
                CompilerCall::from(entry.command.as_ref(), entry.directory.as_path()).ok()
            })
            .filter(|call| {
                let pass = call.pass();
                debug!("Compiler runs this pass: {:?}", pass);
                (self.include_linking && pass.is_compiling()) || (pass == CompilerPass::Compilation)
            })
            .flat_map(|call| {
                debug!("Compiler call: {:?}", call);
                Entry::from(&call, &self.format)
            })
            .inspect(|entry| {
                debug!("The output entry: {:?}", entry)
            })
            .collect();

        debug!("Writing into: {:?}", path);
        file::save(path, current.iter(), &self.format)
            .chain_err(|| "Failed to save compilation database.")
    }
}

impl Default for Builder {
    fn default() -> Self {
        Builder {
            format: Format::default(),
            append_to_existing: false,
            include_headers: false,
            include_linking: false,
            compilers: CompilerFilter::default(),
            sources: SourceFilter::default(),
            flags: FlagFilter::default(),
        }
    }
}

/// Represents the expected format of the JSON compilation database.
pub struct Format {
    pub relative_to: Option<path::PathBuf>, // TODO
    pub command_as_array: bool,
    pub drop_output_field: bool,            // TODO
    pub drop_wrapper: bool,
}

impl Default for Format {
    fn default() -> Self {
        Format {
            relative_to: None,
            command_as_array: true,
            drop_output_field: false,
            drop_wrapper: true,
        }
    }
}


/// Represents a generic entry of the compilation database.
#[derive(Hash, Debug)]
pub struct Entry {
    pub directory: path::PathBuf,
    pub file: path::PathBuf,
    pub command: Vec<String>,
    pub output: Option<path::PathBuf>,
}

impl Entry {
    pub fn from(compilation: &CompilerCall, format: &Format) -> Vec<Entry> {
        entry::from(compilation, format)
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Entry) -> bool {
        self.directory == other.directory
            && self.file == other.file
            && self.command == other.command
    }
}

impl Eq for Entry {
}

pub type Entries = collections::HashSet<Entry>;

mod entry {
    use super::*;

    pub fn from(compilation: &CompilerCall, format: &Format) -> Vec<Entry> {
        let make_output= |source: &path::PathBuf| {
            let is_linking = compilation.pass() == CompilerPass::Linking;
            match (is_linking, compilation.output()) {
                (false, Some(o)) => o.to_path_buf(),
                _ => object_from_source(source),
            }
        };

        let make_command = |source: &path::PathBuf, output: &path::PathBuf| {
            let mut result = compilation.compiler().to_strings(format.drop_wrapper);
            result.push(compilation.pass().to_string());
            result.append(&mut compilation.flags());
            result.push(source.to_string_lossy().into_owned());
            result.push("-o".to_string());
            result.push(output.to_string_lossy().into_owned());
            result
        };

        compilation.sources()
            .iter()
            .map(|source| {
                let output = make_output(source);
                let command = make_command(source, &output);
                Entry {
                    directory: compilation.work_dir.clone(),
                    file: source.to_path_buf(),
                    output: Some(output),
                    command,
                }
            })
            .collect::<Vec<Entry>>()
    }

    fn object_from_source(source: &path::Path) -> path::PathBuf {
        source.with_extension(
            source.extension()
                .map(|e| {
                    let mut result = e.to_os_string();
                    result.push(".o");
                    result
                })
                .unwrap_or(std::ffi::OsString::from("o")))
    }
}
