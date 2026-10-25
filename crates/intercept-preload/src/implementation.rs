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
//! - Optionally "doctors" the environment to ensure child processes continue interception
//! - Calls the real function via dlsym(RTLD_NEXT, ...)
//!
//! ## Environment Doctoring
//!
//! When SESSION is set (meaning we're in an interception session), we check if the
//! environment passed to exec functions still has the correct preload settings.
//! If not, we "doctor" the environment to restore them, ensuring child processes
//! continue to be intercepted even if the build system cleared the variables.

use std::collections::HashMap;
use std::ffi::CStr;
use std::path::PathBuf;
#[cfg(all(any(has_symbol_popen, has_symbol_system), not(target_os = "android")))]
use std::ptr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use ctor::ctor;
use intercept::Execution;
use intercept::reporter::{Reporter, ReporterFactory};
use intercept::tcp::ReporterOnTcp;
use libc::{RTLD_NEXT, c_char, c_int};
#[cfg(any(
    has_symbol_posix_spawn,
    has_symbol_posix_spawnp,
    all(any(has_symbol_popen, has_symbol_system), not(target_os = "android"))
))]
use libc::{pid_t, posix_spawn_file_actions_t, posix_spawnattr_t};

use crate::session::{DoctoredEnvironment, SESSION_CTX, in_session, init_session_from_envp};

#[ctor(unsafe)]
static REAL_EXECVE: AtomicPtr<libc::c_void> = {
    // SAFETY: the name is a NUL-terminated literal. dlsym may return null
    // (symbol absent); every user of this static checks for null before calling
    // through it.
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"execve".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor(unsafe)]
static REAL_EXECVPE: AtomicPtr<libc::c_void> = {
    // SAFETY: the name is a NUL-terminated literal. dlsym may return null
    // (symbol absent); every user of this static checks for null before calling
    // through it.
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"execvpe".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor(unsafe)]
static REAL_EXECVP: AtomicPtr<libc::c_void> = {
    // SAFETY: the name is a NUL-terminated literal. dlsym may return null
    // (symbol absent); every user of this static checks for null before calling
    // through it.
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"execvp".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor(unsafe)]
static REAL_EXECVP_OPENBSD: AtomicPtr<libc::c_void> = {
    // SAFETY: the name is a NUL-terminated literal. dlsym may return null
    // (symbol absent); every user of this static checks for null before calling
    // through it.
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"execvP".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor(unsafe)]
static REAL_EXECT: AtomicPtr<libc::c_void> = {
    // SAFETY: the name is a NUL-terminated literal. dlsym may return null
    // (symbol absent); every user of this static checks for null before calling
    // through it.
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"exect".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor(unsafe)]
static REAL_POSIX_SPAWN: AtomicPtr<libc::c_void> = {
    // SAFETY: the name is a NUL-terminated literal. dlsym may return null
    // (symbol absent); every user of this static checks for null before calling
    // through it.
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"posix_spawn".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor(unsafe)]
static REAL_POSIX_SPAWNP: AtomicPtr<libc::c_void> = {
    // SAFETY: the name is a NUL-terminated literal. dlsym may return null
    // (symbol absent); every user of this static checks for null before calling
    // through it.
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"posix_spawnp".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor(unsafe)]
static REAL_POPEN: AtomicPtr<libc::c_void> = {
    // SAFETY: the name is a NUL-terminated literal. dlsym may return null
    // (symbol absent); every user of this static checks for null before calling
    // through it.
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"popen".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor(unsafe)]
static REAL_SYSTEM: AtomicPtr<libc::c_void> = {
    // SAFETY: the name is a NUL-terminated literal. dlsym may return null
    // (symbol absent); every user of this static checks for null before calling
    // through it.
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"system".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

#[ctor(unsafe)]
static REAL_PCLOSE: AtomicPtr<libc::c_void> = {
    // SAFETY: the name is a NUL-terminated literal. dlsym may return null
    // (symbol absent); every user of this static checks for null before calling
    // through it.
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"pclose".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

/// Map from FILE* address to child pid for our popen implementation.
/// When our `rust_popen` spawns a child, it stores the pid here so
/// `rust_pclose` can later `waitpid` it.
///
/// A `BTreeMap` rather than a `HashMap`: it is const-constructible, so the
/// static needs no lazy init, and it needs no random seed, which a
/// `HashMap` would fetch from the OS inside the host process.
#[cfg(all(has_symbol_popen, not(target_os = "android")))]
static POPEN_CHILDREN: std::sync::Mutex<std::collections::BTreeMap<usize, pid_t>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

/// Reporter for sending execution events to the collector.
/// Initialized once by `rust_session_init` when called from the C shim constructor.
/// Using `OnceLock` ensures safe single-initialization without raw pointers and
/// provides `&ReporterOnTcp` references that are sound across threads (since
/// `ReporterOnTcp` is `Sync` — it only holds a `SocketAddr` and creates a fresh
/// `TcpStream` per report call).
static REPORTER: OnceLock<ReporterOnTcp> = OnceLock::new();

/// Initialize the session from the environment.
///
/// Called from the C shim's `on_load()` constructor. This captures the
/// environment variables before the build system has a chance to clear them.
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers from C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_session_init(envp: *const *const c_char) {
    // Initialize logging first
    #[cfg(not(test))]
    intercept::logging::init("preload");

    // Initialize the session and get the destination for reporter setup
    // SAFETY: envp is the process environment from the C constructor. This fn's
    // # Safety says it is a null-terminated array of NUL-terminated strings;
    // null is handled.
    let destination = unsafe { init_session_from_envp(envp) };

    // Initialize the reporter from the captured destination
    if let Some(address) = destination {
        let reporter = ReporterFactory::create(address);
        if REPORTER.set(reporter).is_err() {
            log::warn!("Reporter already initialized, ignoring duplicate init");
        }
    } else {
        log::debug!("No destination found, reporter not initialized");
    }
}

/// Rust implementation of execve
///
/// Called from C shim for: execv, execve, execl, execle
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
    type ExecveFunc = unsafe extern "C" fn(
        path: *const c_char,
        argv: *const *const c_char,
        envp: *const *const c_char,
    ) -> c_int;

    // SAFETY: the pointer args are the caller's execve arguments, valid under
    // this fn's # Safety; as_path_buf, as_string_vec and as_environment
    // null-check before reading. resolved_env owns any doctored strings until
    // the end of this block, so envp_to_use stays valid. func_ptr is non-null
    // (checked) and comes from dlsym(RTLD_NEXT, "execve"), so the transmute to
    // the matching fn type is sound.
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

        // Doctor the environment if session is active but the build system stripped preload vars
        let resolved_env = resolve_environment(envp);
        let envp_to_use = resolved_env.as_ptr();

        let func_ptr = REAL_EXECVE.load(Ordering::SeqCst);
        if !func_ptr.is_null() {
            let real_func_ptr: ExecveFunc = std::mem::transmute(func_ptr);
            real_func_ptr(path, argv, envp_to_use)
        } else {
            log::error!("Real execve function not found");
            set_errno(libc::ENOSYS);
            -1
        }
    }
}

/// Rust implementation of execvpe (GNU extension)
///
/// Called from C shim for: execvpe
/// On platforms where execvpe is available, execlp and execvp are also routed here.
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
    type ExecvpeFunc = unsafe extern "C" fn(
        file: *const c_char,
        argv: *const *const c_char,
        envp: *const *const c_char,
    ) -> c_int;

    // SAFETY: the pointer args are the caller's execvpe arguments, valid under
    // this fn's # Safety; as_path_buf, as_string_vec and as_environment
    // null-check before reading. resolved_env owns any doctored strings until
    // the end of this block, so envp_to_use stays valid. func_ptr is non-null
    // (checked) and comes from dlsym(RTLD_NEXT, "execvpe"), so the transmute to
    // the matching fn type is sound.
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

        // Doctor the environment if session is active but the build system stripped preload vars
        let resolved_env = resolve_environment(envp);
        let envp_to_use = resolved_env.as_ptr();

        let func_ptr = REAL_EXECVPE.load(Ordering::SeqCst);
        if !func_ptr.is_null() {
            let real_func_ptr: ExecvpeFunc = std::mem::transmute(func_ptr);
            real_func_ptr(file, argv, envp_to_use)
        } else {
            log::error!("Real execvpe function not found");
            set_errno(libc::ENOSYS);
            -1
        }
    }
}

/// Rust implementation of execvp
///
/// Called from C shim for: execvp, execlp (on platforms where execvpe is not available)
///
/// Unlike `rust_execvpe`, this function does not take an explicit `envp` argument.
/// The real `execvp` uses the process's `environ`, so no environment doctoring is
/// performed here (same approach as `popen` and `system`).
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences raw pointers
/// - Calls the real execvp function
#[cfg(has_symbol_execvp)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_execvp(file: *const c_char, argv: *const *const c_char) -> c_int {
    type ExecvpFunc = unsafe extern "C" fn(file: *const c_char, argv: *const *const c_char) -> c_int;

    // SAFETY: the pointer args are the caller's execvp arguments, valid under
    // this fn's # Safety; as_path_buf, as_string_vec and as_environment
    // null-check before reading. func_ptr is non-null (checked) and comes from
    // dlsym(RTLD_NEXT, "execvp"), so the transmute to the matching fn type is
    // sound.
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
            set_errno(libc::ENOSYS);
            -1
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
    type ExecvPFunc = unsafe extern "C" fn(
        file: *const c_char,
        search_path: *const c_char,
        argv: *const *const c_char,
    ) -> c_int;

    // SAFETY: the pointer args are the caller's execvP arguments, valid under
    // this fn's # Safety; as_path_buf, as_string_vec and as_environment
    // null-check before reading. func_ptr is non-null (checked) and comes from
    // dlsym(RTLD_NEXT, "execvP"), so the transmute to the matching fn type is
    // sound.
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
            set_errno(libc::ENOSYS);
            -1
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
    type ExectFunc = unsafe extern "C" fn(
        path: *const c_char,
        argv: *const *const c_char,
        envp: *const *const c_char,
    ) -> c_int;

    // SAFETY: the pointer args are the caller's exect arguments, valid under
    // this fn's # Safety; as_path_buf, as_string_vec and as_environment
    // null-check before reading. resolved_env owns any doctored strings until
    // the end of this block, so envp_to_use stays valid. func_ptr is non-null
    // (checked) and comes from dlsym(RTLD_NEXT, "exect"), so the transmute to
    // the matching fn type is sound.
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

        // Doctor the environment if session is active but the build system stripped preload vars
        let resolved_env = resolve_environment(envp);
        let envp_to_use = resolved_env.as_ptr();

        let func_ptr = REAL_EXECT.load(Ordering::SeqCst);
        if !func_ptr.is_null() {
            let real_func_ptr: ExectFunc = std::mem::transmute(func_ptr);
            real_func_ptr(path, argv, envp_to_use)
        } else {
            log::error!("Real exect function not found");
            set_errno(libc::ENOSYS);
            -1
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
    type PosixSpawnFunc = unsafe extern "C" fn(
        pid: *mut pid_t,
        path: *const c_char,
        file_actions: *const posix_spawn_file_actions_t,
        attrp: *const posix_spawnattr_t,
        argv: *const *const c_char,
        envp: *const *const c_char,
    ) -> c_int;

    // SAFETY: the pointer args are the caller's posix_spawn arguments, valid
    // under this fn's # Safety; as_path_buf, as_string_vec and as_environment
    // null-check before reading. resolved_env owns any doctored strings until
    // the end of this block, so envp_to_use stays valid. func_ptr is non-null
    // (checked) and comes from dlsym(RTLD_NEXT, "posix_spawn"), so the
    // transmute to the matching fn type is sound.
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

        // Doctor the environment if session is active but the build system stripped preload vars
        let resolved_env = resolve_environment(envp);
        let envp_to_use = resolved_env.as_ptr();

        let func_ptr = REAL_POSIX_SPAWN.load(Ordering::SeqCst);
        if !func_ptr.is_null() {
            let real_func_ptr: PosixSpawnFunc = std::mem::transmute(func_ptr);
            real_func_ptr(pid, path, file_actions, attrp, argv, envp_to_use)
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
    type PosixSpawnpFunc = unsafe extern "C" fn(
        pid: *mut pid_t,
        file: *const c_char,
        file_actions: *const posix_spawn_file_actions_t,
        attrp: *const posix_spawnattr_t,
        argv: *const *const c_char,
        envp: *const *const c_char,
    ) -> c_int;

    // SAFETY: the pointer args are the caller's posix_spawnp arguments, valid
    // under this fn's # Safety; as_path_buf, as_string_vec and as_environment
    // null-check before reading. resolved_env owns any doctored strings until
    // the end of this block, so envp_to_use stays valid. func_ptr is non-null
    // (checked) and comes from dlsym(RTLD_NEXT, "posix_spawnp"), so the
    // transmute to the matching fn type is sound.
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

        // Doctor the environment if session is active but the build system stripped preload vars
        let resolved_env = resolve_environment(envp);
        let envp_to_use = resolved_env.as_ptr();

        let func_ptr = REAL_POSIX_SPAWNP.load(Ordering::SeqCst);
        if !func_ptr.is_null() {
            let real_func_ptr: PosixSpawnpFunc = std::mem::transmute(func_ptr);
            real_func_ptr(pid, file, file_actions, attrp, argv, envp_to_use)
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
/// Instead of calling the real `popen` (which uses `environ` internally and
/// may bypass our LD_PRELOAD on older glibc), we reimplement it using
/// `posix_spawnp` with a doctored envp — the same path as exec-family calls.
/// This eliminates the need for `ensure_environ_has_session_vars` / `setenv`.
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences raw pointers (`command` and `mode`) which could be null or invalid
/// - Creates pipes and spawns child processes
/// - Returns a raw pointer to a FILE structure
///
/// The caller must ensure that `command` and `mode` are valid null-terminated C strings.
#[cfg(all(has_symbol_popen, not(target_os = "android")))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_popen(command: *const c_char, mode: *const c_char) -> *mut libc::FILE {
    if command.is_null() || mode.is_null() {
        set_errno(libc::EINVAL);
        return ptr::null_mut();
    }

    // SAFETY: command is non-null (checked above) and NUL-terminated per this
    // fn's # Safety. The borrowed str is only used inside this call; the caller
    // keeps the C string alive.
    let command_str = match unsafe { CStr::from_ptr(command) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            log::warn!("Failed to parse popen command as UTF-8");
            set_errno(libc::EINVAL);
            return ptr::null_mut();
        }
    };

    // SAFETY: mode is non-null (checked above) and NUL-terminated per this fn's
    // # Safety. The borrowed str is only used inside this call.
    let mode_str = match unsafe { CStr::from_ptr(mode) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_errno(libc::EINVAL);
            return ptr::null_mut();
        }
    };

    // Parse mode: 'r' or 'w' for direction, optional 'e' for O_CLOEXEC (glibc extension)
    let reading = mode_str.starts_with('r');
    let cloexec = mode_str.contains('e');

    report(|| {
        Ok(Execution {
            executable: PathBuf::from("/bin/sh"),
            arguments: vec!["/bin/sh".to_string(), "-c".to_string(), command_str.to_string()],
            working_dir: working_dir()?,
            environment: std::env::vars().collect(),
        })
    });

    // Create the pipe, with O_CLOEXEC if requested via 'e' in mode.
    let mut pipe_fds: [c_int; 2] = [0; 2];
    let pipe_result = if cloexec {
        create_pipe_cloexec(pipe_fds.as_mut_ptr())
    } else {
        // SAFETY: pipe_fds is a live local [c_int; 2], which is what pipe(2)
        // writes to.
        unsafe { libc::pipe(pipe_fds.as_mut_ptr()) }
    };
    if pipe_result != 0 {
        return ptr::null_mut();
    }

    // Parent reads from pipe_fds[0], child writes to pipe_fds[1] (reading mode)
    // Parent writes to pipe_fds[1], child reads from pipe_fds[0] (writing mode)
    let (parent_fd, child_fd, child_target_fd) = if reading {
        (pipe_fds[0], pipe_fds[1], libc::STDOUT_FILENO)
    } else {
        (pipe_fds[1], pipe_fds[0], libc::STDIN_FILENO)
    };

    // Set FD_CLOEXEC on the parent fd to prevent leaking it into unrelated
    // children if the caller later fork+execs without going through our override.
    // This matches libc popen behavior. (When 'e' was requested and pipe2 was used,
    // both fds already have CLOEXEC, but setting it again is harmless.)
    // SAFETY: parent_fd is an open end of the pipe made above. F_SETFD takes an
    // int, no pointer.
    unsafe { libc::fcntl(parent_fd, libc::F_SETFD, libc::FD_CLOEXEC) };

    // Set up file actions: redirect child's stdin/stdout to the pipe
    // SAFETY: posix_spawn_file_actions_t is a plain C struct; all-zero bytes is
    // a valid value to pass to _init, which sets it up next.
    let mut file_actions: libc::posix_spawn_file_actions_t = unsafe { std::mem::zeroed() };
    // SAFETY: file_actions is a live, exclusively borrowed local. _init is the
    // required first call on it.
    if unsafe { libc::posix_spawn_file_actions_init(&mut file_actions) } != 0 {
        // SAFETY: both fds are open ends of the pipe made above. This path
        // returns right after, so neither is used again.
        unsafe {
            libc::close(pipe_fds[0]);
            libc::close(pipe_fds[1]);
        }
        return ptr::null_mut();
    }
    // Close the parent's end in the child
    // SAFETY: file_actions was initialized above and is live. The fds are open
    // pipe ends; these calls only record actions.
    unsafe { libc::posix_spawn_file_actions_addclose(&mut file_actions, parent_fd) };
    // Redirect child's target fd (stdin or stdout) to the child's end of the pipe
    // SAFETY: file_actions was initialized above and is live. The fds are open
    // pipe ends; these calls only record actions.
    unsafe { libc::posix_spawn_file_actions_adddup2(&mut file_actions, child_fd, child_target_fd) };
    // Close the child's original pipe fd (now duplicated to target)
    // SAFETY: file_actions was initialized above and is live. The fds are open
    // pipe ends; these calls only record actions.
    unsafe { libc::posix_spawn_file_actions_addclose(&mut file_actions, child_fd) };

    // Build argv for /bin/sh -c <command>
    let sh = c"/bin/sh";
    let dash_c = c"-c";
    let cmd_cstr = match std::ffi::CString::new(command_str) {
        Ok(c) => c,
        Err(_) => {
            // SAFETY: file_actions was initialized above and is destroyed once
            // on this path. The pipe fds are open and unused after the return.
            unsafe {
                libc::posix_spawn_file_actions_destroy(&mut file_actions);
                libc::close(pipe_fds[0]);
                libc::close(pipe_fds[1]);
            }
            return ptr::null_mut();
        }
    };
    let argv: [*const c_char; 4] = [sh.as_ptr(), dash_c.as_ptr(), cmd_cstr.as_ptr(), ptr::null()];

    // Doctor the environment via the same path as exec-family calls
    let envp = get_environ();
    // SAFETY: envp is the process environ (get_environ), a null-terminated
    // array of NUL-terminated strings, which is resolve_environment's contract.
    // A concurrent setenv in another thread would race, as it does for libc.
    let resolved_env = unsafe { resolve_environment(envp) };
    let envp_to_use = resolved_env.as_ptr();

    // Spawn the child
    let mut child_pid: pid_t = 0;
    // SAFETY: func_ptr is non-null (checked first) and comes from
    // dlsym(RTLD_NEXT, "posix_spawnp"), so the transmute is sound. child_pid,
    // file_actions and argv are live locals. argv is null-terminated and its
    // strings (sh, dash_c, cmd_cstr) outlive the call. attrp is null, which is
    // allowed. envp_to_use is kept alive by resolved_env.
    let spawn_result = unsafe {
        let func_ptr = REAL_POSIX_SPAWNP.load(Ordering::SeqCst);
        if func_ptr.is_null() {
            log::error!("Real posix_spawnp function not found");
            libc::posix_spawn_file_actions_destroy(&mut file_actions);
            libc::close(pipe_fds[0]);
            libc::close(pipe_fds[1]);
            return ptr::null_mut();
        }
        type PosixSpawnpFunc = unsafe extern "C" fn(
            pid: *mut pid_t,
            file: *const c_char,
            file_actions: *const posix_spawn_file_actions_t,
            attrp: *const posix_spawnattr_t,
            argv: *const *const c_char,
            envp: *const *const c_char,
        ) -> c_int;
        let real_func: PosixSpawnpFunc = std::mem::transmute(func_ptr);
        real_func(&mut child_pid, sh.as_ptr(), &file_actions, ptr::null(), argv.as_ptr(), envp_to_use)
    };

    // SAFETY: file_actions was initialized above and is destroyed exactly once,
    // after the spawn call is done with it.
    unsafe { libc::posix_spawn_file_actions_destroy(&mut file_actions) };

    if spawn_result != 0 {
        // SAFETY: both fds are still open (we own them) and are not used after
        // this return.
        unsafe {
            libc::close(pipe_fds[0]);
            libc::close(pipe_fds[1]);
            set_errno(spawn_result);
        }
        return ptr::null_mut();
    }

    // Close the child's end in the parent
    // SAFETY: child_fd is an open pipe end owned by us. The child has its own
    // copy and the parent never uses it again.
    unsafe { libc::close(child_fd) };

    // Wrap the parent's fd in a FILE*
    // SAFETY: parent_fd is open and owned by us. mode is non-null and
    // NUL-terminated per this fn's # Safety, and fdopen does not keep the
    // pointer.
    let file_ptr = unsafe { libc::fdopen(parent_fd, mode) };
    if file_ptr.is_null() {
        // SAFETY: fdopen failed, so parent_fd is still ours to close. child_pid
        // is the child just spawned; killing and reaping it leaves no zombie. A
        // null status pointer is allowed for waitpid.
        unsafe {
            libc::close(parent_fd);
            libc::kill(child_pid, libc::SIGKILL);
            libc::waitpid(child_pid, ptr::null_mut(), 0);
        }
        return ptr::null_mut();
    }

    // Track the child pid so pclose can waitpid it
    POPEN_CHILDREN.lock().unwrap_or_else(|e| e.into_inner()).insert(file_ptr as usize, child_pid);

    file_ptr
}

/// Rust implementation of pclose
///
/// Called from C shim for: pclose
///
/// Retrieves the child pid associated with the FILE* (stored by our popen),
/// closes the stream, and waits for the child to exit.
///
/// # Safety
/// This function is unsafe because it dereferences a raw FILE pointer.
#[cfg(all(has_symbol_popen, not(target_os = "android")))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_pclose(stream: *mut libc::FILE) -> c_int {
    if stream.is_null() {
        set_errno(libc::EINVAL);
        return -1;
    }

    // Look up the child pid
    let child_pid = POPEN_CHILDREN.lock().unwrap_or_else(|e| e.into_inner()).remove(&(stream as usize));

    // If we don't have a tracked pid, this FILE* wasn't opened by our popen.
    // Fall back to the real pclose.
    let Some(pid) = child_pid else {
        let func_ptr = REAL_PCLOSE.load(Ordering::SeqCst);
        if !func_ptr.is_null() {
            type PcloseFunc = unsafe extern "C" fn(stream: *mut libc::FILE) -> c_int;
            // SAFETY: func_ptr is non-null (checked above) and comes from
            // dlsym(RTLD_NEXT, "pclose"), so PcloseFunc is its real signature.
            let real_func: PcloseFunc = unsafe { std::mem::transmute(func_ptr) };
            // SAFETY: stream is non-null (checked at the top). It was not
            // opened by our popen, so the real pclose is the right owner for
            // it.
            return unsafe { real_func(stream) };
        }
        set_errno(libc::EINVAL);
        return -1;
    };

    // Close the stream (this closes the underlying fd)
    // SAFETY: stream is non-null and came from rust_popen (its pid was in
    // POPEN_CHILDREN and is now removed, so it is closed once). The caller must
    // not use it after pclose.
    unsafe { libc::fclose(stream) };

    // Wait for the child
    let mut status: c_int = 0;
    loop {
        // SAFETY: status is a live local c_int, the only thing waitpid writes.
        let ret = unsafe { libc::waitpid(pid, &mut status, 0) };
        if ret != -1 || get_errno() != libc::EINTR {
            break;
        }
    }

    status
}

/// Rust implementation of system
///
/// Called from C shim for: system
///
/// Instead of calling the real `system` (which uses `environ` internally and
/// may bypass our LD_PRELOAD on older glibc), we reimplement it using
/// `posix_spawnp` with a doctored envp — the same path as exec-family calls.
/// This eliminates the need for `ensure_environ_has_session_vars` / `setenv`.
///
/// Follows POSIX system() semantics: blocks SIGCHLD, ignores SIGINT/SIGQUIT
/// during the child's execution, then waitpid's and returns the status.
/// `system(NULL)` returns non-zero to indicate a shell is available.
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences a raw pointer (`command`) which could be null or invalid
/// - Spawns child processes and manipulates signal masks
///
/// The caller must ensure that `command` is a valid null-terminated C string.
#[cfg(all(has_symbol_system, not(target_os = "android")))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_system(command: *const c_char) -> c_int {
    // system(NULL) returns non-zero to indicate a command processor is available
    if command.is_null() {
        return 1;
    }

    // SAFETY: command is non-null (checked above) and NUL-terminated per this
    // fn's # Safety. The borrowed str is only used inside this call; the caller
    // keeps the C string alive.
    let command_str = match unsafe { CStr::from_ptr(command) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            log::warn!("Failed to parse system command as UTF-8");
            set_errno(libc::EINVAL);
            return -1;
        }
    };

    report(|| {
        Ok(Execution {
            executable: PathBuf::from("/bin/sh"),
            arguments: vec!["/bin/sh".to_string(), "-c".to_string(), command_str.to_string()],
            working_dir: working_dir()?,
            environment: std::env::vars().collect(),
        })
    });

    // Build argv for /bin/sh -c <command>
    let sh = c"/bin/sh";
    let dash_c = c"-c";
    let cmd_cstr = match std::ffi::CString::new(command_str) {
        Ok(c) => c,
        Err(_) => {
            set_errno(libc::EINVAL);
            return -1;
        }
    };
    let argv: [*const c_char; 4] = [sh.as_ptr(), dash_c.as_ptr(), cmd_cstr.as_ptr(), ptr::null()];

    // Block SIGCHLD, ignore SIGINT and SIGQUIT (POSIX system() semantics)
    // SAFETY: sigset_t is a plain C type; all-zero bytes is a valid value, and
    // sigemptyset initializes it right below.
    let mut block_mask: libc::sigset_t = unsafe { std::mem::zeroed() };
    // SAFETY: sigset_t is a plain C type; all-zero bytes is a valid value, and
    // sigemptyset initializes it right below.
    let mut old_mask: libc::sigset_t = unsafe { std::mem::zeroed() };
    // SAFETY: block_mask and old_mask are live locals. sigemptyset and
    // sigaddset write block_mask, sigprocmask reads it and writes old_mask.
    // SIGCHLD is a valid signal.
    unsafe {
        libc::sigemptyset(&mut block_mask);
        libc::sigaddset(&mut block_mask, libc::SIGCHLD);
        libc::sigprocmask(libc::SIG_BLOCK, &block_mask, &mut old_mask);
    }

    // SAFETY: signal() with SIG_IGN installs no handler code. The old
    // disposition is saved and restored below, as libc system() does.
    let old_sigint = unsafe { libc::signal(libc::SIGINT, libc::SIG_IGN) };
    // SAFETY: signal() with SIG_IGN installs no handler code. The old
    // disposition is saved and restored below, as libc system() does.
    let old_sigquit = unsafe { libc::signal(libc::SIGQUIT, libc::SIG_IGN) };

    // Set up spawn attributes to restore default signal handling in the child
    // SAFETY: posix_spawnattr_t is a plain C type; all-zero bytes is a valid
    // value to pass to _init, called first below.
    let mut attr: libc::posix_spawnattr_t = unsafe { std::mem::zeroed() };
    // SAFETY: attr is a live local and posix_spawnattr_init is the first call
    // on it. default_sigs is zeroed and then set up by sigemptyset. old_mask
    // was filled by sigprocmask above.
    unsafe {
        libc::posix_spawnattr_init(&mut attr);
        libc::posix_spawnattr_setflags(
            &mut attr,
            libc::POSIX_SPAWN_SETSIGDEF as libc::c_short | libc::POSIX_SPAWN_SETSIGMASK as libc::c_short,
        );
        // Restore default handlers for SIGINT, SIGQUIT in the child
        let mut default_sigs: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut default_sigs);
        libc::sigaddset(&mut default_sigs, libc::SIGINT);
        libc::sigaddset(&mut default_sigs, libc::SIGQUIT);
        libc::posix_spawnattr_setsigdefault(&mut attr, &default_sigs);
        // Restore the original signal mask in the child
        libc::posix_spawnattr_setsigmask(&mut attr, &old_mask);
    }

    // Doctor the environment via the same path as exec-family calls
    let envp = get_environ();
    // SAFETY: envp is the process environ (get_environ), a null-terminated
    // array of NUL-terminated strings, which is resolve_environment's contract.
    // A concurrent setenv in another thread would race, as it does for libc.
    let resolved_env = unsafe { resolve_environment(envp) };
    let envp_to_use = resolved_env.as_ptr();

    // Spawn the child
    let mut child_pid: pid_t = 0;
    // SAFETY: func_ptr is non-null (checked first) and comes from
    // dlsym(RTLD_NEXT, "posix_spawnp"), so the transmute is sound. child_pid
    // and attr are live locals; attr was initialized above. file_actions is
    // null, which is allowed. argv is null-terminated and its strings (sh,
    // dash_c, cmd_cstr) outlive the call. envp_to_use is kept alive by
    // resolved_env. The early-return path destroys attr once and restores the
    // signal state saved above.
    let spawn_result = unsafe {
        let func_ptr = REAL_POSIX_SPAWNP.load(Ordering::SeqCst);
        if func_ptr.is_null() {
            log::error!("Real posix_spawnp function not found");
            libc::posix_spawnattr_destroy(&mut attr);
            libc::signal(libc::SIGINT, old_sigint);
            libc::signal(libc::SIGQUIT, old_sigquit);
            libc::sigprocmask(libc::SIG_SETMASK, &old_mask, ptr::null_mut());
            return -1;
        }
        type PosixSpawnpFunc = unsafe extern "C" fn(
            pid: *mut pid_t,
            file: *const c_char,
            file_actions: *const posix_spawn_file_actions_t,
            attrp: *const posix_spawnattr_t,
            argv: *const *const c_char,
            envp: *const *const c_char,
        ) -> c_int;
        let real_func: PosixSpawnpFunc = std::mem::transmute(func_ptr);
        real_func(
            &mut child_pid,
            sh.as_ptr(),
            ptr::null(), // no file actions
            &attr,
            argv.as_ptr(),
            envp_to_use,
        )
    };

    // SAFETY: attr was initialized above and is destroyed once (the early
    // return in the block above skips this line), after the spawn is done with
    // it.
    unsafe { libc::posix_spawnattr_destroy(&mut attr) };

    let status = if spawn_result != 0 {
        // Spawn failed — return as if the shell exited with 127
        set_errno(spawn_result);
        -1
    } else {
        // Wait for the child
        let mut wstatus: c_int = 0;
        loop {
            // SAFETY: wstatus is a live local c_int, the only thing waitpid
            // writes. child_pid is the child spawned above.
            let ret = unsafe { libc::waitpid(child_pid, &mut wstatus, 0) };
            if ret != -1 || get_errno() != libc::EINTR {
                break;
            }
        }
        wstatus
    };

    // Restore signal handling
    // SAFETY: old_sigint and old_sigquit are the dispositions signal() returned
    // above, and old_mask was filled by sigprocmask. Restoring them is valid.
    unsafe {
        libc::signal(libc::SIGINT, old_sigint);
        libc::signal(libc::SIGQUIT, old_sigquit);
        libc::sigprocmask(libc::SIG_SETMASK, &old_mask, ptr::null_mut());
    }

    status
}

// =============================================================================
// Helper functions
// =============================================================================

/// Represents either the original environment pointer or a doctored environment.
///
/// This enum allows us to either pass through the original `envp` unchanged,
/// or use a doctored environment that ensures preload settings are preserved.
enum ResolvedEnvironment {
    /// Use the original environment pointer as-is
    Original(*const *const c_char),
    /// Use a doctored environment (owns the data)
    Doctored(DoctoredEnvironment),
}

impl ResolvedEnvironment {
    /// Get a pointer to the environment array suitable for passing to exec functions.
    fn as_ptr(&self) -> *const *const c_char {
        match self {
            Self::Original(ptr) => *ptr,
            Self::Doctored(doc) => doc.as_ptr(),
        }
    }
}

/// Determine whether to use the original environment or create a doctored one.
///
/// Logic:
/// - If SESSION is not set: use original envp (we're not in an interception session)
/// - If SESSION is set and `in_session` returns true: use original envp (environment is correct)
/// - If SESSION is set and `in_session` returns false: create doctored environment
///
/// # Safety
/// The `envp` pointer must be a valid null-terminated array of null-terminated
/// C strings in "KEY=VALUE" format, or null.
unsafe fn resolve_environment(envp: *const *const c_char) -> ResolvedEnvironment {
    match SESSION_CTX.get() {
        None => {
            // No session, pass through original environment
            ResolvedEnvironment::Original(envp)
        }
        Some(ctx) => {
            // SAFETY: envp is resolve_environment's argument, so its # Safety
            // contract (valid null-terminated array of NUL-terminated strings,
            // or null) is what in_session needs.
            if unsafe { in_session(ctx, envp) } {
                // Environment is still aligned with session, pass through
                ResolvedEnvironment::Original(envp)
            } else {
                // Environment was modified, doctor it
                match DoctoredEnvironment::from_envp(ctx, envp) {
                    Ok(doctored) => ResolvedEnvironment::Doctored(doctored),
                    Err(_) => ResolvedEnvironment::Original(envp),
                }
            }
        }
    }
}

/// Get the process's current `environ` pointer.
///
/// On Linux/BSD, this is the `extern char **environ` variable.
/// On macOS, we use `_NSGetEnviron()` for reliable access.
#[cfg(all(any(has_symbol_popen, has_symbol_system), not(target_os = "android")))]
fn get_environ() -> *const *const c_char {
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn _NSGetEnviron() -> *mut *mut *mut c_char;
        }
        // SAFETY: _NSGetEnviron returns a valid pointer to the process's
        // environ for the whole process lifetime; we only read it.
        unsafe { *_NSGetEnviron() as *const *const c_char }
    }
    #[cfg(not(target_os = "macos"))]
    {
        unsafe extern "C" {
            static environ: *const *const c_char;
        }
        // SAFETY: environ is the libc global. Reading the pointer is valid. A
        // concurrent setenv in another thread would race, as it does for libc.
        unsafe { environ }
    }
}

/// Create a pipe with `O_CLOEXEC` set on both fds.
///
/// Uses `pipe2(O_CLOEXEC)` on platforms that support it (Linux, FreeBSD),
/// falls back to `pipe()` + `fcntl(FD_CLOEXEC)` on macOS.
#[cfg(all(has_symbol_popen, not(target_os = "android")))]
fn create_pipe_cloexec(fds: *mut c_int) -> c_int {
    #[cfg(not(target_os = "macos"))]
    {
        // SAFETY: fds must point to a [c_int; 2] for pipe2. The only caller
        // passes pipe_fds from rust_popen, which is a live local array of two.
        unsafe { libc::pipe2(fds, libc::O_CLOEXEC) }
    }
    #[cfg(target_os = "macos")]
    {
        // SAFETY: fds must point to a [c_int; 2] for pipe. The only caller
        // passes pipe_fds from rust_popen, which is a live local array of two.
        let ret = unsafe { libc::pipe(fds) };
        if ret != 0 {
            return ret;
        }
        // SAFETY: fds points to two c_ints (see above), so *fds and *fds.add(1)
        // are in bounds. The fds are open since pipe succeeded.
        unsafe {
            libc::fcntl(*fds, libc::F_SETFD, libc::FD_CLOEXEC);
            libc::fcntl(*fds.add(1), libc::F_SETFD, libc::FD_CLOEXEC);
        }
        0
    }
}

/// Report a command execution only if reporter is available
fn report<F>(f: F)
where
    F: FnOnce() -> Result<Execution, c_int>,
{
    let Some(reporter) = REPORTER.get() else {
        return;
    };

    let execution = match f() {
        Ok(execution) => execution,
        Err(_err) => {
            log::debug!("Could not generate execution information");
            return;
        }
    };

    if let Err(err) = reporter.report(execution) {
        log::debug!("Failed to report execution: {err}");
    }
}

/// Convert a C string to a Rust String
unsafe fn as_string(s: *const c_char) -> Result<String, c_int> {
    if s.is_null() {
        return Err(libc::EINVAL);
    }
    // SAFETY: s is non-null (checked above) and NUL-terminated, as the callers'
    // C APIs require. The borrowed &CStr is only used inside this expression;
    // the result is copied.
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
    // SAFETY: s is non-null (checked above) and NUL-terminated, as the callers'
    // C APIs require. The borrowed &CStr is only used inside this expression;
    // the result is copied.
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
    // SAFETY: s is non-null (checked above). The array is null-terminated, and
    // the loop stops at the first null, so s.add(i) never goes past the
    // terminator.
    while !unsafe { (*s.add(i)).is_null() } {
        // SAFETY: s.add(i) is in bounds because the entry at i was just seen to
        // be non-null; as_string null-checks and reads a NUL-terminated string.
        let arg = unsafe { as_string(*s.add(i)) }?;
        vec.push(arg);
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
    // SAFETY: s is non-null (checked above). The array is null-terminated, and
    // the loop stops at the first null, so s.add(i) never goes past the
    // terminator.
    while !unsafe { (*s.add(i)).is_null() } {
        // SAFETY: s.add(i) is in bounds because the entry at i was just seen to
        // be non-null; as_string null-checks and reads a NUL-terminated string.
        let key_and_value = unsafe { as_string(*s.add(i)) }?;
        if let Some(pos) = key_and_value.find('=') {
            let key = key_and_value[..pos].to_string();
            let value = key_and_value[pos + 1..].to_string();

            map.insert(key, value);
        }
        // Note: entries without '=' are technically valid but unusual
        i += 1;
    }

    Ok(map)
}

/// Get the current working directory
fn working_dir() -> Result<PathBuf, c_int> {
    let cwd = std::env::current_dir().map_err(|_| libc::EINVAL)?;
    Ok(cwd)
}

/// Set the thread-local errno value.
fn set_errno(value: c_int) {
    errno::set_errno(errno::Errno(value));
}

/// Read the thread-local errno value.
#[cfg(all(any(has_symbol_popen, has_symbol_system), not(target_os = "android")))]
fn get_errno() -> c_int {
    errno::errno().0
}
