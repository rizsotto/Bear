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

#include "Tool.h"

namespace cs {

    struct ToolGcc : public Tool {
        explicit ToolGcc(std::list<fs::path> paths);

        [[nodiscard]]
        bool recognize(const fs::path& program) const override;

        [[nodiscard]]
        rust::Result<output::Entries> compilations(const report::Command &command) const override;

    protected:
        std::list<fs::path> paths;
    };
}
