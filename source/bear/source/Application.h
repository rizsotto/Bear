/*  Copyright (C) 2012-2024 by László Nagy
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
#include "libflags/Flags.h"
#include "libresult/Result.h"
#include "libsys/Environment.h"
#include "libsys/Process.h"
#include "libsys/Signal.h"
#include "libmain/ApplicationFromArgs.h"

#include <spdlog/spdlog.h>
#include <spdlog/sinks/stdout_sinks.h>

#include <filesystem>
#include <optional>
#include <string_view>
#include <utility>

namespace bear {

    struct Command : ps::Command {
    public:
        Command(const sys::Process::Builder& intercept, const sys::Process::Builder& citnames, fs::path output) noexcept;

        [[nodiscard]] rust::Result<int> execute() const override;

        NON_DEFAULT_CONSTRUCTABLE(Command)
        NON_COPYABLE_NOR_MOVABLE(Command)

    private:
        sys::Process::Builder intercept_;
        sys::Process::Builder citnames_;
        fs::path output_;
    };

    struct Application : ps::ApplicationFromArgs {
        Application();

        rust::Result<flags::Arguments> parse(int argc, const char **argv) const override;

        rust::Result<ps::CommandPtr> command(const flags::Arguments &args, const char **envp) const override;
    };
}
