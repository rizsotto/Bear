// SPDX-License-Identifier: GPL-3.0-or-later

//! Preload interception state passed from the supervisor to the preload library.

use std::net::SocketAddr;
use std::path::PathBuf;

/// Represents the state information needed for preload-based interception.
///
/// This struct is serialized to JSON and passed to the preloaded library via
/// an environment variable. It contains all the information the library needs
/// to report execution events back to the Bear process.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PreloadState {
    /// The socket address where execution events should be reported
    pub destination: SocketAddr,
    /// The path to the preload library itself
    pub library: PathBuf,
}

impl TryInto<String> for PreloadState {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        serde_json::to_string(&self)
    }
}
impl TryFrom<&str> for PreloadState {
    type Error = serde_json::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        serde_json::from_str(value)
    }
}
