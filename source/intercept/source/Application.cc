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
#include <memory>
#include <vector>

namespace {

    struct Command {
        std::vector<std::string_view> arguments;

        static rust::Result<Command> from(const flags::Arguments& args)
        {
            return args.as_string_list(ic::Application::COMMAND)
                .map<Command>([](auto vec) { return Command { vec }; });
        }
    };

    std::map<std::string, std::string> current_environment()
    {
        return std::map<std::string, std::string>();
    }

    rust::Result<int> spawn(const Command& command, const ic::Session& session)
    {
        auto current = current_environment();
        auto updated = session.update(std::move(current));
        // TODO: execute and wait
        return rust::Ok(0);
    }

}

namespace ic {

    struct Application::State {
        Command command;
        ReporterPtr reporter_;
        SessionPtr session_;
    };

    ::rust::Result<Application> Application::from(const ::flags::Arguments& args)
    {
        auto command = Command::from(args);
        rust::Result<ReporterPtr> reporter = rust::Ok(std::make_shared<Reporter>());
        rust::Result<SessionPtr> session = rust::Ok(std::shared_ptr<Session>(new FakeSession()));

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
        spdlog::info("Running command now.");
        // Create and start the gRPC server
        ic::InterceptorImpl service(*(impl_->reporter_), *(impl_->session_));
        int server_port = 0;
        grpc::ServerBuilder builder;
        builder.RegisterService(&service);
        builder.AddListeningPort("dns:///localhost", grpc::InsecureServerCredentials(), &server_port);
        auto server = builder.BuildAndStart();
        // Execute the build command
        impl_->session_->set_server_address(fmt::format("dns:///localhost:{0}", server_port));
        auto result = spawn(impl_->command, *(impl_->session_));
        // Stop the gRPC server
        server->Shutdown();
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
