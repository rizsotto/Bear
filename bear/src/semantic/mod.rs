// SPDX-License-Identifier: GPL-3.0-or-later

//! Semantic analysis module for command execution recognition and formatting.
//!
//! This module provides traits and types for recognizing the semantic meaning of executed commands
//! (such as compilers or interpreters) and for formatting their output into structured entries.

pub mod clang;
pub mod interpreters;

use super::intercept::Execution;

use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::PathBuf;

/// Responsible for recognizing the semantic meaning of an executed command.
///
/// Implementers of this trait analyze an [`Execution`] and determine if it matches
/// a known command (such as a compiler or interpreter). If recognized, they
/// return a [`Command`] representing the semantic meaning of the execution.
#[cfg_attr(test, mockall::automock)]
pub trait Interpreter: Send {
    /// An [`Option<Command>`] containing the recognized command, or `None` if not recognized.
    fn recognize(&self, execution: &Execution) -> Option<Command>;
}

/// Represents a recognized command type after semantic analysis.
///
/// This enum aggregates different types of commands that can be recognized
/// by the semantic analysis system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    /// A recognized compiler command (e.g., gcc, clang).
    Compiler(CompilerCommand),
    /// A command that is intentionally ignored and not processed further.
    Ignored(&'static str),
}

/// Represents a full compiler command invocation.
///
/// The [`working_dir`] is the directory where the command is executed,
/// the [`executable`] is the path to the compiler binary,
/// while [`arguments`] contains the command-line arguments annotated
/// with their meaning (e.g., source files, output files, switches).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompilerCommand {
    pub working_dir: PathBuf,
    pub executable: PathBuf,
    pub arguments: Vec<ArgumentGroup>,
}

/// Represents a group of arguments of the same intent.
///
/// Groups the arguments which belongs together, and annotate the meaning of them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArgumentGroup {
    pub args: Vec<String>,
    pub kind: ArgumentKind,
}

impl ArgumentGroup {
    fn as_file(&self) -> Option<String> {
        match self.kind {
            ArgumentKind::Source => self.args.first().cloned(),
            ArgumentKind::Output => self.args.get(1).cloned(),
            _ => None,
        }
    }
}

/// Represents the meaning of the argument in the compiler call. Identifies
/// the purpose of each argument in the command line.
///
/// Variants:
/// - `Compiler`: The compiler executable itself.
/// - `Source`: A source file to be compiled.
/// - `Output`: An output file or related argument (e.g., `-o output.o`).
/// - `Other`: Any other argument not classified above (e.g., compiler switches like `-Wall`).
///   Can optionally specify which compiler pass the argument affects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArgumentKind {
    Compiler,
    Source,
    Output,
    Other(Option<CompilerPass>),
}

/// Represents different compiler passes that an argument might affect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompilerPass {
    Info,
    Preprocessing,
    Compiling,
    Assembling,
    Linking,
}

impl CompilerCommand {
    pub fn new(working_dir: PathBuf, executable: PathBuf, arguments: Vec<ArgumentGroup>) -> Self {
        Self {
            working_dir,
            executable,
            arguments,
        }
    }

    #[cfg(test)]
    pub fn from_strings(
        working_dir: &str,
        executable: &str,
        arguments: Vec<(ArgumentKind, Vec<&str>)>,
    ) -> Self {
        Self {
            working_dir: PathBuf::from(working_dir),
            executable: PathBuf::from(executable),
            arguments: arguments
                .into_iter()
                .map(|(kind, args)| ArgumentGroup {
                    args: args.into_iter().map(String::from).collect(),
                    kind,
                })
                .collect(),
        }
    }
}
