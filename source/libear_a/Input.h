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

namespace ear {

    constexpr char library_flag[] = "-l";
    constexpr char destination_flag[] = "-t";
    constexpr char verbose_flag[] = "-v";

    constexpr char target_env_key[] = "EAR_TARGET";
    constexpr char library_env_key[] = "EAR_LIBRARY";
    constexpr char reporter_env_key[] = "EAR_REPORTER";

    struct Input {
        const char *reporter;
        const char *library;
        const char *destination;
        bool verbose;
    };

}