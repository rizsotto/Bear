/*  Copyright (C) 2012-2022 by László Nagy
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

#include "Configuration.h"
#include "collect/Intercept.h"
#include "collect/Reporter.h"
#include "collect/RpcServices.h"
#include "collect/Session.h"
#include "report/libexec/Resolver.h"
#include "libsys/Environment.h"
#include "libsys/Errors.h"
#include "libsys/Os.h"

#include <grpcpp/security/server_credentials.h>
#include <grpcpp/server_builder.h>
#include <spdlog/spdlog.h>
#include <fmt/format.h>

#include <filesystem>
#include <vector>

namespace fs = std::filesystem;

namespace {

    rust::Result<ic::Configuration>
    into_configuration(const flags::Arguments &args) {
        auto output_file_arg = args.as_string(cmd::intercept::FLAG_OUTPUT);
        auto library_arg = args.as_string(cmd::intercept::FLAG_LIBRARY);
        auto wrapper_arg = args.as_string(cmd::intercept::FLAG_WRAPPER);
        auto wrapper_dir_arg = args.as_string(cmd::intercept::FLAG_WRAPPER_DIR);
        auto verbose_arg = args.as_bool(flags::VERBOSE);
        auto force_preload_arg = args.as_bool(cmd::intercept::FLAG_FORCE_PRELOAD);
        auto force_wrapper_arg = args.as_bool(cmd::intercept::FLAG_FORCE_WRAPPER);
        auto command_arg = args.as_string_list(cmd::intercept::FLAG_COMMAND);

        ic::Configuration config;

        if (output_file_arg.is_ok()) config.output_file = output_file_arg.unwrap();
        if (library_arg.is_ok()) config.library = library_arg.unwrap();
        if (wrapper_arg.is_ok()) config.wrapper = wrapper_arg.unwrap();
        if (wrapper_dir_arg.is_ok()) config.wrapper_dir = wrapper_dir_arg.unwrap();
        if (verbose_arg.is_ok()) config.verbose = verbose_arg.unwrap();
        if (force_preload_arg.is_ok() && force_preload_arg.unwrap()) config.use_wrapper = false;
        if (force_wrapper_arg.is_ok() && force_wrapper_arg.unwrap()) config.use_preload = false;
        if (command_arg.is_ok()) {
            config.command.clear();
            for (const auto& cmd_part : command_arg.unwrap()) {
                config.command.emplace_back(cmd_part);
            }
        }

        // validation
        if (!config.use_preload && !config.use_wrapper) {
            return rust::Err(std::runtime_error("At least one interception method must be enabled"));
        }

        if (config.command.empty()) {
            return rust::Err(std::runtime_error("Missing command to intercept"));
        }

        if (config.output_file.empty()) {
            return rust::Err(std::runtime_error("Missing input file"));
        }

        return rust::Ok(std::move(config));
    }

    rust::Result<ic::Execution> capture_execution(const ic::Configuration& config, sys::env::Vars &&environment)
    {
        const auto path = sys::os::get_path(environment);

        const auto executable = path
                .and_then<fs::path>([&config](const auto& path) {
                    auto executable = config.command.front();

                    el::Resolver resolver;
                    return resolver.from_search_path(executable, path.c_str())
                            .template map<fs::path>([](const auto &ptr) {
                                return fs::path(ptr);
                            })
                            .template map_err<std::runtime_error>([&executable](auto error) {
                                return std::runtime_error(
                                        fmt::format("Could not found: {}: {}", executable, sys::error_string(error)));
                            });
                });

        return executable
                .map<ic::Execution>([&environment, &config](const auto& executable) {
                    return ic::Execution{
                        executable,
                        std::list<std::string>(config.command.begin(), config.command.end()),
                        fs::path("ignored"),
                        std::move(environment)
                    };
                });
    }
}

namespace ic {

    rust::Result<int> Command::execute() const
    {
        // Create and start the gRPC server
        int port = 0;
        ic::SupervisorImpl supervisor(*session_);
        ic::InterceptorImpl interceptor(*reporter_);
        auto server = grpc::ServerBuilder()
                          .RegisterService(&supervisor)
                          .RegisterService(&interceptor)
                          .AddListeningPort("dns:///localhost:0", grpc::InsecureServerCredentials(), &port)
                          .BuildAndStart();

        // Create session_locator URL for the services
        auto session_locator = SessionLocator(fmt::format("dns:///localhost:{}", port));
        spdlog::debug("Running gRPC server. {0}", session_locator);
        // Execute the build command
        auto result = session_->run(execution_, session_locator);
        // Stop the gRPC server
        spdlog::debug("Stopping gRPC server.");
        server->Shutdown();
        // Exit with the build status
        return result;
    }

    Intercept::Intercept(const ps::ApplicationLogConfig& log_config) noexcept
            : ps::SubcommandFromArgs("intercept", log_config)
    { }

    rust::Result<ps::CommandPtr> Intercept::command(const flags::Arguments &args, const char **envp) const {
        return into_configuration(args)
                .and_then<std::tuple<Execution, Session::Ptr, Reporter::Ptr>>([&envp](const auto& configuration) {
                    const auto execution = capture_execution(configuration, sys::env::from(envp));
                    const auto session = Session::from(configuration, envp);
                    const auto reporter = Reporter::from(configuration);

                    return rust::merge(execution, session, reporter);
                })
                .map<ps::CommandPtr>([](auto tuple) {
                    const auto&[execution, session, reporter] = tuple;
                    return std::make_unique<Command>(execution, session, reporter);
                });
    }
}
