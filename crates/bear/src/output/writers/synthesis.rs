// SPDX-License-Identifier: GPL-3.0-or-later

//! Header entry synthesis for the output pipeline.
//!
//! When enabled via configuration, this stage clones the compile flags of a
//! compiled translation unit onto header files discovered on disk, so that
//! editors and linters can resolve compile flags for headers as well as
//! sources. See `docs/requirements/output-header-entries.md`.
//!
//! Three discovery strategies are implemented here:
//!
//! - `Siblings`: header files that live in the same directory as a compiled
//!   C, C++, or Objective-C source.
//! - `IncludeDirs`: a superset of `Siblings` that also scans a donor's own
//!   `-I`/`-iquote` include directories, but only those that resolve inside
//!   the compilation's own working directory (the frame the compiler resolves
//!   relative includes against), to avoid flooding the database with system
//!   headers.
//! - `DependencyFiles`: reads the make-style dependency file (`.d`) a
//!   donor's build already emitted, and synthesizes an entry per header
//!   prerequisite listed there, scoped to headers that resolve inside the
//!   compilation's working directory.
//!
//! All three strategies stream: entries observed as they pass through are
//! recorded, and the synthesized header entries are emitted once, in an
//! epilogue, after the input iterator is exhausted. See
//! [`HeaderCollector`].

use crate::config::{HeaderStrategy, Headers};
use crate::output::WriterError;
use crate::output::clang::Entry;
use crate::output::statistics::OutputStatistics;
use crate::semantic::interpreters::matchers::{is_c_family_source, is_header_file, looks_like_a_source_file};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::IteratorWriter;

/// Collects donor observations while entries stream through, then yields the
/// synthesized header entries once the stream is exhausted. Implementations
/// hold whatever state they need (e.g. one donor per directory, or a
/// dedup set) so that memory stays proportional to the number of donors or
/// directories considered, not to the number of entries in the database.
trait HeaderCollector {
    /// Called once per entry as it streams through, in order.
    fn observe(&mut self, entry: &Entry);

    /// Called once, after all entries have been observed, to produce the
    /// synthesized header entries.
    fn synthesize(&self) -> Vec<Entry>;
}

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

    /// Drives `collector` over `entries`: each entry is observed as it
    /// streams through, and once the stream is exhausted, an epilogue asks
    /// the collector to synthesize the header entries and appends them.
    fn write_with_collector(
        self,
        entries: impl Iterator<Item = Entry>,
        collector: impl HeaderCollector + 'static,
    ) -> Result<(), WriterError> {
        let collector = Rc::new(RefCell::new(collector));
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

        let strategy = self.config.strategy;

        match strategy {
            HeaderStrategy::Siblings | HeaderStrategy::IncludeDirs => {
                self.write_with_collector(entries, DirectoryCollector::new(strategy))
            }
            HeaderStrategy::DependencyFiles => self.write_with_collector(entries, DepFilesCollector::new()),
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
    /// Keyed by the physical directory to scan for headers.
    records: HashMap<PathBuf, DirRecord>,
}

impl DirectoryCollector {
    fn new(strategy: HeaderStrategy) -> Self {
        Self { strategy, records: HashMap::new() }
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
            // The compilation's own working directory is the reference frame:
            // the compiler resolves relative include paths against it, so an
            // include dir is in scope only when it resolves inside that
            // directory. Absolute system paths and `..` sequences that escape
            // it are left out, so the database is not flooded with headers the
            // compilation did not treat as project-local.
            let base = lexical_normalize(&entry.directory);
            for inc in extract_include_dirs(&entry.arguments) {
                let raw = if inc.is_absolute() { inc.clone() } else { entry.directory.join(&inc) };
                let physical = lexical_normalize(&raw);
                if !physical.starts_with(&base) {
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
                let header_file = join_header_path(&rec.display_dir, &name, &rec.source_file);

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

impl HeaderCollector for DirectoryCollector {
    fn observe(&mut self, entry: &Entry) {
        DirectoryCollector::observe(self, entry)
    }

    fn synthesize(&self) -> Vec<Entry> {
        DirectoryCollector::synthesize(self)
    }
}

/// One eligible donor: its own compile flags, and the dependency file its
/// build is expected to have emitted. The dependency file is a build
/// artifact that does not exist yet at the time its compiler invocation is
/// intercepted (interception happens before the compiler runs, not after),
/// so it must be read later, in the epilogue - never during `observe`.
struct DepRecord {
    arguments: Vec<String>,
    directory: PathBuf,
    source_file: PathBuf,
    dep_path: PathBuf,
}

/// Collects, per eligible donor, the dependency file (`.d`) its build is
/// expected to emit, then - once all entries have streamed through, in the
/// epilogue - reads each recorded dependency file and synthesizes an entry
/// for each in-project header prerequisite listed there. Unlike
/// [`DirectoryCollector`], no directory is scanned: the exact header set
/// comes from the dependency file, so headers reached from other
/// directories (e.g. via `-I`) are picked up precisely, without scanning
/// those directories wholesale.
struct DepFilesCollector {
    donors: Vec<DepRecord>,
}

impl DepFilesCollector {
    fn new() -> Self {
        Self { donors: Vec::new() }
    }
}

impl HeaderCollector for DepFilesCollector {
    /// Records `entry` as a donor, with the dependency file its build is
    /// expected to have emitted, unless it is command-form (no `arguments`
    /// to clone), not a C-family source, or its arguments do not locate a
    /// dependency file at all.
    fn observe(&mut self, entry: &Entry) {
        if entry.arguments.is_empty() {
            return;
        }
        if !is_c_family_source(&entry.file) {
            return;
        }

        let Some(dep_rel) = locate_dep_file(&entry.arguments) else {
            return;
        };
        let dep_path = if dep_rel.is_absolute() { dep_rel } else { entry.directory.join(&dep_rel) };

        self.donors.push(DepRecord {
            arguments: entry.arguments.clone(),
            directory: entry.directory.clone(),
            source_file: entry.file.clone(),
            dep_path,
        });
    }

    /// Reads each recorded donor's dependency file once and synthesizes an
    /// entry per in-project header prerequisite listed there, deduplicated
    /// across donors that share a header.
    fn synthesize(&self) -> Vec<Entry> {
        let mut seen: HashSet<(PathBuf, PathBuf)> = HashSet::new();
        let mut result = Vec::new();

        for donor in &self.donors {
            let content = match std::fs::read_to_string(&donor.dep_path) {
                Ok(content) => content,
                Err(err) => {
                    log::debug!(
                        "Skipping header synthesis for dependency file {:?}: {}",
                        donor.dep_path,
                        err
                    );
                    continue;
                }
            };

            // The compilation's own working directory is the reference frame:
            // dependency-file paths are relative to it, and a header is in
            // scope only when it resolves inside that directory. System headers
            // (absolute, or reached via `..`) are left out.
            let base = lexical_normalize(&donor.directory);

            for prereq in parse_make_prerequisites(&content) {
                if !is_header_file(&prereq) {
                    // The source file itself is also listed as a prerequisite.
                    continue;
                }

                let raw = if prereq.is_absolute() { prereq.clone() } else { donor.directory.join(&prereq) };
                let phys = lexical_normalize(&raw);
                if !phys.starts_with(&base) {
                    continue;
                }

                if !seen.insert((donor.directory.clone(), prereq.clone())) {
                    continue;
                }

                if let Some(args) = rewrite_arguments(&donor.arguments, &donor.source_file, &prereq) {
                    result.push(Entry::with_arguments(
                        prereq.clone(),
                        args,
                        donor.directory.clone(),
                        None::<PathBuf>,
                    ));
                }
            }
        }

        result
    }
}

/// Locates the dependency file a donor's build would have emitted, from its
/// own compile arguments: prefers an explicit `-MF <path>` (or glued
/// `-MF<path>`); else derives it from the object output (`-o <path>` ->
/// `<path>.d`); else from the target name (`-MT <path>` -> `<path>.d`).
/// Returns `None` when no dependency file can be located.
fn locate_dep_file(args: &[String]) -> Option<PathBuf> {
    if let Some(value) = extract_flag_value(args, "-MF") {
        return Some(PathBuf::from(value));
    }
    if let Some(value) = extract_flag_value(args, "-o") {
        return Some(PathBuf::from(value).with_extension("d"));
    }
    if let Some(value) = extract_flag_value(args, "-MT") {
        return Some(PathBuf::from(value).with_extension("d"));
    }
    None
}

/// Extracts the value of `flag` from `args`, in both separate (`flag value`)
/// and glued (`flagvalue`) forms. Returns the first match.
fn extract_flag_value(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        if arg == flag {
            return iter.next().cloned();
        }
        if let Some(rest) = arg.strip_prefix(flag)
            && !rest.is_empty()
        {
            return Some(rest.to_string());
        }
    }

    None
}

/// Parses the prerequisites of the first rule in a make-style dependency
/// file (as emitted by `-MD`/`-MMD`/`-MF`). `-MP` emits extra phony rules
/// for each header on later lines (`header:` with no prerequisites), which
/// are not the first rule and are ignored here.
fn parse_make_prerequisites(content: &str) -> Vec<PathBuf> {
    // Unfold line continuations (backslash-newline) into spaces so the
    // first rule becomes one logical line.
    let unfolded = content.replace("\\\r\n", " ").replace("\\\n", " ");
    let first_line = unfolded.lines().next().unwrap_or("");
    let Some((_, prereqs)) = first_line.split_once(':') else { return Vec::new() };

    // Tokenize on unescaped whitespace, treating "\ " as a literal space
    // (make escapes spaces in filenames this way).
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = prereqs.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&' ') => {
                current.push(' ');
                chars.next();
            }
            c if c.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(PathBuf::from(std::mem::take(&mut current)));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(PathBuf::from(current));
    }

    tokens
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

/// Lexically normalizes a path by folding `.` and `..` components without
/// touching the filesystem. The project-root scope check relies on this so a
/// `..` sequence cannot textually escape the root (e.g. an include dir of
/// `-I../../shared` resolving outside the project must not pass the guard).
/// Symlinks are deliberately not resolved: that needs disk access and would
/// fail on directories the build has not created.
fn lexical_normalize(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(out.components().next_back(), Some(Component::Normal(_))) {
                    out.pop();
                } else {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Joins a directory prefix and a header file name into the header's path.
///
/// The separator mirrors the donor's own paths rather than the host OS
/// separator, so a synthesized entry reads like the compilation it clones and
/// its `file`/argument strings stay stable across platforms (`PathBuf::join`
/// would emit a backslash on Windows even when the donor used `/`). POSIX `/`
/// is the default; `dir`'s own separators, when it has any, are left intact.
/// `sample` is the donor's source file, consulted for the style only when
/// `dir` is a single component with no separator of its own.
fn join_header_path(dir: &Path, name: &std::ffi::OsStr, sample: &Path) -> PathBuf {
    if dir.as_os_str().is_empty() {
        return PathBuf::from(name);
    }
    let dir = dir.to_string_lossy();
    let backslash = if dir.contains('/') {
        false
    } else if dir.contains('\\') {
        true
    } else {
        let sample = sample.to_string_lossy();
        sample.contains('\\') && !sample.contains('/')
    };
    let separator = if backslash { '\\' } else { '/' };
    PathBuf::from(format!("{dir}{separator}{}", name.to_string_lossy()))
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

    // --- join_header_path ---

    #[test]
    fn test_join_header_path_mirrors_donor_separator() {
        struct Case {
            name: &'static str,
            dir: &'static str,
            file: &'static str,
            sample: &'static str,
            expected: &'static str,
        }

        // String-based, so the outcome is the same on every platform (this is
        // the whole point: not the host OS separator). Verifies the Windows
        // behavior from any host.
        let cases = [
            Case {
                name: "forward-slash donor keeps forward slashes",
                dir: "src",
                file: "util.h",
                sample: "src/main.c",
                expected: "src/util.h",
            },
            Case {
                name: "backslash donor keeps backslashes",
                dir: "src",
                file: "util.h",
                sample: "src\\main.c",
                expected: "src\\util.h",
            },
            Case {
                name: "include-dir prefix keeps forward slashes",
                dir: "include",
                file: "util.h",
                sample: "src/main.c",
                expected: "include/util.h",
            },
            Case {
                name: "separator already in the prefix is respected",
                dir: "a/b",
                file: "x.h",
                sample: "irrelevant",
                expected: "a/b/x.h",
            },
            Case {
                name: "empty prefix yields the bare name",
                dir: "",
                file: "util.h",
                sample: "main.c",
                expected: "util.h",
            },
        ];

        for case in cases {
            let sut = join_header_path(
                Path::new(case.dir),
                std::ffi::OsStr::new(case.file),
                Path::new(case.sample),
            );

            assert_eq!(sut.to_string_lossy(), case.expected, "case: {}", case.name);
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
        let sut = HeaderEntrySynthesizer::new(writer, include_dirs_config(), Arc::clone(&stats));

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
        let sut = HeaderEntrySynthesizer::new(writer, include_dirs_config(), Arc::clone(&stats));

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

    #[test]
    fn test_include_dirs_scope_rejects_parent_dir_escape() {
        // The compilation runs in `work`. An include dir under it is in scope;
        // one that escapes via `..` resolves outside `work` and must be
        // rejected even though it textually shares the prefix before its `..`
        // components are folded.
        let root = tempfile::tempdir().unwrap();
        let work = root.path().join("work");
        std::fs::create_dir_all(work.join("local")).unwrap();
        std::fs::write(work.join("main.c"), "").unwrap();
        std::fs::write(work.join("local").join("kept.h"), "").unwrap();

        let escaped = root.path().join("outside");
        std::fs::create_dir_all(&escaped).unwrap();
        std::fs::write(escaped.join("escaped.h"), "").unwrap();

        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let sut = HeaderEntrySynthesizer::new(writer, include_dirs_config(), Arc::clone(&stats));

        // From work/: `local` stays inside (kept); `../outside` -> root/outside
        // escapes the working directory (rejected).
        let entry = Entry::from_arguments_str(
            "main.c",
            vec!["cc", "-c", "main.c", "-Ilocal", "-I../outside", "-o", "main.o"],
            work.to_str().unwrap(),
            None,
        );

        sut.write(vec![entry].into_iter()).unwrap();

        let out = collected.lock().unwrap();
        assert_eq!(out.len(), 2, "expected donor + only the in-directory header: {:?}", *out);
        assert!(
            out.iter().any(|e| e.file.as_path() == Path::new("local/kept.h")),
            "header inside the working directory kept"
        );
        assert!(
            !out.iter().any(|e| e.file.file_name() == Some(std::ffi::OsStr::new("escaped.h"))),
            "header reached via .. outside the working directory must not be synthesized"
        );
        assert_eq!(stats.entries_synthesized.load(Ordering::Relaxed), 1);
    }

    // --- parse_make_prerequisites ---

    #[test]
    fn test_parse_make_prerequisites() {
        struct Case {
            name: &'static str,
            content: &'static str,
            expected: Vec<&'static str>,
        }

        let cases = vec![
            Case {
                name: "single line",
                content: "main.o: main.c util.h\n",
                expected: vec!["main.c", "util.h"],
            },
            Case {
                name: "line continuation joins two lines",
                content: "main.o: main.c \\\n  util.h\n",
                expected: vec!["main.c", "util.h"],
            },
            Case {
                name: "escaped space in a filename",
                content: "main.o: main.c a\\ b.h\n",
                expected: vec!["main.c", "a b.h"],
            },
            Case {
                name: "-MP phony rules on later lines are ignored",
                content: "main.o: main.c util.h\n\nutil.h:\n",
                expected: vec!["main.c", "util.h"],
            },
            Case { name: "no ':' yields empty", content: "not a rule at all\n", expected: vec![] },
        ];

        for case in cases {
            let sut = parse_make_prerequisites(case.content);

            let expected: Vec<PathBuf> = case.expected.iter().map(PathBuf::from).collect();
            assert_eq!(sut, expected, "case: {}", case.name);
        }
    }

    // --- locate_dep_file ---

    #[test]
    fn test_locate_dep_file() {
        struct Case {
            name: &'static str,
            args: Vec<&'static str>,
            expected: Option<&'static str>,
        }

        let cases = vec![
            Case {
                name: "explicit -MF",
                args: vec!["cc", "-c", "main.c", "-MF", "dep.d", "-o", "main.o"],
                expected: Some("dep.d"),
            },
            Case {
                name: "glued -MF",
                args: vec!["cc", "-c", "main.c", "-MFdep.d", "-o", "main.o"],
                expected: Some("dep.d"),
            },
            Case {
                name: "derived from -o",
                args: vec!["cc", "-c", "main.c", "-o", "main.o"],
                expected: Some("main.d"),
            },
            Case {
                name: "derived from -MT",
                args: vec!["cc", "-c", "main.c", "-MT", "main.o"],
                expected: Some("main.d"),
            },
            Case { name: "none present", args: vec!["cc", "-c", "main.c"], expected: None },
            Case {
                name: "-MF wins over -o",
                args: vec!["cc", "-c", "main.c", "-MF", "custom.d", "-o", "main.o"],
                expected: Some("custom.d"),
            },
        ];

        for case in cases {
            let args: Vec<String> = case.args.iter().map(|s| s.to_string()).collect();

            let sut = locate_dep_file(&args);

            let expected = case.expected.map(PathBuf::from);
            assert_eq!(sut, expected, "case: {}", case.name);
        }
    }

    // --- HeaderEntrySynthesizer / dependency-files strategy ---

    fn dependency_files_config() -> Headers {
        Headers { enabled: true, strategy: HeaderStrategy::DependencyFiles }
    }

    #[test]
    fn test_synthesizes_header_entries_for_dependency_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src").join("main.c"), "").unwrap();
        std::fs::write(root.join("src").join("util.h"), "").unwrap();
        std::fs::write(root.join("main.d"), "main.o: src/main.c src/util.h /usr/include/stdio.h\n").unwrap();

        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let sut = HeaderEntrySynthesizer::new(writer, dependency_files_config(), Arc::clone(&stats));

        let entry = Entry::from_arguments_str(
            "src/main.c",
            vec!["cc", "-c", "src/main.c", "-MF", "main.d", "-o", "src/main.o"],
            root.to_str().unwrap(),
            None,
        );

        sut.write(vec![entry].into_iter()).unwrap();

        let out = collected.lock().unwrap();
        assert_eq!(out.len(), 2, "expected donor + 1 synthesized header entry: {:?}", *out);

        assert_eq!(out[0].file, PathBuf::from("src/main.c"));

        assert_eq!(out[1].file, PathBuf::from("src/util.h"));
        assert_eq!(out[1].arguments, vec!["cc", "-c", "src/util.h", "-MF", "main.d"]);
        assert_eq!(out[1].directory, root);
        assert_eq!(out[1].output, None);

        assert_eq!(stats.entries_synthesized.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_dependency_files_missing_dep_file_synthesizes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src").join("main.c"), "").unwrap();

        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let sut = HeaderEntrySynthesizer::new(writer, dependency_files_config(), Arc::clone(&stats));

        let entry = Entry::from_arguments_str(
            "src/main.c",
            vec!["cc", "-c", "src/main.c", "-MF", "missing.d", "-o", "src/main.o"],
            root.to_str().unwrap(),
            None,
        );

        sut.write(vec![entry].into_iter()).unwrap();

        let out = collected.lock().unwrap();
        assert_eq!(out.len(), 1, "expected donor only, no synthesized entries: {:?}", *out);
        assert_eq!(stats.entries_synthesized.load(Ordering::Relaxed), 0);
    }
}
