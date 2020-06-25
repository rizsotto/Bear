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

#include "libresult/Result.h"
#include "libsys/Context.h"

namespace wr {

    class Application {
    public:
        static ::rust::Result<Application> create(const char** args, const sys::Context&);

        ::rust::Result<int> operator()() const;

    public:
        Application() = delete;
        ~Application();

        Application(const Application&) = delete;
        Application(Application&&) noexcept;

        Application& operator=(const Application&) = delete;
        Application& operator=(Application&&) noexcept;

    private:
        struct State;

        explicit Application(State*);

    private:
        State const* impl_;
    };
}
