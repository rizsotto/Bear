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
        // and immediately update the final environment with wrapper paths. Values may
        // include trailing flags (e.g. CC="gcc -std=c11"); those are preserved in the
        // rewritten override so the build still receives them.
        let mut environment_overrides = HashMap::new();
        for (key, value) in &context.environment {
            if !crate::environment::program_env(key) || value.is_empty() {
                continue;
            }
            let Some((program, flags)) = parse_program_env_value(value) else {
                log::warn!("Skipping compiler env var {}={:?}: could not parse value", key, value);
                continue;
            };
            let Some(program_path) = Self::resolve_program_path(context, &program) else {
                log::warn!(
                    "Skipping compiler env var {}={:?}: could not resolve {:?} to an executable on PATH",
                    key,
                    value,
                    program,
                );
                continue;
            };
            let wrapper_path = wrapper_dir_builder.register_executable(program_path)?;
            let mut override_value = wrapper_path.to_string_lossy().into_owned();
            // Plain space-join, never shell-quoted. Build systems consume
            // compiler env vars by either (a) unquoted variable expansion in
            // the shell (`$CC -c foo.c`), where POSIX quote removal does not
            // apply to the expanded text, or (b) GNU Make recipes passed to
            // `sh -c`, where the literal string is parsed fresh. In both
            // paths, introducing shell quoting on our side yields wrong
            // argv: (a) leaks literal quotes into the argv, (b) adds a layer
            // of quoting the original value did not carry. The user already
            // committed to whitespace-separated tokens when they set the
            // env var; Bear does not add or remove that promise.
            for flag in flags {
                override_value.push(' ');
                override_value.push_str(&flag);
            }
            environment_overrides.insert(key.clone(), override_value);
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
    /// - Absolute path: returned as-is (not canonicalized); if the path is a
    ///   masquerade wrapper, falls back to resolving the basename via PATH.
    /// - Relative path (contains directory component): joined with cwd; same
    ///   masquerade fallback applies.
    /// - Bare name: resolved via PATH, skipping masquerade wrapper
    ///   directories (ccache, distcc, icecc, colorgcc, buildcache).
    ///
    /// Returns `None` if the program cannot be found or every resolution
    /// lands on a masquerade wrapper with no real compiler past it.
    fn resolve_program_path(context: &context::Context, value: &str) -> Option<PathBuf> {
        let name = value.trim();
        if name.is_empty() {
            return None;
        }

        let path = PathBuf::from(name);
        let search_path = context.path().map(|(_, p)| p).unwrap_or_else(|| context.confstr_path.clone());

        // Absolute path, or relative path with a directory component. We do
        // not canonicalize here because that resolves symlinks and can change
        // the filename (e.g. /usr/bin/gcc -> /usr/bin/x86_64-linux-gnu-gcc-13),
        // breaking wrapper name matching. If the supplied path IS a masquerade
        // wrapper, fall back to resolving the basename via PATH so we do not
        // register a wrapper that loops (see interception-wrapper-recursion).
        let supplied: Option<PathBuf> = if path.is_absolute() {
            Some(path)
        } else if path.parent().is_some_and(|p| !p.as_os_str().is_empty()) {
            Some(context.current_directory.join(&path))
        } else {
            None
        };

        if let Some(candidate) = supplied {
            if !is_masquerade_wrapper(&candidate) {
                return Some(candidate);
            }
            let basename = candidate.file_name().and_then(|n| n.to_str())?;
            log::info!(
                "resolve: {} is a masquerade wrapper; retrying basename '{}' via PATH",
                candidate.display(),
                basename,
            );
            return resolve_past_masquerade_wrappers(basename, &search_path, &context.current_directory);
        }

        // Bare name: resolve via PATH, skipping masquerade wrappers.
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

/// Splits a compiler env var value into `(program, flags)` on whitespace.
/// Returns `None` for empty or whitespace-only values; the caller logs a
/// warning and skips. Anything more elaborate than whitespace-separated
/// tokens (shell quoting, metacharacters, command substitutions) belongs
/// in `CFLAGS`/`CXXFLAGS` -- see `interception-compiler-env-with-flags`.
fn parse_program_env_value(value: &str) -> Option<(String, Vec<String>)> {
    let mut tokens = value.split_whitespace().map(str::to_string);
    let program = tokens.next()?;
    Some((program, tokens.collect()))
}

/// Known masquerade wrappers. A directory full of symlinks named after
/// compilers, where each symlink resolves to one of these binaries, is a
/// masquerade directory; Bear skips such directories when resolving the real
/// compiler. See `interception-wrapper-recursion`.
const MASQUERADE_WRAPPERS: &[&str] = &["ccache", "distcc", "icecc", "colorgcc", "buildcache"];

/// Checks whether `path` is a symlink whose ultimate target's filename is one
/// of the known masquerade wrappers.
///
/// Short-circuits on non-symlinks so iterating every executable in a PATH
/// directory stays cheap. For symlinks, uses `canonicalize` to follow the
/// chain because only the basename of the final target is inspected; the
/// canonicalised path itself is discarded. Callers must not use this helper
/// when registering a compiler -- canonicalisation would change the name
/// (e.g. `/usr/bin/gcc` -> `/usr/bin/gcc-13`) and break wrapper lookup.
fn is_masquerade_wrapper(path: &Path) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return false };
    if !meta.file_type().is_symlink() {
        return false;
    }
    let Ok(target) = std::fs::canonicalize(path) else { return false };
    let Some(name) = target.file_name().and_then(|n| n.to_str()) else { return false };
    let stem = name.strip_suffix(".exe").or_else(|| name.strip_suffix(".EXE")).unwrap_or(name);
    MASQUERADE_WRAPPERS.iter().any(|w| w.eq_ignore_ascii_case(stem))
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
        let found = match which::which_in(name, Some(current.as_str()), cwd) {
            Ok(path) => path,
            Err(_) => {
                if !excluded.is_empty() {
                    let dirs =
                        excluded.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ");
                    log::warn!("resolve: no real '{}' on PATH past masquerade dir(s): {}", name, dirs,);
                }
                return None;
            }
        };

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
/// excluded paths. Trailing path separators are stripped on both sides before
/// comparison so that a PATH entry written as `/usr/lib64/ccache/` still
/// matches an excluded dir derived from `PathBuf::parent()` (which has no
/// trailing slash). No further canonicalisation: matching is otherwise by
/// byte-equal path.
fn filter_out_paths(original: &str, excluded: &[PathBuf]) -> String {
    let excluded_normalised: Vec<PathBuf> = excluded.iter().map(|p| normalise_path(p)).collect();
    let kept: Vec<PathBuf> = std::env::split_paths(original)
        .filter(|p| {
            let candidate = normalise_path(p);
            !excluded_normalised.iter().any(|e| e == &candidate)
        })
        .collect();
    std::env::join_paths(kept).map(|os| os.into_string().unwrap_or_default()).unwrap_or_default()
}

/// Strips trailing path separators from a path (except when the path is just
/// a root, e.g. `/` on Unix or `C:\` on Windows). Returns the input unchanged
/// for non-UTF-8 paths.
fn normalise_path(path: &Path) -> PathBuf {
    let Some(s) = path.to_str() else { return path.to_path_buf() };
    let trimmed = s.trim_end_matches(std::path::MAIN_SEPARATOR);
    if trimmed.is_empty() { path.to_path_buf() } else { PathBuf::from(trimmed) }
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

    mod fixture {
        //! Shared fixture builder for `BuildEnvironment` wrapper-mode tests.
        //!
        //! Each test needs roughly the same on-disk scaffolding: a
        //! `current_dir` for the `.bear/` directory, an `install_dir`
        //! containing `bin/<WRAPPER_NAME>` that `register_executable`
        //! copies/hard-links, and some number of fake compiler binaries
        //! on PATH or at known locations. Assembling that inline burns
        //! ~20 lines per test and buries the intent. The `Fixture`
        //! builder below owns the backing `TempDir`s, assembles the
        //! `Context`, and exposes just the knobs the tests care about.
        //!
        //! The fixture cannot mock the filesystem away: `create_as_wrapper`
        //! really creates a directory, `register_executable` really copies
        //! files, and `which::which_in` really stats PATH entries. The
        //! fixture minimises the ceremony, not the I/O.
        use super::*;
        use std::collections::HashMap;
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};
        use std::path::{Path, PathBuf};
        use tempfile::TempDir;

        /// Standard test socket address.
        pub fn test_address() -> SocketAddr {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)
        }

        /// Platform-appropriate fake-compiler basename.
        pub fn compiler_basename() -> &'static str {
            if cfg!(windows) { "fake-cc.exe" } else { "fake-cc" }
        }

        /// Writes a file and marks it executable on Unix.
        pub fn write_executable(path: &Path) {
            std::fs::write(path, "#!/usr/bin/true").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
        }

        /// Builder-style fixture. Owns every `TempDir` it creates so
        /// paths handed out stay valid for the test's lifetime. Drop
        /// cleans everything up.
        pub struct Fixture {
            current_dir: TempDir,
            install_dir: TempDir,
            tool_dirs: Vec<TempDir>,
            environment: HashMap<String, String>,
            preload_supported: bool,
            confstr_path: String,
        }

        impl Fixture {
            /// Fresh fixture: empty `current_dir`, `install_dir` with a
            /// fake wrapper binary under `bin/<WRAPPER_NAME>`, empty
            /// environment, `preload_supported=false`, `confstr_path=""`.
            pub fn new() -> Self {
                let install_dir = TempDir::new().unwrap();
                let bin_dir = install_dir.path().join("bin");
                std::fs::create_dir(&bin_dir).unwrap();
                write_executable(&bin_dir.join(env!("WRAPPER_NAME")));
                Self {
                    current_dir: TempDir::new().unwrap(),
                    install_dir,
                    tool_dirs: Vec::new(),
                    environment: HashMap::new(),
                    preload_supported: false,
                    confstr_path: String::new(),
                }
            }

            /// Overwrites `PATH` with the string produced by joining
            /// the tool_dirs registered via `add_compiler_on_path` /
            /// `add_compilers_on_path`. Callers normally do not need to
            /// invoke this directly.
            fn refresh_path(&mut self) {
                let joined = std::env::join_paths(self.tool_dirs.iter().map(|d| d.path().to_owned()))
                    .expect("join PATH");
                self.environment.insert(KEY_OS__PATH.to_string(), joined.into_string().expect("utf-8 PATH"));
            }

            /// Creates a new tool directory, writes a fake compiler
            /// named `name`, adds the directory to `PATH`, returns the
            /// compiler's absolute path.
            pub fn add_compiler_on_path(&mut self, name: &str) -> PathBuf {
                let paths = self.add_compilers_on_path(&[name]);
                paths.into_iter().next().unwrap()
            }

            /// Creates one tool directory, writes one fake binary per
            /// name, adds the directory to `PATH`, returns their paths
            /// in the order passed.
            pub fn add_compilers_on_path(&mut self, names: &[&str]) -> Vec<PathBuf> {
                let dir = TempDir::new().unwrap();
                let paths: Vec<PathBuf> = names
                    .iter()
                    .map(|n| {
                        let p = dir.path().join(n);
                        write_executable(&p);
                        p
                    })
                    .collect();
                self.tool_dirs.push(dir);
                self.refresh_path();
                paths
            }

            /// Creates a fake compiler in a new tool directory that is
            /// NOT added to `PATH`. Use for tests that set `CC` to an
            /// absolute path directly.
            pub fn add_compiler_off_path(&mut self, name: &str) -> PathBuf {
                let dir = TempDir::new().unwrap();
                let path = dir.path().join(name);
                write_executable(&path);
                self.tool_dirs.push(dir);
                path
            }

            /// Writes a fake compiler at `current_dir/<subdir>/<name>`.
            /// Used for relative-path tests.
            pub fn add_compiler_in_cwd_subdir(&mut self, subdir: &str, name: &str) -> PathBuf {
                let dir = self.current_dir.path().join(subdir);
                std::fs::create_dir_all(&dir).unwrap();
                let path = dir.join(name);
                write_executable(&path);
                path
            }

            pub fn with_env(mut self, key: &str, value: &str) -> Self {
                self.environment.insert(key.into(), value.into());
                self
            }

            /// Overrides PATH with the given string. Use only for tests
            /// that want a specific, non-tool-dir PATH (e.g. to force
            /// PATH resolution failure).
            pub fn with_path_string(mut self, value: &str) -> Self {
                self.environment.insert(KEY_OS__PATH.to_string(), value.into());
                self
            }

            pub fn with_preload_supported(mut self, v: bool) -> Self {
                self.preload_supported = v;
                self
            }

            pub fn current_dir(&self) -> &Path {
                self.current_dir.path()
            }

            /// The `.bear` directory path that `create_as_wrapper` will
            /// populate.
            pub fn wrapper_dir(&self) -> PathBuf {
                self.current_dir.path().join(".bear")
            }

            /// Full path of a named wrapper inside `.bear/`.
            pub fn wrapper_path_for(&self, name: &str) -> PathBuf {
                self.wrapper_dir().join(name)
            }

            /// Borrows the underlying `current_dir` TempDir, for
            /// compatibility with helpers like `get_wrapper_dir_path`.
            pub fn current_dir_tempdir(&self) -> &TempDir {
                &self.current_dir
            }

            pub fn context(&self) -> crate::context::Context {
                crate::context::Context {
                    current_executable: self.install_dir.path().join("bin").join("bear-driver"),
                    current_directory: self.current_dir.path().to_path_buf(),
                    environment: self.environment.clone(),
                    preload_supported: self.preload_supported,
                    confstr_path: self.confstr_path.clone(),
                }
            }
        }
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
        let fx = fixture::Fixture::new();
        let sut = BuildEnvironment::create(
            &fx.context(),
            &config::Intercept::Wrapper,
            &[],
            fixture::test_address(),
            |_| true,
        )
        .unwrap();

        let path = sut.environment_overrides.get("PATH").unwrap();
        let expected = get_wrapper_dir_path(fx.current_dir_tempdir());
        assert_first_path_entry(&expected, path);
    }

    // Sets `CC`/`CXX` to raw absolute paths from a TempDir, which materialise
    // as backslash-separated strings on Windows. That input pattern is not a
    // realistic Windows scenario: MSYS2/Git Bash, where `make` actually runs
    // on Windows, use forward-slash paths. The equivalent Windows coverage
    // lives in
    // `env_with_flags::forward_slash_absolute_cc_registers_wrapper_on_windows`.
    #[cfg(unix)]
    #[test]
    fn test_build_environment_create_wrapper() {
        let mut fx = fixture::Fixture::new().with_preload_supported(true);
        let paths = fx.add_compilers_on_path(&["cc", "clang", "gcc", "g++"]);
        let [cc_path, clang_path, gcc_path, gxx_path] = [&paths[0], &paths[1], &paths[2], &paths[3]];
        let fx = fx.with_env("CC", &gcc_path.to_string_lossy()).with_env("CXX", &gxx_path.to_string_lossy());

        let compilers = vec![
            config::Compiler { path: cc_path.clone(), as_: None, ignore: false },
            config::Compiler { path: clang_path.clone(), as_: None, ignore: false },
        ];
        let address = fixture::test_address();
        let sut =
            BuildEnvironment::create(&fx.context(), &config::Intercept::Wrapper, &compilers, address, |_| {
                true
            })
            .unwrap();

        let path = sut.environment_overrides.get("PATH").unwrap();
        assert_first_path_entry(&get_wrapper_dir_path(fx.current_dir_tempdir()), path);

        let wrapper_dir = sut._wrapper_directory.as_ref().expect("wrapper directory");
        let wrapper_config = wrapper_dir.config();
        assert_eq!(wrapper_config.collector_address, address);
        assert_eq!(wrapper_config.get_executable("cc").unwrap(), cc_path);
        assert_eq!(wrapper_config.get_executable("clang").unwrap(), clang_path);
        assert_eq!(wrapper_config.get_executable("gcc").unwrap(), gcc_path);
        assert_eq!(wrapper_config.get_executable("g++").unwrap(), gxx_path);

        assert!(sut.environment_overrides.get("CC").unwrap().contains(".bear"));
        assert!(sut.environment_overrides.get("CXX").unwrap().contains(".bear"));
    }

    #[test]
    fn test_path_discovery_with_empty_executables() {
        #[cfg(unix)]
        let compilers = ["gcc", "g++", "clang", "notacompiler"];
        #[cfg(not(unix))]
        let compilers = ["gcc.exe", "g++.exe", "clang.exe", "notacompiler"];

        let mut fx = fixture::Fixture::new().with_preload_supported(true);
        fx.add_compilers_on_path(&compilers);

        let known_compilers = ["gcc", "g++", "clang", "gcc.exe", "g++.exe", "clang.exe"];
        let build_env = BuildEnvironment::create_as_wrapper(
            &fx.context(),
            &[], // Empty executables - should trigger PATH discovery
            fixture::test_address(),
            |path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| known_compilers.contains(&n))
                    .unwrap_or(false)
            },
        )
        .expect("PATH discovery should succeed");

        let path = build_env.environment_overrides.get("PATH").unwrap();
        assert_first_path_entry(&get_wrapper_dir_path(fx.current_dir_tempdir()), path);

        let wrapper_config = build_env._wrapper_directory.as_ref().unwrap().config();
        #[cfg(unix)]
        for name in ["gcc", "g++", "clang"] {
            assert!(wrapper_config.get_executable(name).is_some(), "{name} should be discovered");
        }
        #[cfg(not(unix))]
        for name in ["gcc.exe", "g++.exe", "clang.exe"] {
            assert!(wrapper_config.get_executable(name).is_some(), "{name} should be discovered");
        }
        assert!(wrapper_config.get_executable("notacompiler").is_none());
    }

    #[test]
    fn test_path_discovery_skipped_with_executables() {
        let compiler = if cfg!(unix) { "gcc" } else { "gcc.exe" };
        let mut fx = fixture::Fixture::new().with_preload_supported(true);
        fx.add_compiler_on_path(compiler);

        let custom_compiler =
            config::Compiler { path: PathBuf::from("/usr/bin/custom-gcc"), as_: None, ignore: false };

        let build_env = BuildEnvironment::create(
            &fx.context(),
            &config::Intercept::Wrapper,
            &[custom_compiler],
            fixture::test_address(),
            |_| true,
        )
        .expect("should succeed with explicit executables");

        let wrapper_config = build_env._wrapper_directory.as_ref().unwrap().config();
        assert!(wrapper_config.get_executable("custom-gcc").is_some());
        assert!(
            wrapper_config.get_executable(compiler).is_none(),
            "PATH discovery should be skipped when explicit executables are provided",
        );
    }

    // U1: resolve_program_path finds bare name via PATH
    #[test]
    fn test_env_var_bare_name_resolved_via_path() {
        let basename = fixture::compiler_basename();
        let mut fx = fixture::Fixture::new();
        let compiler_path = fx.add_compiler_on_path(basename);
        let fx = fx.with_env("CC", "fake-cc");

        let sut = BuildEnvironment::create_as_wrapper(&fx.context(), &[], fixture::test_address(), |_| false)
            .unwrap();

        let wrapper_config = sut._wrapper_directory.as_ref().unwrap().config();
        assert_eq!(wrapper_config.get_executable(basename).unwrap(), &compiler_path);
        assert!(sut.environment_overrides.get("CC").unwrap().contains(".bear"));
    }

    // U2: resolve_program_path returns absolute path as-is.
    // Unix-only: the TempDir path is backslash-separated on Windows, which is
    // not a realistic `CC` value on that platform. See the Windows-specific
    // counterpart `env_with_flags::forward_slash_absolute_cc_registers_wrapper_on_windows`.
    #[cfg(unix)]
    #[test]
    fn test_env_var_absolute_path_passed_through() {
        let compiler_name = "my-gcc";
        let mut fx = fixture::Fixture::new().with_path_string("/usr/bin");
        let compiler_path = fx.add_compiler_off_path(compiler_name);
        let fx = fx.with_env("CC", &compiler_path.to_string_lossy());

        let sut = BuildEnvironment::create_as_wrapper(&fx.context(), &[], fixture::test_address(), |_| false)
            .unwrap();

        assert!(
            sut._wrapper_directory.as_ref().unwrap().config().get_executable(compiler_name).is_some(),
            "absolute CC path should be registered as {compiler_name}",
        );
    }

    // U4: resolve_program_path returns None for unresolvable name
    #[test]
    fn test_env_var_unresolvable_name_skipped() {
        let fx =
            fixture::Fixture::new().with_path_string("/usr/bin").with_env("CC", "nonexistent-compiler-xyz");

        let sut = BuildEnvironment::create_as_wrapper(&fx.context(), &[], fixture::test_address(), |_| false)
            .unwrap();

        assert!(!sut.environment_overrides.contains_key("CC"));
    }

    // U3: resolve_program_path resolves relative path against cwd
    #[test]
    fn test_env_var_relative_path_resolved_against_cwd() {
        let compiler_name = if cfg!(windows) { "my-cc.exe" } else { "my-cc" };
        let mut fx = fixture::Fixture::new().with_path_string("/usr/bin");
        fx.add_compiler_in_cwd_subdir("tools", compiler_name);
        let fx = fx.with_env("CC", &format!("tools/{compiler_name}"));

        let sut = BuildEnvironment::create_as_wrapper(&fx.context(), &[], fixture::test_address(), |_| false)
            .unwrap();

        let resolved = sut
            ._wrapper_directory
            .as_ref()
            .unwrap()
            .config()
            .get_executable(compiler_name)
            .unwrap_or_else(|| panic!("relative CC=tools/{compiler_name} should resolve against cwd"));
        assert!(resolved.is_absolute(), "resolved path should be absolute, got: {}", resolved.display());
        assert!(sut.environment_overrides.get("CC").unwrap().contains(".bear"));
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

        /// A PATH entry written with a trailing separator must still match an
        /// excluded dir derived from `Path::parent()` (which never has one).
        // Requirements: interception-wrapper-recursion
        #[test]
        fn filter_out_paths_matches_across_trailing_separator() {
            let sep = std::path::MAIN_SEPARATOR;
            let original = format!("{sep}a{sep}:{sep}b{sep}:{sep}c");
            let excluded = PathBuf::from(format!("{sep}b"));

            let kept = filter_out_paths(&original, std::slice::from_ref(&excluded));
            let entries: Vec<PathBuf> = std::env::split_paths(&kept).collect();
            assert_eq!(
                entries,
                vec![PathBuf::from(format!("{sep}a{sep}")), PathBuf::from(format!("{sep}c"))],
                "trailing-slash PATH entry was not filtered",
            );
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

    /// An explicit CC=/path/to/ccache-masquerade/gcc must not be stored
    /// verbatim in the wrapper config -- that would recreate the recursion
    /// the masquerade filter is meant to prevent.
    // Requirements: interception-wrapper-recursion
    #[cfg(unix)]
    #[test]
    fn resolve_program_path_falls_back_past_masquerade_for_absolute_cc() {
        use std::os::unix::fs::PermissionsExt;
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let ccache_bin = dir.path().join("ccache");
        std::fs::write(&ccache_bin, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&ccache_bin, std::fs::Permissions::from_mode(0o755)).unwrap();

        let masq_dir = dir.path().join("masq");
        std::fs::create_dir_all(&masq_dir).unwrap();
        let masq_gcc = masq_dir.join("gcc");
        std::os::unix::fs::symlink(&ccache_bin, &masq_gcc).unwrap();

        let real_dir = dir.path().join("real");
        std::fs::create_dir_all(&real_dir).unwrap();
        let real_gcc = real_dir.join("gcc");
        std::fs::write(&real_gcc, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&real_gcc, std::fs::Permissions::from_mode(0o755)).unwrap();

        let mut environment = HashMap::new();
        environment.insert(
            KEY_OS__PATH.to_string(),
            std::env::join_paths([&masq_dir, &real_dir]).unwrap().into_string().unwrap(),
        );

        let context = crate::context::Context {
            current_executable: PathBuf::from("/unused"),
            current_directory: dir.path().to_path_buf(),
            environment,
            preload_supported: true,
            confstr_path: String::new(),
        };

        let resolved = BuildEnvironment::resolve_program_path(&context, masq_gcc.to_str().unwrap());
        assert_eq!(
            resolved.as_deref(),
            Some(real_gcc.as_path()),
            "absolute CC pointing at a masquerade symlink must fall back to the real compiler on PATH",
        );
    }

    mod env_with_flags {
        //! Requirements: interception-compiler-env-with-flags
        use super::super::parse_program_env_value;
        use super::*;

        #[test]
        fn parser_skips_empty_and_whitespace() {
            assert_eq!(parse_program_env_value(""), None);
            assert_eq!(parse_program_env_value("   "), None);
            assert_eq!(parse_program_env_value("\t\n"), None);
        }

        #[test]
        fn parser_splits_bare_name_with_flags() {
            let (program, flags) = parse_program_env_value("gcc -std=c11 -Wall").unwrap();
            assert_eq!(program, "gcc");
            assert_eq!(flags, vec!["-std=c11", "-Wall"]);
        }

        #[test]
        fn parser_returns_program_alone_when_no_flags() {
            let (program, flags) = parse_program_env_value("gcc").unwrap();
            assert_eq!(program, "gcc");
            assert!(flags.is_empty());
        }

        /// Regression guard: when the env var has no flags, the override must
        /// be the wrapper path as a bare string, byte-identical to how it was
        /// emitted before this requirement.
        ///
        /// Unix-only: the fixture builds `CC` from a TempDir path, which is
        /// backslash-separated on Windows -- not a realistic `CC` value in
        /// MSYS2/Git Bash (the shells where `bear -- make` runs on Windows).
        /// Windows coverage of absolute `CC` paths lives in
        /// `forward_slash_absolute_cc_registers_wrapper_on_windows`.
        #[cfg(unix)]
        #[test]
        fn no_flag_override_is_byte_identical_to_wrapper_path() {
            let basename = fixture::compiler_basename();
            let mut fx = fixture::Fixture::new();
            let compiler_path = fx.add_compiler_on_path(basename);
            let fx = fx.with_env("CC", &compiler_path.to_string_lossy());

            let sut =
                BuildEnvironment::create_as_wrapper(&fx.context(), &[], fixture::test_address(), |_| false)
                    .unwrap();

            assert_eq!(
                sut.environment_overrides.get("CC").unwrap(),
                &fx.wrapper_path_for(basename).to_string_lossy().into_owned(),
                "no-flag override must be the raw wrapper path, no shell quoting",
            );
        }

        #[test]
        fn bare_cc_with_flags_resolves_program_and_preserves_flags() {
            let basename = fixture::compiler_basename();
            let mut fx = fixture::Fixture::new();
            let compiler_path = fx.add_compiler_on_path(basename);
            let fx = fx.with_env("CC", &format!("{basename} -std=c11 -Wall"));

            let sut =
                BuildEnvironment::create_as_wrapper(&fx.context(), &[], fixture::test_address(), |_| false)
                    .unwrap();

            assert_eq!(
                sut._wrapper_directory.as_ref().unwrap().config().get_executable(basename).unwrap(),
                &compiler_path
            );
            let expected = format!("{} -std=c11 -Wall", fx.wrapper_path_for(basename).to_string_lossy());
            assert_eq!(sut.environment_overrides.get("CC").unwrap(), &expected);
        }

        // Unix-only for the same reason as
        // `no_flag_override_is_byte_identical_to_wrapper_path`: backslash-
        // separated TempDir paths are not a realistic Windows `CC` shape.
        #[cfg(unix)]
        #[test]
        fn absolute_cc_with_flags_resolves_program_and_preserves_flags() {
            let basename = fixture::compiler_basename();
            let mut fx = fixture::Fixture::new().with_path_string("/usr/bin");
            let compiler_path = fx.add_compiler_off_path(basename);
            let fx = fx.with_env("CC", &format!("{} -m32 -DX=1", compiler_path.to_string_lossy()));

            let sut =
                BuildEnvironment::create_as_wrapper(&fx.context(), &[], fixture::test_address(), |_| false)
                    .unwrap();

            let expected = format!("{} -m32 -DX=1", fx.wrapper_path_for(basename).to_string_lossy());
            assert_eq!(sut.environment_overrides.get("CC").unwrap(), &expected);
        }

        #[test]
        fn whitespace_only_cc_is_skipped() {
            let fx = fixture::Fixture::new().with_path_string("/usr/bin").with_env("CC", "   ");

            let sut =
                BuildEnvironment::create_as_wrapper(&fx.context(), &[], fixture::test_address(), |_| false)
                    .unwrap();

            assert!(!sut.environment_overrides.contains_key("CC"));
        }

        /// Covers the intersection of `interception-wrapper-recursion` and
        /// this requirement: a ccache masquerade dir first on PATH, with a
        /// real compiler past it, must resolve past the masquerade and still
        /// preserve the flags from the env var value.
        // Requirements: interception-compiler-env-with-flags, interception-wrapper-recursion
        #[cfg(unix)]
        #[test]
        fn masquerade_with_flags_resolves_real_compiler_and_preserves_flags() {
            use std::os::unix::fs::PermissionsExt;

            let fx = fixture::Fixture::new();
            let dir = fx.current_dir();

            // ccache binary and a masq dir with a symlink to it. Bear's
            // resolver must step past the masq dir and land on `real/gcc`.
            let ccache_bin = dir.join("ccache");
            std::fs::write(&ccache_bin, "#!/bin/sh\n").unwrap();
            std::fs::set_permissions(&ccache_bin, std::fs::Permissions::from_mode(0o755)).unwrap();

            let masq_dir = dir.join("masq");
            std::fs::create_dir_all(&masq_dir).unwrap();
            std::os::unix::fs::symlink(&ccache_bin, masq_dir.join("gcc")).unwrap();

            let real_dir = dir.join("real");
            std::fs::create_dir_all(&real_dir).unwrap();
            let real_gcc = real_dir.join("gcc");
            std::fs::write(&real_gcc, "#!/bin/sh\n").unwrap();
            std::fs::set_permissions(&real_gcc, std::fs::Permissions::from_mode(0o755)).unwrap();

            let path_value = std::env::join_paths([&masq_dir, &real_dir]).unwrap().into_string().unwrap();
            let fx = fx.with_path_string(&path_value).with_env("CC", "gcc -std=c11");

            let sut =
                BuildEnvironment::create_as_wrapper(&fx.context(), &[], fixture::test_address(), |_| false)
                    .unwrap();

            assert_eq!(
                sut._wrapper_directory.as_ref().unwrap().config().get_executable("gcc").unwrap(),
                &real_gcc,
                "masquerade symlink must be resolved past, just like without flags",
            );
            let expected = format!("{} -std=c11", fx.wrapper_path_for("gcc").to_string_lossy());
            assert_eq!(sut.environment_overrides.get("CC").unwrap(), &expected);
        }

        /// Windows coverage of `CC=<absolute path> -flag` using the path
        /// style MSYS2/Git Bash users actually produce: forward slashes
        /// with a drive letter (e.g. `C:/tools/gcc.exe`). Backslash-
        /// separated paths are shell escapes in those environments, so
        /// they are not a realistic `CC` value and are not exercised here.
        #[cfg(windows)]
        #[test]
        fn forward_slash_absolute_cc_registers_wrapper_on_windows() {
            let basename = fixture::compiler_basename();
            let mut fx = fixture::Fixture::new().with_path_string("C:/Windows/System32");
            let compiler_path = fx.add_compiler_off_path(basename);
            let forward_slash_path = compiler_path.to_string_lossy().replace('\\', "/");
            let fx = fx.with_env("CC", &format!("{forward_slash_path} -DBEAR_TEST=1"));

            let sut =
                BuildEnvironment::create_as_wrapper(&fx.context(), &[], fixture::test_address(), |_| false)
                    .unwrap();

            assert!(
                sut._wrapper_directory.as_ref().unwrap().config().get_executable(basename).is_some(),
                "forward-slash absolute CC path must register a wrapper for {basename}",
            );
            let cc_value = sut.environment_overrides.get("CC").unwrap();
            assert!(
                cc_value.ends_with(" -DBEAR_TEST=1"),
                "override must preserve -DBEAR_TEST=1: {cc_value:?}"
            );
            assert!(cc_value.contains(".bear"), "override must point at the .bear wrapper: {cc_value:?}");
        }
    }
}
