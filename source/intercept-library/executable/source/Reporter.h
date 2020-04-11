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

#include "libresult/Result.h"
#include "libsys/Context.h"

#include <unistd.h>

#include <iosfwd>
#include <memory>
#include <utility>

namespace er {

    class Event {
    public:
        using SharedPtr = std::shared_ptr<Event>;

        virtual ~Event() noexcept = default;

        virtual const char* name() const = 0;
        virtual void to_json(std::ostream&) const = 0;
    };

    class Reporter {
    public:
        using SharedPtr = std::shared_ptr<Reporter>;

        virtual ~Reporter() noexcept = default;

        virtual rust::Result<int> send(Event::SharedPtr event) noexcept = 0;

        virtual rust::Result<Event::SharedPtr> start(pid_t pid, const char** cmd) = 0;
        virtual rust::Result<Event::SharedPtr> stop(pid_t pid, int exit) = 0;

    public:
        static rust::Result<SharedPtr> from(char const* path, const sys::Context& context) noexcept;
    };
}
