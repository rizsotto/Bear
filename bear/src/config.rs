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
//!   directory: /tmp
//!
//! compilers:
//!   - path: /usr/local/bin/cc
//!     as: gcc
//!   - path: /usr/bin/cc
//!     ignore: true
//!   - path: /usr/bin/clang++
//!     flags:
//!       add: ["-I/opt/MPI/include"]
//!       remove: ["-Wall"]
//!
//! sources:
//!   only_existing_files: true
//!   include: ["/opt/project/sources"]
//!   exclude: ["/opt/project/tests"]
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
pub use loader::Loader;
pub use types::*;

mod types {
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;

    /// Represents the application configuration with flattened structure.
    #[derive(Debug, PartialEq, Deserialize, Serialize)]
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

    /// Simplified intercept configuration with mode and directory.
    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    #[serde(tag = "mode")]
    pub enum Intercept {
        #[serde(rename = "wrapper")]
        Wrapper {
            #[serde(default = "default_wrapper_executable")]
            path: PathBuf,
            #[serde(default = "default_wrapper_directory")]
            directory: PathBuf,
        },
        #[serde(rename = "preload")]
        Preload {
            #[serde(default = "default_preload_library")]
            path: PathBuf,
        },
    }

    /// The default intercept mode is varying based on the target operating system.
    impl Default for Intercept {
        #[cfg(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "dragonfly"
        ))]
        fn default() -> Self {
            Intercept::Preload {
                path: default_preload_library(),
            }
        }

        #[cfg(any(target_os = "macos", target_os = "ios", target_os = "windows"))]
        fn default() -> Self {
            Intercept::Wrapper {
                path: default_wrapper_executable(),
                directory: default_wrapper_directory(),
            }
        }
    }

    /// Represents compiler configuration matching the YAML format.
    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    pub struct Compiler {
        pub path: PathBuf,
        #[serde(rename = "as", skip_serializing_if = "Option::is_none")]
        pub as_: Option<String>,
        #[serde(default)]
        pub ignore: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub flags: Option<CompilerFlags>,
    }

    /// Compiler flags configuration for add/remove operations.
    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    pub struct CompilerFlags {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub add: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub remove: Vec<String>,
    }

    /// Source filter configuration matching the YAML format.
    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    pub struct SourceFilter {
        #[serde(default = "default_enabled")]
        pub only_existing_files: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub include: Vec<PathBuf>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub exclude: Vec<PathBuf>,
    }

    impl Default for SourceFilter {
        fn default() -> Self {
            Self {
                only_existing_files: true,
                include: vec![],
                exclude: vec![],
            }
        }
    }

    /// Duplicate filter configuration matching the YAML format.
    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    pub struct DuplicateFilter {
        pub match_on: Vec<OutputFields>,
    }

    impl Default for DuplicateFilter {
        fn default() -> Self {
            Self {
                match_on: vec![OutputFields::File, OutputFields::Arguments],
            }
        }
    }

    /// Represent the fields of the JSON compilation database record.
    #[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
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
    #[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
    pub struct Format {
        #[serde(default)]
        pub paths: PathFormat,
        #[serde(default)]
        pub entries: EntryFormat,
    }

    /// Format configuration of paths in the JSON compilation database.
    #[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
    pub struct PathFormat {
        #[serde(default)]
        pub directory: PathResolver,
        #[serde(default)]
        pub file: PathResolver,
    }

    /// Path resolver options matching the YAML format.
    #[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
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
    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    pub struct EntryFormat {
        #[serde(default = "default_enabled")]
        pub use_array_format: bool,
        #[serde(default = "default_enabled")]
        pub include_output_field: bool,
    }

    impl Default for EntryFormat {
        fn default() -> Self {
            Self {
                use_array_format: true,
                include_output_field: true,
            }
        }
    }

    const SUPPORTED_SCHEMA_VERSION: &str = "4.0";
    const PRELOAD_LIBRARY_PATH: &str = env!("PRELOAD_LIBRARY_PATH");
    const WRAPPER_EXECUTABLE_PATH: &str = env!("WRAPPER_EXECUTABLE_PATH");

    /// The default directory where the wrapper executables will be stored.
    pub(super) fn default_wrapper_directory() -> PathBuf {
        std::env::temp_dir()
    }

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

pub mod loader {
    use super::Main;
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
        pub fn load(filename: &Option<String>) -> Result<Main, ConfigError> {
            if let Some(path) = filename {
                // If the configuration file is specified, use it.
                Self::from_file(Path::new(path))
            } else {
                // Otherwise, try to find the configuration file in the default locations.
                let locations = Self::file_locations();
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
        fn file_locations() -> Vec<PathBuf> {
            let mut locations = Vec::new();

            if let Ok(current_dir) = std::env::current_dir() {
                locations.push(current_dir);
            }
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

            let reader = OpenOptions::new().read(true).open(path).map_err(|source| {
                ConfigError::FileAccess {
                    path: path.to_path_buf(),
                    source,
                }
            })?;

            let content: Main =
                Self::from_reader(reader).map_err(|source| ConfigError::ParseError {
                    path: path.to_path_buf(),
                    source,
                })?;

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
    }

    #[cfg(test)]
    mod test {
        use super::super::*;
        use super::*;

        #[test]
        fn test_wrapper_config() {
            let content: &[u8] = br#"
            schema: 4.0

            intercept:
                mode: wrapper
                path: /usr/local/libexec/bear/wrapper
                directory: /tmp

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
                only_existing_files: true
                include: ["/opt/project/sources"]
                exclude: ["/opt/project/tests"]

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
                intercept: Intercept::Wrapper {
                    path: PathBuf::from("/usr/local/libexec/bear/wrapper"),
                    directory: PathBuf::from("/tmp"),
                },
                compilers: vec![
                    Compiler {
                        path: PathBuf::from("/usr/local/bin/cc"),
                        as_: Some("gcc".to_string()),
                        ignore: false,
                        flags: None,
                    },
                    Compiler {
                        path: PathBuf::from("/usr/bin/cc"),
                        as_: None,
                        ignore: true,
                        flags: None,
                    },
                    Compiler {
                        path: PathBuf::from("/usr/bin/clang++"),
                        as_: None,
                        ignore: false,
                        flags: Some(CompilerFlags {
                            add: vec!["-I/opt/MPI/include".to_string()],
                            remove: vec!["-Wall".to_string()],
                        }),
                    },
                ],
                sources: SourceFilter {
                    only_existing_files: true,
                    include: vec![PathBuf::from("/opt/project/sources")],
                    exclude: vec![PathBuf::from("/opt/project/tests")],
                },
                duplicates: DuplicateFilter {
                    match_on: vec![OutputFields::File, OutputFields::Directory],
                },
                format: Format {
                    paths: PathFormat {
                        directory: PathResolver::Canonical,
                        file: PathResolver::Canonical,
                    },
                    entries: EntryFormat {
                        use_array_format: true,
                        include_output_field: true,
                    },
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
                intercept: Intercept::Wrapper {
                    path: default_wrapper_executable(),
                    directory: default_wrapper_directory(),
                },
                compilers: vec![],
                sources: SourceFilter {
                    only_existing_files: true,
                    include: vec![],
                    exclude: vec![],
                },
                duplicates: DuplicateFilter {
                    match_on: vec![OutputFields::File, OutputFields::Arguments],
                },
                format: Format {
                    paths: PathFormat {
                        directory: PathResolver::AsIs,
                        file: PathResolver::AsIs,
                    },
                    entries: EntryFormat {
                        use_array_format: true,
                        include_output_field: true,
                    },
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
            sources:
              only_existing_files: false
            format:
              paths:
                directory: absolute
                file: absolute
            "#;

            let result = Loader::from_reader(content).unwrap();

            let expected = Main {
                schema: String::from("4.0"),
                intercept: Intercept::Preload {
                    path: default_preload_library(),
                },
                compilers: vec![],
                sources: SourceFilter {
                    only_existing_files: false,
                    include: vec![],
                    exclude: vec![],
                },
                duplicates: DuplicateFilter {
                    match_on: vec![OutputFields::File, OutputFields::Arguments],
                },
                format: Format {
                    paths: PathFormat {
                        directory: PathResolver::Absolute,
                        file: PathResolver::Absolute,
                    },
                    entries: EntryFormat {
                        use_array_format: true,
                        include_output_field: true,
                    },
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
            assert_eq!(
                "Unsupported schema version: 3.0. Expected: 4.0 at line 2 column 13",
                message
            );
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
    }
}
