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

#include "SessionWrapper.h"
#include "Application.h"
#ifdef SUPPORT_PRELOAD
#include "SessionLibrary.h"
#endif

#include "libsys/Path.h"

namespace ic {

#ifdef SUPPORT_PRELOAD
    rust::Result<Session::SharedPtr> Session::from(const flags::Arguments& args, const sys::Context& ctx)
    {
        if (args.as_bool(ic::Application::FORCE_WRAPPER).unwrap_or(false))
            return WrapperSession::from(args, ctx);
        if (args.as_bool(ic::Application::FORCE_PRELOAD).unwrap_or(false))
            return LibraryPreloadSession::from(args, ctx);

        return ctx.get_uname()
            .and_then<Session::SharedPtr>([&args, &ctx](auto uname) {
                const bool preload = (uname["sysname"] == "Linux");

                return (preload)
                    ? LibraryPreloadSession::from(args, ctx)
                    : WrapperSession::from(args, ctx);
            });
    }
#else
    rust::Result<Session::SharedPtr> Session::from(const flags::Arguments& args, const sys::Context& ctx)
    {
        return WrapperSession::from(args, ctx);
    }
#endif

    void Session::set_server_address(const std::string& value)
    {
        server_address_ = value;
    }

    std::string Session::keep_front_in_path(const std::string& path, const std::string& paths)
    {
        std::list<fs::path> result = { path };

        auto existing = sys::path::split(paths);
        std::copy_if(existing.begin(), existing.end(), std::back_inserter(result), [&path](auto current) {
            return current != path;
        });

        return sys::path::join(result);
    }

    std::string Session::remove_from_path(const std::string& path, const std::string& paths)
    {
        std::list<fs::path> result = { };

        auto existing = sys::path::split(paths);
        std::copy_if(existing.begin(), existing.end(), std::back_inserter(result), [&path](auto current) {
            return current != path;
        });

        return sys::path::join(result);
    }
}
