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

// Generated compiler-id data (KNOWN_IDS, WRAPPER_AS_NAMES) from
// compilers/*.yaml. These are the sole accepted config `as:` spellings; the
// deserializer below validates against them instead of a hand-maintained
// mirror of the family set. See compiler-as-no-aliases.
include!(concat!(env!("OUT_DIR"), "/compiler_ids.rs"));

/// A compiler family's identity: the `compiler.id` declared in its YAML
/// definition, which is also its sole accepted config `as:` spelling.
///
/// Wraps a `&'static str` that is always one of the generated [`KNOWN_IDS`]:
/// [`CompilerType`]'s `Deserialize` resolves user input to a canonical entry,
/// and internal literals are drift-guarded against that list by a test.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct CompilerId(&'static str);

impl CompilerId {
    /// Construct from a compile-time-known id. The caller guarantees the id is
    /// one of [`KNOWN_IDS`]; `compiler_id_literals_are_known` enforces this for
    /// every internal literal.
    pub(crate) const fn new(id: &'static str) -> Self {
        CompilerId(id)
    }

    /// The canonical id string, verbatim.
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

impl fmt::Display for CompilerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// The kind of tool Bear recognizes: a real compiler family (carrying its
/// [`CompilerId`]) or a compiler launcher (wrapper). Mirrors the YAML `type:`
/// field; the family set lives in data ([`KNOWN_IDS`]), not in this enum.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum CompilerType {
    Compiler(CompilerId),
    Wrapper,
}

impl CompilerType {
    /// Construct a compiler-kind value from a compile-time-known id.
    pub(crate) const fn compiler(id: &'static str) -> Self {
        CompilerType::Compiler(CompilerId::new(id))
    }

    /// Map a generated `RECOGNITION_PATTERNS` id column to a value. The string
    /// is trusted generated data: `"wrapper"` names the launcher kind, every
    /// other value is a real compiler id.
    pub(crate) fn from_recognized_id(id: &'static str) -> Self {
        if id == "wrapper" { CompilerType::Wrapper } else { CompilerType::compiler(id) }
    }
}

impl fmt::Display for CompilerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompilerType::Compiler(id) => fmt::Display::fmt(id, f),
            CompilerType::Wrapper => f.write_str("wrapper"),
        }
    }
}

impl serde::Serialize for CompilerType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            CompilerType::Compiler(id) => id.as_str(),
            CompilerType::Wrapper => "wrapper",
        })
    }
}

impl<'de> serde::Deserialize<'de> for CompilerType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let spelling = String::deserialize(deserializer)?;
        // The four launchers share one runtime kind: "wrapper" plus each
        // launcher basename all resolve to it (see compiler-as-no-aliases).
        if spelling == "wrapper" || WRAPPER_AS_NAMES.contains(&spelling.as_str()) {
            return Ok(CompilerType::Wrapper);
        }
        // A compiler family is its id, verbatim -- no aliases. Resolve to the
        // canonical `&'static str` so the value is unconstructible unvalidated.
        if let Some(&canonical) = KNOWN_IDS.iter().find(|&&id| id == spelling) {
            return Ok(CompilerType::Compiler(CompilerId::new(canonical)));
        }
        // List every accepted `as:` value: the compiler ids plus the
        // launcher spellings ("wrapper" and each basename). Omitting the
        // launcher spellings would misdescribe a mistyped `ccache` as if only
        // compiler ids were valid.
        Err(D::Error::custom(format!(
            "unknown compiler id {spelling:?}, expected one of: {}, wrapper, {}",
            KNOWN_IDS.join(", "),
            WRAPPER_AS_NAMES.join(", ")
        )))
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
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SourceFilter {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub directories: Vec<DirectoryRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FileRule>,
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
    /// Disabled by default: an `@file` argument is recorded verbatim.
    #[serde(default)]
    pub from_response_files: bool,
    /// Fold compiler environment variables (e.g. `CPATH`) into each entry's
    /// arguments as explicit flags. Enabled by default.
    #[serde(default = "default_enabled")]
    pub from_environment: bool,
}

impl Default for ArgumentsFormat {
    fn default() -> Self {
        Self { from_response_files: false, from_environment: true }
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
    fn compiler_id_literals_are_known() {
        // Every compiler id hand-written as a literal in production code must
        // be one of the generated KNOWN_IDS. If a YAML rename ever drifts one
        // of these, this fails instead of silently misclassifying at runtime.
        //
        // These are the only such literals: the ambiguous-name probe's two
        // verdicts and the recognition fallback (all `gcc`/`clang`). Family
        // registration and response-file syntax are driven entirely by
        // generated data (the FAMILIES registry), so they carry no id
        // literals to guard here; the dispatch test
        // (every_recognition_pattern_row_is_dispatched_by_a_registered_interpreter)
        // covers that path end to end.
        for id in ["gcc", "clang"] {
            assert!(KNOWN_IDS.contains(&id), "internal id literal {id:?} is not in KNOWN_IDS");
        }
    }

    #[test]
    fn unknown_as_value_error_lists_wrapper_and_a_real_id() {
        // The "expected one of" list must name every accepted `as:` value,
        // including the launcher spellings -- not just the compiler ids.
        let sut = serde_json::from_str::<CompilerType>("\"nope\"").unwrap_err().to_string();

        assert!(sut.contains("unknown compiler id"), "message: {sut}");
        assert!(sut.contains(KNOWN_IDS[0]), "message should list a real id: {sut}");
        assert!(sut.contains("wrapper"), "message should list the wrapper spelling: {sut}");
        assert!(sut.contains(WRAPPER_AS_NAMES[0]), "message should list a launcher name: {sut}");
    }

    #[test]
    fn compiler_type_round_trips_through_serde() {
        // Serialize then deserialize yields the same value, for both kinds.
        for sut in [CompilerType::compiler(KNOWN_IDS[0]), CompilerType::Wrapper] {
            let json = serde_json::to_string(&sut).unwrap();

            let back: CompilerType = serde_json::from_str(&json).unwrap();

            assert_eq!(back, sut, "round-trip via {json}");
        }
    }
}
