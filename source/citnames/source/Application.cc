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

#include "Application.h"

namespace cs {

    struct Application::State {
    };

    rust::Result<Application> Application::create(const char** args, const sys::Context& ctx)
    {
        return rust::Err(std::runtime_error("TODO"));
    }

    rust::Result<int> Application::operator()() const
    {
        return rust::Err(std::runtime_error("TODO"));
    }

    Application::Application(Application::State* const impl)
        : impl_(impl)
    {
    }

    Application::Application(Application&& rhs) noexcept
        : impl_(rhs.impl_)
    {
        rhs.impl_ = nullptr;
    }

    Application& Application::operator=(Application&& rhs) noexcept
    {
        if (&rhs != this) {
            delete impl_;
            impl_ = rhs.impl_;
        }
        return *this;
    }

    Application::~Application()
    {
        delete impl_;
        impl_ = nullptr;
    }
}
