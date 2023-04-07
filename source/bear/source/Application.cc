/* (C) 2012-2022 by László Nagy
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
#include "citnames/citnames-forward.h"
#include "intercept/intercept-forward.h"

namespace {

    constexpr std::optional<std::string_view> ADVANCED_GROUP = {"advanced options"};
    constexpr std::optional<std::string_view> DEVELOPER_GROUP = {"developer options"};

    rust::Result<sys::Process::Builder>
    prepare_intercept(const flags::Arguments &arguments, const sys::env::Vars &environment, const fs::path &output) {
        auto program = arguments.as_string(cmd::bear::FLAG_BEAR);
        auto command = arguments.as_string_list(cmd::intercept::FLAG_COMMAND);
        auto library = arguments.as_string(cmd::intercept::FLAG_LIBRARY);
        auto wrapper = arguments.as_string(cmd::intercept::FLAG_WRAPPER);
        auto wrapper_dir = arguments.as_string(cmd::intercept::FLAG_WRAPPER_DIR);
        auto verbose = arguments.as_bool(flags::VERBOSE).unwrap_or(false);
        auto force_wrapper = arguments.as_bool(cmd::intercept::FLAG_FORCE_WRAPPER).unwrap_or(false);
        auto force_preload = arguments.as_bool(cmd::intercept::FLAG_FORCE_PRELOAD).unwrap_or(false);

        return rust::merge(program, command, rust::merge(library, wrapper, wrapper_dir))
                .map<sys::Process::Builder>(
                        [&environment, &output, &verbose, &force_wrapper, &force_preload](auto tuple) {
                            const auto&[program, command, pack] = tuple;
                            const auto&[library, wrapper, wrapper_dir] = pack;

                            auto builder = sys::Process::Builder(program)
                                    .set_environment(environment)
                                    .add_argument(program)
                                    .add_argument("intercept")
                                    .add_argument(cmd::intercept::FLAG_LIBRARY).add_argument(library)
                                    .add_argument(cmd::intercept::FLAG_WRAPPER).add_argument(wrapper)
                                    .add_argument(cmd::intercept::FLAG_WRAPPER_DIR).add_argument(wrapper_dir)
                                    .add_argument(cmd::intercept::FLAG_OUTPUT).add_argument(output);
                            if (force_wrapper) {
                                builder.add_argument(cmd::intercept::FLAG_FORCE_WRAPPER);
                            }
                            if (force_preload) {
                                builder.add_argument(cmd::intercept::FLAG_FORCE_PRELOAD);
                            }
                            if (verbose) {
                                builder.add_argument(flags::VERBOSE);
                            }
                            builder.add_argument(cmd::intercept::FLAG_COMMAND)
                                    .add_arguments(command.begin(), command.end());
                            return builder;
                        });
    }

    rust::Result<sys::Process::Builder>
    prepare_citnames(const flags::Arguments &arguments, const sys::env::Vars &environment, const fs::path &input) {
        auto program = arguments.as_string(cmd::bear::FLAG_BEAR);
        auto with_link = arguments.as_bool(cmd::citnames::FLAG_WITH_LINK).unwrap_or(false);
        auto output_compile = arguments.as_string(cmd::citnames::FLAG_OUTPUT_COMPILE);
        auto output_link = arguments.as_string(cmd::citnames::FLAG_OUTPUT_LINK);
        auto config = arguments.as_string(cmd::citnames::FLAG_CONFIG);
        auto append = arguments.as_bool(cmd::citnames::FLAG_APPEND).unwrap_or(false);
        auto verbose = arguments.as_bool(flags::VERBOSE).unwrap_or(false);

        return rust::merge(program, output_compile, output_link)
                .map<sys::Process::Builder>([&environment, &input, &config, &append, &verbose, &with_link](auto tuple) {
                    const auto&[program, output_compile, output_link] = tuple;

                    auto builder = sys::Process::Builder(program)
                            .set_environment(environment)
                            .add_argument(program)
                            .add_argument("citnames")
                            .add_argument(cmd::citnames::FLAG_INPUT).add_argument(input)
                            .add_argument(cmd::citnames::FLAG_OUTPUT_COMPILE).add_argument(output_compile)
                            .add_argument(cmd::citnames::FLAG_OUTPUT_LINK).add_argument(output_link)
                            // can run the file checks, because we are on the host.
                            .add_argument(cmd::citnames::FLAG_RUN_CHECKS);
                    if (append) {
                        builder.add_argument(cmd::citnames::FLAG_APPEND);
                    }
                    if (with_link) {
                        builder.add_argument(cmd::citnames::FLAG_WITH_LINK);
                    }
                    if (config.is_ok()) {
                        builder.add_argument(cmd::citnames::FLAG_CONFIG).add_argument(config.unwrap());
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
}

namespace bear {

	Command::Command(const sys::Process::Builder& intercept, const sys::Process::Builder& citnames, fs::path output) noexcept
			: ps::Command()
			, intercept_(intercept)
			, citnames_(citnames)
			, output_(std::move(output))
	{ }

	[[nodiscard]] rust::Result<int> Command::execute() const
	{
		auto result = ::execute(intercept_, "intercept");

		std::error_code error_code;
		if (fs::exists(output_, error_code)) {
			::execute(citnames_, "citnames");
			fs::remove(output_, error_code);
		}
		return result;
	}

	Application::Application()
			: ps::ApplicationFromArgs(ps::ApplicationLogConfig("bear", "br"))
	{ }

	rust::Result<flags::Arguments> Application::parse(int argc, const char **argv) const
	{
            const flags::Parser intercept_parser("intercept", cmd::VERSION, {
                {cmd::intercept::FLAG_OUTPUT,        {1,  false, "path of the result file",        {cmd::intercept::DEFAULT_OUTPUT}, std::nullopt}},
                {cmd::intercept::FLAG_FORCE_PRELOAD, {0,  false, "force to use library preload",   std::nullopt,                     DEVELOPER_GROUP}},
                {cmd::intercept::FLAG_FORCE_WRAPPER, {0,  false, "force to use compiler wrappers", std::nullopt,                     DEVELOPER_GROUP}},
                {cmd::intercept::FLAG_LIBRARY,       {1,  false, "path to the preload library",    {cmd::library::DEFAULT_PATH},     DEVELOPER_GROUP}},
                {cmd::intercept::FLAG_WRAPPER,       {1,  false, "path to the wrapper executable", {cmd::wrapper::DEFAULT_PATH},     DEVELOPER_GROUP}},
                {cmd::intercept::FLAG_WRAPPER_DIR,   {1,  false, "path to the wrapper directory",  {cmd::wrapper::DEFAULT_DIR_PATH}, DEVELOPER_GROUP}},
                {cmd::intercept::FLAG_COMMAND,       {-1, true,  "command to execute",             std::nullopt,                     std::nullopt}}
            });

            const flags::Parser citnames_parser("citnames", cmd::VERSION, {
                {cmd::citnames::FLAG_INPUT,          {1, false, "path of the input file",                    {cmd::intercept::DEFAULT_OUTPUT},        std::nullopt}},
                {cmd::citnames::FLAG_WITH_LINK,      {0, false, "whether to create a link base",             std::nullopt,                            std::nullopt}},
                {cmd::citnames::FLAG_OUTPUT_COMPILE, {1, false, "path of the result compile file",           {cmd::citnames::DEFAULT_OUTPUT_COMPILE}, std::nullopt}},
                {cmd::citnames::FLAG_OUTPUT_LINK,    {1, false, "path of the result link file",              {cmd::citnames::DEFAULT_OUTPUT_LINK},    std::nullopt}},
                {cmd::citnames::FLAG_CONFIG,         {1, false, "path of the config file",                   std::nullopt,                            std::nullopt}},
                {cmd::citnames::FLAG_APPEND,         {0, false, "append to output, instead of overwrite it", std::nullopt,                            std::nullopt}},
                {cmd::citnames::FLAG_RUN_CHECKS,     {0, false, "can run checks on the current host",        std::nullopt,                            std::nullopt}}
            });

            const flags::Parser parser("bear", cmd::VERSION, {intercept_parser, citnames_parser}, {
                {cmd::citnames::FLAG_WITH_LINK,      {0, false, "whether to create a link base",             std::nullopt,                            std::nullopt}},
                {cmd::citnames::FLAG_OUTPUT_COMPILE, {1,  false, "path of the result compile file",          {cmd::citnames::DEFAULT_OUTPUT_COMPILE}, std::nullopt}},
                {cmd::citnames::FLAG_OUTPUT_LINK,    {1,  false, "path of the result link file",             {cmd::citnames::DEFAULT_OUTPUT_LINK},    std::nullopt}},
                {cmd::citnames::FLAG_APPEND,         {0,  false, "append result to an existing output file", std::nullopt,                            ADVANCED_GROUP}},
                {cmd::citnames::FLAG_CONFIG,         {1,  false, "path of the config file",                  std::nullopt,                            ADVANCED_GROUP}},
                {cmd::intercept::FLAG_FORCE_PRELOAD, {0,  false, "force to use library preload",             std::nullopt,                            ADVANCED_GROUP}},
                {cmd::intercept::FLAG_FORCE_WRAPPER, {0,  false, "force to use compiler wrappers",           std::nullopt,                            ADVANCED_GROUP}},
                {cmd::bear::FLAG_BEAR,               {1,  false, "path to the bear executable",              {cmd::bear::DEFAULT_PATH},               DEVELOPER_GROUP}},
                {cmd::intercept::FLAG_LIBRARY,       {1,  false, "path to the preload library",              {cmd::library::DEFAULT_PATH},            DEVELOPER_GROUP}},
                {cmd::intercept::FLAG_WRAPPER,       {1,  false, "path to the wrapper executable",           {cmd::wrapper::DEFAULT_PATH},            DEVELOPER_GROUP}},
                {cmd::intercept::FLAG_WRAPPER_DIR,   {1,  false, "path to the wrapper directory",            {cmd::wrapper::DEFAULT_DIR_PATH},        DEVELOPER_GROUP}},
                {cmd::intercept::FLAG_COMMAND,       {-1, true,  "command to execute",                       std::nullopt,                            std::nullopt}}
            });
            return parser.parse_or_exit(argc, const_cast<const char **>(argv));
	}

	rust::Result<ps::CommandPtr> Application::command(const flags::Arguments &args, const char **envp) const
	{
            if (args.as_string(flags::COMMAND).is_ok()) {
                if (auto citnames = cs::Citnames(log_config_); citnames.matches(args)) {
                    return citnames.subcommand(args, envp);
                }
                if (auto intercept = ic::Intercept(log_config_); intercept.matches(args)) {
                    return intercept.subcommand(args, envp);
                }

                return rust::Err(std::runtime_error("Invalid subcommand"));
            }

            // now only the name of the compile file is used
            // to generate the name of the intermediate file (events)
            auto commands = args.as_string(cmd::citnames::FLAG_OUTPUT_COMPILE)
                .map<fs::path>([](const auto &output) {
                        return fs::path(output).replace_extension(".events.json");
                })
                .unwrap_or(fs::path(cmd::intercept::DEFAULT_OUTPUT));

            auto environment = sys::env::from(const_cast<const char **>(envp));
            auto intercept = prepare_intercept(args, environment, commands);
            auto citnames = prepare_citnames(args, environment, commands);

            return rust::merge(intercept, citnames)
                .map<ps::CommandPtr>([&commands](const auto &tuple) {
                        const auto&[intercept, citnames] = tuple;

                        return std::make_unique<Command>(intercept, citnames, commands);
                });
	}
}
