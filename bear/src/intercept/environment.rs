// SPDX-License-Identifier: GPL-3.0-or-later

#[cfg(not(target_os = "macos"))]
use crate::environment::KEY_OS__PRELOAD_PATH;
use crate::environment::{KEY_INTERCEPT_STATE, KEY_OS__PATH};
#[cfg(target_os = "macos")]
use crate::environment::{KEY_OS__MACOS_FLAT_NAMESPACE, KEY_OS__MACOS_PRELOAD_PATH};
use crate::intercept::supervise;
use crate::semantic::interpreters::compilers::compiler_recognition::CompilerRecognizer;
use crate::{args, config, context};
use std::collections::HashMap;
use std::env::JoinPathsError;
#[cfg(windows)]
use std::env::consts::EXE_EXTENSION;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

use thiserror::Error;

use crate::intercept::wrapper::{WrapperDirectory, WrapperDirectoryBuilder, WrapperDirectoryError};

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

impl TryFrom<String> for PreloadState {
    type Error = serde_json::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&value)
    }
}

/// Manages the environment setup for intercepting build commands during compilation.
///
/// `BuildEnvironment` is responsible for configuring the execution environment to enable
/// Bear's command interception capabilities. It supports two main interception modes:
/// - **Wrapper mode**: Creates a new directory with copies of wrapper executables that
///   can intercept compiler executions, and manipulates the environment variables so
///   that these executables have precedence.
/// - **Preload mode**: Changes the environment variables to inject a dynamic library
///   for system call interception.
pub struct BuildEnvironment {
    environment_overrides: HashMap<String, String>,
    _wrapper_directory: Option<WrapperDirectory>,
}

impl BuildEnvironment {
    /// Creates a new `BuildEnvironment` configured for the specified interception method.
    ///
    /// This method dispatches to the appropriate specialized creation method based on the
    /// configuration type (wrapper or preload mode). In both modes, the interceptor will
    /// report execution events via TCP sockets to the specified address.
    ///
    /// # Arguments
    ///
    /// * `config` - The interception configuration specifying the mode and parameters
    /// * `address` - The socket address where the interceptor should report executions
    ///
    /// # Returns
    ///
    /// Returns a configured `BuildEnvironment` on success, or a `ConfigurationError`
    /// if the configuration is invalid or environment setup fails.
    pub fn create(
        context: &context::Context,
        intercept: &config::Intercept,
        compilers: &[config::Compiler],
        address: SocketAddr,
    ) -> Result<Self, ConfigurationError> {
        match intercept {
            config::Intercept::Wrapper { path, .. } => {
                let executables: Vec<std::path::PathBuf> = compilers
                    .iter()
                    .filter(|compiler| !compiler.ignore)
                    .map(|compiler| compiler.path.clone())
                    .collect();
                Self::create_as_wrapper(context, path, &executables, address)
            }
            config::Intercept::Preload { path } => Self::create_as_preload(context, path, address),
        }
    }

    /// Creates a `BuildEnvironment` configured for wrapper mode interception.
    ///
    /// The wrapper mode is more complicated than preload mode. We create a `.bear/` directory
    /// in the current working directory and insert copies of the wrapper executable. To decide
    /// how to name the executables in the directory, we consider the config file, but also
    /// the current environment variables. Once these are set up, we alter the `PATH` environment
    /// variable to ensure the wrappers are executed instead of the real compilers. To instruct
    /// the wrappers which executables to call, we also create a JSON file.
    ///
    /// The wrapper mode reads the current environment and looks for variables that might
    /// instruct the build process about the compiler locations. Good candidates for such
    /// variables are: `CC`, `CXX`, `LD`, `CPP`, etc.
    ///
    /// An executed wrapper reports the execution and executes the intended process. To
    /// ensure that the wrapper calls the right process, it reads the JSON file from the
    /// wrapper directory, which contains a map of executable names and paths. Using the
    /// arguments (which contain the process name) it looks up the real executable.
    ///
    /// Because this process does not execute the compilers, but the build process does,
    /// we need to alter the build process to use the wrapper executables. This is achieved
    /// by changing the `PATH` variable. But that has limitations; we need to redefine
    /// some of the environment variables mentioned above (`CC`, `CXX`, `LD`, `CPP`, etc.),
    /// allowing the user to use these variables in the build invocations.
    ///
    /// The use of a deterministic `.bear/` directory (instead of a random temp directory)
    /// is essential for autotools-style builds where `./configure` caches compiler paths.
    /// With a random directory, subsequent `make` invocations would fail because the
    /// cached paths would no longer exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the wrapper executable
    /// * `executables` - List of executables to create wrappers for
    /// * `address` - Socket address for interceptor communication
    ///
    /// # Returns
    ///
    /// Returns a configured `BuildEnvironment` on success, or a `ConfigurationError`
    /// if validation fails or wrapper setup encounters errors.
    fn create_as_wrapper(
        context: &context::Context,
        path: &std::path::Path,
        executables: &[std::path::PathBuf],
        address: SocketAddr,
    ) -> Result<Self, ConfigurationError> {
        // Create wrapper directory builder (creates .bear/ in context's working directory)
        let mut wrapper_dir_builder =
            WrapperDirectoryBuilder::create(path, &context.current_directory, address)?;

        // Register executables from config
        for executable in executables {
            wrapper_dir_builder.register_executable(executable.clone())?;
        }

        // Register executables from environment variables that point to compiler programs
        // and immediately update the final environment with wrapper paths
        let mut environment_overrides = HashMap::new();
        for (key, value) in &context.environment {
            if crate::environment::program_env(key) && !value.is_empty() {
                let program_path = std::path::PathBuf::from(value);
                let wrapper_path = wrapper_dir_builder.register_executable(program_path)?;
                // Update the environment overrides with the wrapper path
                environment_overrides.insert(key.clone(), wrapper_path.to_string_lossy().to_string());
            }
        }

        // PATH-based discovery (only if no config executables)
        if executables.is_empty() {
            let compiler_recognizer = CompilerRecognizer::new();
            let predicate = |path: &Path| -> bool { compiler_recognizer.recognize(path).is_some() };

            for candidate in Self::compiler_candidates(context, predicate) {
                wrapper_dir_builder.register_executable(candidate)?;
            }
        }

        // Finalize wrapper directory (writes config file)
        let wrapper_dir = wrapper_dir_builder.build()?;

        // Update PATH environment variable
        if let Some((path_key, path_value)) = context.path() {
            let path_updated =
                insert_to_path(&path_value, wrapper_dir.path()).map_err(ConfigurationError::Path)?;

            environment_overrides.insert(path_key, path_updated);
        } else {
            environment_overrides
                .insert(KEY_OS__PATH.to_string(), wrapper_dir.path().to_string_lossy().to_string());
        }

        Ok(Self { environment_overrides, _wrapper_directory: Some(wrapper_dir) })
    }

    /// Discovers compiler executables in PATH directories using a predicate function.
    ///
    /// This function scans all directories in the PATH environment variable and applies
    /// the provided predicate to each executable file found. Executables that match
    /// the predicate are returned.
    fn compiler_candidates<P>(context: &context::Context, predicate: P) -> impl Iterator<Item = PathBuf>
    where
        P: Fn(&Path) -> bool,
    {
        context
            .paths()
            .into_iter()
            .filter(|dir| dir.exists())
            .flat_map(|dir| match std::fs::read_dir(dir) {
                Ok(entries) => entries
                    .filter_map(|entry| match entry {
                        Ok(e) => Some(e.path()),
                        Err(_) => None,
                    })
                    .collect::<Vec<_>>()
                    .into_iter(),
                Err(e) => {
                    log::debug!("Failed to read directory: {}", e);
                    Vec::new().into_iter()
                }
            })
            .filter(move |path| is_executable_file(path) && predicate(path))
    }

    /// Creates a `BuildEnvironment` configured for preload mode interception.
    ///
    /// The preload mode requires altering one of the environment variables that the OS
    /// dynamic linker reads and loads the shared library into the memory of the executed
    /// process. This will result in all dynamically linked executables reporting back, but
    /// post processing will filter the relevant compiler executions.
    ///
    /// The interceptor will report executions via TCP sockets to the specified address,
    /// which is advertised via a specific environment variable.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the preload library
    /// * `address` - Socket address for interceptor communication
    ///
    /// # Returns
    ///
    /// Returns a configured `BuildEnvironment` on success, or a `ConfigurationError`
    /// if validation fails or the preload library path is invalid.
    fn create_as_preload(
        context: &crate::context::Context,
        path: &std::path::Path,
        address: SocketAddr,
    ) -> Result<Self, ConfigurationError> {
        // Check if preload is supported on this system
        if !context.preload_supported {
            return Err(ConfigurationError::UnsupportedInterceptMode(
                "Preload-based interception is not supported on this system. \
                 This may be due to platform restrictions (e.g., Windows) or \
                 security features (e.g., macOS System Integrity Protection). \
                 Consider using wrapper mode instead.",
            ));
        }
        let mut environment_overrides = HashMap::new();

        // Platform-specific preload configuration
        #[cfg(target_os = "macos")]
        {
            // macOS uses DYLD_INSERT_LIBRARIES and DYLD_FORCE_FLAT_NAMESPACE
            let preload_original =
                context.environment.get(KEY_OS__MACOS_PRELOAD_PATH).cloned().unwrap_or_default();
            let preload_updated =
                insert_to_path(&preload_original, path).map_err(ConfigurationError::Path)?;

            environment_overrides.insert(KEY_OS__MACOS_PRELOAD_PATH.to_string(), preload_updated);
            environment_overrides.insert(KEY_OS__MACOS_FLAT_NAMESPACE.to_string(), "1".to_string());
        }
        #[cfg(not(target_os = "macos"))]
        {
            // Linux and other Unix-like systems use LD_PRELOAD
            let preload_original = context.environment.get(KEY_OS__PRELOAD_PATH).cloned().unwrap_or_default();
            let preload_updated =
                insert_to_path(&preload_original, path).map_err(ConfigurationError::Path)?;

            environment_overrides.insert(KEY_OS__PRELOAD_PATH.to_string(), preload_updated);
        }

        // Make the current state available as a single environment variable
        let state: String = PreloadState { destination: address, library: path.to_path_buf() }
            .try_into()
            .map_err(|_| ConfigurationError::PathNotFound)?;
        environment_overrides.insert(KEY_INTERCEPT_STATE.to_string(), state);

        Ok(Self { environment_overrides, _wrapper_directory: None })
    }

    /// Executes a build command within the configured interception environment.
    ///
    /// This is the main entry point for running build commands with Bear's interception
    /// capabilities enabled. The method sets up the command with the proper environment
    /// and delegates execution to the supervision system for monitoring and control.
    ///
    /// # Arguments
    ///
    /// * `build_command` - The build command to execute with interception enabled
    ///
    /// # Returns
    ///
    /// Returns `Ok(ExitStatus)` containing the exit status of the build command,
    /// or `Err(SuperviseError)` if the command execution fails or cannot be supervised.
    pub fn run_build(
        &self,
        build_command: args::BuildCommand,
    ) -> Result<ExitStatus, supervise::SuperviseError> {
        log::info!("Build command to run: {build_command:?}");

        let [executable, args @ ..] = build_command.arguments.as_slice() else {
            // Safe to presume that the build command was already checked.
            panic!("BuildCommand arguments cannot be empty");
        };

        let mut command = std::process::Command::new(executable);
        command.args(args);

        // Set only the environment variables we need to override
        for (key, value) in &self.environment_overrides {
            log::info!("Build command environment override: {key}={value}");
            command.env(key, value);
        }

        supervise::supervise(&mut command)
    }
}

/// Error types that can occur during build environment configuration.
///
/// This enum represents the various errors that can happen when setting up
/// the build environment for command interception. Each variant provides
/// specific context about what went wrong during the configuration process.
#[derive(Error, Debug)]
pub enum ConfigurationError {
    #[error("Invalid characters in path to join: {0}")]
    Path(#[from] JoinPathsError),
    #[error("Wrapper directory error: {0}")]
    WrapperDirectory(#[from] WrapperDirectoryError),
    #[error("Could not find PATH variable")]
    PathNotFound,
    #[error("Unsupported intercept mode: {0}")]
    UnsupportedInterceptMode(&'static str),
}

/// Manipulates a `PATH`-like environment variable by inserting a path at the beginning.
///
/// This function ensures that the specified path appears first in a colon-separated
/// list of paths (like `PATH` or `LD_PRELOAD`). If the path already exists elsewhere
/// in the list, it is removed from its current position and moved to the front.
/// This guarantees that the specified path takes precedence over other paths.
///
/// # Arguments
///
/// * `original` - The original PATH-like environment variable value
/// * `first` - The path to insert at the beginning of the path list
///
/// # Returns
///
/// Returns `Ok(String)` containing the updated path list, or `Err(ConfigurationError)`
/// if path manipulation fails due to invalid characters or platform limitations.
///
/// # Behavior
///
/// - If `original` is empty, returns just the `first` path
/// - If `first` already exists in `original`, it's moved to the front
/// - If `first` doesn't exist, it's prepended to the existing paths
/// - Uses platform-appropriate path separators and handles path encoding
pub fn insert_to_path<P: AsRef<Path>>(original: &str, first: P) -> Result<String, JoinPathsError> {
    let first_path = first.as_ref();

    if original.is_empty() {
        return Ok(first_path.to_string_lossy().to_string());
    }

    let mut paths: Vec<PathBuf> =
        std::env::split_paths(original).filter(|path| path.as_path() != first_path).collect();
    paths.insert(0, first_path.to_owned());
    std::env::join_paths(paths).map(|os_string| os_string.into_string().unwrap_or_default())
}

/// Checks if a path represents an executable file.
fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    // Check if file is executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = path.metadata() { metadata.permissions().mode() & 0o111 != 0 } else { false }
    }

    #[cfg(not(unix))]
    {
        // On Windows, assume .exe files are executable, others might be scripts
        path.extension().map_or(false, |ext| ext == EXE_EXTENSION)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr};
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Helper function to assert that the first entry in a path-like string equals the expected value.
    /// This can be used to assert PATH, LD_PRELOAD, or any other path-separated environment variable.
    fn assert_first_path_entry(expected: &str, path_like: &str) {
        let path_entries: Vec<String> =
            std::env::split_paths(path_like).map(|p| p.to_string_lossy().to_string()).collect();
        let first_entry = path_entries.first().expect("Path-like string should not be empty");

        assert_eq!(
            first_entry, expected,
            "First path entry should match expected. First entry: {}, expected: {}",
            first_entry, expected
        );
    }

    fn assert_path_entry(expected: &str, path_like: &str) {
        let path_entries: Vec<String> =
            std::env::split_paths(path_like).map(|p| p.to_string_lossy().to_string()).collect();

        assert!(
            path_entries.contains(&expected.to_string()),
            "Path entry should contain expected. Path entries: {:?}, expected: {}",
            path_entries,
            expected
        );
    }

    /// Helper function to get the expected wrapper directory path from a TempDir.
    fn get_wrapper_dir_path(temp_dir: &TempDir) -> String {
        temp_dir.path().join(".bear").to_string_lossy().to_string()
    }

    #[test]
    fn test_insert_to_path_empty_original() {
        let original = "";
        let first = PathBuf::from("/usr/local/bin");
        let result = insert_to_path(original, first.clone()).unwrap();
        // For empty path case, we just return the path as a string
        assert_first_path_entry(&first.to_string_lossy(), &result);
    }

    #[test]
    fn test_insert_to_path_prepend_new() {
        let bin = PathBuf::from("/bin");
        let usr_bin = PathBuf::from("/usr/bin");
        let usr_local_bin = PathBuf::from("/usr/local/bin");

        // Join the original paths using platform-specific separator
        let original =
            std::env::join_paths([usr_bin.clone(), bin.clone()]).unwrap().to_string_lossy().to_string();

        // Apply our function
        let result = insert_to_path(&original, usr_local_bin.clone()).unwrap();

        // Check that the new path is first
        assert_first_path_entry(&usr_local_bin.to_string_lossy(), &result);
        assert_path_entry(&bin.to_string_lossy(), &result);
        assert_path_entry(&usr_bin.to_string_lossy(), &result);
    }

    #[test]
    fn test_insert_to_path_move_existing_to_front() {
        let bin = PathBuf::from("/bin");
        let usr_bin = PathBuf::from("/usr/bin");
        let usr_local_bin = PathBuf::from("/usr/local/bin");

        // Join the original paths using platform-specific separator
        let original = std::env::join_paths([usr_bin.clone(), usr_local_bin.clone(), bin.clone()])
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Apply our function
        let result = insert_to_path(&original, usr_local_bin.clone()).unwrap();

        // Check that the existing path was moved to front
        assert_first_path_entry(&usr_local_bin.to_string_lossy(), &result);
        assert_path_entry(&bin.to_string_lossy(), &result);
        assert_path_entry(&usr_bin.to_string_lossy(), &result);
    }

    #[test]
    fn test_insert_to_path_already_first() {
        let bin = PathBuf::from("/bin");
        let usr_bin = PathBuf::from("/usr/bin");
        let usr_local_bin = PathBuf::from("/usr/local/bin");

        // Join the original paths using platform-specific separator
        let original = std::env::join_paths([usr_local_bin.clone(), usr_bin.clone(), bin.clone()])
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Apply our function
        let result = insert_to_path(&original, usr_local_bin.clone()).unwrap();

        // Check that the path is still first (no change needed)
        assert_first_path_entry(&usr_local_bin.to_string_lossy(), &result);
        assert_path_entry(&bin.to_string_lossy(), &result);
        assert_path_entry(&usr_bin.to_string_lossy(), &result);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_insert_to_path_windows_mingw_preservation() {
        // Test the exact Windows CI failure scenario - MinGW PATH preservation
        let original = "C:\\mingw64\\bin;C:\\Windows\\System32;C:\\Program Files\\Git\\bin";
        let wrapper_dir = "C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\bear-xyz";
        let first = PathBuf::from(wrapper_dir);

        let result = insert_to_path(original, first).unwrap();

        // Wrapper should be first in PATH
        assert_first_path_entry(wrapper_dir, &result);
        assert_path_entry("C:\\mingw64\\bin", &result);
        assert_path_entry("C:\\Windows\\System32", &result);
        assert_path_entry("C:\\Program Files\\Git\\bin", &result);
    }

    #[test]
    fn test_build_environment_create_preload() {
        let preload_path = "/usr/local/lib/libexec.so";

        let sut = {
            let context = {
                let environment = {
                    let mut builder = HashMap::new();
                    builder.insert(KEY_OS__PATH.to_string(), "/usr/bin:/bin".to_string());
                    builder.insert("CC".to_string(), "/usr/bin/gcc".to_string());
                    builder.insert("CXX".to_string(), "/usr/bin/g++".to_string());
                    builder
                };

                crate::context::Context {
                    current_executable: PathBuf::from("/usr/bin/bear"),
                    current_directory: PathBuf::from("/tmp"),
                    environment,
                    preload_supported: true,
                }
            };
            let intercept = config::Intercept::Preload { path: PathBuf::from(preload_path) };
            let compilers = vec![];
            let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

            BuildEnvironment::create(&context, &intercept, &compilers, address).unwrap()
        };

        // Check that destination is set
        assert_eq!(
            sut.environment_overrides.get(KEY_INTERCEPT_STATE),
            Some(&r#"{"destination":"127.0.0.1:8080","library":"/usr/local/lib/libexec.so"}"#.to_string())
        );

        // Check platform-specific preload configuration
        #[cfg(target_os = "macos")]
        {
            // Check that DYLD_INSERT_LIBRARIES contains our library
            let dyld_preload = sut.environment_overrides.get(KEY_OS__MACOS_PRELOAD_PATH).unwrap();
            assert_first_path_entry(preload_path, dyld_preload);

            // Check that DYLD_FORCE_FLAT_NAMESPACE is set to "1"
            assert_eq!(sut.environment_overrides.get(KEY_OS__MACOS_FLAT_NAMESPACE), Some(&"1".to_string()));
        }
        #[cfg(not(target_os = "macos"))]
        {
            // Check that LD_PRELOAD contains our library
            let ld_preload = sut.environment_overrides.get(KEY_OS__PRELOAD_PATH).unwrap();
            assert_first_path_entry(preload_path, ld_preload);
        }

        assert!(sut._wrapper_directory.is_none());
    }

    #[test]
    fn test_wrapper_environment_path_setting_when_it_was_empty_before() {
        let temp_dir = TempDir::new().unwrap();
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        let sut = {
            let wrapper_path = {
                let file = temp_dir.path().join("wrapper");
                std::fs::write(&file, "#!/bin/bash\necho wrapper").unwrap();
                file
            };

            let intercept = config::Intercept::Wrapper { path: wrapper_path.clone() };
            let compilers = vec![];
            let context = {
                crate::context::Context {
                    current_executable: PathBuf::from("/usr/bin/bear"),
                    current_directory: temp_dir.path().to_path_buf(),
                    environment: HashMap::new(),
                    preload_supported: false,
                }
            };

            BuildEnvironment::create(&context, &intercept, &compilers, address).unwrap()
        };

        // Check that PATH is updated (should contain .bear directory at the beginning)
        let path = sut.environment_overrides.get("PATH").unwrap();
        let expected_wrapper_path = get_wrapper_dir_path(&temp_dir);
        assert_first_path_entry(&expected_wrapper_path, path);
    }

    #[test]
    fn test_build_environment_create_wrapper() {
        let temp_dir = TempDir::new().unwrap();
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        let sut = {
            let wrapper_path = {
                let file = temp_dir.path().join("wrapper");
                std::fs::write(&file, "#!/bin/bash\necho wrapper").unwrap();
                file
            };
            let intercept = config::Intercept::Wrapper { path: wrapper_path };
            let compilers = vec![
                config::Compiler { path: PathBuf::from("/usr/bin/cc"), as_: None, ignore: false },
                config::Compiler { path: PathBuf::from("/usr/bin/clang"), as_: None, ignore: false },
            ];
            let context = {
                let environment = {
                    let mut builder = HashMap::new();
                    builder.insert(KEY_OS__PATH.to_string(), "/usr/bin:/bin".to_string());
                    builder.insert("CC".to_string(), "/usr/bin/gcc".to_string());
                    builder.insert("CXX".to_string(), "/usr/bin/g++".to_string());
                    builder
                };

                crate::context::Context {
                    current_executable: PathBuf::from("/usr/bin/bear"),
                    current_directory: temp_dir.path().to_path_buf(),
                    environment,
                    preload_supported: true,
                }
            };

            BuildEnvironment::create(&context, &intercept, &compilers, address).unwrap()
        };

        // Check that PATH is updated (should contain .bear directory at the beginning)
        let path = sut.environment_overrides.get("PATH").unwrap();
        let expected_wrapper_path = get_wrapper_dir_path(&temp_dir);
        assert_first_path_entry(&expected_wrapper_path, path);

        // Verify wrapper directory is kept alive
        assert!(sut._wrapper_directory.is_some());
        let wrapper_dir = sut._wrapper_directory.as_ref().expect("Wrapper directory should exist");
        let wrapper_config = wrapper_dir.config();

        // Check that collector address is in the config
        assert_eq!(wrapper_config.collector_address, address);

        // Check that compilers configured are in the wrapper config
        assert_eq!("/usr/bin/cc", wrapper_config.get_executable("cc").unwrap()); // this is from the config
        assert_eq!("/usr/bin/clang", wrapper_config.get_executable("clang").unwrap()); // this if from the config
        assert_eq!("/usr/bin/gcc", wrapper_config.get_executable("gcc").unwrap()); // this is from context
        assert_eq!("/usr/bin/g++", wrapper_config.get_executable("g++").unwrap()); // this is from context

        // Check that environment variables were redirected to wrapper executables
        let cc_value = sut.environment_overrides.get("CC").unwrap();
        let cxx_value = sut.environment_overrides.get("CXX").unwrap();
        assert!(cc_value.contains(".bear"), "CC should point to wrapper: {}", cc_value);
        assert!(cxx_value.contains(".bear"), "CXX should point to wrapper: {}", cxx_value);
    }

    #[test]
    fn test_path_discovery_with_empty_executables() {
        use std::fs;

        // Create a temp directory for this test
        let temp_dir = TempDir::new().unwrap();

        // Create a directory with mock compilers
        let bin_dir = {
            let bin_dir = temp_dir.path().join("bin");
            fs::create_dir(&bin_dir).unwrap();

            // Create mock compiler executables
            #[cfg(unix)]
            let compilers = ["gcc", "g++", "clang", "notacompiler"];
            #[cfg(not(unix))]
            let compilers = ["gcc.exe", "g++.exe", "clang.exe", "notacompiler"];

            for compiler in &compilers {
                let compiler_path = bin_dir.join(compiler);
                fs::write(&compiler_path, "#!/bin/sh\necho mock compiler").unwrap();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&compiler_path, fs::Permissions::from_mode(0o755)).unwrap();
                }
            }
            bin_dir
        };

        let sut = {
            let context = {
                let environment = {
                    let mut builder = HashMap::new();
                    builder.insert("PATH".to_string(), bin_dir.to_string_lossy().to_string());
                    builder
                };

                crate::context::Context {
                    current_executable: PathBuf::from("/usr/bin/bear"),
                    current_directory: temp_dir.path().to_path_buf(),
                    environment,
                    preload_supported: true,
                }
            };

            let wrapper_exe = temp_dir.path().join("bear-wrapper");
            fs::write(&wrapper_exe, "wrapper").unwrap();

            let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

            BuildEnvironment::create_as_wrapper(
                &context,
                &wrapper_exe,
                &[], // Empty executables - should trigger PATH discovery
                address,
            )
        };

        assert!(sut.is_ok(), "PATH discovery should succeed");
        let build_env = sut.unwrap();

        // Verify that the PATH was updated to include the wrapper directory
        let path = build_env.environment_overrides.get("PATH").unwrap();
        let expected_wrapper_path = get_wrapper_dir_path(&temp_dir);
        assert_first_path_entry(&expected_wrapper_path, path);

        // Verify that compilers were discovered and registered in the config
        let wrapper_dir = build_env._wrapper_directory.as_ref().expect("Wrapper directory should exist");
        let wrapper_config = wrapper_dir.config();

        // Should have discovered gcc, g++, and clang from PATH (but not notacompiler)
        #[cfg(unix)]
        assert!(wrapper_config.get_executable("gcc").is_some(), "gcc should be discovered");
        #[cfg(unix)]
        assert!(wrapper_config.get_executable("g++").is_some(), "g++ should be discovered");
        #[cfg(unix)]
        assert!(wrapper_config.get_executable("clang").is_some(), "clang should be discovered");
        #[cfg(not(unix))]
        assert!(wrapper_config.get_executable("gcc.exe").is_some(), "gcc should be discovered");
        #[cfg(not(unix))]
        assert!(wrapper_config.get_executable("g++.exe").is_some(), "g++ should be discovered");
        #[cfg(not(unix))]
        assert!(wrapper_config.get_executable("clang.exe").is_some(), "clang should be discovered");
        assert!(
            wrapper_config.get_executable("notacompiler").is_none(),
            "notacompiler should not be registered"
        );
    }

    #[test]
    fn test_path_discovery_skipped_with_executables() {
        use std::fs;

        // Create a temp directory for this test
        let temp_dir = TempDir::new().unwrap();

        // Create a directory with a compiler
        let bin_dir = {
            let bin_dir = temp_dir.path().join("bin");
            fs::create_dir(&bin_dir).unwrap();

            let gcc_path = bin_dir.join("gcc");
            fs::write(&gcc_path, "#!/bin/sh\necho gcc").unwrap();

            bin_dir
        };

        // Use temp_dir as working directory through context
        let sut = {
            let context = {
                let environment = {
                    let mut builder = std::collections::HashMap::new();
                    builder.insert("PATH".to_string(), bin_dir.to_string_lossy().to_string());
                    builder
                };

                crate::context::Context {
                    current_executable: std::path::PathBuf::from("/usr/bin/bear"),
                    current_directory: temp_dir.path().to_path_buf(),
                    environment,
                    preload_supported: true,
                }
            };

            let wrapper_exe = temp_dir.path().join("bear-wrapper");
            fs::write(&wrapper_exe, "wrapper").unwrap();

            let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

            let custom_compiler =
                config::Compiler { path: PathBuf::from("/usr/bin/custom-gcc"), as_: None, ignore: false };

            BuildEnvironment::create(
                &context,
                &config::Intercept::Wrapper { path: wrapper_exe },
                &[custom_compiler],
                address,
            )
        };

        assert!(sut.is_ok(), "Should succeed with explicit executables");

        // Verify that only the explicit executable is registered (no PATH discovery)
        let build_env = sut.unwrap();
        let wrapper_dir = build_env._wrapper_directory.as_ref().expect("Wrapper directory should exist");
        let wrapper_config = wrapper_dir.config();

        // Should have custom-gcc registered
        assert!(wrapper_config.get_executable("custom-gcc").is_some(), "custom-gcc should be registered");

        // Should NOT have gcc from PATH (PATH discovery should be skipped)
        // The gcc in bin_dir should not be discovered because we provided explicit executables
        assert!(wrapper_config.get_executable("gcc").is_none(), "gcc from PATH should not be discovered");
    }
}
