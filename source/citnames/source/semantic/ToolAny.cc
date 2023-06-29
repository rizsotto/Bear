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

#include "ToolAny.h"

#include <algorithm>

namespace cs::semantic {

    ToolAny::ToolAny(ToolAny::ToolPtrs &&tools, std::list<fs::path> &&to_exclude) noexcept
            : tools_(tools)
            , to_exclude_(to_exclude)
    { }

    rust::Result<SemanticPtr> ToolAny::recognize(const domain::Execution &execution, const BuildTarget target) const {
        // do different things if the execution is matching one of the nominated compilers.
        if (to_exclude_.end() != std::find(to_exclude_.begin(), to_exclude_.end(), execution.executable)) {
            return rust::Err(std::runtime_error("The tool is on the exclude list from configuration."));
        } else {
            // check if any tool can recognize the execution.
            for (const auto &tool : tools_) {
                auto result = tool->recognize(execution, target);
                // return if it recognized in any way.
                if (Tool::recognized_ok(result) || Tool::recognized_with_error(result)) {
                    return result;
                }
            }
        }
        return rust::Err(std::runtime_error("No tools recognize this execution."));
    }
}
