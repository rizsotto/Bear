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

#include "Domain.h"
#include "libmain/Application.h"
#include "libmain/ApplicationLogConfig.h"
#include "libflags/Flags.h"
#include "libresult/Result.h"

namespace wr {
    using namespace domain;

    struct Command : ps::Command {
        Command(wr::SessionLocator session, wr::Execution execution) noexcept;

        [[nodiscard]] rust::Result<int> execute() const override;

        NON_DEFAULT_CONSTRUCTABLE(Command)
        NON_COPYABLE_NOR_MOVABLE(Command)

    protected:
        wr::SessionLocator session_;
        wr::Execution execution_;
    };

    struct Application : ps::Application {
        Application() noexcept;
        rust::Result<ps::CommandPtr> command(int argc, const char **argv) const override;

        static rust::Result<ps::CommandPtr> from_envs(int argc, const char **argv);
        static rust::Result<ps::CommandPtr> from_args(const flags::Arguments &args);
        static rust::Result<flags::Arguments> parse(int argc, const char **argv);

    private:
        ps::ApplicationLogConfig const &log_config;
    };
}
