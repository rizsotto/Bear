// SPDX-License-Identifier: GPL-3.0-or-later

//! Order-independent comparison of two compilation databases.
//!
//! Equivalence is a multiset comparison over normalized entries, not a textual
//! diff: two databases are equivalent when they contain the same entries with
//! the same multiplicities, regardless of order. When they differ, the result
//! is reported as three sets - entries only in A, only in B, and entries that
//! match on `file` + `output` but differ in `arguments` or `directory`.

use std::collections::HashMap;

use serde::Serialize;

use crate::cdb::{CompilationDatabase, Entry};

/// One side of a "matched but differing" pair: entries that agree on
/// `file` + `output` but disagree on `arguments` and/or `directory`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DifferingEntry {
    pub file: String,
    pub output: Option<String>,
    pub a_directory: String,
    pub b_directory: String,
    pub a_arguments: Vec<String>,
    pub b_arguments: Vec<String>,
}

/// The outcome of comparing two databases.
#[derive(Debug, Serialize)]
pub struct DiffReport {
    /// Entries present in A but absent from B (as a multiset).
    pub only_in_a: Vec<Entry>,
    /// Entries present in B but absent from A (as a multiset).
    pub only_in_b: Vec<Entry>,
    /// Entries matching on `file` + `output` across A and B but differing in
    /// `arguments` and/or `directory`.
    pub differing: Vec<DifferingEntry>,
}

impl DiffReport {
    /// The databases are equivalent when no entry is exclusive to either side
    /// and none of the matched entries differ.
    pub fn is_equivalent(&self) -> bool {
        self.only_in_a.is_empty() && self.only_in_b.is_empty() && self.differing.is_empty()
    }

    /// Render a human-readable summary of the differences.
    pub fn to_human(&self) -> String {
        if self.is_equivalent() {
            return "databases are equivalent\n".to_string();
        }
        let mut out = String::new();
        out.push_str("databases differ\n");

        out.push_str(&format!("\nonly in A ({}):\n", self.only_in_a.len()));
        for entry in &self.only_in_a {
            out.push_str(&format!("  {}\n", describe(entry)));
        }

        out.push_str(&format!("\nonly in B ({}):\n", self.only_in_b.len()));
        for entry in &self.only_in_b {
            out.push_str(&format!("  {}\n", describe(entry)));
        }

        out.push_str(&format!("\nmatched but differing ({}):\n", self.differing.len()));
        for diff in &self.differing {
            let label = match &diff.output {
                Some(output) => format!("{} -> {}", diff.file, output),
                None => diff.file.clone(),
            };
            out.push_str(&format!("  {label}\n"));
            if diff.a_directory != diff.b_directory {
                out.push_str(&format!("    directory: A={} B={}\n", diff.a_directory, diff.b_directory));
            }
            if diff.a_arguments != diff.b_arguments {
                out.push_str(&format!("    arguments A: {}\n", diff.a_arguments.join(" ")));
                out.push_str(&format!("    arguments B: {}\n", diff.b_arguments.join(" ")));
            }
        }
        out
    }
}

fn describe(entry: &Entry) -> String {
    match &entry.output {
        Some(output) => format!("{} -> {} [{}]", entry.file, output, entry.arguments.join(" ")),
        None => format!("{} [{}]", entry.file, entry.arguments.join(" ")),
    }
}

/// Compare two databases as multisets of entries and produce a [`DiffReport`].
///
/// Identical entries (all fields equal) cancel out by multiplicity. Of the
/// remainder, entries that share a `file` + `output` key on both sides are
/// reported as "differing"; the rest are reported as exclusive to their side.
pub fn compare(a: &CompilationDatabase, b: &CompilationDatabase) -> DiffReport {
    let mut leftover_a = subtract_common(&a.entries, &b.entries);
    let mut leftover_b = subtract_common(&b.entries, &a.entries);

    let mut differing = Vec::new();
    // Pair up leftovers that share a file+output key; what does not pair is
    // exclusive to its side.
    let mut b_by_key: HashMap<(String, Option<String>), Vec<Entry>> = HashMap::new();
    for entry in leftover_b.drain(..) {
        b_by_key.entry(entry.match_key()).or_default().push(entry);
    }

    let mut only_in_a = Vec::new();
    for entry in leftover_a.drain(..) {
        match b_by_key.get_mut(&entry.match_key()).and_then(Vec::pop) {
            Some(b_entry) => differing.push(DifferingEntry {
                file: entry.file.clone(),
                output: entry.output.clone(),
                a_directory: entry.directory,
                b_directory: b_entry.directory,
                a_arguments: entry.arguments,
                b_arguments: b_entry.arguments,
            }),
            None => only_in_a.push(entry),
        }
    }

    let mut only_in_b: Vec<Entry> = b_by_key.into_values().flatten().collect();

    // Deterministic ordering so reports and JSON output are reproducible.
    only_in_a.sort_by(sort_entries);
    only_in_b.sort_by(sort_entries);
    differing.sort_by(|x, y| (&x.file, &x.output).cmp(&(&y.file, &y.output)));

    DiffReport { only_in_a, only_in_b, differing }
}

/// Return the entries of `from` that have no equal counterpart in `other`,
/// honouring multiplicity (an entry appearing twice in `from` and once in
/// `other` yields one leftover).
fn subtract_common(from: &[Entry], other: &[Entry]) -> Vec<Entry> {
    let mut available: HashMap<&Entry, usize> = HashMap::new();
    for entry in other {
        *available.entry(entry).or_insert(0) += 1;
    }
    let mut leftover = Vec::new();
    for entry in from {
        match available.get_mut(entry) {
            Some(count) if *count > 0 => *count -= 1,
            _ => leftover.push(entry.clone()),
        }
    }
    leftover
}

fn sort_entries(a: &Entry, b: &Entry) -> std::cmp::Ordering {
    (&a.file, &a.output, &a.directory, &a.arguments).cmp(&(&b.file, &b.output, &b.directory, &b.arguments))
}

// `Entry` is used as a HashMap key in `subtract_common`; derive the hashing
// from its public fields so equal entries collide.
impl std::hash::Hash for Entry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.directory.hash(state);
        self.file.hash(state);
        self.arguments.hash(state);
        self.output.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdb::Normalization;
    use std::path::PathBuf;

    fn db(json: &str) -> CompilationDatabase {
        CompilationDatabase::from_reader(json.as_bytes()).unwrap()
    }

    fn entry_a() -> &'static str {
        r#"{"directory": "/w", "file": "/w/a.c", "arguments": ["cc", "-c", "a.c"], "output": "a.o"}"#
    }
    fn entry_b() -> &'static str {
        r#"{"directory": "/w", "file": "/w/b.c", "arguments": ["cc", "-c", "b.c"], "output": "b.o"}"#
    }

    #[test]
    fn identical_databases_are_equivalent() {
        let left = db(&format!("[{}, {}]", entry_a(), entry_b()));
        let right = db(&format!("[{}, {}]", entry_a(), entry_b()));

        let sut = compare(&left, &right);

        assert!(sut.is_equivalent(), "report: {sut:?}");
    }

    #[test]
    fn reordered_databases_are_equivalent() {
        let left = db(&format!("[{}, {}]", entry_a(), entry_b()));
        let right = db(&format!("[{}, {}]", entry_b(), entry_a()));

        let sut = compare(&left, &right);

        assert!(sut.is_equivalent(), "report: {sut:?}");
    }

    #[test]
    fn normalized_databases_are_equivalent() {
        // Same build seen with an absolute ccache compiler and an absolute build
        // root vs a canonical compiler name and a relative root. Only the
        // structured fields rebase; arguments are compared verbatim, so both
        // sides keep the same absolute source path inside `arguments`.
        let left = db(r#"[{
            "directory": "/work/build",
            "file": "/work/src/a.c",
            "arguments": ["/usr/lib/ccache/cc", "-c", "/work/src/a.c"],
            "output": "a.o"
        }]"#);
        let right = db(r#"[{
            "directory": "build",
            "file": "src/a.c",
            "arguments": ["cc", "-c", "/work/src/a.c"],
            "output": "a.o"
        }]"#);
        let mut normalized_left = left;
        normalized_left.normalize(&Normalization {
            substitute_compiler: Some("cc".to_string()),
            relativize_paths: Some(PathBuf::from("/work")),
            ..Default::default()
        });

        let sut = compare(&normalized_left, &right);

        assert!(sut.is_equivalent(), "report: {sut:?}");
    }

    #[test]
    fn detects_entries_only_in_a() {
        let left = db(&format!("[{}, {}]", entry_a(), entry_b()));
        let right = db(&format!("[{}]", entry_b()));

        let sut = compare(&left, &right);

        assert!(!sut.is_equivalent());
        assert_eq!(sut.only_in_a.len(), 1);
        assert_eq!(sut.only_in_a[0].file, "/w/a.c");
        assert!(sut.only_in_b.is_empty());
        assert!(sut.differing.is_empty());
    }

    #[test]
    fn detects_entries_only_in_b() {
        let left = db(&format!("[{}]", entry_a()));
        let right = db(&format!("[{}, {}]", entry_a(), entry_b()));

        let sut = compare(&left, &right);

        assert!(!sut.is_equivalent());
        assert_eq!(sut.only_in_b.len(), 1);
        assert_eq!(sut.only_in_b[0].file, "/w/b.c");
        assert!(sut.only_in_a.is_empty());
        assert!(sut.differing.is_empty());
    }

    #[test]
    fn detects_matched_but_differing_entries() {
        let left = db(
            r#"[{"directory": "/w", "file": "/w/a.c", "arguments": ["cc", "-O0", "-c", "a.c"], "output": "a.o"}]"#,
        );
        let right = db(
            r#"[{"directory": "/w", "file": "/w/a.c", "arguments": ["cc", "-O2", "-c", "a.c"], "output": "a.o"}]"#,
        );

        let sut = compare(&left, &right);

        assert!(!sut.is_equivalent());
        assert!(sut.only_in_a.is_empty());
        assert!(sut.only_in_b.is_empty());
        assert_eq!(sut.differing.len(), 1);
        assert_eq!(sut.differing[0].file, "/w/a.c");
        assert_eq!(sut.differing[0].a_arguments, vec!["cc", "-O0", "-c", "a.c"]);
        assert_eq!(sut.differing[0].b_arguments, vec!["cc", "-O2", "-c", "a.c"]);
    }

    #[test]
    fn detects_matched_but_differing_directory() {
        // Same file+output and same arguments, but the recorded directory
        // differs - the report must surface the directory disagreement.
        let left = db(
            r#"[{"directory": "/w/a", "file": "/w/a.c", "arguments": ["cc", "-c", "a.c"], "output": "a.o"}]"#,
        );
        let right = db(
            r#"[{"directory": "/w/b", "file": "/w/a.c", "arguments": ["cc", "-c", "a.c"], "output": "a.o"}]"#,
        );

        let sut = compare(&left, &right);

        assert_eq!(sut.differing.len(), 1, "report: {sut:?}");
        assert_eq!(sut.differing[0].a_directory, "/w/a");
        assert_eq!(sut.differing[0].b_directory, "/w/b");
        assert_eq!(sut.differing[0].a_arguments, sut.differing[0].b_arguments);
        assert!(sut.only_in_a.is_empty());
        assert!(sut.only_in_b.is_empty());
    }

    #[test]
    fn multiset_honours_multiplicity() {
        let left = db(&format!("[{}, {}]", entry_a(), entry_a()));
        let right = db(&format!("[{}]", entry_a()));

        let sut = compare(&left, &right);

        assert_eq!(sut.only_in_a.len(), 1);
        assert!(sut.only_in_b.is_empty());
    }

    #[test]
    fn human_report_is_empty_message_when_equivalent() {
        let left = db(&format!("[{}]", entry_a()));
        let right = db(&format!("[{}]", entry_a()));

        let sut = compare(&left, &right).to_human();

        assert_eq!(sut, "databases are equivalent\n");
    }
}
