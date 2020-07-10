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
#include "librpc/InterceptClient.h"
#include "librpc/supervise.grpc.pb.h"
#include "libsys/Path.h"
#include "libsys/Process.h"
#include "libsys/Signal.h"
#include "libwrapper/Environment.h"

#include <fmt/chrono.h>
#include <fmt/format.h>

#include <chrono>
#include <memory>
#include <string>

namespace {

    struct Session {
        const std::string destination;
    };

    rust::Result<Session> make_session() noexcept
    {
        const char* destination = getenv(wr::env::KEY_DESTINATION);
        return (destination == nullptr)
               ? rust::Result<Session>(rust::Err(std::runtime_error("Unknown destination.")))
               : rust::Result<Session>(rust::Ok(Session { destination }));
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
        for (; *end != nullptr; ++end)
            ;
        return std::vector<std::string>(args, end);
    }

    rust::Result<Execution> make_execution(const char** args, const sys::Context& context) noexcept
    {
        auto path = sys::path::basename(args[0]);
        auto command = from(args);
        auto working_dir = context.get_cwd();
        auto environment = context.get_environment();

        return working_dir
            .map<Execution>([&path, &command, &environment](auto cwd) {
                return Execution { path, command, cwd, environment };
            });
    }

    std::string now_as_string()
    {
        const auto now = std::chrono::system_clock::now();
        auto micros = std::chrono::duration_cast<std::chrono::microseconds>(now.time_since_epoch());

        return fmt::format("{:%Y-%m-%dT%H:%M:%S}.{:06d}Z",
                           fmt::localtime(std::chrono::system_clock::to_time_t(now)),
                           micros.count() % 1000000);
    }

    supervise::Event make_start_event(
        pid_t pid,
        const Execution& execution)
    {
        supervise::Event result;
        result.set_timestamp(now_as_string());
        result.set_pid(pid);

        std::unique_ptr<supervise::Event_Started> event = std::make_unique<supervise::Event_Started>();
        event->set_executable(execution.command.data());
        for (const auto& arg : execution.arguments) {
            event->add_arguments(arg.data());
        }
        event->set_working_dir(execution.working_directory);
        event->mutable_environment()->insert(execution.environment.begin(), execution.environment.end());

        result.set_allocated_started(event.release());
        return result;
    }

    supervise::Event make_status_event(pid_t pid, sys::ExitStatus status)
    {
        supervise::Event result;
        result.set_timestamp(now_as_string());
        result.set_pid(pid);

        if (status.is_signaled()) {
            // TODO: this shall return a termination event too
            std::unique_ptr<supervise::Event_Signalled> event = std::make_unique<supervise::Event_Signalled>();
            event->set_number(status.signal().value());

            result.set_allocated_signalled(event.release());
        } else {
            std::unique_ptr<supervise::Event_Terminated> event = std::make_unique<supervise::Event_Terminated>();
            event->set_status(status.code().value());

            result.set_allocated_terminated(event.release());
        }
        return result;
    }
}

namespace wr {

    struct Application::State {
        Session session;
        Execution execution;
    };

    ::rust::Result<Application> Application::create(const char** args, const sys::Context& ctx)
    {
        auto session = make_session();
        auto execution = make_execution(args, ctx);

        return rust::merge(session, execution)
            .map<Application>([](auto in) {
                const auto& [session, execution] = in;
                auto state = new Application::State { session, execution };
                return Application(state);
            });
    }

    rust::Result<int> Application::operator()() const
    {
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
            .and_then<sys::Process>([&client](auto execution) {
                return sys::Process::Builder(execution.command)
                    .add_arguments(execution.arguments.begin(), execution.arguments.end())
                    .set_environment(execution.environment)
                    .spawn()
                    .on_success([&client, &execution](auto& child) {
                        client.report(make_start_event(child.get_pid(), execution));
                    });
            })
            .and_then<sys::ExitStatus>([&client](auto child) {
                sys::SignalForwarder guard(&child);
                while (true) {
                    auto status = child.wait(true);
                    status.on_success([&client, &child](auto exit) {
                        client.report(make_status_event(child.get_pid(), exit));
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
