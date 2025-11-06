// SPDX-License-Identifier: GPL-3.0-or-later

//! Filtering interpreter that wraps another interpreter to filter out compiler commands
//! based on compiler paths and source directories.

use super::InterpreterConfigError;
use crate::config;
use crate::config::PathResolver;

use crate::semantic::{ArgumentKind, Command, CompilerCommand, Execution, Interpreter};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// A wrapper interpreter that applies filtering to recognized compiler commands.
pub(super) struct FilteringInterpreter<T: Interpreter> {
    inner: T,
    filter: Filter,
}

impl<T: Interpreter> FilteringInterpreter<T> {
    /// Creates a filtering interpreter with the given filter.
    pub fn new(inner: T, filter: Filter) -> Self {
        Self { inner, filter }
    }
}

impl<T: Interpreter> Interpreter for FilteringInterpreter<T> {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        // First, let the inner interpreter recognize the command
        let command = self.inner.recognize(execution)?;

        // Apply filtering to the recognized command
        match command {
            Command::Compiler(compiler_cmd) => {
                // Apply filtering to the compiler command
                match self.filter.filter_command(&compiler_cmd) {
                    Ok(_) => Some(Command::Compiler(compiler_cmd)),
                    Err(reason) => Some(Command::Ignored(reason)),
                }
            }
            // Pass through other command types unchanged
            other => Some(other),
        }
    }
}

#[derive(Debug)]
pub(super) struct Filter {
    compiler_filters: HashMap<PathBuf, config::IgnoreOrConsider>,
    source_filters: Vec<config::DirectoryFilter>,
    only_existing_files: bool,
}

impl Filter {
    fn filter_command(&self, cmd: &CompilerCommand) -> Result<(), &'static str> {
        // Check if the compiler should be filtered
        if let Some(reason) = self.should_filter_compiler(&cmd.executable, &self.compiler_filters) {
            return Err(reason);
        }

        // Check if any source files should be filtered
        if let Some(reason) = self.should_filter_sources(cmd, &self.source_filters) {
            return Err(reason);
        }

        Ok(())
    }

    fn should_filter_compiler(
        &self,
        compiler_path: &PathBuf,
        compiler_filters: &HashMap<PathBuf, config::IgnoreOrConsider>,
    ) -> Option<&'static str> {
        if let Some(ignore) = compiler_filters.get(compiler_path) {
            match ignore {
                config::IgnoreOrConsider::Always => Some("Compiler is configured to be ignored"),
                _ => None,
            }
        } else {
            None
        }
    }

    fn should_filter_sources(
        &self,
        cmd: &CompilerCommand,
        source_filters: &[config::DirectoryFilter],
    ) -> Option<&'static str> {
        // Get all source files from the command using as_file method
        let path_updater: &dyn Fn(&std::path::Path) -> std::borrow::Cow<std::path::Path> =
            &|path: &std::path::Path| std::borrow::Cow::Borrowed(path);

        let source_arguments: Vec<_> = cmd
            .arguments
            .iter()
            .filter(|arg| arg.kind() == ArgumentKind::Source)
            .collect();

        // If no source files found, no filtering needed
        if source_arguments.is_empty() {
            return None;
        }

        let mut filtered_count = 0;
        let total_sources = source_arguments.len();

        for source_arg in source_arguments {
            if let Some(source_path) = source_arg.as_file(path_updater) {
                // Use multiple path formatters to check if source should be filtered
                let path_variants = self.get_path_variants(&source_path, &cmd.working_dir);

                let mut should_filter_this_source = false;
                for variant_path in path_variants {
                    for filter in source_filters {
                        // Normalize both paths for comparison to handle platform differences
                        let normalized_variant = self.normalize_path_for_comparison(&variant_path);
                        let normalized_filter = self.normalize_path_for_comparison(&filter.path);

                        if normalized_variant.starts_with(&normalized_filter) {
                            match filter.ignore {
                                config::Ignore::Always => {
                                    should_filter_this_source = true;
                                    break;
                                }
                                config::Ignore::Never => {
                                    // Never ignore takes precedence
                                    should_filter_this_source = false;
                                    break;
                                }
                            }
                        }
                    }
                    if should_filter_this_source {
                        break;
                    }
                }

                if should_filter_this_source {
                    filtered_count += 1;
                }
            }
        }

        // Handle case when there are multiple source files:
        // Only filter if ALL source files should be filtered
        if filtered_count > 0 && filtered_count == total_sources {
            Some("All source files are in filtered directories")
        } else {
            None
        }
    }

    /// Generate multiple path variants for filtering checks using different formatters
    fn get_path_variants(&self, source_path: &Path, working_dir: &Path) -> Vec<PathBuf> {
        let mut variants = Vec::new();

        // Build resolvers array, conditionally including Canonical based on config
        let mut resolvers = vec![
            PathResolver::AsIs,
            PathResolver::Absolute,
            PathResolver::Relative,
        ];

        // Only use canonical if only_existing_files is true (respects config)
        if self.only_existing_files {
            resolvers.push(PathResolver::Canonical);
        }

        // Generate variants using PathResolver resolve method
        for resolver in &resolvers {
            if let Ok(resolved_path) = resolver.resolve(working_dir, source_path) {
                variants.push(resolved_path);
            }
        }

        variants
    }

    /// Normalizes a path for cross-platform comparison by converting to absolute form
    /// and using consistent separators. Handles non-existing files by normalizing
    /// the parent directory and appending the filename.
    fn normalize_path_for_comparison(&self, path: &Path) -> PathBuf {
        // First try to make the path absolute
        let absolute_path = match std::path::absolute(path) {
            Ok(abs) => abs,
            Err(_) => path.to_path_buf(),
        };

        // Try to canonicalize if possible (for existing paths)
        match absolute_path.canonicalize() {
            Ok(canonical) => canonical,
            Err(_) => {
                // If canonicalize fails (e.g., path doesn't exist), try to canonicalize
                // the parent directory and append the filename
                if let Some(parent) = absolute_path.parent() {
                    if let Some(filename) = absolute_path.file_name() {
                        match parent.canonicalize() {
                            Ok(canonical_parent) => canonical_parent.join(filename),
                            Err(_) => absolute_path,
                        }
                    } else {
                        absolute_path
                    }
                } else {
                    absolute_path
                }
            }
        }
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
}

impl TryFrom<(&[config::Compiler], &config::SourceFilter)> for Filter {
    type Error = InterpreterConfigError;

    fn try_from(
        (compilers, sources): (&[config::Compiler], &config::SourceFilter),
    ) -> Result<Self, Self::Error> {
        // Validate compiler configuration
        Self::validate_compiler_configuration(compilers)?;

        let mut compiler_filters = HashMap::new();
        for c in compilers {
            compiler_filters.insert(c.path.clone(), c.ignore.clone());
        }

        // Validate source configuration
        let source_filters = Self::validate_source_configuration(sources)?;

        Ok(Self {
            compiler_filters,
            source_filters,
            only_existing_files: sources.only_existing_files,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Arguments, Compiler, DirectoryFilter, Ignore, IgnoreOrConsider, SourceFilter,
    };
    use crate::semantic::CompilerCommand;
    use crate::semantic::MockInterpreter;
    use std::path::PathBuf;

    #[test]
    fn test_filter_compiler_always_ignored() {
        let compilers = vec![Compiler {
            path: PathBuf::from("/usr/bin/gcc"),
            ignore: IgnoreOrConsider::Always,
            arguments: Arguments::default(),
        }];
        let sources = SourceFilter {
            only_existing_files: false,
            paths: vec![],
        };

        let mut mock_interpreter = MockInterpreter::new();
        mock_interpreter.expect_recognize().times(1).returning(|_| {
            let mock_cmd = CompilerCommand::from_strings(
                "/project",
                "/usr/bin/gcc",
                vec![(ArgumentKind::Source, vec!["main.c"])],
            );
            Some(Command::Compiler(mock_cmd))
        });

        let filter =
            Filter::try_from((compilers.as_slice(), &sources)).expect("Failed to create filter");
        let sut = FilteringInterpreter::new(mock_interpreter, filter);

        let execution = Execution::from_strings(
            "/usr/bin/gcc",
            vec!["gcc", "main.c"],
            "/project",
            HashMap::new(),
        );

        let result = sut.recognize(&execution);
        assert!(matches!(result, Some(Command::Ignored(_))));
    }

    #[test]
    fn test_filter_source_directory_always_ignored() {
        let compilers = vec![];
        let sources = SourceFilter {
            only_existing_files: false,
            paths: vec![DirectoryFilter {
                path: PathBuf::from("/project/tests"),
                ignore: Ignore::Always,
            }],
        };

        let mut mock_interpreter = MockInterpreter::new();
        mock_interpreter.expect_recognize().times(1).returning(|_| {
            let mock_cmd = CompilerCommand::from_strings(
                "/project",
                "/usr/bin/gcc",
                vec![(ArgumentKind::Source, vec!["tests/test_file.c"])],
            );
            Some(Command::Compiler(mock_cmd))
        });

        let filter =
            Filter::try_from((compilers.as_slice(), &sources)).expect("Failed to create filter");
        let sut = FilteringInterpreter::new(mock_interpreter, filter);

        let execution = Execution::from_strings(
            "/usr/bin/gcc",
            vec!["gcc", "tests/test_file.c"],
            "/project",
            HashMap::new(),
        );

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
        let result = Filter::validate_source_configuration(&config);
        assert!(
            matches!(result, Err(SourceFilterConfigurationError::DuplicateSourceInstruction(path)) if path == Path::new("/project/src"))
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
        let result = Filter::validate_source_configuration(&config);
        assert!(
            matches!(result, Err(SourceFilterConfigurationError::DuplicateDirectory(path)) if path == Path::new("/project/src"))
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
        let result = Filter::validate_source_configuration(&config);
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
            let result = Filter::validate_compiler_configuration(&config);
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
            let result = Filter::validate_compiler_configuration(&config);
            assert!(
                result.is_err(),
                "Expected invalid configuration to fail: {config:?}"
            );
        }
    }

    #[test]
    fn test_only_existing_files_config_behavior() {
        use std::fs;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Create an existing file
        let existing_file = temp_path.join("existing.c");
        fs::write(&existing_file, "test content").unwrap();

        let compilers = vec![];

        // Test with only_existing_files: true
        let sources_with_file_access = SourceFilter {
            only_existing_files: true,
            paths: vec![DirectoryFilter {
                path: temp_path.to_path_buf(),
                ignore: Ignore::Always,
            }],
        };

        // Test with only_existing_files: false
        let sources_without_file_access = SourceFilter {
            only_existing_files: false,
            paths: vec![DirectoryFilter {
                path: temp_path.to_path_buf(),
                ignore: Ignore::Always,
            }],
        };

        let filter_with_access =
            Filter::try_from((compilers.as_slice(), &sources_with_file_access))
                .expect("Failed to create filter with file access");
        let filter_without_access =
            Filter::try_from((compilers.as_slice(), &sources_without_file_access))
                .expect("Failed to create filter without file access");

        // Verify that only_existing_files config is stored correctly
        assert!(filter_with_access.only_existing_files);
        assert!(!filter_without_access.only_existing_files);

        // Create command with existing file
        let cmd_existing = CompilerCommand::from_strings(
            temp_path.to_str().unwrap(),
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source, vec!["existing.c"])],
        );

        // Both should filter since directory is filtered
        let result_with_access = filter_with_access
            .should_filter_sources(&cmd_existing, &filter_with_access.source_filters);
        let result_without_access = filter_without_access
            .should_filter_sources(&cmd_existing, &filter_without_access.source_filters);

        assert!(result_with_access.is_some());
        assert!(result_without_access.is_some());

        // Create command with non-existing file
        let cmd_nonexisting = CompilerCommand::from_strings(
            temp_path.to_str().unwrap(),
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source, vec!["nonexisting.c"])],
        );

        // Both should filter since directory is filtered regardless of file existence
        let result_with_access_nonexisting = filter_with_access
            .should_filter_sources(&cmd_nonexisting, &filter_with_access.source_filters);
        let result_without_access_nonexisting = filter_without_access
            .should_filter_sources(&cmd_nonexisting, &filter_without_access.source_filters);

        assert!(result_with_access_nonexisting.is_some());
        assert!(result_without_access_nonexisting.is_some());
    }

    #[test]
    fn test_cross_platform_path_normalization() {
        use std::fs;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Create an existing file
        let existing_file = temp_path.join("test.c");
        fs::write(&existing_file, "test content").unwrap();

        let compilers = vec![];
        let sources = SourceFilter {
            only_existing_files: true,
            paths: vec![DirectoryFilter {
                path: temp_path.to_path_buf(),
                ignore: Ignore::Always,
            }],
        };

        let filter =
            Filter::try_from((compilers.as_slice(), &sources)).expect("Failed to create filter");

        // Test with various path formats that might occur cross-platform
        let test_cases = vec![
            // Relative path
            "test.c", // Path with current directory reference
            "./test.c",
        ];

        for source_file in test_cases {
            let cmd = CompilerCommand::from_strings(
                temp_path.to_str().unwrap(),
                "/usr/bin/gcc",
                vec![(ArgumentKind::Source, vec![source_file])],
            );

            let result = filter.should_filter_sources(&cmd, &filter.source_filters);
            assert!(
                result.is_some(),
                "Failed to filter source file: {} in temp dir: {:?}",
                source_file,
                temp_path
            );
        }

        // Test with non-existing file to ensure it's also filtered
        let cmd_nonexisting = CompilerCommand::from_strings(
            temp_path.to_str().unwrap(),
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source, vec!["nonexisting.c"])],
        );

        let result_nonexisting =
            filter.should_filter_sources(&cmd_nonexisting, &filter.source_filters);
        assert!(
            result_nonexisting.is_some(),
            "Failed to filter non-existing source file in temp dir: {:?}",
            temp_path
        );
    }
}
