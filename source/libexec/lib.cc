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

#include "config.h"

#include <cstdio>
#include <cstdlib>
#include <cstdarg>
#include <atomic>

#include "libexec_a/Array.h"
#include "libexec_a/DynamicLinker.h"
#include "libexec_a/Session.h"
#include "libexec_a/Environment.h"
#include "libexec_a/Storage.h"
#include "libexec_a/Executor.h"


namespace {

    std::atomic<bool> LOADED(false);

    constexpr size_t BUFFER_SIZE = 16 * 1024;
    char BUFFER[BUFFER_SIZE];

    ::ear::Session SESSION;


    bool is_not_valid(const ::ear::Session &input) noexcept {
        return (input.library == nullptr ||
                input.reporter == nullptr ||
                input.destination == nullptr);
    }

    ::ear::Session store_session_attributes(const ::ear::Session &input) noexcept {
        if (is_not_valid(input))
            return input;

        ::ear::Storage storage(BUFFER, BUFFER + BUFFER_SIZE);

        return ::ear::Session {
                storage.store(input.library),
                storage.store(input.reporter),
                storage.store(input.destination),
                input.verbose
        };
    }

    void trace_function_call(const char *message) {
        if (is_not_valid(SESSION))
            fprintf(stderr, "libexec.so: not initialized. Failed to execute: %s\n", message);
        else if (SESSION.verbose)
            fprintf(stderr, "libexec.so: %s\n", message);
    }


    using DynamicLinkerExecutor = ::ear::Executor<::ear::DynamicLinker>;
}

/**
 * Library entry point.
 *
 * The first method to call after the library is loaded into memory.
 */
extern "C" void on_load() __attribute__((constructor));
extern "C" void on_load() {
    // Test whether on_load was called already.
    if (LOADED.exchange(true))
        return;

    const auto environment = ::ear::environment::current();
    const auto session = ::ear::environment::capture_session(environment);
    SESSION = store_session_attributes(session);

    trace_function_call("on_load");
}

/**
 * Library exit point.
 *
 * The last method which needs to be called when the library is unloaded.
 */
extern "C" void on_unload() __attribute__((destructor));
extern "C" void on_unload() {
    // Test whether on_unload was called already.
    if (not LOADED.exchange(false))
        return;

    trace_function_call("on_unload");
}


extern "C"
int execve(const char *path, char *const argv[], char *const envp[]) {
    trace_function_call("execve");

    return is_not_valid(SESSION)
            ? -1
            : DynamicLinkerExecutor(SESSION).execve(path, argv, envp);
}


extern "C"
int execv(const char *path, char *const argv[]) {
    trace_function_call("execv");

    auto envp = const_cast<char *const *>(::ear::environment::current());
    return is_not_valid(SESSION)
            ? -1
            : DynamicLinkerExecutor(SESSION).execve(path, argv, envp);
}


extern "C"
int execvpe(const char *file, char *const argv[], char *const envp[]) {
    trace_function_call("execvpe");

    return is_not_valid(SESSION)
            ? -1
            : DynamicLinkerExecutor(SESSION).execvpe(file, argv, envp);
}


extern "C"
int execvp(const char *file, char *const argv[]) {
    trace_function_call("execvp");

    auto envp = const_cast<char *const *>(::ear::environment::current());
    return is_not_valid(SESSION)
            ? -1
            : DynamicLinkerExecutor(SESSION).execvpe(file, argv, envp);
}


extern "C"
int execvP(const char *file, const char *search_path, char *const argv[]) {
    trace_function_call("execvP");

    auto envp = const_cast<char *const *>(::ear::environment::current());
    return is_not_valid(SESSION)
            ? -1
            : DynamicLinkerExecutor(SESSION).execvP(file, search_path, argv, envp);
}


extern "C"
int exect(const char *path, char *const argv[], char *const envp[]) {
    trace_function_call("exect");

    return is_not_valid(SESSION)
            ? -1
            : DynamicLinkerExecutor(SESSION).execve(path, argv, envp);
}


namespace {
    size_t va_length(va_list &args) {
        size_t arg_count = 0;
        while (va_arg(args, const char *) != nullptr)
            ++arg_count;
        return arg_count;
    };

    void va_copy_n(va_list &args, char *argv[], size_t const argc) {
        for (size_t idx = 0; idx <= argc; ++idx)
            argv[idx] = va_arg(args, char *);
    };
}


extern "C"
int execl(const char *path, const char *arg, ...) {
    trace_function_call("execl");

    // Count the number of arguments.
    va_list ap;
    va_start(ap, arg);
    const size_t argc = va_length(ap);
    va_end(ap);
    // Copy the arguments to the stack.
    va_start(ap, arg);
    char *argv[argc + 2];
    argv[0] = const_cast<char *>(path);
    va_copy_n(ap, &argv[1], argc);
    va_end(ap);

    auto envp = const_cast<char *const *>(::ear::environment::current());
    return is_not_valid(SESSION)
            ? -1
            : DynamicLinkerExecutor(SESSION).execve(path, argv, envp);
}


extern "C"
int execlp(const char *file, const char *arg, ...) {
    trace_function_call("execlp");

    // Count the number of arguments.
    va_list ap;
    va_start(ap, arg);
    const size_t argc = va_length(ap);
    va_end(ap);
    // Copy the arguments to the stack.
    va_start(ap, arg);
    char *argv[argc + 2];
    argv[0] = const_cast<char *>(file);
    va_copy_n(ap, &argv[1], argc);
    va_end(ap);

    auto envp = const_cast<char *const *>(::ear::environment::current());
    return is_not_valid(SESSION)
            ? -1
            : DynamicLinkerExecutor(SESSION).execvpe(file, argv, envp);
}


// int execle(const char *path, const char *arg, ..., char * const envp[]);
extern "C"
int execle(const char *path, const char *arg, ...) {
    trace_function_call("execle");

    // Count the number of arguments.
    va_list ap;
    va_start(ap, arg);
    const size_t argc = va_length(ap);
    va_end(ap);
    // Copy the arguments to the stack.
    va_start(ap, arg);
    char *argv[argc + 2];
    argv[0] = const_cast<char *>(path);
    va_copy_n(ap, &argv[1], argc);
    char **envp = va_arg(ap, char **);
    va_end(ap);

    return is_not_valid(SESSION)
            ? -1
            : DynamicLinkerExecutor(SESSION).execve(path, argv, envp);
}


extern "C"
int posix_spawn(pid_t *pid, const char *path,
                const posix_spawn_file_actions_t *file_actions,
                const posix_spawnattr_t *attrp,
                char *const argv[], char *const envp[]) {
    trace_function_call("posix_spawn");

    return is_not_valid(SESSION)
            ? -1
            : DynamicLinkerExecutor(SESSION).posix_spawn(pid, path, file_actions, attrp, argv, envp);
}


extern "C"
int posix_spawnp(pid_t *pid, const char *file,
                 const posix_spawn_file_actions_t *file_actions,
                 const posix_spawnattr_t *attrp,
                 char *const argv[], char *const envp[]) {
    trace_function_call("posix_spawnp");

    return is_not_valid(SESSION)
            ? -1
            : DynamicLinkerExecutor(SESSION).posix_spawnp(pid, file, file_actions, attrp, argv, envp);
}

//extern "C"
//FILE *popen(const char *command, const char *type) {
//}

//extern "C"
//int execveat(int dirfd,
//             const char *pathname,
//             char *const argv[],
//             char *const envp[],
//             int flags) {
//}

//extern "C"
//int fexecve(int fd, char *const argv[], char *const envp[]) {
//}
