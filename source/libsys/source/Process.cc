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

#include "config.h"
#include "libsys/Process.h"
#include "libsys/Context.h"
#include "libsys/Path.h"
#include "Errors.h"
#include "Environment.h"

#include <cerrno>
#include <climits>
#include <csignal>
#include <cstdlib>
#include <utility>

#ifdef HAVE_SYS_WAIT_H
#include <sys/wait.h>
#endif
#ifdef HAVE_SPAWN_H
#include <spawn.h>
#endif
#ifdef HAVE_DLFCN_H
#include <dlfcn.h>
#endif
#ifdef HAVE_GNU_LIB_NAMES_H
#include <gnu/lib-names.h>
#endif

#include <fmt/format.h>

namespace {

    using posix_spawn_t = int (*)(
        pid_t * pid,
        const char* path,
        const posix_spawn_file_actions_t* file_actions_ptr,
        const posix_spawnattr_t* attr_ptr,
        char* const argv[],
        char* const envp[]);

    using spawn_function_t = std::function<
        rust::Result<pid_t>(
            const char* path,
            const posix_spawn_file_actions_t* file_actions_ptr,
            const posix_spawnattr_t* attr_ptr,
            char* const argv[],
            char* const envp[])>;

    rust::Result<spawn_function_t> reference_spawn_function()
    {
        spawn_function_t result = [](const char* path,
                                        const posix_spawn_file_actions_t* file_actions_ptr,
                                        const posix_spawnattr_t* attr_ptr,
                                        char* const argv[],
                                        char* const envp[]) -> rust::Result<pid_t> {
            errno = 0;
            pid_t child;
            if (0 != posix_spawn(&child, path, nullptr, nullptr, const_cast<char**>(argv), const_cast<char**>(envp))) {
                return rust::Err(std::runtime_error(
                    fmt::format("System call \"posix_spawn\" failed: {}", sys::error_string(errno))));
            } else {
                return rust::Ok(child);
            }
        };
        return rust::Ok(result);
    }

    // This is just a workaround to not call the preloaded execution methods.
    //
    // With static linking the `er` target would deprecate this solution. But
    // The gRPC library brings in a dynamic library. See reported bug:
    //
    //   https://github.com/grpc/grpc/issues/22646
    rust::Result<spawn_function_t> resolve_spawn_function()
    {
        spawn_function_t fp = [](const char* path,
                                 const posix_spawn_file_actions_t* file_actions_ptr,
                                 const posix_spawnattr_t* attr_ptr,
                                 char* const argv[],
                                 char* const envp[]) -> rust::Result<pid_t> {

            auto handle = dlopen(LIBC_SO, RTLD_LAZY);
            if (handle == nullptr) {
                return rust::Err(std::runtime_error(
                    fmt::format("System call \"dlopen\" failed: {}", sys::error_string(errno))));
            }
            dlerror();

            auto fp = reinterpret_cast<posix_spawn_t>(dlsym(handle, "posix_spawn"));
            if (fp == nullptr) {
                return rust::Err(std::runtime_error(
                    fmt::format("System call \"dlsym\" failed: {}", sys::error_string(errno))));
            }
            dlerror();

            errno = 0;
            pid_t child;
            if (0 != (*fp)(&child, path, nullptr, nullptr, const_cast<char**>(argv), const_cast<char**>(envp))) {
                dlclose(handle);
                return rust::Err(std::runtime_error(
                    fmt::format("System call \"posix_spawn\" failed: {}", sys::error_string(errno))));
            } else {
                dlclose(handle);
                return rust::Ok(child);
            }
        };
        return rust::Ok(fp);
    }

    int is_executable(const std::string& path)
    {
        if (0 == access(path.data(), X_OK)) {
            return 0;
        }
        if (0 == access(path.data(), F_OK)) {
            return EACCES;
        }
        return ENOENT;
    }

    rust::Result<std::string> real_path(const std::string& path)
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

    bool contains_separator(const std::string& path)
    {
        return (std::find(path.begin(), path.end(), sys::path::OS_SEPARATOR) != path.end());
    }

    bool starts_with_separator(const std::string& path)
    {
        return (!path.empty()) && (path.at(0) == sys::path::OS_SEPARATOR);
    }

    rust::Result<std::string> resolve_executable(const std::string& name)
    {
        // TODO: inject this!
        sys::Context ctx;

        int error = ENOENT;
        // If the requested program name contains a separator, then we need to use
        // that as is. Otherwise we need to search the paths given.
        if (contains_separator(name)) {
            // If the requested program name starts with the separator, then it's
            // absolute and will be used as is. Otherwise we need to create it from
            // the current working directory.
            auto path = starts_with_separator(name)
                ? rust::Ok(name)
                : ctx.get_cwd().map<std::string>([&name](const auto& cwd) {
                      return fmt::format("{0}{1}{2}", cwd, sys::path::OS_SEPARATOR, name);
                  });
            auto candidate = path.and_then<std::string>([](const auto& path) { return real_path(path); });
            auto executable = candidate
                                  .map<bool>([](auto real) {
                                      return (0 == is_executable(real));
                                  })
                                  .unwrap_or(false);
            if (executable) {
                return candidate;
            }
        } else {
            return ctx.get_path()
                .and_then<std::string>([&name](const auto& directories) {
                    for (const auto& directory : directories) {
                        auto candidate = real_path(fmt::format("{0}{1}{2}", directory, sys::path::OS_SEPARATOR, name));
                        // TODO: check if this is the right thing to do. Shall we look for the
                        //       next executable entry, or shall we fail if we found one with the
                        //       correct name, but has not access rights?
                        auto executable = candidate
                                              .template map<bool>([](auto real) {
                                                  return (0 == is_executable(real));
                                              })
                                              .unwrap_or(false);
                        if (executable) {
                            return candidate;
                        }
                    }
                    return rust::Result<std::string>(rust::Err(std::runtime_error(
                        fmt::format("Could not find executable: {}", sys::error_string(ENOENT)))));
                });
        }
        return rust::Err(std::runtime_error(
            fmt::format("Could not find executable: {}", sys::error_string(error))));
    }
}

namespace sys {

    Process::Process(pid_t pid)
            : pid_(pid)
    {
    }

    pid_t Process::get_pid() const
    {
        return pid_;
    }

    rust::Result<int> Process::wait()
    {
        errno = 0;
        if (int status; - 1 != waitpid(pid_, &status, 0)) {
            const int result = WIFEXITED(status) ? WEXITSTATUS(status) : EXIT_FAILURE;
            return rust::Ok(result);
        } else {
            auto message = fmt::format("System call \"waitpid\" failed: {}", error_string(errno));
            return rust::Err(std::runtime_error(message));
        }
    }

    // TODO: make this to return Result<void>
    rust::Result<int> Process::signal(int num)
    {
        errno = 0;
        if (const int result = kill(pid_, num); 0 == result) {
            return rust::Ok(result);
        } else {
            auto message = fmt::format("System call \"kill\" failed: {}", error_string(errno));
            return rust::Err(std::runtime_error(message));
        }
    }

    Process::Builder::Builder(std::string program)
        : program_(std::move(program))
        , parameters_()
        , environment_()
    {
    }

    Process::Builder::Builder(const std::string_view& program)
        : program_(std::string(program))
        , parameters_()
        , environment_()
    {
    }

    Process::Builder& Process::Builder::add_argument(const char* param)
    {
        parameters_.emplace_back(std::string(param));
        return *this;
    }

    Process::Builder& Process::Builder::add_argument(std::string&& param)
    {
        parameters_.emplace_back(param);
        return *this;
    }

    Process::Builder& Process::Builder::add_argument(const std::string_view& param)
    {
        parameters_.emplace_back(std::string(param));
        return *this;
    }

    Process::Builder& Process::Builder::set_environment(std::map<std::string, std::string>&& environment)
    {
        std::swap(environment_, environment);
        return *this;
    }

    Process::Builder& Process::Builder::set_environment(const std::map<std::string, std::string>& environment)
    {
        environment_ = environment;
        return *this;
    }

    rust::Result<std::string> Process::Builder::resolve_executable()
    {
        return ::resolve_executable(program_);
    }

    rust::Result<Process> Process::Builder::spawn(const bool with_preload)
    {
        auto program = ::resolve_executable(program_);
        auto fp = (with_preload) ? resolve_spawn_function() : reference_spawn_function();

        return rust::merge(program, fp)
            .and_then<pid_t>([this](const auto& pair) {
                const auto& [path, spawn_ptr] = pair;
                // convert the arguments into a c-style array
                std::vector<char*> args;
                std::transform(parameters_.begin(), parameters_.end(),
                    std::back_insert_iterator(args),
                    [](const auto& arg) { return const_cast<char*>(arg.c_str()); });
                args.push_back(nullptr);
                // convert the environment into a c-style array
                sys::env::Guard env(environment_);

                // TODO: check if child process is writing the stdout
                // TODO: check if child process is reading the stdin
                return spawn_ptr(path.c_str(), nullptr, nullptr, const_cast<char**>(args.data()), const_cast<char**>(env.data()));
            })
            .map<Process>([](const auto& pid) {
                return Process(pid);
            });
    }
}
