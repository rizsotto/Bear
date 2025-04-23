// SPDX-License-Identifier: GPL-3.0-or-later

use bear::modes::{Combined, Intercept, Mode, Semantic};
use bear::{args, config};
use std::env;
use std::process::ExitCode;

/// Driver function of the application.
fn main() -> anyhow::Result<ExitCode> {
    // Initialize the logging system.
    env_logger::init();
    // Get the package name and version from Cargo
    let pkg_name = env!("CARGO_PKG_NAME");
    let pkg_version = env!("CARGO_PKG_VERSION");
    log::debug!("{} v{}", pkg_name, pkg_version);

    // Parse the command line arguments.
    let matches = args::cli().get_matches();
    let arguments = args::Arguments::try_from(matches)?;

    // Print the arguments.
    log::debug!("Arguments: {:?}", arguments);
    // Load the configuration.
    let configuration = config::Loader::load(&arguments.config)?;
    log::debug!("Configuration: {:?}", configuration);

    // Run the application.
    let application = Application::configure(arguments, configuration)?;
    let result = application.run();
    log::debug!("Exit code: {:?}", result);

    Ok(result)
}

/// Represent the application state.
enum Application {
    Intercept(Intercept),
    Semantic(Semantic),
    Combined(Combined),
}

impl Application {
    /// Configure the application based on the command line arguments and the configuration.
    ///
    /// Trying to validate the configuration and the arguments, while creating the application
    /// state that will be used by the `run` method. Trying to catch problems early before
    /// the actual execution of the application.
    fn configure(args: args::Arguments, config: config::Main) -> anyhow::Result<Self> {
        match args.mode {
            args::Mode::Intercept { input, output } => {
                log::debug!("Mode: intercept");
                Intercept::from(input, output, config).map(Application::Intercept)
            }
            args::Mode::Semantic { input, output } => {
                log::debug!("Mode: semantic analysis");
                Semantic::from(input, output, config).map(Application::Semantic)
            }
            args::Mode::Combined { input, output } => {
                log::debug!("Mode: intercept and semantic analysis");
                Combined::from(input, output, config).map(Application::Combined)
            }
        }
    }

    fn run(self) -> ExitCode {
        let status = match self {
            Application::Intercept(intercept) => intercept.run(),
            Application::Semantic(semantic) => semantic.run(),
            Application::Combined(all) => all.run(),
        };
        status.unwrap_or_else(|error| {
            log::error!("Bear: {}", error);
            ExitCode::FAILURE
        })
    }
}
