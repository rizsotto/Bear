/*  Copyright (C) 2012-2022 by László Nagy
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

#include "report/libexec/Linker.h"

#include <cerrno>
#include <dlfcn.h>

namespace {

    template <typename T>
    T dynamic_linker(char const* const name)
    {
        return reinterpret_cast<T>(dlsym(RTLD_NEXT, name));
    }
}

namespace el {

    rust::Result<int, int> Linker::execve(const char* path, char* const* argv, char* const* envp) const noexcept
    {
        using type = int (*)(const char*, char* const[], char* const[]);

        const auto fp = dynamic_linker<type>("execve");
        if (fp == nullptr) {
            return rust::Err(EINVAL);
        }
        auto result = fp(path, argv, envp);
        return (result == -1)
            ? rust::Result<int, int>(rust::Err(errno))
            : rust::Result<int, int>(rust::Ok(result));
    }

    rust::Result<int, int> Linker::posix_spawn(
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

        const auto fp = dynamic_linker<type>("posix_spawn");
        if (fp == nullptr) {
            return rust::Err(EINVAL);
        }
        auto result = fp(pid, path, file_actions, attrp, argv, envp);
        return (result != 0)
            ? rust::Result<int, int>(rust::Err(errno))
            : rust::Result<int, int>(rust::Ok(result));
    }
}
