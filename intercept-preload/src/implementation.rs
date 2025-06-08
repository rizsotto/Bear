// SPDX-License-Identifier: GPL-3.0-or-later

//! This file implements a shared library. This library can be pre-loaded by
//! the dynamic linker of the Operating System (OS). It implements a few function
//! related to process creation. By pre-load this library the executed process
//! uses these functions instead of those from the standard library.
//!
//! The idea here is to inject a logic before calling the real methods. The logic is
//! to dump the call into a file. To call the real method, this library is doing
//! the job of the dynamic linker.
//!
//! The only input for the log writing is about the destination directory.
//! This is passed as an environment variable.

use std::collections::HashMap;
use std::ffi::{CStr, CString, OsStr};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Mutex;

use anyhow::{Context, Result};
use bear::intercept::{Event, Execution, Reporter};
use libc::{c_char, c_int, pid_t, posix_spawn_file_actions_t, posix_spawnattr_t};

// Function pointer types for the original functions
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

// Dynamic loading related constants and types
#[cfg(has_symbol_RTLD_NEXT)]
const RTLD_NEXT: *mut libc::c_void = -1isize as *mut libc::c_void;

// Static variables to hold original function pointers
#[cfg(has_symbol_execve)]
static REAL_EXECVE: AtomicPtr<libc::c_void> = AtomicPtr::new(std::ptr::null_mut());
#[cfg(has_symbol_execv)]
static REAL_EXECV: AtomicPtr<libc::c_void> = AtomicPtr::new(std::ptr::null_mut());
#[cfg(has_symbol_execvpe)]
static REAL_EXECVPE: AtomicPtr<libc::c_void> = AtomicPtr::new(std::ptr::null_mut());
#[cfg(has_symbol_execvp)]
static REAL_EXECVP: AtomicPtr<libc::c_void> = AtomicPtr::new(std::ptr::null_mut());
#[cfg(has_symbol_execvP)]
static REAL_EXECVP_OPENBSD: AtomicPtr<libc::c_void> = AtomicPtr::new(std::ptr::null_mut());
#[cfg(has_symbol_exect)]
static REAL_EXECT: AtomicPtr<libc::c_void> = AtomicPtr::new(std::ptr::null_mut());
#[cfg(has_symbol_posix_spawn)]
static REAL_POSIX_SPAWN: AtomicPtr<libc::c_void> = AtomicPtr::new(std::ptr::null_mut());
#[cfg(has_symbol_posix_spawnp)]
static REAL_POSIX_SPAWNP: AtomicPtr<libc::c_void> = AtomicPtr::new(std::ptr::null_mut());

// Global reporter variable with mutex for thread safety
static REPORTER: Mutex<Option<Box<dyn Reporter + Send + Sync>>> = Mutex::new(None);

/// Constructor function that is called when the library is loaded
///
/// # Safety
/// This function is unsafe because it modifies global state.
#[no_mangle]
#[cfg_attr(
    any(target_os = "linux", target_os = "freebsd"),
    link_section = ".init_array"
)]
#[cfg(all(has_symbol_dlsym, has_symbol_RTLD_NEXT))]
pub unsafe extern "C" fn on_load() {
    log::debug!("Initializing intercept-preload library");
    initialize_functions();
    if let Err(e) = initialize_reporter() {
        log::debug!("Failed to initialize reporter: {}", e);
    }
}

/// Destructor function that is called when the library is unloaded
///
/// # Safety
/// This function is unsafe because it modifies global state.
#[no_mangle]
#[cfg_attr(
    any(target_os = "linux", target_os = "freebsd"),
    link_section = ".fini_array"
)]
#[cfg(all(has_symbol_dlsym, has_symbol_RTLD_NEXT))]
pub unsafe extern "C" fn on_unload() {
    log::debug!("Cleaning up intercept-preload library");
    if let Err(e) = cleanup_reporter() {
        log::debug!("Failed to clean up reporter: {}", e);
    }
}

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(has_symbol_execve)]
#[no_mangle]
pub unsafe extern "C" fn execve(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    let exe_path = match c_char_ptr_to_path_buf(path) {
        Some(p) => p,
        None => return libc::EINVAL,
    };

    let args = parse_args(argv);
    let env = parse_env(envp);

    // Try to record but don't fail if we can't
    if let Err(e) = record_execution(&exe_path, &args, &env) {
        log::debug!("Failed to record execve command: {}", e);
    }

    let func_ptr = REAL_EXECVE.load(Ordering::SeqCst);
    if !func_ptr.is_null() {
        let real_execve: ExecveFunc = std::mem::transmute(func_ptr);
        real_execve(path, argv, envp)
    } else {
        log::debug!("Real execve function not found");
        libc::ENOSYS
    }
}

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(has_symbol_execv)]
#[no_mangle]
pub unsafe extern "C" fn execv(path: *const c_char, argv: *const *const c_char) -> c_int {
    let exe_path = match c_char_ptr_to_path_buf(path) {
        Some(p) => p,
        None => return libc::EINVAL,
    };

    let args = parse_args(argv);
    let env = std::env::vars().collect();

    // Try to record but don't fail if we can't
    if let Err(e) = record_execution(&exe_path, &args, &env) {
        log::debug!("Failed to record execv command: {}", e);
    }

    let func_ptr = REAL_EXECV.load(Ordering::SeqCst);
    if !func_ptr.is_null() {
        let real_execv: ExecvFunc = std::mem::transmute(func_ptr);
        real_execv(path, argv)
    } else {
        log::debug!("Real execv function not found");
        libc::ENOSYS
    }
}

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(has_symbol_execvpe)]
#[no_mangle]
pub unsafe extern "C" fn execvpe(
    file: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    let exe_path = match c_char_ptr_to_path_buf(file) {
        Some(p) => p,
        None => return libc::EINVAL,
    };

    let args = parse_args(argv);
    let env = parse_env(envp);

    // Try to record but don't fail if we can't
    if let Err(e) = record_execution(&exe_path, &args, &env) {
        log::debug!("Failed to record execvpe command: {}", e);
    }

    let func_ptr = REAL_EXECVPE.load(Ordering::SeqCst);
    if !func_ptr.is_null() {
        let real_execvpe: ExecvpeFunc = std::mem::transmute(func_ptr);
        real_execvpe(file, argv, envp)
    } else {
        log::debug!("Real execvpe function not found");
        libc::ENOSYS
    }
}

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(has_symbol_execvp)]
#[no_mangle]
pub unsafe extern "C" fn execvp(file: *const c_char, argv: *const *const c_char) -> c_int {
    let exe_path = match c_char_ptr_to_path_buf(file) {
        Some(p) => p,
        None => return libc::EINVAL,
    };

    let args = parse_args(argv);
    let env = std::env::vars().collect();

    // Try to record but don't fail if we can't
    if let Err(e) = record_execution(&exe_path, &args, &env) {
        log::debug!("Failed to record execvp command: {}", e);
    }

    let func_ptr = REAL_EXECVP.load(Ordering::SeqCst);
    if !func_ptr.is_null() {
        let real_execvp: ExecvpFunc = std::mem::transmute(func_ptr);
        real_execvp(file, argv)
    } else {
        log::debug!("Real execvp function not found");
        libc::ENOSYS
    }
}

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(has_symbol_execvP)]
#[no_mangle]
pub unsafe extern "C" fn execvP(
    file: *const c_char,
    search_path: *const c_char,
    argv: *const *const c_char,
) -> c_int {
    let exe_path = match c_char_ptr_to_path_buf(file) {
        Some(p) => p,
        None => return libc::EINVAL,
    };

    let args = parse_args(argv);
    let env = std::env::vars().collect();

    // Try to record but don't fail if we can't
    if let Err(e) = record_execution(&exe_path, &args, &env) {
        log::debug!("Failed to record execvP command: {}", e);
    }

    let func_ptr = REAL_EXECVP_OPENBSD.load(Ordering::SeqCst);
    if !func_ptr.is_null() {
        let real_execvP: ExecvPFunc = std::mem::transmute(func_ptr);
        real_execvP(file, search_path, argv)
    } else {
        log::debug!("Real execvP function not found");
        libc::ENOSYS
    }
}

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(has_symbol_exect)]
#[no_mangle]
pub unsafe extern "C" fn exect(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    let exe_path = match c_char_ptr_to_path_buf(path) {
        Some(p) => p,
        None => return libc::EINVAL,
    };

    let args = parse_args(argv);
    let env = parse_env(envp);

    if let Err(e) = record_execution(&exe_path, &args, &env) {
        log::debug!("Failed to record exect command: {}", e);
    }

    let func_ptr = REAL_EXECT.load(Ordering::SeqCst);
    if !func_ptr.is_null() {
        let real_exect: ExectFunc = std::mem::transmute(func_ptr);
        real_exect(path, argv, envp)
    } else {
        log::debug!("Real exect function not found");
        libc::ENOSYS
    }
}

// Implementations for variable argument functions
// We need to handle C variadic arguments in Rust

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(all(has_symbol_execl, has_symbol_execv))]
#[no_mangle]
pub unsafe extern "C" fn execl(
    path: *const c_char,
    arg: *const c_char,
    args: *const c_char, /* variadic */
) -> c_int {
    // In a real implementation, we would need to pass all the variadic arguments
    // But since we can't access all of them directly in Rust, we'll collect what we can
    // and use execv instead

    // Create a vector of argument pointers that we can access
    let mut argv = Vec::new();
    argv.push(path);

    if !arg.is_null() {
        argv.push(arg);

        // In C, execl() is often implemented with execv() and just copying the
        // arguments we have access to
        let va_arg = args;
        if !va_arg.is_null() {
            argv.push(va_arg);
        }
    }

    // Null-terminate the argument list
    argv.push(ptr::null());

    // Use execv to execute the command
    execv(path, argv.as_ptr())
}

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(all(has_symbol_execlp, has_symbol_execvp))]
#[no_mangle]
pub unsafe extern "C" fn execlp(
    file: *const c_char,
    arg: *const c_char,
    args: *const c_char, /* variadic */
) -> c_int {
    // Create a vector of argument pointers that we can access
    let mut argv = Vec::new();
    argv.push(file);

    if !arg.is_null() {
        argv.push(arg);

        // In C, execlp() is often implemented with execvp() and just copying the
        // arguments we have access to
        let va_arg = args;
        if !va_arg.is_null() {
            argv.push(va_arg);
        }
    }

    // Null-terminate the argument list
    argv.push(ptr::null());

    // Use execvp to execute the command
    execvp(file, argv.as_ptr())
}

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(all(has_symbol_execle, has_symbol_execve))]
#[no_mangle]
pub unsafe extern "C" fn execle(
    path: *const c_char,
    arg: *const c_char,
    args: *const c_char, /* variadic */
) -> c_int {
    // execle is a variadic function in C that takes arguments followed by an environment pointer
    // In a real implementation, the last argument (after NULL) is the environment pointer

    // Create a vector of argument pointers that we can access
    let mut argv = Vec::new();
    argv.push(path);

    if !arg.is_null() {
        argv.push(arg);

        // We can only grab the next argument directly
        let va_arg = args;
        if !va_arg.is_null() {
            argv.push(va_arg);
        }
    }

    // Null-terminate the argument list
    argv.push(ptr::null());

    // In execle, the envp pointer follows the NULL terminator
    // Since we can't access variadic args reliably, we'll use the current environment
    let current_env = std::env::vars()
        .map(|(k, v)| format!("{}={}", k, v))
        .map(|s| CString::new(s).unwrap())
        .collect::<Vec<_>>();

    let mut env_ptrs: Vec<*const c_char> = current_env.iter().map(|cs| cs.as_ptr()).collect();

    env_ptrs.push(ptr::null());

    // Use execve to execute the command
    execve(path, argv.as_ptr(), env_ptrs.as_ptr())
}

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(has_symbol_posix_spawn)]
#[no_mangle]
pub unsafe extern "C" fn posix_spawn(
    pid: *mut pid_t,
    path: *const c_char,
    file_actions: *const posix_spawn_file_actions_t,
    attrp: *const posix_spawnattr_t,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    let exe_path = match c_char_ptr_to_path_buf(path) {
        Some(p) => p,
        None => return libc::EINVAL,
    };

    let args = parse_args(argv);
    let env = parse_env(envp);

    if let Err(e) = record_execution(&exe_path, &args, &env) {
        log::debug!("Failed to record posix_spawn command: {}", e);
    }

    let func_ptr = REAL_POSIX_SPAWN.load(Ordering::SeqCst);
    if !func_ptr.is_null() {
        let real_posix_spawn: PosixSpawnFunc = std::mem::transmute(func_ptr);
        real_posix_spawn(pid, path, file_actions, attrp, argv, envp)
    } else {
        log::debug!("Real posix_spawn function not found");
        libc::ENOSYS
    }
}

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(has_symbol_posix_spawnp)]
#[no_mangle]
pub unsafe extern "C" fn posix_spawnp(
    pid: *mut pid_t,
    file: *const c_char,
    file_actions: *const posix_spawn_file_actions_t,
    attrp: *const posix_spawnattr_t,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    let exe_path = match c_char_ptr_to_path_buf(file) {
        Some(p) => p,
        None => return libc::EINVAL,
    };

    let args = parse_args(argv);
    let env = parse_env(envp);

    if let Err(e) = record_execution(&exe_path, &args, &env) {
        log::debug!("Failed to record posix_spawnp command: {}", e);
    }

    let func_ptr = REAL_POSIX_SPAWNP.load(Ordering::SeqCst);
    if !func_ptr.is_null() {
        let real_posix_spawnp: PosixSpawnpFunc = std::mem::transmute(func_ptr);
        real_posix_spawnp(pid, file, file_actions, attrp, argv, envp)
    } else {
        log::debug!("Real posix_spawnp function not found");
        libc::ENOSYS
    }
}

/// Initialize function pointers
///
/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(all(has_symbol_dlsym, has_symbol_RTLD_NEXT))]
unsafe fn initialize_functions() {
    #[cfg(has_symbol_execve)]
    REAL_EXECVE.store(
        libc::dlsym(RTLD_NEXT, c"execve".as_ptr() as *const _),
        Ordering::SeqCst,
    );

    #[cfg(has_symbol_execv)]
    REAL_EXECV.store(
        libc::dlsym(RTLD_NEXT, c"execv".as_ptr() as *const _),
        Ordering::SeqCst,
    );

    #[cfg(has_symbol_execvpe)]
    REAL_EXECVPE.store(
        libc::dlsym(RTLD_NEXT, c"execvpe".as_ptr() as *const _),
        Ordering::SeqCst,
    );

    #[cfg(has_symbol_execvp)]
    REAL_EXECVP.store(
        libc::dlsym(RTLD_NEXT, c"execvp".as_ptr() as *const _),
        Ordering::SeqCst,
    );

    #[cfg(has_symbol_execvP)]
    REAL_EXECVP_OPENBSD.store(
        libc::dlsym(RTLD_NEXT, c"execvP".as_ptr() as *const _),
        Ordering::SeqCst,
    );

    #[cfg(has_symbol_exect)]
    REAL_EXECT.store(
        libc::dlsym(RTLD_NEXT, c"exect".as_ptr() as *const _),
        Ordering::SeqCst,
    );

    #[cfg(has_symbol_posix_spawn)]
    REAL_POSIX_SPAWN.store(
        libc::dlsym(RTLD_NEXT, c"posix_spawn".as_ptr() as *const _),
        Ordering::SeqCst,
    );

    #[cfg(has_symbol_posix_spawnp)]
    REAL_POSIX_SPAWNP.store(
        // libc::dlsym(RTLD_NEXT, b"posix_spawnp\0".as_ptr() as *const _),
        libc::dlsym(RTLD_NEXT, c"posix_spawnp".as_ptr() as *const _),
        Ordering::SeqCst,
    );
}

// Initialize the reporter
fn initialize_reporter() -> Result<(), anyhow::Error> {
    // Capture destination address from environment at load time
    let destination = match std::env::var(bear::intercept::KEY_DESTINATION) {
        Ok(addr) => {
            log::debug!("Using collector address: {}", addr);
            addr
        }
        Err(e) => {
            return Err(anyhow::Error::msg(format!(
                "Failed to get collector address from environment: {}",
                e
            )));
        }
    };

    // Create the reporter directly using the captured address
    let reporter = bear::intercept::create_reporter_on_tcp(&destination)
        .context("Failed to create TCP reporter")?;

    // Store the reporter in our global variable
    let mut reporter_guard = REPORTER
        .lock()
        .map_err(|_| anyhow::Error::msg("Failed to acquire lock for reporter initialization"))?;

    // Cast the reporter to include Send + Sync bounds
    let reporter: Box<dyn Reporter + Send + Sync> = unsafe { std::mem::transmute(reporter) };
    *reporter_guard = Some(reporter);

    log::debug!("Reporter initialized successfully");
    Ok(())
}

// Cleanup the reporter resources
fn cleanup_reporter() -> Result<(), anyhow::Error> {
    let mut reporter_guard = REPORTER
        .lock()
        .map_err(|_| anyhow::Error::msg("Failed to acquire lock for reporter cleanup"))?;

    if reporter_guard.is_some() {
        log::debug!("Cleaning up reporter");
        *reporter_guard = None;
    }

    Ok(())
}

// Utility functions to convert C arguments to Rust types
unsafe fn c_char_ptr_to_string(s: *const c_char) -> Option<String> {
    if s.is_null() {
        return None;
    }
    CStr::from_ptr(s).to_str().ok().map(String::from)
}

unsafe fn c_char_ptr_to_path_buf(s: *const c_char) -> Option<PathBuf> {
    if s.is_null() {
        return None;
    }
    Some(PathBuf::from(OsStr::from_bytes(
        CStr::from_ptr(s).to_bytes(),
    )))
}

unsafe fn parse_args(argv: *const *const c_char) -> Vec<String> {
    let mut args = Vec::new();
    let mut i = 0;

    while !(*argv.add(i)).is_null() {
        if let Some(arg) = c_char_ptr_to_string(*argv.add(i)) {
            args.push(arg);
        }
        i += 1;
    }

    args
}

unsafe fn parse_env(envp: *const *const c_char) -> HashMap<String, String> {
    let mut env = HashMap::new();

    if envp.is_null() {
        return env;
    }

    let mut i = 0;

    while !(*envp.add(i)).is_null() {
        if let Some(var) = c_char_ptr_to_string(*envp.add(i)) {
            if let Some(pos) = var.find('=') {
                let key = var[..pos].to_string();
                let value = var[pos + 1..].to_string();
                env.insert(key, value);
            }
        }
        i += 1;
    }

    env
}

// Function to record a command execution
fn record_execution(
    executable: &Path,
    args: &[String],
    env: &HashMap<String, String>,
) -> Result<()> {
    // If we can't acquire the lock, just return early
    let reporter_guard = match REPORTER.lock() {
        Ok(guard) => guard,
        Err(_) => {
            log::debug!("Failed to acquire lock for reporter");
            return Ok(());
        }
    };

    // If there's no reporter, just return early
    if reporter_guard.is_none() {
        log::debug!("No reporter initialized");
        return Ok(());
    }

    // Get current working directory
    let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

    // Create execution record
    let execution = Execution {
        executable: executable.to_path_buf(),
        arguments: args.to_vec(),
        working_dir,
        environment: env.clone(),
    };

    // Create event
    let event = Event::new(execution);

    // Report the event using the global reporter
    if let Some(reporter) = reporter_guard.as_ref() {
        reporter
            .report(event)
            .context("Failed to report execution event")?;
    }

    Ok(())
}
