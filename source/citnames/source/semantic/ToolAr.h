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

#include "Tool.h"
#include "Parsers.h"

namespace cs::semantic {

    struct ToolAr : public Tool {

        [[nodiscard]]
        rust::Result<SemanticPtr> recognize(const Execution &execution, const BuildTarget target) const override;

        [[nodiscard]]
        static bool is_linker_call(const fs::path& program);

    protected:

        [[nodiscard]]
        static rust::Result<SemanticPtr> linking(const FlagsByName &flags, const Execution &execution);

        static const FlagsByName FLAG_DEFINITION;
    };
}
