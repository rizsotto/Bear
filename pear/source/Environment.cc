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

#include "Environment.h"

#include <unistd.h>

namespace {
    constexpr const char **array_end(const char **begin) noexcept {
        const char **result = begin;
        while (*result != nullptr)
            ++result;
        return result;
    }
}

namespace pear {
    Environment::Environment(const char **const environment) noexcept
            : environment_(environment)
    { }

    Environment::~Environment() noexcept {
        if (environment_ == nullptr)
            return;

        for (const char **it = environment_; *it != nullptr; ++it)
            delete *it;

        delete environment_;
        environment_ = nullptr;
    }

    const char **Environment::as_array() const noexcept {
        return environment_;
    }


    Environment::Builder::Builder() noexcept
            : Environment::Builder(const_cast<const char **>(environ)) {}

    Environment::Builder::Builder(const char **environment) noexcept
//            : environment_(environment, array_end(environment)) {}
    {
        // TODO
    }

    Environment::Builder &Environment::Builder::add_wrapper(const char *wrapper) noexcept {
        // TODO
        return *this;
    }

    Environment::Builder &Environment::Builder::add_target(const char *target) noexcept {
        // TODO
        return *this;
    }

    Environment::Builder &Environment::Builder::add_library(const char *library) noexcept {
        // TODO
        return *this;
    }

    EnvironmentPtr Environment::Builder::build() const noexcept {
        // TODO
        return std::unique_ptr<Environment>(new Environment(nullptr));
    }

}
