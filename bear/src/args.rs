// SPDX-License-Identifier: GPL-3.0-or-later

//! This module contains the command line interface of the application.
//!
//! The command line parsing is implemented using the `clap` library.
//! The module defines types to represent a structured form of program invocation.
//! The `Arguments` type is used to represent all possible invocations of the program.

use clap::{arg, command, ArgAction, ArgMatches, Command};

/// Common constants used in the module.
const MODE_INTERCEPT_SUBCOMMAND: &str = "intercept";
const MODE_SEMANTIC_SUBCOMMAND: &str = "semantic";
const DEFAULT_OUTPUT_FILE: &str = "compile_commands.json";
const DEFAULT_EVENT_FILE: &str = "events.json";

/// Represents the command line arguments of the application.
#[derive(Debug, PartialEq)]
pub struct Arguments {
    /// The path of the configuration file.
    pub config: Option<String>,
    /// The mode of the application.
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
    Combined {
        input: BuildCommand,
        output: BuildSemantic,
    },
}

/// Represents the execution of a command.
#[derive(Debug, PartialEq)]
pub struct BuildCommand {
    /// The command arguments to execute. (This is a non-empty vector of strings.)
    pub arguments: Vec<String>,
}

/// Represents the semantic output configuration.
#[derive(Debug, PartialEq)]
pub struct BuildSemantic {
    /// The output file path.
    pub path: std::path::PathBuf,
    /// Whether to append to an existing file.
    pub append: bool,
}

/// Represents the build events configuration.
#[derive(Debug, PartialEq)]
pub struct BuildEvents {
    /// The path to the events file.
    pub path: std::path::PathBuf,
}

impl TryFrom<ArgMatches> for Arguments {
    type Error = ParseError;

    fn try_from(matches: ArgMatches) -> Result<Self, Self::Error> {
        let config = matches.get_one::<String>("config").map(String::to_string);
        let mode = Mode::try_from(matches)?;

        Ok(Arguments { config, mode })
    }
}

impl TryFrom<ArgMatches> for Mode {
    type Error = ParseError;

    fn try_from(matches: ArgMatches) -> Result<Self, Self::Error> {
        match matches.subcommand() {
            Some((MODE_INTERCEPT_SUBCOMMAND, intercept_matches)) => {
                let input = BuildCommand::try_from(intercept_matches)?;
                let path = intercept_matches
                    .get_one::<String>("output")
                    .map(std::path::PathBuf::from)
                    .expect("output is defaulted");

                Ok(Mode::Intercept {
                    input,
                    output: BuildEvents { path },
                })
            }
            Some((MODE_SEMANTIC_SUBCOMMAND, semantic_matches)) => {
                let path = semantic_matches
                    .get_one::<String>("input")
                    .map(std::path::PathBuf::from)
                    .expect("input is defaulted");

                let output = BuildSemantic::try_from(semantic_matches)?;
                Ok(Mode::Semantic {
                    input: BuildEvents { path },
                    output,
                })
            }
            None => {
                let input = BuildCommand::try_from(&matches)?;
                let output = BuildSemantic::try_from(&matches)?;
                Ok(Mode::Combined { input, output })
            }
            _ => Err(ParseError::UnrecognizedSubcommand),
        }
    }
}

impl TryFrom<&ArgMatches> for BuildCommand {
    type Error = ParseError;

    fn try_from(matches: &ArgMatches) -> Result<Self, Self::Error> {
        let arguments: Vec<_> = matches
            .get_many("BUILD_COMMAND")
            .ok_or(ParseError::MissingBuildCommand)?
            .cloned()
            .collect();

        // The arguments must not be empty, and that is enforced by the CLI definition.
        Ok(BuildCommand { arguments })
    }
}

impl TryFrom<&ArgMatches> for BuildSemantic {
    type Error = ParseError;

    fn try_from(matches: &ArgMatches) -> Result<Self, Self::Error> {
        let path = matches
            .get_one::<String>("output")
            .map(std::path::PathBuf::from)
            .expect("output is defaulted");
        let append = *matches.get_one::<bool>("append").unwrap_or(&false);
        Ok(BuildSemantic { path, append })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Unrecognized subcommand")]
    UnrecognizedSubcommand,
    #[error("Missing build command")]
    MissingBuildCommand,
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
                    arg!(<BUILD_COMMAND> "Build command")
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
            arg!(<BUILD_COMMAND> "Build command")
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
                config: Some("~/bear.yaml".into()),
                mode: Mode::Intercept {
                    input: BuildCommand {
                        arguments: vec!["make", "all"].into_iter().map(String::from).collect()
                    },
                    output: BuildEvents {
                        path: "custom.json".into()
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
                        arguments: vec!["make", "all"].into_iter().map(String::from).collect()
                    },
                    output: BuildEvents {
                        path: "events.json".into()
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
                config: Some("~/bear.yaml".into()),
                mode: Mode::Semantic {
                    input: BuildEvents {
                        path: "custom.json".into()
                    },
                    output: BuildSemantic {
                        path: "result.json".into(),
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
                        path: "events.json".into()
                    },
                    output: BuildSemantic {
                        path: "compile_commands.json".into(),
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
                mode: Mode::Combined {
                    input: BuildCommand {
                        arguments: vec!["make", "all"].into_iter().map(String::from).collect()
                    },
                    output: BuildSemantic {
                        path: "result.json".into(),
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
                mode: Mode::Combined {
                    input: BuildCommand {
                        arguments: vec!["make", "all"].into_iter().map(String::from).collect(),
                    },
                    output: BuildSemantic {
                        path: "compile_commands.json".into(),
                        append: false
                    },
                },
            }
        );
    }
}
