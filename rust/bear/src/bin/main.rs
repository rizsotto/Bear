// SPDX-License-Identifier: GPL-3.0-or-later
use std::process::ExitCode;

use bear::input::EventFileReader;
use bear::output::OutputWriter;
use bear::{args, config, semantic};
use intercept::Execution;
use log;

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
                let output_writer = OutputWriter::configure(&output, &config.output)?;
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
                    .flat_map(|semantic| semantic_transform.apply(semantic));
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
            config::Output::Clang { compilers, .. } => compilers
                .into_iter()
                .filter(|compiler| compiler.ignore == config::Ignore::Always)
                .map(|compiler| compiler.path.clone())
                .collect(),
            _ => vec![],
        };
        let tool = semantic::tools::Builder::new()
            .compilers_to_recognize(compilers_to_include.as_slice())
            .compilers_to_exclude(compilers_to_exclude.as_slice())
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
    Transform { compilers: Vec<config::Compiler> },
}

impl From<&config::Output> for SemanticTransform {
    fn from(config: &config::Output) -> Self {
        match config {
            config::Output::Clang { compilers, .. } => {
                if compilers.is_empty() {
                    SemanticTransform::NoTransform
                } else {
                    let compilers = compilers.clone();
                    SemanticTransform::Transform { compilers }
                }
            }
            config::Output::Semantic { .. } => SemanticTransform::NoTransform,
        }
    }
}

impl SemanticTransform {
    fn apply(&self, input: semantic::Meaning) -> Option<semantic::Meaning> {
        match &input {
            semantic::Meaning::Compiler {
                compiler,
                passes,
                working_dir,
            } => match self.lookup(&compiler) {
                Some(config::Compiler {
                    ignore: config::Ignore::Always,
                    ..
                }) => None,
                Some(config::Compiler {
                    ignore: config::Ignore::Conditional,
                    arguments,
                    ..
                }) => {
                    if Self::filter(arguments, passes) {
                        None
                    } else {
                        Some(input)
                    }
                }
                Some(config::Compiler {
                    ignore: config::Ignore::Never,
                    arguments,
                    ..
                }) => {
                    let new_passes = SemanticTransform::execute(arguments, passes);
                    Some(semantic::Meaning::Compiler {
                        compiler: compiler.clone(),
                        working_dir: working_dir.clone(),
                        passes: new_passes,
                    })
                }
                None => Some(input),
            },
            _ => Some(input),
        }
    }

    // TODO: allow multiple matches for the same compiler
    fn lookup(&self, compiler: &std::path::Path) -> Option<&config::Compiler> {
        match self {
            SemanticTransform::Transform { compilers } => {
                compilers.iter().find(|c| c.path == compiler)
            }
            _ => None,
        }
    }

    fn filter(arguments: &config::Arguments, passes: &[semantic::CompilerPass]) -> bool {
        let match_flags = arguments.match_.as_slice();
        passes.iter().any(|pass| match pass {
            semantic::CompilerPass::Compile { flags, .. } => {
                flags.iter().any(|flag| match_flags.contains(flag))
            }
            _ => false,
        })
    }

    fn execute(
        arguments: &config::Arguments,
        passes: &[semantic::CompilerPass],
    ) -> Vec<semantic::CompilerPass> {
        let arguments_to_remove = arguments.remove.as_slice();
        let arguments_to_add = arguments.add.as_slice();

        let mut new_passes = Vec::with_capacity(passes.len());
        for pass in passes {
            match pass {
                semantic::CompilerPass::Compile {
                    source,
                    output,
                    flags,
                } => {
                    let mut new_flags = flags.clone();
                    new_flags.retain(|flag| !arguments_to_remove.contains(flag));
                    new_flags.extend(arguments_to_add.iter().cloned());
                    new_passes.push(semantic::CompilerPass::Compile {
                        source: source.clone(),
                        output: output.clone(),
                        flags: new_flags,
                    });
                }
                semantic::CompilerPass::Preprocess => {
                    new_passes.push(semantic::CompilerPass::Preprocess)
                }
            }
        }
        new_passes
    }
}
