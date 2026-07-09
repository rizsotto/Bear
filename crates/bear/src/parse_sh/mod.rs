// SPDX-License-Identifier: GPL-3.0-or-later

//! Pure shell-text lexing for the `parse-sh` producer mode.
//!
//! See `docs/requirements/interception-events-from-shell-text.md` for the
//! contract. This module performs no I/O: it is a pure function from shell
//! command text (`&str`) to an owned command/skip structure. Wiring it to
//! the CLI and folding its output into the execution-event stream are later
//! stages, not implemented here.

pub mod interpreter;
pub mod lexer;

pub use interpreter::{Context, Interpretation, SkippedLine, interpret};
pub use lexer::{LexedCommand, SimpleCommand, SkipReason, lex};
