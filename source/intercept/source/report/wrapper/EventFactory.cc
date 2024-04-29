/*  Copyright (C) 2012-2024 by László Nagy
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
#include "report/wrapper/EventFactory.h"
#include "Convert.h"

#include <random>
#ifdef HAVE_SYS_TIME_H
#include <sys/time.h>
#else
#include <google/protobuf/util/time_util.h>
#endif

using namespace google::protobuf::util;

namespace {

    google::protobuf::Timestamp now() {
#ifdef HAVE_SYS_TIME_H
        timeval tv;
        gettimeofday(&tv, nullptr);

        google::protobuf::Timestamp timestamp;
        timestamp.set_seconds(tv.tv_sec);
        timestamp.set_nanos(tv.tv_usec * 1000);
        return timestamp;
#else
        return TimeUtil::GetCurrentTime();
#endif
    }

    std::uint64_t generate_unique_id() {
        std::random_device random_device;
        std::mt19937_64 generator(random_device());
        std::uniform_int_distribution<std::uint64_t> distribution;

        return distribution(generator);
    }
}

namespace wr {

    EventFactory::EventFactory() noexcept
            : rid_(generate_unique_id())
    { }

    rpc::Event EventFactory::start(ProcessId pid, ProcessId ppid, const Execution &execution) const {
        rpc::Event event;
        event.set_rid(rid_);
        event.mutable_timestamp()->CopyFrom(now());
        {
            rpc::Event_Started &event_started = *event.mutable_started();
            event_started.set_pid(pid);
            event_started.set_ppid(ppid);
            *event_started.mutable_execution() = into(execution);
        }
        return event;
    }

    rpc::Event EventFactory::signal(int number) const {
        rpc::Event event;
        event.set_rid(rid_);
        event.mutable_timestamp()->CopyFrom(now());
        {
            rpc::Event_Signalled &event_signalled = *event.mutable_signalled();
            event_signalled.set_number(number);
        }
        return event;
    }

    rpc::Event EventFactory::terminate(int code) const {
        rpc::Event event;
        event.set_rid(rid_);
        event.mutable_timestamp()->CopyFrom(now());
        {
            rpc::Event_Terminated &event_terminated = *event.mutable_terminated();
            event_terminated.set_status(code);
        }
        return event;
    }
}
