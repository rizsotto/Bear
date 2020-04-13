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

#include "Report.h"
#include "libflags/Flags.h"
#include "libresult/Result.h"

#include "supervise.pb.h"

namespace ic {

    // A helper to build an execution object. Takes the raw process execution
    // events and builds the execution object defined above.
    class Execution::Builder {
    public:
        Builder();
        ~Builder() = default;

        Builder& add(supervise::Event const& event);

        Execution::UniquePtr build();

    private:
        Execution::UniquePtr execution_;
    };

    // Responsible to collect executions and persist them into an output file.
    class Reporter {
    public:
        using SharedPtr = std::shared_ptr<Reporter>;
        static rust::Result<Reporter::SharedPtr> from(const flags::Arguments&);

        void set_host_info(const std::map<std::string, std::string>&);
        void set_session_type(const std::string& name);

        // MT-safe method to add a new execution and persist into the output file.
        void report(const Execution::UniquePtr& execution);

    public:
        Reporter() = delete;
        ~Reporter();

        Reporter(const Reporter&) = delete;
        Reporter(Reporter&&) noexcept = delete;

        Reporter& operator=(const Reporter&) = delete;
        Reporter& operator=(Reporter&&) noexcept = delete;

    private:
        struct State;
        explicit Reporter(State*);

    private:
        State* impl_;
    };
}
