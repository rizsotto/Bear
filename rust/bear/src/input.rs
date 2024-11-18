// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context;
use serde_json::{Deserializer, Error, Value};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::BufReader;
use std::path::PathBuf;

use super::args;
use super::ipc::Execution;

/// Responsible for reading the build events from the intercept mode.
///
/// The file syntax is defined by the `events` module, and the parsing logic is implemented there.
/// Here we only handle the file opening and the error handling.
pub struct EventFileReader {
    reader: BufReader<File>,
}

impl TryFrom<args::BuildEvents> for EventFileReader {
    type Error = anyhow::Error;

    /// Open the file and create a new instance of the event file reader.
    ///
    /// If the file cannot be opened, the error will be logged and escalated.
    fn try_from(value: args::BuildEvents) -> Result<Self, Self::Error> {
        let file_name = PathBuf::from(value.file_name);
        let file = OpenOptions::new()
            .read(true)
            .open(file_name.as_path())
            .with_context(|| format!("Failed to open input file: {:?}", file_name))?;
        let reader = BufReader::new(file);

        Ok(EventFileReader { reader })
    }
}

impl EventFileReader {
    /// Generate the build events from the file.
    ///
    /// Returns an iterator over the build events. Any error during the reading
    /// of the file will be logged and the failed entries will be skipped.
    pub fn generate(self) -> impl Iterator<Item = Execution> {
        // Process the file line by line.
        from_reader(self.reader)
            // Log the errors and skip the failed entries.
            .flat_map(|candidate| match candidate {
                Ok(execution) => Some(execution),
                Err(error) => {
                    log::warn!("Failed to read entry from input: {}", error);
                    None
                }
            })
    }
}

// Based on stream serializer from `serde_json` crate.
//
//   https://docs.rs/serde_json/latest/serde_json/struct.StreamDeserializer.html
pub fn from_reader(reader: impl std::io::Read) -> impl Iterator<Item = Result<Execution, Error>> {
    Deserializer::from_reader(reader)
        .into_iter::<Value>()
        .flat_map(|value| match value {
            Ok(value) => into_execution(value).map(Ok),
            Err(error) => Some(Err(error)),
        })
}

fn into_execution(value: Value) -> Option<Execution> {
    value
        .get("started")
        .and_then(|started| started.get("execution"))
        .and_then(|execution| execution.as_object())
        .and_then(|map| {
            let executable = map
                .get("executable")
                .and_then(Value::as_str)
                .map(PathBuf::from);
            let arguments = map.get("arguments").and_then(Value::as_array).map(|vs| {
                vs.iter()
                    .flat_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<String>>()
            });
            let working_dir = map
                .get("working_dir")
                .and_then(Value::as_str)
                .map(PathBuf::from);
            let environment = map.get("environment").and_then(Value::as_object).map(|m| {
                m.iter()
                    .map(|kv| (kv.0.clone(), kv.1.as_str().unwrap().to_string()))
                    .collect::<HashMap<String, String>>()
            });

            if executable.is_some()
                && arguments.is_some()
                && working_dir.is_some()
                && environment.is_some()
            {
                Some(Execution {
                    executable: executable.unwrap(),
                    arguments: arguments.unwrap(),
                    working_dir: working_dir.unwrap(),
                    environment: environment.unwrap(),
                })
            } else {
                None
            }
        })
}

#[cfg(test)]
mod test {
    use crate::vec_of_strings;
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_reading_events() {
        let content = [
            into_single_line(
                r#"
            {
              "rid": "17014093296157802240",
              "started": {
                "execution": {
                  "executable": "/usr/bin/sh",
                  "arguments": [
                    "sh",
                    "-c",
                    "ls"
                  ],
                  "working_dir": "/var/home/lnagy/Code/Bear.git",
                  "environment": {
                    "COLORTERM": "truecolor",
                    "EDITOR": "/usr/bin/nano",
                    "USER": "lnagy",
                    "HOME": "/var/home/lnagy",
                    "LANG": "C.UTF-8",
                    "HOSTNAME": "tepsi",
                    "MAIL": "/var/spool/mail/lnagy"
                  }
                },
                "pid": 395760,
                "ppid": 395750
              },
              "timestamp": "2023-08-08T12:02:12.760865Z"
            }
            "#,
            ),
            into_single_line(
                r#"
            {
              "rid": "8533747834426684686",
              "started": {
                "execution": {
                  "executable": "/usr/bin/ls",
                  "arguments": [
                    "ls"
                  ],
                  "working_dir": "/var/home/lnagy/Code/Bear.git",
                  "environment": {
                    "COLORTERM": "truecolor",
                    "EDITOR": "/usr/bin/nano",
                    "USER": "lnagy",
                    "HOME": "/var/home/lnagy",
                    "LANG": "C.UTF-8",
                    "HOSTNAME": "tepsi",
                    "MAIL": "/var/spool/mail/lnagy"
                  }
                },
                "pid": 395764,
                "ppid": 395755
              },
              "timestamp": "2023-08-08T12:02:12.771258Z"
            }
            "#,
            ),
            into_single_line(
                r#"
            {
              "rid": "8533747834426684686",
              "terminated": {
                "status": "0"
              },
              "timestamp": "2023-08-08T12:02:12.772584Z"
            }
            "#,
            ),
            into_single_line(
                r#"
            {
              "rid": "17014093296157802240",
              "terminated": {
                "status": "0"
              },
              "timestamp": "2023-08-08T12:02:12.773568Z"
            }
            "#,
            ),
        ]
        .join("\n");

        let mut result = from_reader(content.as_bytes());

        let expected = Execution {
            executable: PathBuf::from("/usr/bin/sh"),
            arguments: vec_of_strings!["sh", "-c", "ls"],
            working_dir: PathBuf::from("/var/home/lnagy/Code/Bear.git"),
            environment: HashMap::from([
                ("COLORTERM".to_string(), "truecolor".to_string()),
                ("EDITOR".to_string(), "/usr/bin/nano".to_string()),
                ("USER".to_string(), "lnagy".to_string()),
                ("HOME".to_string(), "/var/home/lnagy".to_string()),
                ("LANG".to_string(), "C.UTF-8".to_string()),
                ("HOSTNAME".to_string(), "tepsi".to_string()),
                ("MAIL".to_string(), "/var/spool/mail/lnagy".to_string()),
            ]),
        };
        assert_eq!(expected, result.next().unwrap().unwrap());

        let expected = Execution {
            executable: PathBuf::from("/usr/bin/ls"),
            arguments: vec_of_strings!["ls"],
            working_dir: PathBuf::from("/var/home/lnagy/Code/Bear.git"),
            environment: HashMap::from([
                ("COLORTERM".to_string(), "truecolor".to_string()),
                ("EDITOR".to_string(), "/usr/bin/nano".to_string()),
                ("USER".to_string(), "lnagy".to_string()),
                ("HOME".to_string(), "/var/home/lnagy".to_string()),
                ("LANG".to_string(), "C.UTF-8".to_string()),
                ("HOSTNAME".to_string(), "tepsi".to_string()),
                ("MAIL".to_string(), "/var/spool/mail/lnagy".to_string()),
            ]),
        };
        assert_eq!(expected, result.next().unwrap().unwrap());

        assert!(result.next().is_none());
    }

    fn into_single_line(content: &str) -> String {
        content.chars().filter(|c| *c != '\n').collect()
    }
}
