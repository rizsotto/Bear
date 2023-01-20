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

#include "collect/Intercept.h"
#include "collect/Reporter.h"
#include "collect/RpcServices.h"
#include "collect/Session.h"
#include "report/libexec/Resolver.h"
#include "libsys/Environment.h"
#include "libsys/Errors.h"
#include "libsys/Os.h"
#include "libconfig/Configuration.h"

#include <grpcpp/security/server_credentials.h>
#include <grpcpp/server_builder.h>
#include <spdlog/spdlog.h>
#include <fmt/format.h>

#include <filesystem>
#include <vector>

namespace fs = std::filesystem;

namespace {

    rust::Result<ic::Execution> capture_execution(const config::Intercept& config)
    {
        const auto& environment = sys::env::get();
        const auto path = sys::os::get_path();

        const auto executable = path
                .and_then<fs::path>([&config](auto path) {
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
                .map<ic::Execution>([&environment, &config](auto executable) {
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

    Intercept::Intercept(const config::Intercept &config, const ps::ApplicationLogConfig& log_config) noexcept
            : ps::SubcommandFromConfig<config::Intercept>("intercept", log_config, config)
    { }

    rust::Result<ps::CommandPtr> Intercept::command(const config::Intercept &config) const {
        const auto execution = capture_execution(config);
        const auto session = Session::from(config);
        const auto reporter = Reporter::from(config);

        return rust::merge(execution, session, reporter)
                .map<ps::CommandPtr>([](auto tuple) {
                    const auto&[execution, session, reporter] = tuple;
                    return std::make_unique<Command>(execution, session, reporter);
                });
    }

    std::optional<std::runtime_error> Intercept::update_config(const flags::Arguments &args) {
        return config_.update(args);
    }
}
