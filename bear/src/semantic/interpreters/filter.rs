// SPDX-License-Identifier: GPL-3.0-or-later

//! Filtering interpreter that wraps another interpreter to filter out compiler commands
//! based on compiler paths and source directories.

use crate::config::{self, DirectoryFilter, Ignore, IgnoreOrConsider};
use crate::semantic::command::{ArgumentKind, CompilerCommand};
use crate::semantic::{Command, Execution, Interpreter};
use std::path::PathBuf;
use thiserror::Error;

/// A wrapper interpreter that applies filtering to recognized compiler commands.
pub struct FilteringInterpreter {
    inner: Box<dyn Interpreter>,
    compiler_filters: Vec<CompilerFilter>,
    source_filters: Vec<DirectoryFilter>,
}

#[derive(Debug, Clone)]
pub struct CompilerFilter {
    path: PathBuf,
    ignore: IgnoreOrConsider,
}

#[derive(Debug, Error)]
pub enum FilterError {
    #[error("Compiler filtered out: {0}")]
    CompilerFiltered(String),
    #[error("Source directory filtered out: {0}")]
    SourceFiltered(String),
}

impl FilteringInterpreter {
    /// Creates a new filtering interpreter that wraps another interpreter.
    pub fn new(
        inner: Box<dyn Interpreter>,
        compiler_filters: Vec<CompilerFilter>,
        source_filters: Vec<DirectoryFilter>,
    ) -> Self {
        Self {
            inner,
            compiler_filters,
            source_filters,
        }
    }

    /// Creates a filtering interpreter from configuration.
    pub fn from_config(
        inner: Box<dyn Interpreter>,
        compilers: &[config::Compiler],
        sources: &config::SourceFilter,
    ) -> Result<Self, ConfigurationError> {
        let compiler_filters = compilers
            .iter()
            .map(|c| CompilerFilter {
                path: c.path.clone(),
                ignore: c.ignore.clone(),
            })
            .collect();

        let source_filters = if sources.only_existing_files {
            sources
                .paths
                .iter()
                .filter_map(|filter| {
                    filter
                        .path
                        .canonicalize()
                        .ok()
                        .map(|canonical_path| DirectoryFilter {
                            path: canonical_path,
                            ignore: filter.ignore.clone(),
                        })
                })
                .collect()
        } else {
            sources.paths.clone()
        };

        Ok(Self::new(inner, compiler_filters, source_filters))
    }

    fn should_filter_compiler(&self, compiler_path: &PathBuf) -> Option<String> {
        for filter in &self.compiler_filters {
            if filter.path == *compiler_path {
                return match filter.ignore {
                    IgnoreOrConsider::Always => Some(format!(
                        "Compiler {} is configured to be ignored",
                        compiler_path.display()
                    )),
                    _ => None,
                };
            }
        }
        None
    }

    fn should_filter_sources(&self, cmd: &CompilerCommand) -> Option<String> {
        // Get all source files from the command
        let source_files: Vec<&String> = cmd
            .arguments
            .iter()
            .filter(|arg| arg.kind == ArgumentKind::Source)
            .flat_map(|arg| &arg.args)
            .collect();

        for source_file in source_files {
            let source_path = if PathBuf::from(source_file).is_absolute() {
                PathBuf::from(source_file)
            } else {
                cmd.working_dir.join(source_file)
            };

            for filter in &self.source_filters {
                if source_path.starts_with(&filter.path) {
                    return match filter.ignore {
                        Ignore::Always => Some(format!(
                            "Source file {} is in filtered directory {}",
                            source_file,
                            filter.path.display()
                        )),
                        Ignore::Never => None,
                    };
                }
            }
        }
        None
    }
}

impl Interpreter for FilteringInterpreter {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        // First, let the inner interpreter recognize the command
        let command = self.inner.recognize(execution)?;

        match command {
            Command::Compiler(compiler_cmd) => {
                // Check if the compiler should be filtered
                if let Some(reason) = self.should_filter_compiler(&compiler_cmd.executable) {
                    return Some(Command::Filtered(Box::leak(reason.into_boxed_str())));
                }

                // Check if any source files should be filtered
                if let Some(reason) = self.should_filter_sources(&compiler_cmd) {
                    return Some(Command::Filtered(Box::leak(reason.into_boxed_str())));
                }

                // No filtering applied, return the original command
                Some(Command::Compiler(compiler_cmd))
            }
            // Pass through other command types unchanged
            other => Some(other),
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error("IO error during configuration: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::command::{ArgumentGroup, CompilerCommand};
    use std::path::PathBuf;

    struct MockInterpreter {
        result: Option<Command>,
    }

    impl Interpreter for MockInterpreter {
        fn recognize(&self, _execution: &Execution) -> Option<Command> {
            self.result.clone()
        }
    }

    #[test]
    fn test_filter_compiler_always_ignored() {
        let compiler_filters = vec![CompilerFilter {
            path: PathBuf::from("/usr/bin/gcc"),
            ignore: IgnoreOrConsider::Always,
        }];

        let mock_cmd = CompilerCommand::new(
            PathBuf::from("/project"),
            PathBuf::from("/usr/bin/gcc"),
            vec![ArgumentGroup {
                args: vec!["main.c".to_string()],
                kind: ArgumentKind::Source,
            }],
        );

        let mock_interpreter = MockInterpreter {
            result: Some(Command::Compiler(mock_cmd)),
        };

        let filter =
            FilteringInterpreter::new(Box::new(mock_interpreter), compiler_filters, vec![]);

        let execution = Execution {
            executable: PathBuf::from("/usr/bin/gcc"),
            arguments: vec!["gcc".to_string(), "main.c".to_string()],
            working_dir: PathBuf::from("/project"),
            environment: std::collections::HashMap::new(),
        };

        let result = filter.recognize(&execution);
        assert!(matches!(result, Some(Command::Filtered(_))));
    }

    #[test]
    fn test_filter_source_directory_always_ignored() {
        let source_filters = vec![DirectoryFilter {
            path: PathBuf::from("/project/tests"),
            ignore: Ignore::Always,
        }];

        let mock_cmd = CompilerCommand::new(
            PathBuf::from("/project"),
            PathBuf::from("/usr/bin/gcc"),
            vec![ArgumentGroup {
                args: vec!["tests/test.c".to_string()],
                kind: ArgumentKind::Source,
            }],
        );

        let mock_interpreter = MockInterpreter {
            result: Some(Command::Compiler(mock_cmd)),
        };

        let filter = FilteringInterpreter::new(Box::new(mock_interpreter), vec![], source_filters);

        let execution = Execution {
            executable: PathBuf::from("/usr/bin/gcc"),
            arguments: vec!["gcc".to_string(), "tests/test.c".to_string()],
            working_dir: PathBuf::from("/project"),
            environment: std::collections::HashMap::new(),
        };

        let result = filter.recognize(&execution);
        assert!(matches!(result, Some(Command::Filtered(_))));
    }

    #[test]
    fn test_no_filtering_applied() {
        let mock_cmd = CompilerCommand::new(
            PathBuf::from("/project"),
            PathBuf::from("/usr/bin/gcc"),
            vec![ArgumentGroup {
                args: vec!["main.c".to_string()],
                kind: ArgumentKind::Source,
            }],
        );

        let mock_interpreter = MockInterpreter {
            result: Some(Command::Compiler(mock_cmd.clone())),
        };

        let filter = FilteringInterpreter::new(Box::new(mock_interpreter), vec![], vec![]);

        let execution = Execution {
            executable: PathBuf::from("/usr/bin/gcc"),
            arguments: vec!["gcc".to_string(), "main.c".to_string()],
            working_dir: PathBuf::from("/project"),
            environment: std::collections::HashMap::new(),
        };

        let result = filter.recognize(&execution);
        if let Some(Command::Compiler(result_cmd)) = result {
            assert_eq!(result_cmd, mock_cmd);
        } else {
            panic!("Expected Command::Compiler, got {:?}", result);
        }
    }

    #[test]
    fn test_pass_through_non_compiler_commands() {
        let mock_interpreter = MockInterpreter {
            result: Some(Command::Ignored("test reason")),
        };

        let filter = FilteringInterpreter::new(Box::new(mock_interpreter), vec![], vec![]);

        let execution = Execution {
            executable: PathBuf::from("/usr/bin/ls"),
            arguments: vec!["ls".to_string()],
            working_dir: PathBuf::from("/project"),
            environment: std::collections::HashMap::new(),
        };

        let result = filter.recognize(&execution);
        assert!(matches!(result, Some(Command::Ignored(_))));
    }
}
