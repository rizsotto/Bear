// SPDX-License-Identifier: GPL-3.0-or-later

//! Compilation database model, parsing, serialization, and normalization.
//!
//! A Clang JSON compilation database is an array of entries. Each entry has a
//! `directory`, a `file`, and the compiler invocation expressed either as an
//! `arguments` array or as a `command` string; `output` is optional. This
//! module parses both forms into a single in-memory [`Entry`], and applies the
//! explicit, individually toggleable normalization operations the comparator
//! uses (see [`Normalization`]).

use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

use serde::Serializer as _;
use serde::{Deserialize, Serialize};

/// Errors raised while reading or normalizing a compilation database.
#[derive(Debug, thiserror::Error)]
pub enum CdbError {
    #[error("i/o error while reading or writing the compilation database: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse compilation database JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("entry {index} has neither an 'arguments' array nor a 'command' string")]
    MissingInvocation { index: usize },
    #[error("entry {index} has a 'command' string that is not valid shell words: {source}")]
    BadCommand { index: usize, source: shell_words::ParseError },
}

/// A single compilation database entry in normalized in-memory form.
///
/// Both the `arguments` and `command` input encodings are folded into
/// `arguments` here; serialization always emits the `arguments` form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    pub directory: String,
    pub file: String,
    pub arguments: Vec<String>,
    pub output: Option<String>,
}

/// Raw on-disk shape of an entry, accepting both `arguments` and `command`.
#[derive(Debug, Deserialize)]
struct RawEntry {
    directory: String,
    file: String,
    #[serde(default)]
    arguments: Option<Vec<String>>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    output: Option<String>,
}

/// On-disk shape used when serializing a normalized entry.
#[derive(Debug, Serialize)]
struct OutEntry<'a> {
    directory: &'a str,
    file: &'a str,
    arguments: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<&'a str>,
}

impl Entry {
    fn from_raw(index: usize, raw: RawEntry) -> Result<Self, CdbError> {
        let arguments = match (raw.arguments, raw.command) {
            (Some(args), _) => args,
            (None, Some(command)) => {
                shell_words::split(&command).map_err(|source| CdbError::BadCommand { index, source })?
            }
            (None, None) => return Err(CdbError::MissingInvocation { index }),
        };
        Ok(Entry { directory: raw.directory, file: raw.file, arguments, output: raw.output })
    }

    fn to_out(&self) -> OutEntry<'_> {
        OutEntry {
            directory: &self.directory,
            file: &self.file,
            arguments: &self.arguments,
            output: self.output.as_deref(),
        }
    }

    /// A stable key for matching two entries that describe the same translation
    /// unit: the `file` plus the `output` (which disambiguates one source
    /// compiled into several objects with different flags).
    pub fn match_key(&self) -> (String, Option<String>) {
        (self.file.clone(), self.output.clone())
    }
}

/// A parsed compilation database.
#[derive(Debug)]
pub struct CompilationDatabase {
    pub entries: Vec<Entry>,
}

impl CompilationDatabase {
    /// Parse a compilation database from a reader over its JSON text.
    ///
    /// The reader is consumed incrementally, so no full-text copy is held
    /// alongside the parsed entries. The entries themselves are still
    /// materialized into memory; streaming them is a separate step.
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, CdbError> {
        let raw: Vec<RawEntry> = serde_json::from_reader(BufReader::new(reader))?;
        let entries = raw
            .into_iter()
            .enumerate()
            .map(|(index, e)| Entry::from_raw(index, e))
            .collect::<Result<_, _>>()?;
        Ok(CompilationDatabase { entries })
    }

    /// Serialize the database as pretty-printed JSON to a writer (always the
    /// `arguments` form, with a trailing newline).
    ///
    /// Entries are streamed to the writer as they are visited, so no full-text
    /// copy of the output is buffered in memory.
    pub fn to_writer<W: Write>(&self, mut writer: W) -> Result<(), CdbError> {
        let out = self.entries.iter().map(Entry::to_out);
        let mut serializer = serde_json::Serializer::pretty(&mut writer);
        serializer.collect_seq(out)?;
        writer.write_all(b"\n")?;
        Ok(())
    }

    /// Apply the requested normalization operations in place.
    ///
    /// The operations are independent and off by default; only those set on
    /// `norm` run. `sort` is applied last so the canonical order reflects the
    /// post-substitution, post-relativization values.
    pub fn normalize(&mut self, norm: &Normalization) {
        if norm.output_from_o {
            for entry in &mut self.entries {
                if let Some(output) = output_from_o(&entry.directory, &entry.arguments) {
                    entry.output = Some(output);
                }
            }
        }
        if norm.drop_dependency_flags {
            for entry in &mut self.entries {
                entry.arguments = drop_dependency_flags(&entry.arguments);
            }
        }
        if let Some(canonical) = &norm.substitute_compiler {
            for entry in &mut self.entries {
                if let Some(first) = entry.arguments.first_mut() {
                    *first = canonical.clone();
                }
            }
        }
        if let Some(root) = &norm.relativize_paths {
            for entry in &mut self.entries {
                entry.directory = relativize(&entry.directory, root);
                entry.file = relativize(&entry.file, root);
                if let Some(output) = &mut entry.output {
                    *output = relativize(output, root);
                }
            }
        }
        if norm.sort {
            self.entries.sort_by(Entry::sort_key);
        }
    }
}

impl Entry {
    /// Total order over entries for the canonical `sort` normalization. The
    /// arguments are compared as-is; argument order inside an entry is
    /// semantically significant for a compiler invocation, so it is preserved.
    fn sort_key(a: &Entry, b: &Entry) -> std::cmp::Ordering {
        (&a.file, &a.output, &a.directory, &a.arguments).cmp(&(
            &b.file,
            &b.output,
            &b.directory,
            &b.arguments,
        ))
    }
}

/// The set of normalization operations to apply before comparison or
/// serialization. Every field is off/empty by default; construct with
/// [`Normalization::default`] and set only what a given comparison needs.
#[derive(Debug, Default)]
pub struct Normalization {
    /// Emit the database in a canonical entry order.
    pub sort: bool,
    /// Replace the first argument (compiler driver) with this canonical value.
    pub substitute_compiler: Option<String>,
    /// Rebase the absolute `directory`, `file`, and `output` paths against this
    /// root. Paths embedded inside `arguments` (e.g. `-I/abs/path`) are left
    /// untouched; argument-level rebasing is deferred until a real diff needs it.
    pub relativize_paths: Option<PathBuf>,
    /// Rewrite each entry's `output` to the absolute object path derived from the
    /// compiler's `-o` argument resolved against `directory` (lexically, without
    /// touching the filesystem). Makes the match key a true object identity that
    /// is identical across producers regardless of how each encodes `output`.
    pub output_from_o: bool,
    /// Strip the dependency-file generation flags (`-MD`, `-MMD`, `-MP`, and the
    /// argument-consuming `-MF`/`-MT`/`-MQ`/`-MJ`) from `arguments`. They only
    /// control the build system's `.d` side-file and never affect the produced
    /// object or how a Clang tool parses the translation unit.
    pub drop_dependency_flags: bool,
}

/// Rebase an absolute path string against `root`. A path that is not under
/// `root` (or is already relative) is returned unchanged. Only the structured
/// `directory`/`file`/`output` fields are rebased; paths embedded inside
/// arguments are left untouched (see the `relativize_paths` field docs).
fn relativize(value: &str, root: &Path) -> String {
    let path = Path::new(value);
    match path.strip_prefix(root) {
        Ok(rel) => rel.to_string_lossy().into_owned(),
        Err(_) => value.to_string(),
    }
}

/// Derive the absolute object path from the compiler's `-o` argument resolved
/// against `directory`. Only the separate-token `-o <path>` form is recognized
/// (the only form the captured data uses); an attached `-ofoo` is ignored. If
/// `-o` appears more than once the last occurrence wins, matching compiler
/// semantics. Returns `None` when there is no `-o` argument.
fn output_from_o(directory: &str, arguments: &[String]) -> Option<String> {
    let mut value: Option<&str> = None;
    let mut iter = arguments.iter();
    while let Some(arg) = iter.next() {
        if arg == "-o"
            && let Some(next) = iter.next()
        {
            value = Some(next);
        }
    }
    let value = value?;
    // POSIX path semantics, NOT the host's: a compilation database always names
    // files with `/` separators (it is produced inside a Linux container), and
    // the comparator must derive the same object path whether it runs on Linux,
    // macOS, or Windows. Going through `std::path` would join with the host
    // separator (`\` on Windows) and misjudge a leading-`/` path as relative
    // there, so resolve and normalize purely as `/`-delimited strings.
    let joined = if value.starts_with('/') {
        value.to_string()
    } else {
        format!("{}/{value}", directory.trim_end_matches('/'))
    };
    Some(lexically_normalize(&joined))
}

/// Collapse `.`, `..`, and redundant separators in a `/`-delimited path purely
/// lexically - without `canonicalize`/stat - because the paths name files
/// inside a throwaway container that does not exist on this host. A leading `..`
/// (a path that escapes its root) is kept as-is rather than discarded.
fn lexically_normalize(path: &str) -> String {
    let is_absolute = path.starts_with('/');
    let mut stack: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            // Skip empty segments (redundant separators) and current-dir markers.
            "" | "." => {}
            ".." => match stack.last() {
                Some(&last) if last != ".." => {
                    stack.pop();
                }
                _ => stack.push(segment),
            },
            other => stack.push(other),
        }
    }
    let joined = stack.join("/");
    if is_absolute { format!("/{joined}") } else { joined }
}

/// Remove the dependency-file generation flags from `arguments`. `-MD`, `-MMD`,
/// and `-MP` take no argument; `-MF`, `-MT`, `-MQ`, and `-MJ` each consume the
/// single following token, which is dropped with them. Matching is on whole
/// tokens only (the form the data uses); a trailing consuming flag with no
/// following token is simply dropped without panicking.
fn drop_dependency_flags(arguments: &[String]) -> Vec<String> {
    const NO_ARG: [&str; 3] = ["-MD", "-MMD", "-MP"];
    const CONSUMES_ARG: [&str; 4] = ["-MF", "-MT", "-MQ", "-MJ"];

    let mut result = Vec::with_capacity(arguments.len());
    let mut iter = arguments.iter();
    while let Some(arg) = iter.next() {
        if NO_ARG.contains(&arg.as_str()) {
            continue;
        }
        if CONSUMES_ARG.contains(&arg.as_str()) {
            iter.next();
            continue;
        }
        result.push(arg.clone());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse from a string fixture through the reader API.
    fn parse(text: &str) -> Result<CompilationDatabase, CdbError> {
        CompilationDatabase::from_reader(text.as_bytes())
    }

    /// Serialize through the writer API and recover the text for assertions.
    fn serialize(db: &CompilationDatabase) -> String {
        let mut buf = Vec::new();
        db.to_writer(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn arguments_db() -> &'static str {
        r#"[
          {
            "directory": "/work/build",
            "file": "/work/src/a.c",
            "arguments": ["/usr/bin/cc", "-c", "/work/src/a.c", "-o", "a.o"],
            "output": "a.o"
          }
        ]"#
    }

    fn command_db() -> &'static str {
        r#"[
          {
            "directory": "/work/build",
            "file": "/work/src/a.c",
            "command": "/usr/bin/cc -c /work/src/a.c -o a.o",
            "output": "a.o"
          }
        ]"#
    }

    #[test]
    fn parses_arguments_form() {
        let sut = parse(arguments_db()).unwrap();

        assert_eq!(sut.entries.len(), 1);
        assert_eq!(sut.entries[0].arguments[0], "/usr/bin/cc");
        assert_eq!(sut.entries[0].output.as_deref(), Some("a.o"));
    }

    #[test]
    fn parses_command_form_into_arguments() {
        let sut = parse(command_db()).unwrap();

        assert_eq!(sut.entries[0].arguments, vec!["/usr/bin/cc", "-c", "/work/src/a.c", "-o", "a.o"]);
    }

    #[test]
    fn arguments_and_command_forms_compare_equal() {
        let from_args = parse(arguments_db()).unwrap();
        let from_command = parse(command_db()).unwrap();

        assert_eq!(from_args.entries, from_command.entries);
    }

    #[test]
    fn missing_invocation_is_an_error() {
        let json = r#"[{"directory": "/w", "file": "/w/a.c"}]"#;

        let sut = parse(json);

        assert!(matches!(sut, Err(CdbError::MissingInvocation { index: 0 })));
    }

    #[test]
    fn substitute_compiler_replaces_first_argument() {
        let mut sut = parse(arguments_db()).unwrap();
        let norm = Normalization { substitute_compiler: Some("cc".to_string()), ..Default::default() };

        sut.normalize(&norm);

        assert_eq!(sut.entries[0].arguments[0], "cc");
    }

    #[test]
    fn relativize_paths_rebases_fields_but_leaves_arguments_untouched() {
        let json = r#"[
          {
            "directory": "/work/build",
            "file": "/work/src/a.c",
            "arguments": ["cc", "-I/work/include", "-c", "/work/src/a.c"],
            "output": "/work/build/a.o"
          }
        ]"#;
        let mut sut = parse(json).unwrap();
        let norm = Normalization { relativize_paths: Some(PathBuf::from("/work")), ..Default::default() };

        sut.normalize(&norm);

        let entry = &sut.entries[0];
        assert_eq!(entry.directory, "build");
        assert_eq!(entry.file, "src/a.c");
        assert_eq!(entry.output.as_deref(), Some("build/a.o"));
        // Arguments are deliberately left as-is; only the structured fields rebase.
        assert_eq!(entry.arguments, vec!["cc", "-I/work/include", "-c", "/work/src/a.c"]);
    }

    #[test]
    fn sort_produces_canonical_order() {
        // Two input orderings must yield byte-identical canonical output. The
        // sort key covers all entry fields, so true ties are impossible and
        // stability is trivially satisfied; this asserts the canonical-order
        // property, which is what golden-manifest production relies on.
        let unsorted = r#"[
          {"directory": "/w", "file": "/w/b.c", "arguments": ["cc", "-c", "b.c"]},
          {"directory": "/w", "file": "/w/a.c", "arguments": ["cc", "-c", "a.c"]}
        ]"#;
        let reordered = r#"[
          {"directory": "/w", "file": "/w/a.c", "arguments": ["cc", "-c", "a.c"]},
          {"directory": "/w", "file": "/w/b.c", "arguments": ["cc", "-c", "b.c"]}
        ]"#;
        let norm = Normalization { sort: true, ..Default::default() };

        let mut from_unsorted = parse(unsorted).unwrap();
        from_unsorted.normalize(&norm);
        let mut from_reordered = parse(reordered).unwrap();
        from_reordered.normalize(&norm);

        assert_eq!(from_unsorted.entries[0].file, "/w/a.c");
        assert_eq!(serialize(&from_unsorted), serialize(&from_reordered));
    }

    /// An entry whose `-o` value is relative to `directory` (Bear's encoding).
    fn entry_with_relative_o() -> &'static str {
        r#"[{
            "directory": "/build/lib",
            "file": "/src/lib/altsvc.c",
            "arguments": ["cc", "-o", "CMakeFiles/libcurl_shared.dir/altsvc.c.o", "-c", "/src/lib/altsvc.c"],
            "output": "CMakeFiles/libcurl_shared.dir/altsvc.c.o"
        }]"#
    }

    /// The same object as `entry_with_relative_o`, but with the `output` field
    /// encoded relative to the build root (CMake's encoding). The `-o` value is
    /// still relative to `directory`, so `output_from_o` must reconcile them.
    fn entry_with_build_root_output() -> &'static str {
        r#"[{
            "directory": "/build/lib",
            "file": "/src/lib/altsvc.c",
            "arguments": ["cc", "-o", "CMakeFiles/libcurl_shared.dir/altsvc.c.o", "-c", "/src/lib/altsvc.c"],
            "output": "lib/CMakeFiles/libcurl_shared.dir/altsvc.c.o"
        }]"#
    }

    #[test]
    fn output_from_o_reconciles_differently_encoded_outputs() {
        let norm = Normalization { output_from_o: true, ..Default::default() };

        let mut bear = parse(entry_with_relative_o()).unwrap();
        bear.normalize(&norm);
        let mut cmake = parse(entry_with_build_root_output()).unwrap();
        cmake.normalize(&norm);

        let expected = Some("/build/lib/CMakeFiles/libcurl_shared.dir/altsvc.c.o".to_string());
        assert_eq!(bear.entries[0].output, expected);
        assert_eq!(cmake.entries[0].output, expected);
    }

    #[test]
    fn output_from_o_preserves_absolute_o_value() {
        let json = r#"[{
            "directory": "/build/lib",
            "file": "/src/a.c",
            "arguments": ["cc", "-o", "/build/lib/a.o", "-c", "/src/a.c"],
            "output": "a.o"
        }]"#;
        let mut sut = parse(json).unwrap();
        let norm = Normalization { output_from_o: true, ..Default::default() };

        sut.normalize(&norm);

        assert_eq!(sut.entries[0].output.as_deref(), Some("/build/lib/a.o"));
    }

    #[test]
    fn output_from_o_last_occurrence_wins() {
        // Compiler semantics: a repeated -o means the last value is the object.
        let json = r#"[{
            "directory": "/build/lib",
            "file": "/src/a.c",
            "arguments": ["cc", "-o", "first.o", "-o", "second.o", "-c", "/src/a.c"]
        }]"#;
        let mut sut = parse(json).unwrap();
        let norm = Normalization { output_from_o: true, ..Default::default() };

        sut.normalize(&norm);

        assert_eq!(sut.entries[0].output.as_deref(), Some("/build/lib/second.o"));
    }

    #[test]
    fn output_from_o_leaves_output_unchanged_without_o_argument() {
        let json = r#"[{
            "directory": "/build/lib",
            "file": "/src/a.c",
            "arguments": ["cc", "-c", "/src/a.c"],
            "output": "a.o"
        }]"#;
        let mut sut = parse(json).unwrap();
        let norm = Normalization { output_from_o: true, ..Default::default() };

        sut.normalize(&norm);

        assert_eq!(sut.entries[0].output.as_deref(), Some("a.o"));
    }

    #[test]
    fn output_from_o_keeps_multi_target_outputs_distinct() {
        // The same source compiled into two targets has distinct `-o` values, so
        // the derived absolute outputs must stay distinct (file-only matching
        // would falsely collapse them).
        let json = r#"[
            {
                "directory": "/build/lib",
                "file": "/src/lib/base64.c",
                "arguments": ["cc", "-o", "CMakeFiles/libcurl_shared.dir/base64.c.o", "-c", "/src/lib/base64.c"]
            },
            {
                "directory": "/build/src",
                "file": "/src/lib/base64.c",
                "arguments": ["cc", "-o", "CMakeFiles/curl.dir/__/lib/base64.c.o", "-c", "/src/lib/base64.c"]
            }
        ]"#;
        let mut sut = parse(json).unwrap();
        let norm = Normalization { output_from_o: true, ..Default::default() };

        sut.normalize(&norm);

        assert_eq!(
            sut.entries[0].output.as_deref(),
            Some("/build/lib/CMakeFiles/libcurl_shared.dir/base64.c.o")
        );
        // The `..` in `__/lib` is not lexically collapsible here (it is a literal
        // directory name CMake emits, not a real parent reference), so it is kept.
        assert_eq!(
            sut.entries[1].output.as_deref(),
            Some("/build/src/CMakeFiles/curl.dir/__/lib/base64.c.o")
        );
        assert_ne!(sut.entries[0].output, sut.entries[1].output);
    }

    #[test]
    fn drop_dependency_flags_removes_the_depfile_group() {
        let cases: &[(&[&str], &[&str])] = &[
            // No-argument and consuming flags interleaved with kept flags.
            (
                &["cc", "-MD", "-MT", "x.o", "-MF", "x.o.d", "-o", "x.o", "-c", "a.c"],
                &["cc", "-o", "x.o", "-c", "a.c"],
            ),
            // -MMD/-MP no-argument forms and -MQ/-MJ consuming forms.
            (&["cc", "-MMD", "-MP", "-MQ", "t", "-MJ", "db.json", "-c", "a.c"], &["cc", "-c", "a.c"]),
            // A trailing consuming flag with no following token must not panic.
            (&["cc", "-c", "a.c", "-MF"], &["cc", "-c", "a.c"]),
            // Lookalikes and ordinary flags are kept untouched.
            (
                &["cc", "-MMD-not-a-flag", "-Map", "-O2", "-Wall", "-fPIC", "-c", "a.c"],
                &["cc", "-MMD-not-a-flag", "-Map", "-O2", "-Wall", "-fPIC", "-c", "a.c"],
            ),
        ];

        for (input, expected) in cases {
            let arguments: Vec<String> = input.iter().map(|s| s.to_string()).collect();

            let sut = super::drop_dependency_flags(&arguments);

            assert_eq!(sut, *expected, "case: {input:?}");
        }
    }

    #[test]
    fn drop_dependency_flags_makes_depfile_only_difference_cancel() {
        use crate::compare::compare;

        let with_depflags = parse(
            r#"[{
                "directory": "/build/lib",
                "file": "/src/a.c",
                "arguments": ["cc", "-MD", "-MT", "a.o", "-MF", "a.o.d", "-o", "a.o", "-c", "/src/a.c"],
                "output": "a.o"
            }]"#,
        )
        .unwrap();
        let without_depflags = parse(
            r#"[{
                "directory": "/build/lib",
                "file": "/src/a.c",
                "arguments": ["cc", "-o", "a.o", "-c", "/src/a.c"],
                "output": "a.o"
            }]"#,
        )
        .unwrap();
        let norm = Normalization { drop_dependency_flags: true, ..Default::default() };

        let mut left = with_depflags;
        left.normalize(&norm);
        let mut right = without_depflags;
        right.normalize(&norm);
        let sut = compare(&left, &right);

        assert!(sut.is_equivalent(), "report: {sut:?}");
    }

    #[test]
    fn round_trips_through_reader_and_writer() {
        let original = parse(arguments_db()).unwrap();

        let text = serialize(&original);
        let sut = parse(&text).unwrap();

        assert_eq!(sut.entries, original.entries);
    }
}
