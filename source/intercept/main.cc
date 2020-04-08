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

// How it should works?
//
// - Create communication channel for `er` to report process execution
//   - Listens to the channel and collect the received reports
// - Choose interception mode (wrapper or preload)
//   - Set up environment variables accordingly
//   - Execute the build command.
//   - Wait until the child process terminates. (store exit code)
// - Close communication channel.
// - Writes output.
// - Return child exit code.

#include "Application.h"
#include "Flags.h"
#include "config.h"

#include <spdlog/spdlog.h>

#include <optional>
#include <string_view>

namespace {

    constexpr std::optional<std::string_view> DEVELOPER_GROUP = { "developer options" };
}

int main(int argc, char* argv[])
{
    spdlog::set_pattern("intercept [pid: %P, level: %l] %v");
    spdlog::set_level(spdlog::level::info);

    const flags::Parser parser("intercept", VERSION,
        { { ic::Application::VERBOSE, { 0, false, "run the interception verbose", std::nullopt, std::nullopt } },
            { ic::Application::OUTPUT, { 1, false, "path of the result file", { "commands.json" }, std::nullopt } },
            { ic::Application::LIBRARY, { 1, false, "path to the preload library", { LIBRARY_DEFAULT_PATH }, DEVELOPER_GROUP } },
            { ic::Application::EXECUTOR, { 1, false, "path to the preload executable", { EXECUTOR_DEFAULT_PATH }, DEVELOPER_GROUP } },
            { ic::Application::WRAPPER, { 1, false, "path to the wrapper executable", { WRAPPER_DEFAULT_PATH }, DEVELOPER_GROUP } },
            { ic::Application::COMMAND, { -1, true, "command to execute", std::nullopt, std::nullopt } } });
    return parser.parse_or_exit(argc, const_cast<const char**>(argv))
        // change the log verbosity if requested.
        .map<flags::Arguments>([&argv](const auto& args) {
            if (args.as_bool(ic::Application::VERBOSE).unwrap_or(false)) {
                spdlog::set_level(spdlog::level::debug);
                // TODO: log the parsed arguments at debug level
            }
            return args;
        })
        // if parsing success, we create the main command and execute it.
        .and_then<ic::Application>([](auto args) {
            return ic::Application::from(args);
        })
        .and_then<int>([](const auto& command) {
            return command();
        })
        // set the return code from error and print message
        .unwrap_or_else([](auto error) {
            return EXIT_FAILURE;
        });
}
