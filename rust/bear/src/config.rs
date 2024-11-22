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
//! The configuration file content is validated against the schema version,
//! syntax and semantic constraints. If the configuration file is invalid,
//! the application will exit with an error message explaining the issue.
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
//! output:
//!   specification: clang
//!   compilers:
//!     - path: /usr/local/bin/cc
//!       ignore: always
//!     - path: /usr/local/bin/c++
//!       ignore: conditional
//!       arguments:
//!         match:
//!           - -###
//!     - path: /usr/local/bin/clang
//!       ignore: never
//!       arguments:
//!         add:
//!           - -DDEBUG
//!         remove:
//!           - -Wall
//!     - path: /usr/local/bin/clang++
//!       arguments:
//!         remove:
//!           - -Wall
//!   filter:
//!     source:
//!       include_only_existing_files: true
//!       paths_to_include:
//!         - sources
//!       paths_to_exclude:
//!         - tests
//!     duplicates:
//!       by_fields:
//!         - file
//!         - directory
//!   format:
//!     command_as_array: true
//!     drop_output_field: false
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

use std::collections::HashSet;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::{BaseDirs, ProjectDirs};
use log::{debug, info};
use serde::{Deserialize, Serialize};

const SUPPORTED_SCHEMA_VERSION: &str = "4.0";
const PRELOAD_LIBRARY_PATH: &str = env!("PRELOAD_LIBRARY_PATH");
const WRAPPER_EXECUTABLE_PATH: &str = env!("WRAPPER_EXECUTABLE_PATH");

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

impl Main {
    /// Loads the configuration from the specified file or the default locations.
    ///
    /// If the configuration file is specified, it will be used. Otherwise, the default locations
    /// will be searched for the configuration file. If the configuration file is not found, the
    /// default configuration will be returned.
    pub fn load(file: &Option<String>) -> Result<Self> {
        if let Some(path) = file {
            // If the configuration file is specified, use it.
            let config_file_path = PathBuf::from(path);
            Self::from_file(config_file_path.as_path())
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
            Ok(Self::default())
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
    pub fn from_file(file: &Path) -> Result<Self> {
        info!("Loading configuration file: {}", file.display());

        let reader = OpenOptions::new()
            .read(true)
            .open(file)
            .with_context(|| format!("Failed to open configuration file: {:?}", file))?;

        let content: Self = Self::from_reader(reader)
            .with_context(|| format!("Failed to parse configuration from file: {:?}", file))?;

        content.validate()
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

impl Default for Main {
    fn default() -> Self {
        Main {
            schema: String::from(SUPPORTED_SCHEMA_VERSION),
            intercept: Intercept::default(),
            output: Output::default(),
        }
    }
}

impl Validate for Main {
    /// Validate the configuration of the main configuration.
    fn validate(self) -> Result<Self> {
        let intercept = self.intercept.validate()?;
        let output = self.output.validate()?;

        Ok(Main {
            schema: self.schema,
            intercept,
            output,
        })
    }
}

/// Intercept configuration is either a wrapper or a preload mode.
///
/// In wrapper mode, the compiler is wrapped with a script that intercepts the compiler calls.
/// The configuration for that is capturing the directory where the wrapper scripts are stored
/// and the list of executables to wrap.
///
/// In preload mode, the compiler is intercepted by a shared library that is preloaded before
/// the compiler is executed. The configuration for that is the path to the shared library.
#[derive(Debug, PartialEq, Deserialize, Serialize)]
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
    #[cfg(target_os = "linux")]
    fn default() -> Self {
        Intercept::Preload {
            path: default_preload_library(),
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn default() -> Self {
        Intercept::Wrapper {
            path: default_wrapper_executable(),
            directory: default_wrapper_directory(),
            executables: vec![], // FIXME: better default value
        }
    }
}

impl Validate for Intercept {
    /// Validate the configuration of the intercept mode.
    fn validate(self) -> Result<Self> {
        match self {
            Intercept::Wrapper {
                path,
                directory,
                executables,
            } => {
                if is_empty_path(&path) {
                    anyhow::bail!("The wrapper path cannot be empty.");
                }
                if is_empty_path(&directory) {
                    anyhow::bail!("The wrapper directory cannot be empty.");
                }
                if executables.is_empty() {
                    anyhow::bail!("The list of executables to wrap cannot be empty.");
                }
                Ok(Intercept::Wrapper {
                    path,
                    directory,
                    executables,
                })
            }
            Intercept::Preload { path } => {
                if is_empty_path(&path) {
                    anyhow::bail!("The preload library path cannot be empty.");
                }
                Ok(Intercept::Preload { path })
            }
        }
    }
}

/// Output configuration is used to customize the output format.
///
/// Allow to customize the output format of the compiler calls.
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
        filter: Filter,
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
            filter: Filter::default(),
            format: Format::default(),
        }
    }
}

impl Validate for Output {
    /// Validate the configuration of the output writer.
    fn validate(self) -> Result<Self> {
        match self {
            Output::Clang {
                compilers,
                filter,
                format,
            } => {
                let compilers = compilers.validate()?;
                let filter = filter.validate()?;
                Ok(Output::Clang {
                    compilers,
                    filter,
                    format,
                })
            }
            Output::Semantic {} => Ok(Output::Semantic {}),
        }
    }
}

/// Represents instructions to transform the compiler calls.
///
/// Allow to transform the compiler calls by adding or removing arguments.
/// It also can instruct to filter out the compiler call from the output.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Compiler {
    pub path: PathBuf,
    #[serde(default = "default_never_ignore")]
    pub ignore: Ignore,
    #[serde(default)]
    pub arguments: Arguments,
}

impl Validate for Vec<Compiler> {
    /// Validate the configuration of the compiler list.
    ///
    /// Duplicate entries are allowed in the list. The reason behind this is
    /// that the same compiler can be ignored with some arguments and not
    /// ignored (but transformed) with other arguments.
    // TODO: check for duplicate entries
    // TODO: check if a match argument is used after an always or never
    fn validate(self) -> Result<Self> {
        self.into_iter()
            .map(|compiler| compiler.validate())
            .collect::<Result<Vec<_>>>()
    }
}

impl Validate for Compiler {
    /// Validate the configuration of the compiler.
    fn validate(self) -> Result<Self> {
        match self.ignore {
            Ignore::Always if self.arguments != Arguments::default() => {
                anyhow::bail!(
                    "All arguments must be empty in always ignore mode. {:?}",
                    self.path
                );
            }
            Ignore::Conditional if self.arguments.match_.is_empty() => {
                anyhow::bail!(
                    "The match arguments cannot be empty in conditional ignore mode. {:?}",
                    self.path
                );
            }
            Ignore::Never if !self.arguments.match_.is_empty() => {
                anyhow::bail!(
                    "The arguments must be empty in never ignore mode. {:?}",
                    self.path
                );
            }
            _ if is_empty_path(&self.path) => {
                anyhow::bail!("The compiler path cannot be empty.");
            }
            _ => Ok(self),
        }
    }
}

/// Represents instructions to ignore the compiler call.
///
/// The meaning of the possible values are:
/// - Always: Always ignore the compiler call.
/// - Conditional: Ignore the compiler call if the arguments match.
/// - Never: Never ignore the compiler call. (Default)
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Ignore {
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "conditional")]
    Conditional,
    #[serde(rename = "never")]
    Never,
}

/// Argument lists to match, add or remove.
///
/// The `match` field is used to specify the arguments to match. Can be used only with the
/// conditional ignore mode.
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

/// Filter configuration is used to filter the compiler calls.
///
/// Allow to filter the compiler calls by compiler, source files and duplicates.
///
/// - Compilers: Specify on the compiler path and arguments.
/// - Source: Specify the source file location.
/// - Duplicates: Specify the fields of the JSON compilation database record to detect duplicates.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Filter {
    #[serde(default)]
    pub source: SourceFilter,
    #[serde(default)]
    pub duplicates: DuplicateFilter,
}

impl Validate for Filter {
    /// Validate the configuration of the output writer.
    fn validate(self) -> Result<Self> {
        self.duplicates.validate().map(|duplicates| Filter {
            source: self.source,
            duplicates,
        })
    }
}

/// Source filter configuration is used to filter the compiler calls based on the source files.
///
/// Allow to filter the compiler calls based on the source files.
///
/// - Include only existing files: can be true or false.
/// - Paths to include: Only include the compiler calls that compiles source files from this path.
/// - Paths to exclude: Exclude the compiler calls that compiles source files from this path.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct SourceFilter {
    #[serde(default = "default_disabled")]
    pub include_only_existing_files: bool,
    #[serde(default)]
    pub paths_to_include: Vec<PathBuf>,
    #[serde(default)]
    pub paths_to_exclude: Vec<PathBuf>,
}

/// Duplicate filter configuration is used to filter the duplicate compiler calls.
///
/// - By fields: Specify the fields of the JSON compilation database record to detect duplicates.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct DuplicateFilter {
    pub by_fields: Vec<OutputFields>,
}

impl Validate for DuplicateFilter {
    /// Deduplicate the fields of the fields vector.
    fn validate(self) -> Result<Self> {
        let result = Self {
            by_fields: self
                .by_fields
                .iter()
                .cloned()
                .collect::<HashSet<_>>()
                .into_iter()
                .collect(),
        };
        Ok(result)
    }
}

impl Default for DuplicateFilter {
    fn default() -> Self {
        DuplicateFilter {
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
    #[serde(rename = "output")]
    Output,
}

/// Format configuration of the JSON compilation database.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Format {
    #[serde(default = "default_enabled")]
    pub command_as_array: bool,
    #[serde(default = "default_disabled")]
    pub drop_output_field: bool,
}

impl Default for Format {
    fn default() -> Self {
        Format {
            command_as_array: true,
            drop_output_field: false,
        }
    }
}

fn default_disabled() -> bool {
    false
}

fn default_enabled() -> bool {
    true
}

/// The default directory where the wrapper executables will be stored.
fn default_wrapper_directory() -> PathBuf {
    std::env::temp_dir()
}

/// The default path to the wrapper executable.
fn default_wrapper_executable() -> PathBuf {
    PathBuf::from(WRAPPER_EXECUTABLE_PATH)
}

/// The default path to the shared library that will be preloaded.
fn default_preload_library() -> PathBuf {
    PathBuf::from(PRELOAD_LIBRARY_PATH)
}

/// The default don't ignore the compiler.
fn default_never_ignore() -> Ignore {
    Ignore::Never
}

// Custom deserialization function to validate the schema version
fn validate_schema_version<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let schema: String = Deserialize::deserialize(deserializer)?;
    if schema != SUPPORTED_SCHEMA_VERSION {
        use serde::de::Error;
        Err(D::Error::custom(format!(
            "Unsupported schema version: {}. Expected: {}",
            schema, SUPPORTED_SCHEMA_VERSION
        )))
    } else {
        Ok(schema)
    }
}

/// A trait to validate the configuration and return a valid instance.
pub trait Validate {
    fn validate(self) -> Result<Self>
    where
        Self: Sized;
}

fn is_empty_path(path: &Path) -> bool {
    path.to_str().map_or(false, |p| p.is_empty())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{vec_of_pathbuf, vec_of_strings};

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
        output:
          specification: clang
          compilers:
            - path: /usr/local/bin/cc
              ignore: always
            - path: /usr/local/bin/c++
              ignore: conditional
              arguments:
                match:
                  - -###
            - path: /usr/local/bin/clang
              ignore: never
              arguments:
                add:
                  - -DDEBUG
                remove:
                  - -Wall
            - path: /usr/local/bin/clang++
              arguments:
                remove:
                  - -Wall
          filter:
            source:
              include_only_existing_files: true
              paths_to_include:
                - sources
              paths_to_exclude:
                - tests
            duplicates:
              by_fields:
                - file
                - directory
          format:
            command_as_array: true
            drop_output_field: false
        "#;

        let result = Main::from_reader(content).unwrap();

        let expected = Main {
            intercept: Intercept::Wrapper {
                path: default_wrapper_executable(),
                directory: PathBuf::from("/tmp"),
                executables: vec_of_pathbuf!["/usr/bin/cc", "/usr/bin/c++"],
            },
            output: Output::Clang {
                compilers: vec![
                    Compiler {
                        path: PathBuf::from("/usr/local/bin/cc"),
                        ignore: Ignore::Always,
                        arguments: Arguments::default(),
                    },
                    Compiler {
                        path: PathBuf::from("/usr/local/bin/c++"),
                        ignore: Ignore::Conditional,
                        arguments: Arguments {
                            match_: vec_of_strings!["-###"],
                            ..Default::default()
                        },
                    },
                    Compiler {
                        path: PathBuf::from("/usr/local/bin/clang"),
                        ignore: Ignore::Never,
                        arguments: Arguments {
                            add: vec_of_strings!["-DDEBUG"],
                            remove: vec_of_strings!["-Wall"],
                            ..Default::default()
                        },
                    },
                    Compiler {
                        path: PathBuf::from("/usr/local/bin/clang++"),
                        ignore: Ignore::Never,
                        arguments: Arguments {
                            remove: vec_of_strings!["-Wall"],
                            ..Default::default()
                        },
                    },
                ],
                filter: Filter {
                    source: SourceFilter {
                        include_only_existing_files: true,
                        paths_to_include: vec_of_pathbuf!["sources"],
                        paths_to_exclude: vec_of_pathbuf!["tests"],
                    },
                    duplicates: DuplicateFilter {
                        by_fields: vec![OutputFields::File, OutputFields::Directory],
                    },
                },
                format: Format {
                    command_as_array: true,
                    drop_output_field: false,
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
          filter:
            source:
              include_only_existing_files: true
            duplicates:
              by_fields:
                - file
                - directory
          format:
            command_as_array: true
        "#;

        let result = Main::from_reader(content).unwrap();

        let expected = Main {
            intercept: Intercept::Wrapper {
                path: default_wrapper_executable(),
                directory: default_wrapper_directory(),
                executables: vec_of_pathbuf!["/usr/bin/cc", "/usr/bin/c++"],
            },
            output: Output::Clang {
                compilers: vec![],
                filter: Filter {
                    source: SourceFilter {
                        include_only_existing_files: true,
                        paths_to_include: vec_of_pathbuf![],
                        paths_to_exclude: vec_of_pathbuf![],
                    },
                    duplicates: DuplicateFilter {
                        by_fields: vec![OutputFields::File, OutputFields::Directory],
                    },
                },
                format: Format {
                    command_as_array: true,
                    drop_output_field: false,
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

        let result = Main::from_reader(content).unwrap();

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
          filter:
            source:
              include_only_existing_files: false
            duplicates:
              by_fields:
                - file
          format:
            command_as_array: true
            drop_output_field: false
        "#;

        let result = Main::from_reader(content).unwrap();

        let expected = Main {
            intercept: Intercept::Preload {
                path: default_preload_library(),
            },
            output: Output::Clang {
                compilers: vec![
                    Compiler {
                        path: PathBuf::from("/usr/local/bin/cc"),
                        ignore: Ignore::Never,
                        arguments: Arguments::default(),
                    },
                    Compiler {
                        path: PathBuf::from("/usr/local/bin/c++"),
                        ignore: Ignore::Never,
                        arguments: Arguments::default(),
                    },
                    Compiler {
                        path: PathBuf::from("/usr/local/bin/clang"),
                        ignore: Ignore::Always,
                        arguments: Arguments::default(),
                    },
                    Compiler {
                        path: PathBuf::from("/usr/local/bin/clang++"),
                        ignore: Ignore::Always,
                        arguments: Arguments::default(),
                    },
                ],
                filter: Filter {
                    source: SourceFilter {
                        include_only_existing_files: false,
                        paths_to_include: vec_of_pathbuf![],
                        paths_to_exclude: vec_of_pathbuf![],
                    },
                    duplicates: DuplicateFilter {
                        by_fields: vec![OutputFields::File],
                    },
                },
                format: Format {
                    command_as_array: true,
                    drop_output_field: false,
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
                filter: Filter::default(),
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

        let result: serde_yml::Result<Main> = Main::from_reader(content);

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

        let result: serde_yml::Result<Main> = Main::from_reader(content);

        assert!(result.is_err());
    }
}
