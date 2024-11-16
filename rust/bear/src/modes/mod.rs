// SPDX-License-Identifier: GPL-3.0-or-later

mod intercept;

use crate::input::EventFileReader;
use crate::output::OutputWriter;
use crate::recognition::Recognition;
use crate::transformation::Transformation;
use crate::{args, config};
use intercept::{InterceptEnvironment, InterceptService};
use std::process::ExitCode;
use std::thread;

/// The mode trait is used to run the application in different modes.
pub trait Mode {
    fn run(self) -> ExitCode;
}

/// The intercept mode we are only capturing the build commands.
pub struct Intercept {
    input: args::BuildCommand,
    output: args::BuildEvents,
    config: config::Intercept,
}

/// The semantic mode we are deduct the semantic meaning of the
/// executed commands from the build process.
pub struct Semantic {
    event_source: EventFileReader,
    semantic_recognition: Recognition,
    semantic_transform: Transformation,
    output_writer: OutputWriter,
}

/// The all model is combining the intercept and semantic modes.
pub struct All {
    input: args::BuildCommand,
    output: args::BuildSemantic,
    intercept_config: config::Intercept,
    output_config: config::Output,
}

impl Intercept {
    pub fn new(
        input: args::BuildCommand,
        output: args::BuildEvents,
        config: config::Intercept,
    ) -> Self {
        Self {
            input,
            output,
            config,
        }
    }
}

impl Mode for Intercept {
    fn run(self) -> ExitCode {
        match &self.config {
            config::Intercept::Wrapper { .. } => {
                let service =
                    InterceptService::new().expect("Failed to create the intercept service");
                let environment = InterceptEnvironment::new(&self.config, service.address())
                    .expect("Failed to create the intercept environment");

                // start writer thread
                let writer_thread = thread::spawn(move || {
                    let mut writer = std::fs::File::create(self.output.file_name)
                        .expect("Failed to create the output file");
                    for envelope in service.receiver().iter() {
                        envelope
                            .write_into(&mut writer)
                            .expect("Failed to write the envelope");
                    }
                });

                let status = environment.execute_build_command(self.input);

                writer_thread
                    .join()
                    .expect("Failed to join the writer thread");

                status.unwrap_or(ExitCode::FAILURE)
            }
            config::Intercept::Preload { .. } => {
                todo!()
            }
        }
    }
}

impl Semantic {
    pub fn new(
        event_source: EventFileReader,
        semantic_recognition: Recognition,
        semantic_transform: Transformation,
        output_writer: OutputWriter,
    ) -> Self {
        Self {
            event_source,
            semantic_recognition,
            semantic_transform,
            output_writer,
        }
    }
}

impl Mode for Semantic {
    fn run(self) -> ExitCode {
        // Set up the pipeline of compilation database entries.
        let entries = self
            .event_source
            .generate()
            .flat_map(|execution| self.semantic_recognition.apply(execution))
            .flat_map(|semantic| self.semantic_transform.apply(semantic));
        // Consume the entries and write them to the output file.
        // The exit code is based on the result of the output writer.
        match self.output_writer.run(entries) {
            Ok(_) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        }
    }
}

impl All {
    pub fn new(
        input: args::BuildCommand,
        output: args::BuildSemantic,
        intercept_config: config::Intercept,
        output_config: config::Output,
    ) -> Self {
        Self {
            input,
            output,
            intercept_config,
            output_config,
        }
    }
}

impl Mode for All {
    fn run(self) -> ExitCode {
        // TODO: Implement the all mode.
        ExitCode::FAILURE
    }
}
