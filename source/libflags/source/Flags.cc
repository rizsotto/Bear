/*  Copyright (C) 2012-2024 by László Nagy
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

#include "libflags/Flags.h"

#include <algorithm>
#include <cstring>
#include <iostream>
#include <optional>
#include <set>
#include <tuple>
#include <utility>

#include <fmt/format.h>

namespace {

    constexpr char QUERY_GROUP[] = "query options";

    std::optional<std::tuple<const char**, const char**>>
    take(const flags::Option& option, const char** const begin, const char** const end) noexcept
    {
        return (option.arguments < 0)
            ? std::optional(std::make_tuple(begin, end))
            : (begin + option.arguments > end)
                ? std::nullopt
                : std::optional(std::make_tuple(begin, begin + option.arguments));
    }

    std::list<flags::OptionValue> order_by_relevance(const flags::OptionMap& options, const std::optional<std::string_view>& group)
    {
        std::list<flags::OptionValue> result;
        std::copy_if(std::begin(options), std::end(options),
            std::back_inserter(result),
            [&group](auto& option) { return option.second.group_name == group && option.second.arguments >= 0; });
        std::copy_if(std::begin(options), std::end(options),
            std::back_inserter(result),
            [&group](auto& option) { return option.second.group_name == group && option.second.arguments < 0; });
        return result;
    }

    std::list<std::list<flags::OptionValue>> group_by(const flags::OptionMap& options)
    {
        // find out what are the option groups.
        std::set<std::optional<std::string_view>> groups;
        for (const auto& [_, option] : options) {
            groups.emplace(option.group_name);
        }
        std::list<std::list<flags::OptionValue>> result;
        // insert to the result list.
        for (auto& group : groups) {
            result.emplace_back(order_by_relevance(options, group));
        }
        return result;
    }

    std::string format_parameters(const flags::Option& option)
    {
        switch (option.arguments) {
        case 0:
            return "";
        case 1:
            return " <arg>";
        case 2:
            return " <arg0> <arg1>>";
        case 3:
            return " <arg0> <arg1> <arg2>";
        default:
            return " ...";
        }
    }

    void format_options(std::ostream& os, const std::list<flags::OptionValue>& options)
    {
        for (auto& it : options) {
            const auto& [flag, option] = it;

            const std::string parameters = format_parameters(option);
            const std::string short_help = option.required
                ? fmt::format(" {0}{1}", flag, parameters)
                : fmt::format(" [{0}{1}]", flag, parameters);
            os << short_help;
        }
    }

    void format_options_long(std::ostream& os, const std::list<flags::OptionValue>& main_options)
    {
        for (auto& it : main_options) {
            const auto& [flag, option] = it;

            const std::string flag_name = fmt::format("  {0}{1}", flag, format_parameters(option));
            const size_t flag_size = flag_name.length();

            // print flag name
            os << flag_name;
            // decide if the help text goes into the same line or not
            if (flag_size > 25) {
                os << std::endl
                   << std::string(15, ' ');
            } else {
                os << std::string(26 - flag_size, ' ');
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

    Arguments::Arguments(std::string_view program, Arguments::Parameters&& parameters)
            : program_(program)
            , parameters_(parameters)
    { }

    rust::Result<bool> Arguments::as_bool(const std::string_view& key) const
    {
        return rust::Ok(parameters_.find(key) != parameters_.end());
    }

    rust::Result<std::string_view> Arguments::as_string(const std::string_view& key) const
    {
        if (auto values = parameters_.find(key); values != parameters_.end()) {
            return (values->second.size() == 1)
                ? rust::Result<std::string_view>(
                        rust::Ok(values->second.front()))
                : rust::Result<std::string_view>(
                        rust::Err(std::runtime_error(
                                fmt::format("Parameter \"{0}\" is not a single string.", key))));
        }
        return rust::Err(std::runtime_error(
                fmt::format("Parameter \"{0}\" is not available.", key)));
    }

    rust::Result<std::vector<std::string_view>> Arguments::as_string_list(const std::string_view& key) const
    {
        if (auto values = parameters_.find(key); values != parameters_.end()) {
            return rust::Ok(values->second);
        }
        return rust::Err(std::runtime_error(
                fmt::format("Parameter \"{0}\" is not available.", key)));
    }

    std::ostream& operator<<(std::ostream& os, const Arguments& args)
    {
        os << '{';
        os << "program: " << args.program_ << ", arguments: [";
        for (auto arg_it = args.parameters_.begin(); arg_it != args.parameters_.end(); ++arg_it) {
            if (arg_it != args.parameters_.begin()) {
                os << ", ";
            }
            os << '{' << arg_it->first << ": [";
            for (auto param_it = arg_it->second.begin(); param_it != arg_it->second.end(); ++param_it) {
                if (param_it != arg_it->second.begin()) {
                    os << ", ";
                }
                os << *param_it;
            }
            os << "]}";
        }
        os << "]}";
        return os;
    }

    Parser::Parser(std::string_view name, std::string_view version, std::initializer_list<OptionValue> options)
            : name_(name)
            , version_(version)
            , options_(options)
            , commands_()
    {
        options_.insert({ VERBOSE, { 0, false, "run in verbose mode", std::nullopt, std::nullopt } });
        options_.insert({ HELP, { 0, false, "print help and exit", std::nullopt, { QUERY_GROUP } } });
        options_.insert({ VERSION, { 0, false, "print version and exit", std::nullopt, { QUERY_GROUP } } });
    }

    Parser::Parser(std::string_view name, std::initializer_list<OptionValue> options)
            : name_(name)
            , version_()
            , options_(options)
            , commands_()
    {
        options_.insert({ VERBOSE, { 0, false, "run in verbose mode", std::nullopt, std::nullopt } });
        options_.insert({ HELP, { 0, false, "print help and exit", std::nullopt, { QUERY_GROUP } } });
    }

    Parser::Parser(std::string_view name, std::string_view version, std::initializer_list<Parser> commands, std::initializer_list<OptionValue> default_options)
            : name_(name)
            , version_(version)
            , options_(default_options)
            , commands_(commands)
    {
        if (default_options.size() != 0) {
            options_.insert({ VERBOSE, { 0, false, "run in verbose mode", std::nullopt, std::nullopt } });
        }
        options_.insert({ HELP, { 0, false, "print help and exit", std::nullopt, { QUERY_GROUP } } });
        options_.insert({ VERSION, { 0, false, "print version and exit", std::nullopt, { QUERY_GROUP } } });
    }

    rust::Result<Arguments> Parser::parse(const int argc, const char** argv) const
    {
        if (argc < 1 || argv == nullptr) {
            return rust::Err(std::runtime_error("Empty argument list."));
        }

        if (!commands_.empty() && argc >= 2) {
            const std::string_view command = argv[1];
            const auto sub_command = std::find_if(commands_.begin(), commands_.end(),
                                           [&command](auto candidate) { return candidate.name_ == command; });
            if (sub_command != commands_.end()) {
                return sub_command->parse(argc - 1, argv + 1)
                        .map<Arguments>([&sub_command](auto arguments) {
                            arguments.parameters_[COMMAND] = {sub_command->name_};
                            return arguments;
                        });
            }
        }

        std::string_view program(argv[0]);
        Arguments::Parameters parameters;

        const char** const args_end = argv + argc;
        for (const char** args_it = ++argv; args_it != args_end;) {
            // find which option is it.
            if (auto option = options_.find(*args_it); option != options_.end()) {
                // take the required number of arguments if founded.
                if (const auto params = take(option->second, args_it + 1, args_end); params) {
                    const auto& [begin, end] = params.value();
                    auto args = std::vector<std::string_view>(begin, end);

                    if (auto it = parameters.find(option->first); parameters.end() != it) {
                        std::copy(args.begin(), args.end(), std::back_inserter(it->second));
                    } else {
                        parameters.emplace(option->first, args);
                    }

                    args_it = end;
                } else {
                    return rust::Err(std::runtime_error(
                        fmt::format("Not enough parameters for: \"{0}\"", *args_it)));
                }
            } else {
                return rust::Err(std::runtime_error(
                    fmt::format("Unrecognized parameter: \"{0}\"", *args_it)));
            }
        }
        // add default values to the parameters as it would given by the user.
        for (const auto& [flag, option] : options_) {
            if (option.default_value.has_value() && parameters.find(flag) == parameters.end()) {
                std::vector<std::string_view> args = { option.default_value.value() };
                parameters.emplace(flag, args);
            }
        }
        // if this is not a help or version query, then validate the parameters strict.
        if (parameters.find(HELP) == parameters.end() && parameters.find(VERSION) == parameters.end()) {
            for (const auto& [flag, option] : options_) {
                // check if the parameter is required, but not present.
                if (option.required && parameters.find(flag) == parameters.end()) {
                    return rust::Err(std::runtime_error(
                        fmt::format("Parameter is required, but not given: \"{0}\"", flag)));
                }
            }
        }
        return rust::Ok(Arguments(program, std::move(parameters)));
    }

    rust::Result<Arguments> Parser::parse_or_exit(int argc, const char** argv) const
    {
        auto sub_command = [this](const std::string_view &name) -> const Parser * {
            const auto it = std::find_if(commands_.begin(), commands_.end(),
                                         [&name](auto command) { return command.name_ == name; });
            return (it != commands_.end()) ? &(*it) : nullptr;
        };

        return parse(argc, argv)
            // print error if anything bad happens.
            .on_error([this](auto error) {
                std::cerr << error.what() << std::endl;
                print_usage(nullptr, std::cerr);
                exit(EXIT_FAILURE);
            })
            // if parsing success, check for the `--help` and `--version` flags
            .on_success([this, &sub_command](auto args) {
                // print version message and exit zero
                if (args.as_bool(VERSION).unwrap_or(false)) {
                    print_version(std::cout);
                    exit(EXIT_SUCCESS);
                }
                // print help message and exit zero
                if (args.as_bool(HELP).unwrap_or(false)) {
                    if (const auto command = args.as_string(COMMAND); command.is_ok()) {
                        print_help(sub_command(command.unwrap()), std::cout);
                    } else {
                        print_help(nullptr, std::cout);
                    }
                    exit(EXIT_SUCCESS);
                }
            });
    }

    void Parser::print_help(const Parser *const sub_command, std::ostream& os) const {
        print_usage(sub_command, os);

        const Parser &parser = (sub_command != nullptr) ? *sub_command : *this;
        // print commands if exists.
        if (!parser.commands_.empty()) {
            os << std::endl << "commands" << std::endl;
            for (const auto& command : parser.commands_) {
                os << "  " << command.name_ << std::endl;
            }
        }
        // print options
        const std::list<std::list<flags::OptionValue>> options = group_by(parser.options_);
        for (const auto& group : options) {
            os << std::endl;
            if (auto group_name = group.front().second.group_name; group_name) {
                os << group_name.value() << std::endl;
            }
            format_options_long(os, group);
        }
    }

    void Parser::print_usage(const Parser *const sub_command, std::ostream& os) const {
        os << "Usage: " << name_;
        // check for the given command
        if (sub_command != nullptr) {
            os << " " << sub_command->name_;
            const auto options = order_by_relevance(sub_command->options_, std::nullopt);
            format_options(os, options);
        } else {
            if (!commands_.empty()) {
                os << " <command>";
            }
            const auto options = order_by_relevance(options_, std::nullopt);
            format_options(os, options);
        }
        os << std::endl;
    }

    void Parser::print_version(std::ostream& os) const
    {
        os << name_ << " " << version_ << std::endl;
    }
}
