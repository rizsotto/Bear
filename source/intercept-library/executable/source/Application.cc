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
#include "Environment.h"
#include "Reporter.h"
#include "er/Flags.h"

#include <spdlog/spdlog.h>

using rust::Err;
using rust::merge;
using rust::Ok;
using rust::Result;

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

    Result<Session> make_session(const ::flags::Arguments& args) noexcept
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

    Result<Execution> make_execution(const ::flags::Arguments& args) noexcept
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

        return merge(session, execution, reporter)
            .map<Application>([&args, &context](auto in) {
                const auto& [session, execution, reporter] = in;
                auto state = new Application::State { session, execution, reporter, context };
                return Application(state);
            });
    }

    rust::Result<int> Application::operator()(const char** envp) const
    {
        auto environment = ::er::Environment::Builder(const_cast<const char**>(envp))
                               .add_reporter(impl_->session_.reporter.data())
                               .add_destination(impl_->session_.destination.data())
                               .add_library(impl_->session_.library.data())
                               .add_verbose(impl_->session_.verbose)
                               .build();

        auto command = to_char_vector(impl_->execution_.command);

        return impl_->context_.spawn(impl_->execution_.path.data(), command.data(), environment->data())
            .map<pid_t>([this](auto& pid) {
                report_start(impl_->reporter_, pid, to_char_vector(impl_->execution_.command).data());
                return pid;
            })
            .and_then<std::tuple<pid_t, int>>([this](auto pid) {
                return impl_->context_.wait_pid(pid)
                    .template map<std::tuple<pid_t, int>>([&pid](auto exit) {
                        return std::make_tuple(pid, exit);
                    });
            })
            .map<int>([this](auto tuple) {
                const auto& [pid, exit] = tuple;
                report_exit(impl_->reporter_, pid, exit);
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
