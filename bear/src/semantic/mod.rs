// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides semantic recognition for executed commands, particularly
//! compiler invocations. It identifies compiler passes and transforms them into
//! structured entries for use in a JSON compilation database.
//!
//! The main abstractions are:
//! - `Interpreter`: Recognizes the semantic meaning of an executed command, such as
//!   identifying compiler invocations and their relevant passes.
//! - `Command`: Represents a recognized command and provides a way to convert it
//!   into structured entries (e.g., for Clang).
//! - `FormatConfig`: Configuration for formatting output entries.

pub mod clang;
pub mod interpreters;
pub mod transformation;

use super::intercept::Execution;
use std::fmt::Debug;

/// Configuration for formatting output entries.
///
/// This struct can be extended to control how recognized commands are
/// transformed into output entries (e.g., for a compilation database).
#[derive(Debug, Default)]
pub struct FormatConfig {}

/// Represents a recognized command that can be transformed into output entries.
///
/// Implementors of this trait encapsulate the details of a specific command
/// (such as a compiler invocation) and provide a method to convert it into
/// structured entries for further processing.
pub trait Command: Debug + Send {
    /// Converts the command into a list of Clang entries, using the provided format configuration.
    fn to_clang_entries(&self, _: &FormatConfig) -> Vec<clang::Entry>;
}

/// Responsible for recognizing the semantic meaning of an executed command.
///
/// Implementors of this trait analyze an `Execution` and determine if it matches
/// a known command (such as a compiler or interpreter). If recognized, they
/// return a boxed `Command` representing the semantic meaning of the execution.
pub trait Interpreter: Send {
    fn recognize(&self, _: &Execution) -> Option<Box<dyn Command>>;
}
