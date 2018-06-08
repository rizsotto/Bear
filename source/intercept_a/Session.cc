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

    using Parameter = std::vector<const char *>;
    using Parameters = std::map<std::string_view, Parameter>;

    class Parser {
    public:
        Parser(const char *name, std::initializer_list<Description> options)
                : name_(name)
                , options_(options)
        { }

        ::pear::Result<Parameters> parse(const char **args) const noexcept {
            auto exit = [](auto message) {
                return ::pear::Result<Parameters >::failure(std::runtime_error(message));
            };

            Parameters result;
            for (const char **args_it = args; *args_it != nullptr; ) {
                bool match = false;
                for (auto option : options_) {
                    match = option.match(args_it);
                    if (!match)
                        continue;

                    const char *flag = *args_it++;
                    const char **begin = args_it;
                    const char **end = option.take(args_it);
                    if (end == nullptr) {
                        return exit(std::string("Not enough parameters for flag: ") + flag);
                    }
                    result.emplace(Parameters::key_type(flag), Parameter(begin, end));
                    args_it = end;
                    break;
                }
                if ((!match) && (*args_it != nullptr)) {
                    return exit(std::string("Unrecognized parameter: ") + *args_it);
                }
            }
            return ::pear::Result<Parameters>::success(result);
        }

        std::string help() const noexcept {
            std::string result;
            result += std::string("Usage: ") + name_ + std::string(" [OPTION]\n\n");
            // TODO: do better formating
            std::for_each(options_.begin(), options_.end(), [&result](auto it) {
                result += "  " + std::string(it.flag) + "  " + std::string(it.help) + "\n";
            });
            return result;
        }

    private:
        const char *name_;
        const std::vector<Description> options_;
    };

    ::pear::Result<::pear::Context> make_context(const Parameters &parameters, const char *reporter) noexcept {
        // TODO
        return ::pear::Result<::pear::Context>::failure(std::runtime_error("placeholder"));
    }

    ::pear::Result<::pear::Execution> make_execution(const Parameters &parameters) noexcept {
        // TODO
        return ::pear::Result<::pear::Execution>::failure(std::runtime_error("placeholder"));
    }

}

namespace pear {

    ::pear::Environment::Builder &
    Session::set(::pear::Environment::Builder &builder) const noexcept {
        return builder;
    }

    ::pear::Environment::Builder &
    LibrarySession::set(::pear::Environment::Builder &builder) const noexcept {
        builder.add_reporter(context.reporter);
        builder.add_destination(context.destination);
        builder.add_verbose(context.verbose);
        builder.add_library(library);
        return builder;
    }

    ::pear::Environment::Builder &
    WrapperSession::set(::pear::Environment::Builder &builder) const noexcept {
        builder.add_reporter(context.reporter);
        builder.add_destination(context.destination);
        builder.add_verbose(context.verbose);
        builder.add_cc_compiler(cc, cc_wrapper);
        builder.add_cxx_compiler(cxx, cxx_wrapper);
        return builder;
    }

    pear::Result<pear::SessionPtr> parse(int argc, char *argv[]) noexcept {
        auto reporter = argv[0];
        const Parser parser(argv[0], {
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
        return parser.parse(const_cast<const char **>(++argv))
                .bind<::pear::SessionPtr>([&parser](auto params) {
                    if (params.find(::pear::flag::help) != params.end()) {
                        auto lines = parser.help();
                        return pear::Result<pear::SessionPtr>::failure(std::runtime_error(lines));
                    }
                    // TODO
                    return pear::Result<pear::SessionPtr>::failure(std::runtime_error("placeholder"));
                });
    }

}