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

#include "config.h"
#include "Reporter.h"
#include "Application.h"

#include <nlohmann/json.hpp>
#include <spdlog/spdlog.h>

#include <fstream>
#include <iomanip>
#include <memory>
#include <utility>
#include <functional>
#include <unistd.h>

namespace {

    using HostInfo = std::map<std::string, std::string>;

    rust::Result<HostInfo> create_host_info(const sys::Context& context)
    {
        return context.get_uname()
#ifdef HAVE_CS_PATH
            .map<HostInfo>([&context](auto result) {
                context.get_confstr(_CS_PATH)
                    .map<int>([&result](auto value) {
                        result.insert({ "_CS_PATH", value });
                        return 0;
                    });
                return result;
            })
#endif
#ifdef HAVE_CS_GNU_LIBC_VERSION
            .map<HostInfo>([&context](auto result) {
                context.get_confstr(_CS_GNU_LIBC_VERSION)
                    .map<int>([&result](auto value) {
                        result.insert({ "_CS_GNU_LIBC_VERSION", value });
                        return 0;
                    });
                return result;
            })
#endif
#ifdef HAVE_CS_GNU_LIBPTHREAD_VERSION
            .map<HostInfo>([&context](auto result) {
                context.get_confstr(_CS_GNU_LIBPTHREAD_VERSION)
                    .map<int>([&result](auto value) {
                        result.insert({ "_CS_GNU_LIBPTHREAD_VERSION", value });
                        return 0;
                    });
                return result;
            })
#endif
            .map_err<std::runtime_error>([](auto error) {
                return std::runtime_error("failed to get host info.");
            });
    }

    void update_run_with_started(ic::Execution::Run& target, const supervise::Event& source)
    {
        spdlog::debug("Received event is merged into execution report. [start]");
        ic::Execution::Event event = ic::Execution::Event {
            "started",
            source.timestamp(),
            std::nullopt,
            std::nullopt
        };
        target.events.emplace_back(event);
    }

    void update_run_with_signaled(ic::Execution::Run& target, const supervise::Event& source)
    {
        spdlog::debug("Received event is merged into execution report. [signal]");
        ic::Execution::Event event = ic::Execution::Event {
            "signaled",
            source.timestamp(),
            std::nullopt,
            { source.signalled().number() }
        };
        target.events.emplace_back(event);
    }

    void update_run_with_terminated(ic::Execution::Run& target, const supervise::Event& source)
    {
        spdlog::debug("Received event is merged into execution report. [stop]");
        ic::Execution::Event event = ic::Execution::Event {
            "terminated",
            source.timestamp(),
            { source.terminated().status() },
            std::nullopt
        };
        target.events.emplace_back(event);
    }

    inline std::list<std::string> to_list(const google::protobuf::RepeatedPtrField<std::string>& field)
    {
        return std::list<std::string>(field.begin(), field.end());
    }

    inline std::map<std::string, std::string> to_map(const google::protobuf::Map<std::string, std::string>& map)
    {
        return std::map<std::string, std::string>(map.begin(), map.end());
    }

    inline std::optional<int> to_optional(google::protobuf::int64 value)
    {
        return (value == 0 ? std::nullopt : std::make_optional(value));
    }

    ic::Execution init_execution(const supervise::Event& source)
    {
        const auto& started = source.started();

        auto command = ic::Execution::Command {
            started.executable(),
            to_list(started.arguments()),
            started.working_dir(),
            to_map(started.environment())
        };
        auto run = ic::Execution::Run {
            to_optional(source.pid()).value_or(0),
            to_optional(source.ppid()),
            std::list<ic::Execution::Event>()
        };
        update_run_with_started(run, source);

        return ic::Execution { command, run };
    }
}

namespace ic {

    rust::Result<Reporter::SharedPtr> Reporter::from(const flags::Arguments& flags, const sys::Context& ctx, const ic::Session& session)
    {
        auto host_info = create_host_info(ctx);
        auto output = flags.as_string(Application::OUTPUT);

        return merge(host_info, output)
            .map<Reporter::SharedPtr>([&session](auto pair) {
                const auto& [host_info, output] = pair;
                auto context = ic::Context { session.get_session_type(), host_info };
                return Reporter::SharedPtr(new Reporter(output, std::move(context)));
            });
    }

    Reporter::Reporter(const std::string_view& output, ic::Context&& context)
            : output_(output)
            , context_(context)
            , executions_()
    {
    }

    void Reporter::report(const ::supervise::Event& event)
    {
        const pid_t pid = event.pid();
        if (auto it = executions_.find(pid); it != executions_.end()) {
            // the process entry exits
            if (event.has_terminated()) {
                update_run_with_terminated(it->second.run, event);
            } else if (event.has_signalled()) {
                update_run_with_signaled(it->second.run, event);
            } else {
                spdlog::info("Received start event could not be merged into execution report. Ignored.");
            }
        } else {
            // the process entry not exists
            if (event.has_started()) {
                auto entry = init_execution(event);
                executions_.emplace(std::make_pair(pid, std::move(entry)));
            } else {
                spdlog::info("Received event could not be merged into execution report. Ignored.");
            }
        }
    }

    void Reporter::flush()
    {
        std::ofstream targetFile(output_);
        targetFile << std::setw(4);

        flush(targetFile);
    }

    void Reporter::flush(std::ostream& stream)
    {
        ic::Report report = ic::Report { context_, { } };
        std::transform(executions_.begin(), executions_.end(),
                       std::back_inserter(report.executions),
                       [](auto pid_execution_pair) { return pid_execution_pair.second; });

        nlohmann::json j = report;

        stream << j << std::endl;
    }
}
