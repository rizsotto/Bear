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

#include "Config.h"
#include "libresult/Result.h"

#include <iosfwd>
#include <list>
#include <optional>
#include <string>

namespace cs::output {

    struct Entry {
        std::string file;
        std::string directory;
        std::optional<std::string> output;
        std::list<std::string> arguments;
    };

    using CompilationDatabase = std::list<Entry>;

    // Serialization methods with error mapping.
    rust::Result<int> to_json(const char *file, const CompilationDatabase &entries, const cs::cfg::Format& format);
    rust::Result<int> to_json(std::ostream &ostream, const CompilationDatabase &entries, const cs::cfg::Format& format);

    rust::Result<CompilationDatabase> from_json(const char *file);
    rust::Result<CompilationDatabase> from_json(std::istream &istream);

    // Merge two compilation database without duplicate elements.
    CompilationDatabase merge(const CompilationDatabase& lhs, const CompilationDatabase& rhs);

    // Methods used in tests.
    bool operator==(const Entry& lhs, const Entry& rhs);
}
