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

#include "collect/Session.h"
#include "intercept/EventsDatabase.h"
#include "libflags/Flags.h"
#include "intercept/output/Report.h"
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
        using SharedPtr = std::shared_ptr<Reporter>;
        static rust::Result<Reporter::SharedPtr> from(const flags::Arguments&, const ic::Session&);

        void report(const rpc::Event& request);
        void flush();

    public:
        Reporter() = delete;
        virtual ~Reporter() noexcept;

        Reporter(const Reporter&) = delete;
        Reporter(Reporter&&) noexcept = delete;

        Reporter& operator=(const Reporter&) = delete;
        Reporter& operator=(Reporter&&) noexcept = delete;

        Reporter(const std::string_view& view,
                 report::Context&& context,
                 ic::EventsDatabase::Ptr events);

        [[nodiscard]] report::Report makeReport() const;

    private:
        fs::path output_;
        report::Context context_;
        ic::EventsDatabase::Ptr events_;
    };
}
