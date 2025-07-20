// SPDX-License-Identifier: GPL-3.0-or-later

use crate::args::BuildCommand;
use crate::intercept;
use crate::intercept::supervise;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitStatus;

/// A trait for executing build commands.
///
/// Executors are responsible for running the actual build process while
/// allowing command interception to occur. They manage the lifecycle of
/// the build command and report its exit status.
///
/// # Type Parameters
/// - `E`: The error type that can occur during execution
pub trait Executor<E> {
    /// Executes the given build command.
    ///
    /// This is a blocking operation that runs the build command to completion.
    /// During execution, the command and its subprocesses may be intercepted
    /// by Bear's interception mechanisms.
    ///
    /// # Arguments
    /// * `command` - The build command to execute
    ///
    /// # Returns
    /// * `Ok(ExitCode)` - The build completed with the given exit code
    /// * `Err(E)` - An error occurred during execution
    fn run(&self, _: BuildCommand) -> Result<ExitStatus, E>;
}

struct BuildExecutor {
    environment: HashMap<String, String>,
}

impl BuildExecutor {
    fn build(&self, val: BuildCommand) -> std::process::Command {
        let mut command = std::process::Command::new(val.arguments.first().unwrap());
        command.args(val.arguments.iter().skip(1));
        command.envs(self.environment.clone());
        command
    }
}

impl Executor<supervise::SuperviseError> for BuildExecutor {
    fn run(&self, build_command: BuildCommand) -> Result<ExitStatus, supervise::SuperviseError> {
        log::debug!("Running build command: {:?}", build_command);
        let mut command = self.build(build_command);
        supervise::supervise(&mut command)
    }
}
