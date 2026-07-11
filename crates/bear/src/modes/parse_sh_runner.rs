// SPDX-License-Identifier: GPL-3.0-or-later

//! The `parse-sh` batch mode: reads shell command text, runs the pure
//! Stage 2/3 `parse_sh` lex/interpret pipeline over it, and writes the
//! resulting execution events.
//!
//! Unlike `Interceptor`/`Replayer`, this is a straight read -> interpret ->
//! write; by the time interpretation finishes the whole event list is in
//! memory, so there is no producer/consumer channel to coordinate.

use crate::args::{BuildEvents, ShScript};
use crate::output::{ExecutionEventDatabase, SerializationError, SerializationFormat};
use crate::parse_sh::{self, Context};
use intercept_supervisor::context;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Runs the `parse-sh` mode: reads `input` (a file, or standard input when
/// `-`), interprets it starting from `directory`, and writes the resulting
/// event stream to `output` (a file, or standard output when `-`).
pub struct ParseShRunner {
    input: PathBuf,
    output: PathBuf,
    directory: PathBuf,
}

impl ParseShRunner {
    /// Builds the runner from parsed arguments, resolving the working
    /// directory eagerly: when the caller left `--directory` unset, this
    /// falls back to Bear's invocation directory (captured in `context`) so
    /// `try_run` always has a concrete path to interpret from.
    pub fn new(input: ShScript, output: BuildEvents, context: &context::Context) -> Self {
        let directory = input.directory.unwrap_or_else(|| context.current_directory.clone());
        Self { input: input.path, output: output.path, directory }
    }

    /// Runs the mode to completion, translating any error into a logged
    /// message and `ExitCode::FAILURE`.
    pub fn run(self) -> ExitCode {
        match self.try_run() {
            Ok(code) => code,
            Err(error) => {
                log::error!("{error}");
                ExitCode::FAILURE
            }
        }
    }

    fn try_run(self) -> Result<ExitCode, ParseShError> {
        let text = Self::read_input(&self.input)?;

        let environment: HashMap<String, String> = std::env::vars().collect();

        let context = Context { working_dir: self.directory, environment };
        let interpretation = parse_sh::interpret(&text, &context);

        for skipped in &interpretation.skipped {
            log::warn!("line {}: skipped ({})", skipped.line, skipped.reason);
        }

        let event_count = interpretation.executions.len();
        let skip_count = interpretation.skipped.len();

        let writer = Self::open_output(&self.output)?;
        // Trim each execution's environment to the build-relevant subset,
        // matching the shape `bear intercept` emits (see `intercept::Execution::trim`).
        let executions = interpretation.executions.into_iter().map(intercept::Execution::trim);
        ExecutionEventDatabase::write(writer, executions).map_err(ParseShError::Write)?;

        if skip_count > 0 {
            log::warn!("parse-sh: {event_count} event(s) emitted, {skip_count} line(s) skipped");
        } else {
            log::info!("parse-sh: {event_count} event(s) emitted, {skip_count} line(s) skipped");
        }

        if event_count > 0 {
            Ok(ExitCode::SUCCESS)
        } else if skip_count == 0 {
            log::warn!("parse-sh: no commands found in input");
            Ok(ExitCode::SUCCESS)
        } else {
            log::error!("parse-sh: every non-empty line was skipped; no events emitted (see warnings above)");
            Ok(ExitCode::FAILURE)
        }
    }

    /// Reads the whole of `path` as text, or standard input when `path` is
    /// the `-` sentinel.
    fn read_input(path: &Path) -> Result<String, ParseShError> {
        if super::is_stdio(path) {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer).map_err(ParseShError::ReadStdin)?;
            Ok(buffer)
        } else {
            fs::read_to_string(path).map_err(|error| ParseShError::ReadFile(path.to_path_buf(), error))
        }
    }

    /// Opens the output sink for the event stream: standard output when
    /// `path` is the `-` sentinel, or a newly created file otherwise.
    ///
    /// This mode legitimately writes events to stdout (it runs no build, so
    /// there is no build output to collide with); it deliberately does not
    /// reuse `RawEventWriter`, which rejects `-` for exactly the opposite
    /// reason.
    fn open_output(path: &Path) -> Result<Box<dyn Write>, ParseShError> {
        if super::is_stdio(path) {
            Ok(Box::new(io::stdout().lock()))
        } else {
            let file = fs::File::create(path)
                .map_err(|error| ParseShError::CreateOutput(path.to_path_buf(), error))?;
            Ok(Box::new(io::BufWriter::new(file)))
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ParseShError {
    #[error("Failed to read shell text from standard input: {0}")]
    ReadStdin(std::io::Error),
    #[error("Failed to read shell text file {0}: {1}")]
    ReadFile(PathBuf, std::io::Error),
    #[error("Failed to create output file {0}: {1}")]
    CreateOutput(PathBuf, std::io::Error),
    #[error("Failed to write events: {0}")]
    Write(SerializationError),
}
