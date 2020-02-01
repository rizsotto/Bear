#pragma once
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

#include <memory>
#include <map>

namespace pear {

    class Environment {
    public:
        class Builder;

        const char **data() const noexcept;

    public:
        Environment() = delete;

        ~Environment() noexcept;

        Environment(Environment &&) noexcept = delete;

        Environment(Environment const &) = delete;

        Environment &operator=(Environment &&) noexcept = delete;

        Environment &operator=(Environment const &) = delete;

    private:
        explicit Environment(const std::map<std::string, std::string> &environ) noexcept;

    private:
        char **const data_;
    };

    using EnvironmentPtr = std::unique_ptr<Environment>;


    class Environment::Builder {
    public:
        explicit Builder(const char **environment) noexcept;

        Builder &add_reporter(const char *reporter) noexcept;

        Builder &add_destination(const char *target) noexcept;

        Builder &add_verbose(bool verbose) noexcept;

        Builder &add_library(const char *library) noexcept;

        Builder &add_cc_compiler(const char *compiler, const char *wrapper) noexcept;

        Builder &add_cxx_compiler(const char *compiler, const char *wrapper) noexcept;

        EnvironmentPtr build() const noexcept;

    public:
        Builder() noexcept = delete;

        ~Builder() noexcept = default;

        Builder(Builder &&) noexcept = delete;

        Builder(Builder const &) = delete;

        Builder &operator=(Builder &&) noexcept = delete;

        Builder &operator=(Builder const &) = delete;

    private:
        std::map<std::string, std::string> environ_;
    };
}
