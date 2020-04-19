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

#include "Application.h"
#include "config.h"
#include "er/Flags.h"
#include "libflags/Flags.h"
#include "libsys/Context.h"

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
    const sys::Context ctx;

    spdlog::set_pattern(fmt::format("er: [pid: {0}, ppid: {1}] %v", ctx.get_pid(), ctx.get_ppid()));
    spdlog::set_level(spdlog::level::info);

    const flags::Parser parser("er", VERSION,
        { { ::er::flags::VERBOSE, { 0, false, "make the interception run verbose", std::nullopt, std::nullopt } },
            { ::er::flags::DESTINATION, { 1, true, "path to report directory", std::nullopt, std::nullopt } },
            { ::er::flags::EXECUTE, { 1, true, "the path parameter for the command", std::nullopt, std::nullopt } },
            { ::er::flags::COMMAND, { -1, true, "the executed command", std::nullopt, std::nullopt } } });
    return parser.parse_or_exit(argc, const_cast<const char**>(argv))
        // log the original command line as it was received.
        .on_success([&argv](const auto& args) {
            if (args.as_bool(::er::flags::VERBOSE).unwrap_or(false)) {
                spdlog::set_level(spdlog::level::debug);
                spdlog::debug("arguments: {}", Arguments { argv });
            }
        })
        // if parsing success, we create the main command and execute it.
        .and_then<er::Application>([&ctx](auto args) {
            return er::Application::create(args, ctx);
        })
        .and_then<int>([&envp](const auto& command) {
            return command();
        })
        // print out the result of the run
        .on_error([](auto error) {
            spdlog::error(fmt::format("failed with: {}", error.what()));
        })
        .on_success([](auto status_code) {
            spdlog::debug(fmt::format("succeeded with: {}", status_code));
        })
        // set the return code from error
        .unwrap_or_else([](auto error) {
            return EXIT_FAILURE;
        });
}
