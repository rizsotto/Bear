// SPDX-License-Identifier: GPL-3.0-or-later

use bear::{args, config, modes};
use std::env;
use std::process::ExitCode;

/// Driver function of the application.
fn main() -> anyhow::Result<ExitCode> {
    // Initialize the logging system.
    env_logger::init();
    // Get the package name and version from Cargo
    let pkg_name = env!("CARGO_PKG_NAME");
    let pkg_version = env!("CARGO_PKG_VERSION");
    log::debug!("{pkg_name} v{pkg_version}");

    // Parse the command line arguments.
    let matches = args::cli().get_matches();
    let arguments = args::Arguments::try_from(matches)?;

    // Print the arguments.
    log::debug!("Arguments: {arguments:?}");
    // Load the configuration.
    let configuration = config::Loader::load(&arguments.config)?;
    log::debug!("Configuration: {configuration:?}");

    // Run the application.
    let application = modes::Mode::configure(arguments, configuration)?;
    log::debug!("Configuration complete, running the build now...");
    let result = application.run();
    log::debug!("Exit code: {result:?}");

    Ok(result)
}
