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

#include "Command.h"
#include "Environment.h"
#include "Reporter.h"
#include "SystemCalls.h"
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

    struct Context {
        const std::string_view reporter;
        const std::string_view destination;
        bool verbose;
    };

    Result<Context> make_context(const ::flags::Arguments& args) noexcept
    {
        return args.as_string(::er::flags::DESTINATION)
            .map<Context>([&args](const auto destination) {
                const auto reporter = args.program();
                const bool verbose = args.as_bool(::er::flags::VERBOSE).unwrap_or(false);
                return Context { reporter, destination, verbose };
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

    Result<pid_t> spawnp(const Execution& config,
        const ::er::EnvironmentPtr& environment) noexcept
    {
        auto command = to_char_vector(config.command);
        return ::er::SystemCalls::spawn(config.path.data(), command.data(), environment->data());
    }

    void report_start(Result<::er::ReporterPtr> const& reporter, pid_t pid, const char** cmd) noexcept
    {
        merge(reporter, ::er::Event::start(pid, cmd))
            .and_then<int>([](auto tuple) {
                const auto& [rptr, eptr] = tuple;
                return rptr->send(eptr);
            })
            .unwrap_or_else([](auto message) {
                spdlog::warn("report process start failed: ", message.what());
                return 0;
            });
    }

    void report_exit(Result<::er::ReporterPtr> const& reporter, pid_t pid, int exit) noexcept
    {
        merge(reporter, ::er::Event::stop(pid, exit))
            .and_then<int>([](auto tuple) {
                const auto& [rptr, eptr] = tuple;
                return rptr->send(eptr);
            })
            .unwrap_or_else([](auto message) {
                spdlog::error("report process stop failed: ", message.what());
                return 0;
            });
    }
}

namespace er {

    struct Command::State {
        Context context_;
        Execution execution_;
        std::string_view library_;
    };

    ::rust::Result<Command> Command::create(const ::flags::Arguments& params)
    {
        return merge(make_context(params), make_execution(params), params.as_string(::er::flags::LIBRARY))
            .map<Command>([&params](auto in) {
                const auto& [context, execution, library] = in;
                auto state = new Command::State { context, execution, library };
                return Command(state);
            });
    }

    ::rust::Result<int> Command::operator()(const char** envp) const
    {
        auto reporter = ::er::Reporter::tempfile(impl_->context_.destination.data());

        auto environment = ::er::Environment::Builder(const_cast<const char**>(envp))
                               .add_reporter(impl_->context_.reporter.data())
                               .add_destination(impl_->context_.destination.data())
                               .add_verbose(impl_->context_.verbose)
                               .add_library(impl_->library_.data())
                               .build();

        return spawnp(impl_->execution_, environment)
            .map<pid_t>([this, &reporter](auto& pid) {
                report_start(reporter, pid, to_char_vector(impl_->execution_.command).data());
                return pid;
            })
            .and_then<std::tuple<pid_t, int>>([](auto pid) {
                return ::er::SystemCalls::wait_pid(pid)
                    .template map<std::tuple<pid_t, int>>([&pid](auto exit) {
                        return std::make_tuple(pid, exit);
                    });
            })
            .map<int>([&reporter](auto tuple) {
                const auto& [pid, exit] = tuple;
                report_exit(reporter, pid, exit);
                return exit;
            });
    }

    Command::Command(Command::State* const impl)
            : impl_(impl)
    {
    }

    Command::Command(Command&& rhs) noexcept
            : impl_(rhs.impl_)
    {
        rhs.impl_ = nullptr;
    }

    Command& Command::operator=(Command&& rhs) noexcept
    {
        if (&rhs != this) {
            delete impl_;
            impl_ = rhs.impl_;
        }
        return *this;
    }

    Command::~Command()
    {
        delete impl_;
        impl_ = nullptr;
    }
}
