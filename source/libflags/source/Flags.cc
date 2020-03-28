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
#include <iostream>
#include <numeric>
#include <list>
#include <set>
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

    std::string format_option_line(const flags::OptionValue& optionValue) noexcept
    {
        const auto& [flag, option] = optionValue;
        const size_t flag_size = flag.length();

        // TODO: Print out how many arguments it takes
        std::string result;
        result += std::string(2, ' ');
        result += flag;
        result += (flag_size > 22)
            ? "\n" + std::string(15, ' ')
            : std::string(23 - flag_size, ' ');
        result += std::string(option.help) + "\n";
        return result;
    }

    std::list<flags::OptionValue> order_by_relevance(const flags::OptionMap& options, const std::optional<std::string_view>& group) {
        std::list<flags::OptionValue> result;
        std::copy_if(std::begin(options), std::end(options),
                     std::back_inserter(result),
                     [&group](auto &option) { return option.second.group_name == group && option.second.arguments >= 0; });
        std::copy_if(std::begin(options), std::end(options),
                     std::back_inserter(result),
                     [&group](auto &option) { return option.second.group_name == group && option.second.arguments < 0; });
        return result;
    }

    std::list<std::list<flags::OptionValue>> group_by(const flags::OptionMap& options)
    {
        // find out what are the option groups.
        std::set<std::string_view> groups;
        for (const auto &option : options) {
            if (auto group = option.second.group_name; group) {
                groups.emplace(group.value());
            }
        }
        std::list<std::list<flags::OptionValue>> result;
        // insert to the result list.
        result.emplace_back(order_by_relevance(options, std::nullopt));
        for (auto& group : groups) {
            result.emplace_back(order_by_relevance(options, std::optional(group)));
        }
        return result;
    }

    void format_options(std::ostream& os, const std::list<flags::OptionValue>& main_options)
    {
        for (auto& it : main_options) {
            const auto& [flag, option] = it;
            const bool optional = !option.required;
            // mark optional with braces
            if (optional) {
                os << " [" << flag;
            } else {
                os << " " << flag;
            }
            // format the option value
            if (option.arguments < 0) {
                os << " ...";
            } else if (option.arguments > 0) {
                for (int i = option.arguments; i != 0; --i) {
                    os << " _";
                }
            }
            // mark optional with braces
            if (optional) {
                os << "]";
            }
        }
    }

    void format_options_long(std::ostream& os, const std::list<flags::OptionValue>& main_options)
    {
        // TODO: print parameters
        for (auto& it : main_options) {
            const auto& [flag, option] = it;
            const size_t flag_size = flag.length();

            // print flag name
            os << std::string(2, ' ') << flag;
            // decide if the help text goes into the same line or not
            if (flag_size > 22) {
                os << std::endl << std::string(15, ' ');
            } else {
                os << std::string(23 - flag_size, ' ');
            }
            os << option.help;
            // print default value if exists
            if (option.default_value) {
                os << " (default: " << option.default_value.value() << ')';
            }
            os << std::endl;
        }
    }
}

namespace flags {

    Arguments::Arguments(std::string_view&& program, Arguments::Parameters&& parameters)
            : program_(program)
            , parameters_(parameters)
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
        std::string_view program(argv[0]);
        Arguments::Parameters parameters;

        const char** const args_end = argv + argc;
        for (const char** args_it = ++argv; args_it != args_end;) {
            // find which option is it.
            if (auto option = options_.find(*args_it); option != options_.end()) {
                // take the required number of arguments if founded.
                if (const auto params = take(option->second, args_it + 1, args_end); params) {
                    auto [begin, end] = params.value();
                    auto args = std::vector<std::string_view>(begin, end);
                    parameters.emplace(option->first, args);

                    args_it = end;
                } else {
                    return rust::Err(std::runtime_error((std::string("Not enough parameters for flag: ") + *args_it)));
                }
            } else {
                return rust::Err(std::runtime_error((std::string("Unrecognized parameter: ") + *args_it)));
            }
        }
        for (auto& option : options_) {
            // add default values to the parameters as it would given by the user.
            if (option.second.default_value.has_value() && parameters.find(option.first) == parameters.end()) {
                std::vector<std::string_view> args = { option.second.default_value.value() };
                parameters.emplace(option.first, args);
            }
        }
        return rust::Ok(Arguments(std::move(program), std::move(parameters)));
    }

    void Parser::print_help(std::ostream& os) const
    {
        const std::list<std::list<flags::OptionValue>> options = group_by(options_);

        os << "Usage: " << name_;
        const std::list<flags::OptionValue>& main_options = options.front();
        format_options(os, main_options);
        os << std::endl;

        for (const auto& group : options) {
            os << std::endl;
            if (auto group_name = group.front().second.group_name; group_name) {
                os << group_name.value() << std::endl;
            }
            format_options_long(os, group);
        }
    }

    void Parser::print_usage(std::ostream& os) const
    {
        const std::list<std::list<flags::OptionValue>> options = group_by(options_);
        const std::list<flags::OptionValue>& main_options = options.front();

        os << "Usage: " << name_;
        format_options(os, main_options);
        os << std::endl;
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
