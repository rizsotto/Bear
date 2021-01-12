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
#include "semantic/Command.h"
#include "semantic/Semantic.h"
#include "libresult/Result.h"
#include "intercept/EventsDatabase.h"

#include <filesystem>
#include <list>
#include <memory>

namespace fs = std::filesystem;

namespace cs::semantic {

    // Represents a compiler or an executable which produce relevant entries
    // to the compilation database. It can recognize the tool execution from
    // a command line invocation and its context.
    struct Tool {
        virtual ~Tool() noexcept = default;

        // Returns the tool name.
        [[nodiscard]]
        virtual const char* name() const = 0;

        // Returns true if the tool is identified from the executable name or path.
        [[nodiscard]]
        virtual bool recognize(const fs::path &program) const = 0;

        // Returns the compilation entries if those were recognised.
        //
        // Can return an optional with an empty list, which says that it was
        // recognized the tool execution, but the execution was not a compilation.
        [[nodiscard]]
        virtual rust::Result<SemanticPtrs> compilations(const Command &) const = 0;
    };

    // Represents an expert system which can recognize compilation entries from
    // command executions. It covers multiple tools and consider omit results
    // based on configuration.
    class Tools {
    public:
        Tools() = delete;
        ~Tools() noexcept = default;

        static rust::Result<Tools> from(Compilation cfg);

        [[nodiscard]]
        Entries transform(ic::EventsDatabase::Ptr events) const;

    private:
        using ToolPtr = std::shared_ptr<Tool>;
        using ToolPtrs = std::list<ToolPtr>;

        Tools(ToolPtrs &&, std::list<fs::path>&&) noexcept;

//        [[nodiscard]]
//        rust::Result<SemanticPtrs> recognize(const report::Execution &execution) const;
//
//        [[nodiscard]]
//        rust::Result<ToolPtr> select(const report::Command &command) const;

    private:
        ToolPtrs tools_;
        std::list<fs::path> to_exclude_;
    };
}
