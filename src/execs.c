// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "stringarray.h"
#include "environ.h"
#include "protocol.h"

#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <stdarg.h>
#include <malloc.h>
#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>

#include <dlfcn.h>


static void report_call(char const * fun, char const * const argv[]);

static int call_execve(const char * path, char * const argv[], char * const envp[]);
static int call_execvpe(const char * file, char * const argv[], char * const envp[]);

int execve(const char * path, char * const argv[], char * const envp[])
{
    report_call("execve", (char const * const *)argv);
    return call_execve(path, argv, envp);
}

int execv(const char * path, char * const argv[])
{
    report_call("execv", (char const * const *)argv);
    return call_execve(path, argv, environ);
}

int execvpe(const char * file, char * const argv[], char * const envp[])
{
    report_call("execvpe", (char const * const *)argv);
    return call_execvpe(file, argv, envp);
}

int execvp(const char * file, char * const argv[])
{
    report_call("execvp", (char const * const *)argv);
    return call_execvpe(file, argv, environ);
}

int execl(const char * path, const char * arg, ...)
{
    va_list args;
    va_start(args, arg);
    char const ** argv = bear_strings_build(arg, args);
    va_end(args);

    report_call("execl", (char const * const *)argv);
    int const result = call_execve(path, (char * const *)argv, environ);
    bear_strings_release(argv);
    return result;
}

int execlp(const char * file, const char * arg, ...)
{
    va_list args;
    va_start(args, arg);
    char const ** argv = bear_strings_build(arg, args);
    va_end(args);

    report_call("execlp", (char const * const *)argv);
    int const result = call_execvpe(file, (char * const *)argv, environ);
    bear_strings_release(argv);
    return result;
}

// int execle(const char *path, const char *arg, ..., char * const envp[]);
int execle(const char * path, const char * arg, ...)
{
    va_list args;
    va_start(args, arg);
    char const ** argv = bear_strings_build(arg, args);
    char const ** envp = va_arg(args, char const **);
    va_end(args);

    report_call("execle", (char const * const *)argv);
    int const result = call_execve(path, (char * const *)argv, (char * const *)envp);
    bear_strings_release(argv);
    return result;
}


static int call_execve(const char * path, char * const argv[], char * const envp[])
{
    int (*fp)(const char *, char * const *, char * const *) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "execve")))
    {
        perror("dlsym");
        exit(EXIT_FAILURE);
    }

    char const ** menvp = bear_strings_copy((char const * *)envp);
    menvp = bear_env_insert(menvp, ENV_PRELOAD, getenv(ENV_PRELOAD));
    menvp = bear_env_insert(menvp, ENV_OUTPUT, getenv(ENV_OUTPUT));
    int const result = (*fp)(path, argv, (char * const *)menvp);
    bear_strings_release(menvp);
    return result;
}

static int call_execvpe(const char * file, char * const argv[], char * const envp[])
{
    int (*fp)(const char *, char * const *, char * const *) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "execvpe")))
    {
        perror("dlsym");
        exit(EXIT_FAILURE);
    }

    char const ** menvp = bear_strings_copy((char const * *)envp);
    menvp = bear_env_insert(menvp, ENV_PRELOAD, getenv(ENV_PRELOAD));
    menvp = bear_env_insert(menvp, ENV_OUTPUT, getenv(ENV_OUTPUT));
    int const result = (*fp)(file, argv, (char * const *)menvp);
    bear_strings_release(menvp);
    return result;
}


typedef void (*send_message)(char const * socket, struct bear_message const *);

static void report(send_message fp, char const * socket, char const * fun, char const * const argv[])
{
    struct bear_message const msg =
    {
        getpid(),
        getppid(),
        fun,
        get_current_dir_name(),
        (char const **)argv
    };
    (*fp)(socket, &msg);
    free((void *)msg.cwd);
}

static void report_call(char const * fun, char const * const argv[])
{
    char * const socket = getenv(ENV_OUTPUT);
    if (0 == socket)
    {
        perror("getenv");
        exit(EXIT_FAILURE);
    }

    return report(bear_send_message, socket, fun, argv);
}
