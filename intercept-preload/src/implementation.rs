// SPDX-License-Identifier: GPL-3.0-or-later

//! This file implements the Rust side of the preload library interception.
//!
//! All exported functions are prefixed with `rust_` and are called from the C shim
//! (`src/c/shim.c`). This separation exists because:
//!
//! 1. Stable Rust cannot handle C variadic arguments (execl family)
//! 2. On FreeBSD, libc functions may call each other internally. By having all
//!    exported symbols in C call into Rust (which uses dlsym(RTLD_NEXT, ...)),
//!    we avoid recursive interception issues.
//!
//! Each `rust_*` function:
//! - Reports the execution to the collector
//! - Calls the real function via dlsym(RTLD_NEXT, ...)

use std::collections::HashMap;
use std::ffi::CStr;

use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use bear::intercept::reporter::{Reporter, ReporterFactory};
use bear::intercept::tcp::ReporterOnTcp;
use bear::intercept::{Event, Execution};
use ctor::ctor;
use libc::{c_char, c_int, pid_t, posix_spawn_file_actions_t, posix_spawnattr_t};

// =============================================================================
// Function pointer types for the original functions
// =============================================================================

#[cfg(has_symbol_execve)]
type ExecveFunc = unsafe extern "C" fn(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int;

#[cfg(has_symbol_execv)]
type ExecvFunc = unsafe extern "C" fn(path: *const c_char, argv: *const *const c_char) -> c_int;

#[cfg(has_symbol_execvpe)]
type ExecvpeFunc = unsafe extern "C" fn(
    file: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int;

#[cfg(has_symbol_execvp)]
type ExecvpFunc = unsafe extern "C" fn(file: *const c_char, argv: *const *const c_char) -> c_int;

#[cfg(has_symbol_execvP)]
type ExecvPFunc = unsafe extern "C" fn(
    file: *const c_char,
    search_path: *const c_char,
    argv: *const *const c_char,
) -> c_int;

#[cfg(has_symbol_exect)]
type ExectFunc = unsafe extern "C" fn(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int;

#[cfg(has_symbol_posix_spawn)]
type PosixSpawnFunc = unsafe extern "C" fn(
    pid: *mut pid_t,
    path: *const c_char,
    file_actions: *const posix_spawn_file_actions_t,
    attrp: *const posix_spawnattr_t,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int;

#[cfg(has_symbol_posix_spawnp)]
type PosixSpawnpFunc = unsafe extern "C" fn(
    pid: *mut pid_t,
    file: *const c_char,
    file_actions: *const posix_spawn_file_actions_t,
    attrp: *const posix_spawnattr_t,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int;

#[cfg(has_symbol_popen)]
type PopenFunc = unsafe extern "C" fn(command: *const c_char, mode: *const c_char) -> *mut libc::FILE;

#[cfg(has_symbol_system)]
type SystemFunc = unsafe extern "C" fn(command: *const c_char) -> c_int;

// =============================================================================
// Dynamic loading related constants
// =============================================================================

const RTLD_NEXT: *mut libc::c_void = -1isize as *mut libc::c_void;

// =============================================================================
// Constructor and initialization
// =============================================================================

/// Constructor function that is called when the library is loaded
///
/// # Safety
/// This function is unsafe because it modifies global state.
#[ctor]
unsafe fn on_load() {
    #[cfg(not(test))]
    {
        use std::io::Write;

        let pid = std::process::id();
        env_logger::Builder::from_default_env()
            .format(move |buf, record| {
                let timestamp = buf.timestamp();
                writeln!(buf, "[{timestamp} preload/{pid}] {}", record.args())
            })
            .init();
    }
    log::debug!("Initializing intercept-preload library");
}

// =============================================================================
// Static variables to hold original function pointers
// =============================================================================

#[ctor]
static REAL_EXECVE: AtomicPtr<libc::c_void> = {
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"execve".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor]
static REAL_EXECV: AtomicPtr<libc::c_void> = {
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"execv".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor]
static REAL_EXECVPE: AtomicPtr<libc::c_void> = {
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"execvpe".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor]
static REAL_EXECVP: AtomicPtr<libc::c_void> = {
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"execvp".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor]
static REAL_EXECVP_OPENBSD: AtomicPtr<libc::c_void> = {
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"execvP".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor]
static REAL_EXECT: AtomicPtr<libc::c_void> = {
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"exect".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor]
static REAL_POSIX_SPAWN: AtomicPtr<libc::c_void> = {
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"posix_spawn".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor]
static REAL_POSIX_SPAWNP: AtomicPtr<libc::c_void> = {
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"posix_spawnp".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor]
static REAL_POPEN: AtomicPtr<libc::c_void> = {
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"popen".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor]
static REAL_SYSTEM: AtomicPtr<libc::c_void> = {
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"system".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor]
static REPORTER: AtomicPtr<ReporterOnTcp> = {
    // This will capture the reporter address for later use, before any environment variable
    // might be changed.
    ReporterFactory::create_as_ptr()
};

/// Rust implementation of execv
///
/// Called from C shim for: execv, execl
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences raw pointers
/// - Calls the real execv function
#[cfg(has_symbol_execv)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_execv(path: *const c_char, argv: *const *const c_char) -> c_int {
    unsafe {
        report(|| {
            let result = Execution {
                executable: as_path_buf(path)?,
                arguments: as_string_vec(argv)?,
                working_dir: working_dir()?,
                environment: std::env::vars().collect(),
            };
            Ok(result)
        });

        let func_ptr = REAL_EXECV.load(Ordering::SeqCst);
        if !func_ptr.is_null() {
            let real_func_ptr: ExecvFunc = std::mem::transmute(func_ptr);
            real_func_ptr(path, argv)
        } else {
            log::error!("Real execv function not found");
            libc::ENOSYS
        }
    }
}

/// Rust implementation of execve
///
/// Called from C shim for: execve, execle
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences raw pointers
/// - Calls the real execve function
#[cfg(has_symbol_execve)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_execve(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    unsafe {
        report(|| {
            let result = Execution {
                executable: as_path_buf(path)?,
                arguments: as_string_vec(argv)?,
                working_dir: working_dir()?,
                environment: as_environment(envp)?,
            };
            Ok(result)
        });

        let func_ptr = REAL_EXECVE.load(Ordering::SeqCst);
        if !func_ptr.is_null() {
            let real_func_ptr: ExecveFunc = std::mem::transmute(func_ptr);
            real_func_ptr(path, argv, envp)
        } else {
            log::error!("Real execve function not found");
            libc::ENOSYS
        }
    }
}

/// Rust implementation of execvp
///
/// Called from C shim for: execvp, execlp
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences raw pointers
/// - Calls the real execvp function
#[cfg(has_symbol_execvp)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_execvp(file: *const c_char, argv: *const *const c_char) -> c_int {
    unsafe {
        report(|| {
            let result = Execution {
                executable: as_path_buf(file)?,
                arguments: as_string_vec(argv)?,
                working_dir: working_dir()?,
                environment: std::env::vars().collect(),
            };
            Ok(result)
        });

        let func_ptr = REAL_EXECVP.load(Ordering::SeqCst);
        if !func_ptr.is_null() {
            let real_func_ptr: ExecvpFunc = std::mem::transmute(func_ptr);
            real_func_ptr(file, argv)
        } else {
            log::error!("Real execvp function not found");
            libc::ENOSYS
        }
    }
}

/// Rust implementation of execvpe (GNU extension)
///
/// Called from C shim for: execvpe
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences raw pointers
/// - Calls the real execvpe function
#[cfg(has_symbol_execvpe)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_execvpe(
    file: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    unsafe {
        report(|| {
            let result = Execution {
                executable: as_path_buf(file)?,
                arguments: as_string_vec(argv)?,
                working_dir: working_dir()?,
                environment: as_environment(envp)?,
            };
            Ok(result)
        });

        let func_ptr = REAL_EXECVPE.load(Ordering::SeqCst);
        if !func_ptr.is_null() {
            let real_func_ptr: ExecvpeFunc = std::mem::transmute(func_ptr);
            real_func_ptr(file, argv, envp)
        } else {
            log::error!("Real execvpe function not found");
            libc::ENOSYS
        }
    }
}

/// Rust implementation of execvP (BSD extension)
///
/// Called from C shim for: execvP
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences raw pointers
/// - Calls the real execvP function
#[cfg(has_symbol_execvP)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_execvP(
    file: *const c_char,
    search_path: *const c_char,
    argv: *const *const c_char,
) -> c_int {
    unsafe {
        report(|| {
            let result = Execution {
                executable: as_path_buf(file)?,
                arguments: as_string_vec(argv)?,
                working_dir: working_dir()?,
                environment: std::env::vars().collect(),
            };
            Ok(result)
        });

        let func_ptr = REAL_EXECVP_OPENBSD.load(Ordering::SeqCst);
        if !func_ptr.is_null() {
            let real_func_ptr: ExecvPFunc = std::mem::transmute(func_ptr);
            real_func_ptr(file, search_path, argv)
        } else {
            log::error!("Real execvP function not found");
            libc::ENOSYS
        }
    }
}

/// Rust implementation of exect (BSD, deprecated)
///
/// Called from C shim for: exect
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences raw pointers
/// - Calls the real exect function
#[cfg(has_symbol_exect)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_exect(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    unsafe {
        report(|| {
            let result = Execution {
                executable: as_path_buf(path)?,
                arguments: as_string_vec(argv)?,
                working_dir: working_dir()?,
                environment: as_environment(envp)?,
            };
            Ok(result)
        });

        let func_ptr = REAL_EXECT.load(Ordering::SeqCst);
        if !func_ptr.is_null() {
            let real_func_ptr: ExectFunc = std::mem::transmute(func_ptr);
            real_func_ptr(path, argv, envp)
        } else {
            log::error!("Real exect function not found");
            libc::ENOSYS
        }
    }
}

/// Rust implementation of posix_spawn
///
/// Called from C shim for: posix_spawn
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences raw pointers
/// - Calls the real posix_spawn function
#[cfg(has_symbol_posix_spawn)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_posix_spawn(
    pid: *mut pid_t,
    path: *const c_char,
    file_actions: *const posix_spawn_file_actions_t,
    attrp: *const posix_spawnattr_t,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    unsafe {
        report(|| {
            let result = Execution {
                executable: as_path_buf(path)?,
                arguments: as_string_vec(argv)?,
                working_dir: working_dir()?,
                environment: as_environment(envp)?,
            };
            Ok(result)
        });

        let func_ptr = REAL_POSIX_SPAWN.load(Ordering::SeqCst);
        if !func_ptr.is_null() {
            let real_func_ptr: PosixSpawnFunc = std::mem::transmute(func_ptr);
            real_func_ptr(pid, path, file_actions, attrp, argv, envp)
        } else {
            log::error!("Real posix_spawn function not found");
            libc::ENOSYS
        }
    }
}

/// Rust implementation of posix_spawnp
///
/// Called from C shim for: posix_spawnp
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences raw pointers
/// - Calls the real posix_spawnp function
#[cfg(has_symbol_posix_spawnp)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_posix_spawnp(
    pid: *mut pid_t,
    file: *const c_char,
    file_actions: *const posix_spawn_file_actions_t,
    attrp: *const posix_spawnattr_t,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    unsafe {
        report(|| {
            let result = Execution {
                executable: as_path_buf(file)?,
                arguments: as_string_vec(argv)?,
                working_dir: working_dir()?,
                environment: as_environment(envp)?,
            };
            Ok(result)
        });

        let func_ptr = REAL_POSIX_SPAWNP.load(Ordering::SeqCst);
        if !func_ptr.is_null() {
            let real_func_ptr: PosixSpawnpFunc = std::mem::transmute(func_ptr);
            real_func_ptr(pid, file, file_actions, attrp, argv, envp)
        } else {
            log::error!("Real posix_spawnp function not found");
            libc::ENOSYS
        }
    }
}

/// Rust implementation of popen
///
/// Called from C shim for: popen
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences raw pointers (`command` and `mode`) which could be null or invalid
/// - Calls the original popen function through a function pointer
/// - Returns a raw pointer to a FILE structure
///
/// The caller must ensure that `command` and `mode` are valid null-terminated C strings.
#[cfg(has_symbol_popen)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_popen(command: *const c_char, mode: *const c_char) -> *mut libc::FILE {
    // For popen, we need to parse the shell command to extract the executable
    if !command.is_null() {
        let command_str = match unsafe { CStr::from_ptr(command) }.to_str() {
            Ok(s) => s,
            Err(_) => {
                log::warn!("Failed to parse popen command as UTF-8");
                let func_ptr = REAL_POPEN.load(Ordering::SeqCst);
                if !func_ptr.is_null() {
                    let real_func_ptr: PopenFunc = unsafe { std::mem::transmute(func_ptr) };
                    return unsafe { real_func_ptr(command, mode) };
                }
                return ptr::null_mut();
            }
        };

        // Parse the shell command - for simplicity, we'll report it as a shell execution
        report(|| {
            let result = Execution {
                executable: PathBuf::from("/bin/sh"),
                arguments: vec!["/bin/sh".to_string(), "-c".to_string(), command_str.to_string()],
                working_dir: working_dir()?,
                environment: std::env::vars().collect(),
            };
            Ok(result)
        });
    }

    let func_ptr = REAL_POPEN.load(Ordering::SeqCst);
    if !func_ptr.is_null() {
        let real_func_ptr: PopenFunc = unsafe { std::mem::transmute(func_ptr) };
        unsafe { real_func_ptr(command, mode) }
    } else {
        log::error!("Real popen function not found");
        ptr::null_mut()
    }
}

/// Rust implementation of system
///
/// Called from C shim for: system
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences a raw pointer (`command`) which could be null or invalid
/// - Calls the original system function through a function pointer
/// - Executes arbitrary shell commands which can have system-wide effects
///
/// The caller must ensure that `command` is a valid null-terminated C string.
#[cfg(has_symbol_system)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_system(command: *const c_char) -> c_int {
    // For system, we need to parse the shell command to extract the executable
    if !command.is_null() {
        let command_str = match unsafe { CStr::from_ptr(command) }.to_str() {
            Ok(s) => s,
            Err(_) => {
                log::warn!("Failed to parse system command as UTF-8");
                let func_ptr = REAL_SYSTEM.load(Ordering::SeqCst);
                if !func_ptr.is_null() {
                    let real_func_ptr: SystemFunc = unsafe { std::mem::transmute(func_ptr) };
                    return unsafe { real_func_ptr(command) };
                }
                return -1;
            }
        };

        // Parse the shell command - for simplicity, we'll report it as a shell execution
        report(|| {
            let result = Execution {
                executable: PathBuf::from("/bin/sh"),
                arguments: vec!["/bin/sh".to_string(), "-c".to_string(), command_str.to_string()],
                working_dir: working_dir()?,
                environment: std::env::vars().collect(),
            };
            Ok(result)
        });
    }

    let func_ptr = REAL_SYSTEM.load(Ordering::SeqCst);
    if !func_ptr.is_null() {
        let real_func_ptr: SystemFunc = unsafe { std::mem::transmute(func_ptr) };
        unsafe { real_func_ptr(command) }
    } else {
        log::error!("Real system function not found");
        -1
    }
}

// =============================================================================
// Helper functions
// =============================================================================

/// Report a command execution only if reporter is available
fn report<F>(f: F)
where
    F: FnOnce() -> Result<Execution, c_int>,
{
    let reporter = REPORTER.load(Ordering::SeqCst);
    if reporter.is_null() {
        return;
    }

    let event = match f() {
        Ok(execution) => Event::new(execution),
        Err(_err) => {
            log::debug!("Could not generate execution information");
            return;
        }
    };

    if let Err(err) = unsafe { (*reporter).report(event) } {
        log::debug!("Failed to report execution: {err}",);
    }
}

/// Convert a C string to a Rust String
unsafe fn as_string(s: *const c_char) -> Result<String, c_int> {
    if s.is_null() {
        return Err(libc::EINVAL);
    }
    match unsafe { CStr::from_ptr(s) }.to_str() {
        Ok(s) => Ok(s.to_string()),
        Err(_) => Err(libc::EINVAL),
    }
}

/// Convert a C string to a PathBuf
unsafe fn as_path_buf(s: *const c_char) -> Result<PathBuf, c_int> {
    if s.is_null() {
        return Err(libc::EINVAL);
    }
    match unsafe { CStr::from_ptr(s) }.to_str() {
        Ok(s) => Ok(PathBuf::from(s)),
        Err(_) => Err(libc::EINVAL),
    }
}

/// Convert a null-terminated array of C strings to a Vec<String>
unsafe fn as_string_vec(s: *const *const c_char) -> Result<Vec<String>, c_int> {
    if s.is_null() {
        return Err(libc::EINVAL);
    }
    let mut vec = Vec::new();

    let mut i = 0;
    while !unsafe { (*s.add(i)).is_null() } {
        match unsafe { as_string(*s.add(i)) } {
            Ok(arg) => vec.push(arg),
            Err(e) => return Err(e),
        }
        i += 1;
    }

    Ok(vec)
}

/// Convert a null-terminated array of "KEY=VALUE" C strings to a HashMap
unsafe fn as_environment(s: *const *const c_char) -> Result<HashMap<String, String>, c_int> {
    if s.is_null() {
        return Err(libc::EINVAL);
    }
    let mut map = HashMap::new();

    let mut i = 0;
    while !unsafe { (*s.add(i)).is_null() } {
        match unsafe { as_string(*s.add(i)) } {
            Ok(key_and_value) => {
                if let Some(pos) = key_and_value.find('=') {
                    let key = key_and_value[..pos].to_string();
                    let value = key_and_value[pos + 1..].to_string();

                    map.insert(key, value);
                }
                // FIXME: is the `=` always there? Or can be without?
            }
            Err(e) => return Err(e),
        }
        i += 1;
    }

    Ok(map)
}

/// Get the current working directory
fn working_dir() -> Result<PathBuf, c_int> {
    let cwd = std::env::current_dir().map_err(|_| libc::EINVAL)?;
    Ok(cwd)
}
