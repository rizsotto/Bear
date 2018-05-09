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

#pragma once

#include "config.h"

#include <unistd.h>
#if defined HAVE_SPAWN_HEADER
# include <spawn.h>
#endif

namespace ear {

    constexpr char command_separator[] = "--";
    constexpr char file_flag[] = "-f";
    constexpr char search_flag[] = "-s";

    struct Execution {
        const char **argv;
        const char **envp;

        Execution(char *const *argv, char *const *envp)
                : argv(const_cast<const char **>(argv))
                , envp(const_cast<const char **>(envp))
        { }

        virtual ~Execution() noexcept = default;
    };

    struct Execve : public Execution {
        Execve(const char *path, char *const *argv, char *const *envp)
                : Execution(argv, envp)
                , path(path)
        { }

        const char *path;
    };

    struct Execvpe : public Execution {
        Execvpe(const char *file, char *const *argv, char *const *envp)
                : Execution(argv, envp)
                , file(file)
        { }

        const char *file;
    };

    struct ExecvP : public Execution {
        ExecvP(const char *file, const char *search_path, char *const *argv)
                : Execution(argv, nullptr)
                , file(file)
                , search_path(search_path)
        { }

        const char *file;
        const char *search_path;
    };

    struct ExecutionWithoutFork : public Execution {
        ExecutionWithoutFork(
                pid_t *pid,
                const posix_spawn_file_actions_t *file_actions,
                const posix_spawnattr_t *attrp,
                char *const *argv,
                char *const *envp)
                : Execution(argv, envp)
                , pid_(pid)
                , file_actions(file_actions)
                , attrp(attrp)
        { }

        pid_t *pid_;
        const posix_spawn_file_actions_t *file_actions;
        const posix_spawnattr_t *attrp;
    };

    struct Spawn : public ExecutionWithoutFork {
        Spawn(
                pid_t *pid,
                const char *path,
                const posix_spawn_file_actions_t *file_actions,
                const posix_spawnattr_t *attrp,
                char *const *argv,
                char *const *envp)
                : ExecutionWithoutFork(pid, file_actions, attrp, argv, envp)
                , path(path)
        { }

        const char *path;
    };

    struct Spawnp : public ExecutionWithoutFork {
        Spawnp(
                pid_t *pid,
                const char *file,
                const posix_spawn_file_actions_t *file_actions,
                const posix_spawnattr_t *attrp,
                char *const *argv,
                char *const *envp)
                : ExecutionWithoutFork(pid, file_actions, attrp, argv, envp)
                , file(file)
        { }

        const char *file;
    };

}
