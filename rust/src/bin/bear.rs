/*  Copyright (C) 2012-2018 by László Nagy
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

#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate error_chain;
extern crate intercept;
#[macro_use]
extern crate log;

use std::env;
use std::path;
use std::process;

use intercept::{Result, ResultExt};
use intercept::database;
use intercept::event::ExitCode;
use clap::ArgMatches;
use ::Command::Supervise;


fn main() {
    match run() {
        Ok(code) => {
            process::exit(code);
        },
        Err(ref e) => {
            eprintln!("{}", e);

            for e in e.iter().skip(1) {
                eprintln!("caused by: {}", e);
            }

            // The backtrace is not always generated. Try to run this with `RUST_BACKTRACE=1`.
            if let Some(backtrace) = e.backtrace() {
                eprintln!("backtrace: {:?}", backtrace);
            }

            ::std::process::exit(1);
        },
    }
}

fn run() -> Result<ExitCode> {
    drop(env_logger::init());
    info!("bear - {} {}", crate_name!(), crate_version!());

    let args = env::args().collect::<Vec<_>>();
    debug!("invocation: {:?}", &args);

    parse_arguments(args.as_slice())
        .and_then(|command| command.run())
}

fn parse_arguments(args: &[String]) -> Result<Command> {
    let matches = clap::App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .settings(&[
            clap::AppSettings::SubcommandRequired,
            clap::AppSettings::GlobalVersion
        ])
        .subcommand(supervise_command())
        .subcommand(clap::SubCommand::with_name("intercept"))
        .subcommand(clap::SubCommand::with_name("transform"))
        .get_matches_from_safe(args)?;

    build_command(matches)
        .chain_err(|| "")
}

fn build_command(matches: ArgMatches) -> Result<Command> {
    debug!("{:?}", matches);
    match matches.subcommand() {
        ("supervise", Some(sub_matches)) =>
            build_supervise_command(sub_matches),
        ("intercept", Some(_sub_matches)) => {
            unimplemented!()
        },
        ("transform", Some(_sub_matches)) => {
            unimplemented!()
        },
        _ => unimplemented!()
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Supervise {
        session: Session,
        execution: Execution,
    },
    CompilationDatabaseBuild {
//        config: database::config::Config,
//        target: Box<database::CompilationDatabase>,
        command: Vec<String>,
    },
    CompilationDatabaseTransform,
    OntologyBuild,
    OntologyEnrich,
}

#[derive(Debug, PartialEq, Eq)]
struct Session {
    destination: path::PathBuf,
    library: path::PathBuf,
    verbose: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct Execution {
    program: ExecutionTarget,
    arguments: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum ExecutionTarget {
    File(path::PathBuf),
    Path(String),
    WithSearchPath(String, Vec<path::PathBuf>),
}

impl Command {

//    pub fn parse(matches: ArgMatches) -> Result<Command> {
//        unimplemented!()
//    }

    pub fn run(self) -> Result<ExitCode> {

//        let config = Config::default();
//        let target =
//            JsonCompilationDatabase::new(
//                path::Path::new("./compile_commands.json"));
//        let builder = Builder::new(&config, &target);
//
//        intercept_build(&builder, command.as_ref())
        unimplemented!()
    }
}


//fn intercept_build(builder: &Builder, command: &[String]) -> Result<ExitCode> {
//    let collector = protocol::collector::Protocol::new()
//        .chain_err(|| "Failed to set up event collection.")?;
//
//    let exit = run_build(command, collector.path())
//        .chain_err(|| "Failed to run the build.")?;
//
//    builder.build(collector.events())
//        .chain_err(|| "Failed to write output.")?;
//
//    Ok(exit)
//}
//
//fn run_build(command: &[String], destination: &path::Path) -> Result<ExitCode> {
//    env::set_var(KEY_DESTINATION, destination);
//
//    let mut sender = protocol::sender::Protocol::new(destination)?;
//    let mut build = Supervisor::new(|event| sender.send(event));
//    let exit = build.run(command)?;
//    info!("Build finished with status code: {}", exit);
//    Ok(exit)
//}

fn supervise_command<'a, 'b>() -> clap::App<'a, 'b> {
    clap::SubCommand::with_name("supervise")
        // TODO: make verbose a top level param
        .arg(clap::Arg::with_name("verbose")
            .long("session-verbose")
            .takes_value(false))
        .arg(clap::Arg::with_name("destination")
            .long("session-destination")
            .takes_value(true)
            .required(true))
        .arg(clap::Arg::with_name("library")
            .long("session-library")
            .takes_value(true)
            .required(true))
        .arg(clap::Arg::with_name("path")
            .long("execution-path")
            .takes_value(true))
        .arg(clap::Arg::with_name("file")
            .long("execution-file")
            .takes_value(true))
        .arg(clap::Arg::with_name("search-path")
            .long("execution-search-path")
            .takes_value(true))
        .arg(clap::Arg::with_name("command")
            .multiple(true)
            .allow_hyphen_values(true)
            .required(true)
            .last(true))
        .group(clap::ArgGroup::with_name("session")
            .multiple(true)
            .args(&["verbose", "destination", "library"]))
        .group(clap::ArgGroup::with_name("execution")
            .multiple(true)
            .args(&["path", "file", "search-path", "command"]))
        .group(clap::ArgGroup::with_name("execution-mode")
            .multiple(false)
            .required(true)
            .args(&["path", "file"]))
        .settings(&[
            clap::AppSettings::TrailingVarArg,
            clap::AppSettings::Hidden,
        ])
}

fn build_supervise_command(matches: &ArgMatches) -> Result<Command> {
    let destination = matches.value_of("destination").unwrap();
    let library = matches.value_of("library").unwrap();
    let verbose = matches.is_present("verbose");
    let session = Session {
        destination: path::PathBuf::from(destination),
        library: path::PathBuf::from(library),
        verbose,
    };

    let program: Result<ExecutionTarget> =
        match (matches.value_of("search-path"),
               matches.value_of("file"),
               matches.value_of("path")) {
        (Some(sp), _, Some(path)) => {
            let paths = sp.split(":").map(|p| path::PathBuf::from(p)).collect::<Vec<_>>();
            Ok(ExecutionTarget::WithSearchPath(path.to_string(), paths))
        },
        (None, None, Some(path)) =>
            Ok(ExecutionTarget::Path(path.to_string())),
        (None, Some(file), None) =>
            Ok(ExecutionTarget::File(path::PathBuf::from(file))),
        _ =>
            Err(matches.usage().into())
    };
    let command = matches.values_of("command").unwrap();
    let execution = Execution {
        program: program?,
        arguments: command.map(|str| str.to_string()).collect::<Vec<_>>(),
    };

    Ok(Supervise { session, execution, })
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! vec_of_strings {
        ($($x:expr),*) => (vec![$($x.to_string()),*]);
    }

    mod supervise_command {
        use super::*;

        #[test]
        #[should_panic]
        fn missing_destination() {
            let arguments = vec_of_strings!(
                "bear",
                "supervise",
                "--session-library", "/usr/local/lib/libear.so",
                "--execution-path", "cc",
                "--", "cc", "-c", "source.c");
            let _ = parse_arguments(arguments.as_slice()).unwrap();
        }

        #[test]
        #[should_panic]
        fn missing_libray() {
            let arguments = vec_of_strings!(
                "bear",
                "supervise",
                "--session-destination", "/tmp/bear",
                "--execution-path", "cc",
                "--", "cc", "-c", "source.c");
            let _ = parse_arguments(arguments.as_slice()).unwrap();
        }

        #[test]
        #[should_panic]
        fn missing_command() {
            let arguments = vec_of_strings!(
                "bear",
                "supervise",
                "--session-destination", "/tmp/bear",
                "--session-library", "/usr/local/lib/libear.so",
                "--execution-path", "cc");
            let _ = parse_arguments(arguments.as_slice()).unwrap();
        }

        #[test]
        #[should_panic]
        fn conflict_file_and_path() {
            let arguments = vec_of_strings!(
                "bear",
                "supervise",
                "--session-destination", "/tmp/bear",
                "--session-library", "/usr/local/lib/libear.so",
                "--execution-file", "/usr/bin/cc",
                "--execution-path", "cc",
                "--", "cc", "-c", "source.c");
            let _ = parse_arguments(arguments.as_slice()).unwrap();
        }

        #[test]
        #[should_panic]
        fn conflict_file_and_search_path() {
            let arguments = vec_of_strings!(
                "bear",
                "supervise",
                "--session-destination", "/tmp/bear",
                "--session-library", "/usr/local/lib/libear.so",
                "--execution-file", "/usr/bin/cc",
                "--execution-search-path", "/usr/bin:/usr/local/bin",
                "--", "cc", "-c", "source.c");
            let _ = parse_arguments(arguments.as_slice()).unwrap();
        }

        #[test]
        fn parsed_with_path() {
            let arguments = vec_of_strings!(
                "bear",
                "supervise",
                "--session-destination", "/tmp/bear",
                "--session-library", "/usr/local/lib/libear.so",
                "--execution-path", "cc",
                "--", "cc", "-c", "source.c");
            let command = parse_arguments(arguments.as_slice()).unwrap();

            let expected_command = Supervise {
                session: Session {
                    destination: path::PathBuf::from("/tmp/bear"),
                    library: path::PathBuf::from("/usr/local/lib/libear.so"),
                    verbose: false,
                },
                execution: Execution {
                    program: ExecutionTarget::Path("cc".to_string()),
                    arguments: vec_of_strings!("cc", "-c", "source.c"),
                }
            };
            assert_eq!(expected_command, command);
        }

        #[test]
        fn parsed_with_file() {
            let arguments = vec_of_strings!(
                "bear",
                "supervise",
                "--session-destination", "/tmp/bear",
                "--session-library", "/usr/local/lib/libear.so",
                "--execution-file", "/usr/bin/cc",
                "--", "cc", "-c", "source.c");
            let command = parse_arguments(arguments.as_slice()).unwrap();

            let expected_command = Supervise {
                session: Session {
                    destination: path::PathBuf::from("/tmp/bear"),
                    library: path::PathBuf::from("/usr/local/lib/libear.so"),
                    verbose: false,
                },
                execution: Execution {
                    program: ExecutionTarget::File(path::PathBuf::from("/usr/bin/cc")),
                    arguments: vec_of_strings!("cc", "-c", "source.c"),
                }
            };
            assert_eq!(expected_command, command);
        }
    }
}
