/*  Copyright (C) 2012-2022 by László Nagy
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

#include <atomic>
#include <cerrno>
#include <climits>
#include <cstdarg>

#include "report/libexec/Executor.h"
#include "report/libexec/Linker.h"
#include "report/libexec/Logger.h"
#include "report/libexec/Resolver.h"
#include "report/libexec/Session.h"

#ifdef HAVE_SPAWN_H
#include <spawn.h>
#endif
#if defined HAVE_NSGETENVIRON
#include <crt_externs.h>
#else
extern char **environ;
#endif

namespace {

    size_t va_length(va_list& args)
    {
        size_t arg_count = 0;
        while (va_arg(args, const char*) != nullptr)
            ++arg_count;
        return arg_count;
    }

    void va_copy_n(va_list& args, char* argv[], size_t const argc)
    {
        for (size_t idx = 0; idx < argc; ++idx)
            argv[idx] = va_arg(args, char*);
    }

    /**
     * Abstraction to get the current environment.
     *
     * When the dynamic linker loads the library the `environ` variable
     * might not be available. (This is the case for OSX.) This method
     * makes it uniform to access the current environment on all platform.
     *
     * @return the current environment.
     */
    const char** environment() noexcept
    {
#ifdef HAVE_NSGETENVIRON
        return const_cast<const char**>(*_NSGetEnviron());
#else
        return const_cast<const char**>(environ);
#endif
    }
}

/**
 * Library static data
 *
 * Will be initialized, when the library loaded into memory.
 */
namespace {
    // This is the only non stack memory that this library is using.
    constexpr size_t BUFFER_SIZE = PATH_MAX * 2;
    char BUFFER[BUFFER_SIZE];
    // This is used for being multi thread safe (loading time only).
    std::atomic<bool> LOADED(false);
    // These are related to the functionality of this library.
    el::Session SESSION;
    el::Linker LINKER;

    constexpr el::log::Logger LOGGER("lib.cc");
}

/**
 * Library entry point.
 *
 * The first method to call after the library is loaded into memory.
 */
extern "C" void on_load() __attribute__((constructor));
extern "C" void on_load()
{
    // Test whether on_load was called already.
    if (LOADED.exchange(true))
        return;

    el::session::from(SESSION, environment());
    el::session::persist(SESSION, BUFFER, BUFFER + BUFFER_SIZE);

    el::log::Level level = SESSION.verbose ? el::log::VERBOSE : el::log::SILENT;
    el::log::set(level);

    LOGGER.debug("on_load");
    errno = 0;
}

/**
 * Library exit point.
 *
 * The last method which needs to be called when the library is unloaded.
 */
extern "C" void on_unload() __attribute__((destructor));
extern "C" void on_unload()
{
    // Test whether on_unload was called already.
    if (not LOADED.exchange(false))
        return;

    LOGGER.debug("on_unload");
}

extern "C" int execve(const char* path, char* const argv[], char* const envp[])
{
    LOGGER.debug("execve path: ", path);

    el::Resolver resolver;
    const auto result = el::Executor(LINKER, SESSION, resolver).execve(path, argv, envp);
    if (result.is_err()) {
        LOGGER.debug("execve failed.");
        errno = result.unwrap_err();
    }
    return result.unwrap_or(-1);
}

extern "C" int execv(const char* path, char* const argv[])
{
    LOGGER.debug("execv path: ", path);

    auto envp = const_cast<char* const*>(environment());
    el::Resolver resolver;
    const auto result = el::Executor(LINKER, SESSION, resolver).execve(path, argv, envp);
    if (result.is_err()) {
        LOGGER.debug("execv failed.");
        errno = result.unwrap_err();
    }
    return result.unwrap_or(-1);
}

extern "C" int execvpe(const char* file, char* const argv[], char* const envp[])
{
    LOGGER.debug("execvpe file: ", file);

    el::Resolver resolver;
    const auto result = el::Executor(LINKER, SESSION, resolver).execvpe(file, argv, envp);
    if (result.is_err()) {
        LOGGER.debug("execvpe failed.");
        errno = result.unwrap_err();
    }
    return result.unwrap_or(-1);
}

extern "C" int execvp(const char* file, char* const argv[])
{
    LOGGER.debug("execvp file: ", file);

    auto envp = const_cast<char* const*>(environment());
    el::Resolver resolver;
    const auto result = el::Executor(LINKER, SESSION, resolver).execvpe(file, argv, envp);
    if (result.is_err()) {
        LOGGER.debug("execvp failed.");
        errno = result.unwrap_err();
    }
    return result.unwrap_or(-1);
}

extern "C" int execvP(const char* file, const char* search_path, char* const argv[])
{
    LOGGER.debug("execvP file: ", file);

    auto envp = const_cast<char* const*>(environment());
    el::Resolver resolver;
    const auto result = el::Executor(LINKER, SESSION, resolver).execvP(file, search_path, argv, envp);
    if (result.is_err()) {
        LOGGER.debug("execvP failed.");
        errno = result.unwrap_err();
    }
    return result.unwrap_or(-1);
}

extern "C" int exect(const char* path, char* const argv[], char* const envp[])
{
    LOGGER.debug("exect path: ", path);

    el::Resolver resolver;
    const auto result = el::Executor(LINKER, SESSION, resolver).execve(path, argv, envp);
    if (result.is_err()) {
        LOGGER.debug("exect failed.");
        errno = result.unwrap_err();
    }
    return result.unwrap_or(-1);
}

#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wvla"

extern "C" int execl(const char* path, const char* arg, ...)
{
    LOGGER.debug("execl path: ", path);

    // Count the number of arguments.
    va_list ap;
    va_start(ap, arg);
    const size_t argc = va_length(ap);
    va_end(ap);
    // Copy the arguments to the stack.
    va_start(ap, arg);
    char* argv[argc + 2];
    argv[0] = const_cast<char*>(path);
    va_copy_n(ap, &argv[1], argc + 1);
    va_end(ap);

    auto envp = const_cast<char* const*>(environment());
    el::Resolver resolver;
    const auto result = el::Executor(LINKER, SESSION, resolver).execve(path, argv, envp);
    if (result.is_err()) {
        LOGGER.debug("execl failed.");
        errno = result.unwrap_err();
    }
    return result.unwrap_or(-1);
}

extern "C" int execlp(const char* file, const char* arg, ...)
{
    LOGGER.debug("execlp file: ", file);

    // Count the number of arguments.
    va_list ap;
    va_start(ap, arg);
    const size_t argc = va_length(ap);
    va_end(ap);
    // Copy the arguments to the stack.
    va_start(ap, arg);
    char* argv[argc + 2];
    argv[0] = const_cast<char*>(file);
    va_copy_n(ap, &argv[1], argc + 1);
    va_end(ap);

    auto envp = const_cast<char* const*>(environment());
    el::Resolver resolver;
    const auto result = el::Executor(LINKER, SESSION, resolver).execvpe(file, argv, envp);
    if (result.is_err()) {
        LOGGER.debug("execlp failed.");
        errno = result.unwrap_err();
    }
    return result.unwrap_or(-1);
}

// int execle(const char *path, const char *arg, ..., char * const envp[]);
extern "C" int execle(const char* path, const char* arg, ...)
{
    LOGGER.debug("execle path: ", path);

    // Count the number of arguments.
    va_list ap;
    va_start(ap, arg);
    const size_t argc = va_length(ap);
    va_end(ap);
    // Copy the arguments to the stack.
    va_start(ap, arg);
    char* argv[argc + 2];
    argv[0] = const_cast<char*>(path);
    va_copy_n(ap, &argv[1], argc + 1);
    char** envp = va_arg(ap, char**);
    va_end(ap);

    el::Resolver resolver;
    const auto result = el::Executor(LINKER, SESSION, resolver).execve(path, argv, envp);
    if (result.is_err()) {
        LOGGER.debug("execle failed.");
        errno = result.unwrap_err();
    }
    return result.unwrap_or(-1);
}

#pragma GCC diagnostic pop

extern "C" int posix_spawn(pid_t* pid, const char* path,
    const posix_spawn_file_actions_t* file_actions,
    const posix_spawnattr_t* attrp,
    char* const argv[], char* const envp[])
{
    LOGGER.debug("posix_spawn path:", path);

    el::Resolver resolver;
    const auto result = el::Executor(LINKER, SESSION, resolver).posix_spawn(pid, path, file_actions, attrp, argv, envp);
    if (result.is_err()) {
        LOGGER.debug("posix_spawn failed.");
        errno = result.unwrap_err();
    }
    return result.unwrap_or(-1);
}

extern "C" int posix_spawnp(pid_t* pid, const char* file,
    const posix_spawn_file_actions_t* file_actions,
    const posix_spawnattr_t* attrp,
    char* const argv[], char* const envp[])
{
    LOGGER.debug("posix_spawnp file:", file);

    el::Resolver resolver;
    const auto result = el::Executor(LINKER, SESSION, resolver).posix_spawnp(pid, file, file_actions, attrp, argv, envp);
    if (result.is_err()) {
        LOGGER.debug("posix_spawnp failed.");
        errno = result.unwrap_err();
    }
    return result.unwrap_or(-1);
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
