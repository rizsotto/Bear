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

#include "Application.h"

#ifdef SUPPORT_PRELOAD
#include "SessionLibrary.h"
#endif

namespace ic {

#ifdef SUPPORT_PRELOAD
    rust::Result<Session::SharedPtr> Session::from(const flags::Arguments& args, const sys::Context& ctx)
    {
        auto library = args.as_string(ic::Application::LIBRARY);
        auto executor = args.as_string(ic::Application::EXECUTOR);
        auto verbose = args.as_bool(ic::Application::VERBOSE);

        return merge(library, executor, verbose)
            .map<Session::SharedPtr>([&ctx](auto tuple) {
                const auto& [library, executor, verbose] = tuple;
                auto environment = ctx.get_environment();
                auto result = new LibraryPreloadSession(library, executor, verbose, std::move(environment));
                return std::shared_ptr<Session>(result);
            });
    }
#else
    rust::Result<Session::SharedPtr> Session::from(const flags::Arguments& args, const sys::Context& ctx)
    {
        return rust::Err(std::runtime_error("Not implemented."));
    }
#endif
}
