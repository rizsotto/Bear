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
#include "InterceptClient.h"
#include "Reporter.h"
#include "er/Flags.h"
#include "supervise.grpc.pb.h"

#include <fmt/chrono.h>
#include <fmt/format.h>
#include <spdlog/spdlog.h>

#include <chrono>
#include <libsys/Environment.h>
#include <memory>

namespace {

    struct Execution {
        const std::string_view path;
        const std::vector<std::string_view> command;
    };

    struct Session {
        const std::string_view reporter;
        const std::string_view destination;
        const std::string_view library;
        bool verbose;
    };

    rust::Result<Session> make_session(const ::flags::Arguments& args) noexcept
    {
        auto library = args.as_string(er::flags::LIBRARY);
        auto destination = args.as_string(er::flags::DESTINATION);

        return rust::merge(library, destination)
            .map<Session>([&args](const auto& pair) {
                const auto& [library, destination] = pair;
                const auto reporter = args.program();
                const bool verbose = args.as_bool(::er::flags::VERBOSE).unwrap_or(false);
                return Session { reporter, destination, library, verbose };
            });
    }

    rust::Result<Execution> make_execution(const ::flags::Arguments& args) noexcept
    {
        auto path = args.as_string(::er::flags::EXECUTE);
        auto command = args.as_string_list(::er::flags::COMMAND);

        return merge(path, command)
            .map<Execution>([](auto tuple) {
                const auto& [path, command] = tuple;
                return Execution { path, command };
            });
    }

    std::vector<const char*> to_char_vector(const std::vector<std::string_view>& input)
    {
        auto result = std::vector<const char*>(input.size());
        std::transform(input.begin(), input.end(), result.begin(), [](auto it) { return it.data(); });
        result.push_back(nullptr);
        return result;
    }

    void report_start(const er::Reporter::SharedPtr& reporter, pid_t pid, const char** cmd) noexcept
    {
        reporter->start(pid, cmd)
            .and_then<int>([&reporter](auto message) {
                return reporter->send(message);
            })
            .unwrap_or_else([](auto message) {
                spdlog::warn("report process start failed: ", message.what());
                return 0;
            });
    }

    void report_exit(const er::Reporter::SharedPtr& reporter, pid_t pid, int exit) noexcept
    {
        reporter->stop(pid, exit)
            .and_then<int>([&reporter](auto message) {
                return reporter->send(message);
            })
            .unwrap_or_else([](auto message) {
                spdlog::error("report process stop failed: ", message.what());
                return 0;
            });
    }

    long to_millis(const std::chrono::time_point<std::chrono::high_resolution_clock>& t)
    {
        auto millis = std::chrono::duration_cast<std::chrono::milliseconds>(t.time_since_epoch());
        return millis.count() % 1000;
    }

    std::string now_as_string()
    {
        const auto now = std::chrono::system_clock::now();
        const auto millis = to_millis(now);

        // TODO: fix this!!!
        return "todo";
        //        return fmt::format("{:%Y-%m-%d %H:%M:%S}", std::chrono::system_clock::to_time_t(now));

        //        return fmt::format("{:%Y-%m-%dT%H:%M:%S}.{03d}Z",
        //            std::chrono::system_clock::to_time_t(now),
        //            millis);
    }

    std::shared_ptr<supervise::Event> start(
        pid_t pid,
        pid_t ppid,
        const Execution& execution,
        const std::string& cwd,
        const std::map<std::string, std::string>& env)
    {
        std::shared_ptr<supervise::Event> result = std::make_shared<supervise::Event>();
        result->set_timestamp(now_as_string());

        std::unique_ptr<supervise::Event_Started> event = std::make_unique<supervise::Event_Started>();
        event->set_pid(pid);
        event->set_ppid(ppid);
        event->set_executable(execution.path.data());
        for (const auto& arg : execution.command) {
            event->add_arguments(arg.data());
        }
        event->set_working_dir(cwd);
        event->mutable_environment()->insert(env.begin(), env.end());
        result->set_allocated_started(event.release());
        return result;
    }

    std::shared_ptr<supervise::Event> stop(int status)
    {
        std::shared_ptr<supervise::Event> result = std::make_shared<supervise::Event>();
        result->set_timestamp(now_as_string());

        std::unique_ptr<supervise::Event_Stopped> event = std::make_unique<supervise::Event_Stopped>();
        event->set_status(status);

        result->set_allocated_stopped(event.release());
        return result;
    }
}

namespace er {

    struct Application::State {
        Session session_;
        Execution execution_;
        Reporter::SharedPtr reporter_;
        const sys::Context& context_;
    };

    rust::Result<Application> Application::create(const ::flags::Arguments& args, const sys::Context& context)
    {
        auto session = make_session(args);
        auto reporter = session.and_then<Reporter::SharedPtr>([&context](const auto& session_value) {
            return Reporter::from(session_value.destination.data(), context);
        });
        auto execution = make_execution(args);

        return rust::merge(session, execution, reporter)
            .map<Application>([&args, &context](auto in) {
                const auto& [session, execution, reporter] = in;
                auto state = new Application::State { session, execution, reporter, context };
                return Application(state);
            });
    }

    rust::Result<int> Application::operator()() const
    {
        er::InterceptClient client(impl_->session_.destination);
        std::list<supervise::Event> events;

        return client.get_environment_update(impl_->context_.get_environment())
            .and_then<pid_t>([this](auto environment) {
                auto command = to_char_vector(impl_->execution_.command);
                sys::env::Guard guard(environment);

                return impl_->context_.spawn(impl_->execution_.path.data(), command.data(), guard.data());
            })
            .map<pid_t>([this, &events](auto& pid) {
                report_start(impl_->reporter_, pid, to_char_vector(impl_->execution_.command).data());
                // gRPC event update
                impl_->context_.get_cwd()
                    .template map<std::shared_ptr<supervise::Event>>([this, &pid](auto cwd) {
                        return start(pid, impl_->context_.get_ppid(), impl_->execution_, cwd, impl_->context_.get_environment());
                    })
                    .template map<int>([&events](auto event_ptr) {
                        events.push_back(*event_ptr);
                        return 0;
                    });
                return pid;
            })
            .and_then<std::tuple<pid_t, int>>([this](auto pid) {
                return impl_->context_.wait_pid(pid)
                    .template map<std::tuple<pid_t, int>>([&pid](auto exit) {
                        return std::make_tuple(pid, exit);
                    });
            })
            .map<int>([this, &client, &events](auto tuple) {
                const auto& [pid, exit] = tuple;
                report_exit(impl_->reporter_, pid, exit);
                // gRPC event update
                auto event_ptr = stop(exit);
                events.push_back(*event_ptr);
                client.report(events);
                // exit with the client exit code
                return exit;
            });
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
