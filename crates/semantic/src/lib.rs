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
//! 2. **Parsing** - a recognized call becomes a [`Command`]: its source files,
//!    its [`CompilerPass`], and its [`Argument`] list.
//!
//! # Core types
//!
//! - [`Command`] - a structured compiler invocation
//! - [`Argument`] - one command line argument with its classification
//! - [`ArgumentKind`] - what an argument means to the compiler
//! - [`PassEffect`] - how an argument affects the compilation pipeline
//! - [`CompilerHints`] - caller-supplied overrides for which executables are
//!   which compiler, built from the `compilers:` configuration section

pub mod interpreters;

/// Command factories and comparison helpers for tests. Compiled only for this
/// crate's own tests and for dependents that turn on the `testing` feature.
#[cfg(any(test, feature = "testing"))]
pub mod testing;

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
#[derive(Debug)]
pub enum RecognizeResult {
    /// A recognized compiler invocation with parsed, classified arguments.
    Recognized(Command),
    /// A command that is intentionally ignored (e.g. coreutils, excluded compilers).
    Ignored(&'static str),
    /// The interpreter did not recognize this execution. Ownership is returned.
    NotRecognized(Execution),
}

/// Represents a full compiler command invocation.
#[derive(Debug)]
pub struct Command {
    pub working_dir: PathBuf,
    pub executable: PathBuf,
    pub arguments: Vec<Argument>,
    pub source_mode: SourceMode,
}

/// A compiler command-line argument with semantic classification.
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// Flags and other non-file arguments (e.g. `-c`, `-Wall`, `-I /usr/include`).
    Other { arguments: Vec<String>, kind: ArgumentKind },
    /// A source or object file argument.
    Source { path: String, binary: bool },
    /// An output file argument (e.g. `-o main.o`).
    Output { flag: String, path: String },
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
            Self::Output { flag, path } => {
                let p = Path::new(path);
                let updated = path_updater(p);
                vec![flag.clone(), updated.to_string_lossy().to_string()]
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
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompilerPass {
    Preprocessing,
    Compiling,
    Assembling,
    Linking,
}

/// How the sources named in one invocation map to compilation database
/// entries. Set per compiler family in the interpreter factory
/// (`interpreters/compilers/flag_based.rs`), not in the compiler-flag YAML,
/// because it is consumed at the converter (post-parse), not at parse time.
/// See the `output-compilation-entries` requirement for the full contract.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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
