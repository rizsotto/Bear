/*  Copyright (C) 2012, 2013 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#include "config.h"

#include "stringarray.h"
#include "environ.h"
#include "protocol.h"

#include <sys/types.h>
#include <sys/stat.h>
#include <stdarg.h>
#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>

#include <dlfcn.h>

#if defined HAVE_POSIX_SPAWN || defined HAVE_POSIX_SPAWNP
#include <spawn.h>
#endif


static int already_reported = 0;

static char const * * update_environment(char * const envp[]);

static void report_call(char const * fun, char const * const argv[]);
static void report_failed_call(char const * fun, int result, int report_state);

#ifdef HAVE_EXECVE
static int call_execve(const char * path, char * const argv[], char * const envp[]);
#endif
#ifdef HAVE_EXECVP
static int call_execvp(const char * file, char * const argv[]);
#endif
#ifdef HAVE_EXECVPE
static int call_execvpe(const char * file, char * const argv[], char * const envp[]);
#endif
#ifdef HAVE_EXECVP2
static int call_execvP(const char * file, const char * search_path, char * const argv[]);
#endif
#ifdef HAVE_POSIX_SPAWN
static int call_posix_spawn(pid_t *restrict pid,
                            const char *restrict path,
                            const posix_spawn_file_actions_t *file_actions,
                            const posix_spawnattr_t *restrict attrp,
                            char *const argv[restrict],
                            char *const envp[restrict]);
#endif
#ifdef HAVE_POSIX_SPAWNP
static int call_posix_spawnp(pid_t *restrict pid,
                            const char *restrict file,
                            const posix_spawn_file_actions_t *file_actions,
                            const posix_spawnattr_t *restrict attrp,
                            char *const argv[restrict],
                            char * const envp[restrict]);
#endif


#ifdef HAVE_VFORK
pid_t vfork(void)
{
    return fork();
}
#endif

#ifdef HAVE_EXECVE
int execve(const char * path, char * const argv[], char * const envp[])
{
    int const report_state = already_reported;

    report_call("execve", (char const * const *)argv);
    int const result = call_execve(path, argv, envp);
    report_failed_call("execve", result, report_state);

    return result;
}
#endif

#ifdef HAVE_EXECV
# ifndef HAVE_EXECVE
#  error can not implement execv without execve
# endif
int execv(const char * path, char * const argv[])
{
    int const report_state = already_reported;

    report_call("execv", (char const * const *)argv);
    int const result = call_execve(path, argv, bear_get_environ());
    report_failed_call("execv", result, report_state);
    return result;
}
#endif

#ifdef HAVE_EXECVPE
int execvpe(const char * file, char * const argv[], char * const envp[])
{
    int const report_state = already_reported;

    report_call("execvpe", (char const * const *)argv);
    int const result = call_execvpe(file, argv, envp);
    report_failed_call("execvpe", result, report_state);

    return result;
}
#endif

#ifdef HAVE_EXECVP
int execvp(const char * file, char * const argv[])
{
    int const report_state = already_reported;

    report_call("execvp", (char const * const *)argv);
    int const result = call_execvp(file, argv);
    report_failed_call("execvp", result, report_state);
    return result;
}
#endif

#ifdef HAVE_EXECVP2
int execvP(const char * file, const char * search_path, char * const argv[])
{
    int const report_state = already_reported;

    report_call("execvP", (char const * const *)argv);
    int const result = call_execvP(file, search_path, argv);
    report_failed_call("execvP", result, report_state);
    return result;
}
#endif

#ifdef HAVE_EXECL
# ifndef HAVE_EXECVE
#  error can not implement execl without execve
# endif
int execl(const char * path, const char * arg, ...)
{
    va_list args;
    va_start(args, arg);
    char const ** argv = bear_strings_build(arg, &args);
    va_end(args);

    report_call("execl", (char const * const *)argv);
    int const result = call_execve(path, (char * const *)argv, bear_get_environ());
    report_failed_call("execl", result, 0);
    bear_strings_release(argv);
    return result;
}
#endif

#ifdef HAVE_EXECLP
# ifndef HAVE_EXECVP
#  error can not implement execlp without execvp
# endif
int execlp(const char * file, const char * arg, ...)
{
    va_list args;
    va_start(args, arg);
    char const ** argv = bear_strings_build(arg, &args);
    va_end(args);

    report_call("execlp", (char const * const *)argv);
    int const result = call_execvp(file, (char * const *)argv);
    report_failed_call("execlp", result, 0);
    bear_strings_release(argv);
    return result;
}
#endif

#ifdef HAVE_EXECLE
# ifndef HAVE_EXECVE
#  error can not implement execle without execve
# endif
// int execle(const char *path, const char *arg, ..., char * const envp[]);
int execle(const char * path, const char * arg, ...)
{
    va_list args;
    va_start(args, arg);
    char const ** argv = bear_strings_build(arg, &args);
    char const ** envp = va_arg(args, char const **);
    va_end(args);

    report_call("execle", (char const * const *)argv);
    int const result = call_execve(path, (char * const *)argv, (char * const *)envp);
    report_failed_call("execle", result, 0);
    bear_strings_release(argv);
    return result;
}
#endif

#ifdef HAVE_POSIX_SPAWN
int posix_spawn(pid_t *restrict pid,
                const char *restrict path,
                const posix_spawn_file_actions_t *file_actions,
                const posix_spawnattr_t *restrict attrp,
                char *const argv[restrict],
                char *const envp[restrict])
{
    int const report_state = already_reported;

    report_call("posix_spawn", (char const * const *)argv);
    int const result = call_posix_spawn(pid, path, file_actions, attrp, argv, envp);
    report_failed_call("posix_spawn", result, report_state);
    return result;
}
#endif

#ifdef HAVE_POSIX_SPAWNP
int posix_spawnp(pid_t *restrict pid,
                const char *restrict file,
                const posix_spawn_file_actions_t *file_actions,
                const posix_spawnattr_t *restrict attrp,
                char *const argv[restrict],
                char * const envp[restrict])
{
    int const report_state = already_reported;

    report_call("posix_spawnp", (char const * const *)argv);
    int const result = call_posix_spawnp(pid, file, file_actions, attrp, argv, envp);
    report_failed_call("posix_spawnp", result, report_state);
    return result;
}
#endif

#ifdef HAVE_EXECVE
static int call_execve(const char * path, char * const argv[], char * const envp[])
{
    int (*fp)(const char *, char * const *, char * const *) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "execve")))
    {
        perror("bear: dlsym");
        exit(EXIT_FAILURE);
    }

    char const ** const menvp = update_environment(envp);
    int const result = (*fp)(path, argv, (char * const *)menvp);
    bear_strings_release(menvp);
    return result;
}
#endif

#ifdef HAVE_EXECVPE
static int call_execvpe(const char * file, char * const argv[], char * const envp[])
{
    int (*fp)(const char *, char * const *, char * const *) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "execvpe")))
    {
        perror("bear: dlsym");
        exit(EXIT_FAILURE);
    }

    char const ** const menvp = update_environment(envp);
    int const result = (*fp)(file, argv, (char * const *)menvp);
    bear_strings_release(menvp);
    return result;
}
#endif

#ifdef HAVE_EXECVP
static int call_execvp(const char * file, char * const argv[])
{
    int (*fp)(const char * file, char * const argv[]) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "execvp")))
    {
        perror("bear: dlsym");
        exit(EXIT_FAILURE);
    }
    
    return (*fp)(file, argv);
} 
#endif

#ifdef HAVE_EXECVP2
static int call_execvP(const char * file, const char * search_path, char * const argv[])
{
    int (*fp)(const char *, const char *, char * const *) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "execvP")))
    {
        perror("bear: dlsym");
        exit(EXIT_FAILURE);
    }

    return (*fp)(file, search_path, argv);
}
#endif

#ifdef HAVE_POSIX_SPAWN
static int call_posix_spawn(pid_t *restrict pid,
                            const char *restrict path,
                            const posix_spawn_file_actions_t *file_actions,
                            const posix_spawnattr_t *restrict attrp,
                            char *const argv[restrict],
                            char *const envp[restrict])
{
    int (*fp)(pid_t *restrict,
            const char *restrict,
            const posix_spawn_file_actions_t *,
            const posix_spawnattr_t *restrict,
            char *const * restrict,
            char *const * restrict) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "posix_spawn")))
    {
        perror("bear: dlsym");
        exit(EXIT_FAILURE);
    }

    return (*fp)(pid, path, file_actions, attrp, argv, envp);
}
#endif

#ifdef HAVE_POSIX_SPAWNP
static int call_posix_spawnp(pid_t *restrict pid,
                            const char *restrict file,
                            const posix_spawn_file_actions_t *file_actions,
                            const posix_spawnattr_t *restrict attrp,
                            char *const argv[restrict],
                            char * const envp[restrict])
{
    int (*fp)(pid_t *restrict,
            const char *restrict,
            const posix_spawn_file_actions_t *,
            const posix_spawnattr_t *restrict,
            char *const *restrict,
            char * const *restrict) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "posix_spawnp")))
    {
        perror("bear: dlsym");
        exit(EXIT_FAILURE);
    }

    return (*fp)(pid, file, file_actions, attrp, argv, envp);
}
#endif

static char const * * update_environment(char * const envp[])
{
    char const ** result = bear_strings_copy((char const * *)envp);
    result = bear_update_environ(result, ENV_PRELOAD);
    result = bear_update_environ(result, ENV_OUTPUT);
#ifdef ENV_FLAT
    result = bear_update_environ(result, ENV_FLAT);
#endif
    return result;
}

typedef void (*send_message)(char const * socket, bear_message_t const *);

static void report(send_message fp, char const * socket, char const * fun, char const * const argv[])
{
    bear_message_t const msg =
    {
        getpid(),
        getppid(),
        fun,
        getcwd(NULL, 0),
        (char const **)argv
    };
    (*fp)(socket, &msg);
    free((void *)msg.cwd);
}

static void report_call(char const * fun, char const * const argv[])
{
    if (already_reported)
        return;
    already_reported = 1;

    char * const socket = getenv(ENV_OUTPUT);
    if (0 == socket)
    {
        perror("bear: getenv");
        exit(EXIT_FAILURE);
    }

    return report(bear_send_message, socket, fun, argv);
}

static void report_failed_call(char const * fun, int result_code, int report_state)
{
    if (!report_state)
        already_reported = 0;
}
