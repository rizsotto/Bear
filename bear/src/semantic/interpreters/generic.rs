// SPDX-License-Identifier: GPL-3.0-or-later

use super::super::{Execution, Interpreter};
use super::matchers::source::looks_like_a_source_file;
use crate::semantic::{clang, Command, FormatConfig};
use std::collections::HashSet;
use std::path::PathBuf;
use std::vec;

/// Represents an executed command semantic.
#[derive(Debug, PartialEq)]
pub struct CompilerCall {
    pub compiler: PathBuf,
    pub working_dir: PathBuf,
    pub passes: Vec<CompilerPass>,
}

/// Represents a compiler call pass.
#[derive(Debug, PartialEq)]
pub enum CompilerPass {
    Preprocess,
    Compile {
        source: PathBuf,
        output: Option<PathBuf>,
        flags: Vec<String>,
    },
}

impl Clone for CompilerCall {
    fn clone(&self) -> Self {
        Self {
            compiler: self.compiler.clone(),
            working_dir: self.working_dir.clone(),
            passes: self.passes.clone(),
        }
    }
}

impl Clone for CompilerPass {
    fn clone(&self) -> Self {
        match self {
            CompilerPass::Preprocess => CompilerPass::Preprocess,
            CompilerPass::Compile {
                source,
                output,
                flags,
            } => CompilerPass::Compile {
                source: source.clone(),
                output: output.clone(),
                flags: flags.clone(),
            },
        }
    }
}

#[derive(Debug, PartialEq)]
struct FailedCompilerCall {
    reason: String,
}

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
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        None
    }
    // fn recognize(&self, x: &Execution) -> Option<CompilerCall> {
    //     if self.executables.contains(&x.executable) {
    //         let mut flags = vec![];
    //         let mut sources = vec![];
    //
    //         // find sources and filter out requested flags.
    //         for argument in x.arguments.iter().skip(1) {
    //             if looks_like_a_source_file(argument.as_str()) {
    //                 sources.push(PathBuf::from(argument));
    //             } else {
    //                 flags.push(argument.clone());
    //             }
    //         }
    //
    //         if sources.is_empty() {
    //             None
    //         } else {
    //             Some(CompilerCall {
    //                 compiler: x.executable.clone(),
    //                 working_dir: x.working_dir.clone(),
    //                 passes: sources
    //                     .iter()
    //                     .map(|source| CompilerPass::Compile {
    //                         source: source.clone(),
    //                         output: None,
    //                         flags: flags.clone(),
    //                     })
    //                     .collect(),
    //             })
    //         }
    //     } else {
    //         None
    //     }
    // }
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

        // let expected = CompilerCall {
        //     compiler: "/usr/bin/something".into(),
        //     working_dir: "/home/user".into(),
        //     passes: vec![CompilerPass::Compile {
        //         flags: vec!["-Dthis=that", "-I.", "-o", "source.c.o"]
        //             .into_iter()
        //             .map(String::from)
        //             .collect(),
        //         source: "source.c".into(),
        //         output: None,
        //     }],
        // };
        let result = SUT.recognize(&input);

        assert!(matches!(result, Some(_)));
    }

    #[test]
    fn test_matching_without_sources() {
        let input = execution(
            "/usr/bin/something",
            vec!["something", "--help"],
            "/home/user",
            HashMap::new(),
        );
        let result = SUT.recognize(&input);

        assert!(matches!(result, Some(_)));
    }

    #[test]
    fn test_not_matching() {
        let input = execution(
            "/usr/bin/ls",
            vec!["ls", "/home/user/build"],
            "/home/user",
            HashMap::new(),
        );
        let result = SUT.recognize(&input);

        assert!(matches!(result, None));
    }

    static SUT: std::sync::LazyLock<Generic> = std::sync::LazyLock::new(|| Generic {
        executables: vec!["/usr/bin/something"]
            .into_iter()
            .map(PathBuf::from)
            .collect(),
    });
}
