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

#include "report/wrapper/EventFactory.h"

#include <fmt/chrono.h>
#include <fmt/format.h>

#include <chrono>
#include <random>

namespace {

    std::uint64_t generate_unique_id() {
        std::random_device random_device;
        std::mt19937_64 generator(random_device());
        std::uniform_int_distribution<std::uint64_t> distribution;

        return distribution(generator);
    }

    std::string now_as_string() {
        const auto now = std::chrono::system_clock::now();
        auto micros = std::chrono::duration_cast<std::chrono::microseconds>(now.time_since_epoch());

        return fmt::format("{:%Y-%m-%dT%H:%M:%S}.{:06d}Z",
                           fmt::localtime(std::chrono::system_clock::to_time_t(now)),
                           micros.count() % 1000000);
    }
}

namespace wr {

    EventFactory::EventFactory() noexcept
            : rid_(generate_unique_id())
    { }

    rpc::Event EventFactory::start(ProcessId pid, ProcessId ppid, const Execution &execution) const {
        rpc::Event result;
        result.set_rid(rid_);
        result.set_pid(pid);
        result.set_ppid(ppid);
        result.set_timestamp(now_as_string());
        {
            auto event = std::make_unique<rpc::Event_Started>();
            event->set_executable(execution.program);
            for (const auto &argument : execution.arguments) {
                event->add_arguments(argument.data());
            }
            event->set_working_dir(execution.working_dir);
            event->mutable_environment()->insert(execution.environment.begin(), execution.environment.end());

            result.set_allocated_started(event.release());
        }
        return result;
    }

    rpc::Event EventFactory::signal(int number) const {
        rpc::Event result;
        result.set_rid(rid_);
        result.set_timestamp(now_as_string());
        {
            auto event = std::make_unique<rpc::Event_Signalled>();
            event->set_number(number);

            result.set_allocated_signalled(event.release());
        }
        return result;
    }

    rpc::Event EventFactory::terminate(int code) const {
        rpc::Event result;
        result.set_rid(rid_);
        result.set_timestamp(now_as_string());
        {
            auto event = std::make_unique<rpc::Event_Terminated>();
            event->set_status(code);

            result.set_allocated_terminated(event.release());
        }
        return result;
    }
}
