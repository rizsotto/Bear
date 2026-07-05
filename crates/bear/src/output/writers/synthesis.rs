// SPDX-License-Identifier: GPL-3.0-or-later

//! Header entry synthesis for the output pipeline.
//!
//! When enabled via configuration, this stage clones the compile flags of a
//! compiled translation unit onto header files discovered on disk, so that
//! editors and linters can resolve compile flags for headers as well as
//! sources. See `docs/requirements/output-header-entries.md`.
//!
//! Two discovery strategies are implemented here:
//!
//! - `Siblings`: header files that live in the same directory as a compiled
//!   C, C++, or Objective-C source.
//! - `IncludeDirs`: a superset of `Siblings` that also scans a donor's own
//!   `-I`/`-iquote` include directories, but only those that resolve under
//!   the project root (bear's current working directory), to avoid flooding
//!   the database with system headers.
//!
//! The `DependencyFiles` strategy is recognized by configuration but
//! currently forwards entries unchanged.

use crate::config::{HeaderStrategy, Headers};
use crate::output::WriterError;
use crate::output::clang::Entry;
use crate::output::statistics::OutputStatistics;
use crate::semantic::interpreters::matchers::{is_c_family_source, is_header_file, looks_like_a_source_file};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::IteratorWriter;

/// Synthesizes compilation database entries for header files by cloning a
/// donor translation unit's arguments, per the configured discovery strategy.
pub(crate) struct HeaderEntrySynthesizer<T: IteratorWriter<Entry>> {
    writer: T,
    config: Headers,
    project_root: PathBuf,
    stats: Arc<OutputStatistics>,
}

impl<T: IteratorWriter<Entry>> HeaderEntrySynthesizer<T> {
    pub(crate) fn new(
        writer: T,
        config: Headers,
        project_root: PathBuf,
        stats: Arc<OutputStatistics>,
    ) -> Self {
        Self { writer, config, project_root, stats }
    }

    /// Siblings/include-dirs strategies: for each directory recorded as a
    /// scan target, scan the directory once (in the epilogue, after all
    /// entries have streamed through) and synthesize an entry for each header
    /// file found there, cloning the first-seen donor's arguments.
    fn write_directory_scan(self, entries: impl Iterator<Item = Entry>) -> Result<(), WriterError> {
        let collector =
            Rc::new(RefCell::new(DirectoryCollector::new(self.config.strategy, self.project_root.clone())));
        let collector1 = Rc::clone(&collector);
        let collector2 = Rc::clone(&collector);
        let stats = Arc::clone(&self.stats);

        let recording = entries.inspect(move |entry| {
            collector1.borrow_mut().observe(entry);
        });

        let epilogue = std::iter::once_with(move || {
            let synthesized = collector2.borrow().synthesize();
            stats.entries_synthesized.fetch_add(synthesized.len(), Ordering::Relaxed);
            synthesized
        })
        .flatten();

        self.writer.write(recording.chain(epilogue))
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<Entry> for HeaderEntrySynthesizer<T> {
    fn write(self, entries: impl Iterator<Item = Entry>) -> Result<(), WriterError> {
        if !self.config.enabled {
            return self.writer.write(entries);
        }

        match self.config.strategy {
            HeaderStrategy::Siblings | HeaderStrategy::IncludeDirs => self.write_directory_scan(entries),
            // Implemented in a later commit; forward untouched until then.
            HeaderStrategy::DependencyFiles => self.writer.write(entries),
        }
    }
}

/// One scan target: the donor whose arguments are cloned, plus how the
/// compiler refers to this directory (used to build header file paths).
struct DirRecord {
    arguments: Vec<String>,
    directory: PathBuf,
    source_file: PathBuf,
    display_dir: PathBuf,
}

/// Collects the first eligible donor observed per physical directory (the
/// source's own directory, and, for the `IncludeDirs` strategy, its
/// in-project `-I`/`-iquote` directories), then synthesizes header entries
/// for each recorded directory's header files.
struct DirectoryCollector {
    strategy: HeaderStrategy,
    project_root: PathBuf,
    /// Keyed by the physical directory to scan for headers.
    records: HashMap<PathBuf, DirRecord>,
}

impl DirectoryCollector {
    fn new(strategy: HeaderStrategy, project_root: PathBuf) -> Self {
        Self { strategy, project_root, records: HashMap::new() }
    }

    /// Records `entry`'s own directory as a scan target, and, for the
    /// `IncludeDirs` strategy, each of its in-project include directories,
    /// unless it is command-form (no `arguments` to clone) or not a C-family
    /// source. First-seen donor wins per physical directory.
    fn observe(&mut self, entry: &Entry) {
        if entry.arguments.is_empty() {
            return;
        }
        if !is_c_family_source(&entry.file) {
            return;
        }

        let physical = physical_parent(&entry.directory, &entry.file);
        let display = entry.file.parent().map(Path::to_path_buf).unwrap_or_default();
        self.record(physical, entry, display);

        if self.strategy == HeaderStrategy::IncludeDirs {
            for inc in extract_include_dirs(&entry.arguments) {
                let physical = if inc.is_absolute() { inc.clone() } else { entry.directory.join(&inc) };
                if !physical.starts_with(&self.project_root) {
                    // Out of scope: system or out-of-project include dirs
                    // are not scanned, to avoid flooding the database.
                    continue;
                }
                self.record(physical, entry, inc);
            }
        }
    }

    fn record(&mut self, physical_dir: PathBuf, entry: &Entry, display_dir: PathBuf) {
        self.records.entry(physical_dir).or_insert_with(|| DirRecord {
            arguments: entry.arguments.clone(),
            directory: entry.directory.clone(),
            source_file: entry.file.clone(),
            display_dir,
        });
    }

    /// Scans each recorded directory once and synthesizes an entry per header
    /// file found there, cloning that directory's donor arguments. The
    /// header's `file` path is built from the recorded display prefix, so an
    /// include-dir header is reported through the same path the compiler
    /// used to reach it (e.g. `-Iinclude` -> `include/util.h`).
    fn synthesize(&self) -> Vec<Entry> {
        let mut result = Vec::new();

        for (physical_dir, rec) in &self.records {
            let read_dir = match std::fs::read_dir(physical_dir) {
                Ok(read_dir) => read_dir,
                Err(err) => {
                    log::debug!("Skipping header synthesis for {:?}: {}", physical_dir, err);
                    continue;
                }
            };

            let mut header_names: Vec<std::ffi::OsString> = read_dir
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_type().is_ok_and(|ft| ft.is_file()))
                .map(|entry| entry.file_name())
                .filter(|name| is_header_file(Path::new(name)))
                .collect();
            header_names.sort();

            for name in header_names {
                let header_file = if rec.display_dir.as_os_str().is_empty() {
                    PathBuf::from(&name)
                } else {
                    rec.display_dir.join(&name)
                };

                let Some(rewritten) = rewrite_arguments(&rec.arguments, &rec.source_file, &header_file)
                else {
                    log::debug!(
                        "Skipping header synthesis for directory {:?}: could not locate a source token in donor arguments",
                        physical_dir
                    );
                    break;
                };

                result.push(Entry::with_arguments(
                    header_file,
                    rewritten,
                    rec.directory.clone(),
                    None::<PathBuf>,
                ));
            }
        }

        result
    }
}

/// Extracts `-I`/`-iquote` include directory values from `args`, in both
/// separate (`-I dir`, `-iquote dir`) and glued (`-Idir`, `-iquotedir`)
/// forms. Does not extract `-isystem` or `-idirafter` (nor any other flag):
/// only these two are eligible donors of in-project header synthesis.
fn extract_include_dirs(args: &[String]) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        if let Some(rest) = arg.strip_prefix("-iquote") {
            if rest.is_empty() {
                if let Some(dir) = iter.next() {
                    result.push(PathBuf::from(dir));
                }
            } else {
                result.push(PathBuf::from(rest));
            }
            continue;
        }
        if let Some(rest) = arg.strip_prefix("-I") {
            if rest.is_empty() {
                if let Some(dir) = iter.next() {
                    result.push(PathBuf::from(dir));
                }
            } else {
                result.push(PathBuf::from(rest));
            }
        }
    }

    result
}

/// Resolves the physical directory a source file lives in, given the entry's
/// working directory and (possibly relative) file path.
fn physical_parent(dir: &Path, file: &Path) -> PathBuf {
    let joined = if file.is_absolute() { file.to_path_buf() } else { dir.join(file) };
    joined.parent().map(Path::to_path_buf).unwrap_or_else(|| dir.to_path_buf())
}

/// Clone `args`, replace the source-file token with `to_file`, and drop the
/// output-file flag. Returns None when no source token can be located.
fn rewrite_arguments(args: &[String], from_file: &Path, to_file: &Path) -> Option<Vec<String>> {
    let idx = source_token_index(args, from_file)?;
    let mut out: Vec<String> = args.to_vec();
    out[idx] = to_file.to_string_lossy().into_owned();
    Some(remove_output_flag(out))
}

/// Index of the token to replace: the exact match for `from_file`, else the
/// unique token that looks like a source file. None if neither resolves.
fn source_token_index(args: &[String], from_file: &Path) -> Option<usize> {
    if let Some(idx) =
        args.iter().position(|a| Path::new(a) == from_file || a.as_str() == from_file.to_string_lossy())
    {
        return Some(idx);
    }

    let mut candidate = None;
    for (idx, arg) in args.iter().enumerate() {
        if looks_like_a_source_file(arg) {
            if candidate.is_some() {
                // More than one candidate: ambiguous, cannot resolve.
                return None;
            }
            candidate = Some(idx);
        }
    }
    candidate
}

/// Drop `-o <path>` (two tokens) and glued `-o<path>` (one token).
fn remove_output_flag(args: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        if arg == "-o" {
            iter.next();
            continue;
        }
        if arg.starts_with("-o") && arg.len() > 2 {
            continue;
        }
        out.push(arg);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::writers::fixtures::CollectingWriter;

    mod fixture {
        use tempfile::TempDir;

        /// Owns a temp directory laid out with a source file and a few
        /// siblings (headers and a non-header file), so tests can exercise
        /// disk-scanning behavior without repeating the scaffolding.
        pub(super) struct Fixture {
            pub dir: TempDir,
        }

        impl Fixture {
            pub(super) fn new() -> Self {
                let dir = tempfile::tempdir().unwrap();
                let src = dir.path().join("src");
                std::fs::create_dir_all(&src).unwrap();
                std::fs::write(src.join("main.c"), "").unwrap();
                std::fs::write(src.join("util.h"), "").unwrap();
                std::fs::write(src.join("helper.hpp"), "").unwrap();
                std::fs::write(src.join("readme.txt"), "").unwrap();
                Self { dir }
            }

            pub(super) fn path(&self) -> &std::path::Path {
                self.dir.path()
            }
        }
    }

    // --- rewrite_arguments ---

    #[test]
    fn test_rewrite_arguments() {
        struct Case {
            name: &'static str,
            args: Vec<&'static str>,
            from_file: &'static str,
            to_file: &'static str,
            expected: Option<Vec<&'static str>>,
        }

        let cases = vec![
            Case {
                name: "exact source token match",
                args: vec!["cc", "-c", "src/main.c", "-o", "src/main.o"],
                from_file: "src/main.c",
                to_file: "src/util.h",
                expected: Some(vec!["cc", "-c", "src/util.h"]),
            },
            Case {
                name: "glued output flag is dropped",
                args: vec!["cc", "-c", "src/main.c", "-osrc/main.o"],
                from_file: "src/main.c",
                to_file: "src/util.h",
                expected: Some(vec!["cc", "-c", "src/util.h"]),
            },
            Case {
                name: "separate output flag is dropped",
                args: vec!["cc", "-c", "src/main.c", "-o", "src/main.o"],
                from_file: "src/main.c",
                to_file: "src/helper.hpp",
                expected: Some(vec!["cc", "-c", "src/helper.hpp"]),
            },
            Case {
                name: "no matching source token yields None",
                args: vec!["cc", "-c", "-o", "src/main.o"],
                from_file: "src/main.c",
                to_file: "src/util.h",
                expected: None,
            },
        ];

        for case in cases {
            let args: Vec<String> = case.args.iter().map(|s| s.to_string()).collect();
            let from_file = Path::new(case.from_file);
            let to_file = Path::new(case.to_file);

            let sut = rewrite_arguments(&args, from_file, to_file);

            let expected = case.expected.map(|v| v.into_iter().map(String::from).collect::<Vec<_>>());
            assert_eq!(sut, expected, "case: {}", case.name);
            if let Some(actual) = &sut {
                assert!(!actual.iter().any(|a| a == "-o" || a.starts_with("-o") && a.len() > 2));
            }
        }
    }

    // --- HeaderEntrySynthesizer / siblings strategy ---

    fn enabled_config() -> Headers {
        Headers { enabled: true, strategy: HeaderStrategy::Siblings }
    }

    fn donor_entry(dir: &std::path::Path) -> Entry {
        Entry::from_arguments_str(
            "src/main.c",
            vec!["cc", "-c", "src/main.c", "-o", "src/main.o"],
            dir.to_str().unwrap(),
            None,
        )
    }

    #[test]
    fn test_synthesizes_header_entries_for_siblings() {
        let fixture = fixture::Fixture::new();
        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let sut = HeaderEntrySynthesizer::new(
            writer,
            enabled_config(),
            fixture.path().to_path_buf(),
            Arc::clone(&stats),
        );

        sut.write(vec![donor_entry(fixture.path())].into_iter()).unwrap();

        let out = collected.lock().unwrap();
        assert_eq!(out.len(), 3, "expected donor + 2 synthesized header entries: {:?}", *out);

        assert_eq!(out[0].file, PathBuf::from("src/main.c"));

        assert_eq!(out[1].file, PathBuf::from("src/helper.hpp"));
        assert_eq!(out[1].arguments, vec!["cc", "-c", "src/helper.hpp"]);
        assert_eq!(out[1].directory, fixture.path());
        assert_eq!(out[1].output, None);
        assert!(out[1].command.is_empty());

        assert_eq!(out[2].file, PathBuf::from("src/util.h"));
        assert_eq!(out[2].arguments, vec!["cc", "-c", "src/util.h"]);
        assert_eq!(out[2].directory, fixture.path());
        assert_eq!(out[2].output, None);
        assert!(out[2].command.is_empty());

        assert!(!out.iter().any(|e| e.file == Path::new("src/readme.txt")));
        assert_eq!(stats.entries_synthesized.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_disabled_by_default_passes_through_only() {
        let fixture = fixture::Fixture::new();
        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let config = Headers { enabled: false, strategy: HeaderStrategy::Siblings };
        let sut =
            HeaderEntrySynthesizer::new(writer, config, fixture.path().to_path_buf(), Arc::clone(&stats));

        sut.write(vec![donor_entry(fixture.path())].into_iter()).unwrap();

        let out = collected.lock().unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].file, PathBuf::from("src/main.c"));
        assert_eq!(stats.entries_synthesized.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_command_form_donor_is_not_eligible() {
        let fixture = fixture::Fixture::new();
        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let sut = HeaderEntrySynthesizer::new(
            writer,
            enabled_config(),
            fixture.path().to_path_buf(),
            Arc::clone(&stats),
        );

        let entry =
            Entry::from_command_str("src/main.c", "cc -c src/main.c", fixture.path().to_str().unwrap(), None);

        sut.write(vec![entry].into_iter()).unwrap();

        let out = collected.lock().unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(stats.entries_synthesized.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_non_c_family_donor_is_not_eligible() {
        let fixture = fixture::Fixture::new();
        std::fs::write(fixture.path().join("src").join("thing.swift"), "").unwrap();

        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let sut = HeaderEntrySynthesizer::new(
            writer,
            enabled_config(),
            fixture.path().to_path_buf(),
            Arc::clone(&stats),
        );

        let entry = Entry::from_arguments_str(
            "src/thing.swift",
            vec!["swiftc", "-c", "src/thing.swift"],
            fixture.path().to_str().unwrap(),
            None,
        );

        sut.write(vec![entry].into_iter()).unwrap();

        let out = collected.lock().unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(stats.entries_synthesized.load(Ordering::Relaxed), 0);
    }

    // --- extract_include_dirs ---

    #[test]
    fn test_extract_include_dirs() {
        struct Case {
            name: &'static str,
            args: Vec<&'static str>,
            expected: Vec<&'static str>,
        }

        let cases = vec![
            Case { name: "separate -I", args: vec!["-I", "inc"], expected: vec!["inc"] },
            Case { name: "glued -I", args: vec!["-Iinc"], expected: vec!["inc"] },
            Case { name: "separate -iquote", args: vec!["-iquote", "q"], expected: vec!["q"] },
            Case { name: "glued -iquote", args: vec!["-iquoteq"], expected: vec!["q"] },
            Case { name: "-isystem is not extracted", args: vec!["-isystem", "sys"], expected: vec![] },
            Case { name: "-idirafter is not extracted", args: vec!["-idirafter", "d"], expected: vec![] },
            Case {
                name: "mixed realistic arg list",
                args: vec![
                    "cc",
                    "-c",
                    "src/main.c",
                    "-Iinclude",
                    "-isystem",
                    "/usr/include",
                    "-iquote",
                    "q",
                    "-o",
                    "a.o",
                ],
                expected: vec!["include", "q"],
            },
            Case { name: "trailing bare -I with no arg", args: vec!["-I"], expected: vec![] },
        ];

        for case in cases {
            let args: Vec<String> = case.args.iter().map(|s| s.to_string()).collect();

            let sut = extract_include_dirs(&args);

            let expected: Vec<PathBuf> = case.expected.iter().map(PathBuf::from).collect();
            assert_eq!(sut, expected, "case: {}", case.name);
        }
    }

    // --- HeaderEntrySynthesizer / include-dirs strategy ---

    fn include_dirs_config() -> Headers {
        Headers { enabled: true, strategy: HeaderStrategy::IncludeDirs }
    }

    #[test]
    fn test_synthesizes_header_entries_for_include_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("include")).unwrap();
        std::fs::write(root.join("src").join("main.c"), "").unwrap();
        std::fs::write(root.join("include").join("util.h"), "").unwrap();

        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let sut = HeaderEntrySynthesizer::new(
            writer,
            include_dirs_config(),
            root.to_path_buf(),
            Arc::clone(&stats),
        );

        let entry = Entry::from_arguments_str(
            "src/main.c",
            vec!["cc", "-c", "src/main.c", "-Iinclude", "-o", "src/main.o"],
            root.to_str().unwrap(),
            None,
        );

        sut.write(vec![entry].into_iter()).unwrap();

        let out = collected.lock().unwrap();
        assert_eq!(out.len(), 2, "expected donor + 1 synthesized header entry: {:?}", *out);

        assert_eq!(out[0].file, PathBuf::from("src/main.c"));

        assert_eq!(out[1].file, PathBuf::from("include/util.h"));
        assert_eq!(out[1].arguments, vec!["cc", "-c", "include/util.h", "-Iinclude"]);
        assert_eq!(out[1].directory, root);
        assert_eq!(out[1].output, None);

        assert_eq!(stats.entries_synthesized.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_include_dirs_scope_excludes_out_of_project_and_isystem() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let outside = tempfile::tempdir().unwrap();

        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src").join("main.c"), "").unwrap();

        // In-project, but reached only via -isystem: never extracted, so its
        // header must not be synthesized.
        let sys_dir = root.join("sysinc");
        std::fs::create_dir_all(&sys_dir).unwrap();
        std::fs::write(sys_dir.join("sys.h"), "").unwrap();

        // Absolute, out-of-project include dir: extracted, but filtered out
        // by the project-root scope rule.
        std::fs::write(outside.path().join("outside.h"), "").unwrap();

        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let sut = HeaderEntrySynthesizer::new(
            writer,
            include_dirs_config(),
            root.to_path_buf(),
            Arc::clone(&stats),
        );

        let entry = Entry::from_arguments_str(
            "src/main.c",
            vec![
                "cc",
                "-c",
                "src/main.c",
                "-isystem",
                "sysinc",
                "-I",
                outside.path().to_str().unwrap(),
                "-o",
                "src/main.o",
            ],
            root.to_str().unwrap(),
            None,
        );

        sut.write(vec![entry].into_iter()).unwrap();

        let out = collected.lock().unwrap();
        assert_eq!(out.len(), 1, "expected donor only, no synthesized entries: {:?}", *out);
        assert_eq!(stats.entries_synthesized.load(Ordering::Relaxed), 0);
    }
}
