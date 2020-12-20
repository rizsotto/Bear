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
#include "libsys/Environment.h"
#include "libsys/Path.h"
#include "libsys/Process.h"
#include "libsys/Signal.h"

#include <memory>

namespace {

    rust::Result<rpc::Session> make_session(const ::flags::Arguments &args) noexcept {
        return args.as_string(er::DESTINATION)
                .map<rpc::Session>([](const auto &destination) {
                    return rpc::Session{std::string(destination)};
                });
    }

    rust::Result<rpc::ExecutionContext>
    make_execution(const ::flags::Arguments &args, sys::env::Vars &&environment) noexcept {
        auto path = args.as_string(er::EXECUTE)
                .map<std::string>([](auto file) { return std::string(file); });
        auto command = args.as_string_list(er::COMMAND)
                .map<std::vector<std::string>>([](auto args) {
                    return std::vector<std::string>(args.begin(), args.end());
                });
        auto working_dir = sys::path::get_cwd();

        return merge(path, command, working_dir)
                .map<rpc::ExecutionContext>([&environment](auto tuple) {
                    const auto&[_path, _command, _working_dir] = tuple;
                    return rpc::ExecutionContext{_path, _command, _working_dir.string(), std::move(environment)};
                });
    }
}

namespace er {

    rust::Result<int> Command::execute() const {
        rpc::EventFactory event_factory;
        rpc::InterceptClient client(session_);

        auto result = client.get_environment_update(context_.environment)
                .map<rpc::ExecutionContext>([this](auto environment) {
                    return rpc::ExecutionContext{
                            context_.command,
                            context_.arguments,
                            context_.working_directory,
                            environment
                    };
                })
                .and_then<sys::Process>([&client, &event_factory](auto execution) {
                    return sys::Process::Builder(execution.command)
                            .add_arguments(execution.arguments.begin(), execution.arguments.end())
                            .set_environment(execution.environment)
#ifdef SUPPORT_PRELOAD
                            .spawn_with_preload()
#else
                            .spawn()
#endif
                            .on_success([&client, &event_factory, &execution](auto &child) {
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
                        if (status.template map<bool>([](auto _status) { return _status.is_exited(); }).unwrap_or(
                                false)) {
                            return status;
                        }
                    }
                })
                .map<int>([](auto status) {
                    return status.code().value_or(EXIT_FAILURE);
                });

        return result;
    }

    Application::Application() noexcept
            : ps::ApplicationFromArgs(ps::ApplicationLogConfig("er", "er"))
    { }

    rust::Result<flags::Arguments> Application::parse(int argc, const char **argv) const {
        const flags::Parser parser("er", VERSION, {
                {::er::DESTINATION, {1,  true, "path to report directory",           std::nullopt, std::nullopt}},
                {::er::EXECUTE,     {1,  true, "the path parameter for the command", std::nullopt, std::nullopt}},
                {::er::COMMAND,     {-1, true, "the executed command",               std::nullopt, std::nullopt}}
        });
        return parser.parse_or_exit(argc, const_cast<const char **>(argv));
    }

    rust::Result<ps::CommandPtr> Application::command(const flags::Arguments &args, const char **envp) const {
        auto environment = sys::env::from(const_cast<const char **>(envp));
        return rust::merge(make_session(args), make_execution(args, std::move(environment)))
                .map<ps::CommandPtr>([](auto in) {
                    const auto&[session, execution] = in;
                    return std::make_unique<Command>(session, execution);
                });
    }
}
