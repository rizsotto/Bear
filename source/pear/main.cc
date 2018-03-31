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

#include <cstdlib>
#include <cstdio>
#include <cstring>

#include "libpear_a/Parameters.h"
#include "libpear_a/Result.h"
#include "libpear_a/Environment.h"
#include "libpear_a/Reporter.h"
#include "libpear_a/SystemCalls.h"


pear::Result<pear::Parameters> parse(int argc, char *argv[]) noexcept {
    pear::Parameters result;

    int opt;
    while ((opt = getopt(argc, argv, "t:l:f:s:")) != -1) {
        switch (opt) {
            case 't':
                result.target.destination = optarg;
                break;
            case 'l':
                result.library = optarg;
                break;
            case 'f':
                result.execution.file = optarg;
                break;
            case 's':
                result.execution.search_path = optarg;
                break;
            default: /* '?' */
                return pear::Result<pear::Parameters>::failure(
                        std::runtime_error(
                                "Usage: pear [OPTION]... -- command\n\n"
                                "  -t <target url>       where to send execution reports\n"
                                "  -l <path to libear>   where to find the ear libray\n"
                                "  -f <file>             file parameter\n"
                                "  -s <search_path>      search path parameter\n"));
        }
    }

    if (optind >= argc) {
        return pear::Result<pear::Parameters>::failure(
                std::runtime_error(
                        "Usage: pear [OPTION]... -- command\n"
                                "Expected argument after options"));
    } else {
        // TODO: do validation!!!
        result.wrapper = argv[0];
        result.execution.command = const_cast<const char **>(argv + optind);
        return pear::Result<pear::Parameters>::success(std::move(result));
    }
}

pear::Result<pid_t> spawnp(const pear::Parameters::Execution &config,
                           const pear::EnvironmentPtr &environment) noexcept {
    // TODO: use other execution config parameters.

    return pear::spawnp(config.command, environment->as_array());
}

void report_start(pid_t pid, const char **cmd, pear::ReporterPtr reporter) noexcept {
    pear::Event::start(pid, cmd)
            .map<int>([&reporter](auto &eptr) {
                return reporter->send(eptr)
                        .handle_with([](auto &message) {
                            fprintf(stderr, "%s\n", message.what());
                        })
                        .get_or_else(0);
            });
}

void report_exit(pid_t pid, int exit, pear::ReporterPtr reporter) noexcept {
    pear::Event::stop(pid, exit)
            .map<int>([&reporter](auto &eptr) {
                return reporter->send(eptr)
                        .handle_with([](auto &message) {
                            fprintf(stderr, "%s\n", message.what());
                        })
                        .get_or_else(0);
            });
}

int main(int argc, char *argv[], char *envp[]) {
    return parse(argc, argv)
            .bind<int>([&envp](auto &state) {
                auto environment = pear::Environment::Builder(const_cast<const char **>(envp))
                        .add_target(state.target.destination)
                        .add_library(state.library)
                        .add_wrapper(state.wrapper)
                        .build();
                auto reporter = pear::Reporter::tempfile(state.target.destination);

                pear::Result<pid_t> child = spawnp(state.execution, environment);
                return child.map<int>([&reporter, &state](auto &pid) {
                    report_start(pid, state.execution.command, reporter);
                    pear::Result<int> status = pear::wait_pid(pid);
                    return status
                            .map<int>([&reporter, &pid](auto &exit) {
                                report_exit(pid, exit, reporter);
                                return exit;
                            })
                            .get_or_else(EXIT_FAILURE);
                });
            })
            .handle_with([](auto const &message) {
                fprintf(stderr, "%s\n", message.what());
                exit(EXIT_FAILURE);
            })
            .get_or_else(EXIT_FAILURE);
}
