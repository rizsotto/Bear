// SPDX-License-Identifier: GPL-3.0-or-later

//! Wrapper configuration management for Bear interception.
//!
//! This module provides structures and utilities for managing wrapper executable
//! configurations, including serialization and deserialization of executable mappings.

/// The filename used for wrapper configuration files.
pub const CONFIG_FILENAME: &str = "wrappers.cfg";

/// The directory name used for wrapper executables in the current working directory.
pub const WRAPPER_DIR_NAME: &str = ".bear";

use std::collections::HashMap;
use std::fmt::Display;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// A managed directory that is created in the current working directory
/// and cleaned up when dropped.
///
/// Unlike `TempDir`, this uses a deterministic path (`.bear/` in cwd) which
/// is essential for autotools-style builds where `configure` caches compiler
/// paths that must remain valid across multiple Bear invocations.
pub struct ManagedDirectory {
    path: PathBuf,
}

impl ManagedDirectory {
    /// Creates a new managed directory at `.bear/` in the specified working directory.
    ///
    /// If the directory already exists, it is deleted first to ensure a clean state.
    /// This is important because we need fresh wrapper executables for each Bear run.
    ///
    /// # Arguments
    ///
    /// * `working_dir` - The directory where `.bear/` will be created (typically from context)
    pub fn create(working_dir: &Path) -> Result<Self, std::io::Error> {
        let path = working_dir.join(WRAPPER_DIR_NAME);

        // Remove existing directory if present (clean slate for each run)
        if path.exists() {
            std::fs::remove_dir_all(&path)?;
        }

        // Create fresh directory
        std::fs::create_dir_all(&path)?;

        Ok(Self { path })
    }

    /// Returns the path to the managed directory.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ManagedDirectory {
    fn drop(&mut self) {
        // Best-effort cleanup - ignore errors
        if self.path.exists() {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// Configuration structure for wrapper executables.
///
/// This structure contains the mapping from wrapper executable names to their
/// corresponding real executable paths. It is designed to be minimal and focused
/// solely on the executable mappings.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WrapperConfig {
    /// Map from wrapper executable name to the real executable path
    pub executables: HashMap<String, PathBuf>,
}

impl Default for WrapperConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl WrapperConfig {
    /// Creates a new empty wrapper configuration.
    pub fn new() -> Self {
        Self { executables: HashMap::new() }
    }

    /// Adds an executable mapping to the configuration.
    pub fn add_executable(&mut self, name: String, path: PathBuf) {
        self.executables.insert(name, path);
    }

    /// Gets the real executable path for a given wrapper name.
    pub fn get_executable(&self, name: &str) -> Option<&PathBuf> {
        self.executables.get(name)
    }
}

impl Display for WrapperConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.executables.is_empty() {
            write!(f, "No executables configured")
        } else {
            for (name, path) in self.executables.iter() {
                writeln!(f, "    {} -> {}", name, path.display())?;
            }
            Ok(())
        }
    }
}

/// Reader for wrapper configurations.
///
/// Handles deserialization of wrapper configurations from various input sources.
pub struct WrapperConfigReader;

impl WrapperConfigReader {
    /// Reads wrapper configuration from any source implementing `Read`.
    pub fn read<R: Read>(reader: R) -> Result<WrapperConfig, ConfigError> {
        let buf_reader = BufReader::new(reader);
        serde_json::from_reader(buf_reader).map_err(ConfigError::Json)
    }

    /// Reads wrapper configuration from a file path.
    pub fn read_from_file<P: AsRef<Path>>(path: P) -> Result<WrapperConfig, ConfigError> {
        let file = std::fs::File::open(path.as_ref()).map_err(ConfigError::Io)?;
        Self::read(file)
    }
}

/// Writer for wrapper configurations.
///
/// Handles serialization of wrapper configurations to various output destinations.
pub struct WrapperConfigWriter;

impl WrapperConfigWriter {
    /// Writes wrapper configuration to any destination implementing `Write`.
    pub fn write<W: Write>(config: &WrapperConfig, writer: W) -> Result<(), ConfigError> {
        let buf_writer = BufWriter::new(writer);
        serde_json::to_writer_pretty(buf_writer, config).map_err(ConfigError::Json)
    }

    /// Writes wrapper configuration to a file path.
    pub fn write_to_file<P: AsRef<Path>>(config: &WrapperConfig, path: P) -> Result<(), ConfigError> {
        let file = std::fs::File::create(path.as_ref()).map_err(ConfigError::Io)?;
        Self::write(config, file)
    }
}

/// Builder for creating wrapper directories with executable links and configuration.
///
/// This type acts as a builder that collects executable mappings and creates
/// the necessary directory structure, hard links (or file copies on Windows), and configuration file.
/// It owns and manages the wrapper directory (`.bear/` in the current working directory).
///
/// The use of a deterministic directory path is essential for autotools-style builds
/// where `./configure` caches compiler paths. Using a random temporary directory
/// would break subsequent `make` invocations because the cached paths would no longer exist.
pub struct WrapperDirectoryBuilder {
    wrapper_executable_path: PathBuf,
    wrapper_dir: ManagedDirectory,
    config: WrapperConfig,
}

impl WrapperDirectoryBuilder {
    /// Creates a wrapper directory at `.bear/` in the specified working directory.
    ///
    /// This creates a deterministic directory path that persists across Bear invocations,
    /// which is essential for autotools-style builds. The directory is cleaned up when
    /// Bear exits, but the path remains consistent so cached compiler paths remain valid.
    ///
    /// # Arguments
    ///
    /// * `wrapper_executable_path` - Path to the wrapper executable to link/copy
    /// * `working_dir` - The directory where `.bear/` will be created (typically `context.current_directory`)
    pub fn create(wrapper_executable_path: &Path, working_dir: &Path) -> Result<Self, WrapperDirectoryError> {
        let wrapper_dir =
            ManagedDirectory::create(working_dir).map_err(WrapperDirectoryError::DirCreation)?;

        Ok(Self {
            wrapper_executable_path: wrapper_executable_path.to_path_buf(),
            wrapper_dir,
            config: WrapperConfig::new(),
        })
    }

    /// Registers an executable to be wrapped.
    ///
    /// This method checks for filename uniqueness and creates the hard link (or file copy on Windows) immediately.
    /// Returns the path to the wrapper executable on success, or an error if link creation fails.
    pub fn register_executable(
        &mut self,
        executable_path: PathBuf,
    ) -> Result<PathBuf, WrapperDirectoryError> {
        let executable_name = executable_path
            .file_name()
            .ok_or_else(|| WrapperDirectoryError::InvalidExecutablePath(executable_path.clone()))?
            .to_string_lossy()
            .to_string();

        // Check for filename uniqueness
        if self.config.get_executable(&executable_name).is_some() {
            // Same executable is already registered, return existing wrapper path
            return Ok(self.wrapper_dir.path().join(&executable_name));
        }

        // Create hard link immediately (or copy on Windows/when hard link fails)
        let wrapper_path = self.wrapper_dir.path().join(&executable_name);
        #[cfg(windows)]
        std::fs::copy(&self.wrapper_executable_path, &wrapper_path)
            .map(|_| ())
            .map_err(WrapperDirectoryError::LinkCreation)?;
        #[cfg(not(windows))]
        // Try hard link first, fall back to copy if it fails (e.g., in containers with overlay fs)
        if let Err(hard_link_error) = std::fs::hard_link(&self.wrapper_executable_path, &wrapper_path) {
            log::debug!("Hard link failed ({}), falling back to copy", hard_link_error);
            std::fs::copy(&self.wrapper_executable_path, &wrapper_path)
                .map(|_| ())
                .map_err(WrapperDirectoryError::LinkCreation)?;
        }

        // Register the mapping
        self.config.add_executable(executable_name.clone(), executable_path);

        Ok(wrapper_path)
    }

    /// Finalizes the wrapper directory by writing the configuration file and returns an immutable WrapperDirectory.
    pub fn build(self) -> Result<WrapperDirectory, WrapperDirectoryError> {
        let config_path = self.wrapper_dir.path().join(CONFIG_FILENAME);
        WrapperConfigWriter::write_to_file(&self.config, &config_path)
            .map_err(WrapperDirectoryError::ConfigWrite)?;

        let wrapper_directory = WrapperDirectory { wrapper_dir: self.wrapper_dir, _config: self.config };

        log::info!("{}", wrapper_directory);

        Ok(wrapper_directory)
    }

    /// Gets the path to the wrapper directory.
    pub fn path(&self) -> &Path {
        self.wrapper_dir.path()
    }
}

/// Immutable wrapper directory after build.
pub struct WrapperDirectory {
    wrapper_dir: ManagedDirectory,
    _config: WrapperConfig,
}

impl WrapperDirectory {
    /// Gets the path to the wrapper directory.
    pub fn path(&self) -> &Path {
        self.wrapper_dir.path()
    }

    /// Gets the wrapper config (only available in tests).
    #[cfg(test)]
    pub fn config(&self) -> &WrapperConfig {
        &self._config
    }
}

impl Display for WrapperDirectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Set up wrapper directory: {}", self.wrapper_dir.path().display())?;
        if self._config.executables.is_empty() {
            write!(f, "   No wrappers configured")
        } else {
            writeln!(f, "   Set up wrappers as:")?;
            for (name, path) in self._config.executables.iter() {
                writeln!(f, "     {} -> {}", name, path.display())?;
            }
            Ok(())
        }
    }
}

/// Errors that can occur during wrapper directory operations.
#[derive(Error, Debug)]
pub enum WrapperDirectoryError {
    #[error("Invalid executable path: {0}")]
    InvalidExecutablePath(PathBuf),
    #[error("Failed to create wrapper executable: {0}")]
    LinkCreation(#[from] std::io::Error),
    #[error("Failed to write configuration file: {0}")]
    ConfigWrite(#[from] ConfigError),
    #[error("Failed to create wrapper directory: {0}")]
    DirCreation(std::io::Error),
}

/// Errors that can occur during wrapper configuration operations.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::TempDir;

    #[test]
    fn test_wrapper_config_new() {
        let config = WrapperConfig::new();
        assert!(config.executables.is_empty());
    }

    #[test]
    fn test_wrapper_config_add_executable() {
        let mut config = WrapperConfig::new();
        config.add_executable("gcc".to_string(), PathBuf::from("/usr/bin/gcc"));

        assert_eq!(config.executables.len(), 1);
        assert_eq!(config.get_executable("gcc"), Some(&PathBuf::from("/usr/bin/gcc")));
    }

    #[test]
    fn test_wrapper_config_get_executable() {
        let mut config = WrapperConfig::new();
        config.add_executable("gcc".to_string(), PathBuf::from("/usr/bin/gcc"));
        config.add_executable("g++".to_string(), PathBuf::from("/usr/bin/g++"));

        assert_eq!(config.get_executable("gcc"), Some(&PathBuf::from("/usr/bin/gcc")));
        assert_eq!(config.get_executable("g++"), Some(&PathBuf::from("/usr/bin/g++")));
        assert_eq!(config.get_executable("clang"), None);
    }

    #[test]
    fn test_wrapper_config_reader_write_roundtrip() {
        let mut original_config = WrapperConfig::new();
        original_config.add_executable("gcc".to_string(), PathBuf::from("/usr/bin/gcc"));
        original_config.add_executable("g++".to_string(), PathBuf::from("/usr/bin/g++"));

        // Write to memory buffer
        let mut buffer = Vec::new();
        WrapperConfigWriter::write(&original_config, &mut buffer).unwrap();

        // Read back from memory buffer
        let cursor = Cursor::new(buffer);
        let read_config = WrapperConfigReader::read(cursor).unwrap();

        assert_eq!(original_config, read_config);
    }

    #[test]
    fn test_wrapper_config_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(CONFIG_FILENAME);

        let mut original_config = WrapperConfig::new();
        original_config.add_executable("gcc".to_string(), PathBuf::from("/usr/bin/gcc"));
        original_config.add_executable("clang".to_string(), PathBuf::from("/usr/bin/clang"));

        // Write to file
        WrapperConfigWriter::write_to_file(&original_config, &config_path).unwrap();
        assert!(config_path.exists());

        // Read from file
        let read_config = WrapperConfigReader::read_from_file(&config_path).unwrap();
        assert_eq!(original_config, read_config);
    }

    #[test]
    fn test_wrapper_config_reader_invalid_json() {
        let invalid_json = b"{ invalid json }";
        let cursor = Cursor::new(invalid_json);
        let result = WrapperConfigReader::read(cursor);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::Json(_)));
    }

    #[test]
    fn test_wrapper_config_reader_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent.json");

        let result = WrapperConfigReader::read_from_file(&nonexistent_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::Io(_)));
    }

    #[test]
    fn test_managed_directory_creation() {
        let temp_dir = TempDir::new().unwrap();

        let managed_dir = ManagedDirectory::create(temp_dir.path()).unwrap();

        // Verify directory exists
        assert!(managed_dir.path().exists());
        assert!(managed_dir.path().ends_with(WRAPPER_DIR_NAME));
    }

    #[test]
    fn test_managed_directory_cleanup_on_drop() {
        let temp_dir = TempDir::new().unwrap();

        let path;
        {
            let managed_dir = ManagedDirectory::create(temp_dir.path()).unwrap();
            path = managed_dir.path().to_path_buf();
            assert!(path.exists());
        }
        // After drop, directory should be cleaned up
        assert!(!path.exists());
    }

    #[test]
    fn test_managed_directory_replaces_existing() {
        let temp_dir = TempDir::new().unwrap();

        // Create first managed directory and add a file
        let managed_dir1 = ManagedDirectory::create(temp_dir.path()).unwrap();
        let test_file = managed_dir1.path().join("test_file.txt");
        std::fs::write(&test_file, "test content").unwrap();
        assert!(test_file.exists());

        // Drop the first directory (but pretend we didn't clean up)
        let path = managed_dir1.path().to_path_buf();
        std::mem::forget(managed_dir1); // Simulate crash - don't run drop

        // Verify the directory and file still exist (since we skipped drop)
        assert!(path.exists());
        assert!(test_file.exists());

        // Create second managed directory in the SAME working directory - should replace existing
        let managed_dir2 = ManagedDirectory::create(temp_dir.path()).unwrap();
        assert_eq!(managed_dir2.path(), path);

        // Old file should be gone (directory was replaced with a fresh one)
        assert!(!test_file.exists());
        // But the directory itself should exist (freshly created)
        assert!(managed_dir2.path().exists());
    }

    #[test]
    fn test_wrapper_directory_builder() {
        let temp_dir = TempDir::new().unwrap();
        let wrapper_path = temp_dir.path().join("wrapper");

        // Create wrapper executable
        std::fs::write(&wrapper_path, "#!/bin/bash\necho wrapper").unwrap();

        let mut builder = WrapperDirectoryBuilder::create(&wrapper_path, temp_dir.path()).unwrap();

        // Register some executables
        let gcc_wrapper = builder.register_executable(PathBuf::from("/usr/bin/gcc")).unwrap();
        let gpp_wrapper = builder.register_executable(PathBuf::from("/usr/bin/g++")).unwrap();

        // Save path for later assertions (before build consumes builder)
        let builder_path = builder.path().to_path_buf();

        // Verify links were created
        assert!(builder_path.join("gcc").exists());
        assert!(builder_path.join("g++").exists());
        assert_eq!(gcc_wrapper, builder_path.join("gcc"));
        assert_eq!(gpp_wrapper, builder_path.join("g++"));

        // Verify wrapper directory is in .bear
        assert!(builder_path.ends_with(WRAPPER_DIR_NAME));

        // Finalize and check config file
        let wrapper_dir = builder.build().unwrap();
        let config_path = wrapper_dir.path().join(CONFIG_FILENAME);
        assert!(config_path.exists(), "Config file should exist at {:?}", config_path);
        assert_eq!(wrapper_dir.config().executables.len(), 2);
        assert!(wrapper_dir.config().get_executable("gcc").is_some());
        assert!(wrapper_dir.config().get_executable("g++").is_some());
    }

    #[test]
    fn test_wrapper_directory_duplicate_names() {
        let temp_dir = TempDir::new().unwrap();
        let wrapper_path = temp_dir.path().join("wrapper");

        std::fs::write(&wrapper_path, "#!/bin/bash\necho wrapper").unwrap();

        let mut builder = WrapperDirectoryBuilder::create(&wrapper_path, temp_dir.path()).unwrap();

        // Register first executable
        let _gcc_wrapper = builder.register_executable(PathBuf::from("/usr/bin/gcc")).unwrap();

        // Try to register different executable with same name - should fail
        let result = builder.register_executable(PathBuf::from("/usr/local/bin/gcc"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), _gcc_wrapper);
    }

    #[test]
    fn test_wrapper_directory_same_executable_twice() {
        let temp_dir = TempDir::new().unwrap();
        let wrapper_path = temp_dir.path().join("wrapper");

        std::fs::write(&wrapper_path, "#!/bin/bash\necho wrapper").unwrap();

        let mut builder = WrapperDirectoryBuilder::create(&wrapper_path, temp_dir.path()).unwrap();

        // Register same executable twice - should be OK
        let gcc_path = PathBuf::from("/usr/bin/gcc");
        let wrapper_path1 = builder.register_executable(gcc_path.clone()).unwrap();
        let wrapper_path2 = builder.register_executable(gcc_path).unwrap(); // Should not error
        assert_eq!(wrapper_path1, wrapper_path2); // Should return same wrapper path

        let wrapper_dir = builder.build().unwrap();
        assert_eq!(wrapper_dir.config().executables.len(), 1); // Should only have one entry
    }

    #[test]
    fn test_wrapper_directory_uses_bear_dir() {
        let temp_dir = TempDir::new().unwrap();
        let wrapper_path = temp_dir.path().join("wrapper");
        std::fs::write(&wrapper_path, "#!/bin/bash\necho wrapper").unwrap();

        let mut builder = WrapperDirectoryBuilder::create(&wrapper_path, temp_dir.path()).unwrap();

        let clang_wrapper = builder.register_executable(PathBuf::from("/usr/bin/clang")).unwrap();

        // Verify wrapper is in .bear directory
        assert!(builder.path().ends_with(WRAPPER_DIR_NAME));
        assert!(builder.path().join("clang").exists());
        assert_eq!(clang_wrapper, builder.path().join("clang"));

        let wrapper_dir = builder.build().unwrap();
        assert!(wrapper_dir.path().join(CONFIG_FILENAME).exists());
    }
}
