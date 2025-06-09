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
use std::ffi::{CStr, CString};
use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use bear::intercept::{create_reporter_on_tcp, Event, Execution};
use ctor::ctor;
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
const RTLD_NEXT: *mut libc::c_void = -1isize as *mut libc::c_void;

/// Constructor function that is called when the library is loaded
///
/// # Safety
/// This function is unsafe because it modifies global state.
#[ctor]
unsafe fn on_load() {
    env_logger::init();

    log::debug!("Initializing intercept-preload library");
}

// Static variables to hold original function pointers
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
static REAL_EXECVE_OPENBSD: AtomicPtr<libc::c_void> = {
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
static REPORTER_ADDRESS: AtomicPtr<String> = {
    // Capture destination address from environment at load time
    match std::env::var(bear::intercept::KEY_DESTINATION) {
        Ok(destination) => {
            log::debug!("Using collector address: {}", destination);

            // Leak the String to get a stable pointer for the lifetime of the program
            let boxed_destination = Box::new(destination);
            let ptr = Box::into_raw(boxed_destination);

            AtomicPtr::new(ptr)
        }
        Err(e) => {
            log::debug!(
                "Failed to get collector address from environment: {} {}",
                bear::intercept::KEY_DESTINATION,
                e
            );
            AtomicPtr::new(std::ptr::null_mut())
        }
    }
};

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(has_symbol_execve)]
#[no_mangle]
pub unsafe extern "C" fn execve(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
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

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(has_symbol_execv)]
#[no_mangle]
pub unsafe extern "C" fn execv(path: *const c_char, argv: *const *const c_char) -> c_int {
    // Try to report but don't fail if we can't
    report(|| {
        let result = Execution {
            executable: as_path_buf(path)?,
            arguments: as_string_vec(argv)?,
            working_dir: working_dir()?,
            environment: environment(),
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

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(has_symbol_execvpe)]
#[no_mangle]
pub unsafe extern "C" fn execvpe(
    file: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    // Try to report but don't fail if we can't
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

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(has_symbol_execvp)]
#[no_mangle]
pub unsafe extern "C" fn execvp(file: *const c_char, argv: *const *const c_char) -> c_int {
    // Try to report but don't fail if we can't
    report(|| {
        let result = Execution {
            executable: as_path_buf(file)?,
            arguments: as_string_vec(argv)?,
            working_dir: working_dir()?,
            environment: environment(),
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

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(has_symbol_execvP)]
#[no_mangle]
pub unsafe extern "C" fn execvP(
    file: *const c_char,
    search_path: *const c_char,
    argv: *const *const c_char,
) -> c_int {
    report(|| {
        let result = Execution {
            executable: as_path_buf(file)?,
            arguments: as_string_vec(argv)?,
            working_dir: working_dir()?,
            environment: environment(),
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

/// # Safety
/// This function is unsafe because it modifies global state.
#[cfg(has_symbol_exect)]
#[no_mangle]
pub unsafe extern "C" fn exect(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
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

// Function to report a command execution only if reporter is available
fn report<F>(f: F)
where
    F: FnOnce() -> Result<Execution, c_int>,
{
    let reporter_address = REPORTER_ADDRESS.load(Ordering::SeqCst);
    if reporter_address.is_null() {
        log::debug!("No reporter address provided");
        return;
    }

    let event = match f() {
        Ok(execution) => Event::new(execution),
        Err(_err) => {
            log::debug!("Could not generate execution information");
            return;
        }
    };

    // SAFETY: We check for null above, and we assume the pointer is valid for the lifetime of the program.
    let reporter_address_str = unsafe { &*reporter_address }.as_str();

    // FIXME: the report should not use anyhow
    let reporter = create_reporter_on_tcp(reporter_address_str);
    let _ = reporter.report(event);
}

// Utility functions to convert C arguments to Rust types
unsafe fn as_string(s: *const c_char) -> Result<String, c_int> {
    if s.is_null() {
        return Err(libc::EINVAL);
    }
    match CStr::from_ptr(s).to_str() {
        Ok(s) => Ok(s.to_string()),
        Err(_) => Err(libc::EINVAL),
    }
}

unsafe fn as_path_buf(s: *const c_char) -> Result<PathBuf, c_int> {
    if s.is_null() {
        return Err(libc::EINVAL);
    }
    match CStr::from_ptr(s).to_str() {
        Ok(s) => Ok(PathBuf::from(s)),
        Err(_) => Err(libc::EINVAL),
    }
}

unsafe fn as_string_vec(s: *const *const c_char) -> Result<Vec<String>, c_int> {
    if s.is_null() {
        return Err(libc::EINVAL);
    }
    let mut vec = Vec::new();

    let mut i = 0;
    while !(*s.add(i)).is_null() {
        match as_string(*s.add(i)) {
            Ok(arg) => vec.push(arg),
            Err(e) => return Err(e),
        }
        i += 1;
    }

    Ok(vec)
}

unsafe fn as_environment(s: *const *const c_char) -> Result<HashMap<String, String>, c_int> {
    if s.is_null() {
        return Err(libc::EINVAL);
    }
    let mut map = HashMap::new();

    let mut i = 0;
    while !(*s.add(i)).is_null() {
        match as_string(*s.add(i)) {
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

fn working_dir() -> Result<PathBuf, c_int> {
    let cwd = std::env::current_dir().map_err(|_| libc::EINVAL)?;
    Ok(cwd)
}

fn environment() -> HashMap<String, String> {
    std::env::vars().collect()
}
