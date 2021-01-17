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

#include "Convert.h"

namespace domain {

    Execution from(const rpc::Execution &input) noexcept {
        return Execution{
                fs::path(input.executable()),
                std::vector(input.arguments().begin(), input.arguments().end()),
                fs::path(input.working_dir()),
                std::map(input.environment().begin(), input.environment().end())
        };
    }

    rpc::Execution into(const Execution &input) noexcept {
        rpc::Execution result;
        result.set_executable(input.executable.string());
        result.mutable_arguments()->Reserve(input.arguments.size());
        for (const auto &argument : input.arguments) {
            result.add_arguments(argument);
        }
        result.set_working_dir(input.working_dir);
        result.mutable_environment()->insert(input.environment.begin(), input.environment.end());
        return result;
    }
}
