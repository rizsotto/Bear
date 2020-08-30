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

#include "Configuration.h"
#include "Output.h"
#include "libresult/Result.h"
#include "libreport/Report.h"

#include <filesystem>
#include <list>
#include <memory>

namespace fs = std::filesystem;

namespace cs {

    // Represents a compiler or an executable which produce relevant entries
    // to the compilation database. It can recognize the tool execution from
    // a command line invocation and its context.
    struct Tool {
        virtual ~Tool() noexcept = default;

        // Returns true if the tool is identified from the executable name or path.
        [[nodiscard]]
        virtual bool recognize(const fs::path& program) const = 0;

        // Returns the compilation entries if those were recognised.
        //
        // Can return an optional with an empty list, which says that it was
        // recognized the tool execution, but the execution was not a compilation.
        [[nodiscard]]
        virtual rust::Result<output::Entries> compilations(const report::Command &) const = 0;
    };

    // Represents an expert system which can recognize compilation entries from
    // command executions. It covers multiple tools and consider omit results
    // based on configuration.
    class Tools {
    public:
        static rust::Result<Tools> from(const cfg::Compilation&);

        [[nodiscard]]
        output::Entries transform(const report::Report& report) const;

        [[nodiscard]]
        rust::Result<output::Entries> recognize(const report::Command& command) const;

    public:
        using ToolPtr = std::shared_ptr<Tool>;
        using ToolPtrs = std::list<ToolPtr>;

        Tools() = delete;
        ~Tools() noexcept = default;

        explicit Tools(ToolPtrs&&) noexcept;

    private:
        ToolPtrs tools_;
    };
}
