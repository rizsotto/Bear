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

#include "libsys/Os.h"
#include "libsys/Errors.h"
#include "Guard.h"
#include "config.h"

#include <cerrno>
#include <unistd.h>

#ifdef HAVE_SYS_UTSNAME_H
#include <sys/utsname.h>
#endif

#include <fmt/format.h>


namespace sys::os {

#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wvla"

    rust::Result<std::string> get_confstr(const int key)
    {
#ifdef HAVE_CONFSTR
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
        return rust::Err(std::runtime_error("System call \"confstr\" not exists."));
#endif
    }

#pragma GCC diagnostic pop

    rust::Result<std::map<std::string, std::string>> get_uname()
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

    rust::Result<std::string> get_path(const sys::env::Vars& environment)
    {
        if (auto candidate = environment.find("PATH"); candidate != environment.end()) {
            return rust::Ok(candidate->second);
        }
#ifdef HAVE_CS_PATH
        return get_confstr(_CS_PATH)
            .map_err<std::runtime_error>([](auto error) {
                return std::runtime_error(
                    fmt::format("Could not find PATH: {}", error.what()));
            });
#else
        return rust::Err(std::runtime_error("Could not find PATH in environment."));
#endif
    }
}