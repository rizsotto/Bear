/*  Copyright (C) 2012-2021 by László Nagy
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

#include "libresult/Result.h"

#ifdef HAVE_SPAWN_H
#include <spawn.h>
#endif
#include <sys/types.h>

namespace el {

    struct Linker;
    struct Session;
    class Resolver;

    /**
     * This class implements the process execution logic.
     *
     * The caller of this is the POSIX interface for process creation.
     * This class encapsulate most of the logic and leave the C wrapper light
     * in order to test the functionality in unit tests.
     *
     * This is just a subset of all process creation calls.
     *
     * - Variable argument methods are not implemented. (The `execl*` methods.)
     *   Caller needs to convert those convenient functions by collecting the
     *   arguments into a C array.
     *
     * - Environment needs to pass for this methods. If a method does not have
     *   the environment explicitly passed as argument, it needs to grab it
     *   and pass to these methods.
     */
    class Executor {
    public:
        Executor(el::Linker const& linker, el::Session const& session, el::Resolver &resolver) noexcept;

        ~Executor() noexcept = default;

    public:
        rust::Result<int, int> execve(const char* path, char* const argv[], char* const envp[]) const;

        rust::Result<int, int> execvpe(const char* file, char* const argv[], char* const envp[]) const;

        rust::Result<int, int> execvP(const char* file, const char* search_path, char* const argv[], char* const envp[]) const;

        rust::Result<int, int> posix_spawn(pid_t* pid, const char* path,
            const posix_spawn_file_actions_t* file_actions,
            const posix_spawnattr_t* attrp,
            char* const argv[],
            char* const envp[]) const;

        rust::Result<int, int> posix_spawnp(pid_t* pid, const char* file,
            const posix_spawn_file_actions_t* file_actions,
            const posix_spawnattr_t* attrp,
            char* const argv[],
            char* const envp[]) const;

    private:
        el::Linker const &linker_;
        el::Session const &session_;
        el::Resolver &resolver_;
    };
}
