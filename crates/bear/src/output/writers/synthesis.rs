// SPDX-License-Identifier: GPL-3.0-or-later

//! Header entry synthesis for the output pipeline.
//!
//! When enabled via configuration, this stage clones the compile flags of a
//! compiled translation unit onto sibling header files discovered on disk, so
//! that editors and linters can resolve compile flags for headers as well as
//! sources. See `docs/requirements/output-header-entries.md`.
//!
//! Only the `Siblings` discovery strategy is implemented here: header files
//! that live in the same directory as a compiled C, C++, or Objective-C
//! source. The other strategies (`IncludeDirs`, `DependencyFiles`) are
//! recognized by configuration but currently forward entries unchanged.

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
    stats: Arc<OutputStatistics>,
}

impl<T: IteratorWriter<Entry>> HeaderEntrySynthesizer<T> {
    pub(crate) fn new(writer: T, config: Headers, stats: Arc<OutputStatistics>) -> Self {
        Self { writer, config, stats }
    }

    /// Siblings strategy: for each directory that donated at least one
    /// eligible source entry, scan the directory once (in the epilogue, after
    /// all entries have streamed through) and synthesize an entry for each
    /// header file found there, cloning the first-seen donor's arguments.
    fn write_siblings(self, entries: impl Iterator<Item = Entry>) -> Result<(), WriterError> {
        let collector = Rc::new(RefCell::new(SiblingCollector::default()));
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
            HeaderStrategy::Siblings => self.write_siblings(entries),
            // Implemented in later commits; forward untouched until then.
            HeaderStrategy::IncludeDirs | HeaderStrategy::DependencyFiles => self.writer.write(entries),
        }
    }
}

/// A translation unit whose arguments can be cloned onto a sibling header.
struct Donor {
    arguments: Vec<String>,
    directory: PathBuf,
    file: PathBuf,
}

/// Collects the first eligible donor observed per physical directory, then
/// synthesizes header entries for that directory's header files.
#[derive(Default)]
struct SiblingCollector {
    /// Keyed by the physical directory to scan for header siblings.
    donors: HashMap<PathBuf, Donor>,
}

impl SiblingCollector {
    /// Records `entry` as a donor candidate, keyed by its physical directory,
    /// unless it is command-form (no `arguments` to clone) or not a C-family
    /// source. First-seen donor wins per directory.
    fn observe(&mut self, entry: &Entry) {
        if entry.arguments.is_empty() {
            return;
        }
        if !is_c_family_source(&entry.file) {
            return;
        }

        let physical_dir = physical_parent(&entry.directory, &entry.file);
        self.donors.entry(physical_dir).or_insert_with(|| Donor {
            arguments: entry.arguments.clone(),
            directory: entry.directory.clone(),
            file: entry.file.clone(),
        });
    }

    /// Scans each recorded directory once and synthesizes an entry per header
    /// file found there, cloning that directory's donor arguments.
    fn synthesize(&self) -> Vec<Entry> {
        let mut result = Vec::new();

        for (physical_dir, donor) in &self.donors {
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
                let header_file = match donor.file.parent() {
                    Some(parent) => parent.join(&name),
                    None => PathBuf::from(&name),
                };

                let Some(rewritten) = rewrite_arguments(&donor.arguments, &donor.file, &header_file) else {
                    log::debug!(
                        "Skipping header synthesis for directory {:?}: could not locate a source token in donor arguments",
                        physical_dir
                    );
                    break;
                };

                result.push(Entry::with_arguments(
                    header_file,
                    rewritten,
                    donor.directory.clone(),
                    None::<PathBuf>,
                ));
            }
        }

        result
    }
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
        let sut = HeaderEntrySynthesizer::new(writer, enabled_config(), Arc::clone(&stats));

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
        let sut = HeaderEntrySynthesizer::new(writer, config, Arc::clone(&stats));

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
        let sut = HeaderEntrySynthesizer::new(writer, enabled_config(), Arc::clone(&stats));

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
        let sut = HeaderEntrySynthesizer::new(writer, enabled_config(), Arc::clone(&stats));

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
}
