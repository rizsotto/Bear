/*  Copyright (C) 2012-2017 by László Nagy
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

#include "libear_a/Array.h"
#include "libear_a/DynamicLinker.h"
#include "libear_a/String.h"
#include "libear_a/Session.h"
#include "libear_a/Environment.h"
#include "libear_a/Storage.h"
#include "libear_a/Executor.h"


namespace {
    using DynamicLinkerExecutor = ::ear::Executor<::ear::DynamicLinker>;

    std::atomic<bool> loaded = false;

    constexpr size_t buffer_size = 16 * 1024;
    char buffer[buffer_size];
    ::ear::Storage storage(buffer, buffer + buffer_size);
    ::ear::LibrarySession session;

    ::ear::LibrarySession const *session_ptr;
}

/**
 * Library entry point.
 *
 * The first method to call after the library is loaded into memory.
 */
extern "C" void on_load() __attribute__((constructor));
extern "C" void on_load() {
    // Test whether on_load was called already.
    if (loaded.exchange(true))
        return;

    auto environment = ::ear::environment::current();
    session_ptr = ::ear::environment::capture(session, storage, environment);
}

/**
 * Library exit point.
 *
 * The last method which needs to be called when the library is unloaded.
 */
extern "C" void on_unload() __attribute__((destructor));
extern "C" void on_unload() {
    // Test whether on_unload was called already.
    if (not loaded.exchange(false))
        return;

    if (session_ptr != nullptr)
        session_ptr = nullptr;
}


extern "C"
int execve(const char *path, char *const argv[], char *const envp[]) {
    return DynamicLinkerExecutor(session_ptr).execve(path, argv, envp);
}


extern "C"
int execv(const char *path, char *const argv[]) {
    auto envp = const_cast<char *const *>(::ear::environment::current());
    return DynamicLinkerExecutor(session_ptr).execve(path, argv, envp);
}


extern "C"
int execvpe(const char *file, char *const argv[], char *const envp[]) {
    return DynamicLinkerExecutor(session_ptr).execvpe(file, argv, envp);
}


extern "C"
int execvp(const char *file, char *const argv[]) {
    auto envp = const_cast<char *const *>(::ear::environment::current());
    return DynamicLinkerExecutor(session_ptr).execvpe(file, argv, envp);
}


extern "C"
int execvP(const char *file, const char *search_path, char *const argv[]) {
    auto envp = const_cast<char *const *>(::ear::environment::current());
    return DynamicLinkerExecutor(session_ptr).execvP(file, search_path, argv, envp);
}


extern "C"
int exect(const char *path, char *const argv[], char *const envp[]) {
    return DynamicLinkerExecutor(session_ptr).exect(path, argv, envp);
}


namespace {
    constexpr auto va_length = [](va_list &args) -> size_t {
        size_t arg_count = 0;
        while (va_arg(args, const char *) != nullptr)
            ++arg_count;
        return arg_count;
    };

    constexpr auto va_copy_n =
            [](va_list &args, char *argv[], size_t const argc) -> void {
        for (size_t idx = 0; idx <= argc; ++idx)
            argv[idx] = va_arg(args, char *);
    };
}


extern "C"
int execl(const char *path, const char *arg, ...) {
    // Count the number of arguments.
    va_list ap;
    va_start(ap, arg);
    const size_t argc = va_length(ap);
    va_end(ap);
    // Copy the arguments to the stack.
    va_start(ap, arg);
    char *argv[argc + 1];
    va_copy_n(ap, argv, argc);
    va_end(ap);

    auto envp = const_cast<char *const *>(::ear::environment::current());
    return DynamicLinkerExecutor(session_ptr).execve(path, argv, envp);
}


extern "C"
int execlp(const char *file, const char *arg, ...) {
    // Count the number of arguments.
    va_list ap;
    va_start(ap, arg);
    const size_t argc = va_length(ap);
    va_end(ap);
    // Copy the arguments to the stack.
    va_start(ap, arg);
    char *argv[argc + 1];
    va_copy_n(ap, argv, argc);
    va_end(ap);

    auto envp = const_cast<char *const *>(::ear::environment::current());
    return DynamicLinkerExecutor(session_ptr).execvpe(file, argv, envp);
}


// int execle(const char *path, const char *arg, ..., char * const envp[]);
extern "C"
int execle(const char *path, const char *arg, ...) {
    // Count the number of arguments.
    va_list ap;
    va_start(ap, arg);
    const size_t argc = va_length(ap);
    va_end(ap);
    // Copy the arguments to the stack.
    va_start(ap, arg);
    char *argv[argc + 1];
    va_copy_n(ap, argv, argc);
    char **envp = va_arg(ap, char **);
    va_end(ap);

    return DynamicLinkerExecutor(session_ptr).execve(path, argv, envp);
}


extern "C"
int posix_spawn(pid_t *pid, const char *path,
                const posix_spawn_file_actions_t *file_actions,
                const posix_spawnattr_t *attrp,
                char *const argv[], char *const envp[]) {
    return DynamicLinkerExecutor(session_ptr).posix_spawn(pid, path, file_actions, attrp, argv, envp);
}


extern "C"
int posix_spawnp(pid_t *pid, const char *file,
                 const posix_spawn_file_actions_t *file_actions,
                 const posix_spawnattr_t *attrp,
                 char *const argv[], char *const envp[]) {
    return DynamicLinkerExecutor(session_ptr).posix_spawnp(pid, file, file_actions, attrp, argv, envp);
}
