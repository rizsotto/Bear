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

#include "Flags.h"

#include <cstring>
#include <optional>

namespace {

    std::string spaces(size_t num) noexcept
    {
        std::string result;
        for (; num > 0; --num)
            result += ' ';
        return result;
    }

    std::optional<flags::Parameter>
    take(const flags::Option& option, const char** const begin, const char** const end) noexcept
    {
        return (option.arguments < 0)
            ? std::optional(std::make_tuple(begin, end))
            : (begin + option.arguments > end)
                ? std::nullopt
                : std::optional(std::make_tuple(begin, begin + option.arguments));
    }

    std::string format_option_line(const flags::OptionValue & optionValue) noexcept
    {
        // TODO: Print out how many arguments it takes
        const flags::Option &option = std::get<1>(optionValue);
        const std::string_view &flag = std::get<0>(optionValue);

        const size_t flag_size = flag.length();

        std::string result;
        result += spaces(2);
        result += flag;
        result += (flag_size > 22)
            ? "\n" + spaces(15)
            : spaces(23 - flag_size);
        result += std::string(option.help) + "\n";
        return result;
    }
}

namespace flags {

    Parser::Parser(std::initializer_list<OptionValue> options)
            : options_(options)
    {
    }

    rust::Result<Parameters> Parser::parse(const int argc, const char** argv) const noexcept
    {
        Parameters result;
        if (argc < 2 || argv == nullptr) {
            return rust::Err(std::runtime_error("Empty parameter list."));
        }
        result.emplace(Parameters::key_type(PROGRAM_KEY), std::make_tuple(argv, argv + 1));
        const char** const args_end = argv + argc;
        for (const char** args_it = ++argv; args_it != args_end;) {
            // find which option is it.
            if (auto option = options_.find(*args_it); option != options_.end()) {
                if (const auto params = take(option->second, args_it + 1, args_end); params) {
                    result.emplace(Parameters::key_type(*args_it), params.value());
                    args_it = std::get<1>(params.value());
                } else {
                    return rust::Err(std::runtime_error((std::string("Not enough parameters for flag: ") + *args_it)));
                }
            } else {
                return rust::Err(std::runtime_error((std::string("Unrecognized parameter: ") + *args_it)));
            }
        }
        return rust::Ok(std::move(result));
    }

    std::string Parser::help(const char* const name) const noexcept
    {
        std::string result;
        result += std::string("Usage: ") + name + std::string(" [OPTION]\n\n");
        std::for_each(options_.begin(), options_.end(), [&result](auto it) {
            result += format_option_line(it);
        });
        return result;
    }
}
