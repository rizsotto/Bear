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

#include "libmain/Application.h"
#include "libmain/ApplicationLogConfig.h"
#include "report/EventFactory.h"
#include "report/InterceptClient.h"

namespace wr {

    struct Command : ps::Command {
        Command(rpc::Session session, rpc::ExecutionContext context)
                : ps::Command()
                , session_(session)
                , context_(context)
        { }

        [[nodiscard]] rust::Result<int> execute() const override;

    private:
        rpc::Session session_;
        rpc::ExecutionContext context_;
    };

    struct Application : ps::Application {
        Application() noexcept;
        rust::Result<ps::CommandPtr> command(int argc, const char **argv, const char **envp) const override;

    private:
        ps::ApplicationLogConfig log_config;
    };
}
