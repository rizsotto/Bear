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

//! This module contains the command line interface of the application.
//!
//! The command line parsing is implemented using the `clap` library.
//! The module is defining types to represent a structured form of the
//! program invocation. The `Arguments` type is used to represent all
//! possible invocations of the program.

use anyhow::anyhow;
use clap::{arg, command, ArgAction, ArgMatches, Command};

/// Common constants used in the module.
const MODE_INTERCEPT_SUBCOMMAND: &str = "intercept";
const MODE_SEMANTIC_SUBCOMMAND: &str = "semantic";
const DEFAULT_OUTPUT_FILE: &str = "compile_commands.json";
const DEFAULT_EVENT_FILE: &str = "events.json";

/// Represents the command line arguments of the application.
#[derive(Debug, PartialEq)]
pub struct Arguments {
    // The path of the configuration file.
    pub config: Option<String>,
    // The mode of the application.
    pub mode: Mode,
}

/// Represents the mode of the application.
#[derive(Debug, PartialEq)]
pub enum Mode {
    Intercept {
        input: BuildCommand,
        output: BuildEvents,
    },
    Semantic {
        input: BuildEvents,
        output: BuildSemantic,
    },
    All {
        input: BuildCommand,
        output: BuildSemantic,
    },
}

/// Represents the execution of a command.
#[derive(Debug, PartialEq)]
pub struct BuildCommand {
    arguments: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct BuildSemantic {
    pub file_name: String,
    pub append: bool,
}

#[derive(Debug, PartialEq)]
pub struct BuildEvents {
    pub file_name: String,
}

impl TryFrom<ArgMatches> for Arguments {
    type Error = anyhow::Error;

    fn try_from(matches: ArgMatches) -> Result<Self, Self::Error> {
        let config = matches.get_one::<String>("config").map(String::to_string);

        match matches.subcommand() {
            Some((MODE_INTERCEPT_SUBCOMMAND, intercept_matches)) => {
                let input = BuildCommand::try_from(intercept_matches)?;
                let output = intercept_matches
                    .get_one::<String>("output")
                    .map(String::to_string)
                    .expect("output is defaulted");

                // let output = BuildEvents::try_from(intercept_matches)?;
                let mode = Mode::Intercept {
                    input,
                    output: BuildEvents { file_name: output },
                };
                let arguments = Arguments { config, mode };
                Ok(arguments)
            }
            Some((MODE_SEMANTIC_SUBCOMMAND, semantic_matches)) => {
                let input = semantic_matches
                    .get_one::<String>("input")
                    .map(String::to_string)
                    .expect("input is defaulted");

                let output = BuildSemantic::try_from(semantic_matches)?;
                let mode = Mode::Semantic {
                    input: BuildEvents { file_name: input },
                    output,
                };
                let arguments = Arguments { config, mode };
                Ok(arguments)
            }
            None => {
                let input = BuildCommand::try_from(&matches)?;
                let output = BuildSemantic::try_from(&matches)?;
                let mode = Mode::All { input, output };
                let arguments = Arguments { config, mode };
                Ok(arguments)
            }
            _ => Err(anyhow!("unrecognized subcommand")),
        }
    }
}

impl TryFrom<&ArgMatches> for BuildCommand {
    type Error = anyhow::Error;

    fn try_from(matches: &ArgMatches) -> Result<Self, Self::Error> {
        let arguments = matches
            .get_many("COMMAND")
            .expect("missing build command")
            .cloned()
            .collect();
        Ok(BuildCommand { arguments })
    }
}

impl TryFrom<&ArgMatches> for BuildSemantic {
    type Error = anyhow::Error;

    fn try_from(matches: &ArgMatches) -> Result<Self, Self::Error> {
        let file_name = matches
            .get_one::<String>("output")
            .map(String::to_string)
            .expect("output is defaulted");
        let append = *matches.get_one::<bool>("append").unwrap_or(&false);
        Ok(BuildSemantic { file_name, append })
    }
}

/// Represents the command line interface of the application.
///
/// This describes how the user can interact with the application.
/// The different modes of the application are represented as subcommands.
/// The application can be run in intercept mode, semantic mode, or the
/// default mode where both intercept and semantic are executed.
pub fn cli() -> Command {
    command!()
        .subcommand_required(false)
        .subcommand_negates_reqs(true)
        .subcommand_precedence_over_arg(true)
        .arg_required_else_help(true)
        .args(&[
            arg!(-v --verbose ... "Sets the level of verbosity").action(ArgAction::Count),
            arg!(-c --config <FILE> "Path of the config file"),
        ])
        .subcommand(
            Command::new(MODE_INTERCEPT_SUBCOMMAND)
                .about("intercepts command execution")
                .args(&[
                    arg!(<COMMAND> "Build command")
                        .action(ArgAction::Append)
                        .value_terminator("--")
                        .num_args(1..)
                        .last(true)
                        .required(true),
                    arg!(-o --output <FILE> "Path of the event file")
                        .default_value(DEFAULT_EVENT_FILE)
                        .hide_default_value(false),
                ])
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new(MODE_SEMANTIC_SUBCOMMAND)
                .about("detect semantics of command executions")
                .args(&[
                    arg!(-i --input <FILE> "Path of the event file")
                        .default_value(DEFAULT_EVENT_FILE)
                        .hide_default_value(false),
                    arg!(-o --output <FILE> "Path of the result file")
                        .default_value(DEFAULT_OUTPUT_FILE)
                        .hide_default_value(false),
                    arg!(-a --append "Append result to an existing output file")
                        .action(ArgAction::SetTrue),
                ])
                .arg_required_else_help(false),
        )
        .args(&[
            arg!(<COMMAND> "Build command")
                .action(ArgAction::Append)
                .value_terminator("--")
                .num_args(1..)
                .last(true)
                .required(true),
            arg!(-o --output <FILE> "Path of the result file")
                .default_value(DEFAULT_OUTPUT_FILE)
                .hide_default_value(false),
            arg!(-a --append "Append result to an existing output file").action(ArgAction::SetTrue),
        ])
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::vec_of_strings;

    #[test]
    fn test_intercept_call() {
        let execution = vec![
            "bear",
            "-c",
            "~/bear.yaml",
            "intercept",
            "-o",
            "custom.json",
            "--",
            "make",
            "all",
        ];

        let matches = cli().get_matches_from(execution);
        let arguments = Arguments::try_from(matches).unwrap();

        assert_eq!(
            arguments,
            Arguments {
                config: Some("~/bear.yaml".to_string()),
                mode: Mode::Intercept {
                    input: BuildCommand {
                        arguments: vec_of_strings!["make", "all"]
                    },
                    output: BuildEvents {
                        file_name: "custom.json".to_string()
                    },
                },
            }
        );
    }

    #[test]
    fn test_intercept_defaults() {
        let execution = vec!["bear", "intercept", "--", "make", "all"];

        let matches = cli().get_matches_from(execution);
        let arguments = Arguments::try_from(matches).unwrap();

        assert_eq!(
            arguments,
            Arguments {
                config: None,
                mode: Mode::Intercept {
                    input: BuildCommand {
                        arguments: vec_of_strings!["make", "all"]
                    },
                    output: BuildEvents {
                        file_name: "events.json".to_string()
                    },
                },
            }
        );
    }

    #[test]
    fn test_semantic_call() {
        let execution = vec![
            "bear",
            "-c",
            "~/bear.yaml",
            "semantic",
            "-i",
            "custom.json",
            "-o",
            "result.json",
            "-a",
        ];

        let matches = cli().get_matches_from(execution);
        let arguments = Arguments::try_from(matches).unwrap();

        assert_eq!(
            arguments,
            Arguments {
                config: Some("~/bear.yaml".to_string()),
                mode: Mode::Semantic {
                    input: BuildEvents {
                        file_name: "custom.json".to_string()
                    },
                    output: BuildSemantic {
                        file_name: "result.json".to_string(),
                        append: true
                    },
                },
            }
        );
    }

    #[test]
    fn test_semantic_defaults() {
        let execution = vec!["bear", "semantic"];

        let matches = cli().get_matches_from(execution);
        let arguments = Arguments::try_from(matches).unwrap();

        assert_eq!(
            arguments,
            Arguments {
                config: None,
                mode: Mode::Semantic {
                    input: BuildEvents {
                        file_name: "events.json".to_string()
                    },
                    output: BuildSemantic {
                        file_name: "compile_commands.json".to_string(),
                        append: false
                    },
                },
            }
        );
    }

    #[test]
    fn test_all_call() {
        let execution = vec![
            "bear",
            "-c",
            "~/bear.yaml",
            "-o",
            "result.json",
            "-a",
            "--",
            "make",
            "all",
        ];

        let matches = cli().get_matches_from(execution);
        let arguments = Arguments::try_from(matches).unwrap();

        assert_eq!(
            arguments,
            Arguments {
                config: Some("~/bear.yaml".to_string()),
                mode: Mode::All {
                    input: BuildCommand {
                        arguments: vec_of_strings!["make", "all"]
                    },
                    output: BuildSemantic {
                        file_name: "result.json".to_string(),
                        append: true
                    },
                },
            }
        );
    }

    #[test]
    fn test_all_defaults() {
        let execution = vec!["bear", "--", "make", "all"];

        let matches = cli().get_matches_from(execution);
        let arguments = Arguments::try_from(matches).unwrap();

        assert_eq!(
            arguments,
            Arguments {
                config: None,
                mode: Mode::All {
                    input: BuildCommand {
                        arguments: vec_of_strings!["make", "all"]
                    },
                    output: BuildSemantic {
                        file_name: "compile_commands.json".to_string(),
                        append: false
                    },
                },
            }
        );
    }
}
