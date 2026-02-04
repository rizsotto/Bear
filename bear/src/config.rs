// SPDX-License-Identifier: GPL-3.0-or-later

//! This module defines the configuration of the application.
//!
//! The configuration is either loaded from a file or used with default
//! values, which are defined in the code. The configuration exposes the main
//! logical steps that the application will follow.
//!
//! The configuration file syntax is based on the YAML format.
//! The default configuration file name is `bear.yml`.
//!
//! The configuration file location is searched in the following order:
//! 1. The current working directory
//! 2. The local configuration directory of the user
//! 3. The configuration directory of the user
//! 4. The local configuration directory of the application
//! 5. The configuration directory of the application
//!
//! ```yaml
//! schema: 4.0
//!
//! intercept:
//!   mode: wrapper
//!
//! compilers:
//!   - path: /usr/local/bin/cc
//!     as: gcc
//!   - path: /usr/bin/cc
//!     ignore: true
//!   - path: /usr/bin/clang++
//!
//! sources:
//!   directories:
//!     - path: "/opt/project/sources"
//!       action: include
//!     - path: "/opt/project/tests"
//!       action: exclude
//!
//! duplicates:
//!   match_on: [file, directory]
//!
//! format:
//!   paths:
//!     directory: canonical
//!     file: canonical
//!   entries:
//!     use_array_format: true
//!     include_output_field: true
//! ```
//!
//! ```yaml
//! schema: 4.0
//!
//! intercept:
//!   mode: preload
//!
//! format:
//!   paths:
//!     directory: as-is
//!     file: as-is
//!   entries:
//!     use_array_format: true
//!     include_output_field: true
//! ```

// Re-Export the types and the loader module content.
pub use loader::{ConfigError, Loader};
pub use types::*;
pub use validation::Validator;

mod types {
    use serde::Deserialize;
    use std::fmt;
    use std::path::PathBuf;

    /// Represents the application configuration with flattened structure.
    #[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
    pub struct Main {
        #[serde(deserialize_with = "validate_schema_version")]
        pub schema: String,
        #[serde(default)]
        pub intercept: Intercept,
        #[serde(default)]
        pub compilers: Vec<Compiler>,
        #[serde(default)]
        pub sources: SourceFilter,
        #[serde(default)]
        pub duplicates: DuplicateFilter,
        #[serde(default)]
        pub format: Format,
    }

    impl Default for Main {
        fn default() -> Self {
            Self {
                schema: String::from(SUPPORTED_SCHEMA_VERSION),
                intercept: Intercept::default(),
                compilers: vec![],
                sources: SourceFilter::default(),
                duplicates: DuplicateFilter::default(),
                format: Format::default(),
            }
        }
    }

    impl fmt::Display for Main {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            writeln!(f, "Configuration:")?;
            match serde_yml::to_string(self) {
                Ok(yaml_string) => {
                    for line in yaml_string.lines() {
                        writeln!(f, "{}", line)?;
                    }
                    Ok(())
                }
                Err(_) => {
                    panic!("configuration can't be serialized")
                }
            }
        }
    }

    /// Simplified intercept configuration with mode.
    #[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "mode")]
    pub enum Intercept {
        #[serde(rename = "wrapper")]
        Wrapper {
            #[serde(default = "default_wrapper_executable")]
            path: PathBuf,
        },
        #[serde(rename = "preload")]
        Preload {
            #[serde(default = "default_preload_library")]
            path: PathBuf,
        },
    }

    /// The default intercept mode is varying based on the target operating system.
    impl Default for Intercept {
        #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "windows")))]
        fn default() -> Self {
            Intercept::Preload { path: default_preload_library() }
        }

        #[cfg(any(target_os = "macos", target_os = "ios", target_os = "windows"))]
        fn default() -> Self {
            Intercept::Wrapper { path: default_wrapper_executable() }
        }
    }

    /// Represents compiler configuration matching the YAML format.
    #[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
    pub struct Compiler {
        pub path: PathBuf,
        #[serde(rename = "as", skip_serializing_if = "Option::is_none")]
        pub as_: Option<CompilerType>,
        #[serde(default)]
        pub ignore: bool,
    }

    /// Compiler types that we can recognize and configure
    #[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
    #[serde(rename_all = "lowercase")]
    pub enum CompilerType {
        #[serde(alias = "gcc", alias = "gnu")]
        Gcc,
        #[serde(alias = "clang", alias = "llvm")]
        Clang,
        #[serde(alias = "fortran", alias = "gfortran", alias = "flang")]
        Flang,
        #[serde(alias = "ifort", alias = "intel-fortran", alias = "intel_fortran")]
        IntelFortran,
        #[serde(alias = "crayftn", alias = "cray-fortran", alias = "cray_fortran")]
        CrayFortran,
        #[serde(alias = "nvcc", alias = "cuda")]
        Cuda,
        #[serde(alias = "ccache", alias = "distcc", alias = "sccache")]
        Wrapper,
    }

    impl std::fmt::Display for CompilerType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let name = match self {
                CompilerType::Gcc => "GCC",
                CompilerType::Clang => "Clang",
                CompilerType::Flang => "Flang",
                CompilerType::IntelFortran => "Intel Fortran",
                CompilerType::CrayFortran => "Cray Fortran",
                CompilerType::Cuda => "CUDA",
                CompilerType::Wrapper => "Wrapper",
            };
            write!(f, "{}", name)
        }
    }

    /// Action to take for files matching a directory rule
    #[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
    #[serde(rename_all = "lowercase")]
    pub enum DirectoryAction {
        Include,
        Exclude,
    }

    /// A rule that specifies how to handle files within a directory
    #[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
    pub struct DirectoryRule {
        pub path: PathBuf,
        pub action: DirectoryAction,
    }

    /// Source filter configuration for controlling which files are included in the compilation database.
    ///
    /// Uses directory-based rules with order-based evaluation semantics:
    ///
    /// 1. **Order-based evaluation**: For each source file, the *last* rule whose path prefix
    ///    matches determines inclusion/exclusion.
    /// 2. **Empty directories list**: Interpreted as "include everything" (no filtering).
    /// 3. **No-match behavior**: If no rule matches a file, the file is *included*.
    /// 4. **Path matching**: Simple prefix matching, no normalization.
    /// 5. **Case sensitivity**: Always case-sensitive on all platforms.
    /// 6. **Path separators**: Platform-specific (`/` on Unix, `\` on Windows).
    /// 7. **Symlinks**: No symlink resolution — match literal paths only.
    /// 8. **Directory matching**: A rule matches both files directly in the directory and files in subdirectories.
    /// 9. **Empty path fields**: Invalid — validation must fail.
    ///
    /// **Important**: For matching to work correctly, rule paths should use the same format as
    /// configured in `format.paths.file`. This consistency is the user's responsibility.
    #[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
    pub struct SourceFilter {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub directories: Vec<DirectoryRule>,
    }

    /// Duplicate filter configuration matching the YAML format.
    #[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
    pub struct DuplicateFilter {
        pub match_on: Vec<OutputFields>,
    }

    impl Default for DuplicateFilter {
        fn default() -> Self {
            Self { match_on: vec![OutputFields::Directory, OutputFields::File, OutputFields::Arguments] }
        }
    }

    /// Represent the fields of the JSON compilation database record.
    #[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
    pub enum OutputFields {
        #[serde(rename = "directory")]
        Directory,
        #[serde(rename = "file")]
        File,
        #[serde(rename = "arguments")]
        Arguments,
        #[serde(rename = "command")]
        Command,
        #[serde(rename = "output")]
        Output,
    }

    /// Format configuration matching the YAML format.
    #[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
    pub struct Format {
        #[serde(default)]
        pub paths: PathFormat,
        #[serde(default)]
        pub entries: EntryFormat,
    }

    /// Format configuration of paths in the JSON compilation database.
    #[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
    pub struct PathFormat {
        #[serde(default)]
        pub directory: PathResolver,
        #[serde(default)]
        pub file: PathResolver,
    }

    /// Path resolver options matching the YAML format.
    #[derive(Copy, Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
    pub enum PathResolver {
        /// Leave the path as is without any transformation. (Default)
        #[default]
        #[serde(rename = "as-is")]
        AsIs,
        /// The path will be resolved to the canonical path.
        #[serde(rename = "canonical")]
        Canonical,
        /// The path will be resolved to the relative path to the directory attribute.
        #[serde(rename = "relative")]
        Relative,
        /// The path will be resolved to an absolute path.
        #[serde(rename = "absolute")]
        Absolute,
    }

    /// Configuration for formatting output entries matching the YAML format.
    #[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
    pub struct EntryFormat {
        #[serde(default = "default_enabled")]
        pub use_array_format: bool,
        #[serde(default = "default_enabled")]
        pub include_output_field: bool,
    }

    impl Default for EntryFormat {
        fn default() -> Self {
            Self { use_array_format: true, include_output_field: true }
        }
    }

    const SUPPORTED_SCHEMA_VERSION: &str = "4.0";
    const PRELOAD_LIBRARY_PATH: &str = env!("PRELOAD_LIBRARY_PATH");
    const WRAPPER_EXECUTABLE_PATH: &str = env!("WRAPPER_EXECUTABLE_PATH");

    /// The default path to the wrapper executable.
    pub(super) fn default_wrapper_executable() -> PathBuf {
        PathBuf::from(WRAPPER_EXECUTABLE_PATH)
    }

    /// The default path to the shared library that will be preloaded.
    pub(super) fn default_preload_library() -> PathBuf {
        PathBuf::from(PRELOAD_LIBRARY_PATH)
    }

    fn default_enabled() -> bool {
        true
    }

    // Custom deserialization function to validate the schema version
    fn validate_schema_version<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let schema: String = Deserialize::deserialize(deserializer)?;
        if schema != SUPPORTED_SCHEMA_VERSION {
            use serde::de::Error;
            Err(Error::custom(format!(
                "Unsupported schema version: {schema}. Expected: {SUPPORTED_SCHEMA_VERSION}"
            )))
        } else {
            Ok(schema)
        }
    }
}

pub mod validation {

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

            // Validate intercept configuration
            collector.add_result(Intercept::validate(&config.intercept));

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

    impl Validator<Intercept> for Intercept {
        type Error = ValidationError;

        fn validate(config: &Intercept) -> Result<(), Self::Error> {
            match config {
                Intercept::Wrapper { path } => {
                    if !path.exists() {
                        Err(ValidationError::PathNotFound { path: path.display().to_string() })
                    } else {
                        Ok(())
                    }
                }
                Intercept::Preload { path } => {
                    if !path.exists() {
                        Err(ValidationError::PathNotFound { path: path.display().to_string() })
                    } else {
                        Ok(())
                    }
                }
            }
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
        use tempfile::TempDir;

        #[test]
        fn test_validate_intercept_wrapper_valid_paths() {
            let temp_dir = TempDir::new().unwrap();
            let temp_file = temp_dir.path().join("test_file");
            std::fs::write(&temp_file, "test").unwrap();

            let config = Intercept::Wrapper { path: temp_file };

            assert!(Intercept::validate(&config).is_ok());
        }

        #[test]
        fn test_validate_intercept_wrapper_invalid_paths() {
            let config = Intercept::Wrapper { path: PathBuf::from("/nonexistent/path") };

            let result = Intercept::validate(&config);
            assert!(result.is_err());

            match result.unwrap_err() {
                ValidationError::PathNotFound { .. } => {}
                _ => panic!("Expected PathNotFound validation error"),
            }
        }

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
}

pub mod loader {
    use super::{Main, Validator};
    use directories::{BaseDirs, ProjectDirs};
    use log::{debug, info};
    use std::fs::OpenOptions;
    use std::path::{Path, PathBuf};
    use thiserror::Error;

    pub struct Loader {}

    impl Loader {
        /// Loads the configuration from the specified file or the default locations.
        ///
        /// If the configuration file is specified, it will be used. Otherwise, the default locations
        /// will be searched for the configuration file. If the configuration file is not found, the
        /// default configuration will be returned.
        pub fn load(
            context: &crate::context::Context,
            filename: &Option<String>,
        ) -> Result<Main, ConfigError> {
            if let Some(path) = filename {
                // If the configuration file is specified, use it.
                Self::from_file(Path::new(path))
            } else {
                // Otherwise, try to find the configuration file in the default locations.
                let locations = Self::file_locations(context);
                for location in locations {
                    debug!("Checking configuration file: {}", location.display());
                    if location.exists() {
                        return Self::from_file(location.as_path());
                    }
                }
                // If the configuration file is not found, return the default configuration.
                debug!("Configuration file not found. Using the default configuration.");
                Ok(Main::default())
            }
        }

        /// The default locations where the configuration file can be found.
        ///
        /// The locations are searched in the following order:
        /// - The current working directory.
        /// - The local configuration directory of the user.
        /// - The configuration directory of the user.
        /// - The local configuration directory of the application.
        /// - The configuration directory of the application.
        fn file_locations(context: &crate::context::Context) -> Vec<PathBuf> {
            let mut locations = Vec::new();

            locations.push(context.current_directory.clone());
            if let Some(base_dirs) = BaseDirs::new() {
                locations.push(base_dirs.config_local_dir().to_path_buf());
                locations.push(base_dirs.config_dir().to_path_buf());
            }

            if let Some(proj_dirs) = ProjectDirs::from("com.github", "rizsotto", "Bear") {
                locations.push(proj_dirs.config_local_dir().to_path_buf());
                locations.push(proj_dirs.config_dir().to_path_buf());
            }
            // filter out duplicate elements from the list
            locations.dedup();
            // append the default configuration file name to the locations
            locations.iter().map(|p| p.join("bear.yml")).collect()
        }

        /// Loads the configuration from the specified file.
        pub fn from_file(path: &Path) -> Result<Main, ConfigError> {
            info!("Loading configuration file: {}", path.display());

            let reader = OpenOptions::new()
                .read(true)
                .open(path)
                .map_err(|source| ConfigError::FileAccess { path: path.to_path_buf(), source })?;

            let content: Main = Self::from_reader(reader)
                .map_err(|source| ConfigError::ParseError { path: path.to_path_buf(), source })?;

            // Validate the loaded configuration
            Main::validate(&content)
                .map_err(|source| ConfigError::ValidationError { path: path.to_path_buf(), source })?;

            Ok(content)
        }

        /// Define the deserialization format of the config file.
        fn from_reader<R, T>(rdr: R) -> serde_yml::Result<T>
        where
            R: std::io::Read,
            T: serde::de::DeserializeOwned,
        {
            serde_yml::from_reader(rdr)
        }
    }

    /// Represents all possible configuration-related errors.
    #[derive(Debug, Error)]
    pub enum ConfigError {
        /// Error when opening or reading a configuration file.
        #[error("Failed to access configuration file '{path}': {source}")]
        FileAccess {
            path: PathBuf,
            #[source]
            source: std::io::Error,
        },
        /// Error when parsing the configuration file format.
        #[error("Failed to parse configuration from file '{path}': {source}")]
        ParseError {
            path: PathBuf,
            #[source]
            source: serde_yml::Error,
        },
        /// Error when the schema version is not supported.
        #[error("Unsupported schema version: {found}. Expected: {expected}")]
        UnsupportedSchema { found: String, expected: String },
        /// Error when configuration validation fails.
        #[error("Configuration validation failed: {source}")]
        ValidationError {
            path: PathBuf,
            #[source]
            source: crate::config::validation::ValidationError,
        },
    }

    #[cfg(test)]
    mod test {

        use super::super::*;
        use super::*;
        use std::fs;

        #[test]
        fn test_wrapper_config() {
            let content: &[u8] = br#"
            schema: 4.0

            intercept:
                mode: wrapper
                path: /usr/local/libexec/bear/wrapper

            compilers:
              - path: /usr/local/bin/cc
                as: gcc
              - path: /usr/bin/cc
                ignore: true
              - path: /usr/bin/clang++
                flags:
                    add: ["-I/opt/MPI/include"]
                    remove: ["-Wall"]

            sources:
                directories:
                  - path: "/opt/project/sources"
                    action: include
                  - path: "/opt/project/tests"
                    action: exclude

            duplicates:
                match_on: [file, directory]

            format:
                paths:
                    directory: canonical
                    file: canonical
                entries:
                    use_array_format: true
                    include_output_field: true
            "#;

            let result = Loader::from_reader(content).unwrap();

            let expected = Main {
                schema: String::from("4.0"),
                intercept: Intercept::Wrapper { path: PathBuf::from("/usr/local/libexec/bear/wrapper") },
                compilers: vec![
                    Compiler {
                        path: PathBuf::from("/usr/local/bin/cc"),
                        as_: Some(CompilerType::Gcc),
                        ignore: false,
                    },
                    Compiler { path: PathBuf::from("/usr/bin/cc"), as_: None, ignore: true },
                    Compiler { path: PathBuf::from("/usr/bin/clang++"), as_: None, ignore: false },
                ],
                sources: SourceFilter {
                    directories: vec![
                        DirectoryRule {
                            path: PathBuf::from("/opt/project/sources"),
                            action: DirectoryAction::Include,
                        },
                        DirectoryRule {
                            path: PathBuf::from("/opt/project/tests"),
                            action: DirectoryAction::Exclude,
                        },
                    ],
                },
                duplicates: DuplicateFilter { match_on: vec![OutputFields::File, OutputFields::Directory] },
                format: Format {
                    paths: PathFormat { directory: PathResolver::Canonical, file: PathResolver::Canonical },
                    entries: EntryFormat { use_array_format: true, include_output_field: true },
                },
            };

            assert_eq!(expected, result);
        }

        #[test]
        fn test_incomplete_wrapper_config() {
            let content: &[u8] = br#"
            schema: 4.0

            intercept:
              mode: wrapper

            format:
              paths:
                directory: as-is
                file: as-is
            "#;

            let result = Loader::from_reader(content).unwrap();

            let expected = Main {
                schema: String::from("4.0"),
                intercept: Intercept::Wrapper { path: default_wrapper_executable() },
                compilers: vec![],
                sources: SourceFilter { directories: vec![] },
                duplicates: DuplicateFilter {
                    match_on: vec![OutputFields::Directory, OutputFields::File, OutputFields::Arguments],
                },
                format: Format {
                    paths: PathFormat { directory: PathResolver::AsIs, file: PathResolver::AsIs },
                    entries: EntryFormat { use_array_format: true, include_output_field: true },
                },
            };

            assert_eq!(expected, result);
        }

        #[test]
        fn test_incomplete_preload_config() {
            let content: &[u8] = br#"
            schema: 4.0

            intercept:
              mode: preload
            format:
              paths:
                directory: absolute
                file: absolute
            "#;

            let result = Loader::from_reader(content).unwrap();

            let expected = Main {
                schema: String::from("4.0"),
                intercept: Intercept::Preload { path: default_preload_library() },
                compilers: vec![],
                sources: SourceFilter { directories: vec![] },
                duplicates: DuplicateFilter {
                    match_on: vec![OutputFields::Directory, OutputFields::File, OutputFields::Arguments],
                },
                format: Format {
                    paths: PathFormat { directory: PathResolver::Absolute, file: PathResolver::Absolute },
                    entries: EntryFormat { use_array_format: true, include_output_field: true },
                },
            };

            assert_eq!(expected, result);
        }

        #[test]
        fn test_default_config() {
            let result = Main::default();

            let expected = Main {
                schema: String::from("4.0"),
                intercept: Intercept::default(),
                compilers: vec![],
                sources: SourceFilter::default(),
                duplicates: DuplicateFilter::default(),
                format: Format::default(),
            };

            assert_eq!(expected, result);
        }

        #[test]
        fn test_invalid_schema_version() {
            let content: &[u8] = br#"
            schema: 3.0

            intercept:
              mode: wrapper
              directory: /tmp
            "#;

            let result: serde_yml::Result<Main> = Loader::from_reader(content);

            assert!(result.is_err());

            let message = result.unwrap_err().to_string();
            assert_eq!("Unsupported schema version: 3.0. Expected: 4.0 at line 2 column 13", message);
        }

        #[test]
        fn test_failing_config() {
            let content: &[u8] = br#"{
                "output": {
                    "format": {
                        "command_as_array": false
                    },
                    "content": {
                        "duplicates": "files"
                    }
                }
            }"#;

            let result: serde_yml::Result<Main> = Loader::from_reader(content);

            assert!(result.is_err());
        }

        #[test]
        fn test_validation_error_on_invalid_config() {
            use tempfile;

            let temp_dir = tempfile::tempdir().unwrap();
            let config_file = temp_dir.path().join("bear.yml");

            let invalid_config = r#"
            schema: "4.0"

            intercept:
                mode: wrapper
                path: /nonexistent/wrapper/path
                directory: /nonexistent/directory

            compilers:
              - path: /nonexistent/compiler
                as: "invalid_compiler_type"
                flags:
                    add: [""]
                    remove: ["valid", ""]
            "#;

            fs::write(&config_file, invalid_config).unwrap();

            // Try to load the config - should fail validation
            let result = Loader::from_file(&config_file);
            assert!(result.is_err());

            match result.unwrap_err() {
                ConfigError::ParseError { source, .. } => {
                    // Verify we got a parse error for invalid compiler type
                    let error_msg = source.to_string();
                    assert!(error_msg.contains("unknown variant"));
                    assert!(error_msg.contains("invalid_compiler_type"));
                }
                other => panic!("Expected ParseError for invalid compiler type, got: {:?}", other),
            }
        }

        #[test]
        fn test_compiler_type_serialization() {
            fn assert_compiler_type_deserializes(json_str: &str, expected: CompilerType) {
                use serde_json;

                let result = serde_json::from_str::<CompilerType>(json_str).unwrap();
                assert_eq!(result, expected);
            }

            // Test canonical names
            assert_compiler_type_deserializes("\"gcc\"", CompilerType::Gcc);
            assert_compiler_type_deserializes("\"clang\"", CompilerType::Clang);
            assert_compiler_type_deserializes("\"fortran\"", CompilerType::Flang);
            assert_compiler_type_deserializes("\"intelfortran\"", CompilerType::IntelFortran);
            assert_compiler_type_deserializes("\"crayfortran\"", CompilerType::CrayFortran);

            // Test aliases for GCC
            assert_compiler_type_deserializes("\"gnu\"", CompilerType::Gcc);

            // Test aliases for Clang
            assert_compiler_type_deserializes("\"llvm\"", CompilerType::Clang);

            // Test aliases for Fortran
            assert_compiler_type_deserializes("\"gfortran\"", CompilerType::Flang);

            // Test aliases for Intel Fortran
            assert_compiler_type_deserializes("\"ifort\"", CompilerType::IntelFortran);
            assert_compiler_type_deserializes("\"intel-fortran\"", CompilerType::IntelFortran);
            assert_compiler_type_deserializes("\"intel_fortran\"", CompilerType::IntelFortran);

            // Test aliases for Cray Fortran
            assert_compiler_type_deserializes("\"crayftn\"", CompilerType::CrayFortran);
            assert_compiler_type_deserializes("\"cray-fortran\"", CompilerType::CrayFortran);
            assert_compiler_type_deserializes("\"cray_fortran\"", CompilerType::CrayFortran);
        }

        #[test]
        fn test_compiler_config_with_type_hints() {
            let temp_dir = tempfile::tempdir().unwrap();
            let config_file = temp_dir.path().join("bear.yml");

            // Create temporary compiler files for validation
            let gcc_wrapper = temp_dir.path().join("custom-gcc-wrapper");
            let clang_wrapper = temp_dir.path().join("custom-clang");
            let fortran_wrapper = temp_dir.path().join("my-fortran");
            let intel_wrapper = temp_dir.path().join("ifort-wrapper");
            let cray_wrapper = temp_dir.path().join("ftn-wrapper");

            fs::write(&gcc_wrapper, "#!/bin/bash\necho gcc wrapper").unwrap();
            fs::write(&clang_wrapper, "#!/bin/bash\necho clang wrapper").unwrap();
            fs::write(&fortran_wrapper, "#!/bin/bash\necho fortran wrapper").unwrap();
            fs::write(&intel_wrapper, "#!/bin/bash\necho intel wrapper").unwrap();
            fs::write(&cray_wrapper, "#!/bin/bash\necho cray wrapper").unwrap();

            // Create wrapper executable and directory for validation
            let wrapper_dir = temp_dir.path().join("wrapper");
            std::fs::create_dir(&wrapper_dir).unwrap();
            let wrapper_exe = wrapper_dir.join("wrapper");
            fs::write(&wrapper_exe, "#!/bin/bash\necho wrapper").unwrap();

            let config_with_hints = format!(
                r#"
                schema: "4.0"

                intercept:
                    mode: wrapper
                    path: {}
                    directory: {}

                compilers:
                  - path: {}
                    as: "gcc"
                  - path: {}
                    as: "llvm"
                  - path: {}
                    as: "gfortran"
                  - path: {}
                    as: "intel-fortran"
                  - path: {}
                    as: "cray_fortran"
                "#,
                wrapper_exe.display(),
                wrapper_dir.display(),
                gcc_wrapper.display(),
                clang_wrapper.display(),
                fortran_wrapper.display(),
                intel_wrapper.display(),
                cray_wrapper.display()
            );

            fs::write(&config_file, config_with_hints).unwrap();

            let result = Loader::from_file(&config_file);

            assert!(result.is_ok());

            let config = result.unwrap();
            assert_eq!(config.compilers.len(), 5);

            // Verify compiler type hints are correctly parsed
            assert_eq!(config.compilers[0].as_, Some(CompilerType::Gcc));
            assert_eq!(config.compilers[1].as_, Some(CompilerType::Clang));
            assert_eq!(config.compilers[2].as_, Some(CompilerType::Flang));
            assert_eq!(config.compilers[3].as_, Some(CompilerType::IntelFortran));
            assert_eq!(config.compilers[4].as_, Some(CompilerType::CrayFortran));
        }

        #[test]
        fn test_compiler_type_display() {
            assert_eq!(CompilerType::Gcc.to_string(), "GCC");
            assert_eq!(CompilerType::Clang.to_string(), "Clang");
            assert_eq!(CompilerType::Flang.to_string(), "Flang");
            assert_eq!(CompilerType::IntelFortran.to_string(), "Intel Fortran");
            assert_eq!(CompilerType::CrayFortran.to_string(), "Cray Fortran");
        }
    }
}
