// SPDX-License-Identifier: GPL-3.0-or-later

use super::types::*;
use thiserror::Error;

/// Trait for validating configuration objects
pub(super) trait Validator<T> {
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

/// Collapse a list of errors into a single `Result`.
///
/// Returns `Ok` for an empty list, the lone error directly for a singleton,
/// and wraps two-or-more in `ValidationError::Multiple`.
fn collapse(mut errors: Vec<ValidationError>) -> Result<(), ValidationError> {
    match errors.len() {
        0 => Ok(()),
        1 => Err(errors.pop().unwrap()),
        _ => Err(ValidationError::Multiple { errors }),
    }
}

/// Append a sub-validator's outcome to `target`, flattening `Multiple` so the
/// final error list stays flat (avoids ugly nested `Multiple { [Multiple ...] }`).
fn extend_with(target: &mut Vec<ValidationError>, result: Result<(), ValidationError>) {
    match result {
        Ok(()) => {}
        Err(ValidationError::Multiple { errors }) => target.extend(errors),
        Err(single) => target.push(single),
    }
}

impl Validator<Main> for Main {
    type Error = ValidationError;

    fn validate(config: &Main) -> Result<(), Self::Error> {
        let mut errors = Vec::new();

        // Validate each compiler configuration
        for compiler in config.compilers.iter() {
            if let Err(e) = Compiler::validate(compiler) {
                errors.push(e);
            }
        }

        // Check for duplicate compiler paths
        let mut seen_paths = std::collections::HashSet::new();
        for (idx, compiler) in config.compilers.iter().enumerate() {
            if !seen_paths.insert(&compiler.path) {
                errors.push(ValidationError::DuplicateEntry { field: "compiler", idx });
            }
        }

        extend_with(&mut errors, SourceFilter::validate(&config.sources));
        extend_with(&mut errors, DuplicateFilter::validate(&config.duplicates));
        extend_with(&mut errors, PathFormat::validate(&config.format.paths));

        collapse(errors)
    }
}

impl Validator<Compiler> for Compiler {
    type Error = ValidationError;

    fn validate(config: &Compiler) -> Result<(), Self::Error> {
        if config.path.exists() {
            Ok(())
        } else {
            Err(ValidationError::PathNotFound { path: config.path.display().to_string() })
        }
    }
}

impl Validator<SourceFilter> for SourceFilter {
    type Error = ValidationError;

    fn validate(config: &SourceFilter) -> Result<(), Self::Error> {
        let errors = config
            .directories
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.path.as_os_str().is_empty())
            .map(|(idx, _)| ValidationError::EmptyString {
                field: format!("sources.directories[{}].path", idx),
            })
            .collect();
        collapse(errors)
    }
}

impl Validator<DuplicateFilter> for DuplicateFilter {
    type Error = ValidationError;

    fn validate(config: &DuplicateFilter) -> Result<(), Self::Error> {
        // The closure mutates `seen_fields`, which is intentional: `collect` drives
        // the iterator to completion in order, so each entry is visited exactly once.
        let mut seen_fields = std::collections::HashSet::new();
        let errors = config
            .match_on
            .iter()
            .enumerate()
            .filter(|(_, field)| !seen_fields.insert(*field))
            .map(|(idx, _)| ValidationError::DuplicateEntry { field: "duplicates.match_on", idx })
            .collect();
        collapse(errors)
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
