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

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::{Deserializer, Error, Value};

use crate::execution::Execution;

// Based on stream serializer from `serde_json` crate.
//
//   https://docs.rs/serde_json/latest/serde_json/struct.StreamDeserializer.html
pub fn from_reader(reader: impl std::io::Read) -> impl Iterator<Item=Result<Execution, Error>> {
    Deserializer::from_reader(reader)
        .into_iter::<Value>()
        .flat_map(|value| {
            match value {
                Ok(value) =>
                    into_execution(value).map(Ok),
                Err(error) =>
                    Some(Err(error)),
            }
        })
}

fn into_execution(value: Value) -> Option<Execution> {
    value.get("started")
        .and_then(|started| started.get("execution"))
        .and_then(|execution| execution.as_object())
        .and_then(|map| {
            let executable = map.get("executable")
                .and_then(Value::as_str)
                .map(PathBuf::from);
            let arguments = map.get("arguments")
                .and_then(Value::as_array)
                .map(|vs| vs.iter()
                    .flat_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<String>>()
                );
            let working_dir = map.get("working_dir")
                .and_then(Value::as_str)
                .map(PathBuf::from);
            let environment = map.get("environment")
                .and_then(Value::as_object)
                .map(|m| m.iter()
                    .map(|kv| (kv.0.clone(), kv.1.as_str().unwrap().to_string()))
                    .collect::<HashMap<String, String>>()
                );

            if executable.is_some() && arguments.is_some() && working_dir.is_some() && environment.is_some() {
                Some(
                    Execution {
                        executable: executable.unwrap(),
                        arguments: arguments.unwrap(),
                        working_dir: working_dir.unwrap(),
                        environment: environment.unwrap(),
                    }
                )
            } else {
                None
            }
        })
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use crate::vec_of_strings;

    use super::*;

    #[test]
    fn test_reading_events() {
        let content = [into_single_line(r#"
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
            "#),
            into_single_line(r#"
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
            "#),
            into_single_line(r#"
            {
              "rid": "8533747834426684686",
              "terminated": {
                "status": "0"
              },
              "timestamp": "2023-08-08T12:02:12.772584Z"
            }
            "#),
            into_single_line(r#"
            {
              "rid": "17014093296157802240",
              "terminated": {
                "status": "0"
              },
              "timestamp": "2023-08-08T12:02:12.773568Z"
            }
            "#)]
            .join("\n");

        let mut result = from_reader(content.as_bytes());

        let expected = Execution {
            executable: PathBuf::from("/usr/bin/sh"),
            arguments: vec_of_strings![
                "sh",
                "-c",
                "ls"
            ],
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
