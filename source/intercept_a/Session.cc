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

#include "intercept_a/Session.h"
#include "intercept_a/Interface.h"
#include "intercept_a/Result.h"

#include <cstring>
#include <string_view>
#include <list>
#include <vector>
#include <initializer_list>
#include <algorithm>

namespace {

    struct Description {
        const char *flag;
        int arguments;
        const char *help;

        bool match(const char **input) const noexcept {
            return (std::strcmp(*input, flag) == 0);
        }

        const char **take(const char **input) const noexcept {
            if (arguments < 0) {
                while (*input != nullptr)
                    ++input;
            } else {
                for (int idx = arguments; idx != 0; --idx) {
                    if (*input == nullptr)
                        return nullptr;
                    else
                        ++input;
                }
            }
            return input;
        }
    };

    using Parameter = std::tuple<const char **, const char **>;
    using Parameters = std::map<std::string_view, Parameter>;

    constexpr char program_key[] = "program";

    class Parser {
    public:
        Parser(std::initializer_list<Description> options)
                : options_(options)
        { }

        ::pear::Result<Parameters> parse(const char **args) const noexcept {
            Parameters result;
            if (args == nullptr || *args == nullptr) {
                return ::pear::Err(std::runtime_error("Empty parameter list."));
            }
            result.emplace(Parameters::key_type(program_key), std::make_tuple(args, args + 1));
            for (const char **args_it = ++args; *args_it != nullptr; ) {
                bool match = false;
                for (auto option : options_) {
                    match = option.match(args_it);
                    if (!match)
                        continue;

                    const char *flag = *args_it++;
                    const char **begin = args_it;
                    const char **end = option.take(args_it);
                    if (end == nullptr) {
                        return ::pear::Err(std::runtime_error((std::string("Not enough parameters for flag: ") + flag)));
                    }
                    result.emplace(Parameters::key_type(flag), std::make_tuple(begin, end));
                    args_it = end;
                    break;
                }
                if ((!match) && (*args_it != nullptr)) {
                    return ::pear::Err(std::runtime_error((std::string("Unrecognized parameter: ") + *args_it)));
                }
            }
            return ::pear::Ok(std::move(result));
        }

        std::string help(const char *const name) const noexcept {
            std::string result;
            result += std::string("Usage: ") + name + std::string(" [OPTION]\n\n");
            // TODO: do better formating
            std::for_each(options_.begin(), options_.end(), [&result](auto it) {
                result += "  " + std::string(it.flag) + "  " + std::string(it.help) + "\n";
            });
            return result;
        }

    private:
        const std::vector<Description> options_;
    };

    ::pear::Result<::pear::Context> make_context(const Parameters &parameters) noexcept {
        if (auto destination_it = parameters.find(::pear::flag::destination); destination_it != parameters.end()) {
            auto const [ destination, _ ] = destination_it->second;
            const bool verbose = (parameters.find(::pear::flag::verbose) != parameters.end());
            auto const [ reporter, __ ] = parameters.find(program_key)->second;
            return ::pear::Ok<::pear::Context>({ *reporter, *destination, verbose });
        } else {
            return ::pear::Err(std::runtime_error("Missing destination.\n"));
        }
    }

    ::pear::Result<::pear::Execution> make_execution(const Parameters &parameters) noexcept {
        auto get_optional = [&parameters](const char *const name) -> const char * {
            if (auto it = parameters.find(name); it != parameters.end()) {
                auto [ result, _ ] = it->second;
                return *result;
            } else {
                return nullptr;
            }
        };

        auto const nowhere = parameters.end();
        if (auto command_it = parameters.find(::pear::flag::command); command_it != nowhere) {
            auto [ command, _ ] = command_it->second;
            auto file = get_optional(::pear::flag::file);
            auto search_path = get_optional(::pear::flag::search_path);
            return ::pear::Ok<::pear::Execution>({ command, file, search_path });
        } else {
            return ::pear::Err(std::runtime_error("Missing command.\n"));
        }
    }

}

namespace pear {

    void
    Session::configure(::pear::Environment::Builder &builder) const noexcept {
        builder.add_reporter(context_.reporter);
        builder.add_destination(context_.destination);
        builder.add_verbose(context_.verbose);
    }

    void
    LibrarySession::configure(::pear::Environment::Builder &builder) const noexcept {
        Session::configure(builder);
        builder.add_library(library);
    }

    void
    WrapperSession::configure(::pear::Environment::Builder &builder) const noexcept {
        Session::configure(builder);
        builder.add_cc_compiler(cc, cc_wrapper);
        builder.add_cxx_compiler(cxx, cxx_wrapper);
    }

    pear::Result<pear::SessionPtr> parse(int argc, char *argv[]) noexcept {
        const Parser parser({
            { ::pear::flag::help,        0, "this message" },
            { ::pear::flag::verbose,     0, "make the interception run verbose" },
            { ::pear::flag::destination, 1, "path to report directory" },
            { ::pear::flag::library,     1, "path to the intercept library" },
            { ::pear::flag::wrapper_cc,  2, "path to the C compiler and the wrapper" },
            { ::pear::flag::wrapper_cxx, 2, "path to the C++ compiler and the wrapper", },
            { ::pear::flag::file,        1, "the file name for the command" },
            { ::pear::flag::search_path, 1, "the search path for the command" },
            { ::pear::flag::command,    -1, "the executed command" }
        });
        return parser.parse(const_cast<const char **>(argv))
                .bind<::pear::SessionPtr>([&parser, &argv](auto params) -> Result<::pear::SessionPtr> {
                    if (params.find(::pear::flag::help) != params.end())
                        return Err(std::runtime_error(parser.help(argv[0])));
                    else
                        return merge(make_context(params), make_execution(params))
                            .template map<::pear::SessionPtr>([&params](auto in) -> ::pear::SessionPtr {
                                const auto& [context, execution] = in;
                                if (auto library_it = params.find(::pear::flag::library); library_it != params.end()) {
                                    const auto& [library, _] = library_it->second;
                                    auto result = std::make_unique<LibrarySession>(context, execution);
                                    result->library = *library;
                                    return SessionPtr(result.release());
                                } else if ((params.find(::pear::flag::wrapper_cc) != params.end()) &&
                                           (params.find(::pear::flag::wrapper_cxx) != params.end())) {
                                    const auto& [wrapper_cc_begin, wrapper_cc_end] =
                                         params.find(::pear::flag::wrapper_cc)->second;
                                    const auto& [wrapper_cxx_begin, wrapper_cxx_end] =
                                         params.find(::pear::flag::wrapper_cxx)->second;
                                    auto result = std::make_unique<WrapperSession>(context, execution);
                                    result->cc = *wrapper_cc_begin;
                                    result->cc_wrapper = *(wrapper_cc_begin + 1);
                                    result->cxx = *wrapper_cxx_begin;
                                    result->cxx_wrapper = *(wrapper_cxx_begin + 1);
                                    return SessionPtr(result.release());
                                } else {
                                    return std::make_shared<Session>(context, execution);
                                }
                            });
                });
    }

}