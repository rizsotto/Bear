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

#include "Application.h"

#include "Reporter.h"
#include "Services.h"
#include "Session.h"
#include "libsys/Context.h"

#include <grpc/grpc.h>
#include <grpcpp/security/server_credentials.h>
#include <grpcpp/server.h>
#include <grpcpp/server_builder.h>
#include <grpcpp/server_context.h>
#include <spdlog/spdlog.h>
#include <fmt/format.h>

#include <map>
#include <vector>

namespace {

    struct Command {
        static rust::Result<std::vector<std::string_view>> from(const flags::Arguments& args)
        {
            return args.as_string_list(ic::Application::COMMAND)
                    .and_then<std::vector<std::string_view>>([](auto cmd) {
                        return (cmd.empty())
                                ? rust::Result<std::vector<std::string_view>>(rust::Err(std::runtime_error("Command is empty.")))
                                : rust::Result<std::vector<std::string_view>>(rust::Ok(cmd));
                    });
        }
    };
}

namespace ic {

    struct Application::State {
        std::vector<std::string_view> command;
        Reporter::SharedPtr reporter_;
        Session::SharedPtr session_;
    };

    ::rust::Result<Application> Application::from(const flags::Arguments& args, const sys::Context& context)
    {
        auto command = Command::from(args);
        auto session = Session::from(args, context);
        auto reporter = session
                            .and_then<Reporter::SharedPtr>([&args, &context](const auto& session) {
                                return Reporter::from(args, context, *session);
                            });

        return rust::merge(command, reporter, session)
                   .map<Application::State*>([](auto tuple) {
                       const auto& [command, reporter, session] = tuple;
                       return new Application::State { command, reporter, session };
                   })
                   .map<Application>([](auto impl) {
                       return Application { impl };
                   });
    }

    ::rust::Result<int> Application::operator()() const
    {
        // Create and start the gRPC server
        int port = 0;
        ic::SupervisorImpl supervisor(*(impl_->session_));
        ic::InterceptorImpl interceptor(*(impl_->reporter_));
        auto server = grpc::ServerBuilder()
                          .RegisterService(&supervisor)
                          .RegisterService(&interceptor)
                          .AddListeningPort("0.0.0.0:0", grpc::InsecureServerCredentials(), &port)
                          .BuildAndStart();

        std::string server_address = fmt::format("0.0.0.0:{}", port);
        spdlog::debug("Running gRPC server. [Listening on {0}]", server_address);
        // Configure the session and the reporter objects
        impl_->session_->set_server_address(server_address);
        // Execute the build command
        spdlog::debug("Running command.");
        auto result = impl_->session_->supervise(impl_->command)
            .on_success([](auto status) {
                spdlog::debug("Running command. [Exited with {0}]", status);
            });
        // Stop the gRPC server
        spdlog::debug("Stopping gRPC server.");
        server->Shutdown();
        // Write output file.
        impl_->reporter_->flush();
        // Exit with the build status
        return result;
    }

    Application::Application(Application::State* const impl)
            : impl_(impl)
    {
    }

    Application::Application(Application&& rhs) noexcept
            : impl_(rhs.impl_)
    {
        rhs.impl_ = nullptr;
    }

    Application& Application::operator=(Application&& rhs) noexcept
    {
        if (&rhs != this) {
            delete impl_;
            impl_ = rhs.impl_;
        }
        return *this;
    }

    Application::~Application()
    {
        delete impl_;
        impl_ = nullptr;
    }
}
