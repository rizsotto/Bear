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
#include "intercept/Flags.h"
#include "collect/Reporter.h"
#include "libsys/Os.h"

#include <spdlog/spdlog.h>

#include <fstream>
#include <memory>
#include <utility>
#include <functional>
#include <unistd.h>

namespace {

    using HostInfo = std::map<std::string, std::string>;

    rust::Result<HostInfo> create_host_info()
    {
        return sys::os::get_uname()
#ifdef HAVE_CS_PATH
            .map<HostInfo>([](auto result) {
                sys::os::get_confstr(_CS_PATH)
                    .map<int>([&result](auto value) {
                        result.insert({ "_CS_PATH", value });
                        return 0;
                    });
                return result;
            })
#endif
#ifdef HAVE_CS_GNU_LIBC_VERSION
            .map<HostInfo>([](auto result) {
                sys::os::get_confstr(_CS_GNU_LIBC_VERSION)
                    .map<int>([&result](auto value) {
                        result.insert({ "_CS_GNU_LIBC_VERSION", value });
                        return 0;
                    });
                return result;
            })
#endif
#ifdef HAVE_CS_GNU_LIBPTHREAD_VERSION
            .map<HostInfo>([](auto result) {
                sys::os::get_confstr(_CS_GNU_LIBPTHREAD_VERSION)
                    .map<int>([&result](auto value) {
                        result.insert({ "_CS_GNU_LIBPTHREAD_VERSION", value });
                        return 0;
                    });
                return result;
            })
#endif
            .map_err<std::runtime_error>([](auto error) {
                return std::runtime_error(fmt::format("failed to get host info: {}", error.what()));
            });
    }

    void update_run_with_started(report::Run& target, const rpc::Event& source)
    {
        spdlog::debug("Received event is merged into execution report. [pid: {}, event: start]", source.pid());
        auto event = report::Event {
            "started",
            source.timestamp(),
            std::nullopt,
            std::nullopt
        };
        target.events.emplace_back(event);
    }

    void update_run_with_signaled(report::Run& target, const rpc::Event& source)
    {
        spdlog::debug("Received event is merged into execution report. [pid: {}, event: signal]", source.pid());
        auto event = report::Event {
            "signaled",
            source.timestamp(),
            std::nullopt,
            { source.signalled().number() }
        };
        target.events.emplace_back(event);
    }

    void update_run_with_terminated(report::Run& target, const rpc::Event& source)
    {
        spdlog::debug("Received event is merged into execution report. [pid: {}, event: stop]", source.pid());
        auto event = report::Event {
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

    inline std::optional<uint32_t> to_optional(google::protobuf::int64 value)
    {
        return (value == 0 ? std::nullopt : std::make_optional(value));
    }

    report::Execution init_execution(const rpc::Event& source)
    {
        const auto& started = source.started();

        auto command = report::Command {
            started.executable(),
            to_list(started.arguments()),
            started.working_dir(),
            to_map(started.environment())
        };
        auto run = report::Run {
            to_optional(source.pid()).value_or(0u),
            to_optional(source.ppid()).value_or(0u),
            std::list<report::Event>()
        };
        update_run_with_started(run, source);

        return report::Execution { command, run };
    }

    report::Execution to_execution(ic::EventsIterator::value_type const &result)
    {
        report::Execution execution;

        if (result.is_ok()) {
            for (const auto& event : result.unwrap()) {
                if (event->has_started()) {
                    execution = init_execution(*event);
                } else if (event->has_terminated()) {
                    update_run_with_terminated(execution.run, *event);
                } else if (event->has_signalled()) {
                    update_run_with_signaled(execution.run, *event);
                }
            }
        } else {
            spdlog::error("reading event is failed: {}", result.unwrap_err().what());
        }

        return execution;
    }
}

namespace ic {

    rust::Result<Reporter::SharedPtr> Reporter::from(const flags::Arguments& flags, const ic::Session& session)
    {
        auto host_info = create_host_info();
        auto output = flags.as_string(OUTPUT);
        auto events = output
                .and_then<EventsDatabase::Ptr>([](auto file) {
                    return EventsDatabase::create(fmt::format("{}.sqlite3", file));
                });

        return merge(host_info, output, events)
            .map<Reporter::SharedPtr>([&session](auto tuple) {
                const auto& [host_info, output, events] = tuple;
                auto context = report::Context { session.get_session_type(), host_info };
                return std::make_shared<Reporter>(output, std::move(context), events);
            });
    }

    Reporter::Reporter(const std::string_view& output,
                       report::Context&& context,
                       ic::EventsDatabase::Ptr events)
            : output_(output)
            , context_(context)
            , events_(std::move(events))
    {
    }

    Reporter::~Reporter() noexcept {
        events_.reset();

        std::error_code error_code;
        fs::remove(fmt::format("{}.sqlite3", output_.string()), error_code);
    }

    void Reporter::report(const rpc::Event& event)
    {
        if (events_) {
            events_->insert_event(event)
                    .on_error([](auto error) {
                        spdlog::warn("Writing event into database failed: {} Ignored.", error.what());
                    });
        }
    }

    void Reporter::flush()
    {
        report::ReportSerializer serializer;
        serializer.to_json(output_, makeReport())
            .on_error([this](auto error) {
                spdlog::warn("Writing output file \"{}\" failed with: {}", output_, error.what());
            });
    }

    report::Report Reporter::makeReport() const
    {
        report::Report report = report::Report { context_, { } };
        if (events_) {
            std::transform(events_->events_by_process_begin(),
                           events_->events_by_process_end(),
                           std::back_inserter(report.executions),
                           to_execution);
        }
        return report;
    }
}
