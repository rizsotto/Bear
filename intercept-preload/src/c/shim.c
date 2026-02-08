// SPDX-License-Identifier: GPL-3.0-or-later

//
// C shim for intercepted libc functions
//
// This file provides thin C wrappers for all intercepted functions. The actual
// interception logic (reporting and calling real functions via dlsym) is
// implemented in Rust. This separation exists because:
//
// 1. Stable Rust cannot handle C variadic arguments (execl family)
// 2. On FreeBSD, libc functions may call each other internally. By having all
//    exported symbols in C call into Rust (which uses dlsym(RTLD_NEXT, ...)),
//    we avoid recursive interception issues.
//
// For variadic functions (execl, execlp, execle), we use a two-pass approach
// with VLAs (C99) to collect arguments without heap allocation:
// 1. First pass: count the number of variadic arguments
// 2. Second pass: copy arguments into a stack-allocated VLA
//

#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif

#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <spawn.h>
#include <unistd.h>

// Ensure symbols are exported from the shared library
#define EXPORT __attribute__((visibility("default")))

// Platform-specific environment access
//
// When the dynamic linker loads the library, the `environ` variable might not
// be available yet. This is the case on macOS where we need to use
// `_NSGetEnviron()` to reliably access the environment during library
// initialization.

#if defined(__APPLE__)
#include <crt_externs.h>
#define get_environ() (*_NSGetEnviron())
#else
extern char **environ;
#define get_environ() environ
#endif

// Rust implementation functions
//
// These are defined in implementation.rs with #[no_mangle] and handle:
// - Reporting the execution to the collector
// - Calling the real function via dlsym(RTLD_NEXT, ...)

// Session initialization - called from constructor to capture environment
extern void rust_session_init(char *const *envp);

// Exec family functions
extern int rust_execve(const char *path, char *const argv[], char *const envp[]);
extern int rust_execvpe(const char *file, char *const argv[], char *const envp[]);
extern int rust_execvp(const char *file, char *const argv[]);
extern int rust_execvP(const char *file, const char *search_path, char *const argv[]);
extern int rust_exect(const char *path, char *const argv[], char *const envp[]);
extern int rust_posix_spawn(pid_t *pid, const char *path,
                            const posix_spawn_file_actions_t *file_actions,
                            const posix_spawnattr_t *attrp,
                            char *const argv[], char *const envp[]);
extern int rust_posix_spawnp(pid_t *pid, const char *file,
                             const posix_spawn_file_actions_t *file_actions,
                             const posix_spawnattr_t *attrp,
                             char *const argv[], char *const envp[]);
extern FILE *rust_popen(const char *command, const char *mode);
extern int rust_system(const char *command);

// Library constructor
//
// This function is called when the library is loaded into memory. It captures
// the current environment and passes it to Rust for session initialization.
// This is critical because:
//
// 1. On macOS, `environ` is not available during early library initialization,
//    so we use `_NSGetEnviron()` instead.
// 2. Build systems may clear or modify environment variables like LD_PRELOAD
//    and INTERCEPT_COLLECTOR_ADDRESS. By capturing them early, we can restore
//    them when executing child processes.

__attribute__((constructor))
static void on_load(void)
{
    char *const *envp = get_environ();
    if (envp != NULL) {
        rust_session_init(envp);
    }
}

// Count variadic arguments until NULL terminator
// The va_list is consumed by this function
static size_t va_count_args(va_list ap)
{
    size_t count = 0;
    while (va_arg(ap, const char *) != NULL)
        ++count;
    return count;
}

// Copy n arguments from va_list to argv array
// Copies exactly n elements (should include the NULL terminator)
static void va_copy_args(va_list ap, char **argv, size_t n)
{
    for (size_t i = 0; i < n; ++i)
        argv[i] = va_arg(ap, char *);
}

//
// execl - execute a file
//
// int execl(const char *path, const char *arg0, ... /*, (char *)0 */);
//
#if defined(has_symbol_execl)
EXPORT int execl(const char *path, const char *arg0, ...)
{
    va_list ap;
    va_start(ap, arg0);

    // First pass: count the number of arguments after arg0
    va_list ap_count;
    va_copy(ap_count, ap);
    const size_t argc = va_count_args(ap_count);
    va_end(ap_count);

    // Second pass: copy arguments to stack-allocated VLA
    // Layout: [arg0, arg1, ..., argN, NULL] = 1 + argc + 1 elements
    char *argv[argc + 2];
    argv[0] = (char *)arg0;
    va_copy_args(ap, &argv[1], argc + 1);  // argc args + NULL terminator

    va_end(ap);

    return rust_execve(path, argv, get_environ());
}
#endif

//
// execlp - execute a file, searching PATH
//
// int execlp(const char *file, const char *arg0, ... /*, (char *)0 */);
//
#if defined(has_symbol_execlp)
EXPORT int execlp(const char *file, const char *arg0, ...)
{
    va_list ap;
    va_start(ap, arg0);

    // First pass: count the number of arguments after arg0
    va_list ap_count;
    va_copy(ap_count, ap);
    const size_t argc = va_count_args(ap_count);
    va_end(ap_count);

    // Second pass: copy arguments to stack-allocated VLA
    char *argv[argc + 2];
    argv[0] = (char *)arg0;
    va_copy_args(ap, &argv[1], argc + 1);

    va_end(ap);

#if defined(has_symbol_execvpe)
    return rust_execvpe(file, argv, get_environ());
#else
    return rust_execvp(file, argv);
#endif
}
#endif

//
// execle - execute a file with environment
//
// int execle(const char *path, const char *arg0, ... /*, (char *)0, char *const envp[] */);
//
// Note: The environment pointer comes AFTER the NULL terminator in the variadic list
//
#if defined(has_symbol_execle)
EXPORT int execle(const char *path, const char *arg0, ...)
{
    va_list ap;
    va_start(ap, arg0);

    // First pass: count the number of arguments after arg0 (excluding NULL and envp)
    va_list ap_count;
    va_copy(ap_count, ap);
    const size_t argc = va_count_args(ap_count);
    va_end(ap_count);

    // Second pass: copy arguments to stack-allocated VLA
    char *argv[argc + 2];
    argv[0] = (char *)arg0;
    va_copy_args(ap, &argv[1], argc + 1);  // argc args + NULL terminator

    // The next argument after NULL is the environment pointer
    char *const *envp = va_arg(ap, char *const *);

    va_end(ap);

    return rust_execve(path, argv, envp);
}
#endif

//
// execv - execute a file
//
#if defined(has_symbol_execv)
EXPORT int execv(const char *path, char *const argv[])
{
    return rust_execve(path, argv, get_environ());
}
#endif

//
// execve - execute a file with environment
//
#if defined(has_symbol_execve)
EXPORT int execve(const char *path, char *const argv[], char *const envp[])
{
    return rust_execve(path, argv, envp);
}
#endif

//
// execvp - execute a file, searching PATH
//
#if defined(has_symbol_execvp)
EXPORT int execvp(const char *file, char *const argv[])
{
#if defined(has_symbol_execvpe)
    return rust_execvpe(file, argv, get_environ());
#else
    return rust_execvp(file, argv);
#endif
}
#endif

//
// execvpe - execute a file, searching PATH, with environment (GNU extension)
//
#if defined(has_symbol_execvpe)
EXPORT int execvpe(const char *file, char *const argv[], char *const envp[])
{
    return rust_execvpe(file, argv, envp);
}
#endif

//
// execvP - execute a file with custom search path (BSD extension)
//
#if defined(has_symbol_execvP)
EXPORT int execvP(const char *file, const char *search_path, char *const argv[])
{
    return rust_execvP(file, search_path, argv);
}
#endif

//
// exect - execute a file with tracing (BSD, deprecated)
//
#if defined(has_symbol_exect)
EXPORT int exect(const char *path, char *const argv[], char *const envp[])
{
    return rust_exect(path, argv, envp);
}
#endif

//
// posix_spawn - spawn a process
//
#if defined(has_symbol_posix_spawn)
EXPORT int posix_spawn(pid_t *pid, const char *path,
                       const posix_spawn_file_actions_t *file_actions,
                       const posix_spawnattr_t *attrp,
                       char *const argv[], char *const envp[])
{
    return rust_posix_spawn(pid, path, file_actions, attrp, argv, envp);
}
#endif

//
// posix_spawnp - spawn a process, searching PATH
//
#if defined(has_symbol_posix_spawnp)
EXPORT int posix_spawnp(pid_t *pid, const char *file,
                        const posix_spawn_file_actions_t *file_actions,
                        const posix_spawnattr_t *attrp,
                        char *const argv[], char *const envp[])
{
    return rust_posix_spawnp(pid, file, file_actions, attrp, argv, envp);
}
#endif

//
// popen - open a pipe to a process
//
#if defined(has_symbol_popen)
EXPORT FILE *popen(const char *command, const char *mode)
{
    return rust_popen(command, mode);
}
#endif

//
// system - execute a shell command
//
#if defined(has_symbol_system)
EXPORT int system(const char *command)
{
    return rust_system(command);
}
#endif