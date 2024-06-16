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

use std::path::PathBuf;

use intercept::ipc::Execution;
use crate::tools::{CompilerPass, RecognitionResult, Semantic, Tool};
use crate::tools::matchers::source::looks_like_a_source_file;
use crate::tools::RecognitionResult::{NotRecognized, Recognized};

pub(crate) struct Wrapper {}

impl Wrapper {
    pub(crate) fn new() -> Box<dyn Tool> {
        Box::new(Wrapper {})
    }
}

impl Tool for Wrapper {
    // fixme: this is just a quick and dirty implementation.
    fn recognize(&self, x: &Execution) -> RecognitionResult {
        if x.executable == PathBuf::from("/usr/bin/g++") {
            let mut flags = vec![];
            let mut sources = vec![];

            // find sources and filter out requested flags.
            for argument in x.arguments.iter().skip(1) {
                if looks_like_a_source_file(argument.as_str()) {
                    sources.push(PathBuf::from(argument));
                } else {
                    flags.push(argument.clone());
                }
            }

            if sources.is_empty() {
                Recognized(Err(String::from("source file is not found")))
            } else {
                Recognized(
                    Ok(
                        Semantic::Compiler {
                            compiler: x.executable.clone(),
                            working_dir: x.working_dir.clone(),
                            passes: sources.iter()
                                .map(|source| {
                                    CompilerPass::Compile {
                                        source: source.clone(),
                                        output: None,
                                        flags: flags.clone(),
                                    }
                                })
                                .collect(),
                        }
                    )
                )
            }
        } else {
            NotRecognized
        }
    }
}
