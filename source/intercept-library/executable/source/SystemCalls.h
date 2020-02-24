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

#include <memory>
#include <unistd.h>

#include "Result.h"

namespace er {

    struct SystemCalls {

        static Result<pid_t>
        fork_with_execvp(const char* file, const char* search_path, const char** argv, const char** envp) noexcept;

        static Result<pid_t>
        spawn(const char* path, const char** argv, const char** envp) noexcept;

        static Result<pid_t>
        spawnp(const char* file, const char** argv, const char** envp) noexcept;

        static Result<int>
        wait_pid(pid_t pid) noexcept;

        static Result<pid_t>
        get_pid() noexcept;

        static Result<pid_t>
        get_ppid() noexcept;

        static Result<std::string>
        get_cwd() noexcept;

        static Result<std::shared_ptr<std::ostream>>
        temp_file(const char* dir, const char* suffix) noexcept;
    };

}
