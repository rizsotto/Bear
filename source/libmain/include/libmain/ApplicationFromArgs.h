/*  Copyright (C) 2012-2021 by László Nagy
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
#include "libmain/Application.h"
#include "libmain/ApplicationLogConfig.h"
#include "libresult/Result.h"
#include "libflags/Flags.h"

namespace ps {

    struct ApplicationFromArgs : Application {
        explicit ApplicationFromArgs(const ApplicationLogConfig&) noexcept;

        rust::Result<CommandPtr> command(int argc, const char** argv, const char** envp) const override;

        virtual rust::Result<flags::Arguments> parse(int argc, const char** argv) const = 0;
        virtual rust::Result<CommandPtr> command(const flags::Arguments &args, const char** envp) const = 0;

        NON_DEFAULT_CONSTRUCTABLE(ApplicationFromArgs)

    protected:
        ApplicationLogConfig log_config_;
    };
}
