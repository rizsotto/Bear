// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides types and logic for interpreting and representing compiler commands.
//!
//! It defines how to classify arguments, group them, and convert them into entries suitable
//! for further processing (e.g., for use with Clang tooling).

use super::interpreters::combinators::Any;
use super::interpreters::generic::Generic;
use super::interpreters::ignore::IgnoreByPath;
use super::{clang, FormatConfig, Interpreter};
use crate::config;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod combinators;
pub mod generic;
mod ignore;
mod matchers;

/// Represents a full compiler command invocation.
///
/// The [`working_dir`] is the directory where the command is executed,
/// the [`executable`] is the path to the compiler binary,
/// while [`arguments`] contains the command-line arguments annotated
/// with their meaning (e.g., source files, output files, switches).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompilerCommand {
    pub working_dir: PathBuf,
    pub executable: PathBuf,
    pub arguments: Vec<ArgumentGroup>,
}

/// Represents a group of arguments of the same intent.
///
/// Groups the arguments which belongs together, and annotate the meaning of them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArgumentGroup {
    pub args: Vec<String>,
    pub kind: ArgumentKind,
}

/// Represents the meaning of the argument in the compiler call. Identifies
/// the purpose of each argument in the command line.
///
/// Variants:
/// - `Compiler`: The compiler executable itself.
/// - `Source`: A source file to be compiled.
/// - `Output`: An output file or related argument (e.g., `-o output.o`).
/// - `Switch`: A compiler switch or flag (e.g., `-Wall`).
/// - `Other`: Any other argument not classified above.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArgumentKind {
    Compiler,
    Source,
    Output,
    Switch,
    Other,
}

impl CompilerCommand {
    pub fn new(working_dir: PathBuf, executable: PathBuf, arguments: Vec<ArgumentGroup>) -> Self {
        Self {
            working_dir,
            executable,
            arguments,
        }
    }

    pub fn to_entries(&self, _config: &FormatConfig) -> Vec<clang::Entry> {
        // Find all source files in the arguments
        let source_files: Vec<String> = self
            .arguments
            .iter()
            .filter(|arg| arg.kind == ArgumentKind::Source)
            .flat_map(|arg| &arg.args)
            .cloned()
            .collect();

        // If no source files found, return empty vector
        if source_files.is_empty() {
            return vec![];
        }

        // Build the full command arguments by flattening all argument args
        let mut command_args = vec![self.executable.to_string_lossy().to_string()];
        for arg in &self.arguments {
            command_args.extend(arg.args.iter().cloned());
        }

        // Find output file if present
        let output_file = self
            .arguments
            .iter()
            .filter(|arg| arg.kind == ArgumentKind::Output)
            .flat_map(|arg| &arg.args)
            .skip(1) // Skip the "-o" flag itself, take the output filename
            .next()
            .map(|s| PathBuf::from(s));

        // Create one entry per source file
        source_files
            .into_iter()
            .map(|source_file| {
                clang::Entry::from_arguments(
                    source_file,
                    command_args.clone(),
                    &self.working_dir,
                    output_file.as_ref(),
                )
            })
            .collect()
    }
}

/// Creates an interpreter to recognize the compiler calls.
///
/// Using the configuration we can define which compilers to include and exclude.
/// Also read the environment variables to detect the compiler to include (and
/// make sure those are not excluded either).
// TODO: Use the CC or CXX environment variables to detect the compiler to include.
//       Use the CC or CXX environment variables and make sure those are not excluded.
//       Make sure the environment variables are passed to the method.
// TODO: Take environment variables as input.
pub fn create<'a>(config: &config::Main) -> impl Interpreter + 'a {
    let compilers_to_include = match &config.intercept {
        config::Intercept::Wrapper { executables, .. } => executables.clone(),
        _ => vec![],
    };
    let compilers_to_exclude = match &config.output {
        config::Output::Clang { compilers, .. } => compilers
            .iter()
            .filter(|compiler| compiler.ignore == config::IgnoreOrConsider::Always)
            .map(|compiler| compiler.path.clone())
            .collect(),
        _ => vec![],
    };

    let mut interpreters: Vec<Box<dyn Interpreter>> = vec![
        // ignore executables which are not compilers,
        Box::new(IgnoreByPath::default()),
        // recognize default compiler
        Box::new(Generic::from(&[PathBuf::from("/usr/bin/cc")])),
    ];

    if !compilers_to_include.is_empty() {
        let tool = Generic::from(&compilers_to_include);
        interpreters.push(Box::new(tool));
    }

    if !compilers_to_exclude.is_empty() {
        let tool = IgnoreByPath::from(&compilers_to_exclude);
        interpreters.insert(0, Box::new(tool));
    }

    Any::new(interpreters)
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;
    use crate::config;
    use crate::intercept::{execution, Execution};

    #[test]
    fn test_create_interpreter_with_default_config() {
        let config = config::Main::default();

        let interpreter = create(&config);

        let result = interpreter.recognize(&EXECUTION);
        assert!(matches!(result, Some(_)));
    }

    #[test]
    fn test_create_interpreter_with_compilers_to_include() {
        let config = config::Main {
            intercept: config::Intercept::Wrapper {
                executables: vec!["/usr/bin/cc".into()],
                path: "/usr/libexec/bear".into(),
                directory: "/tmp".into(),
            },
            ..Default::default()
        };

        let interpreter = create(&config);

        let result = interpreter.recognize(&EXECUTION);
        assert!(matches!(result, Some(_)));
    }

    // #[test]
    // fn test_create_interpreter_with_compilers_to_exclude() {
    //     let config = config::Main {
    //         output: config::Output::Clang {
    //             compilers: vec![config::Compiler {
    //                 path: PathBuf::from("/usr/bin/cc"),
    //                 ignore: config::IgnoreOrConsider::Always,
    //                 arguments: config::Arguments::default(),
    //             }],
    //             sources: config::SourceFilter::default(),
    //             duplicates: config::DuplicateFilter::default(),
    //             format: config::Format::default(),
    //         },
    //         ..Default::default()
    //     };
    //
    //     let interpreter = create(&config);
    //
    //     let result = interpreter.recognize(&EXECUTION);
    //     assert!(matches!(result, Recognition::Ignored(_)));
    // }

    static EXECUTION: std::sync::LazyLock<Execution> = std::sync::LazyLock::new(|| {
        execution(
            "/usr/bin/cc",
            vec!["cc", "-c", "-Wall", "main.c"],
            "/home/user",
            HashMap::new(),
        )
    });

    #[test]
    fn test_compiler_command_to_entries_single_source() {
        let cmd = CompilerCommand::new(
            PathBuf::from("/home/user"),
            PathBuf::from("/usr/bin/gcc"),
            vec![
                ArgumentGroup {
                    args: vec!["-c".to_string()],
                    kind: ArgumentKind::Switch,
                },
                ArgumentGroup {
                    args: vec!["-Wall".to_string()],
                    kind: ArgumentKind::Switch,
                },
                ArgumentGroup {
                    args: vec!["main.c".to_string()],
                    kind: ArgumentKind::Source,
                },
            ],
        );

        let config = crate::semantic::FormatConfig::default();
        let entries = cmd.to_entries(&config);

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.file, PathBuf::from("main.c"));
        assert_eq!(entry.directory, PathBuf::from("/home/user"));
        assert_eq!(
            entry.arguments,
            vec!["/usr/bin/gcc", "-c", "-Wall", "main.c"]
        );
        assert_eq!(entry.output, None);
    }

    #[test]
    fn test_compiler_command_to_entries_multiple_sources() {
        let cmd = CompilerCommand::new(
            PathBuf::from("/home/user"),
            PathBuf::from("/usr/bin/g++"),
            vec![
                ArgumentGroup {
                    args: vec!["-c".to_string()],
                    kind: ArgumentKind::Switch,
                },
                ArgumentGroup {
                    args: vec!["file1.cpp".to_string()],
                    kind: ArgumentKind::Source,
                },
                ArgumentGroup {
                    args: vec!["file2.cpp".to_string()],
                    kind: ArgumentKind::Source,
                },
            ],
        );

        let config = crate::semantic::FormatConfig::default();
        let entries = cmd.to_entries(&config);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].file, PathBuf::from("file1.cpp"));
        assert_eq!(entries[1].file, PathBuf::from("file2.cpp"));

        for entry in &entries {
            assert_eq!(entry.directory, PathBuf::from("/home/user"));
            assert_eq!(
                entry.arguments,
                vec!["/usr/bin/g++", "-c", "file1.cpp", "file2.cpp"]
            );
        }
    }

    #[test]
    fn test_compiler_command_to_entries_with_output() {
        let cmd = CompilerCommand::new(
            PathBuf::from("/tmp"),
            PathBuf::from("clang"),
            vec![
                ArgumentGroup {
                    args: vec!["-c".to_string()],
                    kind: ArgumentKind::Switch,
                },
                ArgumentGroup {
                    args: vec!["-o".to_string(), "main.o".to_string()],
                    kind: ArgumentKind::Output,
                },
                ArgumentGroup {
                    args: vec!["main.c".to_string()],
                    kind: ArgumentKind::Source,
                },
            ],
        );

        let config = crate::semantic::FormatConfig::default();
        let entries = cmd.to_entries(&config);

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.file, PathBuf::from("main.c"));
        assert_eq!(entry.directory, PathBuf::from("/tmp"));
        assert_eq!(
            entry.arguments,
            vec!["clang", "-c", "-o", "main.o", "main.c"]
        );
        assert_eq!(entry.output, Some(PathBuf::from("main.o")));
    }

    #[test]
    fn test_compiler_command_to_entries_no_sources() {
        let cmd = CompilerCommand::new(
            PathBuf::from("/home/user"),
            PathBuf::from("gcc"),
            vec![ArgumentGroup {
                args: vec!["--version".to_string()],
                kind: ArgumentKind::Switch,
            }],
        );

        let config = crate::semantic::FormatConfig::default();
        let entries = cmd.to_entries(&config);

        assert_eq!(entries.len(), 0);
    }
}
