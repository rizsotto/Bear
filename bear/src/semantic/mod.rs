// SPDX-License-Identifier: GPL-3.0-or-later

//! Semantic analysis module for command execution recognition and formatting.
//!
//! This module provides the core abstractions for analyzing executed commands and determining
//! their semantic meaning (e.g., compiler invocations, ignored commands).
//!
//! # Architecture
//!
//! The semantic analysis follows a pipeline approach:
//!
//! 1. **Recognition** ([`Interpreter`] trait) - Analyzes raw [`Execution`] data to identify
//!    known command types (compilers, build tools, etc.)
//!
//! 2. **Classification** ([`Command`] enum) - Represents the recognized command type:
//!    - [`Command::Compiler`] - A compiler invocation with structured arguments
//!    - [`Command::Ignored`] - A command that should be filtered out
//!
//! 3. **Processing** - Further analysis by specialized modules:
//!    - [`clang`] - Converts compiler commands to clang compilation database format
//!    - [`interpreters`] - Various command recognition strategies
//!
//! # Core Types
//!
//! - [`Command`] - Represents a structured compiler invocation
//! - [`Arguments`] - Trait for representing different types of compiler arguments
//! - [`ArgumentKind`] - Classifies the semantic meaning of arguments
//! - [`PassEffect`] - Represents how an argument affects the compilation pipeline

pub mod interpreters;

#[cfg(test)]
pub mod testing;

use super::intercept::Execution;
use interpreters::matchers::looks_like_a_source_file;

use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// Responsible for recognizing the semantic meaning of an executed command.
#[cfg_attr(test, mockall::automock)]
pub trait Interpreter: Send + Sync {
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

impl Command {
    pub fn new(working_dir: PathBuf, executable: PathBuf, arguments: Vec<Argument>) -> Self {
        Self { working_dir, executable, arguments }
    }
}
