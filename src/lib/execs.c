// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "report.h"
#include "../common/stringarray.h"

#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <stdarg.h>
#include <malloc.h>
#include <stdlib.h>
#include <stdio.h>

#include <dlfcn.h>


static void report_vararg_call(char const * method, const char *arg, ...);
static Strings extend_env_array(Strings);


int execv(const char *path, char *const argv[]) {
    report_call("execv", argv);

    int (*fp)(const char *, char *const *) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "execv"))) {
        perror("dlsym");
        exit(EXIT_FAILURE);
    }
    return (*fp)(path, argv);
}

int execve(const char *path, char *const argv[], char *const envp[]) {
    report_call("execve", argv);

    int (*fp)(const char *, char *const *, char *const *) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "execve"))) {
        perror("dlsym");
        exit(EXIT_FAILURE);
    }

    Strings menvp = extend_env_array((Strings)envp);
    int const result = (*fp)(path, argv, (char *const *)menvp);
    sa_release(menvp);
    return result;
}

int execvp(const char *file, char *const argv[]) {
    report_call("execvp", argv);

    int (*fp)(const char *, char *const *) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "execvp"))) {
        perror("dlsym");
        exit(EXIT_FAILURE);
    }
    return (*fp)(file, argv);
}

int execvpe(const char *file, char *const argv[], char *const envp[]) {
    report_call("execvpe", argv);

    int (*fp)(const char *, char *const *, char *const *) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "execvpe"))) {
        perror("dlsym");
        exit(EXIT_FAILURE);
    }

    Strings menvp = extend_env_array((Strings)envp);
    int const result = (*fp)(file, argv, (char *const *)menvp);
    sa_release(menvp);
    return result;
}

int execl(const char *path, const char *arg, ...) {
    va_list args;
    va_start(args, arg);
    report_vararg_call("execl", arg, args);

    int (*fp)(const char *, const char *, ...) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "execl"))) {
        perror("dlsym");
        exit(EXIT_FAILURE);
    }

    int const result = (*fp)(path, arg, args);
    va_end(args);
    return result;
}

int execlp(const char *file, const char *arg, ...) {
    va_list args;
    va_start(args, arg);
    report_vararg_call("execlp", arg, args);

    int (*fp)(const char *, const char *, ...) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "execlp"))) {
        perror("dlsym");
        exit(EXIT_FAILURE);
    }

    int const result = (*fp)(file, arg, args);
    va_end(args);
    return result;
}

// int execle(const char *path, const char *arg, ..., char * const envp[]);
int execle(const char *path, const char *arg, ...) {
    va_list args;
    va_start(args, arg);
    report_vararg_call("execle", arg, args);

    int (*fp)(const char *, const char *, ...) = 0;
    if (0 == (fp = dlsym(RTLD_NEXT, "execle"))) {
        perror("dlsym");
        exit(EXIT_FAILURE);
    }

    int const result = (*fp)(path, arg, args);
    va_end(args);
    return result;
}


static void report_vararg_call(char const * method, const char *arg, ...) {
    va_list args;
    va_start(args, arg);

    char * it = (char *)arg;
    char * * arg_array = 0;
    size_t arg_array_size = 0;
    for (; *it; it = va_arg(args, char *)) {
        arg_array = (char * *)realloc(arg_array,
                                      (arg_array_size + 1) * sizeof(char *));
        arg_array[arg_array_size++] = it;
    }
    report_call(method, arg_array);
    free(arg_array);

    va_end(args);
}

static String create_env(char const * key) {
    char * result = 0;
    char * const value = getenv(key);
    if (value) {
        if (-1 == asprintf(&result, "%s=%s", key, value)) {
            perror("asprintf");
            exit(EXIT_FAILURE);
        }
    }
    return result;
}

static Strings extend_env_array(Strings in) {
    Strings result = sa_copy(in);
    result = sa_append(result, create_env("LD_PRELOAD"));
    result = sa_append(result, create_env("BEAR_OUTPUT"));
    return result;
}
