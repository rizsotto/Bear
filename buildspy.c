// Copyright 2012 by Laszlo Nagy [see file MIT-LICENSE]

#define _GNU_SOURCE

#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>

#include <dlfcn.h>


// TODO: TBD!!!
size_t length(const char * str) {
    size_t result = 0;
    for (;*str;++str) {
        ++result;
    }
    return result;
}

void report_full_call(const char *path, char *const argv[], const char *cwd) {
    int fd = open("/tmp/test.out", O_CREAT|O_APPEND|O_RDWR, S_IRUSR|S_IWUSR);
    write(fd, path, length(path));
    write(fd, "\n", 1);
    close(fd);
}

void report_call(const char *path, char *const argv[]) {
    // get current working dir
    report_full_call(path, argv, 0);
}
// TODO: TBD!!!

int execv(const char *path, char *const argv[]) {
    report_call(path, argv);
    int (*execv_ptr)(const char *path, char *const argv[]) =
        dlsym(RTLD_NEXT, "execv");
    return (*execv_ptr)(path, argv);
}

int execve(const char *path, char *const argv[], char *const envp[]) {
    report_call(path, argv);
    int (*execve_ptr)(const char *filename, char *const argv[], char *const envp[]) =
        dlsym(RTLD_NEXT, "execve");
    return (*execve_ptr)(path, argv, envp);
}

int execvp(const char *file, char *const argv[]) {
    report_call(file, argv);
    int (*execvp_ptr)(const char *file, char *const argv[])
        = dlsym(RTLD_NEXT, "execvp");
    return (*execvp_ptr)(file, argv);
}

int execvpe(const char *file, char *const argv[], char *const envp[]) {
    report_call(file, argv);
    int (*execvpe_ptr)(const char *file, char *const argv[], char *const envp[])
        = dlsym(RTLD_NEXT, "execvpe");
    return (*execvpe_ptr)(file, argv, envp);
}

