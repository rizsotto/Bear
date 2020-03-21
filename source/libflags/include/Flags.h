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

#include <map>
#include <string_view>
#include <tuple>
#include <vector>

#include "Result.h"

namespace flags {

    constexpr char PROGRAM_KEY[] = "program";

    using Parameter = std::tuple<const char**, const char**>;
    using Parameters = std::map<std::string_view, Parameter>;

    struct Option {
        int arguments;
        const char* help;
    };

    class Parser {
    public:
        using OptionMap = std::map<std::string_view, Option>;
        using OptionValue = OptionMap::value_type;

    public:
        Parser(std::initializer_list<OptionValue> options);
        ~Parser() = default;

        rust::Result<Parameters> parse(int argc, const char** argv) const noexcept;

        std::string help(const char* name) const noexcept;

    public:
        Parser() = delete;
        Parser(const Parser&) = delete;
        Parser(Parser&&) noexcept = delete;

        Parser& operator=(const Parser&) = delete;
        Parser& operator=(Parser&&) noexcept = delete;

    private:
        const OptionMap options_;
    };
}
