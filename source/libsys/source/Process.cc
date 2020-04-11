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

#include "libsys/Process.h"
#include "Errors.h"
#include "config.h"

#ifdef HAVE_WAIT_H
#include <sys/wait.h>
#endif
#ifdef HAVE_SPAWN_H
#include <spawn.h>
#endif
#include <fmt/format.h>

namespace sys {

    pid_t Process::get_pid()
    {
        return getpid();
    }

    pid_t Process::get_ppid()
    {
        return getppid();
    }

    rust::Result<pid_t> Process::spawn(const char* path, const char** argv, const char** envp) const
    {
        errno = 0;
        pid_t child;
        if (0 != posix_spawn(&child, path, nullptr, nullptr, const_cast<char**>(argv), const_cast<char**>(envp))) {
            return rust::Err(std::runtime_error(
                fmt::format("System call \"posix_spawn\" failed: {}", error_string(errno))));
        } else {
            return rust::Ok(child);
        }
    }

    rust::Result<int> Process::wait_pid(pid_t pid) const
    {
        errno = 0;
        int status;
        if (-1 == waitpid(pid, &status, 0)) {
            return rust::Err(std::runtime_error(
                fmt::format("System call \"waitpid\" failed: {}", error_string(errno))));

        } else {
            const int result = WIFEXITED(status) ? WEXITSTATUS(status) : EXIT_FAILURE;
            return rust::Ok(result);
        }
    }
}
