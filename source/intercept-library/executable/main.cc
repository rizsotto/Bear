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

#include <iostream>

#include "Environment.h"
#include "Interface.h"
#include "Reporter.h"
#include "Result.h"
#include "Session.h"
#include "SystemCalls.h"

namespace {

    std::ostream& error_stream()
    {
        std::cerr << "intercept: [pid: "
                  << er::SystemCalls::get_pid().get_or_else(0)
                  << ", ppid: "
                  << er::SystemCalls::get_ppid().get_or_else(0)
                  << "] ";
        return std::cerr;
    }

    er::Result<pid_t> spawnp(const ::er::Execution& config,
        const ::er::EnvironmentPtr& environment) noexcept
    {
        return er::SystemCalls::spawn(config.path, config.command, environment->data());
    }

    void report_start(::er::Result<er::ReporterPtr> const& reporter, pid_t pid, const char** cmd) noexcept
    {
        ::er::merge(reporter, ::er::Event::start(pid, cmd))
            .bind<int>([](auto tuple) {
                const auto& [rptr, eptr] = tuple;
                return rptr->send(eptr);
            })
            .handle_with([](auto message) {
                error_stream() << message.what() << std::endl;
            })
            .get_or_else(0);
    }

    void report_exit(::er::Result<er::ReporterPtr> const& reporter, pid_t pid, int exit) noexcept
    {
        ::er::merge(reporter, ::er::Event::stop(pid, exit))
            .bind<int>([](auto tuple) {
                const auto& [rptr, eptr] = tuple;
                return rptr->send(eptr);
            })
            .handle_with([](auto message) {
                error_stream() << message.what() << std::endl;
            })
            .get_or_else(0);
    }

    ::er::EnvironmentPtr create_environment(char* original[], const ::er::SessionPtr& session)
    {
        auto builder = er::Environment::Builder(const_cast<const char**>(original));
        session->configure(builder);
        return builder.build();
    }

    std::ostream& operator<<(std::ostream& os, char* const* values)
    {
        os << '[';
        for (char* const* it = values; *it != nullptr; ++it) {
            if (it != values) {
                os << ", ";
            }
            os << '"' << *it << '"';
        }
        os << ']';

        return os;
    }

}

int main(int argc, char* argv[], char* envp[])
{
    return ::er::parse(argc, argv)
        .map<er::SessionPtr>([&argv](auto arguments) {
            if (arguments->context_.verbose) {
                error_stream() << argv << std::endl;
            }
            return arguments;
        })
        .bind<int>([&envp](auto arguments) {
            auto reporter = er::Reporter::tempfile(arguments->context_.destination);

            auto environment = create_environment(envp, arguments);
            return spawnp(arguments->execution_, environment)
                .template map<pid_t>([&arguments, &reporter](auto& pid) {
                    report_start(reporter, pid, arguments->execution_.command);
                    return pid;
                })
                .template bind<std::tuple<pid_t, int>>([](auto pid) {
                    return er::SystemCalls::wait_pid(pid)
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
            error_stream() << message.what() << std::endl;
        })
        .get_or_else(EXIT_FAILURE);
}
