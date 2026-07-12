// SPDX-License-Identifier: GPL-3.0-or-later

//! Streaming shell-text parsing for the `parse-sh` producer mode.
//!
//! See `docs/requirements/interception-events-from-shell-text.md` for the
//! contract, and `docs/rationale/parse-sh-single-tokenizer.md` for the
//! internal shape: a single streaming tokenizer owns every lexical rule
//! (quotes, comments, redirections, here-documents, continuations, make
//! markers), and a parser folds its token stream into execution events
//! while tracking the working directory and environment. Both are
//! incremental state machines over a caller-supplied buffered reader:
//! memory is bounded by the longest logical line, and events are yielded
//! as their line completes. No ambient I/O (`std::env`, `std::fs`); the
//! CLI wiring lives in the modes.

pub mod parser;
pub mod tokenizer;

pub use parser::{Context, Event, Parser, SkippedLine, parse};
pub use tokenizer::SkipReason;
