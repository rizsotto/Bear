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

#include <iostream>

#include "intercept_a/Interface.h"
#include "intercept_a/Session.h"
#include "intercept_a/Result.h"
#include "intercept_a/Environment.h"
#include "intercept_a/Reporter.h"
#include "intercept_a/SystemCalls.h"


namespace {

    constexpr char MESSAGE_PREFIX[] = "intercept: ";

    pear::Result<pid_t> spawnp(const ::pear::Execution &config,
                               const ::pear::EnvironmentPtr &environment) noexcept {
        if ((config.search_path != nullptr) && (config.file != nullptr)) {
            return pear::SystemCalls::fork_with_execvp(config.file, config.search_path, config.command, environment->data());
        } else if (config.file != nullptr) {
            return pear::SystemCalls::spawnp(config.file, config.command, environment->data());
        } else {
            return pear::SystemCalls::spawn(config.path, config.command, environment->data());
        }
    }

    void report_start(::pear::Result<pear::ReporterPtr> const &reporter, pid_t pid, const char **cmd) noexcept {
        ::pear::merge(reporter, ::pear::Event::start(pid, cmd))
                .bind<int>([](auto tuple) {
                    const auto& [ rptr, eptr ] = tuple;
                    return rptr->send(eptr);
                })
                .handle_with([](auto message) {
                    std::cerr << MESSAGE_PREFIX << message.what() << std::endl;
                })
                .get_or_else(0);
    }

    void report_exit(::pear::Result<pear::ReporterPtr> const &reporter, pid_t pid, int exit) noexcept {
        ::pear::merge(reporter, ::pear::Event::stop(pid, exit))
                .bind<int>([](auto tuple) {
                    const auto& [ rptr, eptr ] = tuple;
                    return rptr->send(eptr);
                })
                .handle_with([](auto message) {
                    std::cerr << MESSAGE_PREFIX << message.what() << std::endl;
                })
                .get_or_else(0);
    }

    ::pear::EnvironmentPtr create_environment(char *original[], const ::pear::SessionPtr &session) {
        auto builder = pear::Environment::Builder(const_cast<const char **>(original));
        session->configure(builder);
        return builder.build();
    }

    std::ostream &operator<<(std::ostream &os, char *const *values) {
        os << '[';
        for (char *const *it = values; *it != nullptr; ++it) {
            if (it != values) {
                os << ", ";
            }
            os << '"' << *it << '"';
        }
        os << ']';

        return os;
    }

}

int main(int argc, char *argv[], char *envp[]) {
    return ::pear::parse(argc, argv)
            .map<pear::SessionPtr>([&argv](auto arguments) {
                if (arguments->context_.verbose) {
                    std::cerr << MESSAGE_PREFIX << argv << std::endl;
                }
                return arguments;
            })
            .bind<int>([&envp](auto arguments) {
                auto reporter = pear::Reporter::tempfile(arguments->context_.destination);

                auto environment = create_environment(envp, arguments);
                return spawnp(arguments->execution_, environment)
                        .template map<pid_t>([&arguments, &reporter](auto &pid) {
                            report_start(reporter, pid, arguments->execution_.command);
                            return pid;
                        })
                        .template bind<std::tuple<pid_t, int>>([](auto pid) {
                            return pear::SystemCalls::wait_pid(pid)
                                    .template map<std::tuple<pid_t, int>>([&pid](auto exit) {
                                        return std::make_tuple(pid, exit);
                                    });
                        })
                        .template map<int>([&reporter](auto tuple) {
                            const auto& [pid, exit] = tuple;
                            report_exit(reporter, pid, exit);
                            return exit;
                        });
            })
            .handle_with([](auto message) {
                std::cerr << MESSAGE_PREFIX << message.what() << std::endl;
            })
            .get_or_else(EXIT_FAILURE);
}
