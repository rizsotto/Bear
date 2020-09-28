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
#include "libsys/Os.h"
#include "libsys/Path.h"
#include "Errors.h"
#include "Guard.h"

#include <cerrno>
#include <cstdlib>
#include <csignal>
#include <filesystem>
#include <utility>
#include <iostream>

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
#else
#define LIBC_SO "libc.so"
#endif

#include <spdlog/spdlog.h>
#include <spdlog/fmt/ostr.h>
#include <spdlog/sinks/stdout_sinks.h>

namespace {

    struct Arguments {
        const std::list<std::string>& value;
    };

    std::ostream& operator<<(std::ostream& os, const Arguments& arguments)
    {
        os << '[';
        for (auto it = arguments.value.begin(); it != arguments.value.end(); ++it) {
            if (it != arguments.value.begin()) {
                os << ", ";
            }
            os << *it;
        }
        os << ']';

        return os;
    }

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
            char* const argv[],
            char* const envp[])>;

    rust::Result<spawn_function_t> reference_spawn_function()
    {
        spawn_function_t result = [](const char* path,
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

    rust::Result<spawn_function_t> resolve_spawn_function()
    {
        spawn_function_t fp = [](const char* path,
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

    bool contains_separator(const std::string& path)
    {
        return (std::find(path.begin(), path.end(), fs::path::preferred_separator) != path.end());
    }

    bool starts_with_separator(const std::string& path)
    {
        return (!path.empty()) && (path.at(0) == fs::path::preferred_separator);
    }

    rust::Result<fs::path> get_cwd()
    {
        std::error_code error_code;
        auto result = fs::current_path(error_code);
        return (error_code)
               ? rust::Result<fs::path>(rust::Err(std::runtime_error(error_code.message())))
               : rust::Result<fs::path>(rust::Ok(result));
    }

    std::runtime_error could_not_find(const fs::path& name, const int error)
    {
        return std::runtime_error(
                fmt::format("Could not find executable: {} ({})", name.c_str(), sys::error_string(error)));
    }

    rust::Result<fs::path> check_executable(const fs::path& path)
    {
        // Check if we can get the relpath of this file
        std::error_code error_code;
        auto result = fs::canonical(path, error_code);
        if (error_code) {
            return rust::Err(std::runtime_error(error_code.message()));
        }
        // Check if the file is executable.
        return (0 == access(result.c_str(), X_OK))
                ? rust::Result<fs::path>(rust::Ok(result))
                : rust::Result<fs::path>(rust::Err(could_not_find(result, EACCES)));
    }

    rust::Result<fs::path> resolve_executable(const fs::path& name, const sys::env::Vars& environment)
    {
        // If the requested program name contains a separator, then we need to use
        // that as is. Otherwise we need to search the paths given.
        if (contains_separator(name)) {
            // If the requested program name starts with the separator, then it's
            // absolute and will be used as is. Otherwise we need to create it from
            // the current working directory.
            auto path = starts_with_separator(name)
                ? rust::Ok(fs::path(name))
                : get_cwd().map<fs::path>([&name](auto cwd) { return cwd  / name; });

            return path.and_then<fs::path>([](auto path) { return check_executable(path); });
        } else {
            return sys::os::get_path(environment)
                .and_then<fs::path>([&name](const auto& directories) {
                    for (const auto& directory : directories) {
                        if (auto result = check_executable(directory / name); result.is_ok()) {
                            return result;
                        }
                    }
                    return rust::Result<fs::path>(rust::Err(could_not_find(name, ENOENT)));
                });
        }
    }

    rust::Result<sys::Process> spawn_process(
            spawn_function_t fp,
            const fs::path& program,
            const std::list<std::string>& parameters,
            const std::map<std::string, std::string>& environment)
    {
        return resolve_executable(program, environment)
                .and_then<pid_t>([&parameters, &environment, &fp](const auto& path) {
                    // convert the arguments into a c-style array
                    std::vector<char*> args;
                    std::transform(parameters.begin(), parameters.end(),
                                   std::back_insert_iterator(args),
                                   [](const auto& arg) { return const_cast<char*>(arg.c_str()); });
                    args.push_back(nullptr);
                    // convert the environment into a c-style array
                    sys::env::Guard env(environment);

                    return fp(path.c_str(), args.data(), const_cast<char**>(env.data()));
                })
                .map<sys::Process>([](const auto& pid) {
                    return sys::Process(pid);
                })
                .on_success([&parameters](const auto& process) {
                    spdlog::debug("Process spawned. [pid: {}, command: {}]", process.get_pid(), Arguments { parameters });
                })
                .on_error([](const auto& error) {
                    spdlog::debug("Process spawn failed. {}", error.what());
                });
    }

    rust::Result<sys::ExitStatus> wait_for(pid_t pid, bool request_for_signals)
    {
        errno = 0;
        const int mask = request_for_signals ? (WUNTRACED | WCONTINUED) : 0;
        if (int status; - 1 != waitpid(pid, &status, mask)) {
            if (WIFEXITED(status)) {
                return rust::Ok(sys::ExitStatus(true, WEXITSTATUS(status)));
            } else if (WIFSIGNALED(status)) {
                return rust::Ok(sys::ExitStatus(false, WTERMSIG(status)));
            } else if (WIFSTOPPED(status)) {
                return rust::Ok(sys::ExitStatus(false, WSTOPSIG(status)));
            } else if (WIFCONTINUED(status)) {
                return rust::Ok(sys::ExitStatus(false, SIGCONT));
            } else {
                return rust::Err(std::runtime_error("System call \"waitpid\" result is broken."));
            }
        } else {
            auto message = fmt::format("System call \"waitpid\" failed: {}", sys::error_string(errno));
            return rust::Err(std::runtime_error(message));
        }
    }

    rust::Result<int> send_signal(pid_t pid, int num)
    {
        errno = 0;
        if (const int result = kill(pid, num); 0 == result) {
            return rust::Ok(result);
        } else {
            auto message = fmt::format("System call \"kill\" failed: {}", sys::error_string(errno));
            return rust::Err(std::runtime_error(message));
        }
    }
}

namespace sys {

    ExitStatus::ExitStatus(bool is_code, int code)
            : is_code_(is_code)
            , code_(code)
    {
    }

    std::optional<int> ExitStatus::code() const
    {
        return is_code_ ? std::make_optional(code_) : std::optional<int>();
    }

    std::optional<int> ExitStatus::signal() const
    {
        return is_code_ ? std::optional<int>() : std::make_optional(code_);
    }

    bool ExitStatus::is_signaled() const
    {
        return !is_code_;
    }

    bool ExitStatus::is_exited() const
    {
        return is_code_ || ((code_ != SIGCONT) && (code_ != SIGSTOP));
    }

    Process::Process(pid_t pid)
            : pid_(pid)
    {
    }

    pid_t Process::get_pid() const
    {
        return pid_;
    }

    rust::Result<ExitStatus> Process::wait(bool request_for_signals)
    {
        spdlog::debug("Process wait requested. [pid: {}]", pid_);
        return wait_for(pid_, request_for_signals)
            .on_success([this](const auto&) {
                spdlog::debug("Process wait request: done. [pid: {}]", pid_);
            })
            .on_error([this](const auto& error) {
                spdlog::debug("Process wait request: failed. [pid: {}] {}", pid_, error.what());
            });
    }

    rust::Result<int> Process::kill(int num)
    {
        spdlog::debug("Process kill requested. [pid: {}, signum: {}]", pid_, num);
        return send_signal(pid_, num)
            .on_success([this](const auto&) {
                spdlog::debug("Process kill request: done. [pid: {}]", pid_);
            })
            .on_error([this](const auto& error) {
                spdlog::debug("Process kill request: failed. [pid: {}] {}", pid_, error.what());
            });
    }

    Process::Builder::Builder(fs::path program)
        : program_(std::move(program))
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

    rust::Result<fs::path> Process::Builder::resolve_executable()
    {
        return ::resolve_executable(program_, environment_);
    }

    rust::Result<Process> Process::Builder::spawn()
    {
        return reference_spawn_function()
            .and_then<Process>([this](auto fp) {
                return spawn_process(fp, program_, parameters_, environment_);
            });
    }

#ifdef SUPPORT_PRELOAD
    rust::Result<Process> Process::Builder::spawn_with_preload()
    {
        return resolve_spawn_function()
            .and_then<Process>([this](auto fp) {
                return spawn_process(fp, program_, parameters_, environment_);
            });
    }
#endif
}
