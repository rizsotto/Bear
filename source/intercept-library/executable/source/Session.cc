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

#include "Session.h"

#include "flags.h"

#include <algorithm>
#include <cstring>
#include <initializer_list>
#include <list>
#include <string_view>
#include <vector>

#include "Interface.h"
#include "Result.h"

using rust::Result;
using rust::Ok;
using rust::Err;

namespace {

    using Parameter = std::tuple<const char**, const char**>;
    using Parameters = std::map<std::string_view, Parameter>;

    constexpr char PROGRAM_KEY[] = "program";

    struct Option {
        const char* flag;
        int arguments;
        const char* help;

        bool match(const char* input) const noexcept
        {
            return (std::strcmp(input, flag) == 0);
        }

        std::optional<Parameter>
        take(const char** const begin, const char** const end) const noexcept
        {
            return (arguments < 0)
                ? std::optional(std::make_tuple(begin, end))
                : (begin + arguments > end)
                    ? std::optional<Parameter>()
                    : std::optional(std::make_tuple(begin, begin + arguments));
        }

        std::string format_option_line() const noexcept
        {
            const size_t flag_size = std::strlen(flag);

            std::string result;
            result += spaces(2);
            result += flag;
            result += (flag_size > 22)
                ? "\n" + spaces(15)
                : spaces(23 - flag_size);
            result += std::string(help) + "\n";
            return result;
        }

        static std::string spaces(size_t num) noexcept
        {
            std::string result;
            for (; num > 0; --num)
                result += ' ';
            return result;
        }
    };

    class Parser {
    public:
        Parser(std::initializer_list<Option> options)
                : options_(options)
        {
        }

        Result<Parameters> parse(const int argc, const char** argv) const noexcept
        {
            Parameters result;
            if (argc < 2 || argv == nullptr) {
                return Err(std::runtime_error("Empty parameter list."));
            }
            result.emplace(Parameters::key_type(PROGRAM_KEY), std::make_tuple(argv, argv + 1));
            const char** const args_end = argv + argc;
            for (const char** args_it = ++argv; args_it != args_end;) {
                // find which option is it.
                if (auto option = std::find_if(options_.begin(), options_.end(), [&args_it](const auto& option) {
                        return option.match(*args_it);
                    });
                    option != options_.end()) {
                    if (const auto params = option->take(args_it + 1, args_end); params) {
                        result.emplace(Parameters::key_type(*args_it), params.value());
                        args_it = std::get<1>(params.value());
                    } else {
                        return Err(std::runtime_error((std::string("Not enough parameters for flag: ") + *args_it)));
                    }
                } else {
                    return Err(std::runtime_error((std::string("Unrecognized parameter: ") + *args_it)));
                }
            }
            return Ok(std::move(result));
        }

        std::string help(const char* const name) const noexcept
        {
            std::string result;
            result += std::string("Usage: ") + name + std::string(" [OPTION]\n\n");
            std::for_each(options_.begin(), options_.end(), [&result](auto it) {
                result += it.format_option_line();
            });
            return result;
        }

    private:
        const std::vector<Option> options_;
    };

    Result<::er::Context> make_context(const Parameters& parameters) noexcept
    {
        if (auto destination_it = parameters.find(::er::flags::DESTINATION); destination_it != parameters.end()) {
            auto const [destination, _] = destination_it->second;
            const bool verbose = (parameters.find(::er::flags::VERBOSE) != parameters.end());
            auto const [reporter, __] = parameters.find(PROGRAM_KEY)->second;
            return Ok<::er::Context>({ *reporter, *destination, verbose });
        } else {
            return Err(std::runtime_error("Missing destination."));
        }
    }

    Result<::er::Execution> make_execution(const Parameters& parameters) noexcept
    {
        auto get_optional = [&parameters](const char* const name) -> const char* {
            if (auto it = parameters.find(name); it != parameters.end()) {
                auto [result, _] = it->second;
                return *result;
            } else {
                return nullptr;
            }
        };

        auto const nowhere = parameters.end();
        if (auto command_it = parameters.find(::er::flags::COMMAND); command_it != nowhere) {
            auto [command, _] = command_it->second;
            auto path = get_optional(::er::flags::EXECUTE);
            if (path != nullptr) {
                return Ok<::er::Execution>({ path, command });
            } else {
                return Err(std::runtime_error("The 'path' needs to be specified."));
            }
        } else {
            return Err(std::runtime_error("Missing command."));
        }
    }

}

namespace er {

    void Session::configure(::er::Environment::Builder& builder) const noexcept
    {
        builder.add_reporter(context_.reporter);
        builder.add_destination(context_.destination);
        builder.add_verbose(context_.verbose);
    }

    void LibrarySession::configure(::er::Environment::Builder& builder) const noexcept
    {
        Session::configure(builder);
        builder.add_library(library);
    }

    Result<er::SessionPtr> parse(int argc, char* argv[]) noexcept
    {
        const Parser parser({ { ::er::flags::HELP, 0, "this message" },
            { ::er::flags::VERBOSE, 0, "make the interception run verbose" },
            { ::er::flags::DESTINATION, 1, "path to report directory" },
            { ::er::flags::LIBRARY, 1, "path to the intercept library" },
            { ::er::flags::EXECUTE, 1, "the path parameter for the command" },
            { ::er::flags::COMMAND, -1, "the executed command" } });
        return parser.parse(argc, const_cast<const char**>(argv))
            .and_then<::er::SessionPtr>([&parser, &argv](auto params) -> Result<::er::SessionPtr> {
                if (params.find(::er::flags::HELP) != params.end())
                    return Err(std::runtime_error(parser.help(argv[0])));
                else
                    return merge(make_context(params), make_execution(params))
                        .template map<::er::SessionPtr>([&params](auto in) -> ::er::SessionPtr {
                            const auto& [context, execution] = in;
                            if (auto library_it = params.find(::er::flags::LIBRARY); library_it != params.end()) {
                                const auto& [library, _] = library_it->second;
                                auto result = std::make_unique<LibrarySession>(context, execution);
                                result->library = *library;
                                return SessionPtr(result.release());
                            } else {
                                return std::make_shared<Session>(context, execution);
                            }
                        });
            });
    }

}