// SPDX-License-Identifier: GPL-3.0-or-later

use bear::input::EventFileReader;
use bear::intercept::collector::{EventCollector, EventCollectorOnTcp};
use bear::intercept::{Envelope, KEY_DESTINATION, KEY_PRELOAD_PATH};
use bear::output::OutputWriter;
use bear::recognition::Recognition;
use bear::transformation::Transformation;
use bear::{args, config};
use crossbeam_channel::{bounded, Receiver};
use log;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::Arc;
use std::{env, thread};

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
        config: config::Intercept,
    },
    /// The semantic mode we are deduct the semantic meaning of the
    /// executed commands from the build process.
    Semantic {
        event_source: EventFileReader,
        semantic_recognition: Recognition,
        semantic_transform: Transformation,
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
                    config: intercept_config,
                };
                Ok(result)
            }
            args::Mode::Semantic { input, output } => {
                let event_source = EventFileReader::try_from(input)?;
                let semantic_recognition = Recognition::try_from(&config)?;
                let semantic_transform = Transformation::from(&config.output);
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
                config,
            } => {
                match &config {
                    config::Intercept::Wrapper { .. } => {
                        let service = InterceptService::new()
                            .expect("Failed to create the intercept service");
                        let environment = InterceptEnvironment::new(&config, service.address())
                            .expect("Failed to create the intercept environment");

                        // start writer thread
                        let writer_thread = thread::spawn(move || {
                            let mut writer = std::fs::File::create(output.file_name)
                                .expect("Failed to create the output file");
                            for envelope in service.receiver().iter() {
                                envelope
                                    .write_into(&mut writer)
                                    .expect("Failed to write the envelope");
                            }
                        });

                        let status = environment.execute_build_command(input);

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

struct InterceptService {
    collector: Arc<EventCollectorOnTcp>,
    receiver: Receiver<Envelope>,
    collector_thread: Option<thread::JoinHandle<()>>,
}

impl InterceptService {
    pub fn new() -> anyhow::Result<Self> {
        let collector = EventCollectorOnTcp::new()?;
        let collector_arc = Arc::new(collector);
        let (sender, receiver) = bounded(32);

        let collector_in_thread = collector_arc.clone();
        let collector_thread = thread::spawn(move || {
            collector_in_thread.collect(sender).unwrap();
        });

        Ok(InterceptService {
            collector: collector_arc,
            receiver,
            collector_thread: Some(collector_thread),
        })
    }

    pub fn receiver(&self) -> Receiver<Envelope> {
        self.receiver.clone()
    }

    pub fn address(&self) -> String {
        self.collector.address()
    }
}

impl Drop for InterceptService {
    fn drop(&mut self) {
        self.collector.stop().expect("Failed to stop the collector");
        if let Some(thread) = self.collector_thread.take() {
            thread.join().expect("Failed to join the collector thread");
        }
    }
}

enum InterceptEnvironment {
    Wrapper {
        bin_dir: tempfile::TempDir,
        address: String,
    },
    Preload {
        path: PathBuf,
        address: String,
    },
}

impl InterceptEnvironment {
    pub fn new(config: &config::Intercept, address: String) -> anyhow::Result<Self> {
        let result = match config {
            config::Intercept::Wrapper {
                path,
                directory,
                executables,
            } => {
                // Create a temporary directory and populate it with the executables.
                let bin_dir = tempfile::TempDir::with_prefix_in(directory, "bear-")?;
                for executable in executables {
                    std::fs::hard_link(&executable, &path)?;
                }
                InterceptEnvironment::Wrapper { bin_dir, address }
            }
            config::Intercept::Preload { path } => InterceptEnvironment::Preload {
                path: path.clone(),
                address,
            },
        };
        Ok(result)
    }

    pub fn execute_build_command(self, input: args::BuildCommand) -> anyhow::Result<ExitCode> {
        let environment = self.environment();
        let mut child = Command::new(input.arguments[0].clone())
            .args(input.arguments)
            .envs(environment)
            .spawn()?;

        let result = child.wait()?;

        if result.success() {
            Ok(ExitCode::SUCCESS)
        } else {
            result
                .code()
                .map_or(Ok(ExitCode::FAILURE), |code| Ok(ExitCode::from(code as u8)))
        }
    }

    fn environment(&self) -> Vec<(String, String)> {
        match self {
            InterceptEnvironment::Wrapper {
                bin_dir, address, ..
            } => {
                let path_original = env::var("PATH").unwrap_or_else(|_| String::new());
                let path_updated = InterceptEnvironment::insert_to_path(
                    &path_original,
                    Self::to_string(bin_dir.path()),
                );
                vec![
                    ("PATH".to_string(), path_updated),
                    (KEY_DESTINATION.to_string(), address.clone()),
                ]
            }
            InterceptEnvironment::Preload { path, address, .. } => {
                let path_original = env::var(KEY_PRELOAD_PATH).unwrap_or_else(|_| String::new());
                let path_updated =
                    InterceptEnvironment::insert_to_path(&path_original, Self::to_string(path));
                vec![
                    (KEY_PRELOAD_PATH.to_string(), path_updated),
                    (KEY_DESTINATION.to_string(), address.clone()),
                ]
            }
        }
    }

    fn insert_to_path(original: &str, first: String) -> String {
        let mut paths: Vec<_> = original.split(':').filter(|it| it != &first).collect();
        paths.insert(0, first.as_str());
        paths.join(":")
    }

    fn to_string(path: &Path) -> String {
        path.to_str().unwrap_or("").to_string()
    }
}
