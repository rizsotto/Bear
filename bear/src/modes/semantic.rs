// SPDX-License-Identifier: GPL-3.0-or-later

use crate::intercept::Event;
use crate::semantic::interpreters;
use crate::{args, config, output, semantic};

pub(super) struct SemanticAnalysis {
    interpreter: Box<dyn semantic::Interpreter>,
}

impl TryFrom<&config::Main> for SemanticAnalysis {
    type Error = anyhow::Error;

    fn try_from(config: &config::Main) -> Result<Self, Self::Error> {
        let interpreter = interpreters::create(config);

        Ok(Self {
            interpreter: Box::new(interpreter),
        })
    }
}

impl SemanticAnalysis {
    pub fn analyze(&self, event: Event) -> Option<semantic::Command> {
        log::debug!("event: {event}");
        match self.interpreter.recognize(&event.execution) {
            Some(recognized) => {
                log::debug!("recognized semantic: {recognized:?}");
                Some(recognized)
            }
            None => {
                log::debug!("not recognized");
                None
            }
        }
    }
}

/// The semantic analysis that is independent of the event source.
pub(super) struct SemanticAnalysisPipeline {
    analyzer: SemanticAnalysis,
    writer: output::OutputWriter,
}

impl SemanticAnalysisPipeline {
    /// Create a new semantic mode instance.
    pub(super) fn create(
        output: args::BuildSemantic,
        config: &config::Main,
    ) -> anyhow::Result<Self> {
        let analyzer = SemanticAnalysis::try_from(config)?;
        let writer = output::OutputWriter::try_from((&output, &config.output))?;

        Ok(Self { analyzer, writer })
    }

    /// Consumer the envelopes for analysis and write the result to the output file.
    /// This implements the pipeline of the semantic analysis.
    pub(super) fn analyze_and_write(
        self,
        events: impl IntoIterator<Item = Event>,
    ) -> anyhow::Result<()> {
        // Set up the pipeline of compilation database entries.
        let semantics = events
            .into_iter()
            .flat_map(|event| self.analyzer.analyze(event));
        // Consume the entries and write them to the output file.
        // The exit code is based on the result of the output writer.
        self.writer.write(semantics)?;

        Ok(())
    }
}
