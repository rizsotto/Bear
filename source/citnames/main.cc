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

#include "config.h"
#include "Application.h"

#include <spdlog/spdlog.h>
#include <spdlog/fmt/ostr.h>
#include <spdlog/sinks/stdout_sinks.h>

#include <optional>


int main(int argc, char* argv[], char* envp[])
{
    spdlog::set_default_logger(spdlog::stderr_logger_mt("stderr"));
    spdlog::set_pattern("citnames: %v");
    spdlog::set_level(spdlog::level::info);

    const flags::Parser parser("citnames", VERSION,
                               { { cs::Application::VERBOSE, { 0, false, "run the application verbose", std::nullopt, std::nullopt } },
                                 { cs::Application::OUTPUT, { 1, false, "path of the result file", { "compile_commands.json" }, std::nullopt } },
                                 { cs::Application::INPUT, { 1, false, "path of the input file", { "commands.json" }, std::nullopt } },
                                 { cs::Application::INCLUDE, { 1, false, "directory where from source file shall be in the output", std::nullopt, std::nullopt } },
                                 { cs::Application::EXCLUDE, { 1, false, "directory where from source file shall not be in the output", std::nullopt, std::nullopt } },
                                 { cs::Application::APPEND, { 0, false, "append to output, instead of overwrite it", std::nullopt, std::nullopt } },
                                 { cs::Application::RUN_CHECKS, { 0, false, "can run checks on the current host", std::nullopt, std::nullopt } }
                               });
    return parser.parse_or_exit(argc, const_cast<const char**>(argv))
            // change the log verbosity if requested.
            .on_success([](const auto& args) {
                if (args.as_bool(cs::Application::VERBOSE).unwrap_or(false)) {
                    spdlog::set_pattern("[%H:%M:%S.%f, cs, %P] %v");
                    spdlog::set_level(spdlog::level::debug);
                }
                spdlog::debug("citnames: {}", VERSION);
                spdlog::debug("arguments parsed: {}", args);
            })
            // if parsing success, we create the main command and execute it.
            .and_then<cs::Application>([&envp](auto args) {
                auto environment = sys::env::from(const_cast<const char **>(envp));
                return cs::Application::from(args, std::move(environment));
            })
            .and_then<int>([](const auto& command) {
                return command();
            })
            // print out the result of the run
            .on_error([](auto error) {
                spdlog::error("failed with: {}", error.what());
            })
            .on_success([](auto status_code) {
                spdlog::debug("succeeded with: {}", status_code);
            })
            // set the return code from error
            .unwrap_or(EXIT_FAILURE);
}
