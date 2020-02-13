/*  Copyright (C) 2012-2020 by László Nagy
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

#include "Executor.h"

#include "intercept.h"

#include "Array.h"
#include "Environment.h"
#include "Logger.h"
#include "Resolver.h"
#include "Session.h"

#include <cerrno>
#include <functional>
#include <unistd.h>

namespace {

    constexpr int FAILURE = -1;

    constexpr char PATH_SEPARATOR = ':';
    constexpr char DIR_SEPARATOR = '/';

    struct Execution {
        const char** command;
        const char* path;
    };

    size_t length(Execution const& execution) noexcept
    {
        return ear::array::length(execution.command) + 4;
    }

    const char** copy(Execution const& execution, const char** it, const char** it_end) noexcept
    {
        *it++ = pear::flag::PATH;
        *it++ = execution.path;
        *it++ = pear::flag::COMMAND;
        const size_t command_size = ear::array::length(execution.command);
        const char** const command_end = execution.command + (command_size + 1);
        return ear::array::copy(execution.command, command_end, it, it_end);
    }

    size_t length(ear::Session const& session) noexcept
    {
        return session.verbose ? 5 : 6;
    }

    const char** copy(ear::Session const& session, const char** it, const char** it_end) noexcept
    {
        *it++ = session.reporter;
        *it++ = pear::flag::DESTINATION;
        *it++ = session.destination;
        *it++ = pear::flag::LIBRARY;
        *it++ = session.library;
        if (session.verbose)
            *it++ = pear::flag::VERBOSE;
        return it;
    }

#define CREATE_BUFFER(VAR_, SESSION_, EXECUTION_)                           \
    const size_t VAR_##_length = length(EXECUTION_) + length(SESSION_) + 1; \
    const char* VAR_[VAR_##_length];                                        \
    {                                                                       \
        const char** const VAR_##_end = VAR_ + VAR_##_length;               \
        const char** VAR_##it = copy(SESSION_, VAR_, VAR_##_end);           \
        copy(EXECUTION_, VAR_##it, VAR_##_end);                             \
    }

    const char* next_path_separator(const char* input)
    {
        auto it = input;
        while ((*it != 0) && (*it != PATH_SEPARATOR)) {
            ++it;
        }
        return it;
    }

    int execute_from_search_path(
        const ear::Resolver& resolver,
        const ear::Session& session,
        const char* search_path,
        const char* file,
        std::function<int(const char*)> const& function) noexcept
    {
        const char* current = search_path;
        do {
            const char* next = next_path_separator(current);
            // ignore empty entries
            if (current - next == 0) {
                continue;
            }
            // create a path
            const size_t path_length = ear::array::length(file) + (next - current) + 2;
            char path[path_length];
            {
                char* path_it = path;
                char* const path_end = path + path_length;
                path_it = ear::array::copy(current, next, path_it, path_end);
                *path_it++ = DIR_SEPARATOR;
                path_it = ear::array::copy(file, ear::array::end(file), path_it, path_end);
                *path_it = 0;
            }
            // check if path points to an executable.
            if (0 == resolver.access(path, X_OK)) {
                // execute the wrapper
                return function(path);
            }
            ear::Logger(resolver, session).debug("access failed for: path=", path);
            // try the next one
            current = (*next == 0) ? nullptr : ++next;
        } while (current != nullptr);
        // if all attempt were failing, then quit with a failure.
        return FAILURE;
    }

#define CHECK_POINTER(SESSION_, RESOLVER_, PTR_)                             \
    do {                                                                     \
        if (nullptr == PTR_) {                                               \
            ear::Logger(RESOLVER_, SESSION_).debug("null pointer received"); \
            errno = ENOENT;                                                  \
            return FAILURE;                                                  \
        }                                                                    \
    } while (false)

#define CHECK_PATH(SESSION_, RESOLVER_, PATH_)                                         \
    do {                                                                               \
        if (0 != RESOLVER_.access(PATH_, X_OK)) {                                      \
            ear::Logger(RESOLVER_, SESSION_).debug("access failed for: path=", PATH_); \
            errno = ENOEXEC;                                                           \
            return -1;                                                                 \
        }                                                                              \
    } while (false)

#define CHECK_SESSION(SESSION_, RESOLVER_)                                        \
    do {                                                                          \
        if (!ear::session::is_valid(SESSION_)) {                                  \
            ear::Logger(RESOLVER_, SESSION_).debug("session is not initialized"); \
            return -1;                                                            \
        }                                                                         \
    } while (false)

    bool contains_dir_separator(const char* const candidate)
    {
        for (auto it = candidate; *it != 0; ++it) {
            if (*it == DIR_SEPARATOR) {
                return true;
            }
        }
        return false;
    }
}

namespace ear {

    Executor::Executor(ear::Resolver const& resolver, ear::Session const& session) noexcept
            : resolver_(resolver)
            , session_(session)
    {
    }

    int Executor::execve(const char* path, char* const* argv, char* const* envp) const noexcept
    {
        CHECK_SESSION(session_, resolver_);
        CHECK_POINTER(session_, resolver_, path);
        CHECK_PATH(session_, resolver_, path);

        const Execution execution = { const_cast<const char**>(argv), path };
        CREATE_BUFFER(dst, session_, execution);

        return resolver_.execve(session_.reporter, const_cast<char* const*>(dst), envp);
    }

    int Executor::execvpe(const char* file, char* const* argv, char* const* envp) const noexcept
    {
        CHECK_SESSION(session_, resolver_);
        CHECK_POINTER(session_, resolver_, file);

        if (contains_dir_separator(file)) {
            // the file contains a dir separator, it is treated as path.
            return execve(file, argv, envp);
        } else {
            // otherwise use the PATH variable to locate the executable.
            const char* paths = ear::env::get_env_value(const_cast<const char**>(envp), "PATH");
            if (paths != nullptr) {
                return execve_from_search_path(paths, file, argv, envp);
            }
            // fall back to `confstr` PATH value if the environment has no value.
            const size_t search_path_length = resolver_.confstr(_CS_PATH, nullptr, 0);
            char search_path[search_path_length];
            confstr(_CS_PATH, search_path, search_path_length);

            return execve_from_search_path(search_path, file, argv, envp);
        }
    }

    int Executor::execvP(const char* file, const char* search_path, char* const* argv,
        char* const* envp) const noexcept
    {
        CHECK_SESSION(session_, resolver_);
        CHECK_POINTER(session_, resolver_, file);

        if (contains_dir_separator(file)) {
            // the file contains a dir separator, it is treated as path.
            return execve(file, argv, envp);
        } else {
            // otherwise use the given search path to locate the executable.
            return execve_from_search_path(search_path, file, argv, envp);
        }
    }

    int Executor::posix_spawn(pid_t* pid, const char* path, const posix_spawn_file_actions_t* file_actions,
        const posix_spawnattr_t* attrp, char* const* argv,
        char* const* envp) const noexcept
    {
        CHECK_SESSION(session_, resolver_);
        CHECK_POINTER(session_, resolver_, path);
        CHECK_PATH(session_, resolver_, path);

        const Execution execution = { const_cast<const char**>(argv), path };
        CREATE_BUFFER(dst, session_, execution);

        return resolver_.posix_spawn(pid, session_.reporter, file_actions, attrp, const_cast<char* const*>(dst), envp);
    }

    int Executor::posix_spawnp(pid_t* pid, const char* file, const posix_spawn_file_actions_t* file_actions,
        const posix_spawnattr_t* attrp, char* const* argv,
        char* const* envp) const noexcept
    {
        CHECK_SESSION(session_, resolver_);
        CHECK_POINTER(session_, resolver_, file);

        if (contains_dir_separator(file)) {
            // the file contains a dir separator, it is treated as path.
            return posix_spawn(pid, file, file_actions, attrp, argv, envp);
        } else {
            // otherwise use the PATH variable to locate the executable.
            const char* paths = ear::env::get_env_value(const_cast<const char**>(envp), "PATH");
            if (paths != nullptr) {
                return posix_spawn_from_search_path(paths, pid, file, file_actions, attrp, argv, envp);
            }
            // fall back to `confstr` PATH value if the environment has no value.
            const size_t search_path_length = resolver_.confstr(_CS_PATH, nullptr, 0);
            char search_path[search_path_length];
            confstr(_CS_PATH, search_path, search_path_length);

            return posix_spawn_from_search_path(search_path, pid, file, file_actions, attrp, argv, envp);
        }
    }

    int Executor::execve_from_search_path(const char* search_path, const char* file, char* const* argv, char* const* envp) const
    {
        // To avoid heap allocations with std::function
        //
        // Are you familiar with the "small string optimization" for std::string? Basically,
        // the data for a std::string is stored on the heap -- the size is unbounded, so heap
        // allocation is obviously necessary. But in real programs, the vast majority of strings
        // are actually pretty small, so the heap allocation can be avoided by storing the string
        // inside the std::string object instance (re-purposing the memory used for the pointers,
        // usually).
        //
        // The same thing is happening here with std::function. The size of the function object
        // it might be storing is unbounded, so heap allocation is the default behavior. But most
        // function objects are pretty small, so a similar "small function object optimization" is
        // possible.
        struct Context {
            const Resolver& resolver;
            const Session& session;
            char* const* argv;
            char* const* envp;
        } ctx = {
            resolver_, session_, argv, envp
        };
        // Capture context variable by reference.
        const std::function<int(const char*)> fp = [&ctx](const char* path) {
            const Execution execution = { const_cast<const char**>(ctx.argv), path };
            CREATE_BUFFER(dst, ctx.session, execution);

            return (ctx.resolver).execve((ctx.session).reporter, const_cast<char* const*>(dst), ctx.envp);
        };

        return execute_from_search_path(resolver_, session_, search_path, file, fp);
    }

    int Executor::posix_spawn_from_search_path(const char* search_path, pid_t* pid, const char* file, const posix_spawn_file_actions_t* file_actions, const posix_spawnattr_t* attrp, char* const* argv, char* const* envp) const
    {
        // See comment in `Executor::execve_from_search_path` method.
        struct Context {
            const Resolver& resolver;
            const Session& session;
            pid_t* pid;
            const posix_spawn_file_actions_t* file_actions;
            const posix_spawnattr_t* attrp;
            char* const* argv;
            char* const* envp;
        } ctx = {
            resolver_, session_, pid, file_actions, attrp, argv, envp
        };

        const std::function<int(const char*)> fp = [&ctx](const char* path) {
            const Execution execution = { const_cast<const char**>(ctx.argv), path };
            CREATE_BUFFER(dst, ctx.session, execution);

            return (ctx.resolver).posix_spawn(ctx.pid, (ctx.session).reporter, ctx.file_actions, ctx.attrp, const_cast<char* const*>(dst), ctx.envp);
        };

        return execute_from_search_path(resolver_, session_, search_path, file, fp);
    }
}
