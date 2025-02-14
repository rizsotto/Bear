/*  Copyright (C) 2012-2024 by László Nagy
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
#include "libresult/Result.h"

#include <filesystem>
#include <iosfwd>
#include <list>
#include <optional>
#include <string>
#include <nlohmann/json.hpp>

namespace fs = std::filesystem;

namespace cs {

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

    // Represents a linking command in the database
    struct LinkEntry {
        fs::path directory;
        std::optional<fs::path> output;
        std::list<std::string> arguments;
        std::list<fs::path> input_files;
    };

    // Represents an ar command in the database for creating static libraries
    struct ArEntry {
        fs::path directory;
        std::optional<fs::path> output;  // The output .a file
        std::list<std::string> arguments;
        std::list<fs::path> input_files;  // Object files to archive
        std::string operation;  // The ar operation (e.g., "r", "q", etc.)
    };

    // Convenient methods for these types.
    bool operator==(const Entry& lhs, const Entry& rhs);
    bool operator==(const LinkEntry& lhs, const LinkEntry& rhs);
    bool operator==(const ArEntry& lhs, const ArEntry& rhs);
    std::ostream& operator<<(std::ostream&, const Entry&);
    std::ostream& operator<<(std::ostream&, const LinkEntry&);
    std::ostream& operator<<(std::ostream&, const ArEntry&);

    // JSON serialization functions
    void from_json(const nlohmann::json &j, Entry &entry);
    void from_json(const nlohmann::json &j, LinkEntry &entry);
    void from_json(const nlohmann::json &j, ArEntry &entry);
    void to_json(nlohmann::json &j, const Entry &entry, const Format &format);
    void to_json(nlohmann::json &j, const LinkEntry &entry, const Format &format);
    void to_json(nlohmann::json &j, const ArEntry &entry, const Format &format);
    nlohmann::json to_json(const Entry &rhs, const Format &format);
    nlohmann::json to_json(const LinkEntry &rhs, const Format &format);
    nlohmann::json to_json(const ArEntry &rhs, const Format &format);

    // Validation functions
    void validate(const Entry &entry);
    void validate(const LinkEntry &entry);
    void validate(const ArEntry &entry);

    // Utility class to persists JSON compilation database.
    //
    // While the JSON compilation database might have different format
    // (have either "command" or "arguments" fields), this util class
    // provides a simple interface to read any format of the file.
    //
    // It also supports to write different format with configuration
    // parameters. And basic content filtering is also available.
    struct CompilationDatabase {
        using Entries = std::list<Entry>;
        using LinkEntries = std::list<LinkEntry>;
        using ArEntries = std::list<ArEntry>;

        CompilationDatabase(Format, Content);
        virtual ~CompilationDatabase() noexcept = default;

        // Serialization methods with error mapping.
        [[nodiscard]] virtual rust::Result<size_t> to_json(const fs::path& file, const Entries &entries) const;
        [[nodiscard]] virtual rust::Result<size_t> to_json(std::ostream &ostream, const Entries &entries) const;
        [[nodiscard]] virtual rust::Result<size_t> to_link_json(const fs::path& file, const LinkEntries &entries) const;
        [[nodiscard]] virtual rust::Result<size_t> to_link_json(std::ostream &ostream, const LinkEntries &entries) const;
        [[nodiscard]] virtual rust::Result<size_t> to_ar_json(const fs::path& file, const ArEntries &entries) const;
        [[nodiscard]] virtual rust::Result<size_t> to_ar_json(std::ostream &ostream, const ArEntries &entries) const;

        [[nodiscard]] virtual rust::Result<size_t> from_json(const fs::path& file, Entries &entries) const;
        [[nodiscard]] virtual rust::Result<size_t> from_json(std::istream &istream, Entries &entries) const;
        [[nodiscard]] virtual rust::Result<size_t> from_link_json(const fs::path& file, LinkEntries &entries) const;
        [[nodiscard]] virtual rust::Result<size_t> from_link_json(std::istream &istream, LinkEntries &entries) const;
        [[nodiscard]] virtual rust::Result<size_t> from_ar_json(const fs::path& file, ArEntries &entries) const;
        [[nodiscard]] virtual rust::Result<size_t> from_ar_json(std::istream &istream, ArEntries &entries) const;

    private:
        Format format;
        Content content;
    };
}
