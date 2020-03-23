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

    class Parser;

    class Arguments {
    public:
        [[nodiscard]] std::string_view program() const;

        [[nodiscard]] rust::Result<bool> as_bool(const std::string_view& key) const;
        [[nodiscard]] rust::Result<int> as_int(const std::string_view& key) const;
        [[nodiscard]] rust::Result<std::string_view> as_string(const std::string_view& key) const;
        [[nodiscard]] rust::Result<std::vector<std::string_view>> as_string_list(const std::string_view& key) const;

        ~Arguments() = default;

        Arguments(const Arguments&) = default;
        Arguments(Arguments&&) noexcept = default;

        Arguments& operator=(const Arguments&) = default;
        Arguments& operator=(Arguments&&) noexcept = default;

    private:
        using Parameter = std::vector<std::string_view>;
        using Parameters = std::map<std::string_view, Parameter>;

        friend class Parser;

        Arguments(std::string_view&& program, Parameters&& parameters);

    private:
        std::string_view program_;
        Parameters parameters_;
    };

    struct Option {
        int arguments;
        bool hidden;
        const std::string_view help;
        const std::optional<std::string_view> default_value;
    };

    using OptionMap = std::map<std::string_view, Option>;
    using OptionValue = OptionMap::value_type;

    class Parser {
    public:
        Parser(std::string_view name, std::initializer_list<OptionValue> options);
        ~Parser() = default;

        rust::Result<Arguments> parse(int argc, const char** argv) const;

        void print_help(std::ostream&, bool expose_hidden) const;
        void print_help_short(std::ostream&) const;

        // TODO: deprecate it
        std::string help() const;

    public:
        Parser() = delete;
        Parser(const Parser&) = delete;
        Parser(Parser&&) noexcept = delete;

        Parser& operator=(const Parser&) = delete;
        Parser& operator=(Parser&&) noexcept = delete;

    private:
        const std::string_view name_;
        const OptionMap options_;
    };
}
