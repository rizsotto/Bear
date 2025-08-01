// SPDX-License-Identifier: GPL-3.0-or-later

//! This module defines the configuration of the application.
//!
//! The configuration is either loaded from a file or used with the default
//! values, which are defined in the code. The configuration exposes the main
//! logical steps that the application will follow.
//!
//! The configuration file syntax is based on the YAML format.
//! The default configuration file name is `bear.yml`.
//!
//! The configuration file location is searched in the following order:
//! - The current working directory.
//! - The local configuration directory of the user.
//! - The configuration directory of the user.
//! - The local configuration directory of the application.
//! - The configuration directory of the application.
//!
//! ```yaml
//! schema: 4.0
//!
//! intercept:
//!   mode: wrapper
//!   directory: /tmp
//!   executables:
//!     - /usr/bin/cc
//!     - /usr/bin/c++
//!     - /usr/bin/clang
//!     - /usr/bin/clang++
//! output:
//!   specification: clang
//!   compilers:
//!     - path: /usr/local/bin/cc
//!       ignore: never
//!     - path: /usr/bin/cc
//!       ignore: always
//!     - path: /usr/bin/c++
//!       ignore: conditional
//!       arguments:
//!         match:
//!           - -###
//!     - path: /usr/bin/clang
//!       ignore: never
//!       arguments:
//!         add:
//!           - -DDEBUG
//!         remove:
//!           - -Wall
//!     - path: /usr/bin/clang++
//!       arguments:
//!         remove:
//!           - -Wall
//!   sources:
//!     only_existing_files: true
//!     paths:
//!       - path: /opt/project/sources
//!         ignore: never
//!       - path: /opt/project/tests
//!         ignore: always
//!   duplicates:
//!     by_fields:
//!       - file
//!       - directory
//!   format:
//!     paths:
//!       directory: canonical
//!       file: canonical
//!       output: canonical
//!     entry:
//!       command_as_array: true
//!       keep_output_field: true
//! ```
//!
//! ```yaml
//! schema: 4.0
//!
//! intercept:
//!   mode: preload
//! output:
//!   specification: bear
//! ```

// Re-Export the types and the loader module content.
pub use loader::Loader;
pub use types::*;

mod types {
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;

    /// Represents the application configuration.
    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    pub struct Main {
        #[serde(deserialize_with = "validate_schema_version")]
        pub schema: String,
        #[serde(default)]
        pub intercept: Intercept,
        #[serde(default)]
        pub output: Output,
    }

    impl Default for Main {
        fn default() -> Self {
            Self {
                schema: String::from(SUPPORTED_SCHEMA_VERSION),
                intercept: Intercept::default(),
                output: Output::default(),
            }
        }
    }

    /// Intercept configuration is either a wrapper or a preload mode.
    ///
    /// In wrapper mode, the compiler is wrapped with a script that intercepts the compiler calls.
    /// The configuration for that is capturing the directory where the wrapper scripts are stored
    /// and the list of executables to wrap.
    ///
    /// In preload mode, the compiler is intercepted by a shared library preloaded before
    /// the compiler is executed. The configuration for that is the path to the shared library.
    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    #[serde(tag = "mode")]
    pub enum Intercept {
        #[serde(rename = "wrapper")]
        Wrapper {
            #[serde(default = "default_wrapper_executable")]
            path: PathBuf,
            #[serde(default = "default_wrapper_directory")]
            directory: PathBuf,
            executables: Vec<PathBuf>,
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

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        fn default() -> Self {
            Intercept::Wrapper {
                path: default_wrapper_executable(),
                directory: default_wrapper_directory(),
                executables: vec![
                    PathBuf::from("/usr/bin/cc"),
                    PathBuf::from("/usr/bin/c++"),
                    PathBuf::from("/usr/bin/clang"),
                    PathBuf::from("/usr/bin/clang++"),
                ],
            }
        }

        #[cfg(target_os = "windows")]
        fn default() -> Self {
            Intercept::Wrapper {
                path: default_wrapper_executable(),
                directory: default_wrapper_directory(),
                executables: vec![
                    PathBuf::from("C:\\msys64\\mingw64\\bin\\gcc.exe"),
                    PathBuf::from("C:\\msys64\\mingw64\\bin\\g++.exe"),
                ],
            }
        }
    }

    /// Output configuration is used to customize the output format.
    ///
    /// Allow customizing the output format of the compiler calls.
    ///
    /// - Clang: Output the compiler calls in the clang project defined "JSON compilation database"
    ///   format. (The format is used by clang tooling and other tools based on that library.)
    /// - Semantic: Output the compiler calls in the semantic format. (The format is not defined yet.)
    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    #[serde(tag = "specification")]
    pub enum Output {
        #[serde(rename = "clang")]
        Clang {
            #[serde(default)]
            compilers: Vec<Compiler>,
            #[serde(default)]
            sources: SourceFilter,
            #[serde(default)]
            duplicates: DuplicateFilter,
            #[serde(default)]
            format: Format,
        },
        #[serde(rename = "bear")]
        Semantic {},
    }

    /// The default output is the clang format.
    impl Default for Output {
        fn default() -> Self {
            Output::Clang {
                compilers: vec![],
                sources: SourceFilter::default(),
                duplicates: DuplicateFilter::default(),
                format: Format::default(),
            }
        }
    }

    /// Represents instructions to transform the compiler calls.
    ///
    /// Allow transforming the compiler calls by adding or removing arguments.
    /// It also can instruct to filter out the compiler call from the output.
    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    pub struct Compiler {
        pub path: PathBuf,
        #[serde(default)]
        pub ignore: IgnoreOrConsider,
        #[serde(default)]
        pub arguments: Arguments,
    }

    /// Represents instructions to ignore the compiler call.
    ///
    /// The meaning of the possible values is:
    /// - Always: Always ignore the compiler call.
    /// - Never: Never ignore the compiler call. (Default)
    /// - Conditional: Ignore the compiler call if the arguments match.
    #[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
    pub enum IgnoreOrConsider {
        #[serde(rename = "always", alias = "true")]
        Always,
        #[default]
        #[serde(rename = "never", alias = "false")]
        Never,
        #[serde(rename = "conditional")]
        Conditional,
    }

    /// Argument lists to match, add or remove.
    ///
    /// The `match` field is used to specify the arguments to match. Can be used only with the
    /// conditional mode.
    ///
    /// The `add` or `remove` fields are used to specify the arguments to add or remove. These can be
    /// used with the conditional or never ignore mode.
    #[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
    pub struct Arguments {
        #[serde(default, rename = "match")]
        pub match_: Vec<String>,
        #[serde(default)]
        pub add: Vec<String>,
        #[serde(default)]
        pub remove: Vec<String>,
    }

    /// Source filter configuration is used to filter the compiler calls based on the source files.
    ///
    /// Allow filtering the compiler calls based on the source files.
    ///
    /// - Include only existing files: can be true or false.
    /// - List of directories to include or exclude.
    ///   (The order of these entries will imply the order of evaluation.)
    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    pub struct SourceFilter {
        #[serde(default = "default_enabled")]
        pub only_existing_files: bool,
        #[serde(default)]
        pub paths: Vec<DirectoryFilter>,
    }

    impl Default for SourceFilter {
        fn default() -> Self {
            Self {
                only_existing_files: true,
                paths: vec![],
            }
        }
    }

    /// Directory filter configuration is used to filter the compiler calls based on
    /// the source file location.
    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    pub struct DirectoryFilter {
        pub path: PathBuf,
        pub ignore: Ignore,
    }

    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    pub enum Ignore {
        #[serde(rename = "always", alias = "true")]
        Always,
        #[serde(rename = "never", alias = "false")]
        Never,
    }

    /// Duplicate filter configuration is used to filter the duplicate compiler calls.
    ///
    /// - By fields: Specify the fields of the JSON compilation database record to detect duplicates.
    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    pub struct DuplicateFilter {
        pub by_fields: Vec<OutputFields>,
    }

    impl Default for DuplicateFilter {
        fn default() -> Self {
            Self {
                by_fields: vec![OutputFields::File, OutputFields::Arguments],
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

    /// Format configuration of the JSON compilation database.
    #[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
    pub struct Format {
        #[serde(default)]
        pub paths: PathFormat,
        #[serde(default)]
        pub entry: EntryFormat,
    }

    /// Format configuration of paths in the JSON compilation database.
    #[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
    pub struct PathFormat {
        #[serde(default)]
        pub directory: PathResolver,
        #[serde(default)]
        pub file: PathResolver,
        #[serde(default)]
        pub output: PathResolver,
    }

    #[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
    pub enum PathResolver {
        /// The directory path will be resolved to the canonical path. (Default)
        #[default]
        #[serde(rename = "canonical")]
        Canonical,
        /// The directory path will be resolved to the relative path to the directory attribute.
        #[serde(rename = "relative")]
        Relative,
    }

    /// Configuration for formatting output entries.
    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    pub struct EntryFormat {
        #[serde(default = "default_enabled")]
        pub command_field_as_array: bool,
        #[serde(default = "default_enabled")]
        pub keep_output_field: bool,
    }

    impl Default for EntryFormat {
        fn default() -> Self {
            Self {
                command_field_as_array: true,
                keep_output_field: true,
            }
        }
    }

    pub(super) const SUPPORTED_SCHEMA_VERSION: &str = "4.0";
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
          directory: /tmp
          executables:
            - /usr/bin/cc
            - /usr/bin/c++
            - /usr/bin/clang
            - /usr/bin/clang++
        output:
          specification: clang
          compilers:
            - path: /usr/local/bin/cc
              ignore: always
            - path: /usr/bin/cc
              ignore: never
            - path: /usr/bin/c++
              ignore: conditional
              arguments:
                match:
                  - -###
            - path: /usr/bin/clang
              ignore: never
              arguments:
                add:
                  - -DDEBUG
                remove:
                  - -Wall
            - path: /usr/bin/clang++
              arguments:
                remove:
                  - -Wall
          sources:
            only_existing_files: true
            paths:
              - path: /opt/project/sources
                ignore: never
              - path: /opt/project/tests
                ignore: always
          duplicates:
            by_fields:
              - file
              - directory
          format:
            paths:
              directory: canonical
              file: canonical
              output: canonical
        "#;

            let result = Loader::from_reader(content).unwrap();

            let expected = Main {
                intercept: Intercept::Wrapper {
                    path: default_wrapper_executable(),
                    directory: PathBuf::from("/tmp"),
                    executables: vec![
                        "/usr/bin/cc",
                        "/usr/bin/c++",
                        "/usr/bin/clang",
                        "/usr/bin/clang++",
                    ]
                    .into_iter()
                    .map(PathBuf::from)
                    .collect(),
                },
                output: Output::Clang {
                    compilers: vec![
                        Compiler {
                            path: PathBuf::from("/usr/local/bin/cc"),
                            ignore: IgnoreOrConsider::Always,
                            arguments: Arguments::default(),
                        },
                        Compiler {
                            path: PathBuf::from("/usr/bin/cc"),
                            ignore: IgnoreOrConsider::Never,
                            arguments: Arguments::default(),
                        },
                        Compiler {
                            path: PathBuf::from("/usr/bin/c++"),
                            ignore: IgnoreOrConsider::Conditional,
                            arguments: Arguments {
                                match_: vec!["-###".into()],
                                ..Default::default()
                            },
                        },
                        Compiler {
                            path: PathBuf::from("/usr/bin/clang"),
                            ignore: IgnoreOrConsider::Never,
                            arguments: Arguments {
                                add: vec!["-DDEBUG".into()],
                                remove: vec!["-Wall".into()],
                                ..Default::default()
                            },
                        },
                        Compiler {
                            path: PathBuf::from("/usr/bin/clang++"),
                            ignore: IgnoreOrConsider::Never,
                            arguments: Arguments {
                                remove: vec!["-Wall".into()],
                                ..Default::default()
                            },
                        },
                    ],
                    sources: SourceFilter {
                        only_existing_files: true,
                        paths: vec![
                            DirectoryFilter {
                                path: PathBuf::from("/opt/project/sources"),
                                ignore: Ignore::Never,
                            },
                            DirectoryFilter {
                                path: PathBuf::from("/opt/project/tests"),
                                ignore: Ignore::Always,
                            },
                        ],
                    },
                    duplicates: DuplicateFilter {
                        by_fields: vec![OutputFields::File, OutputFields::Directory],
                    },
                    format: Format {
                        paths: PathFormat {
                            directory: PathResolver::Canonical,
                            file: PathResolver::Canonical,
                            output: PathResolver::Canonical,
                        },
                        entry: EntryFormat {
                            command_field_as_array: true,
                            keep_output_field: true,
                        },
                    },
                },
                schema: String::from("4.0"),
            };

            assert_eq!(expected, result);
        }

        #[test]
        fn test_incomplete_wrapper_config() {
            let content: &[u8] = br#"
        schema: 4.0

        intercept:
          mode: wrapper
          executables:
            - /usr/bin/cc
            - /usr/bin/c++
        output:
          specification: clang
          sources:
            only_existing_files: true
          duplicates:
            by_fields:
              - file
              - directory
        "#;

            let result = Loader::from_reader(content).unwrap();

            let expected = Main {
                intercept: Intercept::Wrapper {
                    path: default_wrapper_executable(),
                    directory: default_wrapper_directory(),
                    executables: vec!["/usr/bin/cc", "/usr/bin/c++"]
                        .into_iter()
                        .map(PathBuf::from)
                        .collect(),
                },
                output: Output::Clang {
                    compilers: vec![],
                    sources: SourceFilter {
                        only_existing_files: true,
                        paths: vec![],
                    },
                    duplicates: DuplicateFilter {
                        by_fields: vec![OutputFields::File, OutputFields::Directory],
                    },
                    format: Format {
                        paths: PathFormat {
                            directory: PathResolver::Canonical,
                            file: PathResolver::Canonical,
                            output: PathResolver::Canonical,
                        },
                        entry: EntryFormat {
                            command_field_as_array: true,
                            keep_output_field: true,
                        },
                    },
                },
                schema: String::from("4.0"),
            };

            assert_eq!(expected, result);
        }

        #[test]
        fn test_preload_config() {
            let content: &[u8] = br#"
        schema: 4.0

        intercept:
          mode: preload
          path: /usr/local/lib/libexec.so
        output:
          specification: bear
        "#;

            let result = Loader::from_reader(content).unwrap();

            let expected = Main {
                intercept: Intercept::Preload {
                    path: PathBuf::from("/usr/local/lib/libexec.so"),
                },
                output: Output::Semantic {},
                schema: String::from("4.0"),
            };

            assert_eq!(expected, result);
        }

        #[test]
        fn test_incomplete_preload_config() {
            let content: &[u8] = br#"
        schema: 4.0

        intercept:
          mode: preload
        output:
          specification: clang
          compilers:
            - path: /usr/local/bin/cc
            - path: /usr/local/bin/c++
            - path: /usr/local/bin/clang
              ignore: always
            - path: /usr/local/bin/clang++
              ignore: always
          sources:
            only_existing_files: false
          duplicates:
            by_fields:
              - file
          format:
            paths:
              directory: relative
              file: relative
              output: relative
            entry:
              command_field_as_array: false
              keep_output_field: false
        "#;

            let result = Loader::from_reader(content).unwrap();

            let expected = Main {
                intercept: Intercept::Preload {
                    path: default_preload_library(),
                },
                output: Output::Clang {
                    compilers: vec![
                        Compiler {
                            path: PathBuf::from("/usr/local/bin/cc"),
                            ignore: IgnoreOrConsider::Never,
                            arguments: Arguments::default(),
                        },
                        Compiler {
                            path: PathBuf::from("/usr/local/bin/c++"),
                            ignore: IgnoreOrConsider::Never,
                            arguments: Arguments::default(),
                        },
                        Compiler {
                            path: PathBuf::from("/usr/local/bin/clang"),
                            ignore: IgnoreOrConsider::Always,
                            arguments: Arguments::default(),
                        },
                        Compiler {
                            path: PathBuf::from("/usr/local/bin/clang++"),
                            ignore: IgnoreOrConsider::Always,
                            arguments: Arguments::default(),
                        },
                    ],
                    sources: SourceFilter {
                        only_existing_files: false,
                        paths: vec![],
                    },
                    duplicates: DuplicateFilter {
                        by_fields: vec![OutputFields::File],
                    },
                    format: Format {
                        paths: PathFormat {
                            directory: PathResolver::Relative,
                            file: PathResolver::Relative,
                            output: PathResolver::Relative,
                        },
                        entry: EntryFormat {
                            command_field_as_array: false,
                            keep_output_field: false,
                        },
                    },
                },
                schema: String::from("4.0"),
            };

            assert_eq!(expected, result);
        }

        #[test]
        fn test_default_config() {
            let result = Main::default();

            let expected = Main {
                intercept: Intercept::default(),
                output: Output::Clang {
                    compilers: vec![],
                    sources: SourceFilter {
                        only_existing_files: true,
                        paths: vec![],
                    },
                    duplicates: DuplicateFilter::default(),
                    format: Format::default(),
                },
                schema: String::from(SUPPORTED_SCHEMA_VERSION),
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
          executables:
            - /usr/bin/gcc
            - /usr/bin/g++
        "#;

            let result: serde_yml::Result<Main> = Loader::from_reader(content);

            assert!(result.is_err());

            let message = result.unwrap_err().to_string();
            assert_eq!(
                "Unsupported schema version: 3.0. Expected: 4.0 at line 2 column 9",
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
