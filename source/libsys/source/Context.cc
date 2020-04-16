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
#include "Environment.h"
#include "Errors.h"
#include "config.h"

#include <cerrno>
#include <climits>
#include <cstdlib>
#include <numeric>
#include <unistd.h>

#ifdef HAVE_SPAWN_H
#include <spawn.h>
#endif
#ifdef HAVE_SYS_UTSNAME_H
#include <sys/utsname.h>
#endif
#ifdef HAVE_SYS_WAIT_H
#include <sys/wait.h>
#endif
#ifdef HAVE_DLFCN_H
#include <dlfcn.h>
#endif
#ifdef HAVE_GNU_LIB_NAMES_H
#include <gnu/lib-names.h>
#endif

#include <fmt/format.h>

namespace {

    std::list<std::string> split(const std::string& input, const char sep)
    {
        std::list<std::string> result;

        std::string::size_type previous = 0;
        do {
            const std::string::size_type current = input.find(sep, previous);
            result.emplace_back(input.substr(previous, current - previous));
            previous = (current != std::string::npos) ? current + 1 : current;
        } while (previous != std::string::npos);

        return result;
    }

    std::string join(const std::list<std::string>& input, const char sep)
    {
        std::string result;
        std::accumulate(input.begin(), input.end(), result,
            [&sep](std::string& acc, const std::string& item) {
                return (acc.empty()) ? item : acc + sep + item;
            });
        return result;
    }
}

namespace sys {

    Context::Context(pid_t current, pid_t parent, char** envp)
            : current_(current)
            , parent_(parent)
            , envp_(const_cast<const char**>(envp))
    {
    }

    std::list<std::string> Context::split_path(const std::string& input)
    {
        return split(input, Context::OS_PATH_SEPARATOR);
    }

    std::string Context::join_path(const std::list<std::string>& input)
    {
        return join(input, Context::OS_PATH_SEPARATOR);
    }

    std::map<std::string, std::string> Context::get_environment() const
    {
        return sys::env::from(envp_);
    }

    pid_t Context::get_pid() const
    {
        return current_;
    }

    pid_t Context::get_ppid() const
    {
        return parent_;
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
            return rust::Ok(split_path(candidate->second));
        }
#ifdef HAVE_CS_PATH
        return get_confstr(_CS_PATH)
            .map<std::list<std::string>>([](const auto& paths) {
                return split_path(paths);
            })
            .map_err<std::runtime_error>([](auto error) {
                return std::runtime_error(
                    fmt::format("Could not find PATH: ()", error.what()));
            });
#else
        return rust::Err(std::runtime_error("Could not find PATH in environment."));
#endif
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
}