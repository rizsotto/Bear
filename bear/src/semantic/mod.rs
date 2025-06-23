// SPDX-License-Identifier: GPL-3.0-or-later

//! Semantic analysis module for command execution recognition and formatting.
//!
//! This module provides traits and types for recognizing the semantic meaning of executed commands
//! (such as compilers or interpreters) and for formatting their output into structured entries.

pub mod clang;
pub mod command;
pub mod interpreters;

use super::intercept::Execution;
use crate::config;
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
    Filtered(String),
}

impl Command {
    /// Converts the command into compilation database entries.
    pub fn to_entries(&self, config: &config::EntryFormat) -> Vec<clang::Entry> {
        match self {
            Command::Compiler(cmd) => cmd.to_entries(config),
            Command::Ignored(_) => vec![],
            Command::Filtered(_) => vec![],
        }
    }
}

/// Responsible for recognizing the semantic meaning of an executed command.
///
/// Implementers of this trait analyze an [`Execution`] and determine if it matches
/// a known command (such as a compiler or interpreter). If recognized, they
/// return a [`Command`] representing the semantic meaning of the execution.
pub trait Interpreter: Send {
    /// An [`Option<Command>`] containing the recognized command, or `None` if not recognized.
    fn recognize(&self, execution: &Execution) -> Option<Command>;
}
