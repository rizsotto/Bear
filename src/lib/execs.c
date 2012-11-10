// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "report.h"

#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <stdarg.h>
#include <malloc.h>

#include <dlfcn.h>


static void report_vararg_call(char const * method, const char *arg, ...);


int execv(const char *path, char *const argv[]) {
    report_call("execv", argv);
    int (*execv_ptr)(const char *path, char *const argv[])
        = dlsym(RTLD_NEXT, "execv");
    return (*execv_ptr)(path, argv);
}

int execve(const char *path, char *const argv[], char *const envp[]) {
    report_call("execve", argv);
    int (*execve_ptr)(const char *filename, char *const argv[], char *const envp[])
        = dlsym(RTLD_NEXT, "execve");
    return (*execve_ptr)(path, argv, envp);
}

int execvp(const char *file, char *const argv[]) {
    report_call("execvp", argv);
    int (*execvp_ptr)(const char *file, char *const argv[])
        = dlsym(RTLD_NEXT, "execvp");
    return (*execvp_ptr)(file, argv);
}

int execvpe(const char *file, char *const argv[], char *const envp[]) {
    report_call("execvpe", argv);
    int (*execvpe_ptr)(const char *file, char *const argv[], char *const envp[])
        = dlsym(RTLD_NEXT, "execvpe");
    return (*execvpe_ptr)(file, argv, envp);
}

int execl(const char *path, const char *arg, ...) {
    va_list args;
    va_start(args, arg);
    report_vararg_call("execl", arg, args);
    int (*execl_ptr)(const char *path, const char *arg, ...)
        = dlsym(RTLD_NEXT, "execl");
    int const result = (*execl_ptr)(path, arg, args);
    va_end(args);
    return result;
}

int execlp(const char *file, const char *arg, ...) {
    va_list args;
    va_start(args, arg);
    report_vararg_call("execlp", arg, args);
    int (*execlp_ptr)(const char *file, const char *arg, ...)
        = dlsym(RTLD_NEXT, "execlp");
    int const result = (*execlp_ptr)(file, arg, args);
    va_end(args);
    return result;
}

int execle(const char *path, const char *arg, ...) {
    va_list args;
    va_start(args, arg);
    report_vararg_call("execle", arg, args);
    int (*execle_ptr)(const char *path, const char *arg, ...)
        = dlsym(RTLD_NEXT, "execle");
    int const result = (*execle_ptr)(path, arg, args);
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
        arg_array = (char * *)realloc(arg_array, (arg_array_size + 1) * sizeof(char *));
        arg_array[arg_array_size++] = it;
    }
    report_call(method, arg_array);
    free(arg_array);

    va_end(args);
}

