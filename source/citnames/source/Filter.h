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

#include <memory>

namespace cs::output {

    struct Content {
        bool include_only_existing_source;
        std::list<fs::path> paths_to_include;
        std::list<fs::path> paths_to_exclude;
    };

    // Represents predicate which decides if the entry shall be placed into the output.
    struct Filter {
        virtual ~Filter() noexcept = default;

        virtual bool operator()(const output::Entry &) noexcept = 0;
    };

    using FilterPtr = std::shared_ptr<Filter>;

    FilterPtr make_filter(const Content &cfg);
}
