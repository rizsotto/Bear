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

use super::super::{Result, ResultExt};
use super::{CompilationDatabase, Entry, Entries};
use super::config::Config;
use crate::intercept::Event;
use crate::semantic::c_compiler::CompilerCall;
use crate::semantic::c_compiler::CompilerPass;


pub struct Builder<'a> {
    config: &'a Config,
    target: &'a CompilationDatabase,
}

impl<'a> Builder<'a> {

    pub fn new(config: &'a Config, target: &'a CompilationDatabase) -> Self {
        Builder { config, target }
    }

    pub fn build<I>(&self, events: I) -> Result<()>
        where I: Iterator<Item = Event>
    {
        let previous = if self.config.append_to_existing {
            self.target.load(true)
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
                (self.config.include_linking && pass.is_compiling()) || (pass == CompilerPass::Compilation)
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
        self.target.save(&self.config.format, result)
            .chain_err(|| "Failed to save compilation database.")
    }

    pub fn transform(&self, from_db: &CompilationDatabase) -> Result<()> {
        let previous = from_db.load(false)
            .chain_err(|| "Failed to load compilation database.")?;

        let current: Entries = previous.iter()
            .filter_map(|entry| {
                debug!("Entry from file {:?}", entry);
                CompilerCall::from(entry.command.as_ref(), entry.directory.as_path()).ok()
            })
            .filter(|call| {
                let pass = call.pass();
                debug!("Compiler runs this pass: {:?}", pass);
                (self.config.include_linking && pass.is_compiling()) || (pass == CompilerPass::Compilation)
            })
            .flat_map(|call| {
                debug!("Compiler call: {:?}", call);
                Entry::from(&call)
            })
            .inspect(|entry| {
                debug!("The output entry: {:?}", entry)
            })
            .collect();

        self.target.save(&self.config.format, current)
            .chain_err(|| "Failed to save compilation database.")
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
                .unwrap_or_else(|| std::ffi::OsString::from("o")))
    }
}
