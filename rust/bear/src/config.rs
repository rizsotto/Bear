/*  Copyright (C) 2012-2024 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */
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
///
/// ```yaml
/// schema: 4.0
///
/// intercept:
///   mode: wrapper
///   directory: /tmp
///   executables:
///     - /usr/bin/cc
///     - /usr/bin/c++
/// output:
///   specification: clang
///   transform:
///     arguments_to_remove:
///       - -Wall
///     arguments_to_add:
///       - -DDEBUG
///   filter:
///     compilers:
///       with_arguments:
///         - -###
///       with_paths:
///         - /usr/local/bin/cc
///         - /usr/local/bin/c++
///     source:
///       include_only_existing_files: true
///       paths_to_include:
///         - sources
///       paths_to_exclude:
///         - tests
///     duplicates:
///       by_fields:
///         - file
///         - directory
///   format:
///     command_as_array: true
///     drop_output_field: false
/// ```
///
/// ```yaml
/// schema: 4.0
///
/// intercept:
///   mode: preload
/// output:
///   specification: bear
/// ```
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

        let reader = OpenOptions::new().read(true).open(file)
            .with_context(|| format!("Failed to open configuration file: {:?}", file))?;

        let content = Self::from_reader(reader)
            .with_context(|| format!("Failed to parse configuration from file: {:?}", file))?;

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

impl Default for Main {
    fn default() -> Self {
        Main {
            intercept: Intercept::default(),
            output: Output::default(),
            schema: String::from(SUPPORTED_SCHEMA_VERSION),
        }
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
        // TODO: add support for executables to recognize (as compiler)
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
            executables: vec![],  // FIXME: better default value
        }
    }
}

/// Output configuration is used to customize the output format.
///
/// Allow to customize the output format of the compiler calls.
///
/// - Clang: Output the compiler calls in the clang project defined "JSON compilation database"
/// format. (The format is used by clang tooling and other tools based on that library.)
/// - Semantic: Output the compiler calls in the semantic format. (The format is not defined yet.)
#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "specification")]
pub enum Output {
    #[serde(rename = "clang")]
    Clang {
        #[serde(default)]
        transform: Transform,
        #[serde(default)]
        filter: Filter,
        #[serde(default)]
        format: Format,
    },
    #[serde(rename = "bear")]
    Semantic {
    },
}

/// The default output is the clang format.
impl Default for Output {
    fn default() -> Self {
        Output::Clang {
            transform: Transform::default(),
            filter: Filter::default(),
            format: Format::default(),
        }
    }
}

/// Transform configuration is used to transform the compiler calls.
///
/// Allow to customize the transformation of the compiler calls.
///
/// - Add or remove arguments from the compiler calls.
#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Transform {
    #[serde(default)]
    pub arguments_to_add: Vec<String>,
    #[serde(default)]
    pub arguments_to_remove: Vec<String>,
}

/// Filter configuration is used to filter the compiler calls.
///
/// Allow to filter the compiler calls by compiler, source files and duplicates.
///
/// - Compilers: Specify on the compiler path and arguments.
/// - Source: Specify the source file location.
/// - Duplicates: Specify the fields of the JSON compilation database record to detect duplicates.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Filter {
    #[serde(default)]
    pub compilers: CompilerFilter,
    #[serde(default)]
    pub source: SourceFilter,
    #[serde(default)]
    pub duplicates: DuplicateFilter,
}

impl Default for Filter {
    fn default() -> Self {
        Filter {
            compilers: CompilerFilter::default(),
            source: SourceFilter::default(),
            duplicates: DuplicateFilter::default(),
        }
    }
}

/// Compiler filter configuration is used to filter the compiler calls based on the compiler.
///
/// Allow to filter the compiler calls based on the compiler path and arguments.
///
/// - With paths: Filter the compiler calls based on the compiler path.
/// - With arguments: Filter the compiler calls based on the compiler arguments present.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct CompilerFilter {
    #[serde(default)]
    pub with_paths: Vec<PathBuf>,
    #[serde(default)]
    pub with_arguments: Vec<String>,
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
    command_as_array: bool,
    #[serde(default = "default_disabled")]
    drop_output_field: bool,
    // TODO: add support to customize the paths (absolute, relative or original)
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
    PathBuf::from(PRELOAD_LIBRARY_PATH)
}

/// The default path to the shared library that will be preloaded.
fn default_preload_library() -> PathBuf {
    PathBuf::from(WRAPPER_EXECUTABLE_PATH)
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

#[cfg(test)]
mod test {
    use crate::{vec_of_pathbuf, vec_of_strings};
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
        output:
          specification: clang
          transform:
            arguments_to_remove:
              - -Wall
            arguments_to_add:
              - -DDEBUG
          filter:
            compilers:
              with_arguments:
                - -###
              with_paths:
                - /usr/local/bin/cc
                - /usr/local/bin/c++
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
                transform: Transform {
                    arguments_to_add: vec_of_strings!["-DDEBUG"],
                    arguments_to_remove: vec_of_strings!["-Wall"],
                },
                filter: Filter {
                    compilers: CompilerFilter {
                        with_paths: vec_of_pathbuf!["/usr/local/bin/cc", "/usr/local/bin/c++"],
                        with_arguments: vec_of_strings!["-###"],
                    },
                    source: SourceFilter {
                        include_only_existing_files: true,
                        paths_to_include: vec_of_pathbuf!["sources"],
                        paths_to_exclude: vec_of_pathbuf!["tests"],
                    },
                    duplicates: DuplicateFilter {
                        by_fields: vec![OutputFields::File, OutputFields::Directory],
                    }
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
            output: Output::Semantic {
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
                transform: Transform::default(),
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
        assert_eq!("Unsupported schema version: 3.0. Expected: 4.0 at line 2 column 9", message);
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