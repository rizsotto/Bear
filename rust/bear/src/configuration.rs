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

/// Represents the application configuration.
///
/// ```yaml
/// schema: 4.0
///
/// intercept:
///   mode: wrapper
///   directory: /tmp
///   compilers:
///     - /usr/bin/gcc
///     - /usr/bin/g++
/// semantic:
///   compilers_to_recognize:
///     - path: /usr/bin/gcc
///       flags_to_remove:
///         - -Wall
///       flags_to_add:
///         - -DDEBUG
/// filter:
///   include_only_existing_source: true
///   duplicate_filter_fields: file
///   paths_to_include:
///     - sources
///   paths_to_exclude:
///     - tests
/// output:
///   format: clang
///   command_as_array: true
///   drop_output_field: false
/// ```
///
/// ```yaml
/// schema: 4.0
///
/// intercept:
///   mode: preload
/// semantic:
///   compilers_to_recognize:
///     - path: /usr/bin/gcc
///     - path: /usr/bin/g++
///   compilers_to_ignore:
///     - path: /usr/bin/clang
///       with_flags:
///         - -###
///     - path: /usr/bin/clang++
/// filter:
///   include_only_existing_source: true
/// output:
///   format: semantic
/// ```
#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Configuration {
    #[serde(default)]
    pub intercept: Intercept,
    #[serde(default)]
    pub semantic: Semantic,
    #[serde(default)]
    pub filter: Filter,
    #[serde(default)]
    pub output: Output,
    #[serde(deserialize_with = "validate_schema_version")]
    pub schema: String,
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
            semantic: Semantic::default(),
            filter: Filter::default(),
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
            directory: default_wrapper_directory(),
            compilers: vec![],
        }
    }
}

/// Semantic configuration is used to recognize the compiler calls.
///
/// Allow to customize the semantic analysis of the compiler calls.
#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Semantic {
    #[serde(default)]
    pub compilers_to_recognize: Vec<CompilerToRecognize>,
    #[serde(default)]
    pub compilers_to_ignore: Vec<CompilerToIgnore>,
}

/// Represents a compiler to recognize.
///
/// The compiler is identified by its path. And allow to customize the flags to add or remove from
/// the compiler call.
#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct CompilerToRecognize {
    pub path: PathBuf,
    #[serde(default)]
    pub flags_to_add: Vec<String>,
    #[serde(default)]
    pub flags_to_remove: Vec<String>,
}

/// Represents a compiler to ignore.
///
/// The compiler is identified by its path and if the compiler is called with specific flags.
#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct CompilerToIgnore {
    pub path: PathBuf,
    #[serde(default)]
    pub with_flags: Vec<String>,
}

/// Filter configuration is used to filter the compiler calls.
///
/// Allow to customize the filtering of the compiler calls.
///
/// - Filter the compiler calls based on the source file location and existence.
/// - Filter the compiler calls based on removing the duplicate entries from the output.
#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Filter {
    #[serde(default = "default_enabled")]
    pub include_only_existing_source: bool,
    #[serde(default)]
    pub paths_to_include: Vec<PathBuf>,
    #[serde(default)]
    pub paths_to_exclude: Vec<PathBuf>,
    #[serde(default)]
    pub duplicate_filter_fields: DuplicateFilterFields,
}

/// Represents how the duplicate filtering detects duplicate entries.
///
/// - FileOnly: Detects duplicates based on the source file.
/// - FileAndOutputOnly: Detects duplicates based on the source file and the output.
/// - All: Detects duplicates based on all arguments.
#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(try_from = "String")]
pub enum DuplicateFilterFields {
    FileOnly,
    #[default]
    FileAndOutputOnly,
    All,
}

impl TryFrom<String> for DuplicateFilterFields {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "file" =>
                Ok(DuplicateFilterFields::FileOnly),
            "file_output" =>
                Ok(DuplicateFilterFields::FileAndOutputOnly),
            "all" =>
                Ok(DuplicateFilterFields::All),
            _ =>
                Err(format!(r#"Unknown value "{value}" for duplicate filter"#)),
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
#[serde(tag = "format")]
pub enum Output {
    #[serde(rename = "clang")]
    Clang {
        #[serde(default = "default_enabled")]
        command_as_array: bool,
        #[serde(default = "default_disabled")]
        drop_output_field: bool,
    },
    #[serde(rename = "semantic")]
    Semantic,
}

/// The default output is the clang format.
impl Default for Output {
    fn default() -> Self {
        Output::Clang {
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

fn default_schema_version() -> String {
    String::from(SUPPORTED_SCHEMA_VERSION)
}

/// The default directory where the wrapper executables will be stored.
fn default_wrapper_directory() -> PathBuf {
    std::env::temp_dir()
}

/// The default path to the shared library that will be preloaded.
fn default_preload_library() -> PathBuf {
    PathBuf::from("/usr/lib/libexec.so")
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
            - /usr/bin/gcc
            - /usr/bin/g++
        semantic:
          compilers_to_recognize:
            - path: /usr/bin/gcc
              flags_to_remove:
                - -Wall
              flags_to_add:
                - -DDEBUG
        filter:
          include_only_existing_source: true
          duplicate_filter_fields: file
          paths_to_include:
            - sources
          paths_to_exclude:
            - tests
        output:
          format: clang
          command_as_array: true
          drop_output_field: false
        "#;

        let result = Configuration::from_reader(content).unwrap();

        let expected = Configuration {
            intercept: Intercept::Wrapper {
                directory: PathBuf::from("/tmp"),
                compilers: vec_of_pathbuf!["/usr/bin/gcc", "/usr/bin/g++"],
            },
            semantic: Semantic {
                compilers_to_recognize: vec![
                    CompilerToRecognize {
                        path: PathBuf::from("/usr/bin/gcc"),
                        flags_to_add: vec_of_strings!["-DDEBUG"],
                        flags_to_remove: vec_of_strings!["-Wall"],
                    },
                ],
                compilers_to_ignore: vec![],
            },
            filter: Filter {
                include_only_existing_source: true,
                paths_to_include: vec_of_pathbuf!["sources"],
                paths_to_exclude: vec_of_pathbuf!["tests"],
                duplicate_filter_fields: DuplicateFilterFields::FileOnly,
            },
            output: Output::Clang {
                command_as_array: true,
                drop_output_field: false,
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
        semantic:
          compilers_to_recognize:
            - path: /usr/bin/gcc
            - path: /usr/bin/g++
          compilers_to_ignore:
            - path: /usr/bin/clang
              with_flags:
                - -###
            - path: /usr/bin/clang++
        filter:
          include_only_existing_source: true
        output:
          format: semantic
        "#;

        let result = Configuration::from_reader(content).unwrap();

        let expected = Configuration {
            intercept: Intercept::Preload {
                path: PathBuf::from("/usr/local/lib/libexec.so"),
            },
            semantic: Semantic {
                compilers_to_recognize: vec![
                    CompilerToRecognize {
                        path: PathBuf::from("/usr/bin/gcc"),
                        flags_to_add: vec![],
                        flags_to_remove: vec![],
                    },
                    CompilerToRecognize {
                        path: PathBuf::from("/usr/bin/g++"),
                        flags_to_add: vec![],
                        flags_to_remove: vec![],
                    },
                ],
                compilers_to_ignore: vec![
                    CompilerToIgnore {
                        path: PathBuf::from("/usr/bin/clang"),
                        with_flags: vec_of_strings!["-###"],
                    },
                    CompilerToIgnore {
                        path: PathBuf::from("/usr/bin/clang++"),
                        with_flags: vec![],
                    },
                ],
            },
            filter: Filter {
                include_only_existing_source: true,
                paths_to_include: vec![],
                paths_to_exclude: vec![],
                duplicate_filter_fields: DuplicateFilterFields::FileAndOutputOnly,
            },
            output: Output::Semantic,
            schema: String::from("4.0"),
        };

        assert_eq!(expected, result);
    }

    #[test]
    fn test_default_config() {
        let result = Configuration::default();

        #[cfg(target_os = "linux")]
        let expected = Configuration {
            intercept: Intercept::Preload {
                path: PathBuf::from("/usr/lib/libexec.so"),
            },
            semantic: Semantic {
                compilers_to_recognize: vec![],
                compilers_to_ignore: vec![],
            },
            filter: Filter {
                include_only_existing_source: false,
                duplicate_filter_fields: DuplicateFilterFields::FileAndOutputOnly,
                paths_to_include: vec![],
                paths_to_exclude: vec![],
            },
            output: Output::Clang {
                command_as_array: true,
                drop_output_field: false,
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
        semantic:
          compilers_to_recognize:
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