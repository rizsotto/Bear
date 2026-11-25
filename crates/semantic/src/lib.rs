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
//!    [`Execution`] plus the call read as an [`Interpretation`].
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
//! - [`Recognized`] - an observed execution and its [`Interpretation`]
//! - [`Interpretation`] - a recognized call as classified [`Arguments`]
//! - [`Arguments`], [`Role`], [`FileRef`], [`FileType`] - one item and what
//!   it means
//! - [`PassEffect`] - how an argument affects the compilation pipeline
//! - [`CompilerHints`] - caller-supplied overrides for which executables are
//!   which compiler, built from the `compilers:` configuration section

pub mod interpreters;

mod interpretation;

/// Factories for recognized calls in tests. Compiled only for this
/// crate's own tests and for dependents that turn on the `testing` feature.
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use interpretation::{
    Arguments, CompilerPass, FileRef, FileType, Interpretation, OutputSpelling, PassEffect, Provenance, Role,
    SourceMode, Step,
};
pub use interpreters::compilers::compiler_recognition::{CompilerHints, CompilerRecognizer};
pub use interpreters::compilers::identity::SpellingError;
pub use interpreters::compilers::print_compilers;
pub use interpreters::matchers::{is_c_family_source, is_header_file, looks_like_a_source_file};

use intercept::Execution;

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

/// A recognized call: the observed execution and the call read as an
/// interpretation. This is what the output writer streams.
#[derive(Debug)]
pub struct Recognized {
    pub execution: Execution,
    pub interpretation: Interpretation,
}
