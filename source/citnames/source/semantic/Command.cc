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

#include "Command.h"

#include <iostream>

#include <nlohmann/json.hpp>

namespace cs::semantic {

    bool operator==(const Command& lhs, const Command& rhs)
    {
        return (lhs.program == rhs.program)
               && (lhs.arguments == rhs.arguments)
               && (lhs.working_dir == rhs.working_dir)
               && (lhs.environment == rhs.environment);
    }

    std::ostream& operator<<(std::ostream& os, const Command& rhs)
    {
        nlohmann::json payload = nlohmann::json {
                { "program", rhs.program },
                { "arguments", nlohmann::json(rhs.arguments) },
                { "working_dir", rhs.working_dir },
        };
        os << payload;
        return os;
    }
}
