/*  Copyright (C) 2012-2018 by László Nagy
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

namespace ear {

    /**
     * It is an abstraction of the symbol resolver.
     *
     * It uses the provided symbol resolver method and cast the result
     * to a specific type.
     *
     * Design decisions:
     *
     * - Could have been just a function pointer, but a class might be
     * better choice for this language. It also allows multiple
     * implementation, which might be stateful.
     *
     * - Could have been using inheritance. But virtual functions needs
     * symbols from the `libstdc++` library, which I wanted to avoid.
     */
    class Resolver {
    public:
        using resolver_t =
                void *(*)(char const *const name);

        using execve_t =
                int (*)(const char *path,
                        char *const argv[],
                        char *const envp[]);

        using posix_spawn_t =
                int (*)(pid_t *pid,
                        const char *path,
                        const posix_spawn_file_actions_t *file_actions,
                        const posix_spawnattr_t *attrp,
                        char *const argv[],
                        char *const envp[]);

    public:
        /**
         * Constructor.
         *
         * @param resolver function pointer to the OS's dynamic linker
         * symbol resolver method.
         */
        explicit Resolver(resolver_t resolver) noexcept;

        execve_t execve() const noexcept;

        posix_spawn_t posix_spawn() const noexcept;

    private:
        resolver_t resolver_;
    };

    inline
    Resolver::Resolver(Resolver::resolver_t resolver) noexcept
            : resolver_(resolver)
    { }

    inline
    Resolver::execve_t Resolver::execve() const noexcept {
        return reinterpret_cast<execve_t >(resolver_("execve"));
    }

    inline
    Resolver::posix_spawn_t Resolver::posix_spawn() const noexcept {
        return reinterpret_cast<posix_spawn_t >(resolver_("posix_spawn"));
    }
}
