// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// Which schema a YAML file follows. Closed: `compiler` is a compiler
/// family (`FlagTable`, registered in `TABLES`); `wrapper` is a compiler
/// launcher (`WrapperTable`, discovered by kind, no `TABLES` entry).
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Compiler,
    Wrapper,
}

/// The family identity of a compiler table: a unique `id` (the `extends:`
/// target, emitted into `RECOGNITION_PATTERNS`, and mirrors the config
/// `as:` value) and an optional `extends:` reference to another table's id.
#[derive(Deserialize, Clone)]
pub struct CompilerIdentity {
    pub id: String,
    #[serde(default)]
    pub extends: Option<String>,
}

#[derive(Deserialize)]
pub struct FlagTable {
    #[serde(rename = "type")]
    pub type_: Kind,
    pub compiler: CompilerIdentity,
    pub recognize: Option<Vec<RecognizeEntry>>,
    pub ignore_when: Option<IgnoreWhen>,
    /// When true, arguments starting with '/' are treated as flags (MSVC-style).
    #[serde(default)]
    pub slash_prefix: Option<bool>,
    pub flags: Vec<FlagEntry>,
    #[serde(default)]
    pub environment: Option<Vec<EnvEntry>>,
}

/// A compiler-launcher YAML file (`ccache.yaml`, `distcc.yaml`, ...):
/// no identity block, no flags -- just what it is recognized as and,
/// optionally, its own argv options to skip before the real compiler.
#[derive(Deserialize)]
pub struct WrapperTable {
    #[serde(rename = "type")]
    pub type_: Kind,
    pub recognize: Vec<RecognizeEntry>,
    #[serde(default)]
    pub options: Vec<WrapperOption>,
}

impl WrapperTable {
    /// Validate a wrapper table: every `recognize` entry passes the usual
    /// description/references check and has `versioned`/`cross_compilation`
    /// both false (a launcher basename is matched exactly, never version-
    /// suffixed or treated as a cross-compilation prefix); every `options`
    /// entry's pattern is an exact token, not a glued/prefix/eq/colon form
    /// (the skip loop only compares argv tokens for equality).
    pub fn validate(&self, yaml_file: &str) -> Result<()> {
        for entry in &self.recognize {
            entry.validate().with_context(|| format!("recognize entry in {}", yaml_file))?;
            if entry.versioned {
                bail!(
                    "{}: recognize entry {:?} must not set versioned: true for a wrapper",
                    yaml_file,
                    entry.executables
                );
            }
            if entry.cross_compilation {
                bail!(
                    "{}: recognize entry {:?} must not set cross_compilation: true for a wrapper",
                    yaml_file,
                    entry.executables
                );
            }
        }
        for option in &self.options {
            let pattern = &option.match_.pattern;
            if pattern.contains('*') || pattern.contains('{') {
                bail!(
                    "{}: wrapper option pattern '{}' must be an exact token (no '*' or '{{ }}' forms)",
                    yaml_file,
                    pattern
                );
            }
        }
        Ok(())
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct WrapperOption {
    #[serde(rename = "match")]
    pub match_: FlagMatch,
}

#[derive(Deserialize, Clone)]
pub struct RecognizeEntry {
    pub description: String,
    pub references: Vec<String>,
    pub executables: Vec<String>,
    #[serde(default)]
    pub versioned: bool,
    #[serde(default)]
    pub cross_compilation: bool,
}

impl RecognizeEntry {
    /// Validate the enrichment fields: a non-empty description and at
    /// least one http(s) reference URL. Enforced at codegen time so a
    /// compiler cannot be added without describing and citing it.
    pub fn validate(&self) -> Result<()> {
        if self.description.trim().is_empty() {
            bail!("recognize entry {:?}: description must not be empty", self.executables);
        }
        if self.references.is_empty() {
            bail!("recognize entry {:?}: references must list at least one URL", self.executables);
        }
        for url in &self.references {
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                bail!("recognize entry {:?}: reference '{}' must be an http(s) URL", self.executables, url);
            }
        }
        Ok(())
    }
}

#[derive(Deserialize, Clone, Default)]
pub struct IgnoreWhen {
    #[serde(default)]
    pub executables: Vec<String>,
    #[serde(default)]
    pub flags: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct FlagEntry {
    #[serde(rename = "match")]
    pub match_: FlagMatch,
    pub result: String,
}

#[derive(Deserialize, Clone, Debug)]
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
            if self.count.is_some() { flag.len() + 1 } else { flag.len() }
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

/// Validate that `s` is a C-style identifier:
/// non-empty, first char is ASCII letter or `_`, remaining chars
/// are ASCII alphanumeric or `_`.
fn is_valid_var_name(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl EnvEntry {
    /// Validate this environment entry against the schema.
    pub fn validate(&self) -> Result<()> {
        if !is_valid_var_name(&self.variable) {
            bail!("invalid environment variable name '{}'", self.variable);
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
            other => bail!("variable '{}': unknown effect '{}'", self.variable, other),
        }

        let mapping = &self.mapping;
        if mapping.flag.is_some() && mapping.expand.is_some() {
            bail!("variable '{}': has both 'flag' and 'expand'", self.variable);
        }
        if mapping.flag.is_none() && mapping.expand.is_none() && self.effect != "none" {
            bail!("variable '{}': has neither 'flag' nor 'expand' (and effect is not 'none')", self.variable);
        }

        match mapping.separator.as_str() {
            "path" | "space" | ";" => {}
            other => bail!("variable '{}': unknown separator '{}'", self.variable, other),
        }

        if let Some(ref expand) = mapping.expand {
            match expand.as_str() {
                "prepend" | "append" => {}
                other => bail!("variable '{}': unknown expand position '{}'", self.variable, other),
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
    pub fn to_rust(&self) -> Result<String> {
        if let Some(ref flag) = self.flag {
            let sep = match self.separator.as_str() {
                "path" => "EnvSeparator::Path".to_string(),
                ";" => "EnvSeparator::Fixed(\";\")".to_string(),
                other => format!("EnvSeparator::Fixed(\"{}\")", other),
            };
            Ok(format!("EnvMapping::Flag {{ flag: \"{}\", separator: {} }}", flag, sep))
        } else if let Some(ref expand) = self.expand {
            let pos = match expand.as_str() {
                "prepend" => "EnvPosition::Prepend",
                "append" => "EnvPosition::Append",
                other => bail!("unknown expand position '{}'", other),
            };
            Ok(format!("EnvMapping::Expand {{ position: {} }}", pos))
        } else {
            bail!("mapping has neither 'flag' nor 'expand'")
        }
    }
}
