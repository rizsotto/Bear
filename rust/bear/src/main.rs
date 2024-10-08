/*  Copyright (C) 2012-2024 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */
use crate::output::OutputWriter;
use anyhow::Context;
use intercept::ipc::Execution;
use log;
use semantic;
use std::fs::{File, OpenOptions};
use std::io::BufReader;
use std::path::PathBuf;
use std::process::ExitCode;

mod args;
mod config;
pub mod events;
mod filter;
mod fixtures;
mod output;

/// Driver function of the application.
fn main() -> anyhow::Result<ExitCode> {
    // Initialize the logging system.
    env_logger::init();
    // Get the package name and version from Cargo
    let pkg_name = env!("CARGO_PKG_NAME");
    let pkg_version = env!("CARGO_PKG_VERSION");
    log::debug!("{} v{}", pkg_name, pkg_version);

    // Parse the command line arguments.
    let matches = args::cli().get_matches();
    let arguments = args::Arguments::try_from(matches)?;

    // Print the arguments.
    log::debug!("Arguments: {:?}", arguments);
    // Load the configuration.
    let configuration = config::Main::load(&arguments.config)?;
    log::debug!("Configuration: {:?}", configuration);

    // Run the application.
    let application = Application::configure(arguments, configuration)?;
    let result = application.run();
    log::debug!("Exit code: {:?}", result);

    Ok(result)
}

/// Represent the application state.
enum Application {
    /// The intercept mode we are only capturing the build commands.
    Intercept {
        input: args::BuildCommand,
        output: args::BuildEvents,
        intercept_config: config::Intercept,
    },
    /// The semantic mode we are deduct the semantic meaning of the
    /// executed commands from the build process.
    Semantic {
        event_source: EventFileReader,
        semantic_recognition: SemanticRecognition,
        semantic_transform: SemanticTransform,
        output_writer: OutputWriter,
    },
    /// The all model is combining the intercept and semantic modes.
    All {
        input: args::BuildCommand,
        output: args::BuildSemantic,
        intercept_config: config::Intercept,
        output_config: config::Output,
    },
}

impl Application {
    /// Configure the application based on the command line arguments and the configuration.
    ///
    /// Trying to validate the configuration and the arguments, while creating the application
    /// state that will be used by the `run` method. Trying to catch problems early before
    /// the actual execution of the application.
    fn configure(args: args::Arguments, config: config::Main) -> anyhow::Result<Self> {
        match args.mode {
            args::Mode::Intercept { input, output } => {
                let intercept_config = config.intercept;
                let result = Application::Intercept {
                    input,
                    output,
                    intercept_config,
                };
                Ok(result)
            }
            args::Mode::Semantic { input, output } => {
                let event_source = EventFileReader::try_from(input)?;
                let semantic_recognition = SemanticRecognition::try_from(&config)?;
                let semantic_transform = SemanticTransform::from(&config.output);
                let output_writer = OutputWriter::configure(&output, &config.output);
                let result = Application::Semantic {
                    event_source,
                    semantic_recognition,
                    semantic_transform,
                    output_writer,
                };
                Ok(result)
            }
            args::Mode::All { input, output } => {
                let intercept_config = config.intercept;
                let output_config = config.output;
                let result = Application::All {
                    input,
                    output,
                    intercept_config,
                    output_config,
                };
                Ok(result)
            }
        }
    }

    /// Executes the configured application.
    fn run(self) -> ExitCode {
        match self {
            Application::Intercept {
                input,
                output,
                intercept_config,
            } => {
                // TODO: Implement the intercept mode.
                ExitCode::FAILURE
            }
            Application::Semantic {
                event_source,
                semantic_recognition,
                semantic_transform,
                output_writer,
            } => {
                // Set up the pipeline of compilation database entries.
                let entries = event_source
                    .generate()
                    .flat_map(|execution| semantic_recognition.apply(execution))
                    .map(|semantic| semantic_transform.apply(semantic));
                // Consume the entries and write them to the output file.
                // The exit code is based on the result of the output writer.
                match output_writer.run(entries) {
                    Ok(_) => ExitCode::SUCCESS,
                    Err(_) => ExitCode::FAILURE,
                }
            }
            Application::All {
                input,
                output,
                intercept_config,
                output_config,
            } => {
                // TODO: Implement the all mode.
                ExitCode::FAILURE
            }
        }
    }
}

/// Responsible for reading the build events from the intercept mode.
///
/// The file syntax is defined by the `events` module, and the parsing logic is implemented there.
/// Here we only handle the file opening and the error handling.
struct EventFileReader {
    reader: BufReader<File>,
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
            .with_context(|| format!("Failed to open input file: {:?}", file_name))?;
        let reader = BufReader::new(file);

        Ok(EventFileReader { reader })
    }
}

impl EventFileReader {
    /// Generate the build events from the file.
    ///
    /// Returns an iterator over the build events. Any error during the reading
    /// of the file will be logged and the failed entries will be skipped.
    fn generate(self) -> impl Iterator<Item = Execution> {
        // Process the file line by line.
        events::from_reader(self.reader)
            // Log the errors and skip the failed entries.
            .flat_map(|candidate| match candidate {
                Ok(execution) => Some(execution),
                Err(error) => {
                    log::warn!("Failed to read entry from input: {}", error);
                    None
                }
            })
    }
}

/// Responsible for recognizing the semantic meaning of the executed commands.
///
/// The recognition logic is implemented in the `tools` module. Here we only handle
/// the errors and logging them to the console.
struct SemanticRecognition {
    tool: Box<dyn semantic::Tool>,
}

impl TryFrom<&config::Main> for SemanticRecognition {
    type Error = anyhow::Error;

    fn try_from(config: &config::Main) -> Result<Self, Self::Error> {
        let compilers_to_include = match &config.intercept {
            config::Intercept::Wrapper { executables, .. } => executables.clone(),
            _ => vec![],
        };
        let compilers_to_exclude = match &config.output {
            config::Output::Clang { filter, .. } => filter.compilers.with_paths.clone(),
            _ => vec![],
        };
        let arguments_to_exclude = match &config.output {
            config::Output::Clang { filter, .. } => filter.compilers.with_arguments.clone(),
            _ => vec![],
        };
        let tool = semantic::tools::Builder::new()
            .compilers_to_recognize(compilers_to_include.as_slice())
            .compilers_to_exclude(compilers_to_exclude.as_slice())
            .compilers_to_exclude_by_arguments(arguments_to_exclude.as_slice())
            .build();

        Ok(SemanticRecognition {
            tool: Box::new(tool),
        })
    }
}

impl SemanticRecognition {
    fn apply(&self, execution: Execution) -> Option<semantic::Meaning> {
        match self.tool.recognize(&execution) {
            semantic::RecognitionResult::Recognized(Ok(semantic::Meaning::Ignored)) => {
                log::debug!("execution recognized, but ignored: {:?}", execution);
                None
            }
            semantic::RecognitionResult::Recognized(Ok(semantic)) => {
                log::debug!(
                    "execution recognized as compiler call, {:?} : {:?}",
                    semantic,
                    execution
                );
                Some(semantic)
            }
            semantic::RecognitionResult::Recognized(Err(reason)) => {
                log::debug!(
                    "execution recognized with failure, {:?} : {:?}",
                    reason,
                    execution
                );
                None
            }
            semantic::RecognitionResult::NotRecognized => {
                log::debug!("execution not recognized: {:?}", execution);
                None
            }
        }
    }
}

/// Responsible for transforming the semantic meaning of the compiler calls
/// into compilation database entries.
///
/// Modifies the compiler flags based on the configuration. Ignores non-compiler calls.
enum SemanticTransform {
    NoTransform,
    Transform {
        arguments_to_add: Vec<String>,
        arguments_to_remove: Vec<String>,
    },
}

impl From<&config::Output> for SemanticTransform {
    fn from(config: &config::Output) -> Self {
        match config {
            config::Output::Clang { transform, .. } => {
                if transform.arguments_to_add.is_empty() && transform.arguments_to_remove.is_empty()
                {
                    SemanticTransform::NoTransform
                } else {
                    SemanticTransform::Transform {
                        arguments_to_add: transform.arguments_to_add.clone(),
                        arguments_to_remove: transform.arguments_to_remove.clone(),
                    }
                }
            }
            config::Output::Semantic { .. } => SemanticTransform::NoTransform,
        }
    }
}

impl SemanticTransform {
    fn apply(&self, input: semantic::Meaning) -> semantic::Meaning {
        match input {
            semantic::Meaning::Compiler {
                compiler,
                working_dir,
                passes,
            } if matches!(self, SemanticTransform::Transform { .. }) => {
                let passes_transformed = passes
                    .into_iter()
                    .map(|pass| self.transform_pass(pass))
                    .collect();

                semantic::Meaning::Compiler {
                    compiler,
                    working_dir,
                    passes: passes_transformed,
                }
            }
            _ => input,
        }
    }

    fn transform_pass(&self, pass: semantic::CompilerPass) -> semantic::CompilerPass {
        match pass {
            semantic::CompilerPass::Compile {
                source,
                output,
                flags,
            } => match self {
                SemanticTransform::Transform {
                    arguments_to_add,
                    arguments_to_remove,
                } => {
                    let flags_transformed = flags
                        .into_iter()
                        .filter(|flag| !arguments_to_remove.contains(flag))
                        .chain(arguments_to_add.iter().cloned())
                        .collect();
                    semantic::CompilerPass::Compile {
                        source,
                        output,
                        flags: flags_transformed,
                    }
                }
                _ => panic!("This is a bug! Please report it to the developers."),
            },
            _ => pass,
        }
    }
}
