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

    std::optional<std::tuple<const char**, const char**>>
    take(const flags::Option& option, const char** const begin, const char** const end) noexcept
    {
        return (option.arguments < 0)
            ? std::optional(std::make_tuple(begin, end))
            : (begin + option.arguments > end)
                ? std::nullopt
                : std::optional(std::make_tuple(begin, begin + option.arguments));
    }

    std::string fill_space(size_t num) noexcept
    {
        std::string result;
        for (; num > 0; --num)
            result += ' ';
        return result;
    }

    std::string format_option_line(const flags::OptionValue& optionValue) noexcept
    {
        const auto& [flag, option] = optionValue;
        const size_t flag_size = flag.length();

        // TODO: Print out how many arguments it takes
        std::string result;
        result += fill_space(2);
        result += flag;
        result += (flag_size > 22)
            ? "\n" + fill_space(15)
            : fill_space(23 - flag_size);
        result += std::string(option.help) + "\n";
        return result;
    }
}

namespace flags {

    Arguments::Arguments(const std::string_view& program)
            : program_(program)
            , parameters_()
    {
    }

    std::string_view Arguments::program() const
    {
        return std::string_view(program_);
    }

    rust::Result<bool> Arguments::as_bool(const std::string_view& key) const
    {
        return rust::Ok(parameters_.find(key) != parameters_.end());
    }

    rust::Result<int> Arguments::as_int(const std::string_view& key) const
    {
        // TODO: use fmt to write proper error message
        if (auto values = parameters_.find(key); values != parameters_.end()) {
            return (values->second.size() == 1)
                ? rust::Result<std::string>(rust::Ok(std::string(values->second.front())))
                      .map<int>([](auto str) { return std::stoi(str); })
                : rust::Result<int>(rust::Err(std::runtime_error("Parameter is not a single integer.")));
        }
        return rust::Result<int>(rust::Err(std::runtime_error("Parameter is not available.")));
    }

    rust::Result<std::string_view> Arguments::as_string(const std::string_view& key) const
    {
        // TODO: use fmt to write proper error message
        if (auto values = parameters_.find(key); values != parameters_.end()) {
            return (values->second.size() == 1)
                ? (rust::Ok(values->second.front()))
                : rust::Result<std::string_view>(rust::Err(std::runtime_error("Parameter is not a single string.")));
        }
        return rust::Result<std::string_view>(rust::Err(std::runtime_error("Parameter is not available.")));
    }

    rust::Result<std::vector<std::string_view>> Arguments::as_string_list(const std::string_view& key) const
    {
        // TODO: use fmt to write proper error message
        if (auto values = parameters_.find(key); values != parameters_.end()) {
            return rust::Ok(values->second);
        }
        return rust::Result<std::vector<std::string_view>>(rust::Err(std::runtime_error("Parameter is not available.")));
    }

    Arguments& Arguments::add(const std::string_view& key, const char** begin, const char** end)
    {
        auto args = std::vector<std::string_view>(begin, end);
        parameters_.emplace(key, args);

        return *this;
    }

    Parser::Parser(std::string_view name, std::initializer_list<OptionValue> options)
            : name_(name)
            , options_(options)
    {
    }

    rust::Result<Arguments> Parser::parse(const int argc, const char** argv) const
    {
        if (argc < 1 || argv == nullptr) {
            return rust::Err(std::runtime_error("Empty argument list."));
        }
        Arguments result(argv[0]);
        const char** const args_end = argv + argc;
        for (const char** args_it = ++argv; args_it != args_end;) {
            // find which option is it.
            if (auto option = options_.find(*args_it); option != options_.end()) {
                if (const auto params = take(option->second, args_it + 1, args_end); params) {
                    result.add(*args_it, std::get<0>(params.value()), std::get<1>(params.value()));
                    args_it = std::get<1>(params.value());
                } else {
                    return rust::Err(std::runtime_error((std::string("Not enough parameters for flag: ") + *args_it)));
                }
            } else {
                return rust::Err(std::runtime_error((std::string("Unrecognized parameter: ") + *args_it)));
            }
        }
        // TODO: add default values to the result
        return rust::Ok(result);
    }

    void Parser::print_help(std::ostream& os, bool expose_hidden) const
    {
        // TODO: implement it
    }

    void Parser::print_help_short(std::ostream& os) const
    {
        // TODO: implement it
    }

    std::string Parser::help() const
    {
        std::string result;
        result += std::string("Usage: ") + std::string(name_) + std::string(" [OPTION]\n\n");
        std::for_each(options_.begin(), options_.end(), [&result](auto it) {
            result += format_option_line(it);
        });
        return result;
    }
}
