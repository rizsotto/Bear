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
#include "Errors.h"
#include "Guard.h"
#include "config.h"

#include <cerrno>
#include <sys/types.h>
#include <dirent.h>
#include <unistd.h>

#ifdef HAVE_SYS_UTSNAME_H
#include <sys/utsname.h>
#endif

#include <fmt/format.h>


namespace sys {

#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wvla"

    rust::Result<std::string> Context::get_confstr(const int key) const
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
#error confstr is not found
#endif
    }

#pragma GCC diagnostic pop

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

    rust::Result<std::list<fs::path>> Context::get_path(const sys::env::Vars& environment) const
    {
        if (auto candidate = environment.find("PATH"); candidate != environment.end()) {
            return rust::Ok(sys::path::split(candidate->second));
        }
#ifdef HAVE_CS_PATH
        return get_confstr(_CS_PATH)
            .map<std::list<fs::path>>([](const auto& paths) {
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

    rust::Result<std::list<fs::path>> Context::list_dir(const fs::path& path) const
    {
        DIR *dp = opendir(path.c_str());
        if (dp == nullptr)
            return rust::Err(std::runtime_error(
                fmt::format("Could not open directory: {}", path.string())));

        std::list<fs::path> result;
        while (dirent *ep = readdir(dp)) {
            const std::string file(ep->d_name);
            if (file != "." && file != "..") {
                result.push_back(fs::path(path) / file);
            }
        }
        closedir(dp);

        return rust::Ok(result);
    }
}