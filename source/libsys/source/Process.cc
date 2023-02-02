/*  Copyright (C) 2012-2023 by László Nagy
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
#include "libsys/Path.h"
#include "libsys/Errors.h"
#include "Guard.h"

#include <cerrno>
#include <csignal>
#include <cstdlib>
#include <filesystem>
#include <utility>
#include <iostream>

#ifdef HAVE_SYS_STAT_H
#include <sys/stat.h>
#endif
#ifdef HAVE_SYS_WAIT_H
#include <sys/wait.h>
#endif
#ifdef HAVE_UNISTD_H
#include <unistd.h>
#endif
#ifdef HAVE_SPAWN_H
#include <spawn.h>
#endif
#ifdef HAVE_DLFCN_H
#include <dlfcn.h>
#endif
#ifdef HAVE_GNU_LIB_NAMES_H
#  include <gnu/lib-names.h>
#else
#  include "libsys/lib-names.h"
#endif

#include <fmt/ranges.h>
#include <spdlog/spdlog.h>
#include <spdlog/sinks/stdout_sinks.h>

namespace {

    constexpr char PATH_TO_SH[] = "/bin/sh";

    using posix_spawn_t = int (*)(
        pid_t * pid,
        const char* path,
        const posix_spawn_file_actions_t* file_actions_ptr,
        const posix_spawnattr_t* attr_ptr,
        char* const argv[],
        char* const envp[]);

#ifdef SUPPORT_PRELOAD
    rust::Result<posix_spawn_t> resolve_spawn_function()
    {
        errno = 0;
        void *handle = ::dlopen(LIBC_SO, RTLD_LAZY);
        if (handle == nullptr) {
            const auto message = fmt::format("System call \"dlopen\" failed: {}", ::dlerror());
            return rust::Err(std::runtime_error(message));
        }

        errno = 0;
        auto fp = reinterpret_cast<posix_spawn_t>(::dlsym(handle, "posix_spawnp"));
        if (fp == nullptr) {
            const auto message = fmt::format("System call \"dlsym\" failed: {}", ::dlerror());
            return rust::Err(std::runtime_error(message));
        }

        return rust::Ok(fp);
    }
#endif

    bool is_open(int fd) {
        struct stat stat_buf;

        errno = 0;
        if ((0 != fstat(fd, &stat_buf)) && (errno == EBADF)) {
            return false;
        }
        return true;
    }

    rust::Result<pid_t> spawn_process(
            posix_spawn_t fp,
            const fs::path& program,
            const std::list<std::string>& parameters,
            const std::map<std::string, std::string>& environment,
            const bool redirect_io)
    {
        // convert the arguments into a c-style array
        std::vector<char*> args;
        std::transform(parameters.begin(), parameters.end(),
                       std::back_insert_iterator(args),
                       [](const auto& arg) { return const_cast<char*>(arg.c_str()); });
        args.push_back(nullptr);
        // convert the environment into a c-style array
        sys::env::Guard env(environment);
        // deal with file handles
        posix_spawn_file_actions_t file_actions;
        posix_spawn_file_actions_t *file_actionsp = nullptr;
        if (redirect_io) {
            errno = 0;
            if (0 != posix_spawn_file_actions_init(&file_actions)) {
                const auto message = fmt::format("System call \"posix_spawn_file_actions_init\" failed: {}", sys::error_string(errno));
                return rust::Err(std::runtime_error(message));
            }
            for (int fd = 0; fd < 3; ++fd) {
                if (!is_open(fd)) {
                    errno = 0;
                    if (0 != posix_spawn_file_actions_addclose(&file_actions, fd)) {
                        const auto message = fmt::format("System call \"posix_spawn_file_actions_addclose\" failed: {}", sys::error_string(errno));
                        return rust::Err(std::runtime_error(message));
                    }
                }
            }
            file_actionsp = &file_actions;
        }

        pid_t child;
        errno = 0;
        if (0 != (*fp)(&child, program.c_str(), file_actionsp, nullptr, const_cast<char**>(args.data()), const_cast<char**>(env.data()))) {
            const auto message = fmt::format("System call \"posix_spawnp\" failed: {}", sys::error_string(errno));
            if (redirect_io) {
                posix_spawn_file_actions_destroy(&file_actions);
            }
            return rust::Err(std::runtime_error(message));
        } else {
            if (redirect_io) {
                posix_spawn_file_actions_destroy(&file_actions);
            }
            return rust::Ok(child);
        }
    }


    rust::Result<pid_t> spawn_process_with_retry(
        posix_spawn_t fp,
        const fs::path& program,
        const std::list<std::string>& parameters,
        const std::map<std::string, std::string>& environment,
        const bool redirect_io)
    {
        return spawn_process(fp, program, parameters, environment, redirect_io)
                // The file is accessible, but it is not an executable file.
                // Invoke the shell to interpret it as a script.
                .or_else([&](const std::runtime_error&) {
                    spdlog::debug("Process spawn failed. [will retry as shell]");

                    std::list<std::string> args(parameters);
                    args.insert(args.begin(), std::string(PATH_TO_SH));
                    return spawn_process(fp, PATH_TO_SH, args, environment, redirect_io);
                })
                .on_success([&parameters](const auto& pid) {
                    spdlog::debug("Process spawned. [pid: {}, command: {}]", pid, parameters);
                })
                .on_error([&parameters](const auto& error) {
                    spdlog::debug("Process spawn failed. [error: {}, command: {}]", error.what(), parameters);
                });
    }

    rust::Result<sys::ExitStatus> wait_for(const pid_t pid, const bool request_for_signals)
    {
        const int mask = request_for_signals ? (WUNTRACED | WCONTINUED) : 0;
        errno = 0;
        if (int status; -1 != ::waitpid(pid, &status, mask)) {
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

    rust::Result<int> send_signal(const pid_t pid, const int num)
    {
        errno = 0;
        if (const int result = ::kill(pid, num); 0 == result) {
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

    rust::Result<ExitStatus> Process::wait(const bool request_for_signals)
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

    Process::Builder::Builder(fs::path program, bool with_preload)
        : program_(std::move(program))
        , with_preload_(with_preload)
        , parameters_()
        , environment_()
        , redirect_io_(false)
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

    Process::Builder& Process::Builder::set_redirect_io() {
        redirect_io_ = true;
        return *this;
    }

    rust::Result<Process> Process::Builder::spawn() const
    {
#ifdef SUPPORT_PRELOAD
        const rust::Result<posix_spawn_t> fp = with_preload_
            ? resolve_spawn_function()
            : rust::Ok(&::posix_spawn);
#else
        const rust::Result<posix_spawn_t> fp = rust::Ok(&::posix_spawn);
#endif

        return fp
            .and_then<pid_t>([this](auto fp) {
                return spawn_process_with_retry(fp, program_, parameters_, environment_, redirect_io_);
            })
            .map<Process>([](auto pid) {
                return Process(pid);
            });
    }
}
