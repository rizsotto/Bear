// SPDX-License-Identifier: GPL-3.0-or-later

use crate::ipc::Envelope;
use crate::output::{OutputWriter, OutputWriterImpl};
use crate::semantic::transformation::Transformation;
use crate::semantic::Transform;
use crate::{args, config, semantic};

/// The semantic analysis that is independent of the event source.
pub(super) struct SemanticAnalysisPipeline {
    interpreter: Box<dyn semantic::Interpreter>,
    transform: Transformation,
    output_writer: OutputWriterImpl,
}

impl SemanticAnalysisPipeline {
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
