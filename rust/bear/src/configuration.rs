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

use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};
use serde_json::de::Read;

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
///   compilers:
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
///     duplicate_filter_fields:
///       - file
///       - directory
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
///   filter:
///     compilers:
///       with_paths:
///         - /usr/local/bin/cc
///         - /usr/local/bin/c++
///     source:
///       include_only_existing_files: true
/// ```
#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Configuration {
    #[serde(deserialize_with = "validate_schema_version")]
    pub schema: String,
    #[serde(default)]
    pub intercept: Intercept,
    #[serde(default)]
    pub output: Output,
}

impl Configuration {
    pub fn from_file(file: &Path) -> Result<Self> {
        let reader = OpenOptions::new().read(true).open(file)
            .with_context(|| format!("Failed to open configuration file: {:?}", file))?;

        let content = Configuration::from_reader(reader)
            .with_context(|| format!("Failed to parse configuration from file: {:?}", file))?;

        Ok(content)
    }

    pub fn from_stdin() -> Result<Self> {
        let reader = std::io::stdin();
        let content = Configuration::from_reader(reader)
            .context("Failed to parse configuration from stdin")?;

        Ok(content)
    }

    fn from_reader<R, T>(rdr: R) -> serde_yml::Result<T>
    where
        R: std::io::Read,
        T: serde::de::DeserializeOwned,
    {
        serde_yml::from_reader(rdr)
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
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
/// and the list of compilers to wrap.
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
        compilers: Vec<PathBuf>,
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
            compilers: vec![],
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
        #[serde(default)]
        filter: Filter  // FIXME: should have its own type and limit the filtering options
    }
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
/// Allow to filter the compiler calls based on the compiler, source files, and duplicate fields.
///
/// - Compilers: Filter the compiler calls based on the compiler path and arguments.
/// - Source: Filter the compiler calls based on the source file location.
/// - Duplicate fields: Filter the compiler calls based on the output fields.
#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Filter {
    #[serde(default)]
    pub compilers: CompilerFilter,
    #[serde(default)]
    pub source: SourceFilter,
    #[serde(default)]
    pub duplicate_filter_fields: Vec<OutputFields>,
}

impl Default for Filter {
    fn default() -> Self {
        Filter {
            compilers: CompilerFilter::default(),
            source: SourceFilter::default(),
            duplicate_filter_fields: vec![OutputFields::File],
        }
    }
}

/// Compiler filter configuration is used to filter the compiler calls based on the compiler.
///
/// Allow to filter the compiler calls based on the compiler path and arguments.
///
/// - With paths: Filter the compiler calls based on the compiler path.
/// - With arguments: Filter the compiler calls based on the compiler arguments present.
#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
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
#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct SourceFilter {
    #[serde(default = "default_disabled")]
    pub include_only_existing_files: bool,
    #[serde(default)]
    pub paths_to_include: Vec<PathBuf>,
    #[serde(default)]
    pub paths_to_exclude: Vec<PathBuf>,
}

/// Represent the fields of the JSON compilation database record.
#[derive(Debug, PartialEq, Deserialize, Serialize)]
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
#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
struct Format {
    #[serde(default = "default_enabled")]
    command_as_array: bool,
    #[serde(default = "default_disabled")]
    drop_output_field: bool,
    // TODO: add support to customize the paths (absolute, relative or original)
}

fn default_disabled() -> bool {
    false
}

fn default_enabled() -> bool {
    true
}

fn default_schema_version() -> String {
    String::from(SUPPORTED_SCHEMA_VERSION)
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
          compilers:
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
            duplicate_filter_fields:
              - file
              - directory
          format:
            command_as_array: true
            drop_output_field: false
        "#;

        let result = Configuration::from_reader(content).unwrap();

        let expected = Configuration {
            intercept: Intercept::Wrapper {
                path: default_wrapper_executable(),
                directory: PathBuf::from("/tmp"),
                compilers: vec_of_pathbuf!["/usr/bin/cc", "/usr/bin/c++"],
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
                    duplicate_filter_fields: vec![OutputFields::File, OutputFields::Directory],
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
          filter:
            compilers:
              with_paths:
                - /usr/local/bin/cc
                - /usr/local/bin/c++
            source:
              include_only_existing_files: true
        "#;

        let result = Configuration::from_reader(content).unwrap();

        let expected = Configuration {
            intercept: Intercept::Preload {
                path: PathBuf::from("/usr/local/lib/libexec.so"),
            },
            output: Output::Semantic {
                filter: Filter {
                    compilers: CompilerFilter {
                        with_paths: vec_of_pathbuf!["/usr/local/bin/cc", "/usr/local/bin/c++"],
                        with_arguments: vec![],
                    },
                    source: SourceFilter {
                        include_only_existing_files: true,
                        paths_to_include: vec![],
                        paths_to_exclude: vec![],
                    },
                    duplicate_filter_fields: vec![],
                },
            },
            schema: String::from("4.0"),
        };

        assert_eq!(expected, result);
    }

    #[test]
    fn test_default_config() {
        let result = Configuration::default();

        let expected = Configuration {
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
          compilers:
            - /usr/bin/gcc
            - /usr/bin/g++
        "#;

        let result: serde_yml::Result<Configuration> = Configuration::from_reader(content);

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
                    "duplicate_filter_fields": "files"
                }
            }
        }"#;

        let result: serde_yml::Result<Configuration> = Configuration::from_reader(content);

        assert!(result.is_err());
    }
}