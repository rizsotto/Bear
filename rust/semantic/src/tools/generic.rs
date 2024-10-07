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
use std::collections::HashSet;
use std::path::PathBuf;
use std::vec;

use super::super::{CompilerPass, Meaning, RecognitionResult, Tool};
use super::matchers::source::looks_like_a_source_file;
use intercept::ipc::Execution;

/// A tool to recognize a compiler by executable name.
pub struct Generic {
    executables: HashSet<PathBuf>,
}

impl Generic {
    pub fn from(compilers: &[PathBuf]) -> Box<dyn Tool> {
        let executables = compilers.iter().map(|compiler| compiler.clone()).collect();
        Box::new(Self { executables })
    }
}

impl Tool for Generic {
    /// This tool is a naive implementation only considering:
    /// - the executable name,
    /// - one of the arguments is a source file,
    /// - the rest of the arguments are flags.
    fn recognize(&self, x: &Execution) -> RecognitionResult {
        if self.executables.contains(&x.executable) {
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
                RecognitionResult::Recognized(Err(String::from("source file is not found")))
            } else {
                RecognitionResult::Recognized(Ok(Meaning::Compiler {
                    compiler: x.executable.clone(),
                    working_dir: x.working_dir.clone(),
                    passes: sources
                        .iter()
                        .map(|source| CompilerPass::Compile {
                            source: source.clone(),
                            output: None,
                            flags: flags.clone(),
                        })
                        .collect(),
                }))
            }
        } else {
            RecognitionResult::NotRecognized
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use lazy_static::lazy_static;

    use crate::{vec_of_pathbuf, vec_of_strings};

    use super::*;

    #[test]
    fn test_matching() {
        let input = Execution {
            executable: PathBuf::from("/usr/bin/something"),
            arguments: vec_of_strings![
                "something",
                "-Dthis=that",
                "-I.",
                "source.c",
                "-o",
                "source.c.o"
            ],
            working_dir: PathBuf::from("/home/user"),
            environment: HashMap::new(),
        };

        let expected = Meaning::Compiler {
            compiler: PathBuf::from("/usr/bin/something"),
            working_dir: PathBuf::from("/home/user"),
            passes: vec![CompilerPass::Compile {
                flags: vec_of_strings!["-Dthis=that", "-I.", "-o", "source.c.o"],
                source: PathBuf::from("source.c"),
                output: None,
            }],
        };

        assert_eq!(
            RecognitionResult::Recognized(Ok(expected)),
            SUT.recognize(&input)
        );
    }

    #[test]
    fn test_matching_without_sources() {
        let input = Execution {
            executable: PathBuf::from("/usr/bin/something"),
            arguments: vec_of_strings!["something", "--help"],
            working_dir: PathBuf::from("/home/user"),
            environment: HashMap::new(),
        };

        assert_eq!(
            RecognitionResult::Recognized(Err(String::from("source file is not found"))),
            SUT.recognize(&input)
        );
    }

    #[test]
    fn test_not_matching() {
        let input = Execution {
            executable: PathBuf::from("/usr/bin/cc"),
            arguments: vec_of_strings!["cc", "-Dthis=that", "-I.", "source.c", "-o", "source.c.o"],
            working_dir: PathBuf::from("/home/user"),
            environment: HashMap::new(),
        };

        assert_eq!(RecognitionResult::NotRecognized, SUT.recognize(&input));
    }

    lazy_static! {
        static ref SUT: Generic = Generic {
            executables: vec_of_pathbuf!["/usr/bin/something"].into_iter().collect()
        };
    }
}
