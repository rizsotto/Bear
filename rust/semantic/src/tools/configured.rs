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
use std::vec;

use crate::configuration::CompilerToRecognize;
use crate::execution::Execution;
use crate::tools::{CompilerPass, Semantic};
use crate::tools::{Any, RecognitionResult, Tool};
use crate::tools::matchers::source::looks_like_a_source_file;
use crate::tools::RecognitionResult::{NotRecognized, Recognized};

pub(crate) struct Configured {
    pub executable: PathBuf,
    pub flags_to_add: Vec<String>,
    pub flags_to_remove: Vec<String>,
}

impl Configured {
    pub(crate) fn new(config: &CompilerToRecognize) -> Box<dyn Tool> {
        Box::new(
            Configured {
                executable: config.executable.clone(),
                flags_to_add: config.flags_to_add.clone(),
                flags_to_remove: config.flags_to_remove.clone(),
            }
        )
    }

    pub(crate) fn from(configs: &[CompilerToRecognize]) -> Box<dyn Tool> {
        Any::new(configs.iter().map(Configured::new).collect())
    }
}

impl Tool for Configured {
    /// Any of the tool recognize the semantic, will be returned as result.
    fn recognize(&self, x: &Execution) -> RecognitionResult {
        if x.executable == self.executable {
            let mut flags = vec![];
            let mut sources = vec![];

            // find sources and filter out requested flags.
            for argument in x.arguments.iter().skip(1) {
                if self.flags_to_remove.contains(argument) {
                    continue;
                } else if looks_like_a_source_file(argument.as_str()) {
                    sources.push(PathBuf::from(argument));
                } else {
                    flags.push(argument.clone());
                }
            }
            // extend flags with requested flags.
            for flag in &self.flags_to_add {
                flags.push(flag.clone());
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

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use lazy_static::lazy_static;

    use crate::vec_of_strings;

    use super::*;

    #[test]
    fn test_matching() {
        let input = Execution {
            executable: PathBuf::from("/usr/bin/something"),
            arguments: vec_of_strings!["something", "-Dthis=that", "-I.", "source.c", "-o", "source.c.o"],
            working_dir: PathBuf::from("/home/user"),
            environment: HashMap::new(),
        };

        let expected = Semantic::Compiler {
            compiler: PathBuf::from("/usr/bin/something"),
            working_dir: PathBuf::from("/home/user"),
            passes: vec![
                CompilerPass::Compile {
                    flags: vec_of_strings!["-Dthis=that", "-o", "source.c.o", "-Wall"],
                    source: PathBuf::from("source.c"),
                    output: None,
                }
            ],
        };

        assert_eq!(Recognized(Ok(expected)), SUT.recognize(&input));
    }

    #[test]
    fn test_matching_without_sources() {
        let input = Execution {
            executable: PathBuf::from("/usr/bin/something"),
            arguments: vec_of_strings!["something", "--help"],
            working_dir: PathBuf::from("/home/user"),
            environment: HashMap::new(),
        };

        assert_eq!(Recognized(Err(String::from("source file is not found"))), SUT.recognize(&input));
    }

    #[test]
    fn test_not_matching() {
        let input = Execution {
            executable: PathBuf::from("/usr/bin/cc"),
            arguments: vec_of_strings!["cc", "-Dthis=that", "-I.", "source.c", "-o", "source.c.o"],
            working_dir: PathBuf::from("/home/user"),
            environment: HashMap::new(),
        };

        assert_eq!(NotRecognized, SUT.recognize(&input));
    }

    lazy_static! {
        static ref SUT: Configured = Configured {
            executable: PathBuf::from("/usr/bin/something"),
            flags_to_remove: vec_of_strings!["-I."],
            flags_to_add: vec_of_strings!["-Wall"],
        };
    }
}
