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

use crate::{ErrorKind, Result};
use super::compiler;
use super::flags;
use super::pass;
use super::database;

#[derive(Default)]
struct CompilerExecution {
    directory: path::PathBuf,
    compiler: path::PathBuf,
    pass: pass::CompilerPass,
    flags: Vec<String>,
    inputs: Vec<path::PathBuf>,
    output: Option<path::PathBuf>,
}

impl CompilerExecution {
    fn new(working_dir: &str, compiler: &str) -> Self {
        let mut result: Self = Default::default();
        result.directory = path::PathBuf::from(working_dir);
        result.compiler = path::PathBuf::from(compiler);
        result
    }

    fn pass(&mut self, flag: &str) -> bool {
        self.pass.take(flag)
    }

    fn output(&mut self, flag: &str, it: &mut flags::FlagIterator) -> bool {
        if "-o" == flag {
            if let Some(output) = it.next() {
                self.output = Some(path::PathBuf::from(output));
            }
            return true;
        }
        false
    }

    fn flags(&mut self, flag: &str, it: &mut flags::FlagIterator) -> bool {
        //            # some parameters look like a filename, take those explicitly
        //            elif arg in {'-D', '-I'}:
        //                result.flags.extend([arg, next(args)])
        //            # and consider everything else as compile option.
        //            else:
        //                result.flags.append(arg)
        unimplemented!()
    }

    fn source(&mut self, file: &str) -> bool {
        //            # parameter which looks source file is taken...
        //            elif re.match(r'^[^-].+', arg) and classify_source(arg):
        //                result.files.append(arg)
        unimplemented!()
    }

    fn build(&self) -> Result<Vec<database::Entry>> {
        if !self.pass.is_compiling() {
            Err(ErrorKind::CompilationError("Compiler is not doing compilation.").into())
        } else if self.inputs.is_empty() {
            Err(ErrorKind::CompilationError("Have not found source files.").into())
        } else {
            let entries: Vec<_> = self
                .inputs
                .iter()
                .map(|input| {
                    let mut command: Vec<String> = Vec::new();
                    command.push(
                        self.compiler
                            .clone()
                            .into_os_string()
                            .into_string()
                            .unwrap(),
                    );
                    command.push("-c".to_string());
                    command.extend_from_slice(self.flags.as_ref());
                    command.push(input.clone().into_os_string().into_string().unwrap());
                    database::Entry {
                        directory: self.directory.clone(),
                        file: input.clone(),
                        command: command,
                        output: self.output.clone(),
                    }
                }).collect();
            Ok(entries)
        }
    }
}

/// Returns a value when the command is a compilation, None otherwise.
///
/// # Arguments
/// `classifier` - helper object to detect compiler
/// `command` - the command to classify
fn parse_command(
    classifier: &compiler::Classifier,
    working_dir: &str,
    command: &[String],
) -> Result<Vec<database::Entry>> {
    debug!("input was: {:?}", command);
    match classifier.split(command) {
        Some(compiler_and_parameters) => {
            let mut result =
                CompilerExecution::new(working_dir, compiler_and_parameters.0.as_str());
            let mut it = flags::FlagIterator::from(compiler_and_parameters.1);
            while let Some(arg) = it.next() {
                // if it's a pass modifier flag, update it and move on.
                if result.pass(arg.as_str()) {
                    continue;
                }
                // take the output flag
                if result.output(arg.as_str(), &mut it) {
                    continue;
                }
                // take flags
                if result.flags(arg.as_str(), &mut it) {
                    continue;
                }
                // take the rest as source file
                if result.source(arg.as_str()) {
                    continue;
                }
            }
            result.build()
        }
        _ => Err(ErrorKind::CompilationError("Compiler not recognized.").into()),
    }
}
