// SPDX-License-Identifier: GPL-3.0-or-later

use super::types::*;
use thiserror::Error;

/// Trait for validating configuration objects
pub trait Validator<T> {
    type Error: std::error::Error;

    fn validate(config: &T) -> Result<(), Self::Error>;
}

/// Validation errors for configuration
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Empty string value for field '{field}'")]
    EmptyString { field: String },
    #[error("Path does not exist: '{path}'")]
    PathNotFound { path: String },
    #[error("Duplicate {field} entry at: {idx}")]
    DuplicateEntry { field: &'static str, idx: usize },
    #[error("Path format error: {message}")]
    PathFormatError { message: &'static str },
    #[error("Multiple validation errors: {errors:?}")]
    Multiple { errors: Vec<ValidationError> },
}

/// Combinator for collecting and handling validation errors
#[derive(Default)]
struct ValidationCollector {
    errors: Vec<ValidationError>,
}

impl ValidationCollector {
    fn new() -> Self {
        Self { errors: Vec::new() }
    }

    fn add(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    fn add_result(&mut self, result: Result<(), ValidationError>) {
        if let Err(error) = result {
            match error {
                ValidationError::Multiple { errors } => {
                    self.errors.extend(errors);
                }
                single_error => self.errors.push(single_error),
            }
        }
    }

    fn finish(self) -> Result<(), ValidationError> {
        if self.errors.is_empty() {
            Ok(())
        } else if self.errors.len() == 1 {
            Err(self.errors.into_iter().next().unwrap())
        } else {
            Err(ValidationError::Multiple { errors: self.errors })
        }
    }
}

impl Validator<Main> for Main {
    type Error = ValidationError;

    fn validate(config: &Main) -> Result<(), Self::Error> {
        let mut collector = ValidationCollector::new();

        // Validate each compiler configuration
        for compiler in config.compilers.iter() {
            collector.add_result(Compiler::validate(compiler));
        }

        // Check for duplicate compiler paths
        let mut seen_paths = std::collections::HashSet::new();
        for (idx, compiler) in config.compilers.iter().enumerate() {
            if !seen_paths.insert(&compiler.path) {
                collector.add(ValidationError::DuplicateEntry { field: "compiler", idx });
            }
        }

        // Validate source filter configuration
        collector.add_result(SourceFilter::validate(&config.sources));

        // Validate duplicate filter configuration
        collector.add_result(DuplicateFilter::validate(&config.duplicates));

        // Validate path format configuration
        collector.add_result(PathFormat::validate(&config.format.paths));

        collector.finish()
    }
}

impl Validator<Compiler> for Compiler {
    type Error = ValidationError;

    fn validate(config: &Compiler) -> Result<(), Self::Error> {
        let mut collector = ValidationCollector::new();

        // Check if compiler path exists
        if !config.path.exists() {
            collector.add(ValidationError::PathNotFound { path: config.path.display().to_string() });
        }

        collector.finish()
    }
}

impl Validator<SourceFilter> for SourceFilter {
    type Error = ValidationError;

    fn validate(config: &SourceFilter) -> Result<(), Self::Error> {
        // Validate that directory rule paths are not empty
        let mut collector = ValidationCollector::new();

        for (idx, rule) in config.directories.iter().enumerate() {
            if rule.path.as_os_str().is_empty() {
                collector.add(ValidationError::EmptyString {
                    field: format!("sources.directories[{}].path", idx),
                });
            }
        }

        collector.finish()
    }
}

impl Validator<DuplicateFilter> for DuplicateFilter {
    type Error = ValidationError;

    fn validate(config: &DuplicateFilter) -> Result<(), Self::Error> {
        // Check for duplicate OutputFields in match_on
        let mut collector = ValidationCollector::new();
        let mut seen_fields = std::collections::HashSet::new();

        for (idx, field) in config.match_on.iter().enumerate() {
            if !seen_fields.insert(field) {
                collector.add(ValidationError::DuplicateEntry { field: "duplicates.match_on", idx });
            }
        }

        collector.finish()
    }
}

impl Validator<PathFormat> for PathFormat {
    type Error = ValidationError;

    /// Validates the path format configuration according to the rules:
    /// - When directory is relative, file must be relative too
    /// - When directory is canonical, file can't be absolute
    /// - When directory is absolute, file can't be canonical
    fn validate(config: &PathFormat) -> Result<(), Self::Error> {
        use PathResolver::*;

        match (&config.directory, &config.file) {
            (Relative, Absolute | Canonical) => Err(ValidationError::PathFormatError {
                message: "When directory is relative, file must be relative too",
            }),
            (Canonical, Absolute) => Err(ValidationError::PathFormatError {
                message: "When directory is canonical, file can't be absolute",
            }),
            (Absolute, Canonical) => Err(ValidationError::PathFormatError {
                message: "When directory is absolute, file can't be canonical",
            }),
            (AsIs, Absolute | Relative | Canonical) => Err(ValidationError::PathFormatError {
                message: "When directory as-is, file should be the same",
            }),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_validate_compiler_invalid_path() {
        let config = Compiler {
            path: PathBuf::from("/nonexistent/compiler"),
            as_: Some(CompilerType::Gcc),
            ignore: false,
        };

        let result = Compiler::validate(&config);
        assert!(result.is_err());

        match result.unwrap_err() {
            ValidationError::PathNotFound { .. } => {
                // Expected - path doesn't exist
            }
            _ => panic!("Expected PathNotFound validation error"),
        }
    }

    #[test]
    fn test_validate_source_filter_empty_paths() {
        let config = SourceFilter {
            directories: vec![
                DirectoryRule { path: PathBuf::from("valid/path"), action: DirectoryAction::Include },
                DirectoryRule { path: PathBuf::from(""), action: DirectoryAction::Exclude },
            ],
        };

        let result = SourceFilter::validate(&config);
        assert!(result.is_err());

        match result.unwrap_err() {
            ValidationError::EmptyString { field } => {
                assert_eq!(field, "sources.directories[1].path");
            }
            _ => panic!("Expected empty string validation error"),
        }
    }

    #[test]
    fn test_validate_source_filter_multiple_empty_paths() {
        let config = SourceFilter {
            directories: vec![
                DirectoryRule { path: PathBuf::from(""), action: DirectoryAction::Include },
                DirectoryRule { path: PathBuf::from("valid/path"), action: DirectoryAction::Exclude },
                DirectoryRule { path: PathBuf::from(""), action: DirectoryAction::Include },
            ],
        };

        let result = SourceFilter::validate(&config);
        assert!(result.is_err());

        match result.unwrap_err() {
            ValidationError::Multiple { errors } => {
                assert_eq!(errors.len(), 2);
            }
            _ => panic!("Expected multiple validation errors"),
        }
    }

    #[test]
    fn test_validate_source_filter_valid_config() {
        let config = SourceFilter {
            directories: vec![
                DirectoryRule { path: PathBuf::from("/usr/include"), action: DirectoryAction::Exclude },
                DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Include },
            ],
        };

        let result = SourceFilter::validate(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_source_filter_empty_directories() {
        let config = SourceFilter { directories: vec![] };

        let result = SourceFilter::validate(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_duplicate_filter_no_duplicates() {
        let config = DuplicateFilter {
            match_on: vec![OutputFields::File, OutputFields::Arguments, OutputFields::Directory],
        };

        let result = DuplicateFilter::validate(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_duplicate_filter_with_duplicates() {
        let config = DuplicateFilter {
            match_on: vec![OutputFields::File, OutputFields::Arguments, OutputFields::File],
        };

        let result = DuplicateFilter::validate(&config);
        assert!(result.is_err());

        match result.unwrap_err() {
            ValidationError::DuplicateEntry { field, idx } => {
                assert_eq!(field, "duplicates.match_on");
                assert_eq!(idx, 2);
            }
            _ => panic!("Expected DuplicateEntry validation error"),
        }
    }

    #[test]
    fn test_validate_duplicate_filter_multiple_duplicates() {
        let config = DuplicateFilter {
            match_on: vec![
                OutputFields::File,
                OutputFields::Arguments,
                OutputFields::File,
                OutputFields::Directory,
                OutputFields::Arguments,
            ],
        };

        let result = DuplicateFilter::validate(&config);
        assert!(result.is_err());

        match result.unwrap_err() {
            ValidationError::Multiple { errors } => {
                assert_eq!(errors.len(), 2);
            }
            _ => panic!("Expected multiple validation errors"),
        }
    }

    #[test]
    fn test_validate_duplicate_filter_empty_match_on() {
        let config = DuplicateFilter { match_on: vec![] };

        let result = DuplicateFilter::validate(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_format_success() {
        let valid_configs = vec![
            PathFormat { directory: PathResolver::AsIs, file: PathResolver::AsIs },
            PathFormat { directory: PathResolver::Relative, file: PathResolver::Relative },
            PathFormat { directory: PathResolver::Canonical, file: PathResolver::Relative },
            PathFormat { directory: PathResolver::Absolute, file: PathResolver::Relative },
            PathFormat { directory: PathResolver::Absolute, file: PathResolver::Absolute },
        ];

        for config in valid_configs {
            assert!(PathFormat::validate(&config).is_ok(), "Config should be valid: {:?}", config);
        }
    }

    #[test]
    fn test_validate_path_format_failures() {
        let invalid_configs = vec![
            (
                PathFormat { directory: PathResolver::Relative, file: PathResolver::Absolute },
                "When directory is relative, file must be relative too",
            ),
            (
                PathFormat { directory: PathResolver::Relative, file: PathResolver::Canonical },
                "When directory is relative, file must be relative too",
            ),
            (
                PathFormat { directory: PathResolver::Canonical, file: PathResolver::Absolute },
                "When directory is canonical, file can't be absolute",
            ),
            (
                PathFormat { directory: PathResolver::Absolute, file: PathResolver::Canonical },
                "When directory is absolute, file can't be canonical",
            ),
            (
                PathFormat { directory: PathResolver::AsIs, file: PathResolver::Canonical },
                "When directory as-is, file should be the same",
            ),
        ];

        for (config, expected_error) in invalid_configs {
            let result = PathFormat::validate(&config);
            assert!(result.is_err(), "Config should be invalid: {:?}", config);
            if let Err(ValidationError::PathFormatError { message }) = result {
                assert_eq!(message, expected_error);
            } else {
                panic!("Expected PathFormatError, got: {:?}", result);
            }
        }
    }
}
