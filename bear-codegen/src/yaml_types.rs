// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;

#[derive(Deserialize)]
pub struct FlagTable {
    pub extends: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub recognize: Option<Vec<RecognizeEntry>>,
    pub ignore_when: Option<IgnoreWhen>,
    /// When true, arguments starting with '/' are treated as flags (MSVC-style).
    #[serde(default)]
    pub slash_prefix: Option<bool>,
    pub flags: Vec<FlagEntry>,
    #[serde(default)]
    pub environment: Option<Vec<EnvEntry>>,
}

#[derive(Deserialize, Clone)]
pub struct RecognizeEntry {
    pub executables: Vec<String>,
    #[serde(default)]
    pub cross_compilation: bool,
    #[serde(default)]
    pub versioned: bool,
}

#[derive(Deserialize, Clone, Default)]
pub struct IgnoreWhen {
    #[serde(default)]
    pub executables: Vec<String>,
    #[serde(default)]
    pub flags: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct FlagEntry {
    #[serde(rename = "match")]
    pub match_: FlagMatch,
    pub result: String,
}

#[derive(Deserialize, Clone)]
pub struct FlagMatch {
    pub pattern: String,
    pub count: Option<u32>,
}

impl FlagMatch {
    /// Compute the flag name length as `FlagPattern::flag()` would return it.
    pub fn name_len(&self) -> usize {
        let pattern = &self.pattern;
        if let Some(flag) = pattern.strip_suffix("{ }*") {
            flag.len()
        } else if let Some(flag) = pattern.strip_suffix("{=}*") {
            flag.len()
        } else if let Some(flag) = pattern.strip_suffix("{:}*") {
            flag.len()
        } else if let Some(flag) = pattern.strip_suffix(":*") {
            flag.len()
        } else if let Some(flag) = pattern.strip_suffix("=*") {
            if self.count.is_some() {
                flag.len() + 1 // "=" is part of the flag name
            } else {
                flag.len()
            }
        } else if let Some(flag) = pattern.strip_suffix('*') {
            flag.len()
        } else {
            pattern.len()
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct EnvEntry {
    pub variable: String,
    pub effect: String,
    pub mapping: EnvMappingYaml,
    #[serde(default)]
    #[allow(dead_code)]
    pub note: Option<String>,
}

impl EnvEntry {
    /// Validate this environment entry against the schema.
    pub fn validate(&self, yaml_file: &str) -> Result<(), String> {
        let var_re = regex::Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap();
        if !var_re.is_match(&self.variable) {
            return Err(format!("{}: invalid environment variable name: '{}'", yaml_file, self.variable));
        }

        match self.effect.as_str() {
            "configures_preprocessing"
            | "configures_compiling"
            | "configures_assembling"
            | "configures_linking"
            | "stops_at_preprocessing"
            | "stops_at_compiling"
            | "stops_at_assembling"
            | "info_and_exit"
            | "driver_option"
            | "none" => {}
            other => return Err(format!("{}: unknown effect value: '{}'", yaml_file, other)),
        }

        let mapping = &self.mapping;
        if mapping.flag.is_some() && mapping.expand.is_some() {
            return Err(format!(
                "{}: environment entry '{}' has both 'flag' and 'expand'",
                yaml_file, self.variable
            ));
        }
        if mapping.flag.is_none() && mapping.expand.is_none() && self.effect != "none" {
            return Err(format!(
                "{}: environment entry '{}' has neither 'flag' nor 'expand' (and effect is not 'none')",
                yaml_file, self.variable
            ));
        }

        match mapping.separator.as_str() {
            "path" | "space" | ";" => {}
            other => {
                return Err(format!(
                    "{}: environment entry '{}' has unknown separator: '{}'",
                    yaml_file, self.variable, other
                ));
            }
        }

        if let Some(ref expand) = mapping.expand {
            match expand.as_str() {
                "prepend" | "append" => {}
                other => {
                    return Err(format!(
                        "{}: environment entry '{}' has unknown expand position: '{}'",
                        yaml_file, self.variable, other
                    ));
                }
            }
        }

        Ok(())
    }
}

#[derive(Deserialize, Clone)]
pub struct EnvMappingYaml {
    #[serde(default)]
    pub flag: Option<String>,
    #[serde(default)]
    pub expand: Option<String>,
    pub separator: String,
}

impl EnvMappingYaml {
    /// Convert this mapping to a Rust `EnvMapping` expression string.
    pub fn to_rust(&self) -> String {
        if let Some(ref flag) = self.flag {
            let sep = match self.separator.as_str() {
                "path" => "EnvSeparator::Path".to_string(),
                ";" => "EnvSeparator::Fixed(\";\")".to_string(),
                other => format!("EnvSeparator::Fixed(\"{}\")", other),
            };
            format!("EnvMapping::Flag {{ flag: \"{}\", separator: {} }}", flag, sep)
        } else if let Some(ref expand) = self.expand {
            let pos = match expand.as_str() {
                "prepend" => "EnvPosition::Prepend",
                "append" => "EnvPosition::Append",
                _ => unreachable!(),
            };
            format!("EnvMapping::Expand {{ position: {} }}", pos)
        } else {
            unreachable!()
        }
    }
}
