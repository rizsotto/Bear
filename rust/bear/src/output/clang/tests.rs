// SPDX-License-Identifier: GPL-3.0-or-later

#[cfg(test)]
mod failures {
    use super::super::*;
    use serde_json::error::Category;
    use serde_json::json;

    macro_rules! assert_semantic_error {
        ($x:expr) => {
            match $x {
                Some(Err(error)) => assert_eq!(error.classify(), Category::Data),
                _ => assert!(false, "shout be semantic error"),
            }
        };
    }

    #[test]
    fn load_non_json_content() {
        let content = r#"this is not json"#;
        let mut result = read(content.as_bytes());

        assert_semantic_error!(result.next());
        assert!(result.next().is_none());
    }

    #[test]
    fn load_not_expected_json_content() {
        let content = json!({ "file": "string" }).to_string();
        let mut result = read(content.as_bytes());

        assert_semantic_error!(result.next());
        assert!(result.next().is_none());
    }

    #[test]
    fn load_on_bad_value() {
        let content = json!([
            {
                "directory": " ",
                "file": "./file_a.c",
                "command": "cc -Dvalue=\"this"
            }
        ])
        .to_string();
        let mut result = read(content.as_bytes());

        assert_semantic_error!(result.next());
        assert!(result.next().is_none());
    }

    #[test]
    fn load_on_multiple_commands() {
        let content = json!([
            {
                "directory": " ",
                "file": "./file_a.c",
                "command": "cc source.c",
                "arguments": ["cc", "source.c"],
            }
        ])
        .to_string();
        let mut result = read(content.as_bytes());

        assert_semantic_error!(result.next());
        assert!(result.next().is_none());
    }
}

#[cfg(test)]
mod success {
    use super::super::*;
    use serde_json::json;

    mod empty {
        use super::*;

        #[test]
        fn load_empty_array() {
            let content = json!([]).to_string();

            let mut result = read(content.as_bytes());

            assert!(result.next().is_none());
        }
    }

    mod basic {
        use super::*;
        use crate::vec_of_strings;
        use serde_json::Value;
        use std::io::{Cursor, Seek, SeekFrom};

        fn expected_values() -> Vec<Entry> {
            vec![
                Entry {
                    directory: std::path::PathBuf::from("/home/user"),
                    file: std::path::PathBuf::from("./file_a.c"),
                    arguments: vec_of_strings!("cc", "-c", "./file_a.c", "-o", "./file_a.o"),
                    output: None,
                },
                Entry {
                    directory: std::path::PathBuf::from("/home/user"),
                    file: std::path::PathBuf::from("./file_b.c"),
                    arguments: vec_of_strings!("cc", "-c", "./file_b.c", "-o", "./file_b.o"),
                    output: Some(std::path::PathBuf::from("./file_b.o")),
                },
            ]
        }

        fn expected_with_array_syntax() -> serde_json::Value {
            json!([
                {
                    "directory": "/home/user",
                    "file": "./file_a.c",
                    "arguments": ["cc", "-c", "./file_a.c", "-o", "./file_a.o"]
                },
                {
                    "directory": "/home/user",
                    "file": "./file_b.c",
                    "output": "./file_b.o",
                    "arguments": ["cc", "-c", "./file_b.c", "-o", "./file_b.o"]
                }
            ])
        }

        fn expected_with_string_syntax() -> serde_json::Value {
            json!([
                {
                    "directory": "/home/user",
                    "file": "./file_a.c",
                    "command": "cc -c ./file_a.c -o ./file_a.o"
                },
                {
                    "directory": "/home/user",
                    "file": "./file_b.c",
                    "output": "./file_b.o",
                    "command": "cc -c ./file_b.c -o ./file_b.o"
                }
            ])
        }

        #[test]
        fn load_content_with_string_command_syntax() {
            let content = expected_with_string_syntax().to_string();

            let result = read(content.as_bytes());
            let entries: Vec<Entry> = result.map(|e| e.unwrap()).collect();

            assert_eq!(expected_values(), entries);
        }

        #[test]
        fn load_content_with_array_command_syntax() {
            let content = expected_with_array_syntax().to_string();

            let result = read(content.as_bytes());
            let entries: Vec<Entry> = result.map(|e| e.unwrap()).collect();

            assert_eq!(expected_values(), entries);
        }

        #[test]
        fn save_with_array_command_syntax() -> Result<(), Error> {
            let input = expected_values();

            // Create fake "file"
            let mut buffer = Cursor::new(Vec::new());
            let result = write(&mut buffer, input.into_iter());
            assert!(result.is_ok());

            // Use the fake "file" as input
            buffer.seek(SeekFrom::Start(0)).unwrap();
            let content: Value = serde_json::from_reader(&mut buffer)?;

            assert_eq!(expected_with_array_syntax(), content);

            Ok(())
        }
    }

    mod quoted {
        use super::*;
        use crate::vec_of_strings;
        use serde_json::Value;
        use std::io::{Cursor, Seek, SeekFrom};

        fn expected_values() -> Vec<Entry> {
            vec![
                Entry {
                    directory: std::path::PathBuf::from("/home/user"),
                    file: std::path::PathBuf::from("./file_a.c"),
                    arguments: vec_of_strings!(
                        "cc",
                        "-c",
                        "-D",
                        r#"name=\"me\""#,
                        "./file_a.c",
                        "-o",
                        "./file_a.o"
                    ),
                    output: None,
                },
                Entry {
                    directory: std::path::PathBuf::from("/home/user"),
                    file: std::path::PathBuf::from("./file_b.c"),
                    arguments: vec_of_strings!(
                        "cc",
                        "-c",
                        "-D",
                        r#"name="me""#,
                        "./file_b.c",
                        "-o",
                        "./file_b.o"
                    ),
                    output: None,
                },
            ]
        }

        fn expected_with_array_syntax() -> serde_json::Value {
            json!([
                {
                    "directory": "/home/user",
                    "file": "./file_a.c",
                    "arguments": ["cc", "-c", "-D", r#"name=\"me\""#, "./file_a.c", "-o", "./file_a.o"]
                },
                {
                    "directory": "/home/user",
                    "file": "./file_b.c",
                    "arguments": ["cc", "-c", "-D", r#"name="me""#, "./file_b.c", "-o", "./file_b.o"]
                }
            ])
        }

        #[test]
        fn load_content_with_array_command_syntax() {
            let content = expected_with_array_syntax().to_string();

            let result = read(content.as_bytes());
            let entries: Vec<Entry> = result.map(|e| e.unwrap()).collect();

            assert_eq!(expected_values(), entries);
        }

        #[test]
        fn save_with_array_command_syntax() -> Result<(), Error> {
            let input = expected_values();

            // Create fake "file"
            let mut buffer = Cursor::new(Vec::new());
            let result = write(&mut buffer, input.into_iter());
            assert!(result.is_ok());

            // Use the fake "file" as input
            buffer.seek(SeekFrom::Start(0)).unwrap();
            let content: Value = serde_json::from_reader(&mut buffer)?;

            assert_eq!(expected_with_array_syntax(), content);

            Ok(())
        }
    }
}
