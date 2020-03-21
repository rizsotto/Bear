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

#include "Flags.h"
#include "Interface.h"
#include "Result.h"

using namespace rust;
using namespace flags;

namespace {

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
        const Parser parser({ { ::er::flags::HELP, { 0, "this message" } },
            { ::er::flags::VERBOSE, { 0, "make the interception run verbose" } },
            { ::er::flags::DESTINATION, { 1, "path to report directory" } },
            { ::er::flags::LIBRARY, { 1, "path to the intercept library" } },
            { ::er::flags::EXECUTE, { 1, "the path parameter for the command" } },
            { ::er::flags::COMMAND, { -1, "the executed command" } } });
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