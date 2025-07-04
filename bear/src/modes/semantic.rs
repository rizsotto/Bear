// SPDX-License-Identifier: GPL-3.0-or-later

use crate::intercept::Event;
use crate::semantic::interpreters;
use crate::{args, config, output, semantic};

/// The semantic analysis that is independent of the event source.
pub(super) struct SemanticAnalysisPipeline {
    interpreter: Box<dyn semantic::Interpreter>,
    writer: output::OutputWriter,
}

impl SemanticAnalysisPipeline {
    /// Create a new semantic mode instance.
    pub(super) fn create(
        output: args::BuildSemantic,
        config: &config::Main,
    ) -> anyhow::Result<Self> {
        let interpreter = interpreters::create(config);
        let writer = output::OutputWriter::try_from((&output, &config.output))?;

        Ok(Self {
            interpreter: Box::new(interpreter),
            writer,
        })
    }

    /// Consumer the envelopes for analysis and write the result to the output file.
    /// This implements the pipeline of the semantic analysis.
    pub(super) fn analyze_and_write(
        self,
        events: impl IntoIterator<Item = Event>,
    ) -> anyhow::Result<()> {
        // Set up the pipeline of compilation database entries.
        let semantics = events.into_iter().flat_map(|event| {
            // FIXME: add logging for this step
            self.interpreter.recognize(&event.execution)
        });
        // Consume the entries and write them to the output file.
        // The exit code is based on the result of the output writer.
        self.writer.write(semantics)?;

        Ok(())
    }
}
