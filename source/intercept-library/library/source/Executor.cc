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

    const ear::log::Logger LOGGER("Executor.cc");

    class CommandBuilder {
    public:
        constexpr CommandBuilder(const ear::Session& session, const char* path, char* const* const argv)
                : session(session)
                , path(path)
                , argv(argv)
        {
        }

        constexpr size_t length() const noexcept
        {
            return (session.verbose ? 5 : 6) + ear::array::length(argv) + 4;
        }

        constexpr void assemble(const char** it) const noexcept
        {
            const char** const it_end = it + length();

            *it++ = session.reporter;
            *it++ = pear::flag::DESTINATION;
            *it++ = session.destination;
            *it++ = pear::flag::LIBRARY;
            *it++ = session.library;
            if (session.verbose) {
                *it++ = pear::flag::VERBOSE;
            }
            *it++ = pear::flag::PATH;
            *it++ = path;
            *it++ = pear::flag::COMMAND;
            {
                char* const* const argv_end = ear::array::end(argv);
                it = ear::array::copy(argv, argv_end, it, it_end);
            }
            *it = nullptr;
        }

        constexpr const char* file() const noexcept
        {
            return session.reporter;
        }

    private:
        const ear::Session& session;
        const char* path;
        char* const* const argv;
    };

    class StringView {
    public:
        constexpr explicit StringView(const char* ptr) noexcept
                : begin(ptr)
                , end(ear::array::end(ptr))
        {
        }

        constexpr StringView(const char* begin, const char* end) noexcept
                : begin(begin)
                , end(end)
        {
        }

        constexpr size_t length() const noexcept
        {
            return (end - begin);
        }

        constexpr bool empty() const noexcept
        {
            return 0 == length();
        }

        const char* begin;
        const char* end;
    };

    class PathBuilder {
    public:
        constexpr PathBuilder(const StringView& prefix, const StringView& file)
                : prefix(prefix)
                , file(file)
        {
        }

        constexpr size_t length() const noexcept
        {
            return prefix.length() + file.length() + 2;
        }

        constexpr void assemble(char* it) const noexcept
        {
            char* end = it + length();

            it = ear::array::copy(prefix.begin, prefix.end, it, end);
            *it++ = DIR_SEPARATOR;
            it = ear::array::copy(file.begin, file.end, it, end);
            *it = 0;
        }

    private:
        const StringView prefix;
        const StringView file;
    };

    constexpr const char* next_path_separator(const char* input)
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
            const StringView prefix(current, next);
            // ignore empty entries
            if (prefix.empty()) {
                continue;
            }
            // create a path
            const PathBuilder path_builder(prefix, StringView(file));
            char path[path_builder.length()];
            path_builder.assemble(path);
            // check if path points to an executable.
            if (0 == resolver.access(path, X_OK)) {
                // execute the wrapper
                return function(path);
            }
            LOGGER.debug("access failed for: path=", path);
            // try the next one
            current = (*next == 0) ? nullptr : ++next;
        } while (current != nullptr);
        // if all attempt were failing, then quit with a failure.
        return FAILURE;
    }

#define CHECK_POINTER(SESSION_, RESOLVER_, PTR_)   \
    do {                                           \
        if (nullptr == PTR_) {                     \
            LOGGER.debug("null pointer received"); \
            errno = ENOENT;                        \
            return FAILURE;                        \
        }                                          \
    } while (false)

#define CHECK_PATH(SESSION_, RESOLVER_, PATH_)               \
    do {                                                     \
        if (0 != RESOLVER_.access(PATH_, X_OK)) {            \
            LOGGER.debug("access failed for: path=", PATH_); \
            errno = ENOEXEC;                                 \
            return -1;                                       \
        }                                                    \
    } while (false)

#define CHECK_SESSION(SESSION_, RESOLVER_)              \
    do {                                                \
        if (!ear::session::is_valid(SESSION_)) {        \
            LOGGER.debug("session is not initialized"); \
            return -1;                                  \
        }                                               \
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

        const CommandBuilder cmd(session_, path, argv);
        const char* dst[cmd.length()];
        cmd.assemble(dst);

        return resolver_.execve(cmd.file(), const_cast<char* const*>(dst), envp);
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

        const CommandBuilder cmd(session_, path, argv);
        const char* dst[cmd.length()];
        cmd.assemble(dst);

        return resolver_.posix_spawn(pid, cmd.file(), file_actions, attrp, const_cast<char* const*>(dst), envp);
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
            const CommandBuilder cmd(ctx.session, path, ctx.argv);
            const char* dst[cmd.length()];
            cmd.assemble(dst);

            return (ctx.resolver).execve(cmd.file(), const_cast<char* const*>(dst), ctx.envp);
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
            const CommandBuilder cmd(ctx.session, path, ctx.argv);
            const char* dst[cmd.length()];
            cmd.assemble(dst);

            return (ctx.resolver).posix_spawn(ctx.pid, cmd.file(), ctx.file_actions, ctx.attrp, const_cast<char* const*>(dst), ctx.envp);
        };

        return execute_from_search_path(resolver_, session_, search_path, file, fp);
    }
}
