// SPDX-License-Identifier: GPL-3.0-or-later

#[cfg(not(target_os = "macos"))]
use crate::environment::KEY_OS__PRELOAD_PATH;
use crate::environment::{KEY_INTERCEPT_STATE, KEY_OS__PATH};
#[cfg(target_os = "macos")]
use crate::environment::{KEY_OS__MACOS_FLAT_NAMESPACE, KEY_OS__MACOS_PRELOAD_PATH};
use crate::installation::InstallationLayout;
use crate::intercept::supervise;
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
        is_compiler: impl Fn(&Path) -> bool,
    ) -> Result<Self, ConfigurationError> {
        match intercept {
            config::Intercept::Wrapper => {
                let executables: Vec<std::path::PathBuf> = compilers
                    .iter()
                    .filter(|compiler| !compiler.ignore)
                    .map(|compiler| compiler.path.clone())
                    .collect();
                Self::create_as_wrapper(context, &executables, address, is_compiler)
            }
            config::Intercept::Preload => Self::create_as_preload(context, address),
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
        executables: &[std::path::PathBuf],
        address: SocketAddr,
        is_compiler: impl Fn(&Path) -> bool,
    ) -> Result<Self, ConfigurationError> {
        let layout = InstallationLayout::try_from(context.current_executable.as_path())
            .map_err(ConfigurationError::InstallationLayout)?;
        let wrapper_executable = layout.wrapper_path();
        // Create wrapper directory builder (creates .bear/ in context's working directory)
        let mut wrapper_dir_builder = WrapperDirectoryBuilder::create(
            wrapper_executable.as_path(),
            &context.current_directory,
            address,
        )?;

        // Register executables from config
        for executable in executables {
            wrapper_dir_builder.register_executable(executable.clone())?;
        }

        // Register executables from environment variables that point to compiler programs
        // and immediately update the final environment with wrapper paths.
        // Bare names (e.g. CC=gcc) are resolved via PATH before registration.
        let mut environment_overrides = HashMap::new();
        for (key, value) in &context.environment {
            if crate::environment::program_env(key) && !value.is_empty() {
                if let Some(program_path) = Self::resolve_program_path(context, value) {
                    let wrapper_path = wrapper_dir_builder.register_executable(program_path)?;
                    environment_overrides.insert(key.clone(), wrapper_path.to_string_lossy().to_string());
                } else {
                    log::warn!(
                        "Skipping compiler env var {}={}: could not resolve to an executable on PATH",
                        key,
                        value,
                    );
                }
            }
        }

        // PATH-based discovery (only if no config executables)
        if executables.is_empty() {
            for candidate in Self::compiler_candidates(context, is_compiler) {
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
    /// the predicate are returned. Masquerade wrappers (ccache, distcc, ...) are
    /// filtered out so that the registered compiler is always the real one; the
    /// iteration continues past them to find the next candidate on PATH.
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
            .filter(move |path| is_executable_file(path) && !is_masquerade_wrapper(path) && predicate(path))
    }

    /// Resolves a program env var value to an absolute executable path.
    ///
    /// Handles three cases:
    /// - Absolute path: returned as-is (not canonicalized)
    /// - Relative path (contains directory component): joined with cwd
    /// - Bare name: resolved via PATH, skipping masquerade wrapper
    ///   directories (ccache, distcc, icecc, colorgcc, buildcache)
    ///
    /// Returns `None` if the program cannot be found.
    fn resolve_program_path(context: &context::Context, value: &str) -> Option<PathBuf> {
        let name = value.trim();
        if name.is_empty() {
            return None;
        }

        let path = PathBuf::from(name);

        // Absolute path: pass through as-is. Do not canonicalize because that
        // resolves symlinks, which can change the filename (e.g. /usr/bin/gcc ->
        // /usr/bin/x86_64-linux-gnu-gcc-13) and break wrapper name matching.
        if path.is_absolute() {
            return Some(path);
        }

        // Relative path with directory component (e.g. "./gcc" or "tools/gcc"):
        // resolve against cwd to make it absolute.
        if path.parent().is_some_and(|p| !p.as_os_str().is_empty()) {
            return Some(context.current_directory.join(&path));
        }

        // Bare name: resolve via PATH, skipping masquerade wrappers.
        let search_path = context.path().map(|(_, p)| p).unwrap_or_else(|| context.confstr_path.clone());
        resolve_past_masquerade_wrappers(name, &search_path, &context.current_directory)
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
        let layout = InstallationLayout::try_from(context.current_executable.as_path())
            .map_err(ConfigurationError::InstallationLayout)?;
        let library = layout.preload_path();

        let mut environment_overrides = HashMap::new();

        // Platform-specific preload configuration
        #[cfg(target_os = "macos")]
        {
            // macOS uses DYLD_INSERT_LIBRARIES and DYLD_FORCE_FLAT_NAMESPACE
            let preload_original =
                context.environment.get(KEY_OS__MACOS_PRELOAD_PATH).cloned().unwrap_or_default();
            let preload_updated =
                insert_to_path(&preload_original, library.as_path()).map_err(ConfigurationError::Path)?;

            environment_overrides.insert(KEY_OS__MACOS_PRELOAD_PATH.to_string(), preload_updated);
            environment_overrides.insert(KEY_OS__MACOS_FLAT_NAMESPACE.to_string(), "1".to_string());
        }
        #[cfg(not(target_os = "macos"))]
        {
            // Linux and other Unix-like systems use LD_PRELOAD
            let preload_original = context.environment.get(KEY_OS__PRELOAD_PATH).cloned().unwrap_or_default();
            let preload_updated =
                insert_to_path(&preload_original, library.as_path()).map_err(ConfigurationError::Path)?;

            environment_overrides.insert(KEY_OS__PRELOAD_PATH.to_string(), preload_updated);
        }

        // Make the current state available as a single environment variable
        let state: String = PreloadState { destination: address, library }
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
    #[error("Invalid installation layout: {0}")]
    InstallationLayout(#[from] crate::installation::LayoutError),
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

/// Known masquerade wrappers. A directory full of symlinks named after
/// compilers, where each symlink resolves to one of these binaries, is a
/// masquerade directory; Bear skips such directories when resolving the real
/// compiler. See `interception-wrapper-recursion`.
const MASQUERADE_WRAPPERS: &[&str] = &["ccache", "distcc", "icecc", "colorgcc", "buildcache"];

/// Checks whether `path` is a symlink whose ultimate target's filename is one
/// of the known masquerade wrappers.
///
/// Uses `canonicalize` to follow the symlink chain because we only inspect
/// the basename of the final target; the canonicalised path itself is
/// discarded. This must not be used when registering a compiler, since
/// canonicalisation would change the name (e.g. `/usr/bin/gcc` ->
/// `/usr/bin/gcc-13`) and break wrapper lookup.
fn is_masquerade_wrapper(path: &Path) -> bool {
    let Ok(target) = std::fs::canonicalize(path) else { return false };
    let Some(name) = target.file_name().and_then(|n| n.to_str()) else { return false };
    let stem = name.strip_suffix(".exe").or_else(|| name.strip_suffix(".EXE")).unwrap_or(name);
    let stem_lower = stem.to_ascii_lowercase();
    MASQUERADE_WRAPPERS.iter().any(|w| *w == stem_lower)
}

/// Resolves a bare program name via PATH, transparently skipping masquerade
/// wrapper directories. If the first match on PATH is a masquerade wrapper
/// (e.g. `/usr/lib64/ccache/gcc` -> `/usr/bin/ccache`), the containing
/// directory is excluded and the search is retried. The process repeats until
/// a non-masquerade compiler is found or PATH is exhausted.
///
/// Returns `None` if no real compiler is reachable past the masquerade
/// directories. In that case the caller must not register a wrapper; doing so
/// would re-create the recursion the filtering is designed to prevent.
fn resolve_past_masquerade_wrappers(name: &str, search_path: &str, cwd: &Path) -> Option<PathBuf> {
    let mut excluded: Vec<PathBuf> = Vec::new();
    loop {
        let current = filter_out_paths(search_path, &excluded);
        let found = which::which_in(name, Some(current.as_str()), cwd).ok()?;

        if !is_masquerade_wrapper(&found) {
            return Some(found);
        }

        let parent = found.parent()?.to_path_buf();
        if excluded.contains(&parent) {
            // Defensive: the excluded dir came back, which would loop forever.
            log::warn!(
                "resolve: masquerade dir {} already excluded but returned again for '{}'",
                parent.display(),
                name,
            );
            return None;
        }
        log::info!(
            "resolve: masquerade wrapper at {}; re-resolving '{}' past {}",
            found.display(),
            name,
            parent.display(),
        );
        excluded.push(parent);
    }
}

/// Joins a path-separated string, removing any entries that match one of the
/// excluded paths. Matching is by value; no canonicalisation.
fn filter_out_paths(original: &str, excluded: &[PathBuf]) -> String {
    let kept: Vec<PathBuf> =
        std::env::split_paths(original).filter(|p| !excluded.iter().any(|e| e == p)).collect();
    std::env::join_paths(kept).map(|os| os.into_string().unwrap_or_default()).unwrap_or_default()
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

    #[cfg(unix)]
    #[test]
    fn test_build_environment_create_preload() {
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
                    confstr_path: String::from("/usr/bin:/bin"),
                }
            };
            let intercept = config::Intercept::Preload;
            let compilers = vec![];
            let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

            BuildEnvironment::create(&context, &intercept, &compilers, address, |_| true).unwrap()
        };

        let libdir = env!("INTERCEPT_LIBDIR");
        let preload_name = env!("PRELOAD_NAME");
        let expected_library = format!("/usr/{libdir}/{preload_name}");
        let expected_state = format!(r#"{{"destination":"127.0.0.1:8080","library":"{expected_library}"}}"#);

        // Check platform-specific preload configuration
        #[cfg(target_os = "macos")]
        {
            assert_eq!(sut.environment_overrides.get(KEY_INTERCEPT_STATE), Some(&expected_state));
            let dyld_preload = sut.environment_overrides.get(KEY_OS__MACOS_PRELOAD_PATH).unwrap();
            assert_first_path_entry(&expected_library, dyld_preload);

            assert_eq!(sut.environment_overrides.get(KEY_OS__MACOS_FLAT_NAMESPACE), Some(&"1".to_string()));
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(sut.environment_overrides.get(KEY_INTERCEPT_STATE), Some(&expected_state));
            let ld_preload = sut.environment_overrides.get(KEY_OS__PRELOAD_PATH).unwrap();
            assert_first_path_entry(&expected_library, ld_preload);
        }

        assert!(sut._wrapper_directory.is_none());
    }

    #[test]
    fn test_wrapper_environment_path_setting_when_it_was_empty_before() {
        let current_dir = TempDir::new().unwrap();
        let install_dir = {
            let dir = TempDir::new().unwrap();

            // create bin dir
            let bin_dir = dir.path().join("bin");
            std::fs::create_dir(&bin_dir).unwrap();
            // create wrapper mock
            let file = bin_dir.join(env!("WRAPPER_NAME"));
            std::fs::write(&file, "#!/usr/bin/true").unwrap();

            dir
        };

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        let sut = {
            let intercept = config::Intercept::Wrapper;
            let compilers = vec![];
            let context = {
                crate::context::Context {
                    current_executable: install_dir.path().join("bin").join("bear-driver"),
                    current_directory: current_dir.path().to_path_buf(),
                    environment: HashMap::new(),
                    preload_supported: false,
                    confstr_path: String::from("/usr/bin:/bin"),
                }
            };

            BuildEnvironment::create(&context, &intercept, &compilers, address, |_| true).unwrap()
        };

        // Check that PATH is updated (should contain .bear directory at the beginning)
        let path = sut.environment_overrides.get("PATH").unwrap();
        let expected_wrapper_path = get_wrapper_dir_path(&current_dir);
        assert_first_path_entry(&expected_wrapper_path, path);
    }

    #[test]
    fn test_build_environment_create_wrapper() {
        let current_dir = TempDir::new().unwrap();
        let install_dir = {
            let dir = TempDir::new().unwrap();

            // create bin dir
            let bin_dir = dir.path().join("bin");
            std::fs::create_dir(&bin_dir).unwrap();
            // create wrapper mock
            let file = bin_dir.join(env!("WRAPPER_NAME"));
            std::fs::write(&file, "#!/usr/bin/true").unwrap();

            dir
        };

        // Create fake compiler binaries in a temp directory so the paths are
        // genuinely absolute on every platform (including Windows, where
        // "/usr/bin/gcc" is not absolute because it lacks a drive letter).
        let compiler_dir = TempDir::new().unwrap();
        let cc_path = compiler_dir.path().join("cc");
        let clang_path = compiler_dir.path().join("clang");
        let gcc_path = compiler_dir.path().join("gcc");
        let gxx_path = compiler_dir.path().join("g++");
        for p in [&cc_path, &clang_path, &gcc_path, &gxx_path] {
            std::fs::write(p, "fake").unwrap();
        }

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        let sut = {
            let intercept = config::Intercept::Wrapper;
            let compilers = vec![
                config::Compiler { path: cc_path.clone(), as_: None, ignore: false },
                config::Compiler { path: clang_path.clone(), as_: None, ignore: false },
            ];
            let context = {
                let path_value = compiler_dir.path().to_string_lossy().to_string();
                let environment = {
                    let mut builder = HashMap::new();
                    builder.insert(KEY_OS__PATH.to_string(), path_value.clone());
                    builder.insert("CC".to_string(), gcc_path.to_string_lossy().to_string());
                    builder.insert("CXX".to_string(), gxx_path.to_string_lossy().to_string());
                    builder
                };

                crate::context::Context {
                    current_executable: install_dir.path().join("bin").join("bear-driver"),
                    current_directory: current_dir.path().to_path_buf(),
                    environment,
                    preload_supported: true,
                    confstr_path: path_value,
                }
            };

            BuildEnvironment::create(&context, &intercept, &compilers, address, |_| true).unwrap()
        };

        // Check that PATH is updated (should contain .bear directory at the beginning)
        let path = sut.environment_overrides.get("PATH").unwrap();
        let expected_wrapper_path = get_wrapper_dir_path(&current_dir);
        assert_first_path_entry(&expected_wrapper_path, path);

        // Verify wrapper directory is kept alive
        assert!(sut._wrapper_directory.is_some());
        let wrapper_dir = sut._wrapper_directory.as_ref().expect("Wrapper directory should exist");
        let wrapper_config = wrapper_dir.config();

        // Check that collector address is in the config
        assert_eq!(wrapper_config.collector_address, address);

        // Check that compilers configured are in the wrapper config
        assert_eq!(&cc_path, wrapper_config.get_executable("cc").unwrap());
        assert_eq!(&clang_path, wrapper_config.get_executable("clang").unwrap());
        assert_eq!(&gcc_path, wrapper_config.get_executable("gcc").unwrap());
        assert_eq!(&gxx_path, wrapper_config.get_executable("g++").unwrap());

        // Check that environment variables were redirected to wrapper executables
        let cc_value = sut.environment_overrides.get("CC").unwrap();
        let cxx_value = sut.environment_overrides.get("CXX").unwrap();
        assert!(cc_value.contains(".bear"), "CC should point to wrapper: {}", cc_value);
        assert!(cxx_value.contains(".bear"), "CXX should point to wrapper: {}", cxx_value);
    }

    #[test]
    fn test_path_discovery_with_empty_executables() {
        let current_dir = TempDir::new().unwrap();
        let install_dir = {
            let dir = TempDir::new().unwrap();

            // create bin dir
            let bin_dir = dir.path().join("bin");
            std::fs::create_dir(&bin_dir).unwrap();
            // create wrapper mock
            let file = bin_dir.join(env!("WRAPPER_NAME"));
            std::fs::write(&file, "#!/usr/bin/true").unwrap();

            dir
        };

        // Create a directory with mock compilers
        let tool_dir = {
            let dir = TempDir::new().unwrap();

            // Create mock compiler executables
            #[cfg(unix)]
            let compilers = ["gcc", "g++", "clang", "notacompiler"];
            #[cfg(not(unix))]
            let compilers = ["gcc.exe", "g++.exe", "clang.exe", "notacompiler"];

            for compiler in &compilers {
                let compiler_path = dir.path().join(compiler);
                std::fs::write(&compiler_path, "#!/usr/bin/true").unwrap();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&compiler_path, std::fs::Permissions::from_mode(0o755)).unwrap();
                }
            }
            dir
        };

        let sut = {
            let context = {
                let environment = {
                    let mut builder = HashMap::new();
                    builder.insert("PATH".to_string(), tool_dir.path().to_string_lossy().to_string());
                    builder
                };

                crate::context::Context {
                    current_executable: install_dir.path().join("bin").join("bear-driver"),
                    current_directory: current_dir.path().to_path_buf(),
                    environment,
                    preload_supported: true,
                    confstr_path: String::from("/usr/bin:/bin"),
                }
            };

            let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

            let known_compilers = ["gcc", "g++", "clang", "gcc.exe", "g++.exe", "clang.exe"];
            BuildEnvironment::create_as_wrapper(
                &context,
                &[], // Empty executables - should trigger PATH discovery
                address,
                |path| {
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| known_compilers.contains(&n))
                        .unwrap_or(false)
                },
            )
        };

        assert!(sut.is_ok(), "PATH discovery should succeed");
        let build_env = sut.unwrap();

        // Verify that the PATH was updated to include the wrapper directory
        let path = build_env.environment_overrides.get("PATH").unwrap();
        let expected_wrapper_path = get_wrapper_dir_path(&current_dir);
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
        let current_dir = TempDir::new().unwrap();
        let install_dir = {
            let dir = TempDir::new().unwrap();

            // create bin dir
            let bin_dir = dir.path().join("bin");
            std::fs::create_dir(&bin_dir).unwrap();
            // create wrapper mock
            let file = bin_dir.join(env!("WRAPPER_NAME"));
            std::fs::write(&file, "#!/usr/bin/true").unwrap();

            dir
        };

        // Create a directory with a compiler
        let tool_dir = {
            let dir = TempDir::new().unwrap();

            #[cfg(unix)]
            let compiler = "gcc";
            #[cfg(not(unix))]
            let compiler = "gcc.exe";

            let compiler_path = dir.path().join(compiler);
            std::fs::write(&compiler_path, "#!/usr/bin/true").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&compiler_path, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            dir
        };

        // Use temp_dir as working directory through context
        let sut = {
            let context = {
                let environment = {
                    let mut builder = std::collections::HashMap::new();
                    builder.insert("PATH".to_string(), tool_dir.path().to_string_lossy().to_string());
                    builder
                };

                crate::context::Context {
                    current_executable: install_dir.path().join("bin").join("bear-driver"),
                    current_directory: current_dir.path().to_path_buf(),
                    environment,
                    preload_supported: true,
                    confstr_path: String::from("/usr/bin:/bin"),
                }
            };

            let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

            let custom_compiler =
                config::Compiler { path: PathBuf::from("/usr/bin/custom-gcc"), as_: None, ignore: false };

            BuildEnvironment::create(
                &context,
                &config::Intercept::Wrapper,
                &[custom_compiler],
                address,
                |_| true,
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

    // U1: resolve_program_path finds bare name via PATH
    #[test]
    fn test_env_var_bare_name_resolved_via_path() {
        let current_dir = TempDir::new().unwrap();
        let install_dir = {
            let dir = TempDir::new().unwrap();
            let bin_dir = dir.path().join("bin");
            std::fs::create_dir(&bin_dir).unwrap();
            let file = bin_dir.join(env!("WRAPPER_NAME"));
            std::fs::write(&file, "#!/usr/bin/true").unwrap();
            dir
        };

        // Create a fake compiler on a tool directory that will be in PATH
        let tool_dir = TempDir::new().unwrap();
        let compiler_name = if cfg!(windows) { "fake-cc.exe" } else { "fake-cc" };
        let compiler_path = {
            let path = tool_dir.path().join(compiler_name);
            std::fs::write(&path, "#!/usr/bin/true").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            path
        };

        let context = {
            let mut environment = HashMap::new();
            environment.insert(KEY_OS__PATH.to_string(), tool_dir.path().to_string_lossy().to_string());
            // CC is set to a bare name - no path, just "fake-cc"
            environment.insert("CC".to_string(), "fake-cc".to_string());

            crate::context::Context {
                current_executable: install_dir.path().join("bin").join("bear-driver"),
                current_directory: current_dir.path().to_path_buf(),
                environment,
                preload_supported: false,
                confstr_path: String::new(),
            }
        };

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let sut = BuildEnvironment::create_as_wrapper(&context, &[], address, |_| false).unwrap();

        let wrapper_dir = sut._wrapper_directory.as_ref().expect("Wrapper directory should exist");
        let wrapper_config = wrapper_dir.config();

        // The bare name "fake-cc" should have been resolved via PATH to the real path
        let resolved = wrapper_config
            .get_executable(compiler_name)
            .expect("bare CC=fake-cc should be resolved via PATH and registered");
        assert_eq!(
            resolved, &compiler_path,
            "resolved path should be the actual compiler on PATH, not a bare name",
        );

        // CC should be overridden to point to the wrapper
        let cc_value = sut.environment_overrides.get("CC").expect("CC should be overridden");
        assert!(cc_value.contains(".bear"), "CC should point to wrapper: {}", cc_value);
    }

    // U2: resolve_program_path returns absolute path as-is
    #[test]
    fn test_env_var_absolute_path_passed_through() {
        let current_dir = TempDir::new().unwrap();
        let install_dir = {
            let dir = TempDir::new().unwrap();
            let bin_dir = dir.path().join("bin");
            std::fs::create_dir(&bin_dir).unwrap();
            let file = bin_dir.join(env!("WRAPPER_NAME"));
            std::fs::write(&file, "#!/usr/bin/true").unwrap();
            dir
        };

        let compiler_name = if cfg!(windows) { "my-gcc.exe" } else { "my-gcc" };
        let compiler_path = {
            let path = current_dir.path().join(compiler_name);
            std::fs::write(&path, "#!/usr/bin/true").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            path
        };

        let context = {
            let mut environment = HashMap::new();
            environment.insert(KEY_OS__PATH.to_string(), "/usr/bin".to_string());
            // CC is set to an absolute path
            environment.insert("CC".to_string(), compiler_path.to_string_lossy().to_string());

            crate::context::Context {
                current_executable: install_dir.path().join("bin").join("bear-driver"),
                current_directory: current_dir.path().to_path_buf(),
                environment,
                preload_supported: false,
                confstr_path: String::new(),
            }
        };

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let sut = BuildEnvironment::create_as_wrapper(&context, &[], address, |_| false).unwrap();

        let wrapper_dir = sut._wrapper_directory.as_ref().expect("Wrapper directory should exist");
        let wrapper_config = wrapper_dir.config();

        // The absolute path should be registered using the filename
        assert!(
            wrapper_config.get_executable(compiler_name).is_some(),
            "absolute CC path should be registered as {}",
            compiler_name,
        );
    }

    // U4: resolve_program_path returns None for unresolvable name
    #[test]
    fn test_env_var_unresolvable_name_skipped() {
        let current_dir = TempDir::new().unwrap();
        let install_dir = {
            let dir = TempDir::new().unwrap();
            let bin_dir = dir.path().join("bin");
            std::fs::create_dir(&bin_dir).unwrap();
            let file = bin_dir.join(env!("WRAPPER_NAME"));
            std::fs::write(&file, "#!/usr/bin/true").unwrap();
            dir
        };

        let context = {
            let mut environment = HashMap::new();
            environment.insert(KEY_OS__PATH.to_string(), "/usr/bin".to_string());
            // CC is set to a name that does not exist anywhere on PATH
            environment.insert("CC".to_string(), "nonexistent-compiler-xyz".to_string());

            crate::context::Context {
                current_executable: install_dir.path().join("bin").join("bear-driver"),
                current_directory: current_dir.path().to_path_buf(),
                environment,
                preload_supported: false,
                confstr_path: String::new(),
            }
        };

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        // Should not fail - unresolvable env vars are skipped
        let sut = BuildEnvironment::create_as_wrapper(&context, &[], address, |_| false).unwrap();

        // CC should NOT be in overrides (it was skipped)
        assert!(
            !sut.environment_overrides.contains_key("CC"),
            "unresolvable CC should be skipped, not overridden",
        );
    }

    // U3: resolve_program_path resolves relative path against cwd
    #[test]
    fn test_env_var_relative_path_resolved_against_cwd() {
        let current_dir = TempDir::new().unwrap();
        let install_dir = {
            let dir = TempDir::new().unwrap();
            let bin_dir = dir.path().join("bin");
            std::fs::create_dir(&bin_dir).unwrap();
            let file = bin_dir.join(env!("WRAPPER_NAME"));
            std::fs::write(&file, "#!/usr/bin/true").unwrap();
            dir
        };

        // Create a compiler in a subdirectory of the working directory
        let tools_subdir = current_dir.path().join("tools");
        std::fs::create_dir(&tools_subdir).unwrap();
        let compiler_name = if cfg!(windows) { "my-cc.exe" } else { "my-cc" };
        let _compiler_path = {
            let path = tools_subdir.join(compiler_name);
            std::fs::write(&path, "#!/usr/bin/true").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            path
        };

        let relative_cc = format!("tools/{}", compiler_name);

        let context = {
            let mut environment = HashMap::new();
            environment.insert(KEY_OS__PATH.to_string(), "/usr/bin".to_string());
            // CC is set to a relative path
            environment.insert("CC".to_string(), relative_cc);

            crate::context::Context {
                current_executable: install_dir.path().join("bin").join("bear-driver"),
                current_directory: current_dir.path().to_path_buf(),
                environment,
                preload_supported: false,
                confstr_path: String::new(),
            }
        };

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let sut = BuildEnvironment::create_as_wrapper(&context, &[], address, |_| false).unwrap();

        let wrapper_dir = sut._wrapper_directory.as_ref().expect("Wrapper directory should exist");
        let wrapper_config = wrapper_dir.config();

        // The relative path should be resolved against cwd to an absolute path
        let resolved = wrapper_config
            .get_executable(compiler_name)
            .unwrap_or_else(|| panic!("relative CC=tools/{compiler_name} should be resolved against cwd"));
        assert!(resolved.is_absolute(), "resolved path should be absolute, got: {}", resolved.display(),);

        // CC should be overridden to point to the wrapper
        let cc_value = sut.environment_overrides.get("CC").expect("CC should be overridden");
        assert!(cc_value.contains(".bear"), "CC should point to wrapper: {}", cc_value);
    }

    #[cfg(unix)]
    mod masquerade {
        use super::super::{filter_out_paths, is_masquerade_wrapper, resolve_past_masquerade_wrappers};
        use std::os::unix::fs::PermissionsExt;
        use std::path::PathBuf;
        use tempfile::TempDir;

        fn write_executable(path: &std::path::Path, content: &str) {
            std::fs::write(path, content).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        // Requirements: interception-wrapper-recursion
        #[test]
        fn detects_symlink_to_ccache() {
            let dir = TempDir::new().unwrap();
            let fake_ccache = dir.path().join("ccache");
            write_executable(&fake_ccache, "#!/bin/sh\n");
            let gcc_symlink = dir.path().join("masq").join("gcc");
            std::fs::create_dir_all(gcc_symlink.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(&fake_ccache, &gcc_symlink).unwrap();

            assert!(is_masquerade_wrapper(&gcc_symlink));
        }

        // Requirements: interception-wrapper-recursion
        #[test]
        fn detects_all_known_wrapper_names() {
            let dir = TempDir::new().unwrap();
            for name in ["distcc", "icecc", "colorgcc", "buildcache"] {
                let target = dir.path().join(name);
                write_executable(&target, "#!/bin/sh\n");
                let link = dir.path().join(format!("{name}-gcc-link"));
                std::os::unix::fs::symlink(&target, &link).unwrap();
                assert!(is_masquerade_wrapper(&link), "{name} target should be detected");
            }
        }

        // Requirements: interception-wrapper-recursion
        #[test]
        fn ignores_real_compiler_and_non_wrapper_symlinks() {
            let dir = TempDir::new().unwrap();
            let real_gcc = dir.path().join("gcc-13");
            write_executable(&real_gcc, "#!/bin/sh\n");
            assert!(!is_masquerade_wrapper(&real_gcc));

            let gcc_symlink = dir.path().join("gcc");
            std::os::unix::fs::symlink(&real_gcc, &gcc_symlink).unwrap();
            assert!(!is_masquerade_wrapper(&gcc_symlink));
        }

        // Requirements: interception-wrapper-recursion
        #[test]
        fn ignores_broken_and_missing_paths() {
            let dir = TempDir::new().unwrap();
            let broken = dir.path().join("broken-link");
            std::os::unix::fs::symlink(dir.path().join("does-not-exist"), &broken).unwrap();
            assert!(!is_masquerade_wrapper(&broken));
            assert!(!is_masquerade_wrapper(&dir.path().join("does-not-exist")));
        }

        // Requirements: interception-wrapper-recursion
        #[test]
        fn filter_out_paths_drops_matching_entries_only() {
            let a = PathBuf::from("/a");
            let b = PathBuf::from("/b");
            let c = PathBuf::from("/c");
            let original = std::env::join_paths([&a, &b, &c]).unwrap().into_string().unwrap_or_default();

            let kept = filter_out_paths(&original, std::slice::from_ref(&b));
            let entries: Vec<PathBuf> = std::env::split_paths(&kept).collect();
            assert_eq!(entries, vec![a, c]);
        }

        // Requirements: interception-wrapper-recursion
        #[test]
        fn resolver_returns_real_compiler_when_no_masquerade() {
            let dir = TempDir::new().unwrap();
            let real = dir.path().join("gcc");
            write_executable(&real, "#!/bin/sh\n");

            let path = std::env::join_paths([dir.path()]).unwrap().into_string().unwrap_or_default();
            let found = resolve_past_masquerade_wrappers("gcc", &path, dir.path()).unwrap();
            assert_eq!(found, real);
        }

        // Requirements: interception-wrapper-recursion
        #[test]
        fn resolver_skips_masquerade_and_returns_next_real_compiler() {
            let dir = TempDir::new().unwrap();
            let ccache_bin = dir.path().join("bin").join("ccache");
            std::fs::create_dir_all(ccache_bin.parent().unwrap()).unwrap();
            write_executable(&ccache_bin, "#!/bin/sh\n");

            let masq_dir = dir.path().join("ccache_dir");
            std::fs::create_dir_all(&masq_dir).unwrap();
            let masq_gcc = masq_dir.join("gcc");
            std::os::unix::fs::symlink(&ccache_bin, &masq_gcc).unwrap();

            let real_dir = dir.path().join("real");
            std::fs::create_dir_all(&real_dir).unwrap();
            let real_gcc = real_dir.join("gcc");
            write_executable(&real_gcc, "#!/bin/sh\n");

            let path =
                std::env::join_paths([&masq_dir, &real_dir]).unwrap().into_string().unwrap_or_default();
            let found = resolve_past_masquerade_wrappers("gcc", &path, dir.path()).unwrap();
            assert_eq!(found, real_gcc);
        }

        // Requirements: interception-wrapper-recursion
        #[test]
        fn resolver_returns_none_when_only_masquerade_is_reachable() {
            let dir = TempDir::new().unwrap();
            let ccache_bin = dir.path().join("ccache");
            write_executable(&ccache_bin, "#!/bin/sh\n");

            let masq_dir = dir.path().join("masq");
            std::fs::create_dir_all(&masq_dir).unwrap();
            let masq_gcc = masq_dir.join("gcc");
            std::os::unix::fs::symlink(&ccache_bin, &masq_gcc).unwrap();

            let path = std::env::join_paths([&masq_dir]).unwrap().into_string().unwrap_or_default();
            assert!(resolve_past_masquerade_wrappers("gcc", &path, dir.path()).is_none());
        }

        // Requirements: interception-wrapper-recursion
        #[test]
        fn resolver_skips_multiple_masquerade_layers() {
            let dir = TempDir::new().unwrap();
            let ccache_bin = dir.path().join("ccache");
            let distcc_bin = dir.path().join("distcc");
            write_executable(&ccache_bin, "#!/bin/sh\n");
            write_executable(&distcc_bin, "#!/bin/sh\n");

            let masq1 = dir.path().join("m1");
            let masq2 = dir.path().join("m2");
            let real = dir.path().join("real");
            std::fs::create_dir_all(&masq1).unwrap();
            std::fs::create_dir_all(&masq2).unwrap();
            std::fs::create_dir_all(&real).unwrap();

            std::os::unix::fs::symlink(&ccache_bin, masq1.join("gcc")).unwrap();
            std::os::unix::fs::symlink(&distcc_bin, masq2.join("gcc")).unwrap();
            let real_gcc = real.join("gcc");
            write_executable(&real_gcc, "#!/bin/sh\n");

            let path =
                std::env::join_paths([&masq1, &masq2, &real]).unwrap().into_string().unwrap_or_default();
            let found = resolve_past_masquerade_wrappers("gcc", &path, dir.path()).unwrap();
            assert_eq!(found, real_gcc);
        }
    }
}
