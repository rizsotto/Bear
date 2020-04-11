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

#include "libsys/FileSystem.h"
#include "Errors.h"
#include "config.h"

#include <cerrno>
#include <climits>
#include <cstdlib>
#include <list>
#include <numeric>
#include <string>
#include <unistd.h>

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

    bool contains_separator(const std::string& path)
    {
        return (std::find(path.begin(), path.end(), sys::FileSystem::OS_SEPARATOR) != path.end());
    }

    bool starts_with_separator(const std::string& path)
    {
        return (!path.empty()) && (path.at(0) == sys::FileSystem::OS_SEPARATOR);
    }
}

namespace sys {

    std::list<std::string> FileSystem::split_path(const std::string& input)
    {
        return split(input, FileSystem::OS_PATH_SEPARATOR);
    }

    std::string FileSystem::join_path(const std::list<std::string>& input)
    {
        return join(input, FileSystem::OS_PATH_SEPARATOR);
    }

    rust::Result<std::string> FileSystem::get_cwd() const
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

    rust::Result<std::string> FileSystem::find_in_path(const std::string& name, const std::string& paths) const
    {
        int error = ENOENT;
        // If the requested program name contains a separator, then we need to use
        // that as is. Otherwise we need to search the paths given.
        if (contains_separator(name)) {
            // If the requested program name starts with the separator, then it's
            // absolute and will be used as is. Otherwise we need to create it from
            // the current working directory.
            auto path = starts_with_separator(name)
                ? rust::Ok(name)
                : get_cwd().map<std::string>([&name](const auto& cwd) {
                    return fmt::format("{0}{1}{2}", cwd, OS_SEPARATOR, name);
                });
            auto candidate = path.and_then<std::string>([this](const auto& path) { return real_path(path); });
            auto executable = candidate
                                  .map<bool>([this](auto real) {
                                      return (0 == is_executable(real));
                                  })
                                  .unwrap_or(false);
            if (executable) {
                return candidate;
            }
        } else {
            auto directories = split(paths, OS_PATH_SEPARATOR);
            for (const auto& directory : directories) {
                auto candidate = real_path(fmt::format("{0}{1}{2}", directory, OS_SEPARATOR, name));
                auto executable = candidate
                                      .map<bool>([this](auto real) {
                                          return (0 == is_executable(real));
                                      })
                                      .unwrap_or(false);
                if (executable) {
                    return candidate;
                }
            }
        }
        return rust::Err(std::runtime_error(
            fmt::format("Could not find executable: {}", error_string(error))));
    }

    int FileSystem::is_executable(const std::string& path) const
    {
        if (0 == access(path.data(), X_OK)) {
            return 0;
        }
        if (0 == access(path.data(), F_OK)) {
            return EACCES;
        }
        return ENOENT;
    }

    rust::Result<std::string> FileSystem::real_path(const std::string& path) const
    {
        errno = 0;
        if (char* result_ptr = realpath(path.data(), nullptr); result_ptr != nullptr) {
            std::string result(result_ptr);
            free(result_ptr);
            return rust::Ok(result);
        } else {
            return rust::Err(std::runtime_error(
                fmt::format("Could not create absolute path for \"{}\": ", path, error_string(errno))));
        }
    }
}