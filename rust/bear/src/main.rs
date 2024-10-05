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
use anyhow::Context;
use intercept::ipc::Execution;
use json_compilation_db::Entry;
use log;
use semantic::events;
use semantic::filter;
use semantic::tools;
use semantic::result;
use serde_json::Error;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod args;
mod config;
mod fixtures;

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
    fn configure(
        args: args::Arguments,
        config: config::Main,
    ) -> anyhow::Result<Self> {
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
                    .flat_map(|execution| semantic_recognition.recognize(execution))
                    .flat_map(|semantic| semantic_transform.into_entries(semantic));
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
    tool: Box<dyn tools::Tool>,
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
        let tool = tools::from(
            compilers_to_include.as_slice(),
            compilers_to_exclude.as_slice(),
        );
        Ok(SemanticRecognition { tool })
    }
}

impl SemanticRecognition {
    fn recognize(&self, execution: Execution) -> Option<result::Semantic> {
        match self.tool.recognize(&execution) {
            result::RecognitionResult::Recognized(Ok(result::Semantic::UnixCommand)) => {
                log::debug!("execution recognized as unix command: {:?}", execution);
                None
            }
            result::RecognitionResult::Recognized(Ok(result::Semantic::BuildCommand)) => {
                log::debug!("execution recognized as build command: {:?}", execution);
                None
            }
            result::RecognitionResult::Recognized(Ok(semantic)) => {
                log::debug!(
                    "execution recognized as compiler call, {:?} : {:?}",
                    semantic,
                    execution
                );
                Some(semantic)
            }
            result::RecognitionResult::Recognized(Err(reason)) => {
                log::debug!(
                    "execution recognized with failure, {:?} : {:?}",
                    reason,
                    execution
                );
                None
            }
            result::RecognitionResult::NotRecognized => {
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
    fn into_entries(&self, semantic: result::Semantic) -> Vec<Entry> {
        let transformed = self.transform_semantic(semantic);
        let entries: Result<Vec<Entry>, anyhow::Error> = transformed.try_into();
        entries.unwrap_or_else(|error| {
            log::debug!(
                "compiler call failed to convert to compilation db entry: {}",
                error
            );
            vec![]
        })
    }

    fn transform_semantic(&self, input: result::Semantic) -> result::Semantic {
        match input {
            result::Semantic::Compiler {
                compiler,
                working_dir,
                passes,
            } if matches!(self, SemanticTransform::Transform { .. }) => {
                let passes_transformed = passes
                    .into_iter()
                    .map(|pass| self.transform_pass(pass))
                    .collect();

                result::Semantic::Compiler {
                    compiler,
                    working_dir,
                    passes: passes_transformed,
                }
            }
            _ => input,
        }
    }

    fn transform_pass(&self, pass: result::CompilerPass) -> result::CompilerPass {
        match pass {
            result::CompilerPass::Compile {
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
                    result::CompilerPass::Compile {
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

/// Responsible for writing the final compilation database file.
///
/// Implements filtering, formatting and atomic file writing.
/// (Atomic file writing implemented by writing to a temporary file and renaming it.)
///
/// Filtering is implemented by the `filter` module, and the formatting is implemented by the
/// `json_compilation_db` module.
struct OutputWriter {
    output: PathBuf,
    append: bool,
    filter: config::Filter,
    format: config::Format,
}

impl OutputWriter {
    /// Create a new instance of the output writer.
    pub fn configure(value: &args::BuildSemantic, config: &config::Output) -> Self {
        match config {
            config::Output::Clang { format, filter, .. } => OutputWriter {
                output: PathBuf::from(&value.file_name),
                append: value.append,
                filter: Self::validate_filter(filter),
                format: format.clone(),
            },
            config::Output::Semantic { .. } => {
                todo!("implement this case")
            }
        }
    }

    /// Validate the configuration of the output writer.
    ///
    /// Validation is always successful, but it may modify the configuration values.
    fn validate_filter(filter: &config::Filter) -> config::Filter {
        let mut result = filter.clone();
        result.duplicates.by_fields =
            Self::validate_duplicates_by_fields(filter.duplicates.by_fields.as_slice());
        result
    }

    /// Validate the fields of the configuration.
    ///
    /// Removes the duplicates from the list of fields.
    fn validate_duplicates_by_fields(
        fields: &[config::OutputFields],
    ) -> Vec<config::OutputFields> {
        fields
            .into_iter()
            .map(|field| field.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// Implements the main logic of the output writer.
    pub fn run(&self, entries: impl Iterator<Item = Entry>) -> anyhow::Result<()> {
        if self.append && self.output.exists() {
            let from_db = Self::read_from_compilation_db(Path::new(&self.output))?;
            let final_entries = entries.chain(from_db);
            self.write_into_compilation_db(final_entries)
        } else {
            if self.append {
                log::warn!("The output file does not exist, the append option is ignored.");
            }
            self.write_into_compilation_db(entries)
        }
    }

    fn write_into_compilation_db(
        &self,
        entries: impl Iterator<Item = Entry>,
    ) -> anyhow::Result<()> {
        // Filter out the entries as per the configuration.
        let filter: filter::EntryPredicate = TryFrom::try_from(&self.filter)?;
        let filtered_entries = entries.filter(filter);
        // Write the entries to a temporary file.
        self.write_into_temporary_compilation_db(filtered_entries)
            .and_then(|temp| {
                // Rename the temporary file to the final output.
                std::fs::rename(temp.as_path(), self.output.as_path()).with_context(|| {
                    format!(
                        "Failed to rename file from '{:?}' to '{:?}'.",
                        temp.as_path(),
                        self.output.as_path()
                    )
                })
            })
    }

    /// Write the entries to a temporary file and returns the temporary file name.
    fn write_into_temporary_compilation_db(
        &self,
        entries: impl Iterator<Item = Entry>,
    ) -> anyhow::Result<PathBuf> {
        // FIXME: Implement entry formatting.

        // Generate a temporary file name.
        let file_name = self.output.with_extension("tmp");
        // Open the file for writing.
        let file = File::create(&file_name)
            .with_context(|| format!("Failed to create file: {:?}", file_name.as_path()))?;
        // Write the entries to the file.
        json_compilation_db::write(BufWriter::new(file), entries)?;
        // Return the temporary file name.
        Ok(file_name)
    }

    /// Read the compilation database from a file.
    fn read_from_compilation_db(source: &Path) -> anyhow::Result<impl Iterator<Item = Entry>> {
        let file = OpenOptions::new()
            .read(true)
            .open(source)
            .with_context(|| format!("Failed to open file: {:?}", source))?;
        let entries = json_compilation_db::read(BufReader::new(file))
            .flat_map(Self::failed_entry_read_logged);

        Ok(entries)
    }

    fn failed_entry_read_logged(candidate: Result<Entry, Error>) -> Option<Entry> {
        match candidate {
            Ok(entry) => Some(entry),
            Err(error) => {
                // FIXME: write the file name to the log.
                log::error!("Failed to read entry: {}", error);
                None
            }
        }
    }
}

impl TryFrom<&config::Filter> for filter::EntryPredicate {
    type Error = anyhow::Error;

    /// Create a filter from the configuration.
    fn try_from(config: &config::Filter) -> Result<Self, Self::Error> {
        // - Check if the source file exists
        // - Check if the source file is not in the exclude list of the configuration
        // - Check if the source file is in the include list of the configuration
        let source_exist_check = filter::EntryPredicateBuilder::filter_by_source_existence(
            config.source.include_only_existing_files,
        );
        let source_paths_to_exclude = filter::EntryPredicateBuilder::filter_by_compiler_paths(
            config.source.paths_to_exclude.clone(),
        );
        let source_paths_to_include = filter::EntryPredicateBuilder::filter_by_compiler_paths(
            config.source.paths_to_include.clone(),
        );
        let source_checks = source_exist_check & !source_paths_to_exclude & source_paths_to_include;
        // - Check if the compiler path is not in the list of the configuration
        // - Check if the compiler arguments are not in the list of the configuration
        let compiler_with_path = filter::EntryPredicateBuilder::filter_by_compiler_paths(
            config.compilers.with_paths.clone(),
        );
        let compiler_with_argument = filter::EntryPredicateBuilder::filter_by_compiler_arguments(
            config.compilers.with_arguments.clone(),
        );
        let compiler_checks = !compiler_with_path & !compiler_with_argument;
        // - Check if the entry is not a duplicate based on the fields of the configuration
        let hash_function = create_hash(config.duplicates.by_fields.clone());
        let duplicates = filter::EntryPredicateBuilder::filter_duplicate_entries(hash_function);

        Ok((source_checks & compiler_checks & duplicates).build())
    }
}

fn create_hash(fields: Vec<config::OutputFields>) -> impl Fn(&Entry) -> u64 + 'static {
    move |entry: &Entry| {
        let mut hasher = DefaultHasher::new();
        for field in &fields {
            match field {
                config::OutputFields::Directory => entry.directory.hash(&mut hasher),
                config::OutputFields::File => entry.file.hash(&mut hasher),
                config::OutputFields::Arguments => entry.arguments.hash(&mut hasher),
                config::OutputFields::Output => entry.output.hash(&mut hasher),
            }
        }
        hasher.finish()
    }
}
