// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;
use std::fmt;
use std::path::PathBuf;

/// Represents the application configuration with flattened structure.
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Main {
    #[serde(deserialize_with = "validate_schema_version")]
    pub schema: String,
    #[serde(default)]
    pub intercept: Intercept,
    #[serde(default)]
    pub compilers: Vec<Compiler>,
    #[serde(default)]
    pub sources: SourceFilter,
    #[serde(default)]
    pub duplicates: DuplicateFilter,
    #[serde(default)]
    pub format: Format,
}

impl Default for Main {
    fn default() -> Self {
        Self {
            schema: String::from(SUPPORTED_SCHEMA_VERSION),
            intercept: Intercept::default(),
            compilers: vec![],
            sources: SourceFilter::default(),
            duplicates: DuplicateFilter::default(),
            format: Format::default(),
        }
    }
}

impl fmt::Display for Main {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Configuration:")?;
        match serde_saphyr::to_string(self) {
            Ok(yaml_string) => {
                for line in yaml_string.lines() {
                    writeln!(f, "{}", line)?;
                }
                Ok(())
            }
            Err(_) => Err(fmt::Error),
        }
    }
}

/// Simplified intercept configuration with mode.
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "mode")]
pub enum Intercept {
    #[serde(rename = "wrapper")]
    Wrapper,
    #[serde(rename = "preload")]
    Preload,
}

/// The default intercept mode is varying based on the target operating system.
impl Default for Intercept {
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "windows"))]
    fn default() -> Self {
        Intercept::Wrapper
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "windows")))]
    fn default() -> Self {
        Intercept::Preload
    }
}

/// Represents compiler configuration matching the YAML format.
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Compiler {
    pub path: PathBuf,
    #[serde(rename = "as", skip_serializing_if = "Option::is_none")]
    pub as_: Option<CompilerType>,
    #[serde(default)]
    pub ignore: bool,
}

/// Compiler types that we can recognize and configure
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CompilerType {
    #[serde(alias = "gcc", alias = "gnu")]
    Gcc,
    #[serde(alias = "clang", alias = "llvm")]
    Clang,
    #[serde(alias = "fortran", alias = "gfortran", alias = "flang")]
    Flang,
    #[serde(alias = "ifort", alias = "intel-fortran", alias = "intel_fortran")]
    IntelFortran,
    #[serde(alias = "crayftn", alias = "cray-fortran", alias = "cray_fortran")]
    CrayFortran,
    #[serde(alias = "nvcc", alias = "cuda")]
    Cuda,
    #[serde(alias = "ccache", alias = "distcc", alias = "sccache")]
    Wrapper,
}

impl std::fmt::Display for CompilerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            CompilerType::Gcc => "GCC",
            CompilerType::Clang => "Clang",
            CompilerType::Flang => "Flang",
            CompilerType::IntelFortran => "Intel Fortran",
            CompilerType::CrayFortran => "Cray Fortran",
            CompilerType::Cuda => "CUDA",
            CompilerType::Wrapper => "Wrapper",
        };
        write!(f, "{}", name)
    }
}

/// Action to take for files matching a directory rule
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DirectoryAction {
    Include,
    Exclude,
}

/// A rule that specifies how to handle files within a directory
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DirectoryRule {
    pub path: PathBuf,
    pub action: DirectoryAction,
}

/// Source filter configuration for controlling which files are included in the compilation database.
///
/// Uses directory-based rules with order-based evaluation semantics:
///
/// 1. **Order-based evaluation**: For each source file, the *last* rule whose path prefix
///    matches determines inclusion/exclusion.
/// 2. **Empty directories list**: Interpreted as "include everything" (no filtering).
/// 3. **No-match behavior**: If no rule matches a file, the file is *included*.
/// 4. **Path matching**: Simple prefix matching, no normalization.
/// 5. **Case sensitivity**: Always case-sensitive on all platforms.
/// 6. **Path separators**: Platform-specific (`/` on Unix, `\` on Windows).
/// 7. **Symlinks**: No symlink resolution — match literal paths only.
/// 8. **Directory matching**: A rule matches both files directly in the directory and files in subdirectories.
/// 9. **Empty path fields**: Invalid — validation must fail.
///
/// **Important**: For matching to work correctly, rule paths should use the same format as
/// configured in `format.paths.file`. This consistency is the user's responsibility.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SourceFilter {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub directories: Vec<DirectoryRule>,
}

/// Duplicate filter configuration matching the YAML format.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DuplicateFilter {
    pub match_on: Vec<OutputFields>,
}

impl Default for DuplicateFilter {
    fn default() -> Self {
        Self { match_on: vec![OutputFields::Directory, OutputFields::File, OutputFields::Arguments] }
    }
}

/// Represent the fields of the JSON compilation database record.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum OutputFields {
    #[serde(rename = "directory")]
    Directory,
    #[serde(rename = "file")]
    File,
    #[serde(rename = "arguments")]
    Arguments,
    #[serde(rename = "command")]
    Command,
    #[serde(rename = "output")]
    Output,
}

/// Format configuration matching the YAML format.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Format {
    #[serde(default)]
    pub paths: PathFormat,
    #[serde(default)]
    pub entries: EntryFormat,
}

/// Format configuration of paths in the JSON compilation database.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PathFormat {
    #[serde(default)]
    pub directory: PathResolver,
    #[serde(default)]
    pub file: PathResolver,
}

/// Path resolver options matching the YAML format.
#[derive(Copy, Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PathResolver {
    /// Leave the path as is without any transformation. (Default)
    #[default]
    #[serde(rename = "as-is")]
    AsIs,
    /// The path will be resolved to the canonical path.
    #[serde(rename = "canonical")]
    Canonical,
    /// The path will be resolved to the relative path to the directory attribute.
    #[serde(rename = "relative")]
    Relative,
    /// The path will be resolved to an absolute path.
    #[serde(rename = "absolute")]
    Absolute,
}

/// Configuration for formatting output entries matching the YAML format.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct EntryFormat {
    #[serde(default = "default_enabled")]
    pub use_array_format: bool,
    #[serde(default = "default_enabled")]
    pub include_output_field: bool,
}

impl Default for EntryFormat {
    fn default() -> Self {
        Self { use_array_format: true, include_output_field: true }
    }
}

pub(crate) const SUPPORTED_SCHEMA_VERSION: &str = "4.1";

fn default_enabled() -> bool {
    true
}

// Custom deserialization function to validate the schema version
fn validate_schema_version<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let schema: String = Deserialize::deserialize(deserializer)?;
    if schema != SUPPORTED_SCHEMA_VERSION {
        use serde::de::Error;
        Err(Error::custom(format!(
            "Unsupported schema version: {schema}. Expected: {SUPPORTED_SCHEMA_VERSION}"
        )))
    } else {
        Ok(schema)
    }
}
