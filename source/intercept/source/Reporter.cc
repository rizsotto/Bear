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

#include "Reporter.h"

#include <spdlog/spdlog.h>

#include <memory>

namespace {

    void update_run_with_started(ic::Execution::Run& target, const supervise::Event& source)
    {
        spdlog::debug("Received event is merged into execution report. [start]");
        ic::Execution::Event event = ic::Execution::Event {
            "start",
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
            "signal",
            source.timestamp(),
            std::nullopt,
            { source.signalled().number() }
        };
        target.events.emplace_back(event);
    }

    void update_run_with_stopped(ic::Execution::Run& target, const supervise::Event& source)
    {
        spdlog::debug("Received event is merged into execution report. [stop]");
        ic::Execution::Event event = ic::Execution::Event {
            "stop",
            source.timestamp(),
            { source.stopped().status() },
            std::nullopt
        };
        target.events.emplace_back(event);
    }

    inline
    std::vector<std::string> to_vector(const google::protobuf::RepeatedPtrField<std::string>& field)
    {
        return std::vector<std::string>(field.begin(), field.end());
    }

    inline
    std::map<std::string, std::string> to_map(const google::protobuf::Map<std::string, std::string>& map)
    {
        return std::map<std::string, std::string>(map.begin(), map.end());
    }

    inline std::optional<int> to_optional(google::protobuf::int64 value)
    {
        return (value == 0 ? std::nullopt : std::make_optional(value));
    }

    ic::Execution::UniquePtr init_execution(const supervise::Event& source)
    {
        const auto& started = source.started();

        auto command = ic::Execution::Command {
            started.executable(),
            to_vector(started.arguments()),
            started.working_dir(),
            to_map(started.environment())
        };
        auto run = ic::Execution::Run {
            to_optional(started.pid()),
            to_optional(started.ppid()),
            std::vector<ic::Execution::Event>()
        };
        update_run_with_started(run, source);

        return std::make_unique<ic::Execution>(ic::Execution { command, run });
    }
}

namespace ic {

    Execution::Builder::Builder()
            : execution_(nullptr)
    {
    }

    Execution::Builder& Execution::Builder::add(supervise::Event const& event)
    {
        if (!execution_ && event.has_started()) {
            execution_ = init_execution(event);
            return *this;
        }
        if (execution_ && event.has_stopped()) {
            update_run_with_stopped(execution_->run, event);
            return *this;
        }
        if (execution_ && event.has_signalled()) {
            update_run_with_signaled(execution_->run, event);
            return *this;
        }
        spdlog::info("Received event could not be merged into execution report. Ignored.");
        return *this;
    }

    Execution::UniquePtr Execution::Builder::build()
    {
        return std::move(execution_);
    }

    void Reporter::report(const Execution::UniquePtr& execution)
    {
        // TODO: build the value before writing it down
    }

    rust::Result<Reporter::SharedPtr> Reporter::from(const flags::Arguments& flags)
    {
        // TODO: parse the command line flags and open the output for
        //       create/append.
        return rust::Err(std::runtime_error("not implemented"));
    }
}
