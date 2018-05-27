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

#include <memory>
#include <vector>

namespace pear {

    class Environment {
    public:
        class Builder;

        const char **as_array() const noexcept;

        ~Environment() noexcept = default;

    private:
        explicit Environment(std::vector<std::string> &&environ) noexcept;

    private:
        std::vector<std::string> const environ_;
        std::vector<const char*> rendered_;
    };

    using EnvironmentPtr = std::unique_ptr<Environment>;


    class Environment::Builder {
    public:
        Builder() noexcept;

        explicit Builder(const char ** environment) noexcept;

        ~Builder() noexcept = default;

        Builder &add_reporter(const char *reporter) noexcept;

        Builder &add_target(const char *target) noexcept;

        Builder &add_library(const char *library) noexcept;

        EnvironmentPtr build() const noexcept;

    public:
        Builder(Builder &&) noexcept = delete;

        Builder(Builder const &) = delete;

        Builder &operator=(Builder &&) noexcept = delete;

        Builder &operator=(Builder const &) = delete;

    private:
        std::vector<std::string> environ_;
        std::string reporter_;
        std::string target_;
        std::string library_;
    };
}
