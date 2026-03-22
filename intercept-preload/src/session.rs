// SPDX-License-Identifier: GPL-3.0-or-later

//! Session management and environment doctoring for the preload library.
//!
//! This module handles:
//! - Capturing environment variables at library load time
//! - Storing session state (preload path, destination address)
//! - "Doctoring" environments to ensure child processes continue interception
//!
//! ## Restoration policy
//!
//! Bear's interception variables are always restored in child process environments.
//! The two key variables are:
//!
//! - The preload variable (`LD_PRELOAD` on Linux, `DYLD_INSERT_LIBRARIES` on macOS):
//!   Bear's library must be the **first** entry to ensure our exec overrides have
//!   priority over competing preload libraries. If another library's `execve` runs
//!   before ours and strips the preload variable, our doctoring never fires.
//!
//! - `BEAR_INTERCEPT`: encodes the session state (destination address + library path).
//!   Must exactly match the captured session state.
//!
//! When either condition fails, the environment is doctored to restore both.
//!
//! ## Co-resident library preservation
//!
//! When an envp has no preload variable at all (e.g. after `env -i`), the startup
//! snapshot is used as the base to preserve co-resident libraries (like Gentoo's
//! `libsandbox.so`). This is a compatibility-first policy.
//!
//! The startup snapshot is best-effort: it is captured from `environ` at constructor
//! time, which may already reflect modifications by earlier constructors in other
//! preload libraries.

use std::ffi::{CStr, CString};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::OnceLock;

use bear::environment::KEY_INTERCEPT_STATE;
#[cfg(not(target_os = "macos"))]
use bear::environment::KEY_OS__PRELOAD_PATH;
#[cfg(target_os = "macos")]
use bear::environment::{KEY_OS__MACOS_FLAT_NAMESPACE, KEY_OS__MACOS_PRELOAD_PATH};
use bear::intercept::environment::PreloadState;
use libc::{c_char, c_int};

/// The preload environment variable key for the current platform.
#[cfg(target_os = "macos")]
pub const PRELOAD_KEY: &str = KEY_OS__MACOS_PRELOAD_PATH;
#[cfg(not(target_os = "macos"))]
pub const PRELOAD_KEY: &str = KEY_OS__PRELOAD_PATH;

/// Combined session state, initialized atomically from the C constructor.
///
/// Replaces the previous separate `SESSION` and `INITIAL_PRELOAD` globals,
/// ensuring both are always initialized together.
pub static SESSION_CTX: OnceLock<SessionContext> = OnceLock::new();

/// Holds the parsed session state and the normalized startup preload snapshot.
pub struct SessionContext {
    /// Bear's session state (destination + library path).
    pub state: PreloadState,
    /// Best-effort startup snapshot of the preload variable, parsed and normalized.
    /// `None` if the preload variable was absent at startup.
    /// Stored as `Vec<PathBuf>` so we avoid re-parsing every time we doctor.
    pub startup_preload: Option<Vec<PathBuf>>,
}

/// Iterate over key-value pairs in a C-style envp array.
///
/// Yields `(key, value, full_entry)` for well-formed `"KEY=VALUE"` entries.
/// The `full_entry` includes the `=` separator, useful when callers need
/// to copy the entry verbatim (e.g. `DoctoredEnvironment::from_envp`).
/// Skips entries that are not valid UTF-8 or that lack `'='`.
///
/// The returned `&str` references borrow directly from the C strings pointed
/// to by `envp`. The caller must ensure those strings remain valid for `'a`.
///
/// # Safety
/// The `envp` pointer must be a valid, non-null, null-terminated array of
/// null-terminated C strings that remain valid for lifetime `'a`.
unsafe fn envp_iter<'a>(envp: *const *const c_char) -> impl Iterator<Item = (&'a str, &'a str, &'a str)> {
    let mut ptr = envp;
    std::iter::from_fn(move || {
        loop {
            if unsafe { (*ptr).is_null() } {
                return None;
            }
            // SAFETY: CStr::from_ptr returns &CStr with an unbounded lifetime.
            // We re-borrow it as &'a CStr, which is sound because the caller
            // guarantees the C strings remain valid for 'a.
            let cstr: &'a CStr = unsafe { CStr::from_ptr(*ptr) };
            ptr = unsafe { ptr.add(1) };

            let Ok(entry) = cstr.to_str() else {
                log::debug!("envp entry is not valid UTF-8, skipping");
                continue;
            };
            let Some((key, value)) = entry.split_once('=') else {
                log::debug!("envp entry has no '=' delimiter, skipping: {entry}");
                continue;
            };
            return Some((key, value, entry));
        }
    })
}

/// Create a PreloadState by extracting the intercept state from a C-style envp array.
///
/// Walks the envp to find `BEAR_INTERCEPT` and attempts to parse it into a
/// `PreloadState`.
///
/// # Safety
/// The `envp` pointer must be a valid null-terminated array of null-terminated
/// C strings in "KEY=VALUE" format, or null.
///
/// # Returns
/// - `Some(PreloadState)` if the intercept state variable is found and successfully parsed
/// - `None` if `envp` is null, the variable is not found, or parsing fails
unsafe fn from_envp(envp: *const *const c_char) -> Option<PreloadState> {
    if envp.is_null() {
        return None;
    }

    unsafe { envp_iter(envp) }.find(|(key, _, _)| *key == KEY_INTERCEPT_STATE).and_then(|(_, value, _)| {
        match value.try_into() {
            Ok(state) => Some(state),
            Err(_) => {
                log::debug!("BEAR_INTERCEPT found but failed to parse");
                None
            }
        }
    })
}

/// Read a single environment variable value from a C-style envp array.
///
/// # Safety
/// The `envp` pointer must be a valid null-terminated array of null-terminated
/// C strings in "KEY=VALUE" format, or null.
unsafe fn read_env_value(envp: *const *const c_char, key: &str) -> Option<String> {
    if envp.is_null() {
        return None;
    }

    unsafe { envp_iter(envp) }.find(|(k, _, _)| *k == key).map(|(_, v, _)| v.to_string())
}

/// Normalize a preload value string into a deduplicated list of paths.
///
/// Removes empty segments and deduplicates while preserving order.
fn normalize_preload(value: &str) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    std::env::split_paths(value)
        .filter(|p| !p.as_os_str().is_empty())
        .filter(|p| seen.insert(p.clone()))
        .collect()
}

/// Check whether the given envp is already aligned with the expected session state.
///
/// Returns `true` when both conditions hold:
/// 1. `BEAR_INTERCEPT` is present and parses to a value equal to `state`.
/// 2. Bear's library is the **first** entry in the preload variable (`PRELOAD_KEY`).
///
/// The first-position requirement exists because preload libraries are loaded in
/// order. Bear must be first so that our exec overrides take priority: if a
/// competing library's `execve` runs before ours and strips the preload variable,
/// our doctoring never fires. When Bear is not first, this function returns `false`
/// so that the caller re-doctors the environment to restore the correct ordering.
pub unsafe fn in_session(ctx: &SessionContext, envp: *const *const c_char) -> bool {
    if envp.is_null() {
        return false;
    }

    let mut intercept_state_matches = false;
    let mut preload_matches = false;

    for (key, value, _) in unsafe { envp_iter(envp) } {
        if key == KEY_INTERCEPT_STATE {
            if let Ok(parsed_state) = PreloadState::try_from(value) {
                intercept_state_matches = parsed_state == ctx.state;
            }
        } else if key == PRELOAD_KEY {
            preload_matches =
                std::env::split_paths(value).next().as_deref() == Some(ctx.state.library.as_path());
        }
    }

    intercept_state_matches && preload_matches
}

/// Compute the desired preload variable value for a child process.
///
/// This is the single source of truth for preload restoration, used by both
/// `DoctoredEnvironment::from_envp` (for exec-family calls) and
/// `ensure_environ_has_session_vars` (for system/popen calls).
///
/// Policy:
/// - `Some(value)` with a non-empty value: prepend Bear's library to it.
/// - `Some("")` (present but empty) or `None` (absent): fall back to the
///   startup snapshot so co-resident libraries survive.
/// - If no snapshot exists either, use just Bear's library.
pub fn desired_preload_value(ctx: &SessionContext, current: Option<&str>) -> Result<String, c_int> {
    let base = match current {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => {
            // Fall back to the startup snapshot.
            match &ctx.startup_preload {
                Some(paths) if !paths.is_empty() => std::env::join_paths(paths)
                    .map_err(|_| libc::EINVAL)?
                    .into_string()
                    .map_err(|_| libc::EINVAL)?,
                _ => String::new(),
            }
        }
    };
    bear::intercept::environment::insert_to_path(&base, &ctx.state.library).map_err(|_| libc::EINVAL)
}

/// A doctored environment that owns its strings and can provide a C-style envp.
///
/// This struct manages the memory for environment strings and provides a
/// null-terminated array of pointers suitable for passing to exec functions.
pub struct DoctoredEnvironment {
    /// The environment strings in "KEY=VALUE" format
    /// This field is kept to ensure the CStrings remain valid while ptrs references them.
    #[allow(dead_code)]
    strings: Vec<CString>,
    /// Pointers to the strings, plus a null terminator
    ptrs: Vec<*const c_char>,
}

impl DoctoredEnvironment {
    /// Private constructor that creates a DoctoredEnvironment from a vector of CStrings.
    ///
    /// This builds the pointer array from the strings.
    fn from_strings(strings: Vec<CString>) -> Self {
        let mut ptrs: Vec<*const c_char> = strings.iter().map(|s| s.as_ptr()).collect();
        ptrs.push(std::ptr::null());

        DoctoredEnvironment { strings, ptrs }
    }

    /// Create a doctored environment from a session context and the received environment.
    ///
    /// Returns an error if any key or value contains a null byte.
    ///
    /// The method is called when `in_session` returned false, therefore the
    /// environment is not aligned with the preload mode settings. The result environment
    /// will be aligned with the preload mode settings based on the context.
    pub fn from_envp(ctx: &SessionContext, envp: *const *const c_char) -> Result<Self, c_int> {
        let mut strings = Vec::new();
        let mut original_preload_value: Option<String> = None;

        // Copy existing environment variables, skipping ones we'll set ourselves.
        // Uses envp_iter for consistent parsing and malformed-entry logging.
        if !envp.is_null() {
            for (key, value, full_entry) in unsafe { envp_iter(envp) } {
                #[cfg(target_os = "macos")]
                let dominated_by_bear =
                    key == KEY_INTERCEPT_STATE || key == PRELOAD_KEY || key == KEY_OS__MACOS_FLAT_NAMESPACE;
                #[cfg(not(target_os = "macos"))]
                let dominated_by_bear = key == KEY_INTERCEPT_STATE || key == PRELOAD_KEY;

                if key == PRELOAD_KEY {
                    original_preload_value = Some(value.to_string());
                }

                if !dominated_by_bear {
                    strings.push(CString::new(full_entry).map_err(|_| libc::EINVAL)?);
                }
            }
        }

        // Add KEY_INTERCEPT_STATE with serialized state (JSON)
        let state_json: String = ctx.state.clone().try_into().map_err(|_| libc::EINVAL)?;
        let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);
        strings.push(CString::new(intercept_entry).map_err(|_| libc::EINVAL)?);

        // Add PRELOAD_KEY with the library path inserted at front.
        // Uses the shared policy: fall back to startup snapshot when envp had no preload key.
        let preload_value = desired_preload_value(ctx, original_preload_value.as_deref())?;
        let preload_entry = format!("{}={}", PRELOAD_KEY, preload_value);
        strings.push(CString::new(preload_entry).map_err(|_| libc::EINVAL)?);

        // On macOS, add the flat namespace flag
        #[cfg(target_os = "macos")]
        {
            let flat_namespace_entry = format!("{}=1", KEY_OS__MACOS_FLAT_NAMESPACE);
            strings.push(CString::new(flat_namespace_entry).map_err(|_| libc::EINVAL)?);
        }

        Ok(Self::from_strings(strings))
    }

    /// Get a pointer to the envp array suitable for passing to exec functions.
    pub fn as_ptr(&self) -> *const *const c_char {
        self.ptrs.as_ptr()
    }
}

/// Initialize the global session from a C-style envp array.
///
/// Returns the destination address if present (for reporter initialization).
///
/// # Safety
/// The `envp` pointer must be a valid null-terminated array of null-terminated
/// C strings in "KEY=VALUE" format, or null.
pub unsafe fn init_session_from_envp(envp: *const *const c_char) -> Option<SocketAddr> {
    if envp.is_null() {
        log::info!("session init failed: environ is null pointer");
        return None;
    }

    match unsafe { from_envp(envp) } {
        None => {
            log::info!("session init failed: variables not found");
            None
        }
        Some(session) => {
            let destination = session.destination;

            // Capture and normalize the preload variable before anything modifies it.
            // This preserves co-resident libraries (e.g. libsandbox.so).
            let startup_preload = unsafe { read_env_value(envp, PRELOAD_KEY) }.map(|v| normalize_preload(&v));

            let ctx = SessionContext { state: session, startup_preload };

            if SESSION_CTX.set(ctx).is_err() {
                log::debug!("SESSION_CTX already set, ignoring duplicate init");
            }

            Some(destination)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::path::PathBuf;

    use bear::intercept::environment::PreloadState;

    /// Default test destination address (localhost:12345).
    const TEST_DESTINATION: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345);

    /// Default test library path.
    const TEST_LIBRARY_PATH: &str = "/usr/lib/libear.so";

    /// Helper struct to manage C-style environment arrays for testing.
    /// This ensures the CStrings stay alive while we use pointers to them.
    struct TestEnvp {
        #[allow(dead_code)]
        strings: Vec<CString>,
        ptrs: Vec<*const c_char>,
    }

    impl TestEnvp {
        /// Create a TestEnvp from a slice of "KEY=VALUE" strings.
        fn new(entries: &[&str]) -> Self {
            let strings: Vec<CString> = entries.iter().map(|s| CString::new(*s).unwrap()).collect();
            let mut ptrs: Vec<*const c_char> = strings.iter().map(|s| s.as_ptr()).collect();
            ptrs.push(std::ptr::null()); // Null terminator
            TestEnvp { strings, ptrs }
        }

        /// Get a pointer to the envp array.
        fn as_ptr(&self) -> *const *const c_char {
            self.ptrs.as_ptr()
        }
    }

    /// Helper function to create a test PreloadState with default destination and library.
    fn create_test_state() -> PreloadState {
        PreloadState { destination: TEST_DESTINATION, library: PathBuf::from(TEST_LIBRARY_PATH) }
    }

    /// Helper function to create a test SessionContext with no startup preload.
    fn create_test_ctx() -> SessionContext {
        SessionContext { state: create_test_state(), startup_preload: None }
    }

    /// Helper function to serialize a PreloadState to JSON for environment variable.
    fn state_to_json(state: &PreloadState) -> String {
        serde_json::to_string(state).unwrap()
    }

    #[test]
    fn test_from_envp_returns_none_for_null_pointer() {
        let result = unsafe { from_envp(std::ptr::null()) };
        assert!(result.is_none());
    }

    #[test]
    fn test_from_envp_returns_none_when_intercept_state_not_found() {
        let envp = TestEnvp::new(&["PATH=/usr/bin", "HOME=/home/user", "SHELL=/bin/bash"]);

        let result = unsafe { from_envp(envp.as_ptr()) };
        assert!(result.is_none());
    }

    #[test]
    fn test_from_envp_returns_none_for_empty_environment() {
        let envp = TestEnvp::new(&[]);

        let result = unsafe { from_envp(envp.as_ptr()) };
        assert!(result.is_none());
    }

    #[test]
    fn test_from_envp_returns_none_for_invalid_json() {
        let envp = TestEnvp::new(&["PATH=/usr/bin", "BEAR_INTERCEPT=invalid_json_here", "HOME=/home/user"]);

        let result = unsafe { from_envp(envp.as_ptr()) };
        assert!(result.is_none());
    }

    #[test]
    fn test_from_envp_returns_state_when_found() {
        let state = create_test_state();
        let envp = {
            let state_json = state_to_json(&state);
            let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);
            TestEnvp::new(&["PATH=/usr/bin", &intercept_entry, "HOME=/home/user"])
        };

        let result = unsafe { from_envp(envp.as_ptr()) };
        assert!(result.is_some());

        let parsed_state = result.unwrap();
        assert_eq!(parsed_state, state);
    }

    #[test]
    fn test_from_envp_finds_state_at_beginning() {
        let state = create_test_state();
        let envp = {
            let state_json = state_to_json(&state);
            let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);
            TestEnvp::new(&[&intercept_entry, "PATH=/usr/bin"])
        };

        let result = unsafe { from_envp(envp.as_ptr()) };
        assert!(result.is_some());
        assert_eq!(result.unwrap(), state);
    }

    #[test]
    fn test_from_envp_finds_state_at_end() {
        let state = create_test_state();
        let envp = {
            let state_json = state_to_json(&state);
            let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);
            TestEnvp::new(&["PATH=/usr/bin", "HOME=/home/user", &intercept_entry])
        };

        let result = unsafe { from_envp(envp.as_ptr()) };
        assert!(result.is_some());
        assert_eq!(result.unwrap(), state);
    }

    #[test]
    fn test_in_session_returns_false_for_null_pointer() {
        let ctx = create_test_ctx();
        let result = unsafe { in_session(&ctx, std::ptr::null()) };
        assert!(!result);
    }

    #[test]
    fn test_in_session_returns_false_when_intercept_state_missing() {
        let ctx = create_test_ctx();
        let preload_entry = format!("{}={}", PRELOAD_KEY, TEST_LIBRARY_PATH);

        let envp = TestEnvp::new(&["PATH=/usr/bin", &preload_entry]);

        let result = unsafe { in_session(&ctx, envp.as_ptr()) };
        assert!(!result);
    }

    #[test]
    fn test_in_session_returns_false_when_preload_missing() {
        let ctx = create_test_ctx();
        let state_json = state_to_json(&ctx.state);
        let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);

        let envp = TestEnvp::new(&["PATH=/usr/bin", &intercept_entry]);

        let result = unsafe { in_session(&ctx, envp.as_ptr()) };
        assert!(!result);
    }

    #[test]
    fn test_in_session_returns_false_when_state_differs() {
        let ctx = create_test_ctx();
        let envp = {
            let different_destination = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 54321);
            let different_state = PreloadState {
                destination: different_destination,
                library: PathBuf::from(TEST_LIBRARY_PATH),
            };
            let state_json = state_to_json(&different_state);
            let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);
            let preload_entry = format!("{}={}", PRELOAD_KEY, TEST_LIBRARY_PATH);

            TestEnvp::new(&[&intercept_entry, &preload_entry])
        };

        let result = unsafe { in_session(&ctx, envp.as_ptr()) };
        assert!(!result);
    }

    #[test]
    fn test_in_session_returns_false_when_library_not_first_in_preload() {
        let ctx = create_test_ctx();
        let envp = {
            let state_json = state_to_json(&ctx.state);
            let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);
            // Library is second in the preload path
            let preload_entry = format!("{}=/other/lib.so:{}", PRELOAD_KEY, TEST_LIBRARY_PATH);

            TestEnvp::new(&[&intercept_entry, &preload_entry])
        };

        let result = unsafe { in_session(&ctx, envp.as_ptr()) };
        assert!(!result);
    }

    #[test]
    fn test_in_session_returns_true_when_all_matches() {
        let ctx = create_test_ctx();
        let envp = {
            let state_json = state_to_json(&ctx.state);
            let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);
            let preload_entry = format!("{}={}", PRELOAD_KEY, TEST_LIBRARY_PATH);

            TestEnvp::new(&["PATH=/usr/bin", &intercept_entry, &preload_entry])
        };

        let result = unsafe { in_session(&ctx, envp.as_ptr()) };
        assert!(result);
    }

    #[test]
    fn test_in_session_returns_true_with_library_first_among_multiple() {
        let ctx = create_test_ctx();
        let envp = {
            let state_json = state_to_json(&ctx.state);
            let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);
            // Library is first but there are other libraries too
            let preload_entry =
                format!("{}={}:/other/lib.so:/another/lib.so", PRELOAD_KEY, TEST_LIBRARY_PATH);

            TestEnvp::new(&[&intercept_entry, &preload_entry])
        };

        let result = unsafe { in_session(&ctx, envp.as_ptr()) };
        assert!(result);
    }

    #[test]
    fn test_in_session_returns_false_for_empty_environment() {
        let ctx = create_test_ctx();
        let envp = TestEnvp::new(&[]);

        let result = unsafe { in_session(&ctx, envp.as_ptr()) };
        assert!(!result);
    }

    #[test]
    fn test_doctored_environment_from_null_envp() {
        let ctx = create_test_ctx();

        let result = DoctoredEnvironment::from_envp(&ctx, std::ptr::null());
        assert!(result.is_ok());

        let doctored = result.unwrap();
        // Should have at least the intercept state and preload entries
        assert!(!doctored.ptrs.is_empty());
        // Last pointer should be null
        assert!(doctored.ptrs.last().unwrap().is_null());
    }

    #[test]
    fn test_doctored_environment_preserves_other_variables() {
        let ctx = create_test_ctx();
        let envp = TestEnvp::new(&["PATH=/usr/bin", "HOME=/home/user", "SHELL=/bin/bash"]);

        let result = DoctoredEnvironment::from_envp(&ctx, envp.as_ptr());
        assert!(result.is_ok());

        let doctored = result.unwrap();
        let env_strings: Vec<String> =
            doctored.strings.iter().map(|s| s.to_string_lossy().to_string()).collect();

        // Check that original variables are preserved
        assert!(env_strings.iter().any(|s| s == "PATH=/usr/bin"));
        assert!(env_strings.iter().any(|s| s == "HOME=/home/user"));
        assert!(env_strings.iter().any(|s| s == "SHELL=/bin/bash"));
    }

    #[test]
    fn test_doctored_environment_adds_intercept_state() {
        let ctx = create_test_ctx();
        let envp = TestEnvp::new(&["PATH=/usr/bin"]);

        let result = DoctoredEnvironment::from_envp(&ctx, envp.as_ptr());
        assert!(result.is_ok());

        let doctored = result.unwrap();
        let env_strings: Vec<String> =
            doctored.strings.iter().map(|s| s.to_string_lossy().to_string()).collect();

        // Check that intercept state is added
        let has_intercept = env_strings.iter().any(|s| s.starts_with(&format!("{}=", KEY_INTERCEPT_STATE)));
        assert!(has_intercept, "Should contain BEAR_INTERCEPT variable");
    }

    #[test]
    fn test_doctored_environment_adds_preload_key() {
        let ctx = create_test_ctx();
        let envp = TestEnvp::new(&["PATH=/usr/bin"]);

        let result = DoctoredEnvironment::from_envp(&ctx, envp.as_ptr());
        assert!(result.is_ok());

        let doctored = result.unwrap();
        let env_strings: Vec<String> =
            doctored.strings.iter().map(|s| s.to_string_lossy().to_string()).collect();

        // Check that preload key is added
        let has_preload = env_strings.iter().any(|s| s.starts_with(&format!("{}=", PRELOAD_KEY)));
        assert!(has_preload, "Should contain {} variable", PRELOAD_KEY);
    }

    #[test]
    fn test_doctored_environment_library_first_in_preload() {
        let ctx = create_test_ctx();
        let envp = {
            let existing_preload = format!("{}=/other/lib.so:/another/lib.so", PRELOAD_KEY);
            TestEnvp::new(&["PATH=/usr/bin", &existing_preload])
        };

        let result = DoctoredEnvironment::from_envp(&ctx, envp.as_ptr());
        assert!(result.is_ok());

        let doctored = result.unwrap();
        let env_strings: Vec<String> =
            doctored.strings.iter().map(|s| s.to_string_lossy().to_string()).collect();

        // Find the preload entry
        let preload_entry = env_strings
            .iter()
            .find(|s| s.starts_with(&format!("{}=", PRELOAD_KEY)))
            .expect("Should have preload entry");

        // Extract the value and check first path
        let value = preload_entry.split_once('=').unwrap().1;
        let paths: Vec<PathBuf> = std::env::split_paths(value).collect();
        assert_eq!(
            paths.first().unwrap(),
            &PathBuf::from(TEST_LIBRARY_PATH),
            "Library should be first in preload path"
        );
    }

    #[test]
    fn test_doctored_environment_as_ptr_returns_valid_pointer() {
        let ctx = create_test_ctx();
        let envp = TestEnvp::new(&["PATH=/usr/bin"]);

        let result = DoctoredEnvironment::from_envp(&ctx, envp.as_ptr());
        assert!(result.is_ok());

        let doctored = result.unwrap();
        let ptr = doctored.as_ptr();

        // Pointer should not be null
        assert!(!ptr.is_null());

        // Should be able to iterate through the envp
        let mut count = 0;
        let mut current = ptr;
        unsafe {
            while !(*current).is_null() {
                count += 1;
                current = current.add(1);
            }
        }
        assert!(count > 0, "Should have at least one environment entry");
    }

    #[test]
    fn test_doctored_environment_null_terminated() {
        let ctx = create_test_ctx();
        let envp = TestEnvp::new(&["PATH=/usr/bin", "HOME=/home/user"]);

        let result = DoctoredEnvironment::from_envp(&ctx, envp.as_ptr());
        assert!(result.is_ok());

        let doctored = result.unwrap();

        // The last element in ptrs should be null
        assert!(doctored.ptrs.last().unwrap().is_null());
    }

    #[test]
    fn test_init_session_from_envp_returns_none_for_null() {
        // Note: We can't easily test SESSION_CTX initialization multiple times
        // since it's a OnceLock. We test the return value behavior.
        let result = unsafe { init_session_from_envp(std::ptr::null()) };
        assert!(result.is_none());
    }

    #[test]
    fn test_init_session_from_envp_returns_none_when_not_found() {
        let envp = TestEnvp::new(&["PATH=/usr/bin", "HOME=/home/user"]);

        let result = unsafe { init_session_from_envp(envp.as_ptr()) };
        assert!(result.is_none());
    }

    #[test]
    fn test_from_envp_handles_malformed_entry_without_equals() {
        let state = create_test_state();
        let envp = {
            let state_json = state_to_json(&state);
            let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);

            // Include a malformed entry (no equals sign)
            TestEnvp::new(&["MALFORMED_ENTRY", &intercept_entry, "PATH=/usr/bin"])
        };

        let result = unsafe { from_envp(envp.as_ptr()) };
        // Should still find the valid state
        assert!(result.is_some());
        assert_eq!(result.unwrap(), state);
    }

    #[test]
    fn test_in_session_handles_malformed_entry_without_equals() {
        let ctx = create_test_ctx();
        let envp = {
            let state_json = state_to_json(&ctx.state);
            let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);
            let preload_entry = format!("{}={}", PRELOAD_KEY, TEST_LIBRARY_PATH);

            TestEnvp::new(&["MALFORMED", &intercept_entry, &preload_entry])
        };

        let result = unsafe { in_session(&ctx, envp.as_ptr()) };
        assert!(result);
    }

    #[test]
    fn test_doctored_environment_handles_entry_with_multiple_equals() {
        let ctx = create_test_ctx();
        // Entry with value containing equals signs
        let envp = TestEnvp::new(&["MY_VAR=value=with=equals", "PATH=/usr/bin"]);

        let result = DoctoredEnvironment::from_envp(&ctx, envp.as_ptr());
        assert!(result.is_ok());

        let doctored = result.unwrap();
        let env_strings: Vec<String> =
            doctored.strings.iter().map(|s| s.to_string_lossy().to_string()).collect();

        // The variable with multiple equals should be preserved correctly
        // split_once only splits on the first '='
        assert!(env_strings.iter().any(|s| s == "MY_VAR=value=with=equals"));
    }

    #[test]
    fn test_from_envp_with_ipv6_address() {
        let ipv6_addr = SocketAddr::new(IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 8080);
        let state = PreloadState { destination: ipv6_addr, library: PathBuf::from(TEST_LIBRARY_PATH) };
        let envp = {
            let state_json = state_to_json(&state);
            let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);

            TestEnvp::new(&[&intercept_entry])
        };

        let result = unsafe { from_envp(envp.as_ptr()) };
        assert!(result.is_some());

        let parsed = result.unwrap();
        assert_eq!(parsed, state);
        assert!(parsed.destination.ip().is_ipv6());
    }

    #[test]
    fn test_doctored_environment_from_strings() {
        // Test the private from_strings method indirectly through from_envp
        let ctx = create_test_ctx();
        let envp = TestEnvp::new(&[]);

        let result = DoctoredEnvironment::from_envp(&ctx, envp.as_ptr());
        assert!(result.is_ok());

        let doctored = result.unwrap();
        // Verify strings and ptrs are consistent
        assert_eq!(doctored.ptrs.len(), doctored.strings.len() + 1); // +1 for null terminator
    }

    #[test]
    fn test_normalize_preload_deduplicates() {
        let result = normalize_preload("/a.so:/b.so:/a.so");
        assert_eq!(result, vec![PathBuf::from("/a.so"), PathBuf::from("/b.so")]);
    }

    #[test]
    fn test_normalize_preload_removes_empty_segments() {
        let result = normalize_preload("/a.so::/b.so:");
        assert_eq!(result, vec![PathBuf::from("/a.so"), PathBuf::from("/b.so")]);
    }

    #[test]
    fn test_normalize_preload_preserves_order() {
        let result = normalize_preload("/c.so:/a.so:/b.so");
        assert_eq!(result, vec![PathBuf::from("/c.so"), PathBuf::from("/a.so"), PathBuf::from("/b.so")]);
    }

    #[test]
    fn test_desired_preload_value_with_startup_snapshot() {
        let ctx = SessionContext {
            state: create_test_state(),
            startup_preload: Some(vec![PathBuf::from("/other/lib.so")]),
        };
        // When current is absent, should fall back to startup snapshot
        let result = desired_preload_value(&ctx, None).unwrap();
        let paths: Vec<PathBuf> = std::env::split_paths(&result).collect();
        assert_eq!(paths[0], PathBuf::from(TEST_LIBRARY_PATH));
        assert!(paths.contains(&PathBuf::from("/other/lib.so")));
    }

    #[test]
    fn test_desired_preload_value_with_current() {
        let ctx = create_test_ctx();
        // When current has a value, should prepend Bear's library
        let result = desired_preload_value(&ctx, Some("/existing/lib.so")).unwrap();
        let paths: Vec<PathBuf> = std::env::split_paths(&result).collect();
        assert_eq!(paths[0], PathBuf::from(TEST_LIBRARY_PATH));
        assert!(paths.contains(&PathBuf::from("/existing/lib.so")));
    }

    #[test]
    fn test_desired_preload_value_empty_current_falls_back() {
        let ctx = SessionContext {
            state: create_test_state(),
            startup_preload: Some(vec![PathBuf::from("/snapshot/lib.so")]),
        };
        // Empty current should fall back to startup snapshot
        let result = desired_preload_value(&ctx, Some("")).unwrap();
        let paths: Vec<PathBuf> = std::env::split_paths(&result).collect();
        assert_eq!(paths[0], PathBuf::from(TEST_LIBRARY_PATH));
        assert!(paths.contains(&PathBuf::from("/snapshot/lib.so")));
    }
}
