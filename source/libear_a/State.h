/*  Copyright (C) 2012-2017 by László Nagy
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

#include "libear_a/String.h"
#include "libear_a/Session.h"

namespace ear {

    class State {
    public:
        static State* capture(void *) noexcept;

        LibrarySession get_input() const noexcept;

    public:
        State() noexcept = delete;

        ~State() noexcept = default;

        State(const State &) = delete;

        State(State &&) noexcept = delete;

        State &operator=(const State &) = delete;

        State &operator=(State &&) noexcept = delete;

    protected:
        State(const char *target,
              const char *library,
              const char *reporter,
              bool verbose) noexcept;

        static const char **current() noexcept;

        static State* create(const char**, void *) noexcept;

        static const char *get_env(const char **, const char *) noexcept;

    private:
        ::ear::String<4096> target_;
        ::ear::String<8192> library_;
        ::ear::String<8192> reporter_;
        bool verbose_;
    };

}