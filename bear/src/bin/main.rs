// SPDX-License-Identifier: GPL-3.0-or-later

use bear::{args, config, context, modes};
use std::env;
use std::process::ExitCode;

/// Driver function of the application.
fn main() -> anyhow::Result<ExitCode> {
    // Initialize the logging system.
    env_logger::init();
    // Get the package name and version from Cargo
    let pkg_name = env!("CARGO_PKG_NAME");
    let pkg_version = env!("CARGO_PKG_VERSION");
    log::info!("{pkg_name} v{pkg_version}");
    let os = env::consts::OS;
    let family = env::consts::FAMILY;
    let arch = env::consts::ARCH;
    log::info!("Running on... {family}/{os} {arch}");

    // Capture application context.
    let context = context::Context::capture()?;
    log::info!("{context}");
    // Parse the command line arguments.
    let matches = args::cli().get_matches();
    let arguments = args::Arguments::try_from(matches)?;
    log::info!("{arguments}");
    // Load the configuration.
    let configuration = config::Loader::load(&context, &arguments.config)?;
    log::info!("{configuration}");

    // Run the application.
    let application = modes::Mode::configure(context, arguments, configuration)?;
    log::debug!("Configuration complete, running the build now...");
    let result = application.run();
    log::debug!("Exit code: {result:?}");

    Ok(result)
}
