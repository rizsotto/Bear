// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests for the installation layout that `scripts/install.sh` produces.

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::InstallEnvironment;
use anyhow::Result;

/// Distribution tooling (RPM's debuginfo extraction and strip step) only
/// processes shared libraries that are executable, so packagers had to
/// `chmod` the preload library after running the install script.
#[test]
fn preload_library_is_installed_executable() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let prefix = tempfile::TempDir::new()?;

    InstallEnvironment::install_via_script(prefix.path())?;
    let sut = prefix.path().join("libexec/bear").join(INTERCEPT_LIBDIR).join(PRELOAD_LIBRARY);

    let mode = std::fs::metadata(&sut)?.permissions().mode();
    assert_eq!(mode & 0o777, 0o755, "{} has mode {:o}", sut.display(), mode & 0o777);
    Ok(())
}
