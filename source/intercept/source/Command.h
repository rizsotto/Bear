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

#include "Flags.h"
#include "Result.h"

#include <memory>

namespace ic {

    class Command {
    public:
        static ::rust::Result<Command> create(const ::flags::Arguments& args);

        ::rust::Result<int> operator()() const;

    public:
        Command() = delete;
        ~Command();

        Command(const Command&) = delete;
        Command(Command&&) noexcept;

        Command& operator=(const Command&) = delete;
        Command& operator=(Command&&) noexcept;

    private:
        struct State;

        explicit Command(State*);

    private:
        State const* impl_;
    };
}
