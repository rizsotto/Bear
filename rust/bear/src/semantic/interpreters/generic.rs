// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;
use std::path::PathBuf;
use std::vec;

use super::super::{CompilerCall, CompilerPass, Execution, Interpreter, Recognition};
use super::matchers::source::looks_like_a_source_file;

/// A tool to recognize a compiler by executable name.
pub(super) struct Generic {
    executables: HashSet<PathBuf>,
}

impl Generic {
    pub(super) fn from(compilers: &[PathBuf]) -> Self {
        let executables = compilers.iter().cloned().collect();
        Self { executables }
    }
}

impl Interpreter for Generic {
    /// This tool is a naive implementation only considering:
    /// - the executable name,
    /// - one of the arguments is a source file,
    /// - the rest of the arguments are flags.
    fn recognize(&self, x: &Execution) -> Recognition<CompilerCall> {
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
                Recognition::Error(String::from("source file is not found"))
            } else {
                Recognition::Success(CompilerCall {
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
                })
            }
        } else {
            Recognition::Unknown
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;
    use crate::intercept::execution;

    #[test]
    fn test_matching() {
        let input = execution(
            "/usr/bin/something",
            vec![
                "something",
                "-Dthis=that",
                "-I.",
                "source.c",
                "-o",
                "source.c.o",
            ],
            "/home/user",
            HashMap::new(),
        );

        let expected = CompilerCall {
            compiler: "/usr/bin/something".into(),
            working_dir: "/home/user".into(),
            passes: vec![CompilerPass::Compile {
                flags: vec!["-Dthis=that", "-I.", "-o", "source.c.o"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                source: "source.c".into(),
                output: None,
            }],
        };

        assert_eq!(Recognition::Success(expected), SUT.recognize(&input));
    }

    #[test]
    fn test_matching_without_sources() {
        let input = execution(
            "/usr/bin/something",
            vec!["something", "--help"],
            "/home/user",
            HashMap::new(),
        );

        assert_eq!(
            Recognition::Error(String::from("source file is not found")),
            SUT.recognize(&input)
        );
    }

    #[test]
    fn test_not_matching() {
        let input = execution(
            "/usr/bin/ls",
            vec!["ls", "/home/user/build"],
            "/home/user",
            HashMap::new(),
        );

        assert_eq!(Recognition::Unknown, SUT.recognize(&input));
    }

    static SUT: std::sync::LazyLock<Generic> = std::sync::LazyLock::new(|| Generic {
        executables: vec!["/usr/bin/something"]
            .into_iter()
            .map(PathBuf::from)
            .collect(),
    });
}
