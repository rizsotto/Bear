// SPDX-License-Identifier: GPL-3.0-or-later

//! Implements deserialization of the `Entry` struct.

use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use std::fmt;
use std::path;

use super::Entry;

impl<'de> Deserialize<'de> for Entry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("Entry", FIELDS, EntryVisitor)
    }
}

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
        deserializer.deserialize_identifier(FieldVisitor)
    }
}

struct FieldVisitor;

impl Visitor<'_> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "one of {:?}", FIELDS)
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

struct EntryVisitor;

impl<'de> Visitor<'de> for EntryVisitor {
    type Value = Entry;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("object Entry")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Entry, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut directory_opt: Option<path::PathBuf> = None;
        let mut file_opt: Option<path::PathBuf> = None;
        let mut command_opt: Option<String> = None;
        let mut arguments_opt: Option<Vec<String>> = None;
        let mut output: Option<path::PathBuf> = None;

        while let Some(key) = map.next_key()? {
            match key {
                Field::Directory => directory_opt = Some(map.next_value()?),
                Field::File => file_opt = Some(map.next_value()?),
                Field::Command => command_opt = Some(map.next_value()?),
                Field::Arguments => arguments_opt = Some(map.next_value()?),
                Field::Output => output = Some(map.next_value()?),
            }
        }

        // Validate if the mandatory fields are present.
        let arguments = match (arguments_opt, command_opt) {
            (None, None) => Err(de::Error::missing_field("`command` or `arguments`")),
            (Some(_), Some(_)) => Err(de::Error::custom(
                "Either `command` or `arguments` field need to be specified, but not both.",
            )),
            (Some(args), None) => Ok(args),
            (None, Some(cmd)) => shell_words::split(cmd.as_str()).map_err(|_| {
                de::Error::invalid_value(
                    de::Unexpected::Str(cmd.as_str()),
                    &"valid shell command with proper escaping",
                )
            }),
        }?;

        Ok(Entry {
            directory: directory_opt.ok_or_else(|| de::Error::missing_field("directory"))?,
            file: file_opt.ok_or_else(|| de::Error::missing_field("file"))?,
            arguments,
            output,
        })
    }
}
