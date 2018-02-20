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

#pragma once

#include <unistd.h>

#include <ostream>
#include <utility>
#include <memory>
#include <string>

#include "Result.h"


namespace pear {

    class Event;
    using EventPtr = std::unique_ptr<Event>;

    class Event {
    public:
        virtual ~Event() noexcept = default;

        virtual std::ostream &to_json(std::ostream &) const = 0;

    public:
        static Result<EventPtr> start(pid_t pid, const char **cmd) noexcept;
        static Result<EventPtr> stop(pid_t pid, int exit) noexcept;
    };


    class Reporter;
    using ReporterPtr = std::shared_ptr<Reporter>;

    class Reporter {
    public:
        virtual ~Reporter() noexcept = default;

        virtual Result<int> send(EventPtr &event) noexcept = 0;

    public:
        static ReporterPtr tempfile(char const *) noexcept;
    };
}
