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
#include "citnames/Flags.h"
#include "intercept/Flags.h"
#include "libflags/Flags.h"
#include "libresult/Result.h"
#include "libsys/Environment.h"
#include "libsys/Process.h"
#include "libsys/Signal.h"
#include "libmain/ApplicationFromArgs.h"
#include "libmain/main.h"

#include <spdlog/spdlog.h>
#include <spdlog/sinks/stdout_sinks.h>

#include <filesystem>
#include <optional>
#include <string_view>
#include <utility>

namespace fs = std::filesystem;

namespace {

    constexpr std::optional<std::string_view> ADVANCED_GROUP = {"advanced options"};
    constexpr std::optional<std::string_view> DEVELOPER_GROUP = {"developer options"};

    constexpr char APPEND[] = "--append";
    constexpr char OUTPUT[] = "--output";
    constexpr char CITNAMES[] = "--citnames";
    constexpr char INTERCEPT[] = "--interceptor";
    constexpr char LIBRARY[] = "--libexec";
    constexpr char EXECUTOR[] = "--executor";
    constexpr char WRAPPER[] = "--wrapper";
    constexpr char CONFIG[] = "--config";
    constexpr char FORCE_WRAPPER[] = "--force-wrapper";
    constexpr char FORCE_PRELOAD[] = "--force-preload";
    constexpr char COMMAND[] = "--";

    rust::Result<sys::Process::Builder>
    prepare_intercept(const flags::Arguments &arguments, const sys::env::Vars &environment, const fs::path &output) {
        auto program = arguments.as_string(INTERCEPT);
        auto command = arguments.as_string_list(COMMAND);
        auto library = arguments.as_string(LIBRARY);
        auto executor = arguments.as_string(EXECUTOR);
        auto wrapper = arguments.as_string(WRAPPER);
        auto verbose = arguments.as_bool(flags::VERBOSE).unwrap_or(false);
        auto force_wrapper = arguments.as_bool(FORCE_WRAPPER).unwrap_or(false);
        auto force_preload = arguments.as_bool(FORCE_PRELOAD).unwrap_or(false);

        return rust::merge(program, command, rust::merge(library, executor, wrapper))
                .map<sys::Process::Builder>(
                        [&environment, &output, &verbose, &force_wrapper, &force_preload](auto tuple) {
                            const auto&[program, command, pack] = tuple;
                            const auto&[library, executor, wrapper] = pack;

                            auto builder = sys::Process::Builder(program)
                                    .set_environment(environment)
                                    .add_argument(program)
                                    .add_argument(ic::LIBRARY).add_argument(library)
                                    .add_argument(ic::EXECUTOR).add_argument(executor)
                                    .add_argument(ic::WRAPPER).add_argument(wrapper)
                                    .add_argument(ic::OUTPUT).add_argument(output);
                            if (force_wrapper) {
                                builder.add_argument(ic::FORCE_WRAPPER);
                            }
                            if (force_preload) {
                                builder.add_argument(ic::FORCE_PRELOAD);
                            }
                            if (verbose) {
                                builder.add_argument(flags::VERBOSE);
                            }
                            builder.add_argument(ic::COMMAND)
                                    .add_arguments(command.begin(), command.end());
                            return builder;
                        });
    }

    rust::Result<sys::Process::Builder>
    prepare_citnames(const flags::Arguments &arguments, const sys::env::Vars &environment, const fs::path &input) {
        auto program = arguments.as_string(CITNAMES);
        auto output = arguments.as_string(OUTPUT);
        auto config = arguments.as_string(CONFIG);
        auto append = arguments.as_bool(APPEND).unwrap_or(false);
        auto verbose = arguments.as_bool(flags::VERBOSE).unwrap_or(false);

        return rust::merge(program, output)
                .map<sys::Process::Builder>([&environment, &input, &config, &append, &verbose](auto tuple) {
                    const auto&[program, output] = tuple;

                    auto builder = sys::Process::Builder(program)
                            .set_environment(environment)
                            .add_argument(program)
                            .add_argument(cs::INPUT).add_argument(input)
                            .add_argument(cs::OUTPUT).add_argument(output)
                            // can run the file checks, because we are on the host.
                            .add_argument(cs::RUN_CHECKS);
                    if (append) {
                        builder.add_argument(cs::APPEND);
                    }
                    if (config.is_ok()) {
                        builder.add_argument(cs::CONFIG).add_argument(config.unwrap());
                    }
                    if (verbose) {
                        builder.add_argument(flags::VERBOSE);
                    }
                    return builder;
                });
    }

    rust::Result<int> execute(sys::Process::Builder builder, const std::string_view &name) {
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

    struct Command : ps::Command {
    public:
        Command(sys::Process::Builder intercept, sys::Process::Builder citnames, fs::path output) noexcept
                : ps::Command()
                , intercept_(std::move(intercept))
                , citnames_(std::move(citnames))
                , output_(std::move(output))
        { }

        [[nodiscard]] rust::Result<int> execute() const override
        {
            auto result = ::execute(intercept_, "intercept");

            std::error_code error_code;
            if (fs::exists(output_, error_code)) {
                ::execute(citnames_, "citnames");
                fs::remove(output_, error_code);
            }
            return result;
        }

    private:
        sys::Process::Builder intercept_;
        sys::Process::Builder citnames_;
        fs::path output_;
    };

    struct Application : ps::ApplicationFromArgs {
        Application()
                : ps::ApplicationFromArgs(ps::ApplicationLogConfig("bear", "br"))
        { }

        rust::Result<flags::Arguments> parse(int argc, const char **argv) const override
        {
            const flags::Parser parser(
                    "bear",
                    VERSION,
                    {
                            {OUTPUT,        {1,  false, "path of the result file",                  {"compile_commands.json"}, std::nullopt}},
                            {APPEND,        {0,  false, "append result to an existing output file", std::nullopt,              ADVANCED_GROUP}},
                            {CONFIG,        {1,  false, "path of the config file",                  std::nullopt,              ADVANCED_GROUP}},
                            {FORCE_PRELOAD, {0,  false, "force to use library preload",             std::nullopt,              ADVANCED_GROUP}},
                            {FORCE_WRAPPER, {0,  false, "force to use compiler wrappers",           std::nullopt,              ADVANCED_GROUP}},
                            {LIBRARY,       {1,  false, "path to the preload library",              {LIBRARY_DEFAULT_PATH},    DEVELOPER_GROUP}},
                            {EXECUTOR,      {1,  false, "path to the preload executable",           {EXECUTOR_DEFAULT_PATH},   DEVELOPER_GROUP}},
                            {WRAPPER,       {1,  false, "path to the wrapper directory",            {WRAPPER_DEFAULT_PATH},    DEVELOPER_GROUP}},
                            {CITNAMES,      {1,  false, "path to the citnames executable",          {CITNAMES_DEFAULT_PATH},   DEVELOPER_GROUP}},
                            {INTERCEPT,     {1,  false, "path to the intercept executable",         {INTERCEPT_DEFAULT_PATH},  DEVELOPER_GROUP}},
                            {COMMAND,       {-1, true,  "command to execute",                       std::nullopt,              std::nullopt}}
                    });
            return parser.parse_or_exit(argc, const_cast<const char **>(argv));
        }

        rust::Result<ps::CommandPtr> command(const flags::Arguments &args, const char **envp) const override
        {
            auto commands = args.as_string(OUTPUT)
                    .map<fs::path>([](const auto &output) {
                        return fs::path(output).replace_extension(".commands.json");
                    })
                    .unwrap_or(fs::path("commands.json"));

            auto environment = sys::env::from(const_cast<const char **>(envp));
            auto intercept = prepare_intercept(args, environment, commands);
            auto citnames = prepare_citnames(args, environment, commands);

            return rust::merge(intercept, citnames)
                    .map<ps::CommandPtr>([&commands](auto tuple) {
                        const auto&[intercept, citnames] = tuple;

                        return std::make_unique<Command>(intercept, citnames, commands);
                    });
        }
    };
}

int main(int argc, char *argv[], char *envp[]) {
    return ps::main<Application>(argc, argv, envp);
}
