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

#include "er/Flags.h"

#include "Array.h"
#include "Environment.h"
#include "Logger.h"
#include "Paths.h"
#include "Resolver.h"
#include "Session.h"

#include <algorithm>
#include <cerrno>
#include <climits>
#include <string_view>

#include <unistd.h>

namespace {

    constexpr char DIR_SEPARATOR = '/';

    constexpr el::log::Logger LOGGER("Executor.cc");

    constexpr el::Executor::Result failure(int const error_code) noexcept
    {
        return el::Executor::Result { -1, error_code };
    }

#define CHECK_SESSION(SESSION_)                           \
    do {                                                  \
        if (!el::session::is_valid(SESSION_)) {           \
            LOGGER.warning("session is not initialized"); \
            return failure(EIO);                          \
        }                                                 \
    } while (false)

#define CHECK_POINTER(PTR_)                        \
    do {                                           \
        if (nullptr == (PTR_)) {                   \
            LOGGER.debug("null pointer received"); \
            return failure(EFAULT);                \
        }                                          \
    } while (false)

    // Util class to create command arguments to execute the intercept process.
    //
    // Use this class to allocate buffer and assemble the content of it.
    class CommandBuilder {
    public:
        constexpr CommandBuilder(const el::Session& session, const char* path, char* const* const argv)
                : session(session)
                , path(path)
                , argv(argv)
        {
        }

        [[nodiscard]] constexpr size_t length() const noexcept
        {
            return (session.verbose ? 6 : 7) + el::array::length(argv) + 1;
        }

        constexpr void assemble(const char** it) const noexcept
        {
            const char** const it_end = it + length();

            *it++ = session.reporter;
            *it++ = er::flags::DESTINATION;
            *it++ = session.destination;
            if (session.verbose) {
                *it++ = er::flags::VERBOSE;
            }
            *it++ = er::flags::EXECUTE;
            *it++ = path;
            *it++ = er::flags::COMMAND;
            {
                char* const* const argv_end = el::array::end(argv);
                it = el::array::copy(argv, argv_end, it, it_end);
            }
            *it = nullptr;
        }

        [[nodiscard]] constexpr const char* file() const noexcept
        {
            return session.reporter;
        }

    private:
        const el::Session& session;
        const char* path;
        char* const* const argv;
    };

    class PathResolver {
    public:
        struct Result {
            const char* return_value;
            const int error_code;

            constexpr explicit operator bool() const noexcept {
                return (return_value != nullptr) && (error_code == 0);
            }
        };

    public:
        explicit PathResolver(el::Resolver const &resolver);

        Result from_current_directory(std::string_view const &file);
        Result from_path(std::string_view const &file, char* const* envp);
        Result from_search_path(std::string_view const &file, const char *search_path);

        PathResolver(PathResolver const &) = delete;
        PathResolver(PathResolver &&) noexcept = delete;

        PathResolver &operator=(PathResolver const &) = delete;
        PathResolver &&operator=(PathResolver &&) noexcept = delete;

    private:
        static bool contains_dir_separator(std::string_view const &candidate);

    private:
        el::Resolver const &resolver_;
        char result_[PATH_MAX];
    };

    PathResolver::PathResolver(const el::Resolver &resolver)
            : resolver_(resolver)
            , result_()
    { }

    PathResolver::Result PathResolver::from_current_directory(std::string_view const &file) {
        // create absolute path to the given file.
        if (nullptr == resolver_.realpath(file.begin(), result_)) {
            return PathResolver::Result {nullptr, ENOENT };
        }
        // check if it's okay to execute.
        if (0 == resolver_.access(result_, X_OK)) {
            return PathResolver::Result { result_, 0 };
        }
        // try to set a meaningful error value.
        if (0 == resolver_.access(result_, F_OK)) {
            return PathResolver::Result {nullptr, EACCES };
        }
        return PathResolver::Result {nullptr, ENOENT };
    }

    PathResolver::Result PathResolver::from_path(std::string_view const &file, char* const* envp) {
        if (contains_dir_separator(file)) {
            // the file contains a dir separator, it is treated as path.
            return from_current_directory(file);
        } else {
            // otherwise use the PATH variable to locate the executable.
            const char *paths = el::env::get_env_value(const_cast<const char **>(envp), "PATH");
            if (paths != nullptr) {
                return from_search_path(file, paths);
            }
            // fall back to `confstr` PATH value if the environment has no value.
            const size_t search_path_length = resolver_.confstr(_CS_PATH, nullptr, 0);
            if (search_path_length != 0) {
                char search_path[search_path_length];
                if (resolver_.confstr(_CS_PATH, search_path, search_path_length) != 0) {
                    return from_search_path(file, search_path);
                }
            }
            return PathResolver::Result {nullptr, ENOENT };
        }
    }

    PathResolver::Result PathResolver::from_search_path(std::string_view const &file, const char *search_path) {
         if (contains_dir_separator(file)) {
            // the file contains a dir separator, it is treated as path.
            return from_current_directory(file);
        } else {
             // otherwise use the given search path to locate the executable.
             for (auto path : el::Paths(search_path)) {
                 // ignore empty entries
                 if (path.empty()) {
                     continue;
                 }
                 // create a path
                 char candidate[PATH_MAX];
                 {
                     auto it = el::array::copy(path.begin(), path.end(), candidate, candidate + PATH_MAX);
                     *it++ = DIR_SEPARATOR;
                     it = el::array::copy(file.begin(), file.end(), it, candidate + PATH_MAX);
                     *it = 0;
                 }
                 // check if it's okay to execute.
                 if (auto result = from_current_directory(candidate); result) {
                     return result;
                 }
             }
             // if all attempt were failing, then quit with a failure.
             return PathResolver::Result {nullptr, ENOENT };
        }
    }

    bool PathResolver::contains_dir_separator(std::string_view const &candidate) {
        return std::find(candidate.begin(), candidate.end(), DIR_SEPARATOR) != candidate.end();
    }
}

#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wvla"

namespace el {

    Executor::Executor(el::Resolver const& resolver, el::Session const& session) noexcept
            : resolver_(resolver)
            , session_(session)
    { }

    Executor::Result Executor::execve(const char* path, char* const* argv, char* const* envp) const
    {
        CHECK_SESSION(session_);
        CHECK_POINTER(path);

        PathResolver resolver(resolver_);
        if (auto executable = resolver.from_current_directory(path); executable) {
            const CommandBuilder cmd(session_, executable.return_value, argv);
            const char* dst[cmd.length()];
            cmd.assemble(dst);

            auto return_value = resolver_.execve(cmd.file(), const_cast<char* const*>(dst), envp);
            return Executor::Result { return_value, resolver_.error_code() };
        } else {
            return Executor::Result { -1, executable.error_code };
        }
    }

    Executor::Result Executor::execvpe(const char* file, char* const* argv, char* const* envp) const
    {
        CHECK_SESSION(session_);
        CHECK_POINTER(file);

        PathResolver resolver(resolver_);
        if (auto executable = resolver.from_path(file, envp); executable) {
            const CommandBuilder cmd(session_, executable.return_value, argv);
            const char* dst[cmd.length()];
            cmd.assemble(dst);

            auto return_value = resolver_.execve(cmd.file(), const_cast<char* const*>(dst), envp);
            return Executor::Result { return_value, resolver_.error_code() };
        } else {
            return Executor::Result { -1, executable.error_code };
        }
    }

    Executor::Result Executor::execvP(const char* file, const char* search_path, char* const* argv, char* const* envp) const
    {
        CHECK_SESSION(session_);
        CHECK_POINTER(file);

        PathResolver resolver(resolver_);
        if (auto executable = resolver.from_search_path(file, search_path); executable) {
            const CommandBuilder cmd(session_, executable.return_value, argv);
            const char* dst[cmd.length()];
            cmd.assemble(dst);

            auto return_value = resolver_.execve(cmd.file(), const_cast<char* const*>(dst), envp);
            return Executor::Result { return_value, resolver_.error_code() };
        } else {
            return Executor::Result { -1, executable.error_code };
        }
    }

    Executor::Result Executor::posix_spawn(pid_t* pid, const char* path, const posix_spawn_file_actions_t* file_actions,
        const posix_spawnattr_t* attrp, char* const* argv,
        char* const* envp) const
    {
        CHECK_SESSION(session_);
        CHECK_POINTER(path);

        PathResolver resolver(resolver_);
        if (auto executable = resolver.from_current_directory(path); executable) {
            const CommandBuilder cmd(session_, executable.return_value, argv);
            const char* dst[cmd.length()];
            cmd.assemble(dst);

            auto return_value = resolver_.posix_spawn(pid, cmd.file(), file_actions, attrp, const_cast<char* const*>(dst), envp);
            return Executor::Result { return_value, resolver_.error_code() };
        } else {
            return Executor::Result { -1, executable.error_code };
        }
    }

    Executor::Result Executor::posix_spawnp(pid_t* pid, const char* file, const posix_spawn_file_actions_t* file_actions,
        const posix_spawnattr_t* attrp, char* const* argv,
        char* const* envp) const
    {
        CHECK_SESSION(session_);
        CHECK_POINTER(file);

        PathResolver resolver(resolver_);
        if (auto executable = resolver.from_path(file, envp); executable) {
            const CommandBuilder cmd(session_, executable.return_value, argv);
            const char* dst[cmd.length()];
            cmd.assemble(dst);

            auto return_value = resolver_.posix_spawn(pid, cmd.file(), file_actions, attrp, const_cast<char* const*>(dst), envp);
            return Executor::Result { return_value, resolver_.error_code() };
        } else {
            return Executor::Result { -1, executable.error_code };
        }
    }
}

#pragma GCC diagnostic pop
