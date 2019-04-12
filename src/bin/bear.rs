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
extern crate directories;
extern crate env_logger;
#[macro_use]
extern crate error_chain;
extern crate ear;
#[macro_use]
extern crate log;

use std::env;
use std::path;
use std::process;

use ear::command::Command;
use ear::intercept::{ExecutionRequest, Executable, InterceptMode, InterceptModes, Session};
use ear::intercept::ExitCode;
use clap::ArgMatches;

use error::{Result, ResultExt};
use ear::intercept::report::wrapper;

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
    env_logger::init();
    info!("bear - {} {}", crate_name!(), crate_version!());

    let args = env::args().collect::<Vec<_>>();
    debug!("invocation: {:?}", &args);

    parse_arguments(args.as_slice())
        .and_then(|command| command.run().map_err(|err| err.into()))
}

fn parse_arguments(args: &[String]) -> Result<Command> {
    let program = path::PathBuf::from(&args[0]);
    let default_config = default_config_file();
    let matches = clap::App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .subcommand(parse_supervise())
        .subcommand(parse_configure())
        .subcommand(parse_build(default_config.as_str()))
        .subcommand(parse_intercept())
        .settings(&[
            clap::AppSettings::GlobalVersion,
            clap::AppSettings::SubcommandRequired,
            clap::AppSettings::DisableHelpSubcommand,
        ])
        .get_matches_from_safe(args)?;

    build_command(matches, program.as_ref())
        .chain_err(|| "")
}

fn build_command(matches: ArgMatches, program: &path::Path) -> Result<Command> {
    debug!("{:?}", matches);
    match matches.subcommand() {
        ("supervise", Some(sub_matches)) =>
            build_command_supervise(sub_matches, program),
        ("configure", Some(sub_matches)) =>
            build_command_configure(sub_matches, program),
        ("build", Some(sub_matches)) =>
            build_command_build(sub_matches, program),
        ("intercept", Some(sub_matches)) =>
            build_command_intercept(sub_matches, program),
        _ =>
            Err(matches.usage().into()),
    }
}


fn parse_supervise<'a, 'b>() -> clap::App<'a, 'b> {
    clap::SubCommand::with_name("supervise")
        .arg(clap::Arg::with_name("verbose")
            .long("session-verbose")
            .takes_value(false))
        .arg(clap::Arg::with_name("destination")
            .long("session-destination")
            .value_name("URL")
            .required(true))
        .arg(clap::Arg::with_name("library")
            .long("session-library")
            .value_name("PATH")
            .required(true))
        .arg(clap::Arg::with_name("path")
            .long("exec-path")
            .value_name("FILE"))
        .arg(clap::Arg::with_name("file")
            .long("exec-file")
            .value_name("FILE"))
        .arg(clap::Arg::with_name("search-path")
            .long("exec-searchpath")
            .value_name("PATH"))
        .arg(arg_command())
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

fn arg_command<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("command")
        .multiple(true)
        .allow_hyphen_values(true)
        .required(true)
        .last(true)
}

fn build_command_supervise(matches: &ArgMatches, program: &path::Path) -> Result<Command> {
    let mode = InterceptMode::WrapperPreload {
        library: value_t!(matches, "library", path::PathBuf).unwrap(),
        wrapper: program.to_path_buf(),
    };
    let session = Session {
        destination: value_t!(matches, "destination", path::PathBuf).unwrap(),
        verbose: matches.is_present("verbose"),
        modes: vec!(mode),
    };
    let execution = ExecutionRequest {
        executable: build_execution_target(matches)?,
        arguments: values_t!(matches, "command", String)?,
    };

    Ok(Command::Supervise { session, execution, })
}

fn build_execution_target(matches: &ArgMatches) -> Result<Executable> {
    match (matches.value_of("search-path"),
           matches.value_of("file"),
           matches.value_of("path")) {
        (Some(sp), _, Some(path)) => {
            let paths = sp.split(':').map(path::PathBuf::from).collect::<Vec<_>>();
            Ok(Executable::WithSearchPath(path.to_string(), paths))
        },
        (None, None, Some(path)) =>
            Ok(Executable::WithPath(path.to_string())),
        (None, Some(file), None) =>
            Ok(Executable::WithFilename(path::PathBuf::from(file))),
        _ =>
            Err(matches.usage().into())
    }
}

fn parse_configure<'a, 'b>() -> clap::App<'a, 'b> {
    clap::SubCommand::with_name("configure")
        .args(args_intercept_modes().as_ref())
        .arg(arg_command())
        .settings(&[
            clap::AppSettings::TrailingVarArg,
        ])
}

fn args_intercept_modes<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
    vec!(
        clap::Arg::with_name("library")
            .long("library")
            .value_name("PATH")
            .display_order(50),
        clap::Arg::with_name("wrapper_cc")
            .long("cc-wrapper")
            .value_names(&["COMPILER", "WRAPPER"])
            .display_order(50),
        clap::Arg::with_name("wrapper_cxx")
            .long("cxx-wrapper")
            .value_names(&["COMPILER", "WRAPPER"])
            .display_order(50),
    )
}

fn build_command_configure(matches: &ArgMatches, program: &path::Path) -> Result<Command> {
    let modes = build_intercept_modes(matches, program)?;
    let command = values_t!(matches, "command", String)?;
    Ok(Command::InjectWrappers { modes, command })
}

fn build_intercept_modes(matches: &ArgMatches, program: &path::Path) -> Result<InterceptModes> {
    let mut modes: InterceptModes = vec!();
    if let Ok(library) = value_t!(matches, "library", path::PathBuf) {
        let wrapper = program.to_path_buf();
        modes.push(InterceptMode::WrapperPreload {
            library, wrapper,
        });
    }
    if let Ok(wrapper) = values_t!(matches, "wrapper_cc", String) {
        modes.push(InterceptMode::WrapperCC {
            compiler: path::PathBuf::from(&wrapper[0]),
            wrapper: path::PathBuf::from(&wrapper[1]) })
    }
    if let Ok(wrapper) = values_t!(matches, "wrapper_cxx", String) {
        modes.push(InterceptMode::WrapperCXX {
            compiler: path::PathBuf::from(&wrapper[0]),
            wrapper: path::PathBuf::from(&wrapper[1]) })
    }
    Ok(modes)
}

fn parse_build<'a, 'b>(default_config: &'a str) -> clap::App<'a, 'b> {
    clap::SubCommand::with_name("build")
        .arg(clap::Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .default_value(default_config)
            .display_order(10))
        .arg(clap::Arg::with_name("output")
            .short("o")
            .long("output")
            .value_name("FILE")
            .default_value("compile_commands.json")
            .display_order(10))
        .args(args_intercept_modes().as_ref())
        .arg(arg_command())
        .settings(&[
            clap::AppSettings::TrailingVarArg,
        ])
}

fn default_config_file() -> String {
    if let Some(proj_dirs) =
    directories::ProjectDirs::from("org.github", "rizsotto",  "bear") {
        let config_dir = proj_dirs.config_dir().to_path_buf();
        let config_file = config_dir.join("bear.conf");
        if let Some(str) = config_file.to_str() {
            return str.to_string()
        }
    }
    "./bear.conf".to_string()
}

fn build_command_build(matches: &ArgMatches, program: &path::Path) -> Result<Command> {
    let modes = build_intercept_modes(matches, program)?;
    let command = values_t!(matches, "command", String)?;
    let output = value_t!(matches, "output", path::PathBuf)?;
    let config = value_t!(matches, "config", path::PathBuf)?;
    Ok(Command::CompilationDatabaseBuild { output, modes, command, config })
}

fn parse_intercept<'a, 'b>() -> clap::App<'a, 'b> {
    clap::SubCommand::with_name("intercept")
        .arg(clap::Arg::with_name("output")
            .short("o")
            .long("output")
            .value_name("FILE")
            .default_value("commands.n3")
            .display_order(10))
        .args(args_intercept_modes().as_ref())
        .arg(arg_command())
        .settings(&[
            clap::AppSettings::TrailingVarArg,
        ])
}

fn build_command_intercept(matches: &ArgMatches, program: &path::Path) -> Result<Command> {
    let modes = build_intercept_modes(matches, program)?;
    let command = values_t!(matches, "command", String)?;
    let output = value_t!(matches, "output", path::PathBuf)?;
    Ok(Command::OntologyBuild { output, modes, command })
}

mod error {
    error_chain! {
        foreign_links {
            Clap(::clap::Error);
            Intercept(::ear::Error);
        }
    }
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
                "--exec-path", "cc",
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
                "--exec-path", "cc",
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
                "--exec-path", "cc");
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
                "--exec-file", "/usr/bin/cc",
                "--exec-path", "cc",
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
                "--exec-file", "/usr/bin/cc",
                "--exec-search-path", "/usr/bin:/usr/local/bin",
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
                "--exec-path", "cc",
                "--", "cc", "-c", "source.c");
            let command = parse_arguments(arguments.as_slice()).unwrap();

            let expected_command = Command::Supervise {
                session: Session {
                    destination: path::PathBuf::from("/tmp/bear"),
                    verbose: false,
                    modes: vec!(
                        InterceptMode::WrapperPreload {
                            library: path::PathBuf::from("/usr/local/lib/libear.so"),
                            wrapper: path::PathBuf::from("bear"),
                        }),
                },
                execution: ExecutionRequest {
                    executable: Executable::WithPath("cc".to_string()),
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
                "--exec-file", "/usr/bin/cc",
                "--", "cc", "-c", "source.c");
            let command = parse_arguments(arguments.as_slice()).unwrap();

            let expected_command = Command::Supervise {
                session: Session {
                    destination: path::PathBuf::from("/tmp/bear"),
                    verbose: false,
                    modes: vec!(
                        InterceptMode::WrapperPreload {
                            library: path::PathBuf::from("/usr/local/lib/libear.so"),
                            wrapper: path::PathBuf::from("bear"),
                        }),
                },
                execution: ExecutionRequest {
                    executable: Executable::WithFilename(path::PathBuf::from("/usr/bin/cc")),
                    arguments: vec_of_strings!("cc", "-c", "source.c"),
                }
            };
            assert_eq!(expected_command, command);
        }
    }

    mod configure_command {
        use super::*;

        #[test]
        #[should_panic]
        fn missing_command() {
            let arguments = vec_of_strings!(
                "bear",
                "configure",
                "--library", "/usr/local/lib/libear.so",
                "--wrapper", "cc", "/usr/bin/cc");
            let _ = parse_arguments(arguments.as_slice()).unwrap();
        }

        #[test]
        fn parsed_with_modes() {
            let arguments = vec_of_strings!(
                "bear",
                "configure",
                "--library", "/usr/local/share/bear/libear.so",
                "--cc-wrapper", "/usr/bin/cc", "/usr/local/share/bear/cc",
                "--cxx-wrapper", "/usr/bin/c++", "/usr/local/share/bear/c++",
                "--", "make");
            let command = parse_arguments(arguments.as_slice()).unwrap();

            let expected_command = Command::InjectWrappers {
                modes: vec!(
                    InterceptMode::WrapperPreload {
                        library: path::PathBuf::from("/usr/local/share/bear/libear.so"),
                        wrapper: path::PathBuf::from("bear"),
                    },
                    InterceptMode::WrapperCC {
                        compiler: path::PathBuf::from("/usr/bin/cc"),
                        wrapper: path::PathBuf::from("/usr/local/share/bear/cc"),
                    },
                    InterceptMode::WrapperCXX {
                        compiler: path::PathBuf::from("/usr/bin/c++"),
                        wrapper: path::PathBuf::from("/usr/local/share/bear/c++"),
                    },
                ),
                command: vec_of_strings!("make")
            };
            assert_eq!(expected_command, command);
        }

        #[test]
        fn parsed_without_modes() {
            let arguments = vec_of_strings!(
                "bear",
                "configure",
                "--", "make");
            let command = parse_arguments(arguments.as_slice()).unwrap();

            let expected_command = Command::InjectWrappers {
                modes: vec!(),
                command: vec_of_strings!("make")
            };
            assert_eq!(expected_command, command);
        }
    }

    mod build_command {
        use super::*;

        #[test]
        #[should_panic]
        fn missing_command() {
            let arguments = vec_of_strings!(
                "bear",
                "build",
                "--library", "/usr/local/share/bear/libear.so",
                "--cc-wrapper", "/usr/bin/cc", "/usr/local/share/bear/cc");
            let _ = parse_arguments(arguments.as_slice()).unwrap();
        }

        #[test]
        fn parsed_simple() {
            let arguments = vec_of_strings!(
                "bear",
                "build",
                "--", "make");
            let command = parse_arguments(arguments.as_slice()).unwrap();

            let expected_command = Command::CompilationDatabaseBuild {
                modes: vec!(),
                command: vec_of_strings!("make"),
                output: path::PathBuf::from("compile_commands.json"),
                config: path::PathBuf::from(default_config_file().as_str()),
            };
            assert_eq!(expected_command, command);
        }

        #[test]
        fn parsed_with_modes() {
            let arguments = vec_of_strings!(
                "bear",
                "build",
                "--library", "/usr/local/share/bear/libear.so",
                "--cc-wrapper", "/usr/bin/cc", "/usr/local/share/bear/cc",
                "--cxx-wrapper", "/usr/bin/c++", "/usr/local/share/bear/c++",
                "--", "make");
            let command = parse_arguments(arguments.as_slice()).unwrap();

            let expected_command = Command::CompilationDatabaseBuild {
                modes: vec!(
                    InterceptMode::WrapperPreload {
                        library: path::PathBuf::from("/usr/local/share/bear/libear.so"),
                        wrapper: path::PathBuf::from("bear"),
                    },
                    InterceptMode::WrapperCC {
                        compiler: path::PathBuf::from("/usr/bin/cc"),
                        wrapper: path::PathBuf::from("/usr/local/share/bear/cc"),
                    },
                    InterceptMode::WrapperCXX {
                        compiler: path::PathBuf::from("/usr/bin/c++"),
                        wrapper: path::PathBuf::from("/usr/local/share/bear/c++"),
                    },
                ),
                command: vec_of_strings!("make"),
                output: path::PathBuf::from("compile_commands.json"),
                config: path::PathBuf::from(default_config_file().as_str()),
            };
            assert_eq!(expected_command, command);
        }

        #[test]
        fn parsed_with_output() {
            let arguments = vec_of_strings!(
                "bear",
                "build",
                "-o", "commands.json",
                "--", "make");
            let command = parse_arguments(arguments.as_slice()).unwrap();

            let expected_command = Command::CompilationDatabaseBuild {
                modes: vec!(),
                command: vec_of_strings!("make"),
                output: path::PathBuf::from("commands.json"),
                config: path::PathBuf::from(default_config_file().as_str()),
            };
            assert_eq!(expected_command, command);
        }

        #[test]
        fn parsed_with_config() {
            let arguments = vec_of_strings!(
                "bear",
                "build",
                "-c", "/path/to/bear.conf",
                "--", "make");
            let command = parse_arguments(arguments.as_slice()).unwrap();

            let expected_command = Command::CompilationDatabaseBuild {
                modes: vec!(),
                command: vec_of_strings!("make"),
                output: path::PathBuf::from("compile_commands.json"),
                config: path::PathBuf::from("/path/to/bear.conf"),
            };
            assert_eq!(expected_command, command);
        }
    }

    mod intercept_command {
        use super::*;

        #[test]
        #[should_panic]
        fn missing_command() {
            let arguments = vec_of_strings!(
                "bear",
                "build",
                "--library", "/usr/local/lib/libear.so",
                "--wrapper", "cc", "/usr/bin/cc");
            let _ = parse_arguments(arguments.as_slice()).unwrap();
        }

        #[test]
        fn parsed_simple() {
            let arguments = vec_of_strings!(
                "bear",
                "intercept",
                "--", "make");
            let command = parse_arguments(arguments.as_slice()).unwrap();

            let expected_command = Command::OntologyBuild {
                modes: vec!(),
                command: vec_of_strings!("make"),
                output: path::PathBuf::from("commands.n3"),
            };
            assert_eq!(expected_command, command);
        }

        #[test]
        fn parsed_with_modes() {
            let arguments = vec_of_strings!(
                "bear",
                "intercept",
                "--library", "/usr/local/share/bear/libear.so",
                "--cc-wrapper", "/usr/bin/cc", "/usr/local/share/bear/cc",
                "--cxx-wrapper", "/usr/bin/c++", "/usr/local/share/bear/c++",
                "--", "make");
            let command = parse_arguments(arguments.as_slice()).unwrap();

            let expected_command = Command::OntologyBuild {
                modes: vec!(
                    InterceptMode::WrapperPreload {
                        library: path::PathBuf::from("/usr/local/share/bear/libear.so"),
                        wrapper: path::PathBuf::from("bear"),
                    },
                    InterceptMode::WrapperCC {
                        compiler: path::PathBuf::from("/usr/bin/cc"),
                        wrapper: path::PathBuf::from("/usr/local/share/bear/cc"),
                    },
                    InterceptMode::WrapperCXX {
                        compiler: path::PathBuf::from("/usr/bin/c++"),
                        wrapper: path::PathBuf::from("/usr/local/share/bear/c++"),
                    },
                ),
                command: vec_of_strings!("make"),
                output: path::PathBuf::from("commands.n3"),
            };
            assert_eq!(expected_command, command);
        }

        #[test]
        fn parsed_with_output() {
            let arguments = vec_of_strings!(
                "bear",
                "intercept",
                "-o", "commands.json",
                "--", "make");
            let command = parse_arguments(arguments.as_slice()).unwrap();

            let expected_command = Command::OntologyBuild {
                modes: vec!(),
                command: vec_of_strings!("make"),
                output: path::PathBuf::from("commands.json"),
            };
            assert_eq!(expected_command, command);
        }
    }
}
