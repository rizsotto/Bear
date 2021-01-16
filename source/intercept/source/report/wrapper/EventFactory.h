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

#include <cstdint>
#include <unistd.h>
#include <map>
#include <string>
#include <vector>

#include "intercept.pb.h"

namespace rpc {

    struct ExecutionContext {
        std::string command;
        std::vector<std::string> arguments;
        std::string working_directory;
        std::map<std::string, std::string> environment;
    };

    class EventFactory {
    public:
        EventFactory() noexcept;
        ~EventFactory() noexcept = default;

        [[nodiscard]] rpc::Event start(
                pid_t pid,
                pid_t ppid,
                const ExecutionContext &execution) const;

        [[nodiscard]] rpc::Event signal(int number) const;

        [[nodiscard]] rpc::Event terminate(int code) const;

    private:
        uint64_t rid_;
    };
}
