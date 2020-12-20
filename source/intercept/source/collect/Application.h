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

#include "libmain/ApplicationFromArgs.h"
#include "Session.h"
#include "Reporter.h"

#include <utility>
#include <vector>
#include <string_view>

namespace ic {

    struct Command : ps::Command {
        Command(std::vector<std::string_view> command,
                Session::SharedPtr session,
                Reporter::SharedPtr reporter)
                : ps::Command()
                , command_(std::move(command))
                , session_(session)
                , reporter_(reporter)
        { }

        [[nodiscard]] rust::Result<int> execute() const override;

    private:
        std::vector<std::string_view> command_;
        Session::SharedPtr session_;
        Reporter::SharedPtr reporter_;
    };

    struct Application : ps::ApplicationFromArgs {
        Application() noexcept;
        rust::Result<flags::Arguments> parse(int argc, const char **argv) const override;
        rust::Result<ps::CommandPtr> command(const flags::Arguments &args, const char **envp) const override;
    };
}
