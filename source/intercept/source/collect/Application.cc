/*  Copyright (C) 2012-2021 by László Nagy
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

#include "collect/Application.h"
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

    constexpr std::optional<std::string_view> DEVELOPER_GROUP = { "developer options" };

    rust::Result<ic::Execution> capture_execution(const flags::Arguments& args, sys::env::Vars &&environment)
    {
        auto path = sys::os::get_path(environment);

        auto command = args.as_string_list(cmd::intercept::FLAG_COMMAND)
                .and_then<std::vector<std::string_view>>([](auto args) {
                    using Result = rust::Result<std::vector<std::string_view>>;
                    return (args.empty())
                            ? Result(rust::Err(std::runtime_error("Command is empty.")))
                            : Result(rust::Ok(args));
                });

        auto executable = rust::merge(path, command)
                .and_then<fs::path>([](auto tuple) {
                    const auto&[path, command] = tuple;
                    auto executable = command.front();

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

        return rust::merge(executable, command)
                .map<ic::Execution>([&environment](auto tuple) {
                    const auto&[executable, command] = tuple;
                    return ic::Execution{
                        executable,
                        std::list<std::string>(command.begin(), command.end()),
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

    Application::Application() noexcept
            : ps::ApplicationFromArgs(ps::ApplicationLogConfig("intercept", "ic"))
    { }

    rust::Result<flags::Arguments> Application::parse(int argc, const char **argv) const {
        const flags::Parser parser("intercept", cmd::VERSION, {
                {cmd::intercept::FLAG_OUTPUT,        {1,  false, "path of the result file",        {cmd::intercept::DEFAULT_OUTPUT}, std::nullopt}},
                {cmd::intercept::FLAG_FORCE_PRELOAD, {0,  false, "force to use library preload",   std::nullopt,                     DEVELOPER_GROUP}},
                {cmd::intercept::FLAG_FORCE_WRAPPER, {0,  false, "force to use compiler wrappers", std::nullopt,                     DEVELOPER_GROUP}},
                {cmd::intercept::FLAG_LIBRARY,       {1,  false, "path to the preload library",    {cmd::library::DEFAULT_PATH},     DEVELOPER_GROUP}},
                {cmd::intercept::FLAG_WRAPPER,       {1,  false, "path to the wrapper executable", {cmd::wrapper::DEFAULT_PATH},     DEVELOPER_GROUP}},
                {cmd::intercept::FLAG_WRAPPER_DIR,   {1,  false, "path to the wrapper directory",  {cmd::wrapper::DEFAULT_DIR_PATH}, DEVELOPER_GROUP}},
                {cmd::intercept::FLAG_COMMAND,       {-1, true,  "command to execute",             std::nullopt,                     std::nullopt}}
        });
        return parser.parse_or_exit(argc, const_cast<const char **>(argv));
    }

    rust::Result<ps::CommandPtr> Application::command(const flags::Arguments &args, const char **envp) const {
        auto execution = capture_execution(args, sys::env::from(envp));
        auto session = Session::from(args, envp);
        auto reporter = Reporter::from(args);

        return rust::merge(execution, session, reporter)
                .map<ps::CommandPtr>([](auto tuple) {
                    const auto&[execution, session, reporter] = tuple;
                    return std::make_unique<Command>(execution, session, reporter);
                });
    }
}
