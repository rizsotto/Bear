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

#include <functional>

#include "libear_a/Result.h"

namespace ear {

    class Resolver {
    public:
        using Execution = std::function<int ()>;

        template <typename S, typename E>
        Result<Execution> resolve(const S *session, const E &execution) const noexcept;

    public: // TODO: make them protected
        using Execve_Fp = int (*)(const char *, char *const *, char *const *);
        using ExecvP_Fp = int (*)(const char *, const char *, char *const *);
        using Spawn_Fp  = int (*)(pid_t *,
                                const char *,
                                const posix_spawn_file_actions_t *,
                                const posix_spawnattr_t *,
                                char *const *,
                                char *const *);
        using Execve = std::function<int (const char *, char *const *, char *const *)>;
        using ExecvP = std::function<int (const char *, const char *, char *const *)>;
        using Spawn  = std::function<int (pid_t *,
                                          const char *,
                                          const posix_spawn_file_actions_t *,
                                          const posix_spawnattr_t *,
                                          char *const *,
                                          char *const *)>;

        virtual ~Resolver() noexcept = default;

        virtual Result<Execve> execve() const noexcept = 0;

        virtual Result<Execve> execvpe() const noexcept = 0;

        virtual Result<ExecvP> execvP() const noexcept = 0;

        virtual Result<Spawn> posix_spawn() const noexcept = 0;

        virtual Result<Spawn> posix_spawnp() const noexcept = 0;
    };

}
