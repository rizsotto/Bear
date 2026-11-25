// SPDX-License-Identifier: GPL-3.0-or-later

//! The entries the converter writes today, against the entries the
//! interpretation's steps give for the same argv. The converter still reads
//! `Command`; a later change switches it to `Interpretation::steps()`, and
//! this module says what must stay the same when it does.
//!
//! Requirements: output-compilation-entries, recognition-compiler-launchers, output-response-file-inlining, output-env-derived-flags

use super::super::Entry;
use super::super::path_format::FormatError;
use super::CommandConverter;
use crate::config::EntryFormat;
use intercept::Execution;
use semantic::interpreters::create;
use semantic::{CompilerHints, FileRef, Interpreter, RecognizeResult, Recognized, Role, Step};
use std::borrow::Cow;
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

    fn recognize(
        &self,
        executable: &str,
        argv: &[&str],
        environment: &[(&str, &str)],
        inline_response_files: bool,
    ) -> Recognized {
        let environment: HashMap<&str, &str> = environment.iter().copied().collect();
        let execution = Execution::from_strings(
            executable,
            argv.to_vec(),
            self.dir.path().to_str().unwrap(),
            environment,
        );
        let sut = create(CompilerHints::new(), vec![], inline_response_files, true);
        match sut.recognize(execution) {
            RecognizeResult::Recognized(recognized) => recognized,
            other => panic!("expected Recognized for {argv:?}, got {other:?}"),
        }
    }
}

/// What the converter writes today.
fn from_command(recognized: &Recognized) -> Vec<Entry> {
    CommandConverter::from_resolvers(EntryFormat::default(), resolver_identity, resolver_identity)
        .convert(recognized)
}

/// What the steps give: per compile step, the source, the items with the
/// launcher dropped and the compiler written as its path, and the first output.
fn from_steps(recognized: &Recognized) -> Vec<Entry> {
    let interpretation = &recognized.interpretation;
    let directory = &recognized.execution.working_dir;
    let identity: &dyn Fn(&Path) -> Cow<Path> = &|path| Cow::Borrowed(path);
    let output = interpretation.outputs().next();
    interpretation
        .steps()
        .into_iter()
        .filter_map(|step| match step {
            Step::Compile { source: FileRef::Resolved { path, .. }, arguments } => {
                let arguments: Vec<String> = arguments
                    .iter()
                    .flat_map(|item| match &item.role {
                        Role::Launcher => Vec::new(),
                        Role::Compiler { path } => vec![path.to_string_lossy().to_string()],
                        _ => item.as_arguments(identity),
                    })
                    .collect();
                Some(Entry::with_arguments(path.clone(), arguments, directory.clone(), output))
            }
            _ => None,
        })
        .collect()
}

struct Case {
    name: &'static str,
    executable: &'static str,
    argv: Vec<&'static str>,
    environment: Vec<(&'static str, &'static str)>,
}

fn case(name: &'static str, executable: &'static str, argv: &[&'static str]) -> Case {
    Case { name, executable, argv: argv.to_vec(), environment: Vec::new() }
}

fn with_environment(mut case: Case, environment: &[(&'static str, &'static str)]) -> Case {
    case.environment = environment.to_vec();
    case
}

fn cases() -> Vec<Case> {
    vec![
        case("gcc multi-source", "/usr/bin/gcc", &["gcc", "-c", "a.c", "b.c"]),
        case(
            "gcc compile and link",
            "/usr/bin/gcc",
            &["gcc", "-L/x", "-lm", "-O2", "-o", "app", "a.c", "b.c"],
        ),
        case("gcc link only", "/usr/bin/gcc", &["gcc", "-o", "app", "a.o", "b.o", "-lm"]),
        case("gcc preprocess only", "/usr/bin/gcc", &["gcc", "-E", "a.c"]),
        case("gcc version", "/usr/bin/gcc", &["gcc", "--version"]),
        case("gcc dependencies only", "/usr/bin/gcc", &["gcc", "-M", "a.c"]),
        case("gcc dependency file and compile", "/usr/bin/gcc", &["gcc", "-MD", "-c", "a.c"]),
        case("gcc header", "/usr/bin/gcc", &["gcc", "-c", "a.h"]),
        case("gcc header beside a source", "/usr/bin/gcc", &["gcc", "-c", "a.c", "util.h"]),
        case("gcc object beside sources", "/usr/bin/gcc", &["gcc", "-o", "app", "a.c", "b.o", "c.c"]),
        case("gcc glued output", "/usr/bin/gcc", &["gcc", "-omain.o", "-c", "a.c"]),
        case("gcc assembly", "/usr/bin/gcc", &["gcc", "-c", "foo.s"]),
        case("gcc assembly with preprocessor", "/usr/bin/gcc", &["gcc", "-c", "foo.S"]),
        case("gcc preprocessed source", "/usr/bin/gcc", &["gcc", "-c", "a.i"]),
        case("gcc object with compile flag", "/usr/bin/gcc", &["gcc", "-c", "a.o"]),
        case("gcc syntax only", "/usr/bin/gcc", &["gcc", "-fsyntax-only", "a.c"]),
        case("msvc compile and link flags", "cl", &["cl", "/c", "/Fo:x.obj", "a.cpp", "/link", "foo.lib"]),
        with_environment(
            case("msvc environment", "cl", &["cl", "/c", "a.cpp"]),
            &[("CL", "/O2 /W4"), ("_CL_", "/DX")],
        ),
        with_environment(
            case("gcc include path variable", "/usr/bin/gcc", &["gcc", "-c", "a.c"]),
            &[("CPATH", "/inc")],
        ),
        case(
            "swift multi-source",
            "/usr/bin/swiftc",
            &["swiftc", "-module-name", "m", "a.swift", "b.swift", "-o", "app"],
        ),
        case("valac combined", "/usr/bin/valac", &["valac", "--library", "foo", "a.vala", "b.vala"]),
        case("ccache", "/usr/bin/ccache", &["ccache", "gcc", "-c", "a.c"]),
        case("distcc", "/usr/bin/distcc", &["distcc", "-j", "4", "gcc", "-c", "a.c"]),
        case("gcc response file", "/usr/bin/gcc", &["gcc", "@flags.rsp", "a.c"]),
        case("msvc response file", "cl", &["cl", "@args.rsp", "/c", "a.cpp"]),
    ]
}

#[test]
fn steps_give_the_entries_the_converter_writes() {
    // One listed case differs with inlining off and is asserted apart below;
    // `gcc -S foo.s` has its own test.
    let differs = ["msvc response file"];
    for case in cases() {
        for inline in [false, true] {
            if differs.contains(&case.name) && !inline {
                continue;
            }
            let fixture = Fixture::new();
            let recognized = fixture.recognize(case.executable, &case.argv, &case.environment, inline);

            let sut = from_steps(&recognized);

            assert_eq!(
                sut,
                from_command(&recognized),
                "case {:?} with response files inlined: {inline}",
                case.name
            );
        }
    }
}

#[test]
fn the_cases_are_not_all_empty() {
    // A guard against the comparison passing because both sides are empty.
    let fixture = Fixture::new();
    let total: usize = cases()
        .iter()
        .map(|case| {
            from_command(&fixture.recognize(case.executable, &case.argv, &case.environment, true)).len()
        })
        .sum();

    assert!(total >= 20, "only {total} entries across all cases");
}

// Bead bear-zq3.3 switches the converter to the steps and writes no entry for
// `gcc -S foo.s`: gcc does nothing for it. Until then the converter keeps its.
#[test]
fn assembling_an_assembly_file_is_the_one_known_difference() {
    let fixture = Fixture::new();
    let recognized = fixture.recognize("/usr/bin/gcc", &["gcc", "-S", "foo.s"], &[], false);

    let sut = from_steps(&recognized);

    assert!(sut.is_empty());
    assert_eq!(from_command(&recognized).len(), 1);
}

// Bead bear-zq3.3 switches the converter to the steps, which keep `@args.rsp`
// when it was not read. Today the per-source stripping drops it (bug
// bear-zq3.2).
#[test]
fn an_unread_response_file_is_kept_by_the_steps_and_dropped_by_the_converter() {
    let fixture = Fixture::new();
    let recognized = fixture.recognize("cl", &["cl", "@args.rsp", "/c", "a.cpp"], &[], false);

    let sut = from_steps(&recognized);

    assert_eq!(sut.len(), 1);
    assert_eq!(sut[0].arguments, ["cl", "@args.rsp", "/c", "a.cpp"]);
    let today = from_command(&recognized);
    assert_eq!(today.len(), 1);
    assert_eq!(today[0].arguments, ["cl", "/c", "a.cpp"]);
}
