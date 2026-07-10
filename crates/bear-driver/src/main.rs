// SPDX-License-Identifier: GPL-3.0-or-later

//! This binary is the build supervisor (driver) of the application.
//!
//! It orchestrates interception and output: it captures the application
//! context, derives the installation layout from the current executable,
//! parses the command line arguments, and loads the configuration. From
//! those inputs it configures the selected mode and runs it, returning the
//! build's exit code.
//!
//! The heavy lifting lives elsewhere: the `bear` library provides argument
//! parsing, configuration, and the modes, while `intercept_supervisor`
//! supplies the application context and installation layout.

use bear::{args, config, modes};
use intercept_supervisor::{context, installation};
use std::env;
use std::process::ExitCode;

/// Driver function of the application.
fn main() -> anyhow::Result<ExitCode> {
    // Initialize the logging system. `RUST_LOG` is honored when set; when
    // unset, default to `warn` so recoverable anomalies (e.g. parse-sh skip
    // reports) reach stderr without opting in.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
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
    // Log the installation layout derived from the current executable.
    let layout = installation::InstallationLayout::try_from(context.current_executable.as_path())?;
    log::info!("{layout}");
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
