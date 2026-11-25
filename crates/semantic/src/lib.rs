// SPDX-License-Identifier: GPL-3.0-or-later

//! Compiler recognition and command line flag parsing.
//!
//! Decides whether an intercepted [`Execution`] is a compiler invocation, and
//! if so parses its command line into classified arguments. This crate is the
//! analysis core: it knows about compilers, not about how Bear intercepts a
//! build or how it writes its output.
//!
//! # Pipeline
//!
//! 1. **Recognition** ([`Interpreter`]) - classifies an [`Execution`] as a
//!    recognized compiler call, an ignored program, or neither
//!    ([`RecognizeResult`]). Build the configured chain with
//!    [`interpreters::create`].
//!
//! 2. **Parsing** - a recognized call is a [`Recognized`]: the observed
//!    [`Execution`] plus a [`Command`] with its source files, its
//!    [`CompilerPass`], and its [`Argument`] list, and the same call as an
//!    [`Interpretation`].
//!
//! # Interpretation
//!
//! An [`Interpretation`] is a list of [`Arguments`]. Each item keeps its raw
//! tokens and a [`Role`]: launcher, compiler, input, output, flag, or an
//! `@file` that was not read. Response files that were read are spliced in
//! where the `@file` stood, and environment-derived words are kept per
//! variable. Everything else is a view over that list:
//!
//! - `arguments()` and `provenance()` - the items in output order, and where
//!   each came from ([`Provenance`])
//! - `inputs()` and `outputs()` - the files the call reads and writes
//! - `passes()` and `runs()` - the [`CompilerPass`]es each input goes through
//! - `steps()` - the [`Step`]s: one compile step per translation unit, and a
//!   link step when an input reaches linking
//!
//! # Core types
//!
//! - [`Recognized`] - an observed execution, its parsed [`Command`], and its
//!   [`Interpretation`]
//! - [`Interpretation`] - a recognized call as classified [`Arguments`]
//! - [`Arguments`], [`Role`], [`FileRef`], [`FileType`] - one item and what
//!   it means
//! - [`Command`] - a structured compiler invocation
//! - [`Argument`] - one command line argument with its classification
//! - [`ArgumentKind`] - what an argument means to the compiler
//! - [`PassEffect`] - how an argument affects the compilation pipeline
//! - [`CompilerHints`] - caller-supplied overrides for which executables are
//!   which compiler, built from the `compilers:` configuration section

pub mod interpreters;

mod interpretation;

/// Command factories and comparison helpers for tests. Compiled only for this
/// crate's own tests and for dependents that turn on the `testing` feature.
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use interpretation::{Arguments, FileRef, FileType, Interpretation, Provenance, Role, Step};
pub use interpreters::compilers::compiler_recognition::{CompilerHints, CompilerRecognizer};
pub use interpreters::compilers::identity::SpellingError;
pub use interpreters::compilers::print_compilers;
pub use interpreters::matchers::{is_c_family_source, is_header_file, looks_like_a_source_file};

use intercept::Execution;

use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// Responsible for recognizing the semantic meaning of an executed command.
///
/// `Send` because the interpreter is moved into the consumer thread that
/// drains the interception channel (see `modes::execution::Interceptor`).
/// Recognition runs on that one thread only, so `Sync` is deliberately
/// not required: implementations may use single-threaded interior
/// mutability (e.g. `RefCell`) for caches.
#[cfg_attr(test, mockall::automock)]
pub trait Interpreter: Send {
    fn recognize(&self, execution: Execution) -> RecognizeResult;
}

/// Result of semantic recognition of an executed command.
///
/// Every outcome carries the execution exactly as it was observed, so a
/// later stage can report what ran even when the entry names something else
/// (a launcher's inner compiler, for one).
#[derive(Debug)]
pub enum RecognizeResult {
    /// A recognized compiler invocation with parsed, classified arguments.
    Recognized(Recognized),
    /// A command that is intentionally ignored (e.g. coreutils, excluded compilers).
    Ignored { execution: Execution, reason: &'static str },
    /// The interpreter did not recognize this execution. Ownership is returned.
    NotRecognized(Execution),
}

/// A recognized call: the observed execution, its parsed command, and the
/// same call as an interpretation. This is what the output writer streams.
#[derive(Debug)]
pub struct Recognized {
    pub execution: Execution,
    pub command: Command,
    pub interpretation: Interpretation,
}

/// A parsed compiler invocation.
///
/// `executable` is the compiler itself. Under a launcher it is the inner
/// compiler token, which differs from the observed `Recognized::execution`.
#[derive(Debug)]
pub struct Command {
    pub executable: PathBuf,
    pub arguments: Vec<Argument>,
    pub source_mode: SourceMode,
}

/// A compiler command-line argument with semantic classification.
#[derive(Debug, PartialEq)]
pub enum Argument {
    /// Flags and other non-file arguments (e.g. `-c`, `-Wall`, `-I /usr/include`).
    Other { arguments: Vec<String>, kind: ArgumentKind },
    /// A source or object file argument.
    Source { path: String, binary: bool },
    /// An output file argument (e.g. `-o main.o`). The flag and the path are
    /// stored apart so the path stays individually addressable, and
    /// [`OutputSpelling`] remembers how the build wrote them so they can be
    /// spelled back the way the compiler accepts.
    Output { flag: String, path: String, spelling: OutputSpelling },
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
    fn as_arguments(self, flag: &str, path: &Path) -> Vec<String> {
        let path = path.to_string_lossy();
        match self {
            Self::Separate => vec![flag.to_string(), path.to_string()],
            Self::Glued => vec![format!("{flag}{path}")],
            Self::Equals => vec![format!("{flag}={path}")],
            Self::Colon => vec![format!("{flag}:{path}")],
        }
    }
}

impl Argument {
    /// Creates a Source variant, auto-detecting binary vs compilable from extension.
    pub fn new_source(path: String) -> Self {
        let binary = !looks_like_a_source_file(&path);
        Self::Source { path, binary }
    }

    pub fn kind(&self) -> ArgumentKind {
        match self {
            Self::Other { kind, .. } => *kind,
            Self::Source { binary, .. } => ArgumentKind::Source { binary: *binary },
            Self::Output { .. } => ArgumentKind::Output,
        }
    }

    pub fn as_arguments(&self, path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Vec<String> {
        match self {
            Self::Other { arguments, .. } => arguments.clone(),
            Self::Source { path, .. } => {
                let p = Path::new(path);
                let updated = path_updater(p);
                vec![updated.to_string_lossy().to_string()]
            }
            Self::Output { flag, path, spelling } => {
                let updated = path_updater(Path::new(path));
                spelling.as_arguments(flag, &updated)
            }
        }
    }

    pub fn as_file(&self, path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Option<PathBuf> {
        match self {
            Self::Other { .. } => None,
            Self::Source { path, .. } => {
                let p = Path::new(path);
                Some(path_updater(p).to_path_buf())
            }
            Self::Output { path, .. } => {
                let p = Path::new(path);
                Some(path_updater(p).to_path_buf())
            }
        }
    }
}

/// Represents the meaning of the argument in the compiler call.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ArgumentKind {
    Compiler,
    Source { binary: bool },
    Output,
    Other(PassEffect),
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

/// How the sources named in one invocation map to compilation database
/// entries. Set per compiler family by the family YAML's top-level
/// `source_mode:` key (default `per-source-stripped`). It is not used at
/// parse time; the converter consumes it (post-parse).
/// See the `output-compilation-entries` requirement for the full contract.
#[derive(Copy, Clone, Debug)]
pub enum SourceMode {
    /// Each source is a separable translation unit: the converter emits one
    /// entry per source, and each entry's arguments keep only that source
    /// (sibling sources in the same invocation are stripped). The default
    /// for GCC, Clang, and most other families.
    PerSourceStripped,
    /// Every source in the invocation is analyzed together as a single
    /// "whole module", but per-file consuming tooling still looks up a
    /// compile command by file path: the converter emits one entry per
    /// source, and every entry's arguments are the complete invocation (no
    /// sibling stripping). Used by Swift's whole-module compilation
    /// (`swiftc`).
    PerSourceFull,
    /// All sources in the invocation form a single translation unit and
    /// produce one combined output: the converter emits exactly one entry
    /// per invocation, with `file` set to the first source and every source
    /// retained. Used by `valac`.
    Combined,
}
