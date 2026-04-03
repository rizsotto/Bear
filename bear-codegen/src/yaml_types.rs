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

#[derive(Deserialize, Clone)]
pub struct EnvEntry {
    pub variable: String,
    pub effect: String,
    pub mapping: EnvMappingYaml,
    #[serde(default)]
    #[allow(dead_code)]
    pub note: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct EnvMappingYaml {
    #[serde(default)]
    pub flag: Option<String>,
    #[serde(default)]
    pub expand: Option<String>,
    pub separator: String,
}
