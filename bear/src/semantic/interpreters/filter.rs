// SPDX-License-Identifier: GPL-3.0-or-later

//! Filtering interpreter that wraps another interpreter to filter out compiler commands
//! based on compiler paths and source directories.

use crate::config;
use crate::semantic::command::{ArgumentKind, CompilerCommand};
use crate::semantic::{Command, Execution, Interpreter};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// A wrapper interpreter that applies filtering to recognized compiler commands.
pub struct FilteringInterpreter {
    inner: Box<dyn Interpreter>,
    compiler_filters: HashMap<PathBuf, config::IgnoreOrConsider>,
    source_filters: Vec<config::DirectoryFilter>,
}

impl FilteringInterpreter {
    /// Creates a new filtering interpreter that wraps another interpreter.
    pub fn new(
        inner: Box<dyn Interpreter>,
        compiler_filters: HashMap<PathBuf, config::IgnoreOrConsider>,
        source_filters: Vec<config::DirectoryFilter>,
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
        // Validate compiler configuration
        Self::validate_compiler_configuration(compilers)?;

        let mut compiler_filters = HashMap::new();
        for c in compilers {
            compiler_filters.insert(c.path.clone(), c.ignore.clone());
        }

        // Validate source configuration
        let source_filters = Self::validate_source_configuration(sources)?;

        Ok(Self::new(inner, compiler_filters, source_filters))
    }

    /// Validates the compiler configuration.
    fn validate_compiler_configuration(
        compilers: &[config::Compiler],
    ) -> Result<(), CompilerFilterConfigurationError> {
        use config::{Arguments, IgnoreOrConsider};

        // Group the compilers by path
        let mut compilers_by_path: HashMap<PathBuf, Vec<&config::Compiler>> = HashMap::new();
        for compiler in compilers {
            compilers_by_path
                .entry(compiler.path.clone())
                .or_default()
                .push(compiler);
        }

        // Validate the configuration for each compiler path
        for (path, path_compilers) in compilers_by_path {
            let mut has_always = false;
            let mut has_conditional = false;
            let mut has_never = false;

            for compiler in path_compilers {
                match compiler.ignore {
                    // Problems with the order of the configuration
                    IgnoreOrConsider::Conditional if has_conditional => {
                        return Err(CompilerFilterConfigurationError::MultipleConditional(path));
                    }
                    IgnoreOrConsider::Always if has_always => {
                        return Err(CompilerFilterConfigurationError::MultipleAlways(path));
                    }
                    IgnoreOrConsider::Never if has_never => {
                        return Err(CompilerFilterConfigurationError::MultipleNever(path));
                    }
                    IgnoreOrConsider::Always | IgnoreOrConsider::Never if has_conditional => {
                        return Err(CompilerFilterConfigurationError::AfterConditional(path));
                    }
                    IgnoreOrConsider::Always | IgnoreOrConsider::Conditional if has_never => {
                        return Err(CompilerFilterConfigurationError::AfterNever(path));
                    }
                    IgnoreOrConsider::Never | IgnoreOrConsider::Conditional if has_always => {
                        return Err(CompilerFilterConfigurationError::AfterAlways(path));
                    }
                    // Problems with the arguments
                    IgnoreOrConsider::Always if compiler.arguments != Arguments::default() => {
                        return Err(CompilerFilterConfigurationError::AlwaysWithArguments(path));
                    }
                    IgnoreOrConsider::Conditional if compiler.arguments.match_.is_empty() => {
                        return Err(CompilerFilterConfigurationError::ConditionalWithoutMatch(
                            path,
                        ));
                    }
                    IgnoreOrConsider::Never if !compiler.arguments.match_.is_empty() => {
                        return Err(CompilerFilterConfigurationError::NeverWithArguments(path));
                    }
                    // Update the flags, no problems found
                    IgnoreOrConsider::Conditional => {
                        has_conditional = true;
                    }
                    IgnoreOrConsider::Always => {
                        has_always = true;
                    }
                    IgnoreOrConsider::Never => {
                        has_never = true;
                    }
                }
            }
        }

        Ok(())
    }

    /// Normalizes the source filter paths (canonicalizes if needed).
    fn normalize_source_filter_paths(
        sources: &config::SourceFilter,
    ) -> Result<Vec<config::DirectoryFilter>, SourceFilterConfigurationError> {
        if sources.only_existing_files {
            let mut result = Vec::new();
            for filter in &sources.paths {
                match filter.path.canonicalize() {
                    Ok(p) => result.push(config::DirectoryFilter {
                        path: p,
                        ignore: filter.ignore.clone(),
                    }),
                    Err(e) => return Err(SourceFilterConfigurationError::Canonicalization(e)),
                }
            }
            Ok(result)
        } else {
            Ok(sources.paths.clone())
        }
    }

    /// Normalizes and validates source directory configuration and returns the validated filters.
    fn validate_source_configuration(
        sources: &config::SourceFilter,
    ) -> Result<Vec<config::DirectoryFilter>, SourceFilterConfigurationError> {
        let filters = Self::normalize_source_filter_paths(sources)?;

        let mut verified: Vec<config::DirectoryFilter> = vec![];
        for filter in filters {
            if let Some(duplicate) = verified.iter().find(|f| f.path == filter.path) {
                let path = filter.path.clone();
                return if duplicate.ignore == filter.ignore {
                    Err(SourceFilterConfigurationError::DuplicateDirectory(path))
                } else {
                    Err(SourceFilterConfigurationError::DuplicateSourceInstruction(
                        path,
                    ))
                };
            }
            verified.push(filter.clone());
        }
        Ok(verified)
    }

    fn should_filter_compiler(&self, compiler_path: &PathBuf) -> Option<String> {
        if let Some(ignore) = self.compiler_filters.get(compiler_path) {
            match ignore {
                config::IgnoreOrConsider::Always => Some(format!(
                    "Compiler {} is configured to be ignored",
                    compiler_path.display()
                )),
                _ => None,
            }
        } else {
            None
        }
    }

    fn should_filter_sources(&self, cmd: &CompilerCommand) -> Option<String> {
        // Get all source files from the command
        let source_files: Vec<&String> = cmd
            .arguments
            .iter()
            .filter(|arg| arg.kind == ArgumentKind::Source)
            .flat_map(|arg| &arg.args)
            .collect();

        // TODO: handle cases when there are multiple source files, but filtering
        //       keeps at least one source file in the result.
        for source_file in source_files {
            // FIXME: this is not needed if the command is already using absolute paths
            let source_path = if PathBuf::from(source_file).is_absolute() {
                PathBuf::from(source_file)
            } else {
                cmd.working_dir.join(source_file)
            };

            for filter in &self.source_filters {
                if source_path.starts_with(&filter.path) {
                    return match filter.ignore {
                        config::Ignore::Always => Some(format!(
                            "Source file {} is in filtered directory {}",
                            source_file,
                            filter.path.display()
                        )),
                        config::Ignore::Never => None,
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
                    return Some(Command::Filtered(reason));
                }

                // Check if any source files should be filtered
                if let Some(reason) = self.should_filter_sources(&compiler_cmd) {
                    return Some(Command::Filtered(reason));
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
pub enum CompilerFilterConfigurationError {
    #[error("'Never' or 'Conditional' can't be used after 'Always' for path {0:?}")]
    AfterAlways(PathBuf),
    #[error("'Never' can't be used after 'Conditional' for path {0:?}")]
    AfterConditional(PathBuf),
    #[error("'Always' or 'Conditional' can't be used after 'Never' for path {0:?}")]
    AfterNever(PathBuf),
    #[error("'Always' can't be used multiple times for path {0:?}")]
    MultipleAlways(PathBuf),
    #[error("'Conditional' can't be used multiple times for path {0:?}")]
    MultipleConditional(PathBuf),
    #[error("'Never' can't be used multiple times for path {0:?}")]
    MultipleNever(PathBuf),
    #[error("'Always' can't be used with arguments for path {0:?}")]
    AlwaysWithArguments(PathBuf),
    #[error("'Conditional' can't be used without arguments for path {0:?}")]
    ConditionalWithoutMatch(PathBuf),
    #[error("'Never' can't be used with arguments for path {0:?}")]
    NeverWithArguments(PathBuf),
}

#[derive(Debug, Error)]
pub enum SourceFilterConfigurationError {
    #[error("Duplicate directory: {0}")]
    DuplicateDirectory(PathBuf),
    #[error("Same directory to include and exclude: {0}")]
    DuplicateSourceInstruction(PathBuf),
    #[error("Canonicalization failed: {0}")]
    Canonicalization(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error("Compiler filter configuration error: {0}")]
    CompilerFilter(#[from] CompilerFilterConfigurationError),
    #[error("Source filter configuration error: {0}")]
    SourceFilter(#[from] SourceFilterConfigurationError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Arguments, Compiler, DirectoryFilter, Ignore, IgnoreOrConsider, SourceFilter,
    };
    use crate::semantic::command::CompilerCommand;
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
        let mut compiler_filters = HashMap::new();
        compiler_filters.insert(
            PathBuf::from("/usr/bin/gcc"),
            config::IgnoreOrConsider::Always,
        );

        let mock_cmd = CompilerCommand::from_strings(
            "/project",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source, vec!["main.c"])],
        );

        let mock_interpreter = MockInterpreter {
            result: Some(Command::Compiler(mock_cmd)),
        };

        let sut = FilteringInterpreter::new(Box::new(mock_interpreter), compiler_filters, vec![]);

        let execution = Execution::from_strings(
            "/usr/bin/gcc",
            vec!["gcc", "main.c"],
            "/project",
            HashMap::new(),
        );

        let result = sut.recognize(&execution);
        assert!(matches!(result, Some(Command::Filtered(_))));
    }

    #[test]
    fn test_filter_source_directory_always_ignored() {
        let source_filters = vec![config::DirectoryFilter {
            path: PathBuf::from("/project/tests"),
            ignore: config::Ignore::Always,
        }];

        let mock_cmd = CompilerCommand::from_strings(
            "/project",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source, vec!["tests/test_file.c"])],
        );

        let mock_interpreter = MockInterpreter {
            result: Some(Command::Compiler(mock_cmd)),
        };

        let sut =
            FilteringInterpreter::new(Box::new(mock_interpreter), HashMap::new(), source_filters);

        let execution = Execution::from_strings(
            "/usr/bin/gcc",
            vec!["gcc", "tests/test_file.c"],
            "/project",
            HashMap::new(),
        );

        let result = sut.recognize(&execution);
        assert!(matches!(result, Some(Command::Filtered(_))));
    }

    #[test]
    fn test_no_filtering_applied() {
        let mock_cmd = CompilerCommand::from_strings(
            "/project",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source, vec!["main.c"])],
        );

        let mock_interpreter = MockInterpreter {
            result: Some(Command::Compiler(mock_cmd.clone())),
        };

        let sut = FilteringInterpreter::new(Box::new(mock_interpreter), HashMap::new(), vec![]);

        let execution = Execution::from_strings(
            "/usr/bin/gcc",
            vec!["gcc", "main.c"],
            "/project",
            HashMap::new(),
        );

        let result = sut.recognize(&execution);
        if let Some(Command::Compiler(result_cmd)) = result {
            assert_eq!(result_cmd, mock_cmd);
        } else {
            panic!("Expected Command::Compiler, got {result:?}");
        }
    }

    #[test]
    fn test_pass_through_non_compiler_commands() {
        let mock_interpreter = MockInterpreter {
            result: Some(Command::Ignored("test reason")),
        };

        let sut = FilteringInterpreter::new(Box::new(mock_interpreter), HashMap::new(), vec![]);

        let execution =
            Execution::from_strings("/usr/bin/ls", vec!["ls"], "/project", HashMap::new());

        let result = sut.recognize(&execution);
        assert!(matches!(result, Some(Command::Ignored(_))));
    }

    #[test]
    fn test_source_filter_duplicate_instruction() {
        let config = SourceFilter {
            only_existing_files: false,
            paths: vec![
                DirectoryFilter {
                    path: PathBuf::from("/project/src"),
                    ignore: Ignore::Always,
                },
                DirectoryFilter {
                    path: PathBuf::from("/project/test"),
                    ignore: Ignore::Always,
                },
                DirectoryFilter {
                    path: PathBuf::from("/project/src"),
                    ignore: Ignore::Never,
                },
            ],
        };
        let result = FilteringInterpreter::validate_source_configuration(&config);
        assert!(
            matches!(result, Err(SourceFilterConfigurationError::DuplicateSourceInstruction(path)) if path == PathBuf::from("/project/src"))
        );
    }

    #[test]
    fn test_source_filter_duplicate_entry() {
        let config = SourceFilter {
            only_existing_files: false,
            paths: vec![
                DirectoryFilter {
                    path: PathBuf::from("/project/src"),
                    ignore: Ignore::Always,
                },
                DirectoryFilter {
                    path: PathBuf::from("/project/test"),
                    ignore: Ignore::Never,
                },
                DirectoryFilter {
                    path: PathBuf::from("/project/src"),
                    ignore: Ignore::Always,
                },
            ],
        };
        let result = FilteringInterpreter::validate_source_configuration(&config);
        assert!(
            matches!(result, Err(SourceFilterConfigurationError::DuplicateDirectory(path)) if path == PathBuf::from("/project/src"))
        );
    }

    #[test]
    fn test_source_filter_valid_config() {
        let config = SourceFilter {
            only_existing_files: false,
            paths: vec![
                DirectoryFilter {
                    path: PathBuf::from("/project/src"),
                    ignore: Ignore::Never,
                },
                DirectoryFilter {
                    path: PathBuf::from("/project/tests"),
                    ignore: Ignore::Always,
                },
            ],
        };
        let result = FilteringInterpreter::validate_source_configuration(&config);
        assert!(result.is_ok());
        let filters = result.unwrap();
        assert_eq!(filters.len(), 2);
        assert_eq!(filters[0].path, PathBuf::from("/project/src"));
        assert_eq!(filters[0].ignore, Ignore::Never);
        assert_eq!(filters[1].path, PathBuf::from("/project/tests"));
        assert_eq!(filters[1].ignore, Ignore::Always);
    }

    #[test]
    fn test_compiler_filter_valid_configs() {
        let valid_configs = vec![
            vec![Compiler {
                path: PathBuf::from("/usr/bin/gcc"),
                ignore: IgnoreOrConsider::Never,
                arguments: Arguments::default(),
            }],
            vec![Compiler {
                path: PathBuf::from("/usr/bin/clang"),
                ignore: IgnoreOrConsider::Never,
                arguments: Arguments {
                    add: vec!["-Wall".to_string()],
                    remove: vec!["-O2".to_string()],
                    ..Default::default()
                },
            }],
            vec![Compiler {
                path: PathBuf::from("/usr/bin/gcc"),
                ignore: IgnoreOrConsider::Always,
                arguments: Arguments::default(),
            }],
            vec![Compiler {
                path: PathBuf::from("/usr/bin/clang"),
                ignore: IgnoreOrConsider::Conditional,
                arguments: Arguments {
                    match_: vec!["-DDEBUG".to_string()],
                    ..Default::default()
                },
            }],
            vec![Compiler {
                path: PathBuf::from("/usr/bin/clang"),
                ignore: IgnoreOrConsider::Conditional,
                arguments: Arguments {
                    match_: vec!["-DDEBUG".to_string()],
                    add: vec!["-Wall".to_string()],
                    remove: vec!["-O2".to_string()],
                },
            }],
        ];
        for config in valid_configs {
            let result = FilteringInterpreter::validate_compiler_configuration(&config);
            assert!(
                result.is_ok(),
                "Expected valid configuration to pass: {config:?}, got {result:?}"
            );
        }
    }

    #[test]
    fn test_compiler_filter_invalid_configs() {
        let invalid_configs = vec![
            // Multiple "Always" for the same path
            vec![
                Compiler {
                    path: PathBuf::from("/usr/bin/gcc"),
                    ignore: IgnoreOrConsider::Always,
                    arguments: Arguments::default(),
                },
                Compiler {
                    path: PathBuf::from("/usr/bin/gcc"),
                    ignore: IgnoreOrConsider::Always,
                    arguments: Arguments::default(),
                },
            ],
            // "Always" after "Never"
            vec![
                Compiler {
                    path: PathBuf::from("/usr/bin/gcc"),
                    ignore: IgnoreOrConsider::Never,
                    arguments: Arguments::default(),
                },
                Compiler {
                    path: PathBuf::from("/usr/bin/gcc"),
                    ignore: IgnoreOrConsider::Always,
                    arguments: Arguments::default(),
                },
            ],
            // "Never" after "Conditional"
            vec![
                Compiler {
                    path: PathBuf::from("/usr/bin/gcc"),
                    ignore: IgnoreOrConsider::Conditional,
                    arguments: Arguments {
                        match_: vec!["-O2".to_string()],
                        ..Default::default()
                    },
                },
                Compiler {
                    path: PathBuf::from("/usr/bin/gcc"),
                    ignore: IgnoreOrConsider::Never,
                    arguments: Arguments::default(),
                },
            ],
            // "Always" with arguments
            vec![Compiler {
                path: PathBuf::from("/usr/bin/gcc"),
                ignore: IgnoreOrConsider::Always,
                arguments: Arguments {
                    add: vec!["-Wall".to_string()],
                    ..Default::default()
                },
            }],
            // "Conditional" without match arguments
            vec![Compiler {
                path: PathBuf::from("/usr/bin/gcc"),
                ignore: IgnoreOrConsider::Conditional,
                arguments: Arguments::default(),
            }],
            // "Never" with match arguments
            vec![Compiler {
                path: PathBuf::from("/usr/bin/gcc"),
                ignore: IgnoreOrConsider::Never,
                arguments: Arguments {
                    match_: vec!["-O2".to_string()],
                    ..Default::default()
                },
            }],
        ];
        for config in invalid_configs {
            let result = FilteringInterpreter::validate_compiler_configuration(&config);
            assert!(
                result.is_err(),
                "Expected invalid configuration to fail: {config:?}"
            );
        }
    }
}
