// SPDX-License-Identifier: GPL-3.0-or-later

//! This module defines the configuration of the application.
//!
//! The configuration is either loaded from a file or used with default
//! values, which are defined in the code. The configuration exposes the main
//! logical steps that the application will follow.
//!
//! The configuration file syntax is based on the YAML format.
//! The default configuration file name is `bear.yml`.
//!
//! The configuration file location is searched in the following order:
//! 1. The current working directory
//! 2. The local configuration directory of the user
//! 3. The configuration directory of the user
//! 4. The local configuration directory of the application
//! 5. The configuration directory of the application
//!
//! ```yaml
//! schema: 4.1
//!
//! intercept:
//!   mode: wrapper
//!
//! compilers:
//!   - path: /usr/local/bin/cc
//!     as: gcc
//!   - path: /usr/bin/cc
//!     ignore: true
//!   - path: /usr/bin/clang++
//!
//! sources:
//!   directories:
//!     - path: "/opt/project/sources"
//!       action: include
//!     - path: "/opt/project/tests"
//!       action: exclude
//!
//! duplicates:
//!   match_on: [file, directory]
//!
//! format:
//!   paths:
//!     directory: canonical
//!     file: canonical
//!   entries:
//!     use_array_format: true
//!     include_output_field: true
//! ```
//!
//! ```yaml
//! schema: 4.1
//!
//! intercept:
//!   mode: preload
//!
//! format:
//!   paths:
//!     directory: as-is
//!     file: as-is
//!   entries:
//!     use_array_format: true
//!     include_output_field: true
//! ```

// Re-Export the types and the loader module content.
pub use loader::{ConfigError, Loader};
pub use types::*;
pub use validation::Validator;

pub mod loader;
mod types;
pub mod validation;
