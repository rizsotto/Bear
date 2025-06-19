// SPDX-License-Identifier: GPL-3.0-or-later

//! Semantic analysis module for command execution recognition and formatting.
//!
//! This module provides traits and types for recognizing the semantic meaning of executed commands
//! (such as compilers or interpreters) and for formatting their output into structured entries.
//!
//! # Main Components
//!
//! - [`Command`]: Enum representing recognized command types.
//! - [`Interpreter`]: Trait for recognizing the semantic meaning of an `Execution`.
//! - [`Formattable`]: Trait for converting recognized commands into output entries.
//! - [`FormatConfig`]: Configuration for formatting output entries.
//!
//! Implementors of [`Interpreter`] analyze an `Execution` and determine if it matches a known command.
//! If recognized, they return a boxed [`Command`] representing the semantic meaning of the execution.
//!
//! The [`Formattable`] trait allows recognized commands to be transformed into output entries (e.g.,
//! for a compilation database), using the provided [`FormatConfig`].

pub mod clang;
pub mod command;
pub mod interpreters;
pub mod transformation;

use super::intercept::Execution;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Represents a recognized command type after semantic analysis.
///
/// This enum aggregates different types of commands that can be recognized
/// by the semantic analysis system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    /// A recognized compiler command (e.g., gcc, clang).
    Compiler(command::CompilerCommand),
    /// A command that is intentionally ignored and not processed further.
    Ignored(&'static str),
    /// A command that is filtered out and not included in the output.
    Filtered(&'static str),
}

/// Responsible for recognizing the semantic meaning of an executed command.
///
/// Implementors of this trait analyze an [`Execution`] and determine if it matches
/// a known command (such as a compiler or interpreter). If recognized, they
/// return a [`Command`] representing the semantic meaning of the execution.
pub trait Interpreter: Send {
    /// An [`Option<Command>`] containing the recognized command, or `None` if not recognized.
    fn recognize(&self, execution: &Execution) -> Option<Command>;
}

/// Configuration for formatting output entries.
///
/// This struct can be extended to control how recognized commands are
/// transformed into output entries (e.g., for a compilation database).
#[derive(Debug, Default)]
pub struct FormatConfig {}

/// Trait for types that can be formatted into output entries.
pub trait Formattable {
    /// Converts the command into a list of entries, using the provided format configuration.
    fn to_entries(&self, config: &FormatConfig) -> Vec<clang::Entry>;
}

impl Formattable for Command {
    fn to_entries(&self, config: &FormatConfig) -> Vec<clang::Entry> {
        match self {
            Command::Compiler(cmd) => cmd.to_entries(config),
            Command::Ignored(_) => vec![],
            Command::Filtered(_) => vec![],
        }
    }
}
