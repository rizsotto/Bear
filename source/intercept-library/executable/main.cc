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

#include "Command.h"
#include "Flags.h"
#include "SystemCalls.h"
#include "er.h"
#include "config.h"

#include <spdlog/spdlog.h>
#include <spdlog/fmt/ostr.h>
#include <spdlog/sinks/stdout_sinks.h>

#include <iostream>
#include <optional>

namespace {

    struct Arguments {
        char *const * values;
    };

    std::ostream& operator<<(std::ostream& os, const Arguments& arguments)
    {
        os << '[';
        for (char* const* it = arguments.values; *it != nullptr; ++it) {
            if (it != arguments.values) {
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
    int const pid = er::SystemCalls::get_pid().unwrap_or(0);
    int const ppid = er::SystemCalls::get_ppid().unwrap_or(0);
    spdlog::set_pattern(fmt::format("er: [pid: {0}, ppid: {1}] %v", pid, ppid));
    spdlog::set_level(spdlog::level::info);

    const flags::Parser parser("er", VERSION,
        { { ::er::flags::VERBOSE, { 0, false, "make the interception run verbose", std::nullopt, std::nullopt } },
            { ::er::flags::DESTINATION, { 1, true, "path to report directory", std::nullopt, std::nullopt } },
            { ::er::flags::LIBRARY, { 1, true, "path to the intercept library", std::nullopt, std::nullopt } },
            { ::er::flags::EXECUTE, { 1, true, "the path parameter for the command", std::nullopt, std::nullopt } },
            { ::er::flags::COMMAND, { -1, true, "the executed command", std::nullopt, std::nullopt } } });
    return parser.parse_or_exit(argc, const_cast<const char**>(argv))
        // log the original command line as it was received.
        .map<flags::Arguments>([&argv](const auto& args) {
            if (args.as_bool(::er::flags::VERBOSE).unwrap_or(false)) {
                spdlog::set_level(spdlog::level::debug);
                spdlog::debug("arguments: {}", Arguments { argv });
            }
            return args;
        })
        // if parsing success, we create the main command and execute it.
        .and_then<er::Command>([](auto args) {
            return er::Command::create(args);
        })
        .and_then<int>([&envp](const auto& command) {
            const char** environment = const_cast<const char**>(envp);
            return command(environment);
        })
        // set the return code from error and print message
        .unwrap_or_else([](auto error) {
            return EXIT_FAILURE;
        });
}
