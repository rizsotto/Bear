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
#include "libmain/ApplicationLogConfig.h"
#include "libsys/Environment.h"
#include "libsys/Path.h"
#include "libsys/Process.h"
#include "libsys/Signal.h"
#include "Environment.h"
#include "Flags.h"

#include <spdlog/spdlog.h>

#include <filesystem>
#include <memory>
#include <string>

namespace {

    struct ApplicationLogConfig : ps::ApplicationLogConfig {

        ApplicationLogConfig()
                : ps::ApplicationLogConfig("wrapper", "wr")
        { }

        void initForVerbose() const override {
            spdlog::set_pattern(fmt::format("[%H:%M:%S.%f, wr, {0}, ppid: {1}] %v", getpid(), getppid()));
            spdlog::set_level(spdlog::level::debug);
        }
    };

    const ApplicationLogConfig APPLICATION_LOG_CONFIG;

    bool is_wrapper_call(int argc, const char **argv) {
        if (argc > 0) {
            auto cmd = fs::path(argv[0]);
            auto prg = cmd.filename();
            return prg != fs::path("wrapper");
        }
        return false;
    }

    struct Wrapper : wr::Command {
        Wrapper(const rpc::Session &session, const rpc::ExecutionContext &context)
                : Command(session, context)
        { }

        [[nodiscard]] rust::Result<rpc::ExecutionContext> context() const override {
            rpc::InterceptClient client(session_);

            auto command = client.get_wrapped_command(context_.command);
            auto environment = client.get_environment_update(context_.environment);

            return rust::merge(command, environment)
                    .map<rpc::ExecutionContext>([this](auto tuple) {
                        const auto&[command, environment] = tuple;
                        auto arguments = context_.arguments;
                        arguments.front() = command;
                        return rpc::ExecutionContext{
                                command,
                                arguments,
                                context_.working_directory,
                                environment
                        };
                    });
        }

        static rust::Result<rpc::Session> make_session(const sys::env::Vars& environment) noexcept
        {
            auto destination = environment.find(wr::env::KEY_DESTINATION);
            return (destination == environment.end())
                   ? rust::Result<rpc::Session>(rust::Err(std::runtime_error("Unknown destination.")))
                   : rust::Result<rpc::Session>(rust::Ok(rpc::Session { destination->second }));
        }

        static std::vector<std::string> from(const char** args)
        {
            const char** end = args;
            while (*end != nullptr)
                ++end;
            return std::vector<std::string>(args, end);
        }

        static rust::Result<rpc::ExecutionContext> make_execution(const char** args, sys::env::Vars&& environment) noexcept
        {
            auto path = fs::path(args[0]).string();
            auto command = from(args);
            auto working_dir = sys::path::get_cwd();

            return working_dir
                    .map<rpc::ExecutionContext>([&path, &command, &environment](auto cwd) {
                        return rpc::ExecutionContext {path, command, cwd.string(), environment };
                    });
        }
    };

    struct Supervisor : wr::Command {
        Supervisor(const rpc::Session &session, const rpc::ExecutionContext &context)
                : Command(session, context)
        { }

        [[nodiscard]] rust::Result<rpc::ExecutionContext> context() const override {
            rpc::InterceptClient client(session_);

            return client.get_environment_update(context_.environment)
                    .map<rpc::ExecutionContext>([this](auto environment) {
                        return rpc::ExecutionContext{
                                context_.command,
                                context_.arguments,
                                context_.working_directory,
                                environment
                        };
                    });
        }

        static rust::Result<rpc::Session> make_session(const ::flags::Arguments &args) noexcept {
            return args.as_string(wr::DESTINATION)
                    .map<rpc::Session>([](const auto &destination) {
                        return rpc::Session{std::string(destination)};
                    });
        }

        static rust::Result<rpc::ExecutionContext>
        make_execution(const ::flags::Arguments &args, sys::env::Vars &&environment) noexcept {
            auto path = args.as_string(wr::EXECUTE)
                    .map<std::string>([](auto file) { return std::string(file); });
            auto command = args.as_string_list(wr::COMMAND)
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
    };
}

namespace wr {

    Command::Command(rpc::Session session, rpc::ExecutionContext context) noexcept
            : ps::Command()
            , session_(std::move(session))
            , context_(std::move(context))
    { }

    rust::Result<int> Command::execute() const {
        rpc::EventFactory event_factory;
        rpc::InterceptClient client(session_);

        return context()
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
                        if (status.template map<bool>([](auto _status) { return _status.is_exited(); }).unwrap_or(false)) {
                            return status;
                        }
                    }
                })
                .map<int>([](auto status) {
                    return status.code().value_or(EXIT_FAILURE);
                });
    }

    Application::Application() noexcept
            : ps::Application()
            , log_config(APPLICATION_LOG_CONFIG)
    {
        log_config.initForSilent();
    }

    rust::Result<ps::CommandPtr> Application::command(int argc, const char **argv, const char **envp) const {
        if (const bool wrapper = is_wrapper_call(argc, argv); wrapper) {
            if (const bool verbose = (nullptr != getenv(wr::env::KEY_VERBOSE)); verbose) {
                log_config.initForVerbose();
            }
            log_config.record(argv, envp);

            return from_envs(argc, argv, envp);
        } else {
            return Application::parse(argc, argv)
                    .on_success([this, &argv, &envp](const auto& args) {
                        if (args.as_bool(flags::VERBOSE).unwrap_or(false)) {
                            log_config.initForVerbose();
                        }
                        log_config.record(argv, envp);
                        spdlog::debug("arguments parsed: {0}", args);
                    })
                    .and_then<ps::CommandPtr>([&envp](auto args) {
                        // if parsing success, we create the main command and execute it.
                        return Application::from_args(args, envp);
                    });
        }
    }

    rust::Result<ps::CommandPtr> Application::from_envs(int, const char **argv, const char **envp) {
        auto environment = sys::env::from(const_cast<const char **>(envp));
        auto session = Wrapper::make_session(environment);
        auto execution = Wrapper::make_execution(argv, std::move(environment));

        return rust::merge(session, execution)
                .map<ps::CommandPtr>([](auto in) {
                    const auto& [session, execution] = in;
                    return std::make_unique<Wrapper>(session, execution);
                });
    }

    rust::Result<ps::CommandPtr> Application::from_args(const flags::Arguments &args, const char **envp) {
        auto environment = sys::env::from(const_cast<const char **>(envp));
        auto session = Supervisor::make_session(args);
        auto execution = Supervisor::make_execution(args, std::move(environment));

        return rust::merge(session, execution)
                .map<ps::CommandPtr>([](auto in) {
                    const auto&[session, execution] = in;
                    return std::make_unique<Supervisor>(session, execution);
                });
    }

    rust::Result<flags::Arguments> Application::parse(int argc, const char **argv) {
        const flags::Parser parser("wrapper", VERSION, {
                {DESTINATION, {1,  true, "path to report directory",   std::nullopt, std::nullopt}},
                {EXECUTE,     {1,  true, "the path to the executable", std::nullopt, std::nullopt}},
                {COMMAND,     {-1, true, "the command arguments",      std::nullopt, std::nullopt}},
        });
        return parser.parse_or_exit(argc, const_cast<const char **>(argv));
    }
}
