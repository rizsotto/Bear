/*  Copyright (C) 2012-2023 by László Nagy
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

#include "config.h"
#include <map>
#include <string>

namespace sys::env {

    // Memory resource guard class.
    //
    // The OS expect `const char**`, but the caller usually manipulated
    // the values in different form. This class let the caller use a more
    // convenient form (`std::map<std::string, std::string>`)  to use,
    // but makes the final `const char**` not leak.
    class Guard {
    public:
        explicit Guard(const std::map<std::string, std::string> &environment);
        ~Guard() noexcept;

        [[nodiscard]] const char** data() const;

    public:
        NON_DEFAULT_CONSTRUCTABLE(Guard)
        NON_COPYABLE_NOR_MOVABLE(Guard)

    private:
        const char** data_;
    };
}
