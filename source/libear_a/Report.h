/*  Copyright (C) 2012-2017 by László Nagy
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

#include <functional>

#include "libear_a/Session.h"

namespace ear {

    constexpr char file_flag[] = "-f";
    constexpr char search_flag[] = "-s";

    struct Execution {
        const char **command;
        const char *file;
        const char *search_path;
    };

    int serialize(Session const &, Execution const &, std::function<int (const char **)> forward) noexcept;
}
