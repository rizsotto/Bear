/*  Copyright (C) 2012-2017 by László Nagy
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

#include <unistd.h>

#include <string>
#include <chrono>

namespace {

    inline
    std::ostream &operator<<(std::ostream &os, pear::Event &event) {
        return event.to_json(os);
    }

    class TimedEvent : public pear::Event {
    private:
        std::chrono::system_clock::time_point const when_;

    public:
        TimedEvent() noexcept
                : when_(std::chrono::system_clock::now())
        { }

        std::chrono::system_clock::time_point const &when() const noexcept {
            return when_;
        }
    };

    struct ProcessStartEvent : public TimedEvent {
        pid_t pid_;
        pid_t ppid_;
        std::string cwd_;
        std::vector<std::string_view> cmd_;

        ProcessStartEvent(pid_t pid_,
                          pid_t ppid_,
                          std::string cwd_,
                          std::vector<std::string_view> cmd_)
                : TimedEvent()
                , pid_(pid_)
                , ppid_(ppid_)
                , cwd_(std::move(cwd_))
                , cmd_(std::move(cmd_))
        {}

        std::ostream &to_json(std::ostream &os) const override {
            // TODO
            return os;
        }
    };

    class TempfileReporter : public pear::Reporter{
    public:
        explicit TempfileReporter(const char *target) noexcept;

        pear::Result<int> send(pear::EventPtr &event) noexcept override;
    };

    TempfileReporter::TempfileReporter(const char *target) noexcept {

    }

    pear::Result<int> TempfileReporter::send(pear::EventPtr &event) noexcept {
        return pear::Result<int>::failure("");
    }
}


namespace pear {


    Result<EventPtr> Event::start(pid_t pid, const char **cmd) noexcept {
        // TODO
        return Result<EventPtr>::failure("");
    };

    Result<EventPtr> Event::stop(pid_t pid, int exit) noexcept {
        // TODO
        return Result<EventPtr>::failure("");
    }

    ReporterPtr Reporter::tempfile(char const *dir_name) noexcept {
        // TODO
        return std::make_unique<TempfileReporter>(dir_name);
    }
}
