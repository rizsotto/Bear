/*  Copyright (C) 2012-2017 by László Nagy
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

#include <unistd.h>
#include <wait.h>
#include <spawn.h>

#include <cerrno>
#include <cstdlib>
#include <cstdio>
#include <cstring>

#include "Result.h"
#include "Environment.h"
#include "Reporter.h"


template <typename T>
pear::Result<T> failure(const char *message) noexcept {
    return pear::Result<T>::failure(std::string(message));
};

template <typename T>
pear::Result<T> failure(const char *message, int errnum) noexcept {
    std::string result = message != nullptr ? std::string(message) : std::string();

    const size_t buffer_length = 1024 + strlen(message);
    char buffer[buffer_length] = { ':', ' ', '\0' };
    if (0 == strerror_r(errnum, buffer + 2, buffer_length - 2)) {
        result += std::string(buffer);
    } else {
        result += std::string(": Couldn't get error message.");
    }
    return pear::Result<T>::failure(result);
};

struct EarLibraryConfig {
    char *wrapper;
    char *library;
    char *target;
};

struct ExecutionConfig {
    const char **command;
    char *method;
    char *file;
    char *search_path;
};

struct Arguments {
    EarLibraryConfig forward;
    ExecutionConfig execution;
};

pear::Result<Arguments> parse(int argc, char *argv[]) {
    Arguments result = {nullptr, nullptr, nullptr, nullptr};

    int opt;
    while ((opt = getopt(argc, argv, "t:l:m:f:s:")) != -1) {
        switch (opt) {
            case 't':
                result.forward.target = optarg;
                break;
            case 'l':
                result.forward.library = optarg;
                break;
            case 'm':
                result.execution.method = optarg;
                break;
            case 'f':
                result.execution.file = optarg;
                break;
            case 's':
                result.execution.search_path = optarg;
                break;
            default: /* '?' */
                return failure<Arguments>(
                        "Usage: pear [-t target_url]\n"
                        "            [-l path_to_libear]\n"
                        "            [-m method]\n"
                        "            [-f file]\n"
                        "            [-s search_path]\n"
                        "            -- command");
        }
    }

    if (optind >= argc) {
        return failure<Arguments>("Expected argument after options");
    } else {
        result.forward.wrapper = argv[0];
        result.execution.command = const_cast<const char **>(argv + optind);
        return pear::Result<Arguments>::success(std::move(result));
    }
}

pear::Result<pid_t> spawn(const ExecutionConfig &config,
                          const pear::EnvironmentPtr &environment) noexcept {
    char **envp = const_cast<char **>(environment->as_array());
    char **argv = const_cast<char **>(config.command);

    // TODO: use other execution config parameters.

    pid_t child;
    if (0 != posix_spawn(&child, config.command[0], 0, 0, argv, envp)) {
        return failure<pid_t>("posix_spawn", errno);
    } else {
        return pear::Result<pid_t>::success(std::move(child));
    }
}

pear::Result<int> wait_pid(pid_t pid) noexcept {
    int status;
    if (-1 == waitpid(pid, &status, 0)) {
        return failure<int>("waitpid", errno);
    } else {
        int result = WIFEXITED(status) ? WEXITSTATUS(status) : EXIT_FAILURE;
        return pear::Result<int>::success(std::move(result));
    }
}

void report_start(pid_t pid, const char **cmd, pear::ReporterPtr reporter) noexcept {
    pear::Event::start(pid, cmd)
            .map<int>([&reporter](auto &eptr) {
                return reporter->send(eptr)
                        .handle_with([](auto &message) {
                            fprintf(stderr, "%s\n", message);
                        })
                        .get_or_else(0);
            });
}

void report_exit(pid_t pid, int exit, pear::ReporterPtr reporter) noexcept {
    pear::Event::stop(pid, exit)
            .map<int>([&reporter](auto &eptr) {
                return reporter->send(eptr)
                        .handle_with([](auto &message) {
                            fprintf(stderr, "%s\n", message);
                        })
                        .get_or_else(0);
            });
}

int main(int argc, char *argv[], char *envp[]) {
    return parse(argc, argv)
            .bind<int>([&envp](auto &state) {
                auto environment = pear::Environment::Builder(const_cast<const char **>(envp))
                        .add_library(state.forward.library)
                        .add_target(state.forward.target)
                        .add_wrapper(state.forward.wrapper)
                        .build();
                auto reporter = pear::Reporter::tempfile(state.forward.target);

                pear::Result<pid_t> child = spawn(state.execution, environment);
                return child.map<int>([&reporter, &state](auto &pid) {
                    report_start(pid, state.execution.command, reporter);
                    pear::Result<int> status = wait_pid(pid);
                    return status
                            .map<int>([&reporter, &pid](auto &exit) {
                                report_exit(pid, exit, reporter);
                                return exit;
                            })
                            .get_or_else(EXIT_FAILURE);
                });
            })
            .handle_with([](auto const &message) {
                fprintf(stderr, "%s\n", message);
                exit(EXIT_FAILURE);
            })
            .get_or_else(EXIT_FAILURE);
}
