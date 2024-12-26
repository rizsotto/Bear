// SPDX-License-Identifier: GPL-3.0-or-later

//! Implements deserialization of the `Entry` struct.

use std::fmt;
use std::path;

use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};

use super::Entry;

impl<'de> Deserialize<'de> for Entry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Directory,
            File,
            Command,
            Arguments,
            Output,
        }
        const FIELDS: &[&str] = &["directory", "file", "command", "arguments", "output"];

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl Visitor<'_> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter
                            .write_str("`directory`, `file`, `command`, `arguments`, or `output`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "directory" => Ok(Field::Directory),
                            "file" => Ok(Field::File),
                            "command" => Ok(Field::Command),
                            "arguments" => Ok(Field::Arguments),
                            "output" => Ok(Field::Output),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct EntryVisitor;

        impl<'de> Visitor<'de> for EntryVisitor {
            type Value = Entry;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Entry")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Entry, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut directory: Option<path::PathBuf> = None;
                let mut file: Option<path::PathBuf> = None;
                let mut command: Option<String> = None;
                let mut arguments: Option<Vec<String>> = None;
                let mut output: Option<path::PathBuf> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Directory => {
                            if directory.is_some() {
                                return Err(de::Error::duplicate_field("directory"));
                            }
                            directory = Some(map.next_value()?);
                        }
                        Field::File => {
                            if file.is_some() {
                                return Err(de::Error::duplicate_field("file"));
                            }
                            file = Some(map.next_value()?);
                        }
                        Field::Command => {
                            if command.is_some() {
                                return Err(de::Error::duplicate_field("command"));
                            }
                            command = Some(map.next_value()?);
                        }
                        Field::Arguments => {
                            if arguments.is_some() {
                                return Err(de::Error::duplicate_field("arguments"));
                            }
                            arguments = Some(map.next_value()?);
                        }
                        Field::Output => {
                            if output.is_some() {
                                return Err(de::Error::duplicate_field("output"));
                            }
                            output = Some(map.next_value()?);
                        }
                    }
                }
                let directory = directory.ok_or_else(|| de::Error::missing_field("directory"))?;
                let file = file.ok_or_else(|| de::Error::missing_field("file"))?;
                if arguments.is_some() && command.is_some() {
                    return Err(de::Error::custom(
                        "Either `command` or `arguments` field need to be specified, but not both.",
                    ));
                }
                let arguments = arguments.map_or_else(
                    || {
                        command
                            .ok_or_else(|| de::Error::missing_field("`command` or `arguments`"))
                            .and_then(|cmd| {
                                shell_words::split(cmd.as_str()).map_err(|_| {
                                    de::Error::invalid_value(
                                        de::Unexpected::Str(cmd.as_str()),
                                        &"quotes needs to be matched",
                                    )
                                })
                            })
                    },
                    Ok,
                )?;
                Ok(Entry {
                    directory,
                    file,
                    arguments,
                    output,
                })
            }
        }

        deserializer.deserialize_struct("Entry", FIELDS, EntryVisitor)
    }
}
