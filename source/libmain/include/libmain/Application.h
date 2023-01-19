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

#include "libresult/Result.h"
#include "libflags/Flags.h"

#include <memory>

namespace ps {

    struct Command {
        virtual ~Command() noexcept = default;

        [[nodiscard]]
        virtual rust::Result<int> execute() const = 0;
    };

    using CommandPtr = std::unique_ptr<Command>;

    struct Application {
        virtual ~Application() noexcept = default;

        [[nodiscard]]
        virtual rust::Result<CommandPtr> command(int argc, const char** argv) const = 0;
    };

    struct Subcommand {
        virtual ~Subcommand() noexcept = default;

        [[nodiscard]]
        virtual rust::Result<CommandPtr> subcommand(const flags::Arguments &argv) const = 0;
    };
}
