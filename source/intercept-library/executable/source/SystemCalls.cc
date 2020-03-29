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

#include "SystemCalls.h"

#include <climits>
#include <spawn.h>
#include <sys/wait.h>

#include <cstring>
#include <cerrno>
#include <fstream>
#include <memory>

using rust::Result;
using rust::Ok;
using rust::Err;

namespace {
    constexpr char OS_PATH_SEPARATOR = '/';

    template <typename T>
    Result<T> error(const char* message, const int error) noexcept
    {
        std::string result = message != nullptr ? std::string(message) : std::string("generic error");

        result += " (errno: ";
        result += std::to_string(error);
        result += ")";

        return Err(std::runtime_error(result));
    };
}

namespace er {

    Result<int> SystemCalls::spawn(const char* path, const char** argv, const char** envp) noexcept
    {
        errno = ENOENT;
        pid_t child;
        if (0 != posix_spawn(&child, path, nullptr, nullptr, const_cast<char**>(argv), const_cast<char**>(envp))) {
            return error<pid_t>("posix_spawn", errno);
        } else {
            return Ok(child);
        }
    }

    Result<int> SystemCalls::wait_pid(pid_t pid) noexcept
    {
        errno = ENOENT;
        int status;
        if (-1 == waitpid(pid, &status, 0)) {
            return error<int>("waitpid", errno);
        } else {
            const int result = WIFEXITED(status) ? WEXITSTATUS(status) : EXIT_FAILURE;
            return Ok(result);
        }
    }

    Result<pid_t> SystemCalls::get_pid() noexcept
    {
        return Ok(getpid());
    }

    Result<pid_t> SystemCalls::get_ppid() noexcept
    {
        return Ok(getppid());
    }

    Result<std::string> SystemCalls::get_cwd() noexcept
    {
        constexpr static const size_t buffer_size = PATH_MAX;
        errno = ENOENT;

        char buffer[buffer_size];
        if (nullptr == getcwd(buffer, buffer_size)) {
            return error<std::string>("getcwd", errno);
        } else {
            return Ok(std::string(buffer));
        }
    }

    Result<std::shared_ptr<std::ostream>> SystemCalls::temp_file(const char* dir, const char* suffix) noexcept
    {
        // TODO: validate input?
        const auto& path = std::string(dir) + OS_PATH_SEPARATOR + "XXXXXX" + suffix;
        // create char buffer with this filename.
        const size_t buffer_size = path.length() + 1;
        char buffer[buffer_size];
        std::copy(path.c_str(), path.c_str() + path.length() + 1, (char*)buffer);
        // create the temporary file.
        errno = ENOENT;
        if (-1 == mkstemps(buffer, strlen(suffix))) {
            return error<std::shared_ptr<std::ostream>>("mkstemp", errno);
        } else {
            auto result = std::make_shared<std::ofstream>(std::string(buffer));
            return Ok(std::dynamic_pointer_cast<std::ostream>(result));
        }
    }
}
