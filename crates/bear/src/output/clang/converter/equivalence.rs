// SPDX-License-Identifier: GPL-3.0-or-later

//! Argv-driven regression tests for the converter: real argv goes through the
//! interpreters and the entries come out of the converter. The expected
//! entries are the ones the converter wrote before it read the steps, apart
//! from the two cases that changed on purpose (`gcc -S foo.s` and an unread
//! `@file`), which have their own tests.
//!
//! Requirements: output-compilation-entries, recognition-compiler-launchers, output-response-file-inlining, output-env-derived-flags

use super::super::Entry;
use super::super::path_format::FormatError;
use super::CommandConverter;
use crate::config::EntryFormat;
use intercept::Execution;
use semantic::interpreters::create;
use semantic::{CompilerHints, Interpreter, RecognizeResult, Recognized};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn resolver_identity(_base: &Path, path: &Path) -> Result<PathBuf, FormatError> {
    Ok(path.to_path_buf())
}

/// A scratch directory with the response files the cases refer to, used as
/// the working directory of every case.
struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("flags.rsp"), "-DFROM_FILE -c b.c").unwrap();
        std::fs::write(dir.path().join("args.rsp"), "/DFROM_FILE").unwrap();
        Self { dir }
    }

    fn directory(&self) -> &str {
        self.dir.path().to_str().unwrap()
    }

    fn recognize(
        &self,
        executable: &str,
        argv: &[&str],
        environment: &[(&str, &str)],
        inline_response_files: bool,
    ) -> Recognized {
        let environment: HashMap<&str, &str> = environment.iter().copied().collect();
        let execution = Execution::from_strings(executable, argv.to_vec(), self.directory(), environment);
        let sut = create(CompilerHints::new(), vec![], inline_response_files, true);
        match sut.recognize(execution) {
            RecognizeResult::Recognized(recognized) => recognized,
            other => panic!("expected Recognized for {argv:?}, got {other:?}"),
        }
    }

    /// The entries the converter writes, with the default format.
    fn convert(&self, recognized: &Recognized) -> Vec<Entry> {
        CommandConverter::from_resolvers(EntryFormat::default(), resolver_identity, resolver_identity)
            .convert(recognized)
    }

    /// The entries as `(file, arguments, output)`.
    fn entries(
        &self,
        executable: &str,
        argv: &[&str],
        environment: &[(&str, &str)],
        inline_response_files: bool,
    ) -> Vec<Expected> {
        let recognized = self.recognize(executable, argv, environment, inline_response_files);
        let directory = self.directory();
        self.convert(&recognized)
            .into_iter()
            .map(|entry| {
                assert_eq!(entry.directory, Path::new(directory));
                (
                    entry.file.to_string_lossy().to_string(),
                    entry.arguments,
                    entry.output.map(|output| output.to_string_lossy().to_string()),
                )
            })
            .collect()
    }
}

type Expected = (String, Vec<String>, Option<String>);

fn entry(file: &str, arguments: &[&str], output: Option<&str>) -> Expected {
    (file.to_string(), arguments.iter().map(|arg| arg.to_string()).collect(), output.map(String::from))
}

struct Case {
    name: &'static str,
    executable: &'static str,
    argv: Vec<&'static str>,
    environment: Vec<(&'static str, &'static str)>,
    expected: Vec<Expected>,
}

fn case(
    name: &'static str,
    executable: &'static str,
    argv: &[&'static str],
    expected: Vec<Expected>,
) -> Case {
    Case { name, executable, argv: argv.to_vec(), environment: Vec::new(), expected }
}

fn with_environment(mut case: Case, environment: &[(&'static str, &'static str)]) -> Case {
    case.environment = environment.to_vec();
    case
}

/// Cases whose entries do not depend on whether response files are read.
fn cases() -> Vec<Case> {
    let gcc = "/usr/bin/gcc";
    vec![
        case(
            "gcc multi-source",
            gcc,
            &["gcc", "-c", "a.c", "b.c"],
            vec![entry("a.c", &[gcc, "-c", "a.c"], None), entry("b.c", &[gcc, "-c", "b.c"], None)],
        ),
        case(
            "gcc compile and link",
            gcc,
            &["gcc", "-L/x", "-lm", "-O2", "-o", "app", "a.c", "b.c"],
            vec![
                entry("a.c", &[gcc, "-O2", "-o", "app", "a.c"], Some("app")),
                entry("b.c", &[gcc, "-O2", "-o", "app", "b.c"], Some("app")),
            ],
        ),
        case("gcc link only", gcc, &["gcc", "-o", "app", "a.o", "b.o", "-lm"], vec![]),
        case("gcc preprocess only", gcc, &["gcc", "-E", "a.c"], vec![]),
        case("gcc version", gcc, &["gcc", "--version"], vec![]),
        case("gcc dependencies only", gcc, &["gcc", "-M", "a.c"], vec![]),
        case(
            "gcc dependency file and compile",
            gcc,
            &["gcc", "-MD", "-c", "a.c"],
            vec![entry("a.c", &[gcc, "-MD", "-c", "a.c"], None)],
        ),
        case("gcc header", gcc, &["gcc", "-c", "a.h"], vec![entry("a.h", &[gcc, "-c", "a.h"], None)]),
        case(
            "gcc header beside a source",
            gcc,
            &["gcc", "-c", "a.c", "util.h"],
            vec![entry("a.c", &[gcc, "-c", "a.c"], None), entry("util.h", &[gcc, "-c", "util.h"], None)],
        ),
        case(
            "gcc object beside sources",
            gcc,
            &["gcc", "-o", "app", "a.c", "b.o", "c.c"],
            vec![
                entry("a.c", &[gcc, "-o", "app", "a.c"], Some("app")),
                entry("c.c", &[gcc, "-o", "app", "c.c"], Some("app")),
            ],
        ),
        case(
            "gcc glued output",
            gcc,
            &["gcc", "-omain.o", "-c", "a.c"],
            vec![entry("a.c", &[gcc, "-omain.o", "-c", "a.c"], Some("main.o"))],
        ),
        case("gcc assembly", gcc, &["gcc", "-c", "foo.s"], vec![entry("foo.s", &[gcc, "-c", "foo.s"], None)]),
        case(
            "gcc assembly with preprocessor",
            gcc,
            &["gcc", "-c", "foo.S"],
            vec![entry("foo.S", &[gcc, "-c", "foo.S"], None)],
        ),
        case(
            "gcc preprocessed source",
            gcc,
            &["gcc", "-c", "a.i"],
            vec![entry("a.i", &[gcc, "-c", "a.i"], None)],
        ),
        case("gcc object with compile flag", gcc, &["gcc", "-c", "a.o"], vec![]),
        case(
            "gcc syntax only",
            gcc,
            &["gcc", "-fsyntax-only", "a.c"],
            vec![entry("a.c", &[gcc, "-fsyntax-only", "a.c"], None)],
        ),
        case(
            "msvc compile and link flags",
            "cl",
            &["cl", "/c", "/Fo:x.obj", "a.cpp", "/link", "foo.lib"],
            vec![entry("a.cpp", &["cl", "/c", "/Fo:x.obj", "a.cpp"], Some("x.obj"))],
        ),
        with_environment(
            case(
                "msvc environment",
                "cl",
                &["cl", "/c", "a.cpp"],
                vec![entry("a.cpp", &["cl", "/O2", "/W4", "/c", "a.cpp", "/DX"], None)],
            ),
            &[("CL", "/O2 /W4"), ("_CL_", "/DX")],
        ),
        with_environment(
            case(
                "gcc include path variable",
                gcc,
                &["gcc", "-c", "a.c"],
                vec![entry("a.c", &[gcc, "-c", "a.c", "-I", "/inc"], None)],
            ),
            &[("CPATH", "/inc")],
        ),
        case(
            "swift multi-source",
            "/usr/bin/swiftc",
            &["swiftc", "-module-name", "m", "a.swift", "b.swift", "-o", "app"],
            vec![
                entry(
                    "a.swift",
                    &["/usr/bin/swiftc", "-module-name", "m", "a.swift", "b.swift", "-o", "app"],
                    Some("app"),
                ),
                entry(
                    "b.swift",
                    &["/usr/bin/swiftc", "-module-name", "m", "a.swift", "b.swift", "-o", "app"],
                    Some("app"),
                ),
            ],
        ),
        case(
            "valac combined",
            "/usr/bin/valac",
            &["valac", "--library", "foo", "a.vala", "b.vala"],
            vec![entry("a.vala", &["/usr/bin/valac", "--library", "foo", "a.vala", "b.vala"], None)],
        ),
        case(
            "ccache",
            "/usr/bin/ccache",
            &["ccache", "gcc", "-c", "a.c"],
            vec![entry("a.c", &["gcc", "-c", "a.c"], None)],
        ),
        case(
            "distcc",
            "/usr/bin/distcc",
            &["distcc", "-j", "4", "gcc", "-c", "a.c"],
            vec![entry("a.c", &["gcc", "-c", "a.c"], None)],
        ),
    ]
}

#[test]
fn the_converter_writes_the_same_entries_whether_or_not_response_files_are_read() {
    for case in cases() {
        for inline in [false, true] {
            let fixture = Fixture::new();

            let sut = fixture.entries(case.executable, &case.argv, &case.environment, inline);

            assert_eq!(sut, case.expected, "case {:?} with response files inlined: {inline}", case.name);
        }
    }
}

#[test]
fn the_cases_are_not_all_empty() {
    // A guard against the comparison passing because every expectation is empty.
    let total: usize = cases().iter().map(|case| case.expected.len()).sum();

    assert!(total >= 20, "only {total} entries across all cases");
}

// Requirements: output-response-file-inlining
#[test]
fn a_read_response_file_is_part_of_the_entries() {
    let gcc = "/usr/bin/gcc";
    let fixture = Fixture::new();

    let sut = fixture.entries(gcc, &["gcc", "@flags.rsp", "a.c"], &[], true);

    assert_eq!(
        sut,
        [
            entry("b.c", &[gcc, "-DFROM_FILE", "-c", "b.c"], None),
            entry("a.c", &[gcc, "-DFROM_FILE", "-c", "a.c"], None),
        ]
    );
}

// Requirements: output-response-file-inlining
#[test]
fn an_unread_response_file_stays_in_the_entry_for_gcc() {
    let fixture = Fixture::new();

    let sut = fixture.entries("/usr/bin/gcc", &["gcc", "@flags.rsp", "a.c"], &[], false);

    assert_eq!(sut, [entry("a.c", &["/usr/bin/gcc", "@flags.rsp", "a.c"], None)]);
}

// Requirements: output-response-file-inlining
#[test]
fn a_read_response_file_is_part_of_the_entry_for_msvc() {
    let fixture = Fixture::new();

    let sut = fixture.entries("cl", &["cl", "@args.rsp", "/c", "a.cpp"], &[], true);

    assert_eq!(sut, [entry("a.cpp", &["cl", "/DFROM_FILE", "/c", "a.cpp"], None)]);
}

// Requirements: output-response-file-inlining
// The converter used to drop the token here (bug bear-zq3.2).
#[test]
fn an_unread_response_file_stays_in_the_entry_for_msvc() {
    let fixture = Fixture::new();

    let sut = fixture.entries("cl", &["cl", "@args.rsp", "/c", "a.cpp"], &[], false);

    assert_eq!(sut, [entry("a.cpp", &["cl", "@args.rsp", "/c", "a.cpp"], None)]);
}

// Requirements: output-compilation-entries
// gcc does nothing for it, and the converter used to write an entry.
#[test]
fn assembling_an_assembly_file_gets_no_entry() {
    for inline in [false, true] {
        let fixture = Fixture::new();

        let sut = fixture.entries("/usr/bin/gcc", &["gcc", "-S", "foo.s"], &[], inline);

        assert!(sut.is_empty(), "entries: {sut:?}");
    }
}
