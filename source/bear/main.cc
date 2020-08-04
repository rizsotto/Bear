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
#include "libflags/Flags.h"
#include "libresult/Result.h"
#include "libsys/Environment.h"
#include "libsys/Process.h"
#include "libsys/Signal.h"

#include <spdlog/spdlog.h>
//#include <spdlog/fmt/ostr.h>
#include <spdlog/sinks/stdout_sinks.h>

#include <filesystem>
#include <optional>
#include <string_view>

namespace fs = std::filesystem;

namespace {

    constexpr std::optional<std::string_view> DEVELOPER_GROUP = { "developer options" };

    constexpr char VERBOSE[] = "-vvvv";
    constexpr char APPEND[] = "--append";
    constexpr char OUTPUT[] = "--cdb";
    constexpr char CITNAMES[] = "--citnames";
    constexpr char INTERCEPT[] = "--interceptor";
    constexpr char LIBRARY[] = "--libexec";
    constexpr char EXECUTOR[] = "--executor";
    constexpr char WRAPPER[] = "--wrapper";
    constexpr char COMMAND[] = "--";
    constexpr char FORCE_WRAPPER[] = "--force-wrapper";
    constexpr char FORCE_PRELOAD[] = "--force-preload";

    rust::Result<int> execute(sys::Process::Builder builder, const std::string_view& name)
    {
        return builder.spawn()
                .and_then<sys::ExitStatus>([](auto child) {
                    sys::SignalForwarder guard(child);
                    return child.wait();
                })
                .map<int>([](auto status) {
                    return status.code().value_or(EXIT_FAILURE);
                })
                .map_err<std::runtime_error>([&name](auto error) {
                    spdlog::warn("Running {} failed: {}", name, error.what());
                    return error;
                })
                .on_success([&name](auto status) {
                    spdlog::debug("Running {} finished. [Exited with {}]", name, status);
                });
    }

    rust::Result<sys::Process::Builder> prepare_intercept(const flags::Arguments& arguments, const sys::env::Vars& environment, const fs::path& output)
    {
        auto program = arguments.as_string(INTERCEPT);
        auto command = arguments.as_string_list(COMMAND);
        auto library = arguments.as_string(LIBRARY);
        auto executor = arguments.as_string(EXECUTOR);
        auto wrapper = arguments.as_string(WRAPPER);
        auto verbose = arguments.as_bool(VERBOSE).unwrap_or(false);

        return rust::merge(program, command, rust::merge(library, executor, wrapper))
                .map<sys::Process::Builder>([&environment, &output, &verbose](auto tuple) {
                    const auto& [program, command, pack] = tuple;
                    const auto& [library, executor, wrapper] = pack;

                    auto builder = sys::Process::Builder(program)
                            .set_environment(environment)
                            .add_argument(program);
                    builder.add_argument("--library")
                            .add_argument(library);
                    builder.add_argument("--executor")
                            .add_argument(executor);
                    builder.add_argument("--wrapper")
                            .add_argument(wrapper);
                    builder.add_argument("--output")
                            .add_argument(output);
                    if (verbose) {
                        builder.add_argument("--verbose");
                    }
                    builder.add_argument("--")
                            .add_arguments(command.begin(), command.end());
                    return builder;
                });
    }

    rust::Result<sys::Process::Builder> prepare_citnames(const flags::Arguments& arguments, const sys::env::Vars& environment, const fs::path& input)
    {
        auto program = arguments.as_string(CITNAMES);
        auto output = arguments.as_string(OUTPUT);
        auto append = arguments.as_bool(APPEND).unwrap_or(false);
        auto verbose = arguments.as_bool(VERBOSE).unwrap_or(false);

        return rust::merge(program, output)
                .map<sys::Process::Builder>([&environment, &input, &append, &verbose](auto tuple) {
                    const auto& [program, output] = tuple;

                    auto builder = sys::Process::Builder(program)
                            .set_environment(environment)
                            .add_argument(program)
                            .add_argument("--input")
                            .add_argument(input)
                            .add_argument("--output")
                            .add_argument(output);
                    if (append) {
                        builder.add_argument("--append");
                    }
                    if (verbose) {
                        builder.add_argument("--verbose");
                    }
                    return builder;
                });
    }

    rust::Result<int> run(const flags::Arguments& arguments, const sys::env::Vars& environment)
    {
        auto commands = fs::path("commands.json"); // TODO: derive the location from OUTPUT flag
        auto intercept = prepare_intercept(arguments, environment, commands);
        auto citnames = prepare_citnames(arguments, environment, commands);

        return rust::merge(intercept, citnames)
                .and_then<int>([](auto tuple) {
                    const auto& [intercept, citnames] = tuple;
                    auto result = execute(intercept, "intercept");
                    execute(citnames, "citnames");
                    return result;
                });
    }
}

int main(int argc, char* argv[], char* envp[])
{
    spdlog::set_default_logger(spdlog::stderr_logger_mt("stderr"));
    spdlog::set_pattern("bear: %v [pid: %P]");
    spdlog::set_level(spdlog::level::info);

    const flags::Parser parser("bear", VERSION,
                               { { VERBOSE, { 0, false, "run the interception verbose", std::nullopt, std::nullopt } },
                                 { APPEND, { 0, false, "append result to an existing output file", std::nullopt, std::nullopt } },
                                 { OUTPUT, { 1, false, "path of the result file", { "compile_commands.json" }, std::nullopt } },
                                 { FORCE_PRELOAD, { 0, false, "force to use library preload", std::nullopt, std::nullopt } },
                                 { FORCE_WRAPPER, { 0, false, "force to use compiler wrappers", std::nullopt, std::nullopt } },
                                 { LIBRARY, { 1, false, "path to the preload library", { LIBRARY_DEFAULT_PATH }, DEVELOPER_GROUP } },
                                 { EXECUTOR, { 1, false, "path to the preload executable", { EXECUTOR_DEFAULT_PATH }, DEVELOPER_GROUP } },
                                 { WRAPPER, { 1, false, "path to the wrapper directory", { WRAPPER_DEFAULT_PATH }, DEVELOPER_GROUP } },
                                 { CITNAMES, { 1, false, "path to the citnames executable", { CITNAMES_DEFAULT_PATH }, DEVELOPER_GROUP } },
                                 { INTERCEPT, { 1, false, "path to the intercept executable", { INTERCEPT_DEFAULT_PATH }, DEVELOPER_GROUP } },
                                 { COMMAND, { -1, true, "command to execute", std::nullopt, std::nullopt } } });
    return parser.parse_or_exit(argc, const_cast<const char**>(argv))
            // change the log verbosity if requested.
            .on_success([](const auto& args) {
                if (args.as_bool(VERBOSE).unwrap_or(false)) {
                    spdlog::set_pattern("[%H:%M:%S.%f, br, %P] %v");
                    spdlog::set_level(spdlog::level::debug);
                }
                spdlog::debug("arguments parsed: {}", args);
            })
            // if parsing success, we create the main command and execute it.
            .and_then<int>([&envp](auto args) {
                auto environment = sys::env::from(const_cast<const char **>(envp));
                return run(args, environment);
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
