// SPDX-License-Identifier: GPL-3.0-or-later

//! Responsible for recognizing the semantic meaning of the executed commands.
//!
//! The recognition logic is implemented in the `interpreters` module.
//! Here we only handle the errors and logging them to the console.

use super::super::intercept;
use super::super::semantic;
use super::config;
use std::convert::TryFrom;

pub struct Recognition {
    interpreter: Box<dyn semantic::Interpreter>,
}

impl TryFrom<&config::Main> for Recognition {
    type Error = anyhow::Error;

    /// Creates an interpreter to recognize the compiler calls.
    ///
    /// Using the configuration we can define which compilers to include and exclude.
    /// Also read the environment variables to detect the compiler to include (and
    /// make sure those are not excluded either).
    // TODO: Use the CC or CXX environment variables to detect the compiler to include.
    //       Use the CC or CXX environment variables and make sure those are not excluded.
    //       Make sure the environment variables are passed to the method.
    fn try_from(config: &config::Main) -> Result<Self, Self::Error> {
        let compilers_to_include = match &config.intercept {
            config::Intercept::Wrapper { executables, .. } => executables.clone(),
            _ => vec![],
        };
        let compilers_to_exclude = match &config.output {
            config::Output::Clang { compilers, .. } => compilers
                .iter()
                .filter(|compiler| compiler.ignore == config::Ignore::Always)
                .map(|compiler| compiler.path.clone())
                .collect(),
            _ => vec![],
        };
        let interpreter = semantic::interpreters::Builder::new()
            .compilers_to_recognize(compilers_to_include.as_slice())
            .compilers_to_exclude(compilers_to_exclude.as_slice())
            .build();

        Ok(Recognition {
            interpreter: Box::new(interpreter),
        })
    }
}

impl Recognition {
    /// Simple call the semantic module to recognize the execution.
    /// Forward only the compiler calls, and log each recognition result.
    pub fn apply(&self, execution: intercept::Execution) -> Option<semantic::CompilerCall> {
        match self.interpreter.recognize(&execution) {
            semantic::Recognition::Success(semantic) => {
                log::debug!(
                    "execution recognized as compiler call, {:?} : {:?}",
                    semantic,
                    execution
                );
                Some(semantic)
            }
            semantic::Recognition::Ignored => {
                log::debug!("execution recognized, but ignored: {:?}", execution);
                None
            }
            semantic::Recognition::Error(reason) => {
                log::debug!(
                    "execution recognized with failure, {:?} : {:?}",
                    reason,
                    execution
                );
                None
            }
            semantic::Recognition::Unknown => {
                log::debug!("execution not recognized: {:?}", execution);
                None
            }
        }
    }
}
