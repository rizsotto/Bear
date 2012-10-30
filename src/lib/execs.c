// This file is distributed under MIT-LICENSE. See COPYING for details.

#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>

#include <dlfcn.h>


void report_call(const char *method, char const * const argv[]);


int execv(const char *path, char *const argv[]) {
    report_call("execv", argv);
    int (*execv_ptr)(const char *path, char *const argv[]) =
        dlsym(RTLD_NEXT, "execv");
    return (*execv_ptr)(path, argv);
}

int execve(const char *path, char *const argv[], char *const envp[]) {
    report_call("execve", argv);
    int (*execve_ptr)(const char *filename, char *const argv[], char *const envp[]) =
        dlsym(RTLD_NEXT, "execve");
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

