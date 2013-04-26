// This file is distributed under MIT-LICENSE. See COPYING for details.

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


static void report_call(char const * fun, char const * const argv[]);

static int call_execve(const char * path, char * const argv[], char * const envp[]);
static int call_execvpe(const char * file, char * const argv[], char * const envp[]);
static int call_execvp(const char * file, char * const argv[]);
#ifdef HAVE_EXECVP2
static int call_execvP(const char * file, const char * search_path, char * const argv[]);
#endif

static int already_reported = 0;

#ifdef HAVE_EXECVE
int execve(const char * path, char * const argv[], char * const envp[])
{
    int clear_reported = (!already_reported);
    report_call("execve", (char const * const *)argv);
    int const result = call_execve(path, argv, envp);
    if (clear_reported)
        already_reported = 0;
    return result;
}
#endif

#ifdef HAVE_EXECV
int execv(const char * path, char * const argv[])
{
    int clear_reported = (!already_reported);
    report_call("execv", (char const * const *)argv);
    int const result = call_execve(path, argv, bear_get_environ());
    if (clear_reported)
        already_reported = 0;
    return result;
}
#endif

#ifdef HAVE_EXECVPE
int execvpe(const char * file, char * const argv[], char * const envp[])
{
    int clear_reported = (!already_reported);
    report_call("execvpe", (char const * const *)argv);
    int const result = call_execvpe(file, argv, envp);
    if (clear_reported)
        already_reported = 0;
    return result;
}
#endif

#ifdef HAVE_EXECVP
int execvp(const char * file, char * const argv[])
{
    int clear_reported = (!already_reported);
    report_call("execvp", (char const * const *)argv);
    int const result = call_execvp(file, argv);
    if (clear_reported)
        already_reported = 0;
    return result;
}
#endif

#ifdef HAVE_EXECVP2
int execvP(const char * file, const char * search_path, char * const argv[])
{
    int clear_reported = (!already_reported);
    report_call("execvP", (char const * const *)argv);
    int const result = call_execvP(file, search_path, argv);
    if (clear_reported)
        already_reported = 0;
    return result;
}
#endif

#ifdef HAVE_EXECL
int execl(const char * path, const char * arg, ...)
{
    va_list args;
    va_start(args, arg);
    char const ** argv = bear_strings_build(arg, args);
    va_end(args);

    int clear_reported = (!already_reported);
    report_call("execl", (char const * const *)argv);
    int const result = call_execve(path, (char * const *)argv, bear_get_environ());
    bear_strings_release(argv);
    if (clear_reported)
        already_reported = 0;
    return result;
}
#endif

#ifdef HAVE_EXECLP
int execlp(const char * file, const char * arg, ...)
{
    va_list args;
    va_start(args, arg);
    char const ** argv = bear_strings_build(arg, args);
    va_end(args);

    int clear_reported = (!already_reported);
    report_call("execlp", (char const * const *)argv);
    int const result = call_execvp(file, (char * const *)argv);
    bear_strings_release(argv);
    if (clear_reported)
        already_reported = 0;
    return result;
}
#endif

#ifdef HAVE_EXECLE
// int execle(const char *path, const char *arg, ..., char * const envp[]);
int execle(const char * path, const char * arg, ...)
{
    va_list args;
    va_start(args, arg);
    char const ** argv = bear_strings_build(arg, args);
    char const ** envp = va_arg(args, char const **);
    va_end(args);

    int clear_reported = (!already_reported);
    report_call("execle", (char const * const *)argv);
    int const result = call_execve(path, (char * const *)argv, (char * const *)envp);
    bear_strings_release(argv);
    if (clear_reported)
        already_reported = 0;
    return result;
}
#endif


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
#ifdef ENV_FLAT
    menvp = bear_env_insert(menvp, ENV_FLAT, getenv(ENV_FLAT));
#endif
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
#ifdef ENV_FLAT
    menvp = bear_env_insert(menvp, ENV_FLAT, getenv(ENV_FLAT));
#endif
    int const result = (*fp)(file, argv, (char * const *)menvp);
    bear_strings_release(menvp);
    return result;
}

static int call_execvp(const char * file, char * const argv[]) 
{
    int (*fp)(const char * file, char * const argv[]) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "execvp")))
    {
        perror("dlsym");
        exit(EXIT_FAILURE);
    }
    
    return (*fp)(file, argv);
} 

#ifdef HAVE_EXECVP2
static int call_execvP(const char * file, const char * search_path, char * const argv[])
{
    int (*fp)(const char *, const char *, char * const *) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "execvP")))
    {
        perror("dlsym");
        exit(EXIT_FAILURE);
    }

    return (*fp)(file, search_path, argv);
}
#endif

typedef void (*send_message)(char const * socket, struct bear_message const *);

static void report(send_message fp, char const * socket, char const * fun, char const * const argv[])
{
    struct bear_message const msg =
    {
        getpid(),
        getppid(),
        fun,
        getcwd(NULL, 0),
        (char const **)argv
    };
    (*fp)(socket, &msg);
    free((void *)msg.cwd);
    already_reported = 1;
}

static void report_call(char const * fun, char const * const argv[])
{
    if (already_reported)
        return;
    char * const socket = getenv(ENV_OUTPUT);
    if (0 == socket)
    {
        perror("getenv");
        exit(EXIT_FAILURE);
    }

    return report(bear_send_message, socket, fun, argv);
}
