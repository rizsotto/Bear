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
// - Choose interception mode (wrapper or preload)
//   - Set up environment variables accordingly
// - Create communication channel for `er` to report process execution
//   - Listens to the channel and collect the received reports
// - Calls `er` to supervise the build (with the received command).
//   - Wait until the child process terminates. (store exit code)
// - Close communication channel.
// - Writes output.
// - Return child exit code.
//
// Communication channel means: filesystem or socket. Do migration easy,
// start with the filesystem (means create a temporary directory and
// delete it when everything is finished). This can be later changed to
// UNIX or TCP sockets.

#include <iostream>
#include <string>

#include "Command.h"
#include "Flags.h"

namespace {

    constexpr char VERSION[] = INTERCEPT_VERSION;

    struct Error {
        int code;
        std::string message;
    };
}

int main(int argc, char* argv[])
{
    const flags::Parser parser("intercept",
        { { "--help", { 0, false, "this message", std::nullopt } },
            { "--version", { 0, false, "print version and exit", std::nullopt } },
            { "--verbose", { 0, false, "make the interception run verbose", std::nullopt } },
            { "--output", { 1, false, "where the result shall be written", { "commands.json" } } },
            // { ::er::flags::LIBRARY, { 1, false, "path to the intercept library", std::nullopt } },
            // { ::er::flags::EXECUTE, { 1, false, "the path parameter for the command", std::nullopt } },
            { "--", { -1, false, "the executed command", std::nullopt } } });
    return parser.parse(argc, const_cast<const char**>(argv))
        // if parsing fail, set the return value and fall through
        .map_err<Error>([](auto error) {
            return Error { EXIT_FAILURE, std::string(error.what()) };
        })
        // if parsing success, check for the `--help` and `--version` flags
        .and_then<flags::Arguments>([&parser](auto args) {
            // print help message and exit zero
            if (args.as_bool("--help").unwrap_or(false)) {
                parser.print_help(std::cout, true);
                Error error = { EXIT_SUCCESS, std::string() };
                return rust::Result<flags::Arguments, Error>(rust::Err(error));
            }
            // print version message and exit zero
            if (args.as_bool("--version").unwrap_or(false)) {
                std::cout << "intercept " << VERSION << std::endl;
                return rust::Result<flags::Arguments, Error>(rust::Err(Error { EXIT_SUCCESS, std::string() }));
            }
            return rust::Result<flags::Arguments, Error>(rust::Ok(args));
        })
        // if parsing success, we create the main command and execute it
        .and_then<int>([](auto args) {
            return ic::create(args)
                .template and_then<int>([](auto command) {
                    return command();
                })
                .template map_err<Error>([](auto error) {
                    return Error { EXIT_FAILURE, error.what() };
                });
        })
        // set the return code from error and print message
        .unwrap_or_else([&parser](auto error) {
            if (error.code != EXIT_SUCCESS) {
                std::cerr << error.message << std::endl;
                parser.print_help_short(std::cerr);
            }
            return error.code;
        });
}
