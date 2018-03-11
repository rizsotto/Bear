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

#include "SystemCalls.h"

#include <wait.h>
#include <spawn.h>

#include <cstring>

namespace {

    template <typename T>
    pear::Result<T> failure(const char *message, int errnum) noexcept {
        std::string result = message != nullptr ? std::string(message) : std::string();

        const size_t buffer_length = 1024 + strlen(message);
        char buffer[buffer_length] = { ':', ' ', '\0' };
        if (0 == strerror_r(errnum, buffer + 2, buffer_length - 2)) {
            result += std::string(buffer);
        } else {
            result += std::string(": Couldn't get error message.");
        }
        return pear::Result<T>::failure(std::runtime_error(result));
    };

}


namespace pear {

    Result<int> spawn(const char **argv, const char **envp) noexcept {
        pid_t child;
        if (0 != posix_spawn(&child,
                             argv[0],
                             nullptr,
                             nullptr,
                             const_cast<char **>(argv),
                             const_cast<char **>(envp))) {
            return failure<pid_t>("posix_spawn", errno);
        } else {
            return pear::Result<pid_t>::success(std::move(child));
        }
    }

    Result<int> wait_pid(pid_t pid) noexcept {
        int status;
        if (-1 == waitpid(pid, &status, 0)) {
            return failure<int>("waitpid", errno);
        } else {
            int result = WIFEXITED(status) ? WEXITSTATUS(status) : EXIT_FAILURE;
            return pear::Result<int>::success(std::move(result));
        }
    }

    Result<pid_t> get_pid() noexcept {
        return pear::Result<pid_t>::success(getpid());
    }

    Result<pid_t> get_ppid() noexcept {
        return pear::Result<pid_t>::success(getppid());
    }

    Result<std::string> get_cwd() noexcept {
        constexpr static const size_t buffer_size = 8192;

        char buffer[buffer_size];
        if (nullptr == getcwd(buffer, buffer_size)) {
            return failure<std::string>("getcwd", errno);
        } else {
            return pear::Result<std::string>::success(std::string(buffer));
        }
    }

}
