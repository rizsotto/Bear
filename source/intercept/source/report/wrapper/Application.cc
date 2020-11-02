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

#include "report/wrapper/Application.h"
#include "librpc/EventFactory.h"
#include "librpc/InterceptClient.h"
#include "libsys/Process.h"
#include "libsys/Signal.h"
#include "Environment.h"

#include <filesystem>
#include <memory>
#include <string>

namespace {

    struct Session {
        const std::string destination;
    };

    rust::Result<Session> make_session(const sys::env::Vars& environment) noexcept
    {
        auto destination = environment.find(wr::env::KEY_DESTINATION);
        return (destination == environment.end())
               ? rust::Result<Session>(rust::Err(std::runtime_error("Unknown destination.")))
               : rust::Result<Session>(rust::Ok(Session { destination->second }));
    }

    struct Execution {
        const std::string command;
        const std::vector<std::string> arguments;
        const std::string working_directory;
        const std::map<std::string, std::string> environment;
    };

    std::vector<std::string> from(const char** args)
    {
        const char** end = args;
        while (*end != nullptr)
            ++end;
        return std::vector<std::string>(args, end);
    }

    rust::Result<std::string> get_cwd()
    {
        std::error_code error_code;
        auto result = fs::current_path(error_code);
        return (error_code)
               ? rust::Result<std::string>(rust::Err(std::runtime_error(error_code.message())))
               : rust::Result<std::string>(rust::Ok(result.string()));
    }

    rust::Result<Execution> make_execution(const char** args, sys::env::Vars&& environment) noexcept
    {
        auto path = fs::path(args[0]).string();
        auto command = from(args);
        auto working_dir = get_cwd();

        return working_dir
            .map<Execution>([&path, &command, &environment](auto cwd) {
                return Execution { path, command, cwd, environment };
            });
    }
}

namespace wr {

    struct Application::State {
        Session session;
        Execution execution;
    };

    rust::Result<Application> Application::create(const char** args, sys::env::Vars&& environment)
    {
        auto session = make_session(environment);
        auto execution = make_execution(args, std::move(environment));

        return rust::merge(session, execution)
            .map<Application>([](auto in) {
                const auto& [session, execution] = in;
                auto state = new Application::State { session, execution };
                return Application(state);
            });
    }

    rust::Result<int> Application::operator()() const
    {
        rpc::EventFactory event_factory;
        rpc::InterceptClient client(impl_->session.destination);
        auto command = client.get_wrapped_command(impl_->execution.command);
        auto environment = client.get_environment_update(impl_->execution.environment);

        auto result = rust::merge(command, environment)
            .map<Execution>([this](auto tuple) {
                const auto& [command, environment] = tuple;
                auto arguments = impl_->execution.arguments;
                arguments.front() = command;
                return Execution {
                    command,
                    arguments,
                    impl_->execution.working_directory,
                    environment
                };
            })
            .and_then<sys::Process>([&client, &event_factory](auto execution) {
                return sys::Process::Builder(execution.command)
                    .add_arguments(execution.arguments.begin(), execution.arguments.end())
                    .set_environment(execution.environment)
                    .spawn()
                    .on_success([&client, &event_factory, &execution](auto& child) {
                        auto event = event_factory.start(
                                child.get_pid(),
                                getppid(),
                                execution.command,
                                execution.arguments,
                                execution.working_directory,
                                execution.environment);
                        client.report(std::move(event));
                    });
            })
            .and_then<sys::ExitStatus>([&client, &event_factory](auto child) {
                sys::SignalForwarder guard(child);
                while (true) {
                    auto status = child.wait(true);
                    status.on_success([&client, &event_factory](auto exit) {
                        auto event = exit.is_signaled()
                                     ? event_factory.signal(exit.signal().value())
                                     : event_factory.terminate(exit.code().value());
                        client.report(std::move(event));
                    });
                    if (status.template map<bool>([](auto _status) { return _status.is_exited(); }).unwrap_or(false)) {
                        return status;
                    }
                }
            })
            .map<int>([](auto status) {
                return status.code().value_or(EXIT_FAILURE);
            });

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
