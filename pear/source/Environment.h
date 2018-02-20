#pragma once
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

#include <bits/unique_ptr.h>

namespace pear {

    class Environment {
    public:
        class Builder;

    private:
        const char **environment_;

        explicit Environment(const char **environment) noexcept;

    public:
        const char **envp() const noexcept;

        Environment(Environment &&) noexcept;

        Environment(Environment const &) = delete;

        Environment &operator=(Environment &&) noexcept = delete;

        Environment &operator=(Environment const &) = delete;

        ~Environment() noexcept;
    };

    using EnvironmentPtr = std::unique_ptr<Environment>;


    class Environment::Builder {
    private:
//        std::vector<std::string> environment_;
//        std::map<std::string, std::string> environment_;

    public:
        Builder() noexcept;

        explicit Builder(const char ** environment) noexcept;

        ~Builder() noexcept = default;

        Builder(Builder &&) noexcept = delete;

        Builder(Builder const &) = delete;

        Builder &operator=(Builder &&) noexcept = delete;

        Builder &operator=(Builder const &) = delete;

    public:
        Builder &add_wrapper(const char *wrapper) noexcept;

        Builder &add_target(const char *target) noexcept;

        Builder &add_library(const char *library) noexcept;

        EnvironmentPtr build() const noexcept;
    };
}
