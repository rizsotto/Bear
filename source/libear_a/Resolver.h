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

#include <unistd.h>
#if defined HAVE_SPAWN_HEADER
# include <spawn.h>
#endif

#include <functional>

namespace ear {

    class Resolver {
    public:
        using Execve = std::function<int (const char *, char *const *, char *const *)>;
        using ExecvP = std::function<int (const char *, const char *, char *const *)>;
        using Spawn  = std::function<int (pid_t *,
                                          const char *,
                                          const posix_spawn_file_actions_t *,
                                          const posix_spawnattr_t *,
                                          char *const *,
                                          char *const *)>;

        virtual ~Resolver() noexcept = default;

        virtual Execve execve() const noexcept = 0;

        virtual Execve execvpe() const noexcept = 0;

        virtual ExecvP execvP() const noexcept = 0;

        virtual Spawn posix_spawn() const noexcept = 0;

        virtual Spawn posix_spawnp() const noexcept = 0;
    };

}
