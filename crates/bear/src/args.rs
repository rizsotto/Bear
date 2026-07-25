// SPDX-License-Identifier: GPL-3.0-or-later

//! This module contains the command line interface of the application.
//!
//! The command line parsing is implemented using the `clap` library.
//! The module defines types to represent a structured form of program invocation.
//! The `Arguments` type is used to represent all possible invocations of the program.

use clap::{ArgAction, ArgMatches, Command, arg, command};
use std::fmt;

/// Common constants used in the module.
const MODE_INTERCEPT_SUBCOMMAND: &str = "intercept";
const MODE_SEMANTIC_SUBCOMMAND: &str = "semantic";
const MODE_PARSE_SH_SUBCOMMAND: &str = "parse-sh";
const DEFAULT_OUTPUT_FILE: &str = "compile_commands.json";
/// Default for `parse-sh`'s and `semantic`'s `--input`: the `-` sentinel,
/// meaning standard input, so these non-executing modes read a pipe with no
/// flags. Their output defaults to the compilation database name instead
/// (`-` names standard output explicitly). `intercept` has no event-file
/// default at all: it cannot fall back to standard output (the build owns
/// that stream), so its destination must be named explicitly (see
/// `docs/rationale/event-file-defaults.md`).
const DEFAULT_STDIO: &str = "-";

/// Returns true when `path` is the [`DEFAULT_STDIO`] sentinel meaning
/// standard input or standard output, depending on context.
pub(crate) fn is_stdio(path: &std::path::Path) -> bool {
    path.as_os_str() == DEFAULT_STDIO
}

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
    Intercept { input: BuildCommand, output: BuildEvents },
    Semantic { input: BuildEvents, output: BuildSemantic },
    Combined { input: BuildCommand, output: BuildSemantic },
    ParseSh { input: ShScript, output: BuildSemantic },
    PrintCompilers,
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

/// Represents the shell text input for the `parse-sh` mode.
#[derive(Debug, PartialEq)]
pub struct ShScript {
    /// The path of the shell text to parse (`-` reads from standard input).
    pub path: std::path::PathBuf,
    /// The initial working directory for the parsed commands. When unset, the
    /// process's current directory is used.
    pub directory: Option<std::path::PathBuf>,
}

impl fmt::Display for Arguments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Arguments:")?;
        match &self.config {
            Some(config) => writeln!(f, "Configuration File: {}", config)?,
            None => writeln!(f, "Configuration File: <default>")?,
        }
        write!(f, "Mode: {}", self.mode)
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::Intercept { input, output } => {
                writeln!(f, "Intercept")?;
                writeln!(f, "  Input: {}", input)?;
                write!(f, "  Output: {}", output)
            }
            Mode::Semantic { input, output } => {
                writeln!(f, "Semantic Analysis")?;
                writeln!(f, "  Input: {}", input)?;
                write!(f, "  Output: {}", output)
            }
            Mode::Combined { input, output } => {
                writeln!(f, "Combined (Intercept + Semantic Analysis)")?;
                writeln!(f, "  Input: {}", input)?;
                write!(f, "  Output: {}", output)
            }
            Mode::ParseSh { input, output } => {
                writeln!(f, "Parse Shell Text")?;
                writeln!(f, "  Input: {}", input)?;
                write!(f, "  Output: {}", output)
            }
            Mode::PrintCompilers => write!(f, "Print Compilers"),
        }
    }
}

impl fmt::Display for BuildCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Build Command: {}", self.arguments.join(" "))
    }
}

impl fmt::Display for BuildSemantic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Semantic Output: {} (append: {})", self.path.display(), self.append)
    }
}

impl fmt::Display for BuildEvents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Events Output: {}", self.path.display())
    }
}

impl fmt::Display for ShScript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.directory {
            Some(dir) => write!(f, "Shell Script: {} (directory: {})", self.path.display(), dir.display()),
            None => write!(f, "Shell Script: {} (directory: <current>)", self.path.display()),
        }
    }
}

impl TryFrom<ArgMatches> for Arguments {
    type Error = ParseError;

    fn try_from(matches: ArgMatches) -> Result<Self, Self::Error> {
        let config = matches.get_one::<String>("config").map(String::to_string);
        let mode = Mode::try_from(matches)?;

        // `--print-compilers` only lists the compilers Bear recognizes from
        // the static, built-in tables and consults no config, so accepting
        // `--config` would silently ignore it. Reject it instead.
        if config.is_some() && matches!(mode, Mode::PrintCompilers) {
            return Err(ParseError::ConfigNotApplicableToPrintCompilers);
        }

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
                    .expect("output is required");

                Ok(Mode::Intercept { input, output: BuildEvents { path } })
            }
            Some((MODE_SEMANTIC_SUBCOMMAND, semantic_matches)) => {
                if semantic_matches.get_flag("print-compilers") {
                    return Ok(Mode::PrintCompilers);
                }

                let path = semantic_matches
                    .get_one::<String>("input")
                    .map(std::path::PathBuf::from)
                    .expect("input is defaulted");

                let output = BuildSemantic::try_from(semantic_matches)?;
                Ok(Mode::Semantic { input: BuildEvents { path }, output })
            }
            Some((MODE_PARSE_SH_SUBCOMMAND, parse_sh_matches)) => {
                let input = parse_sh_matches
                    .get_one::<String>("input")
                    .map(std::path::PathBuf::from)
                    .expect("input is defaulted");
                let directory = parse_sh_matches.get_one::<String>("directory").map(std::path::PathBuf::from);
                let output = BuildSemantic::try_from(parse_sh_matches)?;

                Ok(Mode::ParseSh { input: ShScript { path: input, directory }, output })
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
        let arguments: Vec<_> =
            matches.get_many("BUILD_COMMAND").ok_or(ParseError::MissingBuildCommand)?.cloned().collect();

        // The arguments must not be empty, and that is enforced by the CLI definition.
        Ok(BuildCommand { arguments })
    }
}

impl TryFrom<&ArgMatches> for BuildSemantic {
    type Error = ParseError;

    fn try_from(matches: &ArgMatches) -> Result<Self, Self::Error> {
        let path =
            matches.get_one::<String>("output").map(std::path::PathBuf::from).expect("output is defaulted");
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
    #[error(
        "The --config option does not apply to semantic --print-compilers: it only lists the \
         compilers Bear recognizes from its built-in tables and consults no config"
    )]
    ConfigNotApplicableToPrintCompilers,
}

/// Represents the command line interface of the application.
///
/// This describes how the user can interact with the application.
/// The different modes of the application are represented as subcommands.
/// The application can be run in intercept mode, semantic mode, parse-sh
/// mode, or the default mode where both intercept and semantic are executed.
pub fn cli() -> Command {
    // The binary is `bear-driver` but users invoke it as `bear` via a
    // shell wrapper, so we hardcode the user-facing name instead of
    // letting `command!()` pick up CARGO_BIN_NAME.
    command!()
        .name("bear")
        .subcommand_required(false)
        .subcommand_negates_reqs(true)
        .subcommand_precedence_over_arg(true)
        .arg_required_else_help(true)
        .args(&[arg!(-c --config <FILE> "Path of the config file")])
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
                    arg!(-o --output <FILE> "Path of the event file to write").required(true),
                ])
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new(MODE_SEMANTIC_SUBCOMMAND)
                .about("detect semantics of command executions")
                .args(&[
                    arg!(-i --input <FILE> "Path of the event file to read")
                        .default_value(DEFAULT_STDIO)
                        .hide_default_value(false),
                    arg!(-o --output <FILE> "Path of the result file")
                        .default_value(DEFAULT_OUTPUT_FILE)
                        .hide_default_value(false),
                    arg!(-a --append "Append result to an existing output file").action(ArgAction::SetTrue),
                    arg!(--"print-compilers" "Print the compilers Bear recognizes and exit")
                        .action(ArgAction::SetTrue),
                ])
                .arg_required_else_help(false)
                .after_help(
                    "Reads the event stream that `bear intercept` (or a third-party \
                     producer) writes and produces the compilation database, e.g.:\n\n  \
                     bear semantic --input events.json",
                ),
        )
        .subcommand(
            Command::new(MODE_PARSE_SH_SUBCOMMAND)
                .about("produces the compilation database from build-system dry-run text (e.g. `make -n` output)")
                .args(&[
                    arg!(-i --input <FILE> "Path of the shell text to parse")
                        .default_value(DEFAULT_STDIO)
                        .hide_default_value(false),
                    arg!(-o --output <FILE> "Path of the result file")
                        .default_value(DEFAULT_OUTPUT_FILE)
                        .hide_default_value(false),
                    arg!(-a --append "Append result to an existing output file").action(ArgAction::SetTrue),
                    arg!(-C --directory <DIR> "Initial working directory for the parsed commands"),
                ])
                .arg_required_else_help(false)
                .after_help(
                    "Parses the text a build system prints in dry-run mode and writes the \
                     compilation database, without running a build, e.g.:\n\n  \
                     make -n | bear parse-sh",
                ),
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
        let execution =
            vec!["bear", "-c", "~/bear.yaml", "intercept", "-o", "custom.json", "--", "make", "all"];

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
                    output: BuildEvents { path: "custom.json".into() },
                },
            }
        );
    }

    // Requirements: interception-events-format
    //
    // Interception has no default event-file name: it cannot fall back to
    // standard output (the build owns that stream), so the destination
    // must be named and omitting it is a usage error.
    #[test]
    fn test_intercept_requires_output() {
        let execution = vec!["bear", "intercept", "--", "make", "all"];

        let sut = cli().try_get_matches_from(execution);

        let error = sut.expect_err("intercept without --output must be a usage error");
        assert_eq!(error.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn test_semantic_call() {
        let execution =
            vec!["bear", "-c", "~/bear.yaml", "semantic", "-i", "custom.json", "-o", "result.json", "-a"];

        let matches = cli().get_matches_from(execution);
        let arguments = Arguments::try_from(matches).unwrap();

        assert_eq!(
            arguments,
            Arguments {
                config: Some("~/bear.yaml".into()),
                mode: Mode::Semantic {
                    input: BuildEvents { path: "custom.json".into() },
                    output: BuildSemantic { path: "result.json".into(), append: true },
                },
            }
        );
    }

    // Requirements: interception-events-format
    //
    // `bear semantic` is a filter: with no input named it reads the event
    // stream from standard input, so a producer pipes into it flag-free.
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
                    input: BuildEvents { path: "-".into() },
                    output: BuildSemantic { path: "compile_commands.json".into(), append: false },
                },
            }
        );
    }

    // Requirements: interception-events-format
    #[test]
    fn test_semantic_input_stdin() {
        let execution = vec!["bear", "semantic", "-i", "-"];

        let matches = cli().get_matches_from(execution);
        let arguments = Arguments::try_from(matches).unwrap();

        assert_eq!(
            arguments,
            Arguments {
                config: None,
                mode: Mode::Semantic {
                    input: BuildEvents { path: "-".into() },
                    output: BuildSemantic { path: "compile_commands.json".into(), append: false },
                },
            }
        );
    }

    // Requirements: interception-events-format
    //
    // `-` parses successfully here: the CLI layer treats it as an ordinary
    // path value. The rejection (a mode that runs the build must not accept
    // `-` for output) happens at `Mode::configure` time, not at parse time,
    // and is covered by the integration tests for that requirement.
    #[test]
    fn test_intercept_output_dash_parses() {
        let execution = vec!["bear", "intercept", "-o", "-", "--", "make", "all"];

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
                    output: BuildEvents { path: "-".into() },
                },
            }
        );
    }

    // Requirements: interception-shell-text-parsing
    //
    // `parse-sh` reads shell text from standard input and writes the
    // compilation database under its ecosystem-contracted name, so the
    // headline usage is flag-free: `make -n | bear parse-sh`.
    #[test]
    fn test_parse_sh_defaults() {
        let execution = vec!["bear", "parse-sh"];

        let matches = cli().get_matches_from(execution);
        let arguments = Arguments::try_from(matches).unwrap();

        assert_eq!(
            arguments,
            Arguments {
                config: None,
                mode: Mode::ParseSh {
                    input: ShScript { path: "-".into(), directory: None },
                    output: BuildSemantic { path: "compile_commands.json".into(), append: false },
                },
            }
        );
    }

    // Requirements: interception-shell-text-parsing
    #[test]
    fn test_parse_sh_call() {
        let execution = vec!["bear", "parse-sh", "-i", "in.sh", "-o", "db.json", "-a", "-C", "/build"];

        let matches = cli().get_matches_from(execution);
        let arguments = Arguments::try_from(matches).unwrap();

        assert_eq!(
            arguments,
            Arguments {
                config: None,
                mode: Mode::ParseSh {
                    input: ShScript { path: "in.sh".into(), directory: Some("/build".into()) },
                    output: BuildSemantic { path: "db.json".into(), append: true },
                },
            }
        );
    }

    // Requirements: interception-shell-text-parsing
    //
    // Configuration shapes the semantic analysis parse-sh runs, exactly as
    // in the other database-producing modes, so `--config` is accepted.
    #[test]
    fn test_parse_sh_accepts_config() {
        let execution = vec!["bear", "-c", "~/bear.yaml", "parse-sh"];

        let matches = cli().get_matches_from(execution);
        let arguments = Arguments::try_from(matches).unwrap();

        assert_eq!(
            arguments,
            Arguments {
                config: Some("~/bear.yaml".into()),
                mode: Mode::ParseSh {
                    input: ShScript { path: "-".into(), directory: None },
                    output: BuildSemantic { path: "compile_commands.json".into(), append: false },
                },
            }
        );
    }

    // Requirements: recognition-compiler-names
    #[test]
    fn test_semantic_print_compilers() {
        let execution = vec!["bear", "semantic", "--print-compilers"];

        let matches = cli().get_matches_from(execution);
        let arguments = Arguments::try_from(matches).unwrap();

        assert_eq!(arguments, Arguments { config: None, mode: Mode::PrintCompilers });
    }

    // Requirements: recognition-compiler-names
    #[test]
    fn test_semantic_print_compilers_rejects_config() {
        // arrange
        let execution = vec!["bear", "-c", "~/bear.yaml", "semantic", "--print-compilers"];

        // act
        let matches = cli().get_matches_from(execution);
        let sut = Arguments::try_from(matches);

        // assert
        assert!(matches!(sut, Err(ParseError::ConfigNotApplicableToPrintCompilers)));
    }

    #[test]
    fn test_all_call() {
        let execution = vec!["bear", "-c", "~/bear.yaml", "-o", "result.json", "-a", "--", "make", "all"];

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
                    output: BuildSemantic { path: "result.json".into(), append: true },
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
                    output: BuildSemantic { path: "compile_commands.json".into(), append: false },
                },
            }
        );
    }
}
