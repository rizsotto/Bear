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

#include "libsys/Context.h"
#include "libsys/Path.h"
#include "Environment.h"
#include "Errors.h"
#include "config.h"

#include <cerrno>
#include <climits>
#include <cstdio>
#include <sys/types.h>
#include <dirent.h>
#include <unistd.h>

#ifdef HAVE_SYS_UTSNAME_H
#include <sys/utsname.h>
#endif
#ifndef HAVE_ENVIRON
extern char **environ;
#endif

#include <fmt/format.h>


namespace sys {

    std::map<std::string, std::string> Context::get_environment() const
    {
        return sys::env::from(const_cast<const char**>(environ));
    }

    pid_t Context::get_pid() const
    {
        return getpid();
    }

    pid_t Context::get_ppid() const
    {
        return getppid();
    }

    rust::Result<std::string> Context::get_confstr(const int key) const
    {
#ifdef HAVE_UNAME
        errno = 0;
        if (const size_t buffer_size = confstr(key, nullptr, 0); buffer_size != 0) {
            char buffer[buffer_size];
            if (const size_t size = confstr(key, buffer, buffer_size); size != 0) {
                return rust::Ok(std::string(buffer));
            }
        }
        return rust::Err(std::runtime_error(
            fmt::format("System call \"confstr\" failed.: {}", error_string(errno))));
#else
#error confstr is not found
#endif
    }

    rust::Result<std::map<std::string, std::string>> Context::get_uname() const
    {
        std::map<std::string, std::string> result;
#ifdef HAVE_UNAME
        auto name = utsname {};
        if (const int status = uname(&name); status >= 0) {
            result.insert({ "sysname", std::string(name.sysname) });
            result.insert({ "release", std::string(name.release) });
            result.insert({ "version", std::string(name.version) });
            result.insert({ "machine", std::string(name.machine) });
        }
#else
        result.insert({ "sysname", "unknown" });
#endif
        return rust::Ok(result);
    }

    rust::Result<std::list<std::string>> Context::get_path() const
    {
        const auto environment = get_environment();
        if (auto candidate = environment.find("PATH"); candidate != environment.end()) {
            return rust::Ok(sys::path::split(candidate->second));
        }
#ifdef HAVE_CS_PATH
        return get_confstr(_CS_PATH)
            .map<std::list<std::string>>([](const auto& paths) {
                return sys::path::split(paths);
            })
            .map_err<std::runtime_error>([](auto error) {
                return std::runtime_error(
                    fmt::format("Could not find PATH: ()", error.what()));
            });
#else
        return rust::Err(std::runtime_error("Could not find PATH in environment."));
#endif
    }

    rust::Result<std::list<std::string>> Context::list_dir(const std::string_view& path) const
    {
        DIR *dp = opendir(path.data());
        if (dp == nullptr)
            return rust::Err(std::runtime_error(
                fmt::format("Could not open directory: {}", path)));

        std::list<std::string> result;
        while (dirent *ep = readdir(dp)) {
            const std::string file(ep->d_name);
            if (file != "." && file != "..") {
                result.push_back(sys::path::concat(std::string(path), file));
            }
        }
        closedir(dp);

        return rust::Ok(result);
    }

    rust::Result<std::string> Context::get_cwd() const
    {
        constexpr static const size_t buffer_size = PATH_MAX;
        errno = 0;

        char buffer[buffer_size];
        if (nullptr == getcwd(buffer, buffer_size)) {
            return rust::Err(std::runtime_error(
                fmt::format("System call \"getcwd\" failed: {}", error_string(errno))));
        } else {
            return rust::Ok(std::string(buffer));
        }
    }

    int Context::is_executable(const std::string& path) const
    {
        if (0 == access(path.data(), X_OK)) {
            return 0;
        }
        if (0 == access(path.data(), F_OK)) {
            return EACCES;
        }
        return ENOENT;
    }

    rust::Result<std::string> Context::real_path(const std::string& path) const
    {
        errno = 0;
        if (char* result_ptr = realpath(path.data(), nullptr); result_ptr != nullptr) {
            std::string result(result_ptr);
            free(result_ptr);
            return rust::Ok(result);
        } else {
            return rust::Err(std::runtime_error(
                fmt::format("Could not create absolute path for \"{}\": ", path, sys::error_string(errno))));
        }
    }
}