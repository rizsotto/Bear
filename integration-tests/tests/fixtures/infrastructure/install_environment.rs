// SPDX-License-Identifier: GPL-3.0-or-later

use crate::fixtures::constants::*;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Install environment for Bear
///
/// On Unix, uses `scripts/install.sh` to create a proper installation layout
/// in a temp directory — this exercises the install script on every test run.
///
/// On Windows, runs `bear-driver.exe` directly (no preload library, no shell
/// entry script).
pub struct InstallEnvironment {
    #[allow(dead_code)]
    install_dir: tempfile::TempDir,
    bear: PathBuf,
}

impl InstallEnvironment {
    pub fn new() -> Result<Self> {
        let install_dir = tempfile::TempDir::new().with_context(|| "install dir tempdir failed")?;

        if cfg!(unix) {
            Self::install_via_script(install_dir.path())?;
            let bear = install_dir.path().join("bin").join("bear");
            Ok(Self { install_dir, bear })
        } else {
            // Windows: run bear-driver.exe directly from the build output
            let bear = PathBuf::from(DRIVER_EXECUTABLE_PATH);
            Ok(Self { install_dir, bear })
        }
    }

    fn install_via_script(destdir: &Path) -> Result<()> {
        let source_dir = Path::new(DRIVER_EXECUTABLE_PATH)
            .parent()
            .with_context(|| "cannot determine artifact directory from DRIVER_EXECUTABLE_PATH")?;

        let output = std::process::Command::new("bash")
            .arg(INSTALL_SCRIPT_PATH)
            .env("PREFIX", destdir)
            .env("INTERCEPT_LIBDIR", INTERCEPT_LIBDIR)
            .env("SRCDIR", source_dir)
            .output()
            .with_context(|| "failed to run install.sh")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "install.sh failed with exit code {:?}\nstdout: {}\nstderr: {}",
                output.status.code(),
                stdout,
                stderr
            );
        }

        Ok(())
    }

    pub fn path(&self) -> &Path {
        self.bear.as_path()
    }
}
