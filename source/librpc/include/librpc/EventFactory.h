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

#include "librpc/supervise.pb.h"

namespace rpc {

    class EventFactory {
    public:
        EventFactory() noexcept;
        ~EventFactory() noexcept = default;

        [[nodiscard]] supervise::Event start(
                pid_t pid,
                pid_t ppid,
                const std::string &command,
                const std::vector<std::string> &arguments,
                const std::string &working_directory,
                const std::map<std::string, std::string> &environment) const;

        [[nodiscard]] supervise::Event signal(int number) const;

        [[nodiscard]] supervise::Event terminate(int code) const;

    private:
        uint64_t rid_;
    };
}
