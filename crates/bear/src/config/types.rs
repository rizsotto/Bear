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
    #[serde(default)]
    pub headers: Headers,
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
            headers: Headers::default(),
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

/// The interception configuration: the mechanism and the report socket.
#[derive(Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Intercept {
    #[serde(default)]
    pub mode: InterceptMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket: Option<Socket>,
}

impl Intercept {
    /// The `socket` section, or `None` when it is absent or has no key.
    ///
    /// An empty section counts as absent, so it does not turn off the
    /// fallback to the loopback interface.
    pub fn effective_socket(&self) -> Option<&Socket> {
        self.socket.as_ref().filter(|socket| socket.dir.is_some() || socket.access.is_some())
    }
}

/// How the interception is injected into the build.
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum InterceptMode {
    #[serde(rename = "wrapper")]
    Wrapper,
    #[serde(rename = "preload")]
    Preload,
}

/// The default intercept mode is varying based on the target operating system.
impl Default for InterceptMode {
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "windows"))]
    fn default() -> Self {
        InterceptMode::Wrapper
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "windows")))]
    fn default() -> Self {
        InterceptMode::Preload
    }
}

/// The `intercept.socket` section: where the report socket lives and who may use it.
#[derive(Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Socket {
    /// An existing absolute directory. Bear makes its own directory inside it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access: Option<SocketAccess>,
}

/// Who may connect to the report socket.
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SocketAccess {
    /// The user who runs Bear.
    Owner,
    /// Every user.
    All,
}

/// Represents compiler configuration matching the YAML format.
///
/// `as_` holds the spelling the user wrote, verbatim. Resolving it onto a
/// compiler family is semantic analysis, not schema validation, so it happens
/// in the wiring layer (`modes`) against the semantic hint builder.
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Compiler {
    pub path: PathBuf,
    #[serde(rename = "as", skip_serializing_if = "Option::is_none")]
    pub as_: Option<String>,
    #[serde(default)]
    pub ignore: bool,
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

/// A rule that specifies how to handle files matching a filename glob pattern.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct FileRule {
    pub pattern: String,
    pub action: DirectoryAction,
}

/// Source filter configuration for controlling which files are included in the compilation database.
///
/// Uses directory-based rules and filename-pattern rules with order-based evaluation semantics:
///
/// 1. **Order-based evaluation**: For each source file, the *last* rule whose path prefix
///    matches determines inclusion/exclusion. Filename-pattern rules are evaluated the same
///    way, independently: the *last* rule whose glob matches determines inclusion/exclusion.
/// 2. **Empty directories/files list**: Interpreted as "include everything" (no filtering) for
///    that list.
/// 3. **No-match behavior**: If no rule matches a file, the file is *included*.
/// 4. **Path matching**: Simple prefix matching for directory rules, no normalization.
/// 5. **Case sensitivity**: Always case-sensitive on all platforms.
/// 6. **Path separators**: Platform-specific (`/` on Unix, `\` on Windows).
/// 7. **Symlinks**: No symlink resolution — match literal paths only.
/// 8. **Directory matching**: A rule matches both files directly in the directory and files in subdirectories.
/// 9. **Empty path/pattern fields**: Invalid — validation must fail.
/// 10. **Filename-pattern matching**: A pattern without a path separator matches the source
///     file's basename; a pattern containing a separator matches the full source path as it
///     appears in the entry.
/// 11. **Composition**: An entry is emitted only when both the directory rules and the
///     filename-pattern rules accept it (logical AND of the two independent verdicts).
///
/// **Important**: For matching to work correctly, rule paths/patterns should use the same
/// format as configured in `format.paths.file`. This consistency is the user's responsibility.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SourceFilter {
    /// Drop entries whose source file is known to be missing from the file system.
    /// Enabled by default.
    #[serde(default = "default_enabled")]
    pub only_existing_files: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub directories: Vec<DirectoryRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FileRule>,
}

impl Default for SourceFilter {
    fn default() -> Self {
        Self { only_existing_files: true, directories: vec![], files: vec![] }
    }
}

/// Duplicate filter configuration matching the YAML format.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DuplicateFilter {
    pub match_on: Vec<OutputFields>,
}

impl Default for DuplicateFilter {
    fn default() -> Self {
        Self { match_on: vec![OutputFields::Directory, OutputFields::File] }
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
    #[serde(default)]
    pub arguments: ArgumentsFormat,
}

/// Controls how the `arguments` field of each entry is assembled during
/// semantic analysis.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ArgumentsFormat {
    /// Replace `@file` response-file references with their tokenized contents.
    /// Unset means on, except off when the input comes from another machine.
    /// Off records an `@file` argument verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_response_files: Option<bool>,
    /// Fold compiler environment variables (e.g. `CPATH`) into each entry's
    /// arguments as explicit flags. Enabled by default.
    #[serde(default = "default_enabled")]
    pub from_environment: bool,
}

impl Default for ArgumentsFormat {
    fn default() -> Self {
        Self { from_response_files: None, from_environment: true }
    }
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

/// Configuration for synthesizing compilation database entries for header files.
///
/// Off by default: with `enabled: false` the output is unchanged. When enabled,
/// `strategy` selects how header files are discovered and which translation unit
/// donates the compile flags. Which file extensions count as headers is fixed
/// (a built-in C-family header set), not configurable here.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Headers {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub strategy: HeaderStrategy,
}

/// Strategy for discovering header files and their donor translation unit.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HeaderStrategy {
    /// Clone from a compiled source in the same directory as the header.
    #[default]
    Siblings,
    /// Read the dependency files the build already emitted.
    DependencyFiles,
}

pub(crate) const SUPPORTED_SCHEMA_VERSION: &str = "4.2";

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_strategy_default_is_siblings() {
        let sut = HeaderStrategy::default();

        assert_eq!(sut, HeaderStrategy::Siblings);
    }

    #[test]
    fn test_headers_default_is_disabled_with_siblings_strategy() {
        let sut = Headers::default();

        assert!(!sut.enabled, "headers should be disabled by default");
        assert_eq!(sut.strategy, HeaderStrategy::Siblings);
    }

    #[test]
    fn test_header_strategy_deserializes_kebab_case_variants() {
        let cases =
            [("siblings", HeaderStrategy::Siblings), ("dependency-files", HeaderStrategy::DependencyFiles)];

        for (input, expected) in cases {
            let yaml = format!("strategy: {input}");

            let sut: Headers = serde_saphyr::from_str(&yaml).unwrap();

            assert_eq!(sut.strategy, expected, "case: strategy: {input}");
        }
    }

    #[test]
    fn test_header_strategy_rejects_unknown_variant() {
        let yaml = "strategy: unknown";

        let sut: Result<Headers, _> = serde_saphyr::from_str(yaml);

        assert!(sut.is_err(), "expected deserialization to fail for: {yaml}");
    }

    #[test]
    fn compiler_as_spelling_is_kept_verbatim() {
        // The schema layer validates nothing about `as:`: it round-trips the
        // spelling the user wrote, whatever it is.
        let cases = ["gcc", "ccache", "not-a-compiler"];

        for spelling in cases {
            let yaml = format!("path: /usr/bin/cc\nas: {spelling}");

            let sut: Compiler = serde_saphyr::from_str(&yaml).unwrap();

            assert_eq!(sut.as_.as_deref(), Some(spelling), "case: as: {spelling}");
        }
    }

    fn parse_intercept(yaml: &str) -> Intercept {
        serde_saphyr::from_str(yaml).unwrap()
    }

    // Requirements: interception-report-channel
    #[test]
    fn test_intercept_without_socket_parses() {
        let cases = [("mode: wrapper", InterceptMode::Wrapper), ("mode: preload", InterceptMode::Preload)];

        for (yaml, mode) in cases {
            let sut = parse_intercept(yaml);

            assert_eq!(sut, Intercept { mode, socket: None }, "case: {yaml}");
        }
    }

    // Requirements: interception-report-channel
    #[test]
    fn test_intercept_socket_keys_parse() {
        let cases = [
            ("dir: /run/build", Some(PathBuf::from("/run/build")), None),
            ("access: owner", None, Some(SocketAccess::Owner)),
            ("access: all", None, Some(SocketAccess::All)),
            ("dir: /run/build\n    access: all", Some(PathBuf::from("/run/build")), Some(SocketAccess::All)),
        ];

        for (keys, dir, access) in cases {
            let yaml = format!("mode: preload\nsocket:\n    {keys}");

            let sut = parse_intercept(&yaml);

            assert_eq!(sut.socket, Some(Socket { dir, access }), "case: {keys}");
            assert!(sut.effective_socket().is_some(), "case: {keys}");
        }
    }

    // Requirements: interception-report-channel
    #[test]
    fn test_intercept_rejects_a_bad_access_value() {
        let yaml = "mode: preload\nsocket:\n    access: group";

        let sut: Result<Intercept, _> = serde_saphyr::from_str(yaml);

        assert!(sut.is_err());
    }

    // Requirements: interception-report-channel
    #[test]
    fn test_empty_socket_section_counts_as_absent() {
        let cases = ["mode: preload\nsocket:", "mode: preload\nsocket: {}", "mode: preload"];

        for yaml in cases {
            let sut = parse_intercept(yaml);

            assert!(sut.effective_socket().is_none(), "case: {yaml}");
        }
    }
}
