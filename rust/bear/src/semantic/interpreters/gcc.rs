// SPDX-License-Identifier: GPL-3.0-or-later

use nom::branch::alt;
use nom::multi::many1;
use nom::sequence::preceded;

use super::super::{Interpreter, Meaning, Recognition};
use intercept::Execution;
use internal::Argument;

pub(super) struct Gcc {}

impl Gcc {
    pub(super) fn new() -> Box<dyn Interpreter> {
        Box::new(Gcc {})
    }
}

impl Interpreter for Gcc {
    fn recognize(&self, execution: &Execution) -> Recognition<Meaning> {
        let mut parser = preceded(
            internal::compiler,
            many1(alt((internal::flag, internal::source))),
        );

        match parser(execution.arguments.as_slice()) {
            Ok(result) => {
                // TODO: append flags from environment
                let flags = result.1;
                let passes = Argument::passes(flags.as_slice());

                Recognition::Success(Meaning::Compiler {
                    compiler: execution.executable.clone(),
                    working_dir: execution.working_dir.clone(),
                    passes,
                })
            }
            Err(error) => {
                log::debug!("Gcc failed to parse it: {error}.");
                Recognition::Unknown
            }
        }
    }
}

mod internal {
    use lazy_static::lazy_static;
    use nom::{error::ErrorKind, IResult};
    use regex::Regex;
    use std::path::PathBuf;

    use super::super::super::CompilerPass;
    use super::super::matchers::source::looks_like_a_source_file;

    #[derive(Debug, PartialEq)]
    enum Language {
        C,
        Cpp,
        ObjectiveC,
        ObjectiveCpp,
        Ada,
        Fortran,
        Go,
        D,
        Assembler,
        Other,
    }

    #[derive(Debug, PartialEq)]
    enum Pass {
        Preprocessor,
        Compiler,
        Linker,
    }

    #[derive(Debug, PartialEq)]
    enum Meaning {
        Compiler,
        ControlKindOfOutput { stop_before: Option<Pass> },
        ControlLanguage(Language),
        ControlPass(Pass),
        Diagnostic,
        Debug,
        Optimize,
        Instrumentation,
        DirectorySearch(Option<Pass>),
        Developer,
        Input(Pass),
        Output,
    }

    /// Compiler flags are varies the number of arguments, but means one thing.
    pub(crate) struct Argument<'a> {
        arguments: &'a [String],
        meaning: Meaning,
    }

    impl<'a> Argument<'a> {
        pub(crate) fn passes(flags: &[Argument]) -> Vec<CompilerPass> {
            let mut pass: Pass = Pass::Linker;
            let mut inputs: Vec<String> = vec![];
            let mut output: Option<String> = None;
            let mut args: Vec<String> = vec![];

            for flag in flags {
                match flag.meaning {
                    Meaning::ControlKindOfOutput {
                        stop_before: Some(Pass::Compiler),
                    } => {
                        pass = Pass::Preprocessor;
                        args.extend(flag.arguments.into_iter().map(String::to_owned));
                    }
                    Meaning::ControlKindOfOutput {
                        stop_before: Some(Pass::Linker),
                    } => {
                        pass = Pass::Compiler;
                        args.extend(flag.arguments.into_iter().map(String::to_owned));
                    }
                    Meaning::ControlKindOfOutput { .. }
                    | Meaning::ControlLanguage(_)
                    | Meaning::ControlPass(Pass::Preprocessor)
                    | Meaning::ControlPass(Pass::Compiler)
                    | Meaning::Diagnostic
                    | Meaning::Debug
                    | Meaning::Optimize
                    | Meaning::Instrumentation
                    | Meaning::DirectorySearch(None) => {
                        args.extend(flag.arguments.into_iter().map(String::to_owned));
                    }
                    Meaning::Input(_) => {
                        assert_eq!(flag.arguments.len(), 1);
                        inputs.push(flag.arguments[0].clone())
                    }
                    Meaning::Output => {
                        assert_eq!(flag.arguments.len(), 1);
                        output = Some(flag.arguments[0].clone())
                    }
                    _ => {}
                }
            }

            match pass {
                Pass::Preprocessor if inputs.is_empty() => {
                    vec![]
                }
                Pass::Preprocessor => {
                    vec![CompilerPass::Preprocess]
                }
                Pass::Compiler | Pass::Linker => inputs
                    .into_iter()
                    .map(|source| CompilerPass::Compile {
                        source: PathBuf::from(source),
                        output: output.as_ref().map(PathBuf::from),
                        flags: args.clone(),
                    })
                    .collect(),
            }
        }
    }

    pub(crate) fn compiler(i: &[String]) -> IResult<&[String], Argument> {
        let candidate = &i[0];
        if COMPILER_REGEX.is_match(candidate) {
            const MEANING: Meaning = Meaning::Compiler;
            Ok((
                &i[1..],
                Argument {
                    arguments: &i[..0],
                    meaning: MEANING,
                },
            ))
        } else {
            // Declare it as a non-recoverable error, so argument processing will stop after this.
            Err(nom::Err::Failure(nom::error::Error::new(i, ErrorKind::Tag)))
        }
    }

    pub(crate) fn source(i: &[String]) -> IResult<&[String], Argument> {
        let candidate = &i[0];
        if looks_like_a_source_file(candidate.as_str()) {
            const MEANING: Meaning = Meaning::Input(Pass::Preprocessor);
            Ok((
                &i[1..],
                Argument {
                    arguments: &i[..0],
                    meaning: MEANING,
                },
            ))
        } else {
            Err(nom::Err::Error(nom::error::Error::new(i, ErrorKind::Tag)))
        }
    }

    pub(crate) fn flag(i: &[String]) -> IResult<&[String], Argument> {
        todo!()
    }

    lazy_static! {
        // - cc
        // - c++
        // - cxx
        // - CC
        // - mcc, gcc, m++, g++, gfortran, fortran
        //   - with prefixes like: arm-none-eabi-
        //   - with postfixes like: -7.0 or 6.4.0
        static ref COMPILER_REGEX: Regex = Regex::new(
            r"(^(cc|c\+\+|cxx|CC|(([^-]*-)*([mg](cc|\+\+)|[g]?fortran)(-?\d+(\.\d+){0,2})?))$)"
        ).unwrap();
    }
}
