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
//!     - /usr/bin/clang
//!     - /usr/bin/clang++
//! output:
//!   specification: clang
//!   compilers:
//!     - path: /usr/local/bin/cc
//!       ignore: always
//!     - path: /usr/bin/cc
//!       ignore: never
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
//!     command_as_array: true
//!     drop_output_field: false
//!     paths_as: canonical
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

impl Validate for Output {
    /// Validate the configuration of the output writer.
    fn validate(self) -> Result<Self> {
        match self {
            Output::Clang {
                compilers,
                sources,
                duplicates,
                format,
            } => {
                let compilers = compilers.validate()?;
                let sources = sources.validate()?;
                let duplicates = duplicates.validate()?;
                Ok(Output::Clang {
                    compilers,
                    sources,
                    duplicates,
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
    #[serde(default)]
    pub ignore: IgnoreOrConsider,
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
            IgnoreOrConsider::Always if self.arguments != Arguments::default() => {
                anyhow::bail!(
                    "All arguments must be empty in always ignore mode. {:?}",
                    self.path
                );
            }
            IgnoreOrConsider::Conditional if self.arguments.match_.is_empty() => {
                anyhow::bail!(
                    "The match arguments cannot be empty in conditional ignore mode. {:?}",
                    self.path
                );
            }
            IgnoreOrConsider::Never if !self.arguments.match_.is_empty() => {
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
/// - Never: Never ignore the compiler call. (Default)
/// - Conditional: Ignore the compiler call if the arguments match.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum IgnoreOrConsider {
    #[serde(rename = "always", alias = "true")]
    Always,
    #[serde(rename = "never", alias = "false")]
    Never,
    #[serde(rename = "conditional")]
    Conditional,
}

/// The default ignore mode is never ignore.
impl Default for IgnoreOrConsider {
    fn default() -> Self {
        IgnoreOrConsider::Never
    }
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

/// Source filter configuration is used to filter the compiler calls based on the source files.
///
/// Allow to filter the compiler calls based on the source files.
///
/// - Include only existing files: can be true or false.
/// - List of directories to include or exclude.
///   (The order of these entries will imply the order of evaluation.)
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct SourceFilter {
    #[serde(default = "default_disabled")]
    pub only_existing_files: bool,
    #[serde(default)]
    pub paths: Vec<DirectoryFilter>,
}

impl Validate for SourceFilter {
    /// Fail when the same directory is in multiple times in the list.
    /// Otherwise, return the received source filter.
    fn validate(self) -> Result<Self> {
        let mut already_seen = HashSet::new();
        for directory in &self.paths {
            if !already_seen.insert(&directory.path) {
                anyhow::bail!("The directory {:?} is duplicated.", directory.path);
            }
        }
        Ok(self)
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
    #[serde(default)]
    pub paths_as: PathFormat,
}

impl Default for Format {
    fn default() -> Self {
        Format {
            command_as_array: true,
            drop_output_field: false,
            paths_as: PathFormat::default(),
        }
    }
}

/// Path format configuration describes how the paths should be formatted.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum PathFormat {
    #[serde(rename = "original", alias = "is")]
    Original,
    #[serde(rename = "absolute")]
    Absolute,
    #[serde(rename = "canonical")]
    Canonical,
}

/// The default path format is the original path.
impl Default for PathFormat {
    fn default() -> Self {
        PathFormat::Original
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
            command_as_array: true
            drop_output_field: false
            paths_as: canonical
        "#;

        let result = Main::from_reader(content).unwrap();

        let expected = Main {
            intercept: Intercept::Wrapper {
                path: default_wrapper_executable(),
                directory: PathBuf::from("/tmp"),
                executables: vec_of_pathbuf![
                    "/usr/bin/cc",
                    "/usr/bin/c++",
                    "/usr/bin/clang",
                    "/usr/bin/clang++"
                ],
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
                            match_: vec_of_strings!["-###"],
                            ..Default::default()
                        },
                    },
                    Compiler {
                        path: PathBuf::from("/usr/bin/clang"),
                        ignore: IgnoreOrConsider::Never,
                        arguments: Arguments {
                            add: vec_of_strings!["-DDEBUG"],
                            remove: vec_of_strings!["-Wall"],
                            ..Default::default()
                        },
                    },
                    Compiler {
                        path: PathBuf::from("/usr/bin/clang++"),
                        ignore: IgnoreOrConsider::Never,
                        arguments: Arguments {
                            remove: vec_of_strings!["-Wall"],
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
                    command_as_array: true,
                    drop_output_field: false,
                    paths_as: PathFormat::Canonical,
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
                sources: SourceFilter {
                    only_existing_files: true,
                    paths: vec![],
                },
                duplicates: DuplicateFilter {
                    by_fields: vec![OutputFields::File, OutputFields::Directory],
                },
                format: Format {
                    command_as_array: true,
                    drop_output_field: false,
                    paths_as: PathFormat::Original,
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
          sources:
            only_existing_files: false
          duplicates:
            by_fields:
              - file
          format:
            command_as_array: true
            drop_output_field: true
            use_absolute_path: false
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
                    command_as_array: true,
                    drop_output_field: true,
                    paths_as: PathFormat::Original,
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
                sources: SourceFilter::default(),
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
