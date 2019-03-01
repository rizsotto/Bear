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

use std::path;

use crate::{Result, ResultExt};
use crate::event::Event;
use crate::compilation::CompilerCall;
use crate::compilation::pass::CompilerPass;
use crate::database::{CompilationDatabase, Entry, Entries};
use database::file::JsonCompilationDatabase;


/// Represents a compilation database building strategy.
pub struct Builder {
    pub database: Box<CompilationDatabase>,
    pub append_to_existing: bool,
    pub include_headers: bool,      // TODO
    pub include_linking: bool,
    pub relative_to: Option<path::PathBuf>, // TODO
    pub compilers: CompilerFilter,  // TODO
    pub sources: SourceFilter,      // TODO
    pub flags: FlagFilter,          // TODO
}

impl Builder {

    pub fn build<I>(&self, events: I) -> Result<()>
        where I: Iterator<Item = Event>
    {
        let previous = if self.append_to_existing && self.database.exists() {
            self.database.load()
                .chain_err(|| "Failed to load compilation database.")?
        } else {
            Entries::new()
        };

        let current: Entries = events
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
                Entry::from(&call)
            })
            .inspect(|entry| {
                debug!("The output entry: {:?}", entry)
            })
            .collect();

        let mut result = Entries::new();
        result.extend(previous);
        result.extend(current);
        result.dedup();
        self.database.save(result)
            .chain_err(|| "Failed to save compilation database.")
    }

    pub fn transform(&self) -> Result<()> {
        let previous = self.database.load()
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
                Entry::from(&call)
            })
            .inspect(|entry| {
                debug!("The output entry: {:?}", entry)
            })
            .collect();

        self.database.save(current)
            .chain_err(|| "Failed to save compilation database.")
    }
}

impl Default for Builder {
    fn default() -> Self {
        Builder {
            database: Box::new(
                JsonCompilationDatabase::new(
                    path::Path::new("./compile_commands.json"),
                    Format::default()
            )),
            append_to_existing: false,
            include_headers: false,
            include_linking: false,
            relative_to: None,
            compilers: CompilerFilter::default(),
            sources: SourceFilter::default(),
            flags: FlagFilter::default(),
        }
    }
}

/// Represents the expected format of the JSON compilation database.
pub struct Format {
    pub command_as_array: bool,
    pub drop_output_field: bool,            // TODO
}

impl Default for Format {
    fn default() -> Self {
        Format {
            command_as_array: true,
            drop_output_field: false,
        }
    }
}

pub struct CompilerFilter {
    pub drop_wrapper: bool,                 // TODO
//    c_compilers: Vec<String>,
//    cxx_compilers: Vec<String>,
}

impl Default for CompilerFilter {
    fn default() -> Self {
        CompilerFilter {
            drop_wrapper: true,
        }
    }
}

pub struct FlagFilter {

}

impl Default for FlagFilter {
    fn default() -> Self {
        unimplemented!()
    }
}

pub struct SourceFilter {
    pub extensions_to_exclude: Vec<String>,
    pub extensions_to_include: Vec<String>,
    pub path_to_exclude: Vec<std::path::PathBuf>,
    pub path_to_include: Vec<std::path::PathBuf>,
}

impl Default for SourceFilter {
    fn default() -> Self {
        unimplemented!()
    }
}

impl Entry {
    pub fn from(compilation: &CompilerCall) -> Entries {
        entry::from(compilation)
    }
}

mod entry {
    use super::*;

    pub fn from(compilation: &CompilerCall) -> Vec<Entry> {
        let make_output= |source: &path::PathBuf| {
            let is_linking = compilation.pass() == CompilerPass::Linking;
            match (is_linking, compilation.output()) {
                (false, Some(o)) => o.to_path_buf(),
                _ => object_from_source(source),
            }
        };

        let make_command = |source: &path::PathBuf, output: &path::PathBuf| {
            let mut result = compilation.compiler().to_strings();
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
