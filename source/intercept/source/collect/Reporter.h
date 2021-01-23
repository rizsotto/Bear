/*  Copyright (C) 2012-2021 by László Nagy
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

#include "collect/Session.h"
#include "EventsDatabase.h"
#include "libflags/Flags.h"
#include "libresult/Result.h"
#include "intercept.pb.h"

#include <filesystem>
#include <memory>
#include <cstdint>

namespace fs = std::filesystem;

namespace ic {

    // Responsible to collect executions and persist them into an output file.
    class Reporter {
    public:
        using Ptr = std::shared_ptr<Reporter>;
        static rust::Result<Reporter::Ptr> from(const flags::Arguments&);

        void report(const rpc::Event& request);

    public:
        explicit Reporter(ic::EventsDatabase::Ptr events);

        Reporter() = delete;
        ~Reporter() noexcept = default;

        Reporter(const Reporter&) = delete;
        Reporter(Reporter&&) noexcept = delete;

        Reporter& operator=(const Reporter&) = delete;
        Reporter& operator=(Reporter&&) noexcept = delete;

    private:
        ic::EventsDatabase::Ptr events_;
    };
}
