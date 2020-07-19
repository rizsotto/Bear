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
        std::list<std::string> mpi;
        std::list<std::string> cuda;
        std::list<std::string> distcc;
        std::list<std::string> ccache;
        std::list<std::string> cc;
        std::list<std::string> cxx;
        std::list<std::string> fortran;
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
        cfg::Sources sources;
        std::list<Flag> flags_to_filter;
    };

    struct Configuration {
        cfg::Format format;
        cfg::Content content;
        cfg::Compilation compilation;
    };

    // Serialization methods with error mapping.
    rust::Result<int> to_json(const char* file, const Configuration& rhs);
    rust::Result<int> to_json(std::ostream& ostream, const Configuration& rhs);

    rust::Result<Configuration> from_json(const char* file);
    rust::Result<Configuration> from_json(std::istream& istream);
}
