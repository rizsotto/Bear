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

#include "ToolWrapper.h"
#include "ToolGcc.h"

#include <regex>

namespace cs::semantic {

    const char* ToolWrapper::name() const {
        return "distcc";
    }

    bool ToolWrapper::recognize(const fs::path& program) const {
        static const auto pattern = std::regex(R"(^(distcc|ccache)$)");

        std::cmatch m;
        return std::regex_match(program.filename().c_str(), m, pattern);
    }

    rust::Result<SemanticPtrs> ToolWrapper::compilations(const report::Command &command) const {
        return ToolGcc().compilations(command);
    }
}
