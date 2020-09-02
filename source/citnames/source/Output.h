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

    // The definition of the JSON compilation database format can be
    // found here:
    //   https://clang.llvm.org/docs/JSONCompilationDatabase.html
    //
    // The entry represents one element of the database. While the
    // database might contains multiple entries (even for the same
    // source file). A list of entries represents a whole compilation
    // database. (No other metadata is provided.)
    //
    // The only unique field in the database the output field can be,
    // but that is an optional field. So, in this sense this is not
    // really a database with keys.
    struct Entry {
        fs::path file;
        fs::path directory;
        std::optional<fs::path> output;
        std::list<std::string> arguments;
    };

    using Entries = std::list<Entry>;

    // Convenient methods for these types.
    bool operator==(const Entry& lhs, const Entry& rhs);

    std::ostream& operator<<(std::ostream&, const Entry&);
    std::ostream& operator<<(std::ostream&, const Entries&);

    // Merge two compilation database without duplicate elements.
    //
    // Duplicate detection is based on the equal operator defined
    // above. (More advanced duplicate detection can be done by
    // checking the output field. This might only work if all entries
    // have this field.)
    Entries merge(const Entries& already_in, const Entries& rhs);

    struct Format {
        bool command_as_array;
        bool drop_output_field;
    };

    struct Content {
        bool include_only_existing_source;
        std::list<fs::path> paths_to_include;
        std::list<fs::path> paths_to_exclude;
    };

    // Utility class to persists JSON compilation database.
    //
    // While the JSON compilation database might have different format
    // (have either "command" or "arguments" fields), this util class
    // provides a simple interface to read any format of the file.
    //
    // It also supports to write different format with configuration
    // parameters. And basic content filtering is also available.
    struct CompilationDatabase {
        CompilationDatabase(const Format&, const Content&);
        virtual ~CompilationDatabase() noexcept = default;

        // Serialization methods with error mapping.
        virtual rust::Result<int> to_json(const fs::path& file, const Entries &entries) const;
        virtual rust::Result<int> to_json(std::ostream &ostream, const Entries &entries) const;

        virtual rust::Result<Entries> from_json(const fs::path& file) const;
        virtual rust::Result<Entries> from_json(std::istream &istream) const;

    private:
        Format format;
        Content content;
    };
}
