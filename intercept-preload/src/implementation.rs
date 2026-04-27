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
use std::ptr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use bear::intercept::reporter::{Reporter, ReporterFactory};
use bear::intercept::tcp::ReporterOnTcp;
use bear::intercept::{Event, Execution};
use ctor::ctor;
use libc::{RTLD_NEXT, c_char, c_int, pid_t, posix_spawn_file_actions_t, posix_spawnattr_t};

use crate::session::{DoctoredEnvironment, SESSION_CTX, in_session, init_session_from_envp};

#[ctor]
static REAL_EXECVE: AtomicPtr<libc::c_void> = {
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"execve".as_ptr() as *const _) };
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
static REAL_PCLOSE: AtomicPtr<libc::c_void> = {
    let ptr = unsafe { libc::dlsym(RTLD_NEXT, c"pclose".as_ptr() as *const _) };
    AtomicPtr::new(ptr)
};

/// Map from FILE* address to child pid for our popen implementation.
/// When our `rust_popen` spawns a child, it stores the pid here so
/// `rust_pclose` can later `waitpid` it.
static POPEN_CHILDREN: std::sync::LazyLock<std::sync::Mutex<HashMap<usize, pid_t>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

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
    {
        use std::io::Write;

        let pid = std::process::id();
        let _ = env_logger::Builder::from_default_env()
            .format(move |buf, record| {
                let now =
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                let secs = now.as_secs();
                let ms = now.subsec_millis();
                let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
                writeln!(buf, "[{h:02}:{m:02}:{s:02}.{ms:03} preload/{pid}] {}", record.args())
            })
            .try_init();
    }

    // Initialize the session and get the destination for reporter setup
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
#[cfg(has_symbol_popen)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_popen(command: *const c_char, mode: *const c_char) -> *mut libc::FILE {
    if command.is_null() || mode.is_null() {
        set_errno(libc::EINVAL);
        return ptr::null_mut();
    }

    let command_str = match unsafe { CStr::from_ptr(command) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            log::warn!("Failed to parse popen command as UTF-8");
            set_errno(libc::EINVAL);
            return ptr::null_mut();
        }
    };

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
    unsafe { libc::fcntl(parent_fd, libc::F_SETFD, libc::FD_CLOEXEC) };

    // Set up file actions: redirect child's stdin/stdout to the pipe
    let mut file_actions: libc::posix_spawn_file_actions_t = unsafe { std::mem::zeroed() };
    if unsafe { libc::posix_spawn_file_actions_init(&mut file_actions) } != 0 {
        unsafe {
            libc::close(pipe_fds[0]);
            libc::close(pipe_fds[1]);
        }
        return ptr::null_mut();
    }
    // Close the parent's end in the child
    unsafe { libc::posix_spawn_file_actions_addclose(&mut file_actions, parent_fd) };
    // Redirect child's target fd (stdin or stdout) to the child's end of the pipe
    unsafe { libc::posix_spawn_file_actions_adddup2(&mut file_actions, child_fd, child_target_fd) };
    // Close the child's original pipe fd (now duplicated to target)
    unsafe { libc::posix_spawn_file_actions_addclose(&mut file_actions, child_fd) };

    // Build argv for /bin/sh -c <command>
    let sh = c"/bin/sh";
    let dash_c = c"-c";
    let cmd_cstr = match std::ffi::CString::new(command_str) {
        Ok(c) => c,
        Err(_) => {
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
    let resolved_env = unsafe { resolve_environment(envp) };
    let envp_to_use = resolved_env.as_ptr();

    // Spawn the child
    let mut child_pid: pid_t = 0;
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

    unsafe { libc::posix_spawn_file_actions_destroy(&mut file_actions) };

    if spawn_result != 0 {
        unsafe {
            libc::close(pipe_fds[0]);
            libc::close(pipe_fds[1]);
            set_errno(spawn_result);
        }
        return ptr::null_mut();
    }

    // Close the child's end in the parent
    unsafe { libc::close(child_fd) };

    // Wrap the parent's fd in a FILE*
    let file_ptr = unsafe { libc::fdopen(parent_fd, mode) };
    if file_ptr.is_null() {
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
#[cfg(has_symbol_popen)]
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
            let real_func: PcloseFunc = unsafe { std::mem::transmute(func_ptr) };
            return unsafe { real_func(stream) };
        }
        set_errno(libc::EINVAL);
        return -1;
    };

    // Close the stream (this closes the underlying fd)
    unsafe { libc::fclose(stream) };

    // Wait for the child
    let mut status: c_int = 0;
    loop {
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
#[cfg(has_symbol_system)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_system(command: *const c_char) -> c_int {
    // system(NULL) returns non-zero to indicate a command processor is available
    if command.is_null() {
        return 1;
    }

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
    let mut block_mask: libc::sigset_t = unsafe { std::mem::zeroed() };
    let mut old_mask: libc::sigset_t = unsafe { std::mem::zeroed() };
    unsafe {
        libc::sigemptyset(&mut block_mask);
        libc::sigaddset(&mut block_mask, libc::SIGCHLD);
        libc::sigprocmask(libc::SIG_BLOCK, &block_mask, &mut old_mask);
    }

    let old_sigint = unsafe { libc::signal(libc::SIGINT, libc::SIG_IGN) };
    let old_sigquit = unsafe { libc::signal(libc::SIGQUIT, libc::SIG_IGN) };

    // Set up spawn attributes to restore default signal handling in the child
    let mut attr: libc::posix_spawnattr_t = unsafe { std::mem::zeroed() };
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
    let resolved_env = unsafe { resolve_environment(envp) };
    let envp_to_use = resolved_env.as_ptr();

    // Spawn the child
    let mut child_pid: pid_t = 0;
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

    unsafe { libc::posix_spawnattr_destroy(&mut attr) };

    let status = if spawn_result != 0 {
        // Spawn failed — return as if the shell exited with 127
        set_errno(spawn_result);
        -1
    } else {
        // Wait for the child
        let mut wstatus: c_int = 0;
        loop {
            let ret = unsafe { libc::waitpid(child_pid, &mut wstatus, 0) };
            if ret != -1 || get_errno() != libc::EINTR {
                break;
            }
        }
        wstatus
    };

    // Restore signal handling
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
fn get_environ() -> *const *const c_char {
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn _NSGetEnviron() -> *mut *mut *mut c_char;
        }
        unsafe { *_NSGetEnviron() as *const *const c_char }
    }
    #[cfg(not(target_os = "macos"))]
    {
        unsafe extern "C" {
            static environ: *const *const c_char;
        }
        unsafe { environ }
    }
}

/// Create a pipe with `O_CLOEXEC` set on both fds.
///
/// Uses `pipe2(O_CLOEXEC)` on platforms that support it (Linux, FreeBSD),
/// falls back to `pipe()` + `fcntl(FD_CLOEXEC)` on macOS.
fn create_pipe_cloexec(fds: *mut c_int) -> c_int {
    #[cfg(not(target_os = "macos"))]
    {
        unsafe { libc::pipe2(fds, libc::O_CLOEXEC) }
    }
    #[cfg(target_os = "macos")]
    {
        let ret = unsafe { libc::pipe(fds) };
        if ret != 0 {
            return ret;
        }
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

    let event = match f() {
        Ok(execution) => Event::new(execution),
        Err(_err) => {
            log::debug!("Could not generate execution information");
            return;
        }
    };

    if let Err(err) = reporter.report(event) {
        log::debug!("Failed to report execution: {err}");
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
                // Note: entries without '=' are technically valid but unusual
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

/// Set the thread-local errno value.
fn set_errno(value: c_int) {
    errno::set_errno(errno::Errno(value));
}

/// Read the thread-local errno value.
fn get_errno() -> c_int {
    errno::errno().0
}
