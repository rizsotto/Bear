/*  Copyright (C) 2012-2018 by László Nagy
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

#include <unistd.h>
#include <stdio.h>
#include <stdarg.h>

#include <spawn.h>


static size_t va_length(va_list *args) {
    size_t arg_count = 0;
    while (va_arg(*args, const char *) != 0)
        ++arg_count;
    return arg_count;
}

static void va_copy_n(va_list *args, char *argv[], size_t const argc) {
    for (size_t idx = 0; idx <= argc; ++idx)
        argv[idx] = va_arg(*args, char *);
}

extern const char * hello_rust();

/**
 * Library entry point.
 *
 * The first method to call after the library is loaded into memory.
 */
void on_load() __attribute__((constructor));
void on_load() {
    hello_rust();
}

/**
 * Library exit point.
 *
 * The last method which needs to be called when the library is unloaded.
 */
void on_unload() __attribute__((destructor));
void on_unload() {
}

int execve(const char *path, char *const argv[], char *const envp[]) {
    hello_rust();
    return -1;
}


int execv(const char *path, char *const argv[]) {
    hello_rust();
    return -1;
}


int execvpe(const char *file, char *const argv[], char *const envp[]) {
    hello_rust();
    return -1;
}


int execvp(const char *file, char *const argv[]) {
    return -1;
}


int execvP(const char *file, const char *search_path, char *const argv[]) {
    return -1;
}


int exect(const char *path, char *const argv[], char *const envp[]) {
    return -1;
}


int execl(const char *path, const char *arg, ...) {
    // Count the number of arguments.
    va_list ap;
    va_start(ap, arg);
    const size_t argc = va_length(&ap);
    va_end(ap);
    // Copy the arguments to the stack.
    va_start(ap, arg);
    char *argv[argc + 2];
    argv[0] = (char *)path;
    va_copy_n(&ap, &argv[1], argc);
    va_end(ap);

    return -1;
}


int execlp(const char *file, const char *arg, ...) {
    // Count the number of arguments.
    va_list ap;
    va_start(ap, arg);
    const size_t argc = va_length(&ap);
    va_end(ap);
    // Copy the arguments to the stack.
    va_start(ap, arg);
    char *argv[argc + 2];
    argv[0] = (char *)file;
    va_copy_n(&ap, &argv[1], argc);
    va_end(ap);

    return -1;
}


// int execle(const char *path, const char *arg, ..., char * const envp[]);
int execle(const char *path, const char *arg, ...) {
    // Count the number of arguments.
    va_list ap;
    va_start(ap, arg);
    const size_t argc = va_length(&ap);
    va_end(ap);
    // Copy the arguments to the stack.
    va_start(ap, arg);
    char *argv[argc + 2];
    argv[0] = (char *)path;
    va_copy_n(&ap, &argv[1], argc);
    char **envp = va_arg(ap, char **);
    va_end(ap);

    return -1;
}


int posix_spawn(pid_t *pid, const char *path,
                const posix_spawn_file_actions_t *file_actions,
                const posix_spawnattr_t *attrp,
                char *const argv[], char *const envp[]) {
    return -1;
}



int posix_spawnp(pid_t *pid, const char *file,
                 const posix_spawn_file_actions_t *file_actions,
                 const posix_spawnattr_t *attrp,
                 char *const argv[], char *const envp[]) {
    return -1;
}

//
//FILE *popen(const char *command, const char *type) {
//}

//
//int execveat(int dirfd,
//             const char *pathname,
//             char *const argv[],
//             char *const envp[],
//             int flags) {
//}

//
//int fexecve(int fd, char *const argv[], char *const envp[]) {
//}
