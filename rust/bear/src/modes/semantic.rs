// SPDX-License-Identifier: GPL-3.0-or-later

use crate::ipc::Envelope;
use crate::modes::Mode;
use crate::output::event::read;
use crate::output::{OutputWriter, OutputWriterImpl};
use crate::semantic::transformation::Transformation;
use crate::semantic::Transform;
use crate::{args, config, semantic};
use anyhow::Context;
use std::fs::{File, OpenOptions};
use std::io::BufReader;
use std::process::ExitCode;

/// The semantic mode we are deduct the semantic meaning of the
/// executed commands from the build process.
pub struct Semantic {
    event_file: BufReader<File>,
    semantic: SemanticFromEnvelopes,
}

impl Semantic {
    pub fn from(
        input: args::BuildEvents,
        output: args::BuildSemantic,
        config: config::Main,
    ) -> anyhow::Result<Self> {
        let file_name = input.file_name.as_str();
        let event_file = OpenOptions::new()
            .read(true)
            .open(file_name)
            .map(BufReader::new)
            .with_context(|| format!("Failed to open file: {:?}", file_name))?;

        let semantic = SemanticFromEnvelopes::from(output, &config)?;

        Ok(Self {
            event_file,
            semantic,
        })
    }
}

impl Mode for Semantic {
    /// Run the semantic mode by reading the event file and analyzing the events.
    ///
    /// The exit code is based on the result of the output writer.
    fn run(self) -> anyhow::Result<ExitCode> {
        self.semantic
            .analyze_and_write(read(self.event_file))
            .map(|_| ExitCode::SUCCESS)
    }
}

/// The semantic analysis that is independent of the event source.
pub struct SemanticFromEnvelopes {
    interpreter: Box<dyn semantic::Interpreter>,
    transform: Transformation,
    output_writer: OutputWriterImpl,
}

impl SemanticFromEnvelopes {
    /// Create a new semantic mode instance.
    pub(super) fn from(output: args::BuildSemantic, config: &config::Main) -> anyhow::Result<Self> {
        let interpreter = Self::interpreter(config)?;
        let transform = Transformation::from(&config.output);
        let output_writer = OutputWriterImpl::create(&output, &config.output)?;

        Ok(Self {
            interpreter,
            transform,
            output_writer,
        })
    }

    /// Creates an interpreter to recognize the compiler calls.
    ///
    /// Using the configuration we can define which compilers to include and exclude.
    /// Also read the environment variables to detect the compiler to include (and
    /// make sure those are not excluded either).
    // TODO: Use the CC or CXX environment variables to detect the compiler to include.
    //       Use the CC or CXX environment variables and make sure those are not excluded.
    //       Make sure the environment variables are passed to the method.
    // TODO: Move this method to the `semantic` module. (instead of expose the builder)
    // TODO: Take environment variables as input.
    fn interpreter(config: &config::Main) -> anyhow::Result<Box<dyn semantic::Interpreter>> {
        let compilers_to_include = match &config.intercept {
            config::Intercept::Wrapper { executables, .. } => executables.clone(),
            _ => vec![],
        };
        let compilers_to_exclude = match &config.output {
            config::Output::Clang { compilers, .. } => compilers
                .iter()
                .filter(|compiler| compiler.ignore == config::Ignore::Always)
                .map(|compiler| compiler.path.clone())
                .collect(),
            _ => vec![],
        };
        let interpreter = semantic::interpreters::Builder::new()
            .compilers_to_recognize(compilers_to_include.as_slice())
            .compilers_to_exclude(compilers_to_exclude.as_slice())
            .build();

        Ok(Box::new(interpreter))
    }

    /// Consumer the envelopes for analysis and write the result to the output file.
    /// This implements the pipeline of the semantic analysis.
    pub(super) fn analyze_and_write(
        self,
        envelopes: impl IntoIterator<Item = Envelope>,
    ) -> anyhow::Result<()> {
        // Set up the pipeline of compilation database entries.
        let entries = envelopes
            .into_iter()
            .map(|envelope| envelope.event.execution)
            .inspect(|execution| log::debug!("execution: {}", execution))
            .flat_map(|execution| self.interpreter.recognize(&execution))
            .inspect(|semantic| log::debug!("semantic: {:?}", semantic))
            .flat_map(|semantic| self.transform.apply(semantic));
        // Consume the entries and write them to the output file.
        // The exit code is based on the result of the output writer.
        self.output_writer.run(entries)
    }
}
