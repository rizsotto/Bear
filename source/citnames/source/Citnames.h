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

#include "Output.h"
#include "semantic/Tool.h"
#include "libmain/SubcommandFromArgs.h"
#include "libresult/Result.h"
#include "libsys/Environment.h"

#include <filesystem>
#include <utility>

#include "citnames/citnames-forward.h"

namespace fs = std::filesystem;

namespace cs {

    struct Arguments {
        fs::path input;
        fs::path output;
        bool append;
        bool update;
    };

    struct Command : ps::Command {
        Command(Arguments arguments, cs::Configuration configuration) noexcept;

        [[nodiscard]] rust::Result<int> execute() const override;

        NON_DEFAULT_CONSTRUCTABLE(Command)
        NON_COPYABLE_NOR_MOVABLE(Command)

    private:
        Arguments arguments_;
        cs::Configuration configuration_;
    };
}
