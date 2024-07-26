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
use std::process::ExitCode;
use log::{debug, LevelFilter};
use simple_logger::SimpleLogger;
use crate::command::Mode;
use crate::configuration::Configuration;

mod command;
mod configuration;
mod fixtures;


/// Driver function of the application.
fn main() -> anyhow::Result<ExitCode> {
    // Parse the command line arguments.
    let matches = command::cli().get_matches();
    let arguments = command::Arguments::try_from(matches)?;
    // Initialize the logging system.
    prepare_logging(arguments.verbose)?;

    // Get the package name and version from Cargo
    let pkg_name = env!("CARGO_PKG_NAME");
    let pkg_version = env!("CARGO_PKG_VERSION");
    debug!("{} v{}", pkg_name, pkg_version);
    // Print the arguments.
    debug!("Arguments: {:?}", arguments);
    // Load the configuration.
    let configuration = Configuration::load(&arguments.config)?;
    debug!("Configuration: {:?}", configuration);

    // Run the application.
    let result = match arguments.mode {
        Mode::Intercept { input, output } => ExitCode::SUCCESS,
        Mode::Semantic { input, output } => ExitCode::FAILURE,
        Mode::All { input, output } => ExitCode::from(100),
    };

    return Ok(result);
}

/// Initializes the logging system.
///
/// # Arguments
///
/// * `level` - The verbosity level of the logging system.
///
/// # Returns
///
/// Failure when the downstream library fails to initialize the logging system.
fn prepare_logging(level: u8) -> anyhow::Result<()> {
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
