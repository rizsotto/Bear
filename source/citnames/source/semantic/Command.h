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

#include <filesystem>
#include <list>
#include <map>
#include <string>
#include <iosfwd>

namespace fs = std::filesystem;

namespace cs::semantic {

    // This represents the executed command itself. Describes all the
    // context that the caller was given. And these are the those parameters
    // which are essential for re-run the command.
    struct Command {
        fs::path program;
        std::list<std::string> arguments;
        fs::path working_dir;
        std::map<std::string, std::string> environment;
    };

    bool operator==(const Command& lhs, const Command& rhs);
    std::ostream& operator<<(std::ostream&, const Command&);
}
