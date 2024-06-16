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

extern crate core;

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, stdin, stdout};
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use clap::{arg, ArgAction, command};
use json_compilation_db::Entry;
use log::LevelFilter;
use serde_json::Error;
use simple_logger::SimpleLogger;
use intercept::ipc::Execution;

use crate::configuration::Configuration;
use crate::filter::EntryPredicate;
use crate::tools::{RecognitionResult, Semantic, Tool};

mod configuration;
mod events;
mod compilation;
mod tools;
mod filter;
mod fixtures;

fn main() -> Result<()> {
    let arguments = Arguments::parse()?;
    prepare_logging(arguments.verbose)?;

    let application = Application::configure(arguments)?;
    application.run()?;

    Ok(())
}

#[derive(Debug, PartialEq)]
struct Arguments {
    input: String,
    output: String,
    config: Option<String>,
    append: bool,
    verbose: u8,
}

impl Arguments {
    fn parse() -> Result<Self> {
        let matches = command!()
            .args(&[
                arg!(-i --input <FILE> "Path of the event file")
                    .default_value("commands.json")
                    .hide_default_value(false),
                arg!(-o --output <FILE> "Path of the result file")
                    .default_value("compile_commands.json")
                    .hide_default_value(false),
                arg!(-c --config <FILE> "Path of the config file"),
                arg!(-a --append "Append result to an existing output file")
                    .action(ArgAction::SetTrue),
                arg!(-v --verbose ... "Sets the level of verbosity")
                    .action(ArgAction::Count),
            ])
            .get_matches();

        Arguments {
            input: matches.get_one::<String>("input")
                .expect("input is defaulted")
                .clone(),
            output: matches.get_one::<String>("output")
                .expect("output is defaulted")
                .clone(),
            config: matches.get_one::<String>("config")
                .map(String::to_string),
            append: *matches.get_one::<bool>("append")
                .unwrap_or(&false),
            verbose: matches.get_count("verbose"),
        }
            .validate()
    }

    fn validate(self) -> Result<Self> {
        if self.input == "-" && self.config.as_deref() == Some("-") {
            return Err(anyhow!("Both input and config reading the standard input."));
        }
        if self.append && self.output == "-" {
            return Err(anyhow!("Append can't applied to the standard output."));
        }

        Ok(self)
    }
}

fn prepare_logging(level: u8) -> Result<()> {
    let level = match level {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        3 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };
    let mut logger = SimpleLogger::new()
        .with_level(level);
    if level <= LevelFilter::Debug {
        logger = logger.with_local_timestamps()
    }
    logger.init()?;

    Ok(())
}

fn read_configuration(file: &Option<String>) -> Result<Configuration> {
    let configuration = match file.as_deref() {
        Some("-") | Some("/dev/stdin") => {
            let reader = stdin();
            serde_json::from_reader(reader)
                .context("Failed to read configuration from stdin")?
        }
        Some(file) => {
            let reader = OpenOptions::new().read(true).open(file)?;
            serde_json::from_reader(reader)
                .with_context(|| format!("Failed to read configuration from file: {}", file))?
        }
        None =>
            Configuration::default(),
    };
    Ok(configuration)
}

#[derive(Debug, PartialEq)]
struct Application {
    input: String,
    output: String,
    append: bool,
    configuration: Configuration,
}

impl Application {
    fn configure(arguments: Arguments) -> Result<Self> {
        let configuration = read_configuration(&arguments.config)?;

        Ok(
            Application {
                input: arguments.input,
                output: arguments.output,
                append: arguments.append,
                configuration,
            }
        )
    }

    fn run(self) -> Result<()> {
        let filter: EntryPredicate = (&self.configuration.output.content).into();
        let entries = self.create_entries()?
            .inspect(|entry| log::debug!("{:?}", entry))
            .filter(filter);
        self.write_entries(entries)?;

        Ok(())
    }

    fn create_entries(&self) -> Result<Box<dyn Iterator<Item=Entry>>> {
        let tool: Box<dyn Tool> = (&self.configuration.compilation).into();
        let from_events = entries_from_execution_events(self.input.as_str(), tool)?;
        // Based on the append flag, we should read the existing compilation database too.
        if self.append {
            let from_db = entries_from_compilation_db(Path::new(&self.output))?;
            Ok(Box::new(from_events.chain(from_db)))
        } else {
            Ok(Box::new(from_events))
        }
    }

    fn write_entries(&self, entries: impl Iterator<Item=Entry>) -> Result<(), anyhow::Error> {
        match self.output.as_str() {
            "-" | "/dev/stdout" => {
                let buffer = BufWriter::new(stdout());
                json_compilation_db::write(buffer, entries)?
            }
            output => {
                let temp = format!("{}.tmp", output);
                // Create scope for the file, so it will be closed when the scope is over.
                {
                    let file = File::create(&temp)
                        .with_context(|| format!("Failed to create file: {}", temp))?;
                    let buffer = BufWriter::new(file);
                    json_compilation_db::write(buffer, entries)?;
                }
                std::fs::rename(&temp, output)
                    .with_context(|| format!("Failed to rename file from '{}' to '{}'.", temp, output))?;
            }
        };

        Ok(())
    }
}

fn entries_from_execution_events(source: &str, tool: Box<dyn Tool>) -> Result<impl Iterator<Item=Entry>> {
    let reader: BufReader<Box<dyn Read>> = match source {
        "-" | "/dev/stdin" =>
            BufReader::new(Box::new(stdin())),
        _ => {
            let file = OpenOptions::new().read(true).open(source)
                .with_context(|| format!("Failed to open file: {}", source))?;
            BufReader::new(Box::new(file))
        }
    };
    let entries = events::from_reader(reader)
        .flat_map(failed_execution_read_logged)
        .flat_map(move |execution| execution_into_semantic(tool.as_ref(), execution))
        .flat_map(semantic_into_entries);

    Ok(entries)
}

fn failed_execution_read_logged(candidate: Result<Execution, Error>) -> Option<Execution> {
    match candidate {
        Ok(execution) => Some(execution),
        Err(error) => {
            log::error!("Failed to read entry: {}", error);
            None
        }
    }
}

fn execution_into_semantic(tool: &dyn Tool, execution: Execution) -> Option<Semantic> {
    match tool.recognize(&execution) {
        RecognitionResult::Recognized(Ok(Semantic::UnixCommand)) => {
            log::debug!("execution recognized as unix command: {:?}", execution);
            None
        }
        RecognitionResult::Recognized(Ok(Semantic::BuildCommand)) => {
            log::debug!("execution recognized as build command: {:?}", execution);
            None
        }
        RecognitionResult::Recognized(Ok(semantic)) => {
            log::debug!("execution recognized as compiler call, {:?} : {:?}", semantic, execution);
            Some(semantic)
        }
        RecognitionResult::Recognized(Err(reason)) => {
            log::debug!("execution recognized with failure, {:?} : {:?}", reason, execution);
            None
        }
        RecognitionResult::NotRecognized => {
            log::debug!("execution not recognized: {:?}", execution);
            None
        }
    }
}

fn semantic_into_entries(semantic: Semantic) -> Vec<Entry> {
    let entries: Result<Vec<Entry>, anyhow::Error> = semantic.try_into();
    entries.unwrap_or_else(|error| {
        log::debug!("compiler call failed to convert to compilation db entry: {}", error);
        vec![]
    })
}

fn entries_from_compilation_db(source: &Path) -> Result<impl Iterator<Item=Entry>> {
    let file = OpenOptions::new().read(true).open(source)
        .with_context(|| format!("Failed to open file: {:?}", source))?;
    let buffer = BufReader::new(file);
    let entries = json_compilation_db::read(buffer)
        .flat_map(failed_entry_read_logged);

    Ok(entries)
}

fn failed_entry_read_logged(candidate: Result<Entry, Error>) -> Option<Entry> {
    match candidate {
        Ok(entry) => Some(entry),
        Err(error) => {
            log::error!("Failed to read entry: {}", error);
            None
        }
    }
}
