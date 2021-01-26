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

namespace el {

    /**
     * It is an abstraction of the symbol resolver.
     *
     * It uses the provided symbol resolver method and cast the result
     * to a specific type.
     */
    struct Linker {
        virtual ~Linker() noexcept = default;

        [[nodiscard]]
        virtual rust::Result<int, int> execve(
            const char* path,
            char* const argv[],
            char* const envp[]) const noexcept;

        [[nodiscard]]
        virtual rust::Result<int, int> posix_spawn(
            pid_t* pid,
            const char* path,
            const posix_spawn_file_actions_t* file_actions,
            const posix_spawnattr_t* attrp,
            char* const argv[],
            char* const envp[]) const noexcept;
    };
}
