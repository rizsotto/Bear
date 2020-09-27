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
#include "Logger.h"
#include "Resolver.h"
#include "Linker.h"
#include "Session.h"

#include <algorithm>
#include <cerrno>

namespace {

    constexpr el::log::Logger LOGGER("Executor.cc");

#define CHECK_SESSION(SESSION_)                           \
    do {                                                  \
        if (!el::session::is_valid(SESSION_)) {           \
            LOGGER.warning("session is not initialized"); \
            return el::Executor::Result { -1, EIO };      \
        }                                                 \
    } while (false)

#define CHECK_POINTER(PTR_)                             \
    do {                                                \
        if (nullptr == (PTR_)) {                        \
            LOGGER.debug("null pointer received");      \
            return el::Executor::Result { -1, EFAULT }; \
        }                                               \
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
        { }

        [[nodiscard]]
        constexpr size_t length() const noexcept
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

        [[nodiscard]]
        constexpr const char* file() const noexcept
        {
            return session.reporter;
        }

    private:
        const el::Session& session;
        const char* path;
        char* const* const argv;
    };
}

#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wvla"

namespace el {

    Executor::Executor(el::Linker const& linker, el::Session const& session, el::Resolver &resolver) noexcept
            : linker_(linker)
            , session_(session)
            , resolver_(resolver)
    { }

    Executor::Result Executor::execve(const char* path, char* const* argv, char* const* envp) const
    {
        CHECK_SESSION(session_);
        CHECK_POINTER(path);

        if (auto executable = resolver_.from_current_directory(path); executable) {
            const CommandBuilder cmd(session_, executable.return_value, argv);
            const char* dst[cmd.length()];
            cmd.assemble(dst);

            auto return_value = linker_.execve(cmd.file(), const_cast<char* const*>(dst), envp);
            return Executor::Result {return_value, linker_.error_code() };
        } else {
            return Executor::Result { -1, executable.error_code };
        }
    }

    Executor::Result Executor::execvpe(const char* file, char* const* argv, char* const* envp) const
    {
        CHECK_SESSION(session_);
        CHECK_POINTER(file);

        if (auto executable = resolver_.from_path(file, envp); executable) {
            const CommandBuilder cmd(session_, executable.return_value, argv);
            const char* dst[cmd.length()];
            cmd.assemble(dst);

            auto return_value = linker_.execve(cmd.file(), const_cast<char* const*>(dst), envp);
            return Executor::Result {return_value, linker_.error_code() };
        } else {
            return Executor::Result { -1, executable.error_code };
        }
    }

    Executor::Result Executor::execvP(const char* file, const char* search_path, char* const* argv, char* const* envp) const
    {
        CHECK_SESSION(session_);
        CHECK_POINTER(file);

        if (auto executable = resolver_.from_search_path(file, search_path); executable) {
            const CommandBuilder cmd(session_, executable.return_value, argv);
            const char* dst[cmd.length()];
            cmd.assemble(dst);

            auto return_value = linker_.execve(cmd.file(), const_cast<char* const*>(dst), envp);
            return Executor::Result {return_value, linker_.error_code() };
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

        if (auto executable = resolver_.from_current_directory(path); executable) {
            const CommandBuilder cmd(session_, executable.return_value, argv);
            const char* dst[cmd.length()];
            cmd.assemble(dst);

            auto return_value = linker_.posix_spawn(pid, cmd.file(), file_actions, attrp, const_cast<char* const*>(dst), envp);
            return Executor::Result {return_value, linker_.error_code() };
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

        if (auto executable = resolver_.from_path(file, envp); executable) {
            const CommandBuilder cmd(session_, executable.return_value, argv);
            const char* dst[cmd.length()];
            cmd.assemble(dst);

            auto return_value = linker_.posix_spawn(pid, cmd.file(), file_actions, attrp, const_cast<char* const*>(dst), envp);
            return Executor::Result {return_value, linker_.error_code() };
        } else {
            return Executor::Result { -1, executable.error_code };
        }
    }
}

#pragma GCC diagnostic pop
