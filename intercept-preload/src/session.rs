// SPDX-License-Identifier: GPL-3.0-or-later

//! Session management and environment doctoring for the preload library.
//!
//! This module handles:
//! - Capturing environment variables at library load time
//! - Storing session state (preload path, destination address)
//! - "Doctoring" environments to ensure child processes continue interception
//!
//! The environment doctoring ensures that `LD_PRELOAD` (or `DYLD_INSERT_LIBRARIES`
//! on macOS) and `INTERCEPT_COLLECTOR_ADDRESS` are preserved across exec calls,
//! even if the build system attempts to clear them.

use std::ffi::{CStr, CString};
use std::net::SocketAddr;
use std::sync::OnceLock;

use bear::environment::KEY_INTERCEPT_STATE;
#[cfg(not(target_os = "macos"))]
use bear::environment::KEY_OS__PRELOAD_PATH;
#[cfg(target_os = "macos")]
use bear::environment::{KEY_OS__MACOS_FLAT_NAMESPACE, KEY_OS__MACOS_PRELOAD_PATH};
use bear::intercept::environment::{PreloadState, insert_to_path};
use libc::{c_char, c_int};

/// The preload environment variable key for the current platform.
#[cfg(target_os = "macos")]
pub const PRELOAD_KEY: &str = KEY_OS__MACOS_PRELOAD_PATH;
#[cfg(not(target_os = "macos"))]
pub const PRELOAD_KEY: &str = KEY_OS__PRELOAD_PATH;

/// Global session storage, initialized once from the C constructor.
pub static SESSION: OnceLock<PreloadState> = OnceLock::new();

/// Create a PreloadState by extracting the intercept state from a C-style envp array.
///
/// This walks through the C pointers directly to find the `KEY_INTERCEPT_STATE`
/// environment variable and attempts to parse it into a `PreloadState`.
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

    let mut ptr = envp;
    while !unsafe { (*ptr).is_null() } {
        let cstr = unsafe { CStr::from_ptr(*ptr) };
        if let Ok(key_and_value) = cstr.to_str()
            && let Some((key, value)) = key_and_value.split_once('=')
            && key == KEY_INTERCEPT_STATE
        {
            return value.try_into().ok();
        }
        ptr = unsafe { ptr.add(1) };
    }

    None
}

/// The method does check the passed environment if still aligned with the expected
/// preload mode environment settings.
///
/// This walks through the C pointers directly and check if the `KEY_INTERCEPT_STATE`
/// and `LD_PRELOAD` variables are all set as expected. Returns true if the variables
/// are not changed.
pub unsafe fn in_session(state: PreloadState, envp: *const *const c_char) -> bool {
    if envp.is_null() {
        return false;
    }

    let mut intercept_state_matches = false;
    let mut preload_matches = false;

    let mut ptr = envp;
    while !unsafe { (*ptr).is_null() } {
        let cstr = unsafe { CStr::from_ptr(*ptr) };
        if let Ok(key_and_value) = cstr.to_str()
            && let Some((key, value)) = key_and_value.split_once('=')
        {
            if key == KEY_INTERCEPT_STATE {
                if let Ok(parsed_state) = PreloadState::try_from(value) {
                    // expect full match on this
                    intercept_state_matches = parsed_state == state;
                }
            } else if key == PRELOAD_KEY {
                preload_matches = {
                    // we check if our library is at the first position
                    let paths = std::env::split_paths(value);
                    paths.into_iter().next() == Some(state.library.clone())
                };
            }
        }
        ptr = unsafe { ptr.add(1) };
    }

    intercept_state_matches && preload_matches
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

    /// Create a doctored environment from a preload state and the received environment.
    ///
    /// Returns an error if any key or value contains a null byte.
    ///
    /// The method is called when the `in_session` was returning false, therefore the
    /// environment is not aligned with the preload mode settings. The result environment
    /// will be aligned with the preload mode settings based on the state.
    pub fn from_envp(state: PreloadState, envp: *const *const c_char) -> Result<Self, c_int> {
        use std::ffi::CString;

        let mut strings = Vec::new();
        let mut original_preload_value = String::new();

        // First, copy existing environment variables, skipping ones we'll set ourselves
        if !envp.is_null() {
            let mut ptr = envp;
            while !unsafe { (*ptr).is_null() } {
                let cstr = unsafe { CStr::from_ptr(*ptr) };
                if let Ok(key_and_value) = cstr.to_str()
                    && let Some((key, value)) = key_and_value.split_once('=')
                {
                    #[cfg(target_os = "macos")]
                    let dominated_by_bear = key == KEY_INTERCEPT_STATE
                        || key == PRELOAD_KEY
                        || key == KEY_OS__MACOS_FLAT_NAMESPACE;
                    #[cfg(not(target_os = "macos"))]
                    let dominated_by_bear = key == KEY_INTERCEPT_STATE || key == PRELOAD_KEY;

                    if key == PRELOAD_KEY {
                        // Save the original preload value for later doctoring
                        original_preload_value = value.to_string();
                    }

                    if !dominated_by_bear {
                        // Keep other variables as-is
                        strings.push(CString::new(key_and_value).map_err(|_| libc::EINVAL)?);
                    }
                }
                ptr = unsafe { ptr.add(1) };
            }
        }

        // Add KEY_INTERCEPT_STATE with serialized state (JSON)
        let state_json: String = state.clone().try_into().map_err(|_| libc::EINVAL)?;
        let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);
        strings.push(CString::new(intercept_entry).map_err(|_| libc::EINVAL)?);

        // Add PRELOAD_KEY with the library path inserted at front
        let preload_value =
            insert_to_path(&original_preload_value, &state.library).map_err(|_| libc::EINVAL)?;
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
            let _ = SESSION.set(session);

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
        let state = create_test_state();
        let result = unsafe { in_session(state, std::ptr::null()) };
        assert!(!result);
    }

    #[test]
    fn test_in_session_returns_false_when_intercept_state_missing() {
        let state = create_test_state();
        let preload_entry = format!("{}={}", PRELOAD_KEY, TEST_LIBRARY_PATH);

        let envp = TestEnvp::new(&["PATH=/usr/bin", &preload_entry]);

        let result = unsafe { in_session(state, envp.as_ptr()) };
        assert!(!result);
    }

    #[test]
    fn test_in_session_returns_false_when_preload_missing() {
        let state = create_test_state();
        let state_json = state_to_json(&state);
        let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);

        let envp = TestEnvp::new(&["PATH=/usr/bin", &intercept_entry]);

        let result = unsafe { in_session(state, envp.as_ptr()) };
        assert!(!result);
    }

    #[test]
    fn test_in_session_returns_false_when_state_differs() {
        let state = create_test_state();
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

        let result = unsafe { in_session(state, envp.as_ptr()) };
        assert!(!result);
    }

    #[test]
    fn test_in_session_returns_false_when_library_not_first_in_preload() {
        let state = create_test_state();
        let envp = {
            let state_json = state_to_json(&state);
            let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);
            // Library is second in the preload path
            let preload_entry = format!("{}=/other/lib.so:{}", PRELOAD_KEY, TEST_LIBRARY_PATH);

            TestEnvp::new(&[&intercept_entry, &preload_entry])
        };

        let result = unsafe { in_session(state, envp.as_ptr()) };
        assert!(!result);
    }

    #[test]
    fn test_in_session_returns_true_when_all_matches() {
        let state = create_test_state();
        let envp = {
            let state_json = state_to_json(&state);
            let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);
            let preload_entry = format!("{}={}", PRELOAD_KEY, TEST_LIBRARY_PATH);

            TestEnvp::new(&["PATH=/usr/bin", &intercept_entry, &preload_entry])
        };

        let result = unsafe { in_session(state, envp.as_ptr()) };
        assert!(result);
    }

    #[test]
    fn test_in_session_returns_true_with_library_first_among_multiple() {
        let state = create_test_state();
        let envp = {
            let state_json = state_to_json(&state);
            let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);
            // Library is first but there are other libraries too
            let preload_entry =
                format!("{}={}:/other/lib.so:/another/lib.so", PRELOAD_KEY, TEST_LIBRARY_PATH);

            TestEnvp::new(&[&intercept_entry, &preload_entry])
        };

        let result = unsafe { in_session(state, envp.as_ptr()) };
        assert!(result);
    }

    #[test]
    fn test_in_session_returns_false_for_empty_environment() {
        let state = create_test_state();
        let envp = TestEnvp::new(&[]);

        let result = unsafe { in_session(state, envp.as_ptr()) };
        assert!(!result);
    }

    #[test]
    fn test_doctored_environment_from_null_envp() {
        let state = create_test_state();

        let result = DoctoredEnvironment::from_envp(state, std::ptr::null());
        assert!(result.is_ok());

        let doctored = result.unwrap();
        // Should have at least the intercept state and preload entries
        assert!(!doctored.ptrs.is_empty());
        // Last pointer should be null
        assert!(doctored.ptrs.last().unwrap().is_null());
    }

    #[test]
    fn test_doctored_environment_preserves_other_variables() {
        let state = create_test_state();
        let envp = TestEnvp::new(&["PATH=/usr/bin", "HOME=/home/user", "SHELL=/bin/bash"]);

        let result = DoctoredEnvironment::from_envp(state, envp.as_ptr());
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
        let state = create_test_state();
        let envp = TestEnvp::new(&["PATH=/usr/bin"]);

        let result = DoctoredEnvironment::from_envp(state.clone(), envp.as_ptr());
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
        let state = create_test_state();
        let envp = TestEnvp::new(&["PATH=/usr/bin"]);

        let result = DoctoredEnvironment::from_envp(state, envp.as_ptr());
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
        let state = create_test_state();
        let envp = {
            let existing_preload = format!("{}=/other/lib.so:/another/lib.so", PRELOAD_KEY);
            TestEnvp::new(&["PATH=/usr/bin", &existing_preload])
        };

        let result = DoctoredEnvironment::from_envp(state, envp.as_ptr());
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
        let state = create_test_state();
        let envp = TestEnvp::new(&["PATH=/usr/bin"]);

        let result = DoctoredEnvironment::from_envp(state, envp.as_ptr());
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
        let state = create_test_state();
        let envp = TestEnvp::new(&["PATH=/usr/bin", "HOME=/home/user"]);

        let result = DoctoredEnvironment::from_envp(state, envp.as_ptr());
        assert!(result.is_ok());

        let doctored = result.unwrap();

        // The last element in ptrs should be null
        assert!(doctored.ptrs.last().unwrap().is_null());
    }

    #[test]
    fn test_init_session_from_envp_returns_none_for_null() {
        // Note: We can't easily test SESSION initialization multiple times
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
        let state = create_test_state();
        let envp = {
            let state_json = state_to_json(&state);
            let intercept_entry = format!("{}={}", KEY_INTERCEPT_STATE, state_json);
            let preload_entry = format!("{}={}", PRELOAD_KEY, TEST_LIBRARY_PATH);

            TestEnvp::new(&["MALFORMED", &intercept_entry, &preload_entry])
        };

        let result = unsafe { in_session(state, envp.as_ptr()) };
        assert!(result);
    }

    #[test]
    fn test_doctored_environment_handles_entry_with_multiple_equals() {
        let state = create_test_state();
        // Entry with value containing equals signs
        let envp = TestEnvp::new(&["MY_VAR=value=with=equals", "PATH=/usr/bin"]);

        let result = DoctoredEnvironment::from_envp(state, envp.as_ptr());
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
        let state = create_test_state();
        let envp = TestEnvp::new(&[]);

        let result = DoctoredEnvironment::from_envp(state, envp.as_ptr());
        assert!(result.is_ok());

        let doctored = result.unwrap();
        // Verify strings and ptrs are consistent
        assert_eq!(doctored.ptrs.len(), doctored.strings.len() + 1); // +1 for null terminator
    }
}
