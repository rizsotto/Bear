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

#include "Resolver.h"

#include <dlfcn.h>

#if defined HAVE_NSGETENVIRON
#include <crt_externs.h>
#else
extern "C" char** environ;
#endif

namespace {

    void* dynamic_linker(char const* const name)
    {
        return dlsym(RTLD_NEXT, name);
    }
}

namespace ear {

    int Resolver::execve(const char* path, char* const* argv, char* const* envp) const noexcept
    {
        using type = int (*)(const char*, char* const [], char* const []);

        auto fp = reinterpret_cast<type>(dynamic_linker("execve"));
        return (fp == nullptr)
            ? -1
            : fp(path, argv, envp);
    }

    int Resolver::posix_spawn(
        pid_t* pid,
        const char* path,
        const posix_spawn_file_actions_t* file_actions,
        const posix_spawnattr_t* attrp,
        char* const* argv,
        char* const* envp) const noexcept
    {
        using type = int (*)(
            pid_t * pid,
            const char* path,
            const posix_spawn_file_actions_t* file_actions,
            const posix_spawnattr_t* attrp,
            char* const argv[],
            char* const envp[]);

        auto fp = reinterpret_cast<type>(dynamic_linker("posix_spawn"));
        return (fp == nullptr)
            ? -1
            : fp(pid, path, file_actions, attrp, argv, envp);
    }

    int Resolver::access(const char* pathname, int mode) const noexcept
    {
        using type = int (*)(const char*, int);

        auto fp = reinterpret_cast<type>(dynamic_linker("access"));
        return (fp == nullptr)
            ? -1
            : fp(pathname, mode);
    }

    /**
     * Abstraction to get the current environment.
     *
     * When the dynamic linker loads the library the `environ` variable
     * might not be available. (This is the case for OSX.) This method
     * makes it uniform to access the current environment on all platform.
     *
     * @return the current environment.
     */
    const char** Resolver::environment() const noexcept
    {
#ifdef HAVE_NSGETENVIRON
        return const_cast<const char**>(*_NSGetEnviron());
#else
        return const_cast<const char**>(environ);
#endif
    }
}
