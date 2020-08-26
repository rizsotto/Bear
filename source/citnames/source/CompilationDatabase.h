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

#include <filesystem>
#include <iosfwd>
#include <list>
#include <optional>
#include <string>

namespace fs = std::filesystem;

namespace cs::output {

    struct Format {
        bool command_as_array;
        bool drop_output_field;
    };

    struct Entry {
        fs::path file;
        fs::path directory;
        std::optional<fs::path> output;
        std::list<std::string> arguments;
    };

    using Entries = std::list<Entry>;

    // Merge two compilation database without duplicate elements.
    Entries merge(const Entries& lhs, const Entries& rhs);

    // Convenient methods for these types.
    bool operator==(const Entry& lhs, const Entry& rhs);

    std::ostream& operator<<(std::ostream&, const Entry&);
    std::ostream& operator<<(std::ostream&, const Entries&);

    // Utility class to persists entries.
    struct CompilationDatabase {
        explicit CompilationDatabase(const Format&);
        virtual ~CompilationDatabase() noexcept = default;

        // Serialization methods with error mapping.
        virtual rust::Result<int> to_json(const fs::path& file, const Entries &entries) const;
        virtual rust::Result<int> to_json(std::ostream &ostream, const Entries &entries) const;

        virtual rust::Result<Entries> from_json(const fs::path& file) const;
        virtual rust::Result<Entries> from_json(std::istream &istream) const;

    private:
        Format format;
    };
}
