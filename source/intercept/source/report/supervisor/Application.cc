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

#include "report/supervisor/Application.h"
#include "report/supervisor/Flags.h"
#include "report/EventFactory.h"
#include "report/InterceptClient.h"
#include "libsys/Process.h"
#include "libsys/Signal.h"

#include <filesystem>
#include <memory>

namespace {

    struct Session {
        const std::string_view destination;
    };

    rust::Result<Session> make_session(const ::flags::Arguments& args) noexcept
    {
        return args.as_string(er::flags::DESTINATION)
                .map<Session>([](const auto& destination) {
                    return Session { destination };
                });
    }

    rust::Result<std::string> get_cwd()
    {
        std::error_code error_code;
        auto result = fs::current_path(error_code);
        return (error_code)
                ? rust::Result<std::string>(rust::Err(std::runtime_error(error_code.message())))
                : rust::Result<std::string>(rust::Ok(result.string()));
    }

    rust::Result<rpc::ExecutionContext> make_execution(const ::flags::Arguments &args, sys::env::Vars &&environment) noexcept
    {
        auto path = args.as_string(::er::flags::EXECUTE)
                .map<std::string>([](auto file) { return std::string(file); });
        auto command = args.as_string_list(::er::flags::COMMAND)
                .map<std::vector<std::string>>([](auto args) {
                    return std::vector<std::string>(args.begin(), args.end());
                });
        auto working_dir = get_cwd();

        return merge(path, command, working_dir)
                .map<rpc::ExecutionContext>([&environment](auto tuple) {
                    const auto&[_path, _command, _working_dir] = tuple;
                    return rpc::ExecutionContext{_path, _command, _working_dir, std::move(environment)};
                });
    }
}

namespace er {

    struct Application::State {
        Session session;
        rpc::ExecutionContext execution;
    };

    rust::Result<Application> Application::create(const ::flags::Arguments& args, sys::env::Vars &&environment)
    {
        return rust::merge(make_session(args), make_execution(args, std::move(environment)))
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

        auto result = client.get_environment_update(impl_->execution.environment)
            .map<rpc::ExecutionContext>([this](auto environment) {
                return rpc::ExecutionContext {
                    impl_->execution.command,
                    impl_->execution.arguments,
                    impl_->execution.working_directory,
                    environment
                };
            })
            .and_then<sys::Process>([&client, &event_factory](auto execution) {
                return sys::Process::Builder(execution.command)
                    .add_arguments(execution.arguments.begin(), execution.arguments.end())
                    .set_environment(execution.environment)
                    .spawn_with_preload()
                    .on_success([&client, &event_factory, &execution](auto& child) {
                        auto event = event_factory.start(child.get_pid(), getppid(), execution);
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
