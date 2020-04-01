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

#include "Command.h"
#include "Flags.h"

#include <spdlog/spdlog.h>

#include <optional>
#include <string_view>

namespace {

    constexpr char VERSION[] = _INTERCEPT_VERSION;

    constexpr char LIBRARY_PATH[] = _LIBRARY_PATH;
    constexpr char EXECUTOR_PATH[] = _EXECUTOR_PATH;
    constexpr char WRAPPER_PATH[] = _WRAPPER_PATH;

    constexpr std::optional<std::string_view> DEVELOPER_GROUP = { "developer options" };
}

int main(int argc, char* argv[])
{
    spdlog::set_pattern("intercept [pid: %P, level: %l] %v");
    spdlog::set_level(spdlog::level::info);

    const flags::Parser parser("intercept", VERSION,
        { { "--verbose", { 0, false, "run the interception verbose", std::nullopt, std::nullopt } },
            { "--output", { 1, false, "path of the result file", { "commands.json" }, std::nullopt } },
            { "--library", { 1, false, "path to the preload library", { LIBRARY_PATH }, DEVELOPER_GROUP } },
            { "--executor", { 1, false, "path to the preload executable", { EXECUTOR_PATH }, DEVELOPER_GROUP } },
            { "--wrapper", { 1, false, "path to the wrapper executable", { WRAPPER_PATH }, DEVELOPER_GROUP } },
            { "--", { -1, true, "command to execute", std::nullopt, std::nullopt } } });
    return parser.parse_or_exit(argc, const_cast<const char**>(argv))
        // if parsing success, we create the main command and execute it.
        .and_then<ic::Command>([](auto args) {
            return ic::Command::create(args);
        })
        .and_then<int>([](const auto& command) {
            return command();
        })
        // set the return code from error and print message
        .unwrap_or_else([](auto error) {
            return EXIT_FAILURE;
        });
}
