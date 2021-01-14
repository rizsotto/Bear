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

#include "collect/Session.h"

#include "intercept/Flags.h"
#include "collect/SessionWrapper.h"
#ifdef SUPPORT_PRELOAD
#include "collect/SessionLibrary.h"
#endif

#include "libsys/Path.h"
#include "libsys/Signal.h"

#include <spdlog/spdlog.h>

namespace ic {

    rust::Result<Session::Ptr> Session::from(const flags::Arguments& args, const char **envp)
#ifdef SUPPORT_PRELOAD
    {
        if (args.as_bool(ic::FORCE_WRAPPER).unwrap_or(false))
            return WrapperSession::from(args, envp);
        if (args.as_bool(ic::FORCE_PRELOAD).unwrap_or(false))
            return LibraryPreloadSession::from(args, envp);

        return LibraryPreloadSession::from(args, envp);
    }
#else
    {
        return WrapperSession::from(args, envp);
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

    rust::Result<int> Session::execute(const std::vector<std::string_view> &command) const {
        return supervise(command)
                .and_then<sys::Process>([](auto builder) {
                    return builder.spawn();
                })
                .and_then<sys::ExitStatus>([](auto child) {
                    sys::SignalForwarder guard(child);
                    return child.wait();
                })
                .map<int>([](auto status) {
                    return status.code().value_or(EXIT_FAILURE);
                })
                .map_err<std::runtime_error>([](auto error) {
                    spdlog::warn("Command execution failed: {}", error.what());
                    return error;
                })
                .on_success([](auto status) {
                    spdlog::debug("Running command. [Exited with {0}]", status);
                });
    }
}
