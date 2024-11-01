// SPDX-License-Identifier: GPL-3.0-or-later

//! This module is defining the semantic of executed commands.
//!
//! The semantic identifies the intent of the execution. It not only
//! recognizes the compiler calls, but also identifies the compiler
//! passes that are executed.
//!
//! A compilation of a source file can be divided into multiple passes.
//! We are interested in the compiler passes, because those are the
//! ones that are relevant to build a JSON compilation database.

pub mod interpreters;

use intercept::Execution;
use std::path::PathBuf;

/// Represents an executed command semantic.
#[derive(Debug, PartialEq)]
pub enum Meaning {
    /// This is a compiler call.
    Compiler {
        compiler: PathBuf,
        working_dir: PathBuf,
        passes: Vec<CompilerPass>,
    },
    /// This is something else we recognised, but not interested to fully specify.
    Ignored,
}

/// Represents a compiler call pass.
#[derive(Debug, PartialEq)]
pub enum CompilerPass {
    Preprocess,
    Compile {
        source: PathBuf,
        output: Option<PathBuf>,
        flags: Vec<String>,
    },
}

/// Responsible to recognize the semantic of an executed command.
///
/// The implementation can be responsible for a single compiler,
/// a set of compilers, or a set of commands that are not compilers.
///
/// The benefit to recognize a non-compiler command, is to not
/// spend more time to try to recognize with other interpreters.
/// Or classify the recognition as ignored to not be further processed
/// later on.
pub trait Interpreter: Send {
    fn recognize(&self, _: &Execution) -> Recognition<Meaning>;
}

/// Represents a semantic recognition result.
///
/// The unknown recognition is used when the interpreter is not
/// able to recognize the command. This can signal the search process
/// to continue with the next interpreter.
#[derive(Debug, PartialEq)]
pub enum Recognition<T> {
    Success(T),
    Error(String),
    Unknown,
}
