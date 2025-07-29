// SPDX-License-Identifier: GPL-3.0-or-later

use super::super::{ArgumentGroup, ArgumentKind, CompilerCommand, Execution, Interpreter};
use super::matchers::source::looks_like_a_source_file;
use crate::semantic::Command;
use std::collections::HashSet;
use std::path::PathBuf;
use std::vec;

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
        if !self.executables.contains(&execution.executable) {
            return None;
        }

        let mut annotated_args = Vec::new();
        let mut iter = execution.arguments.iter().peekable();

        // First argument is the compiler itself
        if let Some(first) = iter.next() {
            annotated_args.push(ArgumentGroup {
                args: vec![first.clone()],
                kind: ArgumentKind::Compiler,
            });
        }

        while let Some(arg) = iter.next() {
            if looks_like_a_source_file(arg) {
                annotated_args.push(ArgumentGroup {
                    args: vec![arg.clone()],
                    kind: ArgumentKind::Source,
                });
            } else if arg == "-o" {
                if let Some(output) = iter.next() {
                    annotated_args.push(ArgumentGroup {
                        args: vec![arg.clone(), output.clone()],
                        kind: ArgumentKind::Output,
                    });
                } else {
                    annotated_args.push(ArgumentGroup {
                        args: vec![arg.clone()],
                        kind: ArgumentKind::Other(None),
                    });
                }
            } else if arg.starts_with('-') {
                // Handle switches with values (e.g., -I include, -D define)
                if (arg == "-I" || arg == "-D" || arg == "-L") && iter.peek().is_some() {
                    let value = iter.next().unwrap();
                    annotated_args.push(ArgumentGroup {
                        args: vec![arg.clone(), value.clone()],
                        kind: ArgumentKind::Other(None),
                    });
                } else if arg.starts_with("-I") || arg.starts_with("-D") || arg.starts_with("-L") {
                    // Handle combined flags like -I. or -DFOO=bar
                    annotated_args.push(ArgumentGroup {
                        args: vec![arg.clone()],
                        kind: ArgumentKind::Other(None),
                    });
                } else {
                    annotated_args.push(ArgumentGroup {
                        args: vec![arg.clone()],
                        kind: ArgumentKind::Other(None),
                    });
                }
            } else {
                annotated_args.push(ArgumentGroup {
                    args: vec![arg.clone()],
                    kind: ArgumentKind::Other(None),
                });
            }
        }

        Some(Command::Compiler(CompilerCommand::new(
            execution.working_dir.clone(),
            execution.executable.clone(),
            annotated_args,
        )))
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_matching() {
        let input = Execution::from_strings(
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

        let result = SUT.recognize(&input);

        match result {
            Some(Command::Compiler(cmd)) => {
                assert_eq!(cmd.working_dir, PathBuf::from("/home/user"));
                assert_eq!(cmd.executable, PathBuf::from("/usr/bin/something"));
                assert_eq!(cmd.arguments.len(), 5);

                // Check compiler argument
                assert_eq!(cmd.arguments[0].kind, ArgumentKind::Compiler);
                assert_eq!(cmd.arguments[0].args, vec!["something"]);

                // Check switch argument
                assert_eq!(cmd.arguments[1].kind, ArgumentKind::Other(None));
                assert_eq!(cmd.arguments[1].args, vec!["-Dthis=that"]);

                // Check switch with value (combined form)
                assert_eq!(cmd.arguments[2].kind, ArgumentKind::Other(None));
                assert_eq!(cmd.arguments[2].args, vec!["-I."]);

                // Check source file
                assert_eq!(cmd.arguments[3].kind, ArgumentKind::Source);
                assert_eq!(cmd.arguments[3].args, vec!["source.c"]);

                // Check output
                assert_eq!(cmd.arguments[4].kind, ArgumentKind::Output);
                assert_eq!(cmd.arguments[4].args, vec!["-o", "source.c.o"]);
            }
            _ => panic!("Expected Some(Command::Compiler(_))"),
        }
    }

    #[test]
    fn test_matching_without_sources() {
        let input = Execution::from_strings(
            "/usr/bin/something",
            vec!["something", "--help"],
            "/home/user",
            HashMap::new(),
        );
        let result = SUT.recognize(&input);

        match result {
            Some(Command::Compiler(cmd)) => {
                assert_eq!(cmd.working_dir, PathBuf::from("/home/user"));
                assert_eq!(cmd.executable, PathBuf::from("/usr/bin/something"));
                assert_eq!(cmd.arguments.len(), 2);

                // Check compiler argument
                assert_eq!(cmd.arguments[0].kind, ArgumentKind::Compiler);
                assert_eq!(cmd.arguments[0].args, vec!["something"]);

                // Check switch argument
                assert_eq!(cmd.arguments[1].kind, ArgumentKind::Other(None));
                assert_eq!(cmd.arguments[1].args, vec!["--help"]);
            }
            _ => panic!("Expected Some(Command::Compiler(_))"),
        }
    }

    #[test]
    fn test_not_matching() {
        let input = Execution::from_strings(
            "/usr/bin/ls",
            vec!["ls", "/home/user/build"],
            "/home/user",
            HashMap::new(),
        );
        let result = SUT.recognize(&input);

        assert!(result.is_none());
    }

    #[test]
    fn test_complex_argument_parsing() {
        let input = Execution::from_strings(
            "/usr/bin/something",
            vec![
                "gcc",
                "-c",
                "-Wall",
                "-Werror",
                "-I/usr/include",
                "-I.",
                "-DDEBUG=1",
                "-DVERSION=\"1.0\"",
                "main.c",
                "utils.c",
                "-o",
                "output.o",
                "-L/usr/lib",
                "-lmath",
            ],
            "/home/user/project",
            HashMap::new(),
        );

        let result = SUT.recognize(&input);

        match result {
            Some(Command::Compiler(cmd)) => {
                assert_eq!(cmd.working_dir, PathBuf::from("/home/user/project"));
                assert_eq!(cmd.executable, PathBuf::from("/usr/bin/something"));
                assert_eq!(cmd.arguments.len(), 13);

                // Check compiler
                assert_eq!(cmd.arguments[0].kind, ArgumentKind::Compiler);
                assert_eq!(cmd.arguments[0].args, vec!["gcc"]);

                // Check various switches
                assert_eq!(cmd.arguments[1].kind, ArgumentKind::Other(None));
                assert_eq!(cmd.arguments[1].args, vec!["-c"]);

                assert_eq!(cmd.arguments[2].kind, ArgumentKind::Other(None));
                assert_eq!(cmd.arguments[2].args, vec!["-Wall"]);

                assert_eq!(cmd.arguments[3].kind, ArgumentKind::Other(None));
                assert_eq!(cmd.arguments[3].args, vec!["-Werror"]);

                // Check include paths (both separate and combined forms)
                assert_eq!(cmd.arguments[4].kind, ArgumentKind::Other(None));
                assert_eq!(cmd.arguments[4].args, vec!["-I/usr/include"]);

                assert_eq!(cmd.arguments[5].kind, ArgumentKind::Other(None));
                assert_eq!(cmd.arguments[5].args, vec!["-I."]);

                // Check defines
                assert_eq!(cmd.arguments[6].kind, ArgumentKind::Other(None));
                assert_eq!(cmd.arguments[6].args, vec!["-DDEBUG=1"]);

                assert_eq!(cmd.arguments[7].kind, ArgumentKind::Other(None));
                assert_eq!(cmd.arguments[7].args, vec!["-DVERSION=\"1.0\""]);

                // Check source files
                assert_eq!(cmd.arguments[8].kind, ArgumentKind::Source);
                assert_eq!(cmd.arguments[8].args, vec!["main.c"]);

                assert_eq!(cmd.arguments[9].kind, ArgumentKind::Source);
                assert_eq!(cmd.arguments[9].args, vec!["utils.c"]);

                // Check output
                assert_eq!(cmd.arguments[10].kind, ArgumentKind::Output);
                assert_eq!(cmd.arguments[10].args, vec!["-o", "output.o"]);

                // Check library link arguments
                assert_eq!(cmd.arguments[11].kind, ArgumentKind::Other(None));
                assert_eq!(cmd.arguments[11].args, vec!["-L/usr/lib"]);

                assert_eq!(cmd.arguments[12].kind, ArgumentKind::Other(None));
                assert_eq!(cmd.arguments[12].args, vec!["-lmath"]);
            }
            _ => panic!("Expected Some(Command::Compiler(_))"),
        }
    }

    static SUT: std::sync::LazyLock<Generic> = std::sync::LazyLock::new(|| Generic {
        executables: vec!["/usr/bin/something"]
            .into_iter()
            .map(PathBuf::from)
            .collect(),
    });
}
