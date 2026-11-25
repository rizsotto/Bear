// SPDX-License-Identifier: GPL-3.0-or-later

//! The interpretation of a recognized compiler call.
//!
//! An [`Interpretation`] keeps the call as a list of [`Arguments`], each with
//! its raw tokens and its [`Role`]. Everything a consumer wants to know is a
//! view over that list: the arguments in output order, where each came from,
//! the inputs and outputs, the passes each input goes through, and the
//! [`Step`]s the call performs.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

const PIPELINE: [CompilerPass; 4] =
    [CompilerPass::Preprocessing, CompilerPass::Compiling, CompilerPass::Assembling, CompilerPass::Linking];

/// A recognized call, read as a list of classified arguments.
#[derive(Debug)]
pub struct Interpretation {
    /// The id of the compiler family that read the flags.
    pub interpreted_by: &'static str,
    source_mode: SourceMode,
    /// Every item in output order, each with where it came from: launcher
    /// (if any), compiler, environment words that go first, the command line
    /// with read response files spliced in where the `@file` stood,
    /// environment words that go last.
    arguments: Vec<(Provenance, Arguments)>,
}

/// One item of a call: the raw tokens as written, and what they mean.
#[derive(Clone, Debug, PartialEq)]
pub struct Arguments {
    pub args: Vec<String>,
    pub role: Role,
}

/// What an item means in the call.
#[derive(Clone, Debug, PartialEq)]
pub enum Role {
    /// A compiler launcher and its own options (`distcc -j 4`).
    Launcher,
    /// The compiler token. `path` is the observed executable, or the token as
    /// written when a launcher runs it.
    Compiler {
        path: PathBuf,
    },
    /// An `@file` that was not read. `path` is as written, without the `@`.
    ArgumentsFile {
        path: PathBuf,
    },
    Input(FileRef),
    /// An output flag with its path. `args` holds the tokens as written, and
    /// `spelling` how to write them back.
    Output {
        flag: String,
        path: PathBuf,
        spelling: OutputSpelling,
    },
    Flag(PassEffect),
}

/// Where an item came from. For an item whose tokens come from two places
/// (a flag at the end of a response file, its value on the command line),
/// it is where the first token came from.
#[derive(Clone, Debug, PartialEq)]
pub enum Provenance {
    CommandLine,
    /// A response file, as written after the `@`. The innermost one when
    /// files nest.
    ResponseFile(PathBuf),
    /// An environment variable, by name.
    Environment(String),
}

/// A file the compiler is asked to process.
#[derive(Clone, Debug, PartialEq)]
pub enum FileRef {
    Resolved {
        path: PathBuf,
        file_type: FileType,
    },
    /// A library named, not found by path (`-lfoo` is `foo`).
    UnresolvedLibrary {
        name: String,
    },
}

/// What a file is, told by its extension from one table shared by every
/// compiler family. An unknown extension is an `Object`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileType {
    Source,
    SourceHeader,
    SourceModule,
    PreprocessedSource,
    Assembly,
    AssemblyWithCpp,
    Object,
}

/// A thing the call does.
#[derive(Debug)]
pub enum Step<'a> {
    /// One compilation of `source`, with the arguments that belong to it.
    Compile {
        source: &'a FileRef,
        arguments: Vec<&'a Arguments>,
    },
    Link {
        inputs: Vec<&'a FileRef>,
        output: Option<&'a Path>,
    },
}

/// How an output flag and its value were written on the observed command line.
///
/// Compilers do not all accept the same spelling: `cl.exe` documents that
/// `/Fo` takes no space before its value, and TI's long options glue their
/// value after `=`. Re-spelling one form as another produces a database entry
/// the compiler would reject, so the observed form is kept.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputSpelling {
    /// Two tokens: `-o main.o`.
    Separate,
    /// One token, value glued straight onto the flag: `-omain.o`, `/FoX.obj`.
    Glued,
    /// One token, flag and value joined by `=`: `--output_file=x.obj`.
    Equals,
    /// One token, flag and value joined by `:`: `/Fo:X.obj`.
    Colon,
}

impl OutputSpelling {
    /// Writes `flag` and `path` back as the command line tokens this spelling
    /// stands for: one joined token, or the two the build passed separately.
    pub(crate) fn as_arguments(self, flag: &str, path: &Path) -> Vec<String> {
        let path = path.to_string_lossy();
        match self {
            Self::Separate => vec![flag.to_string(), path.to_string()],
            Self::Glued => vec![format!("{flag}{path}")],
            Self::Equals => vec![format!("{flag}={path}")],
            Self::Colon => vec![format!("{flag}:{path}")],
        }
    }
}

/// Represents how an argument affects the compilation pipeline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PassEffect {
    Configures(CompilerPass),
    StopsAt(CompilerPass),
    InfoAndExit,
    DriverOption,
    /// Indicates remaining arguments should be passed through without interpretation.
    /// Used for flags like MSVC's `/link` that forward all subsequent args to a different tool.
    PassThrough,
    None,
}

/// Represents different compiler passes that an argument might affect.
/// The declared order is the pipeline order.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CompilerPass {
    Preprocessing,
    Compiling,
    Assembling,
    Linking,
}

/// How the sources named in one invocation map to compile steps, and so to
/// compilation database entries. Set per compiler family by the family
/// YAML's top-level `source_mode:` key (default `per-source-stripped`). Not
/// used at parse time; [`Interpretation::steps`] consumes it.
/// See the `output-compilation-entries` requirement for the full contract.
#[derive(Copy, Clone, Debug)]
pub enum SourceMode {
    /// Each source is a separable translation unit: one compile step per
    /// source, and each step's arguments keep only that source
    /// (sibling sources in the same invocation are stripped). The default
    /// for GCC, Clang, and most other families.
    PerSourceStripped,
    /// Every source in the invocation is analyzed together as a single
    /// "whole module", but per-file consuming tooling still looks up a
    /// compile command by file path: one compile step per source, and every
    /// step's arguments are the complete invocation (no
    /// sibling stripping). Used by Swift's whole-module compilation
    /// (`swiftc`).
    PerSourceFull,
    /// All sources in the invocation form a single translation unit and
    /// produce one combined output: one compile step per invocation, for the
    /// first source, with every source retained. Used by `valac`.
    Combined,
}

impl FileType {
    fn first_pass(self) -> CompilerPass {
        match self {
            Self::Source | Self::SourceHeader | Self::SourceModule | Self::AssemblyWithCpp => {
                CompilerPass::Preprocessing
            }
            Self::PreprocessedSource => CompilerPass::Compiling,
            Self::Assembly => CompilerPass::Assembling,
            Self::Object => CompilerPass::Linking,
        }
    }
}

impl FileRef {
    fn first_pass(&self) -> CompilerPass {
        match self {
            Self::Resolved { file_type, .. } => file_type.first_pass(),
            Self::UnresolvedLibrary { .. } => CompilerPass::Linking,
        }
    }
}

impl Arguments {
    /// The tokens to write for this item. Input and output paths go through
    /// `path_updater`, an output is spelled the way the build wrote it. The
    /// rest is written as it came.
    pub fn as_arguments(&self, path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Vec<String> {
        match &self.role {
            Role::Input(FileRef::Resolved { path, .. }) => {
                vec![path_updater(path).to_string_lossy().to_string()]
            }
            Role::Output { flag, path, spelling } => spelling.as_arguments(flag, &path_updater(path)),
            _ => self.args.clone(),
        }
    }
}

impl Interpretation {
    pub(crate) fn new(
        interpreted_by: &'static str,
        source_mode: SourceMode,
        arguments: Vec<(Provenance, Arguments)>,
    ) -> Self {
        Self { interpreted_by, source_mode, arguments }
    }

    #[cfg(any(test, feature = "testing"))]
    pub(crate) fn set_source_mode(&mut self, source_mode: SourceMode) {
        self.source_mode = source_mode;
    }

    /// The items in output order: launcher, compiler, environment words that
    /// go first, the command line, environment words that go last.
    pub fn arguments(&self) -> impl Iterator<Item = &Arguments> {
        self.arguments.iter().map(|(_, item)| item)
    }

    /// The same items as [`arguments`](Self::arguments), each with where it came from.
    pub fn provenance(&self) -> impl Iterator<Item = (&Provenance, &Arguments)> {
        self.arguments.iter().map(|(provenance, item)| (provenance, item))
    }

    pub fn inputs(&self) -> impl Iterator<Item = &FileRef> {
        self.arguments().filter_map(|item| match &item.role {
            Role::Input(file) => Some(file),
            _ => None,
        })
    }

    pub fn outputs(&self) -> impl Iterator<Item = &Path> {
        self.arguments().filter_map(|item| match &item.role {
            Role::Output { path, .. } => Some(path.as_path()),
            _ => None,
        })
    }

    /// The last pass that runs: the earliest `StopsAt`, else linking. `None`
    /// when a flag asks for information only, so nothing runs.
    fn last_pass(&self) -> Option<CompilerPass> {
        let mut last = CompilerPass::Linking;
        for item in self.arguments() {
            match item.role {
                Role::Flag(PassEffect::InfoAndExit) => return None,
                Role::Flag(PassEffect::StopsAt(pass)) => last = last.min(pass),
                _ => {}
            }
        }
        Some(last)
    }

    /// The passes `input` goes through. `None` when it goes through none.
    pub fn passes(&self, input: &FileRef) -> Option<RangeInclusive<CompilerPass>> {
        let last = self.last_pass()?;
        let first = input.first_pass();
        (first <= last).then_some(first..=last)
    }

    /// The passes that run for at least one input.
    pub fn runs(&self) -> BTreeSet<CompilerPass> {
        self.inputs()
            .filter_map(|input| self.passes(input))
            .flat_map(|range| PIPELINE.into_iter().filter(move |pass| range.contains(pass)))
            .collect()
    }

    /// The compile steps first, then one link step when an input reaches linking.
    ///
    /// These steps decide which entries the compilation database gets: one per
    /// compile step. An input is compiled when it starts before linking and
    /// its passes go past preprocessing. So these calls have no compile step:
    ///
    /// - information only (`--version`, `--help`)
    /// - preprocessing only (`-E`), including a dependency scan (`-M`)
    /// - link only: objects and libraries, with no source to compile
    /// - an input that starts after the last pass (`-c a.o`, `-S foo.s`)
    ///
    /// A call that stops early (`-c`, `-S`) has one compile step per source.
    /// So does a compile-and-link call, and the link-only flags and libraries
    /// stay out of its steps. The source mode decides what happens to the
    /// call's other inputs: each step keeps only its own input
    /// (`PerSourceStripped`), keeps every input (`PerSourceFull`), or the
    /// whole call is one step for the first source (`Combined`).
    pub fn steps(&self) -> Vec<Step<'_>> {
        let compiled: Vec<&FileRef> = self
            .inputs()
            .filter(|input| input.first_pass() < CompilerPass::Linking)
            .filter(|input| {
                self.passes(input).is_some_and(|range| *range.end() > CompilerPass::Preprocessing)
            })
            .collect();
        let mut steps: Vec<Step<'_>> = match self.source_mode {
            SourceMode::PerSourceStripped => compiled
                .into_iter()
                .map(|source| Step::Compile { source, arguments: self.compile_arguments(Some(source)) })
                .collect(),
            SourceMode::PerSourceFull => compiled
                .into_iter()
                .map(|source| Step::Compile { source, arguments: self.compile_arguments(None) })
                .collect(),
            SourceMode::Combined => compiled
                .into_iter()
                .take(1)
                .map(|source| Step::Compile { source, arguments: self.compile_arguments(None) })
                .collect(),
        };
        let linked: Vec<&FileRef> = self
            .inputs()
            .filter(|input| self.passes(input).is_some_and(|range| range.contains(&CompilerPass::Linking)))
            .collect();
        if !linked.is_empty() {
            steps.push(Step::Link { inputs: linked, output: self.outputs().next() });
        }
        steps
    }

    /// The arguments of a compile step. Link-only items and libraries are
    /// left out. With `only`, the other inputs are left out too.
    fn compile_arguments<'a>(&'a self, only: Option<&'a FileRef>) -> Vec<&'a Arguments> {
        self.arguments()
            .filter(|item| {
                !matches!(
                    item.role,
                    Role::Flag(PassEffect::Configures(CompilerPass::Linking))
                        | Role::Flag(PassEffect::StopsAt(CompilerPass::Linking))
                        | Role::Flag(PassEffect::PassThrough)
                )
            })
            .filter(|item| match &item.role {
                Role::Input(FileRef::UnresolvedLibrary { .. }) => false,
                Role::Input(file) => only.is_none_or(|keep| std::ptr::eq(file, keep)),
                _ => true,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    //! Requirements: output-compilation-entries
    use super::*;
    use crate::interpreters::compilers::CompilerInterpreter;
    use crate::{Interpreter, RecognizeResult};
    use CompilerPass::*;
    use intercept::Execution;
    use std::collections::HashMap;

    fn recognize(sut: &CompilerInterpreter, execution: Execution) -> Interpretation {
        match sut.recognize(execution) {
            RecognizeResult::Recognized(recognized) => recognized.interpretation,
            other => panic!("expected Recognized, got {other:?}"),
        }
    }

    fn interpret(executable: &str, argv: &[&str]) -> Interpretation {
        let execution = Execution::from_strings(executable, argv.to_vec(), "/work", HashMap::new());
        recognize(&CompilerInterpreter::default(), execution)
    }

    fn kinds(sut: &Interpretation) -> Vec<&'static str> {
        sut.steps()
            .iter()
            .map(|step| match step {
                Step::Compile { .. } => "compile",
                Step::Link { .. } => "link",
            })
            .collect()
    }

    fn compiled_files(sut: &Interpretation) -> Vec<String> {
        sut.steps()
            .iter()
            .filter_map(|step| match step {
                Step::Compile { source: FileRef::Resolved { path, .. }, .. } => {
                    Some(path.display().to_string())
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn steps_follow_the_call_shape() {
        let cases: &[(&[&str], &[&str])] = &[
            (&["gcc", "-c", "a.c"], &["compile"]),
            (&["gcc", "a.c", "-o", "app"], &["compile", "link"]),
            (&["gcc", "-o", "app", "a.o", "b.o"], &["link"]),
            (&["gcc", "-o", "app", "a.c", "b.o", "-lm"], &["compile", "link"]),
        ];

        for (argv, expected) in cases {
            let sut = interpret("/usr/bin/gcc", argv);

            assert_eq!(kinds(&sut), *expected, "case {argv:?}");
        }
    }

    #[test]
    fn link_step_names_inputs_and_first_output() {
        let sut = interpret("/usr/bin/gcc", &["gcc", "-o", "app", "a.o", "b.o"]);

        let steps = sut.steps();

        let [Step::Link { inputs, output }] = steps.as_slice() else { panic!("expected one link step") };
        assert_eq!(inputs.len(), 2);
        assert_eq!(*output, Some(Path::new("app")));
    }

    // The rules of the entry list, written as the files that get a compile step.
    #[test]
    fn compile_steps_follow_the_entry_rules() {
        let cases: &[(&[&str], &[&str])] = &[
            (&["gcc", "-c", "a.c", "b.c"], &["a.c", "b.c"]),
            (&["gcc", "-o", "a.out", "a.c", "b.c"], &["a.c", "b.c"]),
            (&["gcc", "-o", "a.out", "a.o", "b.o"], &[]),
            (&["gcc", "--version"], &[]),
            (&["gcc", "-E", "a.c"], &[]),
            (&["gcc", "-c", "foo.s"], &["foo.s"]),
            (&["gcc", "-E", "foo.S"], &[]),
            (&["gcc", "-c", "a.i"], &["a.i"]),
            (&["gcc", "-fsyntax-only", "a.c"], &["a.c"]),
            (&["gcc", "-c", "a.o"], &[]),
            // gcc does nothing for `-S foo.s`, so it has no compile step.
            (&["gcc", "-S", "foo.s"], &[]),
        ];

        for (argv, expected) in cases {
            let sut = interpret("/usr/bin/gcc", argv);

            assert_eq!(compiled_files(&sut), *expected, "case {argv:?}");
        }
    }

    #[test]
    fn passes_follow_type_and_stop_flags() {
        type Case<'a> = (&'a [&'a str], &'a str, Option<(CompilerPass, CompilerPass)>);
        let cases: &[Case] = &[
            (&["gcc", "a.c"], "a.c", Some((Preprocessing, Linking))),
            (&["gcc", "-c", "a.c"], "a.c", Some((Preprocessing, Assembling))),
            (&["gcc", "-S", "a.c"], "a.c", Some((Preprocessing, Compiling))),
            (&["gcc", "-E", "a.c"], "a.c", Some((Preprocessing, Preprocessing))),
            (&["gcc", "-c", "a.i"], "a.i", Some((Compiling, Assembling))),
            (&["gcc", "-c", "a.s"], "a.s", Some((Assembling, Assembling))),
            (&["gcc", "a.o"], "a.o", Some((Linking, Linking))),
            (&["gcc", "-c", "a.o"], "a.o", None),
            (&["gcc", "--version", "a.c"], "a.c", None),
            (&["gcc", "-S", "a.s"], "a.s", None),
        ];

        for (argv, file, expected) in cases {
            let sut = interpret("/usr/bin/gcc", argv);
            let input = sut
                .inputs()
                .find(|input| matches!(input, FileRef::Resolved { path, .. } if path == Path::new(file)))
                .expect("the file is an input");

            let passes = sut.passes(input).map(|range| (*range.start(), *range.end()));

            assert_eq!(passes, *expected, "case {argv:?}");
        }
    }

    #[test]
    fn runs_is_the_union_over_inputs() {
        let sut = interpret("/usr/bin/gcc", &["gcc", "-c", "a.c", "b.s"]);

        assert_eq!(sut.runs(), BTreeSet::from([Preprocessing, Compiling, Assembling]));
        assert_eq!(interpret("/usr/bin/gcc", &["gcc", "--version"]).runs(), BTreeSet::new());
    }

    #[test]
    fn file_type_follows_the_extension() {
        let cases = [
            ("a.c", FileType::Source),
            ("a.cu", FileType::Source),
            ("a.h", FileType::SourceHeader),
            ("a.ixx", FileType::SourceModule),
            ("a.i", FileType::PreprocessedSource),
            ("a.mii", FileType::PreprocessedSource),
            ("a.s", FileType::Assembly),
            ("a.asm", FileType::Assembly),
            ("a.S", FileType::AssemblyWithCpp),
            ("a.sx", FileType::AssemblyWithCpp),
            ("a.o", FileType::Object),
            ("liba.a", FileType::Object),
            ("noextension", FileType::Object),
        ];

        for (file, expected) in cases {
            let sut = interpret("/usr/bin/gcc", &["gcc", file]);

            let types: Vec<FileType> = sut
                .inputs()
                .filter_map(|input| match input {
                    FileRef::Resolved { file_type, .. } => Some(*file_type),
                    FileRef::UnresolvedLibrary { .. } => None,
                })
                .collect();
            assert_eq!(types, [expected], "file {file}");
        }
    }

    #[test]
    fn libraries_and_siblings_stay_out_of_a_stripped_compile_step() {
        let sut = interpret("/usr/bin/gcc", &["gcc", "-O2", "-lm", "a.c", "b.c", "c.o", "-o", "app"]);

        let steps = sut.steps();

        let Step::Compile { arguments, .. } = &steps[0] else { panic!("expected a compile step") };
        let tokens: Vec<&str> = arguments.iter().flat_map(|a| a.args.iter().map(String::as_str)).collect();
        assert_eq!(tokens, ["gcc", "-O2", "a.c", "-o", "app"]);
    }

    fn unresolved(name: &str) -> FileRef {
        FileRef::UnresolvedLibrary { name: name.to_string() }
    }

    #[test]
    fn an_unresolved_library_goes_to_the_link_step_only() {
        let sut = interpret("/usr/bin/gcc", &["gcc", "-c", "a.c"]);

        let lib = unresolved("foo");

        assert_eq!(sut.passes(&lib).map(|r| (*r.start(), *r.end())), None);
        let sut = interpret("/usr/bin/gcc", &["gcc", "a.c"]);
        assert_eq!(sut.passes(&lib).map(|r| (*r.start(), *r.end())), Some((Linking, Linking)));
    }

    fn output(flag: &str, spelling: OutputSpelling, args: &[&str]) -> Arguments {
        Arguments {
            args: args.iter().map(|s| s.to_string()).collect(),
            role: Role::Output { flag: flag.to_string(), path: PathBuf::from("main.o"), spelling },
        }
    }

    // Requirements: output-compilation-entries
    #[test]
    fn as_arguments_spells_outputs_back_through_the_path_updater() {
        let cases = [
            (output("-o", OutputSpelling::Separate, &["-o", "main.o"]), vec!["-o", "/b/main.o"]),
            (output("-o", OutputSpelling::Glued, &["-omain.o"]), vec!["-o/b/main.o"]),
            (
                output("--output_file", OutputSpelling::Equals, &["--output_file=main.o"]),
                vec!["--output_file=/b/main.o"],
            ),
            (output("/Fo", OutputSpelling::Colon, &["/Fo:main.o"]), vec!["/Fo:/b/main.o"]),
        ];
        // Joined by hand so the expected strings hold on every platform;
        // `Path::join` would put a `\` between them on Windows.
        let updater: &dyn Fn(&Path) -> Cow<Path> =
            &|path| Cow::Owned(PathBuf::from(format!("/b/{}", path.display())));

        for (sut, expected) in cases {
            let actual = sut.as_arguments(updater);

            assert_eq!(actual, expected, "case {:?}", sut.args);
        }
    }

    #[test]
    fn as_arguments_rewrites_resolved_inputs_and_keeps_the_rest() {
        let updater: &dyn Fn(&Path) -> Cow<Path> = &|path| Cow::Owned(Path::new("/b").join(path));
        let input = Arguments {
            args: vec!["a.c".to_string()],
            role: Role::Input(FileRef::Resolved { path: PathBuf::from("a.c"), file_type: FileType::Source }),
        };
        let library = Arguments { args: vec!["-lm".to_string()], role: Role::Input(unresolved("m")) };
        let flag = Arguments { args: vec!["-O2".to_string()], role: Role::Flag(PassEffect::None) };
        let file = Arguments {
            args: vec!["@x.rsp".to_string()],
            role: Role::ArgumentsFile { path: PathBuf::from("x.rsp") },
        };
        let compiler = Arguments {
            args: vec!["gcc".to_string()],
            role: Role::Compiler { path: PathBuf::from("/usr/bin/gcc") },
        };

        assert_eq!(input.as_arguments(updater), [Path::new("/b").join("a.c").display().to_string()]);
        assert_eq!(library.as_arguments(updater), ["-lm"]);
        assert_eq!(flag.as_arguments(updater), ["-O2"]);
        assert_eq!(file.as_arguments(updater), ["@x.rsp"]);
        assert_eq!(compiler.as_arguments(updater), ["gcc"]);
    }

    fn words(sut: &Interpretation) -> Vec<String> {
        sut.arguments().flat_map(|item| item.args.iter().cloned()).collect()
    }

    fn execution(executable: &str, argv: &[&str]) -> Execution {
        Execution::from_strings(executable, argv.to_vec(), "/work", HashMap::new())
    }

    // Requirements: recognition-compiler-launchers
    #[test]
    fn a_launcher_is_an_item_before_the_compiler() {
        let sut = interpret("/usr/bin/distcc", &["distcc", "-j", "4", "gcc", "-c", "a.c"]);

        let items: Vec<&Arguments> = sut.arguments().take(3).collect();

        assert_eq!(items[0].args, ["distcc", "-j", "4"]);
        assert_eq!(items[0].role, Role::Launcher);
        assert_eq!(items[1].args, ["gcc"]);
        assert_eq!(items[1].role, Role::Compiler { path: PathBuf::from("gcc") });
        assert_eq!(items[2].args, ["-c"]);
        assert_eq!(sut.interpreted_by, "gcc");
    }

    // Requirements: recognition-compiler-launchers
    #[test]
    fn the_compiler_path_is_observed_without_a_launcher_and_as_written_with_one() {
        let cases = [
            ("/usr/bin/gcc", vec!["gcc", "-c", "a.c"], "gcc", "/usr/bin/gcc"),
            ("/usr/bin/ccache", vec!["ccache", "gcc", "-c", "a.c"], "gcc", "gcc"),
            ("/usr/bin/ccache", vec!["ccache", "/opt/bin/gcc", "-c", "a.c"], "/opt/bin/gcc", "/opt/bin/gcc"),
            ("/usr/lib/ccache/gcc", vec!["gcc", "-c", "a.c"], "gcc", "/usr/lib/ccache/gcc"),
        ];

        for (executable, argv, token, path) in cases {
            let sut = interpret(executable, &argv);

            let compiler = sut
                .arguments()
                .find(|item| matches!(item.role, Role::Compiler { .. }))
                .expect("a compiler item");
            assert_eq!(compiler.args, [token], "case {argv:?}");
            assert_eq!(compiler.role, Role::Compiler { path: PathBuf::from(path) }, "case {argv:?}");
        }
    }

    fn provenance_of(sut: &Interpretation) -> Vec<(String, String)> {
        let name = |provenance: &Provenance| match provenance {
            Provenance::CommandLine => "argv".to_string(),
            Provenance::ResponseFile(path) => format!("file {}", path.display()),
            Provenance::Environment(variable) => format!("env {variable}"),
        };
        sut.provenance().map(|(provenance, item)| (name(provenance), item.args.join(" "))).collect()
    }

    // Requirements: output-env-derived-flags
    #[test]
    fn msvc_environment_words_go_before_and_after_the_command_line() {
        let env = HashMap::from([("CL", "/O2 /W4"), ("_CL_", "/DX")]);
        let execution = Execution::from_strings("cl", vec!["cl", "/c", "a.cpp"], "/work", env);

        let sut = recognize(&CompilerInterpreter::default(), execution);

        let pairs = |items: &[(&str, &str)]| -> Vec<(String, String)> {
            items.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
        };
        assert_eq!(
            provenance_of(&sut),
            pairs(&[
                ("argv", "cl"),
                ("env CL", "/O2"),
                ("env CL", "/W4"),
                ("argv", "/c"),
                ("argv", "a.cpp"),
                ("env _CL_", "/DX"),
            ])
        );
    }

    // Requirements: output-env-derived-flags
    #[test]
    fn gcc_path_variable_becomes_include_flags_after_the_command_line() {
        let env = HashMap::from([("CPATH", "/a")]);
        let execution = Execution::from_strings("/usr/bin/gcc", vec!["gcc", "-c", "a.c"], "/work", env);

        let sut = recognize(&CompilerInterpreter::default(), execution);

        assert_eq!(words(&sut), ["gcc", "-c", "a.c", "-I", "/a"]);
        let (provenance, item) = sut.provenance().last().expect("an item");
        assert_eq!(provenance, &Provenance::Environment("CPATH".to_string()));
        assert_eq!(item.role, Role::Flag(PassEffect::Configures(Preprocessing)));
    }

    // Requirements: output-response-file-inlining
    #[test]
    fn a_read_response_file_is_spliced_in_with_its_provenance() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("f.rsp"), "-I/opt extra.c").unwrap();
        let execution = Execution::from_strings(
            "/usr/bin/gcc",
            vec!["gcc", "-DBASE=1", "@f.rsp", "-c", "a.c"],
            dir.path().to_str().unwrap(),
            HashMap::new(),
        );

        let sut = recognize(&CompilerInterpreter::new_with_format(Default::default(), true, true), execution);

        let pairs = |items: &[(&str, &str)]| -> Vec<(String, String)> {
            items.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
        };
        assert_eq!(
            provenance_of(&sut),
            pairs(&[
                ("argv", "gcc"),
                ("argv", "-DBASE=1"),
                ("file f.rsp", "-I/opt"),
                ("file f.rsp", "extra.c"),
                ("argv", "-c"),
                ("argv", "a.c"),
            ])
        );
    }

    /// Recognizes `argv` with response files read, in `dir`.
    fn inlined(dir: &tempfile::TempDir, executable: &str, argv: &[&str]) -> Interpretation {
        let execution =
            Execution::from_strings(executable, argv.to_vec(), dir.path().to_str().unwrap(), HashMap::new());
        recognize(&CompilerInterpreter::new_with_format(Default::default(), true, true), execution)
    }

    // Requirements: output-response-file-inlining
    #[test]
    fn a_flag_at_the_end_of_a_file_does_not_take_its_value_from_argv() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("m.rsp"), "-c -o").unwrap();

        let sut = inlined(&dir, "/usr/bin/gcc", &["gcc", "@m.rsp", "mixed.o", "a.c"]);

        // The lone -o is a plain flag; the next argv token is classified on its own.
        let items: Vec<(&Role, &[String])> =
            sut.arguments().skip(1).map(|i| (&i.role, &i.args[..])).collect();
        assert_eq!(items[1].0, &Role::Flag(PassEffect::None));
        assert_eq!(items[1].1, ["-o"]);
        assert_eq!(sut.outputs().count(), 0);
        assert_eq!(items[2].1, ["mixed.o"]);
        assert!(matches!(items[2].0, Role::Input(_)));
        assert_eq!(compiled_files(&sut), ["a.c"]);
    }

    // Requirements: output-response-file-inlining
    #[test]
    fn a_flag_value_starting_with_at_is_not_read_even_when_the_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("rpath")).unwrap();
        std::fs::write(dir.path().join("rpath/libfoo.dylib"), "-DFROM_FILE").unwrap();
        let argv = ["clang", "-dynamiclib", "-install_name", "@rpath/libfoo.dylib", "a.c"];

        let present = inlined(&dir, "/usr/bin/clang", &argv);
        let absent = inlined(&tempfile::tempdir().unwrap(), "/usr/bin/clang", &argv);

        for sut in [present, absent] {
            let item =
                sut.arguments().find(|item| item.args.first().map(String::as_str) == Some("-install_name"));
            assert_eq!(item.expect("the flag item").args, ["-install_name", "@rpath/libfoo.dylib"]);
            assert!(sut.arguments().all(|item| !matches!(item.role, Role::ArgumentsFile { .. })));
            assert!(provenance_of(&sut).iter().all(|(origin, _)| origin == "argv"));
        }
    }

    // Requirements: output-response-file-inlining
    #[test]
    fn an_rpath_value_starting_with_at_is_not_read() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("loader_path"), "-DFROM_FILE").unwrap();

        let sut = inlined(&dir, "/usr/bin/clang", &["clang", "-rpath", "@loader_path", "a.o", "-o", "app"]);

        let item = sut.arguments().find(|item| item.args.first().map(String::as_str) == Some("-rpath"));
        assert_eq!(item.expect("the flag item").args, ["-rpath", "@loader_path"]);
        assert!(sut.arguments().all(|item| !matches!(item.role, Role::ArgumentsFile { .. })));
    }

    // Requirements: output-response-file-inlining
    #[test]
    fn a_response_file_after_link_is_not_read() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("x.rsp"), "/DFROM_FILE").unwrap();

        let sut = inlined(&dir, "cl", &["cl", "/c", "a.cpp", "/link", "@x.rsp"]);

        let last = sut.arguments().last().expect("an item");
        assert_eq!(last.args, ["@x.rsp"]);
        assert_eq!(last.role, Role::Flag(PassEffect::Configures(Linking)));
    }

    // Requirements: output-response-file-inlining
    #[test]
    fn nested_files_carry_the_innermost_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("outer.rsp"), "-DOUTER @inner.rsp -DTAIL").unwrap();
        std::fs::write(dir.path().join("inner.rsp"), "-DINNER b.c").unwrap();

        let sut = inlined(&dir, "/usr/bin/gcc", &["gcc", "@outer.rsp", "-c", "a.c"]);

        let pair = |a: &str, b: &str| (a.to_string(), b.to_string());
        assert_eq!(
            provenance_of(&sut),
            [
                pair("argv", "gcc"),
                pair("file outer.rsp", "-DOUTER"),
                pair("file inner.rsp", "-DINNER"),
                pair("file inner.rsp", "b.c"),
                pair("file outer.rsp", "-DTAIL"),
                pair("argv", "-c"),
                pair("argv", "a.c"),
            ]
        );
        assert_eq!(compiled_files(&sut), ["b.c", "a.c"]);
    }

    // Requirements: output-response-file-inlining
    #[test]
    fn a_missing_file_stays_an_item_with_the_provenance_of_where_it_was_written() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("outer.rsp"), "-DOUTER @gone.rsp").unwrap();

        let sut = inlined(&dir, "/usr/bin/gcc", &["gcc", "@outer.rsp", "@gone2.rsp", "-c", "a.c"]);

        let pair = |a: &str, b: &str| (a.to_string(), b.to_string());
        assert_eq!(
            provenance_of(&sut),
            [
                pair("argv", "gcc"),
                pair("file outer.rsp", "-DOUTER"),
                pair("file outer.rsp", "@gone.rsp"),
                pair("argv", "@gone2.rsp"),
                pair("argv", "-c"),
                pair("argv", "a.c"),
            ]
        );
        let kept: Vec<&Role> =
            sut.arguments().filter(|i| i.args[0].starts_with('@')).map(|i| &i.role).collect();
        assert_eq!(
            kept,
            [
                &Role::ArgumentsFile { path: PathBuf::from("gone.rsp") },
                &Role::ArgumentsFile { path: PathBuf::from("gone2.rsp") },
            ]
        );
    }

    // Requirements: output-response-file-inlining
    #[test]
    fn a_file_that_names_itself_is_read_up_to_the_depth_limit() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("loop.rsp"), "-DBEFORE @loop.rsp").unwrap();

        let sut = inlined(&dir, "/usr/bin/gcc", &["gcc", "@loop.rsp", "-c", "a.c"]);

        let words = words(&sut);
        assert_eq!(words.iter().filter(|w| *w == "-DBEFORE").count(), 16);
        assert_eq!(words.iter().filter(|w| *w == "@loop.rsp").count(), 1);
        assert_eq!(compiled_files(&sut), ["a.c"]);
        let last = sut.arguments().find(|i| i.args[0] == "@loop.rsp").expect("the left over reference");
        assert_eq!(last.role, Role::ArgumentsFile { path: PathBuf::from("loop.rsp") });
    }

    // Requirements: output-response-file-inlining
    #[test]
    fn link_in_a_file_affects_only_the_rest_of_that_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("l.rsp"), "/c /link foo.lib").unwrap();

        let sut = inlined(&dir, "cl", &["cl", "@l.rsp", "a.cpp"]);

        let roles: Vec<&Role> = sut.arguments().skip(1).map(|item| &item.role).collect();
        assert_eq!(roles[1], &Role::Flag(PassEffect::PassThrough));
        assert_eq!(roles[2], &Role::Flag(PassEffect::Configures(Linking)));
        assert!(matches!(roles[3], Role::Input(_)));
        assert_eq!(compiled_files(&sut), ["a.cpp"]);
    }

    // Requirements: output-response-file-inlining
    #[test]
    fn an_ignore_flag_inside_a_response_file_is_seen() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("s.rsp"), "-frontend -c a.swift").unwrap();
        let execution = Execution::from_strings(
            "/usr/bin/swiftc",
            vec!["swiftc", "@s.rsp"],
            dir.path().to_str().unwrap(),
            HashMap::new(),
        );

        let sut = CompilerInterpreter::new_with_format(Default::default(), true, true).recognize(execution);

        assert!(matches!(sut, RecognizeResult::Ignored { .. }), "got {sut:?}");
    }

    // Requirements: output-response-file-inlining
    #[test]
    fn an_unread_response_file_is_an_item_in_every_family() {
        let cases = [
            ("cl", vec!["cl", "@args.rsp", "/c", "a.cpp"]),
            ("/usr/bin/gcc", vec!["gcc", "@args.rsp", "-c", "a.c"]),
        ];

        for (executable, argv) in cases {
            let sut = recognize(&CompilerInterpreter::default(), execution(executable, &argv));

            let item = sut.arguments().nth(1).expect("the second item");
            assert_eq!(item.args, ["@args.rsp"], "case {argv:?}");
            assert_eq!(item.role, Role::ArgumentsFile { path: PathBuf::from("args.rsp") }, "case {argv:?}");
            assert_eq!(compiled_files(&sut).len(), 1, "case {argv:?}");
        }
    }

    // Requirements: output-response-file-inlining
    #[test]
    fn a_flag_value_that_looks_like_a_response_file_stays_a_value() {
        let sut = interpret("/usr/bin/clang", &["clang", "-install_name", "@rpath/x", "a.o"]);

        let items: Vec<&Arguments> = sut.arguments().collect();

        assert_eq!(items[1].args, ["-install_name", "@rpath/x"]);
        assert!(matches!(items[1].role, Role::Flag(_)));
    }

    // Requirements: output-response-file-inlining
    #[test]
    fn after_link_every_token_is_a_link_flag_a_response_file_included() {
        let sut = interpret("cl", &["cl", "/c", "a.cpp", "/link", "@x.rsp", "foo.lib"]);

        let roles: Vec<&Role> = sut.arguments().skip(3).map(|item| &item.role).collect();

        assert_eq!(
            roles,
            [
                &Role::Flag(PassEffect::PassThrough),
                &Role::Flag(PassEffect::Configures(Linking)),
                &Role::Flag(PassEffect::Configures(Linking)),
            ]
        );
    }
}
