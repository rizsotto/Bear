/* -*- coding: utf-8 -*-
//                     The LLVM Compiler Infrastructure
//
// This file is distributed under the University of Illinois Open Source
// License. See LICENSE.TXT for details.
*/

/**
 * This file implements a shared library. This library can be pre-loaded by
 * the dynamic linker of the Operating System (OS). It implements a few function
 * related to process creation. By pre-load this library the executed process
 * uses these functions instead of those from the standard library.
 *
 * The idea here is to inject a logic before call the real methods. The logic is
 * to dump the call into a file. To call the real method this library is doing
 * the job of the dynamic linker.
 *
 * The only input for the log writing is about the destination directory.
 * This is passed as environment variable.
 */

#include "config.h"

#include <sys/types.h>
#include <sys/stat.h>
#include <ctype.h>
#include <stddef.h>
#include <stdarg.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <dlfcn.h>

#if defined HAVE_POSIX_SPAWN || defined HAVE_POSIX_SPAWNP
#include <spawn.h>
#endif

#if defined HAVE_NSGETENVIRON
#include <crt_externs.h>
static char **environ;
#else
extern char **environ;
#endif


typedef char const * bear_env_t[ENV_SIZE];

static void bear_capture_env_t(bear_env_t *env);
static int bear_is_valid_env_t(bear_env_t *env);
static void bear_restore_env_t(bear_env_t *env);
static void bear_release_env_t(bear_env_t *env);
static char const **bear_update_environment(char *const envp[], bear_env_t *env);
static char const **bear_update_environ(char const **in, char const *key, char const *value);
static void bear_report_call(char const *fun, char const *const argv[]);
static char const **bear_strings_build(char const *arg, va_list *ap);
static char const **bear_strings_copy(char const **const in);
static char const **bear_strings_append(char const **in, char const *e);
static size_t bear_strings_length(char const *const *in);
static void bear_strings_release(char const **);


static bear_env_t env_names =
    { ENV_OUTPUT
    , ENV_PRELOAD
#ifdef ENV_FLAT
    , ENV_FLAT
#endif
    };

static bear_env_t initial_env =
    { 0
    , 0
#ifdef ENV_FLAT
    , 0
#endif
    };

static void on_load(void) __attribute__((constructor));
static void on_unload(void) __attribute__((destructor));


#define DLSYM(TYPE_, VAR_, SYMBOL_)                                            \
    union {                                                                    \
        void *from;                                                            \
        TYPE_ to;                                                              \
    } cast;                                                                    \
    if (0 == (cast.from = dlsym(RTLD_NEXT, SYMBOL_))) {                        \
        perror("bear: dlsym");                                                 \
        exit(EXIT_FAILURE);                                                    \
    }                                                                          \
    TYPE_ const VAR_ = cast.to;


#ifdef HAVE_EXECVE
static int call_execve(const char *path, char *const argv[],
                       char *const envp[]);
#endif
#ifdef HAVE_EXECVP
static int call_execvp(const char *file, char *const argv[]);
#endif
#ifdef HAVE_EXECVPE
static int call_execvpe(const char *file, char *const argv[],
                        char *const envp[]);
#endif
#ifdef HAVE_EXECVP2
static int call_execvP(const char *file, const char *search_path,
                       char *const argv[]);
#endif
#ifdef HAVE_POSIX_SPAWN
static int call_posix_spawn(pid_t *restrict pid, const char *restrict path,
                            const posix_spawn_file_actions_t *file_actions,
                            const posix_spawnattr_t *restrict attrp,
                            char *const argv[restrict],
                            char *const envp[restrict]);
#endif
#ifdef HAVE_POSIX_SPAWNP
static int call_posix_spawnp(pid_t *restrict pid, const char *restrict file,
                             const posix_spawn_file_actions_t *file_actions,
                             const posix_spawnattr_t *restrict attrp,
                             char *const argv[restrict],
                             char *const envp[restrict]);
#endif


/* Initialization method to Captures the relevant environment variables.
 */

static void on_load(void) {
#ifdef HAVE_NSGETENVIRON
    environ = *_NSGetEnviron();
#endif
    bear_capture_env_t(&initial_env);
}

static void on_unload(void) {
    bear_release_env_t(&initial_env);
}


/* These are the methods we are try to hijack.
 */

#ifdef HAVE_EXECVE
int execve(const char *path, char *const argv[], char *const envp[]) {
    bear_report_call(__func__, (char const *const *)argv);
    return call_execve(path, argv, envp);
}
#endif

#ifdef HAVE_EXECV
#ifndef HAVE_EXECVE
#error can not implement execv without execve
#endif
int execv(const char *path, char *const argv[]) {
    bear_report_call(__func__, (char const *const *)argv);
    return call_execve(path, argv, environ);
}
#endif

#ifdef HAVE_EXECVPE
int execvpe(const char *file, char *const argv[], char *const envp[]) {
    bear_report_call(__func__, (char const *const *)argv);
    return call_execvpe(file, argv, envp);
}
#endif

#ifdef HAVE_EXECVP
int execvp(const char *file, char *const argv[]) {
    bear_report_call(__func__, (char const *const *)argv);
    return call_execvp(file, argv);
}
#endif

#ifdef HAVE_EXECVP2
int execvP(const char *file, const char *search_path, char *const argv[]) {
    bear_report_call(__func__, (char const *const *)argv);
    return call_execvP(file, search_path, argv);
}
#endif

#ifdef HAVE_EXECL
#ifndef HAVE_EXECVE
#error can not implement execl without execve
#endif
int execl(const char *path, const char *arg, ...) {
    va_list args;
    va_start(args, arg);
    char const **argv = bear_strings_build(arg, &args);
    va_end(args);

    bear_report_call(__func__, (char const *const *)argv);
    int const result = call_execve(path, (char *const *)argv, environ);

    bear_strings_release(argv);
    return result;
}
#endif

#ifdef HAVE_EXECLP
#ifndef HAVE_EXECVP
#error can not implement execlp without execvp
#endif
int execlp(const char *file, const char *arg, ...) {
    va_list args;
    va_start(args, arg);
    char const **argv = bear_strings_build(arg, &args);
    va_end(args);

    bear_report_call(__func__, (char const *const *)argv);
    int const result = call_execvp(file, (char *const *)argv);

    bear_strings_release(argv);
    return result;
}
#endif

#ifdef HAVE_EXECLE
#ifndef HAVE_EXECVE
#error can not implement execle without execve
#endif
// int execle(const char *path, const char *arg, ..., char * const envp[]);
int execle(const char *path, const char *arg, ...) {
    va_list args;
    va_start(args, arg);
    char const **argv = bear_strings_build(arg, &args);
    char const **envp = va_arg(args, char const **);
    va_end(args);

    bear_report_call(__func__, (char const *const *)argv);
    int const result =
        call_execve(path, (char *const *)argv, (char *const *)envp);

    bear_strings_release(argv);
    return result;
}
#endif

#ifdef HAVE_POSIX_SPAWN
int posix_spawn(pid_t *restrict pid, const char *restrict path,
                const posix_spawn_file_actions_t *file_actions,
                const posix_spawnattr_t *restrict attrp,
                char *const argv[restrict], char *const envp[restrict]) {
    bear_report_call(__func__, (char const *const *)argv);
    return call_posix_spawn(pid, path, file_actions, attrp, argv, envp);
}
#endif

#ifdef HAVE_POSIX_SPAWNP
int posix_spawnp(pid_t *restrict pid, const char *restrict file,
                 const posix_spawn_file_actions_t *file_actions,
                 const posix_spawnattr_t *restrict attrp,
                 char *const argv[restrict], char *const envp[restrict]) {
    bear_report_call(__func__, (char const *const *)argv);
    return call_posix_spawnp(pid, file, file_actions, attrp, argv, envp);
}
#endif

/* These are the methods which forward the call to the standard implementation.
 */

#ifdef HAVE_EXECVE
static int call_execve(const char *path, char *const argv[],
                       char *const envp[]) {
    typedef int (*func)(const char *, char *const *, char *const *);

    DLSYM(func, fp, "execve");

    char const **const menvp = bear_update_environment(envp, &initial_env);
    int const result = (*fp)(path, argv, (char *const *)menvp);
    bear_strings_release(menvp);
    return result;
}
#endif

#ifdef HAVE_EXECVPE
static int call_execvpe(const char *file, char *const argv[],
                        char *const envp[]) {
    typedef int (*func)(const char *, char *const *, char *const *);

    DLSYM(func, fp, "execvpe");

    char const **const menvp = bear_update_environment(envp, &initial_env);
    int const result = (*fp)(file, argv, (char *const *)menvp);
    bear_strings_release(menvp);
    return result;
}
#endif

#ifdef HAVE_EXECVP
static int call_execvp(const char *file, char *const argv[]) {
    typedef int (*func)(const char *file, char *const argv[]);

    DLSYM(func, fp, "execvp");

    bear_env_t current;
    bear_capture_env_t(&current);
    bear_restore_env_t(&initial_env);
    int const result = (*fp)(file, argv);
    bear_restore_env_t(&current);
    bear_release_env_t(&current);

    return result;
}
#endif

#ifdef HAVE_EXECVP2
static int call_execvP(const char *file, const char *search_path,
                       char *const argv[]) {
    typedef int (*func)(const char *, const char *, char *const *);

    DLSYM(func, fp, "execvP");

    bear_env_t current;
    bear_capture_env_t(&current);
    bear_restore_env_t(&initial_env);
    int const result = (*fp)(file, search_path, argv);
    bear_restore_env_t(&current);
    bear_release_env_t(&current);

    return result;
}
#endif

#ifdef HAVE_POSIX_SPAWN
static int call_posix_spawn(pid_t *restrict pid, const char *restrict path,
                            const posix_spawn_file_actions_t *file_actions,
                            const posix_spawnattr_t *restrict attrp,
                            char *const argv[restrict],
                            char *const envp[restrict]) {
    typedef int (*func)(pid_t *restrict, const char *restrict,
                        const posix_spawn_file_actions_t *,
                        const posix_spawnattr_t *restrict,
                        char *const *restrict, char *const *restrict);

    DLSYM(func, fp, "posix_spawn");

    char const **const menvp = bear_update_environment(envp, &initial_env);
    int const result =
        (*fp)(pid, path, file_actions, attrp, argv, (char *const *restrict)menvp);
    bear_strings_release(menvp);
    return result;
}
#endif

#ifdef HAVE_POSIX_SPAWNP
static int call_posix_spawnp(pid_t *restrict pid, const char *restrict file,
                             const posix_spawn_file_actions_t *file_actions,
                             const posix_spawnattr_t *restrict attrp,
                             char *const argv[restrict],
                             char *const envp[restrict]) {
    typedef int (*func)(pid_t *restrict, const char *restrict,
                        const posix_spawn_file_actions_t *,
                        const posix_spawnattr_t *restrict,
                        char *const *restrict, char *const *restrict);

    DLSYM(func, fp, "posix_spawnp");

    char const **const menvp = bear_update_environment(envp, &initial_env);
    int const result =
        (*fp)(pid, file, file_actions, attrp, argv, (char *const *restrict)menvp);
    bear_strings_release(menvp);
    return result;
}
#endif

/* this method is to write log about the process creation. */

static void bear_report_call(char const *fun, char const *const argv[]) {
    static int const RS = 0x1e;
    static int const US = 0x1f;

    if (!bear_is_valid_env_t(&initial_env))
        return;

    const char *cwd = getcwd(NULL, 0);
    if (0 == cwd) {
        perror("bear: getcwd");
        exit(EXIT_FAILURE);
    }
    char *filename = 0;
    if (-1 == asprintf(&filename, "%s/cmd.XXXXXX", initial_env[0])) {
        perror("bear: asprintf");
        exit(EXIT_FAILURE);
    }
    int fd = mkstemp(filename);
    if (-1 == fd) {
        perror("bear: open");
        exit(EXIT_FAILURE);
    }
    dprintf(fd, "%d%c", getpid(), RS);
    dprintf(fd, "%d%c", getppid(), RS);
    dprintf(fd, "%s%c", fun, RS);
    dprintf(fd, "%s%c", cwd, RS);
    size_t const argc = bear_strings_length(argv);
    for (size_t it = 0; it < argc; ++it) {
        dprintf(fd, "%s%c", argv[it], US);
    }
    close(fd);
    free((void *)filename);
    free((void *)cwd);
}

/* update environment assure that chilren processes will copy the desired
 * behaviour */

static void bear_capture_env_t(bear_env_t *env) {
    for (size_t it = 0; it < ENV_SIZE; ++it) {
        char const * const env_value = getenv(env_names[it]);
        *env[it] = (env_value) ? strdup(env_value) : env_value;
    }
}

static int bear_is_valid_env_t(bear_env_t *env) {
    for (size_t it = 0; it < ENV_SIZE; ++it)
        if (*env[it])
            return -1;
    return 0;
}

static void bear_restore_env_t(bear_env_t *env) {
    for (size_t it = 0; it < ENV_SIZE; ++it)
        if ((*env[it])
                ? setenv(env_names[it], *env[it], 1)
                : unsetenv(env_names[it])) {
            perror("bear: setenv");
            exit(EXIT_FAILURE);
        }
}

static void bear_release_env_t(bear_env_t *env) {
    for (size_t it = 0; it < ENV_SIZE; ++it)
        free((void *)*env[it]);
}

static char const **bear_update_environment(char *const envp[], bear_env_t *env) {
    char const **result = bear_strings_copy((char const **)envp);
    if (bear_is_valid_env_t(env))
        for (size_t it = 0; it < ENV_SIZE; ++it)
            result = bear_update_environ(result, env_names[it], *env[it]);
    return result;
}

static char const **bear_update_environ(char const *envs[], char const *key, char const * const value) {
    // find the key if it's there
    size_t const key_length = strlen(key);
    char const **it = envs;
    for (; (it) && (*it); ++it) {
        if ((0 == strncmp(*it, key, key_length)) &&
            (strlen(*it) > key_length) && ('=' == (*it)[key_length]))
            break;
    }
    // check the value might already correct
    char const **result = envs;
    if ((0 != it) && ((0 == *it) || (strcmp(*it + key_length + 1, value)))) {
        char *env = 0;
        if (-1 == asprintf(&env, "%s=%s", key, value)) {
            perror("bear: asprintf");
            exit(EXIT_FAILURE);
        }
        if (*it) {
            free((void *)*it);
            *it = env;
        } else
            result = bear_strings_append(envs, env);
    }
    return result;
}

/* util methods to deal with string arrays. environment and process arguments
 * are both represented as string arrays. */

static char const **bear_strings_build(char const *const arg, va_list *args) {
    char const **result = 0;
    size_t size = 0;
    for (char const *it = arg; it; it = va_arg(*args, char const *)) {
        result = realloc(result, (size + 1) * sizeof(char const *));
        if (0 == result) {
            perror("bear: realloc");
            exit(EXIT_FAILURE);
        }
        char const *copy = strdup(it);
        if (0 == copy) {
            perror("bear: strdup");
            exit(EXIT_FAILURE);
        }
        result[size++] = copy;
    }
    result = realloc(result, (size + 1) * sizeof(char const *));
    if (0 == result) {
        perror("bear: realloc");
        exit(EXIT_FAILURE);
    }
    result[size++] = 0;

    return result;
}

static char const **bear_strings_copy(char const **const in) {
    size_t const size = bear_strings_length(in);

    char const **const result = malloc((size + 1) * sizeof(char const *));
    if (0 == result) {
        perror("bear: malloc");
        exit(EXIT_FAILURE);
    }

    char const **out_it = result;
    for (char const *const *in_it = in; (in_it) && (*in_it);
         ++in_it, ++out_it) {
        *out_it = strdup(*in_it);
        if (0 == *out_it) {
            perror("bear: strdup");
            exit(EXIT_FAILURE);
        }
    }
    *out_it = 0;
    return result;
}

static char const **bear_strings_append(char const **const in,
                                        char const *const e) {
    size_t size = bear_strings_length(in);
    char const **result = realloc(in, (size + 2) * sizeof(char const *));
    if (0 == result) {
        perror("bear: realloc");
        exit(EXIT_FAILURE);
    }
    result[size++] = e;
    result[size++] = 0;
    return result;
}

static size_t bear_strings_length(char const *const *const in) {
    size_t result = 0;
    for (char const *const *it = in; (it) && (*it); ++it)
        ++result;
    return result;
}

static void bear_strings_release(char const **in) {
    for (char const *const *it = in; (it) && (*it); ++it) {
        free((void *)*it);
    }
    free((void *)in);
}
