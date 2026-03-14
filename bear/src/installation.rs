// SPDX-License-Identifier: GPL-3.0-or-later

//! # Installation Layout
//!
//! This module encodes the expected installation layout of Bear's artifacts.
//!
//! ## Layout
//!
//! Bear is installed with the following directory structure:
//!
//! ```text
//! <prefix>/
//! ├── bin/
//! │   └── bear                ← shell script, calls bear-driver by absolute path
//! └── <out_of_path>/
//!     ├── bin/
//!     │   ├── bear-driver     ← current_executable
//!     │   └── bear-wrapper    ← sibling of bear-driver
//!     └── <INTERCEPT_LIBDIR>/
//!         └── libexec.so      ← one level up from bin/, then into INTERCEPT_LIBDIR/
//! ```
//!
//! `bear-driver` locates its siblings using **relative paths only**:
//! - `bear-wrapper` is a sibling in the same `bin/` directory.
//! - `libexec.so` is reached via `../<INTERCEPT_LIBDIR>/libexec.so` relative to `bin/`.
//!
//! The `INTERCEPT_LIBDIR` value is set at build time (defaults to `lib`). On glibc-based
//! Linux, packagers can set it to `$LIB` so the dynamic linker expands it at runtime.

use std::fmt;
use std::path::{Path, PathBuf};
use thiserror::Error;

const WRAPPER_NAME: &str = env!("WRAPPER_NAME");
const PRELOAD_NAME: &str = env!("PRELOAD_NAME");
const INTERCEPT_LIBDIR: &str = env!("INTERCEPT_LIBDIR");

/// Errors that can occur when constructing an [`InstallationLayout`].
#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("Executable path is not absolute: {0:?}")]
    NotAbsolute(PathBuf),
    #[error("Executable path has no parent directory: {0:?}")]
    NoParent(PathBuf),
}

/// Represents the installation layout of Bear's artifacts.
///
/// This struct is constructed from the path of the `bear-driver` executable
/// and derives all other artifact locations using relative paths, matching
/// the layout described in `INSTALL.md`.
#[derive(Debug, Clone)]
pub struct InstallationLayout {
    /// The directory that contains the `bear-driver` executable.
    /// `wrapper_path` is derived relative to this directory.
    bin_dir: PathBuf,
    /// The directory that contains the `libexec.so` file.
    lib_dir: PathBuf,
}

impl InstallationLayout {
    /// Returns the path to the `bear-wrapper` executable.
    ///
    /// `bear-wrapper` is a sibling of `bear-driver` in the same `bin/` directory.
    pub fn wrapper_path(&self) -> PathBuf {
        self.bin_dir.join(WRAPPER_NAME)
    }

    /// Returns the path to the preload shared library (`libexec.so` / `libexec.dylib`).
    ///
    /// The library lives one directory level above `bin/`, inside the `INTERCEPT_LIBDIR`
    /// subdirectory (set at build time, defaults to `lib`).
    pub fn preload_path(&self) -> PathBuf {
        self.lib_dir.join(PRELOAD_NAME)
    }
}

impl fmt::Display for InstallationLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Installation Layout:")?;
        writeln!(f, "  Wrapper Executable: {}", self.wrapper_path().display())?;
        writeln!(f, "  Preload Library:    {}", self.preload_path().display())?;
        Ok(())
    }
}

impl TryFrom<&Path> for InstallationLayout {
    type Error = LayoutError;

    /// Construct an [`InstallationLayout`] from the absolute path of the
    /// `bear-driver` executable.
    ///
    /// # Errors
    ///
    /// - [`LayoutError::NotAbsolute`] if `executable` is not an absolute path.
    /// - [`LayoutError::NoParent`] if `executable` has no usable parent directory
    ///   (bare filename with no directory component, or sitting at the filesystem root).
    fn try_from(executable: &Path) -> Result<Self, Self::Error> {
        if !executable.is_absolute() {
            return Err(LayoutError::NotAbsolute(executable.to_path_buf()));
        }

        let bin_dir =
            executable.parent().ok_or_else(|| LayoutError::NoParent(executable.to_path_buf()))?.to_path_buf();

        let parent_dir =
            bin_dir.parent().ok_or_else(|| LayoutError::NoParent(executable.to_path_buf()))?.to_path_buf();

        Ok(Self { bin_dir, lib_dir: parent_dir.join(INTERCEPT_LIBDIR) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DRIVER_NAME: &str = env!("DRIVER_NAME");

    /// Build an absolute path from components, portable across platforms.
    ///
    /// On Unix the root is `/`, on Windows it is `C:\`.
    fn abs_path(components: &[&str]) -> PathBuf {
        let mut p = if cfg!(windows) { PathBuf::from("C:\\") } else { PathBuf::from("/") };
        for c in components {
            p.push(c);
        }
        p
    }

    #[test]
    fn test_layout_from_typical_install_path() {
        let exe = abs_path(&["usr", "local", "share", "bear", "bin", DRIVER_NAME]);
        let layout = InstallationLayout::try_from(exe.as_path()).unwrap();

        assert_eq!(layout.wrapper_path(), abs_path(&["usr", "local", "share", "bear", "bin", WRAPPER_NAME]));
        assert_eq!(
            layout.preload_path(),
            abs_path(&["usr", "local", "share", "bear", INTERCEPT_LIBDIR, PRELOAD_NAME])
        );
    }

    #[test]
    fn test_layout_from_minimal_path() {
        // Minimal valid absolute path: executable directly in a root-level directory.
        let exe = abs_path(&["bin", DRIVER_NAME]);
        let layout = InstallationLayout::try_from(exe.as_path()).unwrap();

        assert_eq!(layout.wrapper_path(), abs_path(&["bin", WRAPPER_NAME]));
        assert_eq!(layout.preload_path(), abs_path(&[INTERCEPT_LIBDIR, PRELOAD_NAME]));
    }

    #[test]
    fn test_layout_rom_bare_executable_name_fails() {
        // A bare filename is not absolute — must be rejected.
        let bare = Path::new(DRIVER_NAME);
        let err = InstallationLayout::try_from(bare).unwrap_err();
        assert!(matches!(err, LayoutError::NotAbsolute(_)));
    }

    #[test]
    fn test_layout_from_relative_path_fails() {
        let relative = PathBuf::from("bin").join(DRIVER_NAME);
        let err = InstallationLayout::try_from(relative.as_path()).unwrap_err();
        assert!(matches!(err, LayoutError::NotAbsolute(_)));
    }

    #[test]
    fn test_layout_from_root_executable_fails() {
        // An executable sitting directly at the filesystem root has no usable parent.
        let root_exe = abs_path(&[DRIVER_NAME]);
        let err = InstallationLayout::try_from(root_exe.as_path()).unwrap_err();
        assert!(matches!(err, LayoutError::NoParent(_)));
    }

    #[test]
    fn test_display_contains_expected_sections() {
        let exe = abs_path(&["usr", "local", "share", "bear", "bin", DRIVER_NAME]);
        let layout = InstallationLayout::try_from(exe.as_path()).unwrap();
        let output = format!("{layout}");

        assert!(output.contains("Installation Layout:"));
        assert!(output.contains("Wrapper Executable:"));
        assert!(output.contains("Preload Library:"));
    }

    #[test]
    fn test_display_contains_correct_paths() {
        let exe = abs_path(&["usr", "local", "share", "bear", "bin", DRIVER_NAME]);
        let layout = InstallationLayout::try_from(exe.as_path()).unwrap();
        let output = format!("{layout}");

        let expected_wrapper = abs_path(&["usr", "local", "share", "bear", "bin", WRAPPER_NAME]);
        let expected_preload = abs_path(&["usr", "local", "share", "bear", INTERCEPT_LIBDIR, PRELOAD_NAME]);

        assert!(output.contains(&expected_wrapper.display().to_string()));
        assert!(output.contains(&expected_preload.display().to_string()));
    }
}
