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

#include "libmain/Application.h"

#include <spdlog/spdlog.h>

namespace ps {

    template <class App>
    int main(int argc, char* argv[], char* envp[]) {
        App app;
        auto ptr = reinterpret_cast<ps::Application*>(&app);

        return ptr->command(argc,
                            const_cast<const char **>(argv),
                            const_cast<const char **>(envp))
                .and_then<int>([](const ps::CommandPtr &cmd) {
                    return cmd->execute();
                })
                // print out the result of the run
                .on_error([](auto error) {
                    spdlog::error("failed with: {}", error.what());
                })
                .on_success([](auto status_code) {
                    spdlog::debug("succeeded with: {}", status_code);
                })
                // set the return code from error
                .unwrap_or(EXIT_FAILURE);
    }
}
