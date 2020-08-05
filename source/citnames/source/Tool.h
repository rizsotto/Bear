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

#include "CompilationDatabase.h"
#include "libresult/Result.h"
#include "libreport/Report.h"

#include <filesystem>
#include <optional>
#include <string>
#include <regex>

namespace fs = std::filesystem;

namespace cs {

    // Represents a compiler or an executable which produce relevant entries
    // to the compilation database. It can recognize the tool execution from
    // a command line invocation and its context.
    struct Tool {
        virtual ~Tool() noexcept = default;

        // Returns the compilation entries if those were recognised.
        //
        // Can return an optional with an empty list, which says that it was
        // recognized the tool execution, but the execution was not a compilation.
        [[nodiscard]]
        virtual rust::Result<output::Entries> recognize(const report::Command &) const = 0;
    };

    struct GnuCompilerCollection : public Tool {
        explicit GnuCompilerCollection(std::list<fs::path> paths);

        [[nodiscard]]
        rust::Result<output::Entries> recognize(const report::Command &command) const override;

        [[nodiscard]]
        bool recognize(const fs::path& program) const;

    protected:
        std::list<fs::path> paths;
    };
}
