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

#include <libresult/Result.h>

#include <list>
#include <map>
#include <string>
#include <optional>

namespace cs::cfg {

    struct Format {
        bool command_as_array;
        bool drop_output_field;
    };

    struct Content {
        std::optional<std::string> relative_to;
        bool include_only_existing_source;
        bool include_only_successful_commands;
        std::list<std::string> paths_to_include;
        std::list<std::string> paths_to_exclude;
    };

    struct ExpandWrappers {
        bool mpi;
        bool cuda;
        bool ccache;
        bool distcc;
    };

    struct Compilers {
        std::optional<std::string> cc;
        std::optional<std::string> cxx;
        std::optional<std::string> fortran;
    };

    struct Sources {
        std::list<std::string> extensions_to_exclude;
        std::list<std::string> extensions_to_include;
    };

    struct Flag {
        std::string flag;
        std::string pattern;
        std::string clazz;
        bool split;
        int count;
    };

    struct Compilation {
        cfg::ExpandWrappers expand_wrappers;
        cfg::Compilers compilers;
        std::list<Flag> flags_to_filter;
    };

    struct Value {
        cfg::Format format;
        cfg::Content content;
        cfg::Compilation compilation;
    };

    struct Configuration {
        virtual ~Configuration() noexcept = default;

        // Serialization methods with error mapping.
        virtual rust::Result<int> to_json(const char *file, const Value &rhs) const;
        virtual rust::Result<int> to_json(std::ostream &ostream, const Value &rhs) const;

        virtual rust::Result<Value> from_json(const char *file) const;
        virtual rust::Result<Value> from_json(std::istream &istream) const;
    };

    // Create a default value.
    Value default_value(const std::map<std::string, std::string>& environment);

    // Returns list of violations of semantic.
    std::list<std::string> validate(const cfg::Value& value);

    // Methods used in tests.
    bool operator==(const Value& lhs, const Value& rhs);
    bool operator==(const Compilation& lhs, const Compilation& rhs);
    bool operator==(const Flag& lhs, const Flag& rhs);
    bool operator==(const Sources& lhs, const Sources& rhs);
    bool operator==(const Compilers& lhs, const Compilers& rhs);
    bool operator==(const ExpandWrappers& lhs, const ExpandWrappers& rhs);
    bool operator==(const Content& lhs, const Content& rhs);
    bool operator==(const Format& lhs, const Format& rhs);
}
