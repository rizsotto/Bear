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

#include "ToolGcc.h"

#include <list>
#include <string>

namespace el {
    class Resolver;
}

namespace cs::semantic {

    struct ToolWrapper : public ToolGcc {
        [[nodiscard]]
        rust::Result<SemanticPtr> recognize(const Execution &execution) const override;

    // visible for testing
    public:
        static bool is_ccache_call(const fs::path &program);
        static bool is_ccache_query(const std::list<std::string> &arguments);

        static bool is_distcc_call(const fs::path &program);
        static bool is_distcc_query(const std::list<std::string> &arguments);

        static domain::Execution remove_wrapper(const domain::Execution &);
        static domain::Execution remove_wrapper(el::Resolver &, const domain::Execution &);
    };
}
