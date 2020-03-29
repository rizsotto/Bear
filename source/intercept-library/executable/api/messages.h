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

#pragma once

#include <memory>
#include <string>

namespace er {
    namespace messages {

        struct Event;
        using EventPtr = std::shared_ptr<Event>;

        struct Message {
            // either: started, stopped, signalled.
            std::string type;
            // iso time format with milliseconds.
            std::string at;
            // the content of the event defined under.
            EventPtr event;
        };

        struct Event {
            pid_t pid;

            virtual ~Event() = default;
        };

        struct ProcessStarted : public Event {
            // executable
            // arguments
            // environment
            // parent pid ???
        };

        struct ProcessStopped : public Event {
            // exit status
        };

        struct ProcessSignalled : public Event {
            // signal number
        };
    }
}
