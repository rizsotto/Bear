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

#pragma once

#include <spawn.h>

namespace el {

    /**
     * It is an abstraction of the symbol resolver.
     *
     * It uses the provided symbol resolver method and cast the result
     * to a specific type.
     */
    class Resolver {
    public:
        Resolver() noexcept = default;

        virtual ~Resolver() = default;

        virtual int execve(
            const char* path,
            char* const argv[],
            char* const envp[]) const noexcept;

        virtual int posix_spawn(
            pid_t* pid,
            const char* path,
            const posix_spawn_file_actions_t* file_actions,
            const posix_spawnattr_t* attrp,
            char* const argv[],
            char* const envp[]) const noexcept;

        virtual int access(
            const char* pathname,
            int mode) const noexcept;

        virtual char* realpath(const char* path, char* resolved_path) const noexcept;

        virtual size_t confstr(int name, char* buf, size_t len) const noexcept;

        [[nodiscard]] virtual const char** environment() const noexcept;

        [[nodiscard]] virtual int error_code() const noexcept;
    };
}
