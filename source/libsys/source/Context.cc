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
#include "Errors.h"
#include "config.h"
#include "libsys/Environment.h"

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

    bool contains_separator(const std::string& path)
    {
        return (std::find(path.begin(), path.end(), sys::Context::OS_SEPARATOR) != path.end());
    }

    bool starts_with_separator(const std::string& path)
    {
        return (!path.empty()) && (path.at(0) == sys::Context::OS_SEPARATOR);
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

    rust::Result<std::string> Context::resolve_executable(const std::string& name) const
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
            return get_path()
                .and_then<std::string>([this, &name](const auto& directories) {
                    for (const auto& directory : directories) {
                        auto candidate = real_path(fmt::format("{0}{1}{2}", directory, OS_SEPARATOR, name));
                        // TODO: check if this is the right thing to do. Shall we look for the
                        //       next executable entry, or shall we fail if we found one with the
                        //       correct name, but has not access rights?
                        auto executable = candidate
                                              .template map<bool>([this](auto real) {
                                                  return (0 == is_executable(real));
                                              })
                                              .unwrap_or(false);
                        if (executable) {
                            return candidate;
                        }
                    }
                    return rust::Result<std::string>(rust::Err(std::runtime_error(
                        fmt::format("Could not find executable: {}", error_string(ENOENT)))));
                });
        }
        return rust::Err(std::runtime_error(
            fmt::format("Could not find executable: {}", error_string(error))));
    }

    rust::Result<pid_t> Context::spawn(const char* path, const char** argv, const char** envp) const
    {
        using spawn_t = int (*)(pid_t*, const char*, const posix_spawn_file_actions_t*, const posix_spawnattr_t*,
                                char* const argv[], char* const envp[]);

#if defined(HAVE_DLOPEN) && defined(HAVE_DLSYM) && defined(HAVE_DLERROR)
        // This is just a workaround to not call the preloaded execution methods.
        //
        // With static linking the `er` target would deprecate this solution. But
        // The gRPC library brings in a dynamic library. See reported bug:
        //
        //   https://github.com/grpc/grpc/issues/22646
        auto handle = dlopen(LIBC_SO, RTLD_LAZY);
        if (handle == nullptr) {
            return rust::Err(std::runtime_error(
                fmt::format("System call \"dlopen\" failed: {}", error_string(errno))));
        }
        dlerror();

        auto fp = reinterpret_cast<spawn_t>(dlsym(handle, "posix_spawn"));
        if (fp == nullptr) {
            return rust::Err(std::runtime_error(
                fmt::format("System call \"dlsym\" failed: {}", error_string(errno))));
        }
        dlerror();
#else
        auto fp = &posix_spawn;
#endif

        errno = 0;
        pid_t child;
        if (0 != (*fp)(&child, path, nullptr, nullptr, const_cast<char**>(argv), const_cast<char**>(envp))) {
#if defined(HAVE_DLCLOSE)
            dlclose(handle);
#endif
            return rust::Err(std::runtime_error(
                fmt::format("System call \"posix_spawn\" failed: {}", error_string(errno))));
        } else {
#if defined(HAVE_DLCLOSE)
            dlclose(handle);
#endif
            return rust::Ok(child);
        }
    }

    rust::Result<int> Context::wait_pid(pid_t pid) const
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
                fmt::format("Could not create absolute path for \"{}\": ", path, error_string(errno))));
        }
    }
}