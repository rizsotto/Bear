/*  Copyright (C) 2012-2022 by László Nagy
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
#include <libflags/Flags.h>

#include <iosfwd>
#include <list>
#include <map>
#include <string>
#include <optional>
#include <utility>

#include "Citnames-config.h"

namespace config {

    // Represents the application configuration.
    struct Configuration {
        Citnames citnames;

        static rust::Result<Configuration> load_config(const flags::Arguments& args);
        //std::string to_json();
    };

    // Convenient methods for these types.
    std::ostream& operator<<(std::ostream&, const Configuration&);
}
