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

    struct Execution_Z {
        const char **argv_;
        const char **envp_;

        Execution_Z(const char **argv, const char **envp)
                : argv_(argv)
                , envp_(envp)
        { }

        virtual ~Execution_Z() noexcept = default;
    };

    struct Execve_Z : public Execution_Z {
        Execve_Z(const char *path, char *const *argv, char *const *envp)
                : Execution_Z(const_cast<const char **>(argv), const_cast<const char **>(envp))
                , path_(path)
        { }

        const char *path_;
    };

    struct Execvpe_Z : public Execution_Z {
        Execvpe_Z(const char *file, char *const *argv, char *const *envp)
                : Execution_Z(const_cast<const char **>(argv), const_cast<const char **>(envp))
                , file_(file)
        { }

        const char *file_;
    };

    struct ExecvP_Z : public Execution_Z {
        ExecvP_Z(const char *file, const char *search_path, char *const *argv)
                : Execution_Z(const_cast<const char **>(argv), nullptr)
                , file_(file)
                , search_path_(search_path)
        { }

        const char *file_;
        const char *search_path_;
    };

    struct ExecutionWithoutFork_Z : public Execution_Z {
        ExecutionWithoutFork_Z(
                pid_t *pid,
                const posix_spawn_file_actions_t *file_actions,
                const posix_spawnattr_t *attrp,
                char *const *argv,
                char *const *envp)
                : Execution_Z(const_cast<const char **>(argv), const_cast<const char **>(envp))
                , pid_(pid)
                , file_actions_(file_actions)
                , attrp_(attrp)
        { }

        pid_t *pid_;
        const posix_spawn_file_actions_t *file_actions_;
        const posix_spawnattr_t *attrp_;
    };

    struct Spawn_Z : public ExecutionWithoutFork_Z {
        Spawn_Z(
                pid_t *pid,
                const char *path,
                const posix_spawn_file_actions_t *file_actions,
                const posix_spawnattr_t *attrp,
                char *const *argv,
                char *const *envp)
                : ExecutionWithoutFork_Z(pid, file_actions, attrp, argv, envp)
                , path_(path)
        { }

        const char *path_;
    };

    struct Spawnp_Z : public ExecutionWithoutFork_Z {
        Spawnp_Z(
                pid_t *pid,
                const char *file,
                const posix_spawn_file_actions_t *file_actions,
                const posix_spawnattr_t *attrp,
                char *const *argv,
                char *const *envp)
                : ExecutionWithoutFork_Z(pid, file_actions, attrp, argv, envp)
                , file_(file)
        { }

        const char *file_;
    };

}
