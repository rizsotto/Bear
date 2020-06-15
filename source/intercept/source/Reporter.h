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
#include "Session.h"
#include "libflags/Flags.h"
#include "libresult/Result.h"
#include "libsys/Context.h"
#include "librpc/supervise.pb.h"

namespace ic {

    // Responsible to collect executions and persist them into an output file.
    class Reporter {
    public:
        using SharedPtr = std::shared_ptr<Reporter>;
        static rust::Result<Reporter::SharedPtr> from(const flags::Arguments&, const sys::Context&, const ic::Session&);

        void report(const ::supervise::Event& request);
        void flush();

    public:
        Reporter() = delete;
        virtual ~Reporter() noexcept = default;

        Reporter(const Reporter&) = delete;
        Reporter(Reporter&&) noexcept = delete;

        Reporter& operator=(const Reporter&) = delete;
        Reporter& operator=(Reporter&&) noexcept = delete;

    protected:
        // These methods are visible for testing...
        Reporter(const std::string_view& view, ic::Context&& context);

        void flush(std::ostream&) const;
        [[nodiscard]] ic::Report makeReport() const;

    private:
        std::string output_;
        ic::Context context_;
        std::map<pid_t, Execution> executions_;
    };
}
