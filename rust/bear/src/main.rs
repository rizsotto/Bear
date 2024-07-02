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

use log::{debug, LevelFilter};
use simple_logger::SimpleLogger;

mod command;
mod configuration;
mod fixtures;


/// Driver function of the application.
fn main() -> anyhow::Result<()> {
    let matches = command::cli().get_matches();
    let arguments = command::Arguments::try_from(matches)?;
    prepare_logging(arguments.verbose)?;

    debug!("Arguments: {:?}", arguments);

    return Ok(());
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
