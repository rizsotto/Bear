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

#include "collect/Application.h"
#include "collect/Reporter.h"
#include "collect/Services.h"
#include "collect/Session.h"
#include "intercept/Flags.h"
#include "libsys/Os.h"
#include "libsys/Signal.h"

#include <grpcpp/security/server_credentials.h>
#include <grpcpp/server_builder.h>
#include <spdlog/spdlog.h>
#include <fmt/format.h>

#include <vector>

namespace {

    constexpr std::optional<std::string_view> DEVELOPER_GROUP = { "developer options" };

    rust::Result<int> execute_command(const ic::Session& session, const std::vector<std::string_view>& command) {
        return session.supervise(command)
            .and_then<sys::Process>([](auto builder) {
                return builder.spawn();
            })
            .and_then<sys::ExitStatus>([](auto child) {
                sys::SignalForwarder guard(child);
                return child.wait();
            })
            .map<int>([](auto status) {
                return status.code().value_or(EXIT_FAILURE);
            })
            .map_err<std::runtime_error>([](auto error) {
                spdlog::warn("Command execution failed: {}", error.what());
                return error;
            })
            .on_success([](auto status) {
                spdlog::debug("Running command. [Exited with {0}]", status);
            });
    }

    rust::Result<std::vector<std::string_view>> get_command(const flags::Arguments& args)
    {
        return args.as_string_list(ic::COMMAND)
                .and_then<std::vector<std::string_view>>([](auto cmd) {
                    return (cmd.empty())
                            ? rust::Result<std::vector<std::string_view>>(rust::Err(std::runtime_error("Command is empty.")))
                            : rust::Result<std::vector<std::string_view>>(rust::Ok(cmd));
                });
    }
}

namespace ic {

    rust::Result<int> Command::execute() const
    {
        // Create and start the gRPC server
        int port = 0;
        ic::SupervisorImpl supervisor(*(session_));
        ic::InterceptorImpl interceptor(*(reporter_));
        auto server = grpc::ServerBuilder()
                          .RegisterService(&supervisor)
                          .RegisterService(&interceptor)
                          .AddListeningPort("127.0.0.1:0", grpc::InsecureServerCredentials(), &port)
                          .BuildAndStart();

        std::string server_address = fmt::format("127.0.0.1:{}", port);
        spdlog::debug("Running gRPC server. [Listening on {0}]", server_address);
        // Configure the session and the reporter objects
        session_->set_server_address(server_address);
        // Execute the build command
        auto result = execute_command(*session_, command_);
        // Stop the gRPC server
        spdlog::debug("Stopping gRPC server.");
        server->Shutdown();
        // Write output file.
        reporter_->flush();
        // Exit with the build status
        return result;
    }

    Application::Application() noexcept
            : ps::ApplicationFromArgs(ps::ApplicationLogConfig("intercept", "ic"))
    { }

    rust::Result<flags::Arguments> Application::parse(int argc, const char **argv) const {
        const flags::Parser parser("intercept", VERSION, {
                {ic::OUTPUT,        {1,  false, "path of the result file",        {"commands.json"},       std::nullopt}},
                {ic::FORCE_PRELOAD, {0,  false, "force to use library preload",   std::nullopt,            DEVELOPER_GROUP}},
                {ic::FORCE_WRAPPER, {0,  false, "force to use compiler wrappers", std::nullopt,            DEVELOPER_GROUP}},
                {ic::LIBRARY,       {1,  false, "path to the preload library",    {LIBRARY_DEFAULT_PATH},  DEVELOPER_GROUP}},
                {ic::EXECUTOR,      {1,  false, "path to the preload executable", {EXECUTOR_DEFAULT_PATH}, DEVELOPER_GROUP}},
                {ic::WRAPPER,       {1,  false, "path to the wrapper directory",  {WRAPPER_DEFAULT_PATH},  DEVELOPER_GROUP}},
                {ic::COMMAND,       {-1, true,  "command to execute",             std::nullopt,            std::nullopt}}
        });
        return parser.parse_or_exit(argc, const_cast<const char **>(argv));
    }

    rust::Result<ps::CommandPtr> Application::command(const flags::Arguments &args, const char **envp) const {
        auto command = get_command(args);
        auto session = Session::from(args, envp);
        auto reporter = session
                .and_then<Reporter::SharedPtr>([&args](const auto& session) {
                    return Reporter::from(args, *session);
                });

        return rust::merge(command, session, reporter)
                .map<ps::CommandPtr>([](auto tuple) {
                    const auto& [command, session, reporter] = tuple;
                    return std::make_unique<Command>(command, session, reporter);
                });
    }
}
