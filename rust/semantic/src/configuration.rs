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

use std::path::PathBuf;

use serde::Deserialize;

// Represents the application configuration.
#[derive(Debug, Default, Deserialize, PartialEq)]
pub struct Configuration {
    #[serde(default)]
    pub output: Output,
    #[serde(default)]
    pub compilation: Compilation,
}

// Represents compiler related configuration.
#[derive(Debug, Default, Deserialize, PartialEq)]
pub struct Compilation {
    #[serde(default)]
    pub compilers_to_recognize: Vec<CompilerToRecognize>,
    #[serde(default)]
    pub compilers_to_exclude: Vec<PathBuf>,
}

// Represents a compiler wrapper that the tool will recognize.
//
// When executable name matches it tries to parse the flags as it would
// be a known compiler, and append the additional flags to the output
// entry if the compiler is recognized.
#[derive(Debug, Deserialize, PartialEq)]
pub struct CompilerToRecognize {
    pub executable: PathBuf,
    #[serde(default)]
    pub flags_to_add: Vec<String>,
    #[serde(default)]
    pub flags_to_remove: Vec<String>,
}

// Groups together the output related configurations.
#[derive(Debug, Default, Deserialize, PartialEq)]
pub struct Output {
    #[serde(default)]
    pub format: Format,
    #[serde(default)]
    pub content: Content,
}

// Controls the output format.
//
// The entries in the JSON compilation database can have different forms.
// One format element is how the command is represented: it can be an array
// of strings or a single string (shell escaping to protect white spaces).
// Another format element is if the output field is emitted or not.
#[derive(Debug, Deserialize, PartialEq)]
pub struct Format {
    #[serde(default = "enabled")]
    pub command_as_array: bool,
    #[serde(default = "disabled")]
    pub drop_output_field: bool,
}

impl Default for Format {
    fn default() -> Self {
        Format {
            command_as_array: enabled(),
            drop_output_field: disabled(),
        }
    }
}

// Controls the content of the output.
//
// This will act as a filter on the output elements.
// These attributes can be read from the configuration file, and can be
// overridden by command line arguments.
#[derive(Debug, Deserialize, PartialEq)]
pub struct Content {
    #[serde(default = "disabled")]
    pub include_only_existing_source: bool,
    #[serde(default)]
    pub duplicate_filter_fields: DuplicateFilterFields,
    #[serde(default)]
    pub paths_to_include: Vec<PathBuf>,
    #[serde(default)]
    pub paths_to_exclude: Vec<PathBuf>,
}

impl Default for Content {
    fn default() -> Self {
        Content {
            include_only_existing_source: disabled(),
            duplicate_filter_fields: DuplicateFilterFields::default(),
            paths_to_include: vec![],
            paths_to_exclude: vec![],
        }
    }
}

fn disabled() -> bool {
    false
}

fn enabled() -> bool {
    true
}

/// Represents how the duplicate filtering detects duplicate entries.
#[derive(Debug, Default, Deserialize, PartialEq)]
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

#[cfg(test)]
mod test {
    use crate::{vec_of_pathbuf, vec_of_strings};
    use super::*;

    #[test]
    fn test_full_config() {
        let content: &[u8] = br#"{
            "output": {
                "format": {
                    "command_as_array": true,
                    "drop_output_field": false
                },
                "content": {
                    "include_only_existing_source": false,
                    "duplicate_filter_fields": "all",
                    "paths_to_include": ["sources"],
                    "paths_to_exclude": ["tests"]
                }
            },
            "compilation": {
                "compilers_to_recognize": [
                    {
                        "executable": "/usr/local/bin/clang",
                        "flags_to_add": ["-Dfoo=bar"],
                        "flags_to_remove": ["-Wall"]
                    }
                ],
                "compilers_to_exclude": [
                    "clang"
                ]
            }
        }"#;

        let result = serde_json::from_reader(content).unwrap();

        let expected = Configuration {
            output: Output {
                format: Format {
                    command_as_array: true,
                    drop_output_field: false,
                },
                content: Content {
                    include_only_existing_source: false,
                    duplicate_filter_fields: DuplicateFilterFields::All,
                    paths_to_include: vec_of_pathbuf!["sources"],
                    paths_to_exclude: vec_of_pathbuf!["tests"],
                },
            },
            compilation: Compilation {
                compilers_to_recognize: vec![
                    CompilerToRecognize {
                        executable: PathBuf::from("/usr/local/bin/clang"),
                        flags_to_add: vec_of_strings!["-Dfoo=bar"],
                        flags_to_remove: vec_of_strings!["-Wall"],
                    }
                ],
                compilers_to_exclude: vec_of_pathbuf!["clang"],
            },
        };

        assert_eq!(expected, result);
    }

    #[test]
    fn test_only_output_config() {
        let content: &[u8] = br#"{
            "output": {
                "format": {
                    "command_as_array": false
                },
                "content": {
                    "duplicate_filter_fields": "file"
                }
            }
        }"#;

        let result = serde_json::from_reader(content).unwrap();

        let expected = Configuration {
            output: Output {
                format: Format {
                    command_as_array: false,
                    drop_output_field: false,
                },
                content: Content {
                    include_only_existing_source: false,
                    duplicate_filter_fields: DuplicateFilterFields::FileOnly,
                    paths_to_include: vec_of_pathbuf![],
                    paths_to_exclude: vec_of_pathbuf![],
                },
            },
            compilation: Compilation::default(),
        };

        assert_eq!(expected, result);
    }

    #[test]
    fn test_compilation_only_config() {
        let content: &[u8] = br#"{
            "compilation": {
                "compilers_to_recognize": [
                    {
                        "executable": "/usr/local/bin/clang"
                    },
                    {
                        "executable": "/usr/local/bin/clang++"
                    }
                ],
                "compilers_to_exclude": [
                    "clang", "clang++"
                ]
            }
        }"#;

        let result = serde_json::from_reader(content).unwrap();

        let expected = Configuration {
            output: Output::default(),
            compilation: Compilation {
                compilers_to_recognize: vec![
                    CompilerToRecognize {
                        executable: PathBuf::from("/usr/local/bin/clang"),
                        flags_to_add: vec![],
                        flags_to_remove: vec![],
                    },
                    CompilerToRecognize {
                        executable: PathBuf::from("/usr/local/bin/clang++"),
                        flags_to_add: vec![],
                        flags_to_remove: vec![],
                    },
                ],
                compilers_to_exclude: vec_of_pathbuf!["clang", "clang++"],
            },
        };

        assert_eq!(expected, result);
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

        let result: Result<Configuration, serde_json::Error> = serde_json::from_reader(content);

        assert!(result.is_err());

        let message = result.unwrap_err().to_string();
        assert_eq!("Unknown value \"files\" for duplicate filter at line 8 column 17", message);
    }
}