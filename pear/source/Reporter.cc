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
#include "SystemCalls.h"

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
        pid_t child_;
        pid_t supervisor_;
        pid_t parent_;
        std::string cwd_;
        //std::vector<std::string_view> cmd_;
        const char **cmd_;

        ProcessStartEvent(pid_t child,
                          pid_t supervisor,
                          pid_t parent,
                          std::string cwd,
                          const char **cmd)
                : TimedEvent()
                , child_(child)
                , supervisor_(supervisor)
                , parent_(parent)
                , cwd_(std::move(cwd))
                , cmd_(cmd)
        {}

        std::ostream &to_json(std::ostream &os) const override {
            // TODO: do json escaping of strings.
            // TODO: serialize other attributes too.

            os << R"({ "pid": )" << child_
               << R"(, "cwd": ")" << cwd_ << '\"'
               << R"(, "cmd": )";

            to_json(os, cmd_);

            os << "}";

            return os;
        }

        static std::ostream &to_json(std::ostream &os, const char **array) {
            os << "[";
            for (const char **it = array; *it != nullptr; ++it) {
                if (it != array)
                    os << ", ";
                os << '\"' << *it << '\"';
            }
            os << "]";

            return os;
        }
    };

    struct ProcessStopEvent : public TimedEvent {
        pid_t child_;
        pid_t supervisor_;
        int exit_;

        ProcessStopEvent(pid_t child,
                         pid_t supervisor,
                         int exit)
                : TimedEvent()
                , child_(child)
                , supervisor_(supervisor)
                , exit_(exit)
        {}

        std::ostream &to_json(std::ostream &os) const override {
            // TODO: serialize other attributes too.

            os << R"({ "pid": )" << child_
               << R"(, "exit": )" << exit_
               << "}";

            return os;
        }
    };

    class TempfileReporter : public pear::Reporter {
    public:
        explicit TempfileReporter(const char *target) noexcept;

        pear::Result<int> send(pear::EventPtr &event) noexcept override;
    };

    TempfileReporter::TempfileReporter(const char *target) noexcept {
        // TODO
    }

    pear::Result<int> TempfileReporter::send(pear::EventPtr &event) noexcept {
        // TODO
        return pear::Result<int>::failure(std::runtime_error(""));
    }
}


namespace pear {

    Result<EventPtr> Event::start(pid_t pid, const char **cmd) noexcept {
        Result<pid_t> current_pid = get_pid();
        return current_pid.bind<EventPtr>([&pid, &cmd](auto &current) {
            Result<pid_t> parent_pid = get_ppid();
            return parent_pid.bind<EventPtr>([&pid, &cmd, &current](auto &parent){
                Result<std::string> working_dir = get_cwd();
                return working_dir.map<EventPtr>([&pid, &cmd, &current, &parent](auto &cwd){
                    return EventPtr(new ProcessStartEvent(pid, current, parent, cwd, cmd));
                });
            });
        });
    };

    Result<EventPtr> Event::stop(pid_t pid, int exit) noexcept {
        Result<pid_t> current_pid = get_pid();
        return current_pid.map<EventPtr>([&pid, &exit](auto &current) {
            return EventPtr(new ProcessStopEvent(pid, current, exit));
        });
    }

    ReporterPtr Reporter::tempfile(char const *dir_name) noexcept {
        return std::make_unique<TempfileReporter>(dir_name);
    }
}
