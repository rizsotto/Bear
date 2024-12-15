// SPDX-License-Identifier: GPL-3.0-or-later

use super::super::semantic;
use crate::ipc::{Envelope, Execution};
use crate::modes::Mode;
use crate::output::{OutputWriter, OutputWriterImpl};
use crate::semantic::transformation::Transformation;
use crate::semantic::Transform;
use crate::{args, config};
use anyhow::Context;
use serde_json::de::IoRead;
use serde_json::{Error, StreamDeserializer};
use std::convert::TryFrom;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::ExitCode;

/// The semantic mode we are deduct the semantic meaning of the
/// executed commands from the build process.
pub struct Semantic {
    event_source: EventFileReader,
    interpreter: Box<dyn semantic::Interpreter>,
    semantic_transform: Transformation,
    output_writer: OutputWriterImpl,
}

impl Semantic {
    /// Create a new semantic mode instance.
    pub fn from(
        input: args::BuildEvents,
        output: args::BuildSemantic,
        config: config::Main,
    ) -> anyhow::Result<Self> {
        let event_source = EventFileReader::try_from(input)?;
        let interpreter = Self::interpreter(&config)?;
        let semantic_transform = Transformation::from(&config.output);
        let output_writer = OutputWriterImpl::create(&output, &config.output)?;

        Ok(Self {
            event_source,
            interpreter,
            semantic_transform,
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
    pub fn interpreter(config: &config::Main) -> anyhow::Result<Box<dyn semantic::Interpreter>> {
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
}

impl Mode for Semantic {
    /// Run the semantic mode by generating the compilation database entries
    /// from the event source. The entries are then processed by the semantic
    /// recognition and transformation. The result is written to the output file.
    ///
    /// The exit code is based on the result of the output writer.
    fn run(self) -> anyhow::Result<ExitCode> {
        // Set up the pipeline of compilation database entries.
        let entries = self
            .event_source
            .generate()
            .inspect(|execution| log::debug!("execution: {}", execution))
            .flat_map(|execution| self.interpreter.recognize(&execution))
            .inspect(|semantic| log::debug!("semantic: {:?}", semantic))
            .flat_map(|semantic| self.semantic_transform.apply(semantic));
        // Consume the entries and write them to the output file.
        // The exit code is based on the result of the output writer.
        match self.output_writer.run(entries) {
            Ok(_) => Ok(ExitCode::SUCCESS),
            Err(_) => Ok(ExitCode::FAILURE),
        }
    }
}

/// Responsible for reading the build events from the intercept mode.
///
/// The file syntax is defined by the `events` module, and the parsing logic is implemented there.
/// Here we only handle the file opening and the error handling.
pub struct EventFileReader {
    stream: Box<dyn Iterator<Item = Result<Envelope, Error>>>,
}

impl TryFrom<args::BuildEvents> for EventFileReader {
    type Error = anyhow::Error;

    /// Open the file and create a new instance of the event file reader.
    ///
    /// If the file cannot be opened, the error will be logged and escalated.
    fn try_from(value: args::BuildEvents) -> Result<Self, Self::Error> {
        let file_name = PathBuf::from(value.file_name);
        let file = OpenOptions::new()
            .read(true)
            .open(file_name.as_path())
            .map(BufReader::new)
            .with_context(|| format!("Failed to open input file: {:?}", file_name))?;
        let stream = Box::new(StreamDeserializer::new(IoRead::new(file)));

        Ok(EventFileReader { stream })
    }
}

impl EventFileReader {
    /// Generate the build events from the file.
    ///
    /// Returns an iterator over the build events. Any error during the reading
    /// of the file will be logged and the failed entries will be skipped.
    pub fn generate(self) -> impl Iterator<Item = Execution> {
        self.stream.filter_map(|result| match result {
            Ok(value) => Some(value.event.execution),
            Err(error) => {
                log::error!("Failed to read event: {:?}", error);
                None
            }
        })
    }
}
