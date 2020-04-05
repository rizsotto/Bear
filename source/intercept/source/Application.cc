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

#include "Interceptor.h"
#include "Reporter.h"
#include "Session.h"

#include <grpc/grpc.h>
#include <grpcpp/security/server_credentials.h>
#include <grpcpp/server.h>
#include <grpcpp/server_builder.h>
#include <grpcpp/server_context.h>
#include <spdlog/spdlog.h>

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

    ::rust::Result<Application> Application::from(const ::flags::Arguments& args)
    {
        auto command = Command::from(args);
        auto reporter = Reporter::from(args);
        auto session = Session::from(args);

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
        spdlog::debug("Running gRPC server.");
        ic::InterceptorImpl service(*(impl_->reporter_), *(impl_->session_));
        int server_port = 0;
        grpc::ServerBuilder builder;
        builder.RegisterService(&service);
        builder.AddListeningPort("dns:///localhost", grpc::InsecureServerCredentials(), &server_port);
        auto server = builder.BuildAndStart();
        spdlog::debug("Running gRPC server. [Listening on dns:///localhost:{0}]", server_port);
        // Configure the session and the reporter objects
        impl_->session_->set_server_address(fmt::format("dns:///localhost:{0}", server_port));
        impl_->reporter_->set_host_info(impl_->session_->get_host_info());
        impl_->reporter_->set_session_type(impl_->session_->get_session_type());
        // Execute the build command
        spdlog::debug("Running command.");
        auto result = impl_->session_->supervise(impl_->command)
            .map<int>([](auto status) {
                spdlog::debug("Running command. [Exited with {0}]", status);
                return status;
            });
        // Stop the gRPC server
        server->Shutdown();
        spdlog::debug("Stopping gRPC server.");
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
