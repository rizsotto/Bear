// SPDX-License-Identifier: GPL-3.0-or-later

use crate::environment::{KEY_DESTINATION, KEY_OS__PATH, KEY_OS__PRELOAD_PATH};
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
            config::Intercept::Wrapper { path, directory } => {
                let executables: Vec<std::path::PathBuf> = compilers
                    .iter()
                    .filter(|compiler| !compiler.ignore)
                    .map(|compiler| compiler.path.clone())
                    .collect();
                Self::create_as_wrapper(context, path, directory, &executables, address)
            }
            config::Intercept::Preload { path } => Self::create_as_preload(context, path, address),
        }
    }

    /// Creates a `BuildEnvironment` configured for wrapper mode interception.
    ///
    /// The wrapper mode is more complicated than preload mode. We create a temporary directory
    /// and insert copies of the wrapper executable. To decide how to name the executables
    /// in the directory, we consider the config file, but also the current environment
    /// variables. Once these are set up, we alter the `PATH` environment variable to ensure the
    /// wrappers are executed instead of the real compilers. To instruct the wrappers which
    /// executables to call, we also create a JSON file.
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
    /// # Arguments
    ///
    /// * `path` - Path to the wrapper executable
    /// * `directory` - Directory where wrapper copies will be created
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
        directory: &std::path::Path,
        executables: &[std::path::PathBuf],
        address: SocketAddr,
    ) -> Result<Self, ConfigurationError> {
        // Create wrapper directory builder
        let mut wrapper_dir_builder = WrapperDirectoryBuilder::create(path, directory)?;

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
        environment_overrides.insert(KEY_DESTINATION.to_string(), address.to_string());

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
            .inspect(|path| log::debug!("Found compiler candidate: {}", path.display()))
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
        // Update LD_PRELOAD environment variable
        let preload_original = context.environment.get(KEY_OS__PRELOAD_PATH).cloned().unwrap_or_default();
        let preload_updated = insert_to_path(&preload_original, path).map_err(ConfigurationError::Path)?;

        let mut environment_overrides = HashMap::new();
        environment_overrides.insert(KEY_OS__PRELOAD_PATH.to_string(), preload_updated);
        environment_overrides.insert(KEY_DESTINATION.to_string(), address.to_string());

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
fn insert_to_path<P: AsRef<Path>>(original: &str, first: P) -> Result<String, JoinPathsError> {
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

    fn create_test_context() -> crate::context::Context {
        let mut environment = HashMap::new();
        environment.insert(KEY_OS__PATH.to_string(), "/usr/bin:/bin".to_string());
        environment.insert(KEY_OS__PRELOAD_PATH.to_string(), "".to_string());
        environment.insert("CC".to_string(), "/usr/bin/gcc".to_string());
        environment.insert("CXX".to_string(), "/usr/bin/g++".to_string());

        crate::context::Context {
            current_executable: PathBuf::from("/usr/bin/bear"),
            current_directory: PathBuf::from("/tmp"),
            environment,
        }
    }

    #[test]
    fn test_insert_to_path_empty_original() {
        let original = "";
        let first = PathBuf::from("/usr/local/bin");
        let result = insert_to_path(original, first.clone()).unwrap();
        // For empty path case, we just return the path as a string
        assert_eq!(result, first.to_string_lossy());
    }

    #[test]
    fn test_insert_to_path_prepend_new() {
        // Create platform-independent paths using std::env functions
        let bin = PathBuf::from("/bin");
        let usr_bin = PathBuf::from("/usr/bin");
        let usr_local_bin = PathBuf::from("/usr/local/bin");

        // Join the original paths using platform-specific separator
        let original =
            std::env::join_paths([usr_bin.clone(), bin.clone()]).unwrap().to_string_lossy().to_string();

        // Apply our function
        let result = insert_to_path(&original, usr_local_bin.clone()).unwrap();

        // Create expected result using platform-specific separator
        let expected =
            std::env::join_paths([usr_local_bin, usr_bin, bin]).unwrap().to_string_lossy().to_string();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_insert_to_path_move_existing_to_front() {
        // Create platform-independent paths using std::env functions
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

        // Create expected result using platform-specific separator
        let expected =
            std::env::join_paths([usr_local_bin, usr_bin, bin]).unwrap().to_string_lossy().to_string();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_insert_to_path_already_first() {
        // Create platform-independent paths using std::env functions
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

        assert_eq!(result, original);
    }

    #[test]
    fn test_build_environment_create_preload() {
        let intercept = config::Intercept::Preload { path: PathBuf::from("/usr/local/lib/libintercept.so") };
        let compilers = vec![];
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        let context = create_test_context();
        let env = BuildEnvironment::create(&context, &intercept, &compilers, address).unwrap();

        // Check that destination is set
        assert_eq!(env.environment_overrides.get(KEY_DESTINATION), Some(&"127.0.0.1:8080".to_string()));

        // Check that LD_PRELOAD contains our library
        let ld_preload = env.environment_overrides.get(KEY_OS__PRELOAD_PATH).unwrap();
        assert!(ld_preload.starts_with("/usr/local/lib/libintercept.so"));
    }

    #[test]
    fn test_build_environment_create_wrapper() {
        use tempfile::TempDir;

        // Create a temporary directory for the test
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a dummy wrapper executable
        let wrapper_path = temp_path.join("wrapper");
        std::fs::write(&wrapper_path, "#!/bin/bash\necho wrapper").unwrap();

        let intercept =
            config::Intercept::Wrapper { path: wrapper_path.clone(), directory: temp_path.to_path_buf() };
        let compilers = vec![
            config::Compiler { path: PathBuf::from("/usr/bin/gcc"), as_: None, ignore: false },
            config::Compiler { path: PathBuf::from("/usr/bin/clang"), as_: None, ignore: false },
        ];
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        let context = create_test_context();
        let env = BuildEnvironment::create(&context, &intercept, &compilers, address).unwrap();

        // Check that destination is set
        assert_eq!(env.environment_overrides.get(KEY_DESTINATION), Some(&"127.0.0.1:8080".to_string()));

        // Check that PATH is updated (should contain our temp directory at the beginning)
        let path = env.environment_overrides.get("PATH").unwrap();
        assert!(path.contains(&"bear-".to_string()), "PATH should contain bear temp directory: {path}");

        // Verify wrapper directory is kept alive
        assert!(env._wrapper_directory.is_some());
    }

    #[test]
    fn test_build_environment_create_wrapper_with_env_vars() {
        use tempfile::TempDir;

        // Clean up any existing environment variables first
        unsafe {
            std::env::remove_var("CC");
            std::env::remove_var("CXX");
        }

        // Create a temporary directory for the test
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a dummy wrapper executable
        let wrapper_path = temp_path.join("wrapper");
        std::fs::write(&wrapper_path, "#!/bin/bash\necho wrapper").unwrap();

        // Set up environment variables that should be detected
        unsafe {
            std::env::set_var("CC", "/usr/bin/gcc");
            std::env::set_var("CXX", "/usr/bin/g++");
        }

        let intercept = config::Intercept::Wrapper { path: wrapper_path, directory: temp_path.to_path_buf() };
        let compilers =
            vec![config::Compiler { path: PathBuf::from("/usr/bin/clang"), as_: None, ignore: false }];
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        let context = create_test_context();
        let env = BuildEnvironment::create(&context, &intercept, &compilers, address).unwrap();

        // Get the wrapper directory from the BuildEnvironment
        let wrapper_dir = env._wrapper_directory.as_ref().expect("Wrapper directory should exist");
        let config_path = wrapper_dir.path().join(crate::intercept::wrapper::CONFIG_FILENAME);
        assert!(config_path.exists(), "JSON config file should exist");

        // Read and verify JSON config content
        let wrapper_config =
            crate::intercept::wrapper::WrapperConfigReader::read_from_file(&config_path).unwrap();

        // Should contain executables from both config and environment variables
        assert!(wrapper_config.get_executable("clang").is_some());
        assert!(wrapper_config.get_executable("gcc").is_some());
        assert!(wrapper_config.get_executable("g++").is_some());

        // Check that environment variables were redirected to wrapper executables
        let cc_value = env.environment_overrides.get("CC").unwrap();
        let cxx_value = env.environment_overrides.get("CXX").unwrap();
        assert!(cc_value.contains("bear-"), "CC should point to wrapper: {}", cc_value);
        assert!(cxx_value.contains("bear-"), "CXX should point to wrapper: {}", cxx_value);

        // Clean up environment variables
        unsafe {
            std::env::remove_var("CC");
            std::env::remove_var("CXX");
        }
    }

    #[test]
    fn test_path_discovery_with_empty_executables() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory with mock compilers
        let temp_dir = TempDir::new().unwrap();
        let bin_dir = temp_dir.path().join("bin");
        fs::create_dir(&bin_dir).unwrap();

        // Create mock compiler executables
        let compilers = ["gcc", "g++", "clang", "notacompiler"];
        for compiler in &compilers {
            let compiler_path = bin_dir.join(compiler);
            fs::write(&compiler_path, "#!/bin/sh\necho mock compiler").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&compiler_path, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }

        // Create test context with custom PATH
        let mut env = std::collections::HashMap::new();
        env.insert("PATH".to_string(), bin_dir.to_string_lossy().to_string());

        let context = crate::context::Context {
            current_executable: std::path::PathBuf::from("/usr/bin/bear"),
            current_directory: std::path::PathBuf::from("/tmp"),
            environment: env,
        };

        // Test with empty executables - should trigger PATH discovery
        let wrapper_dir = TempDir::new().unwrap();
        let wrapper_exe = wrapper_dir.path().join("bear-wrapper");
        fs::write(&wrapper_exe, "wrapper").unwrap();

        let address = "127.0.0.1:12345".parse().unwrap();

        let result = BuildEnvironment::create_as_wrapper(
            &context,
            &wrapper_exe,
            wrapper_dir.path(),
            &[], // Empty executables - should trigger PATH discovery
            address,
        );

        assert!(result.is_ok(), "PATH discovery should succeed");

        // Verify that the PATH was updated to include the wrapper directory
        let build_env = result.unwrap();
        let updated_path = build_env.environment_overrides.get("PATH").unwrap();
        assert!(updated_path.contains("bear-"), "PATH should contain wrapper directory");
    }

    #[test]
    fn test_path_discovery_skipped_with_executables() {
        use std::fs;
        use tempfile::TempDir;

        // Create test context with PATH containing compilers
        let temp_dir = TempDir::new().unwrap();
        let bin_dir = temp_dir.path().join("bin");
        fs::create_dir(&bin_dir).unwrap();

        let gcc_path = bin_dir.join("gcc");
        fs::write(&gcc_path, "#!/bin/sh\necho gcc").unwrap();

        let mut env = std::collections::HashMap::new();
        env.insert("PATH".to_string(), bin_dir.to_string_lossy().to_string());

        let context = crate::context::Context {
            current_executable: std::path::PathBuf::from("/usr/bin/bear"),
            current_directory: std::path::PathBuf::from("/tmp"),
            environment: env,
        };

        // Test with non-empty executables - should skip PATH discovery
        let wrapper_dir = TempDir::new().unwrap();
        let wrapper_exe = wrapper_dir.path().join("bear-wrapper");
        fs::write(&wrapper_exe, "wrapper").unwrap();

        let address = "127.0.0.1:12345".parse().unwrap();
        let custom_compiler = std::path::PathBuf::from("/usr/bin/custom-gcc");

        let result = BuildEnvironment::create_as_wrapper(
            &context,
            &wrapper_exe,
            wrapper_dir.path(),
            &[custom_compiler], // Non-empty executables - should skip PATH discovery
            address,
        );

        assert!(result.is_ok(), "Should succeed with explicit executables");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_insert_to_path_windows_mingw_preservation() {
        // Test the exact Windows CI failure scenario - MinGW PATH preservation
        let original = "C:\\mingw64\\bin;C:\\Windows\\System32;C:\\Program Files\\Git\\bin";
        let wrapper_dir = "C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\bear-xyz";
        let first = PathBuf::from(wrapper_dir);

        let result = insert_to_path(original, first).unwrap();

        // Critical assertion: MinGW path must be preserved for gcc.exe to be found
        assert!(
            result.contains("mingw64\\bin"),
            "MinGW path should be preserved so gcc.exe can be found. Original: {}, Result: {}",
            original,
            result
        );

        // Wrapper should be first in PATH
        assert!(
            result.starts_with(wrapper_dir),
            "Wrapper directory should be first in PATH. Result: {}",
            result
        );

        // Should have all original paths plus wrapper (4 total)
        let path_count = result.split(';').filter(|s| !s.is_empty()).count();
        assert_eq!(path_count, 4, "Should preserve all original paths plus wrapper, got: {}", path_count);

        println!("Windows PATH preservation test passed. Result: {}", result);
    }
}
