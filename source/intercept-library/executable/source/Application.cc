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
#include "er/Flags.h"
#include "libsys/Process.h"
#include "libsys/Signal.h"

#include <fmt/chrono.h>
#include <fmt/format.h>

#include <chrono>
#include <filesystem>
#include <memory>

namespace {

    struct Execution {
        const std::string_view command;
        const std::vector<std::string_view> arguments;
        const std::string working_directory;
        const std::map<std::string, std::string> environment;
    };

    struct Session {
        const std::string_view destination;
    };

    rust::Result<std::string> get_cwd()
    {
        std::error_code error_code;
        auto result = fs::current_path(error_code);
        return (error_code)
                ? rust::Result<std::string>(rust::Err(std::runtime_error(error_code.message())))
                : rust::Result<std::string>(rust::Ok(result.string()));
    }

    rust::Result<Execution> make_execution(const ::flags::Arguments& args, const sys::Context& context) noexcept
    {
        auto path = args.as_string(::er::flags::EXECUTE);
        auto command = args.as_string_list(::er::flags::COMMAND);
        auto working_dir = get_cwd();
        auto environment = context.get_environment();

        return merge(path, command, working_dir)
            .map<Execution>([&environment](auto tuple) {
                const auto& [_path, _command, _working_dir] = tuple;
                return Execution { _path, _command, _working_dir, environment };
            });
    }

    rust::Result<Session> make_session(const ::flags::Arguments& args) noexcept
    {
        return args.as_string(er::flags::DESTINATION)
            .map<Session>([](const auto& destination) {
                return Session { destination };
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
        pid_t ppid,
        const Execution& execution)
    {
        supervise::Event result;
        result.set_timestamp(now_as_string());
        result.set_pid(pid);
        result.set_ppid(ppid);

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

namespace er {

    struct Application::State {
        Session session;
        Execution execution;
    };

    rust::Result<Application> Application::create(const ::flags::Arguments& args, const sys::Context& context)
    {
        return rust::merge(make_session(args), make_execution(args, context))
            .map<Application>([&args, &context](auto in) {
                const auto& [session, execution] = in;
                auto state = new Application::State { session, execution };
                return Application(state);
            });
    }

    rust::Result<int> Application::operator()() const
    {
        rpc::InterceptClient client(impl_->session.destination);

        auto result = client.get_environment_update(impl_->execution.environment)
            .map<Execution>([this](auto environment) {
                return Execution {
                    impl_->execution.command,
                    impl_->execution.arguments,
                    impl_->execution.working_directory,
                    environment
                };
            })
            .and_then<sys::Process>([&client](auto execution) {
                return sys::Process::Builder(execution.command)
                    .add_arguments(execution.arguments.begin(), execution.arguments.end())
                    .set_environment(execution.environment)
                    .spawn_with_preload()
                    .on_success([&client, &execution](auto& child) {
                        client.report(make_start_event(child.get_pid(), getppid(), execution));
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
