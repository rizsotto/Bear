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

    Result<::er::Context> make_context(const Arguments& args) noexcept
    {
        return args.as_string(::er::flags::DESTINATION)
            .map<::er::Context>([&args](const auto destination) {
                const auto reporter = args.program();
                const bool verbose = args.as_bool(::er::flags::VERBOSE).unwrap_or(false);
                return er::Context { reporter, destination, verbose };
            });
    }

    Result<::er::Execution> make_execution(const Arguments& args) noexcept
    {
        auto path = args.as_string(::er::flags::EXECUTE);
        auto command = args.as_string_list(::er::flags::COMMAND);

        return merge(path, command)
            .map<::er::Execution>([](auto tuple) {
                const auto& [path, command] = tuple;
                return ::er::Execution { path, command };
            });
    }
}

namespace er {

    void Session::configure(::er::Environment::Builder& builder) const noexcept
    {
        builder.add_reporter(context_.reporter.data());
        builder.add_destination(context_.destination.data());
        builder.add_verbose(context_.verbose);
    }

    void LibrarySession::configure(::er::Environment::Builder& builder) const noexcept
    {
        Session::configure(builder);
        builder.add_library(library);
    }

    Result<er::SessionPtr> parse(int argc, char* argv[]) noexcept
    {
        const Parser parser("er",
            { { ::er::flags::HELP, { 0, false, "this message", std::nullopt } },
                { ::er::flags::VERBOSE, { 0, false, "make the interception run verbose", std::nullopt } },
                { ::er::flags::DESTINATION, { 1, false, "path to report directory", std::nullopt } },
                { ::er::flags::LIBRARY, { 1, false, "path to the intercept library", std::nullopt } },
                { ::er::flags::EXECUTE, { 1, false, "the path parameter for the command", std::nullopt } },
                { ::er::flags::COMMAND, { -1, false, "the executed command", std::nullopt } } });
        return parser.parse(argc, const_cast<const char**>(argv))
            .and_then<::er::SessionPtr>([&parser, &argv](auto params) -> Result<::er::SessionPtr> {
                if (params.as_bool(::er::flags::HELP).unwrap_or(false))
                    return Err(std::runtime_error(parser.help()));
                else
                    return merge(make_context(params), make_execution(params), params.as_string(::er::flags::LIBRARY))
                        .template map<::er::SessionPtr>([&params](auto in) -> ::er::SessionPtr {
                            const auto& [context, execution, library] = in;
                            auto result = std::make_unique<LibrarySession>(context, execution);
                            result->library = library.data();
                            return SessionPtr(result.release());
                        });
            });
    }

}