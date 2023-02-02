/*  Copyright (C) 2012-2023 by László Nagy
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

#include "config.h"
#include "collect/Session.h"

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
        if (args.as_bool(cmd::intercept::FLAG_FORCE_WRAPPER).unwrap_or(false))
            return WrapperSession::from(args, envp);
        if (args.as_bool(cmd::intercept::FLAG_FORCE_PRELOAD).unwrap_or(false))
            return LibraryPreloadSession::from(args);

        return LibraryPreloadSession::from(args);
    }
#else
    {
        return WrapperSession::from(args, envp);
    }
#endif

    std::string Session::keep_front_in_path(const std::string& path, const std::string& paths)
    {
        if (paths == path) {
            return paths;
        } else {
            std::list<fs::path> result = {path};

            auto existing = sys::path::split(paths);
            std::copy_if(existing.begin(), existing.end(),
                         std::back_inserter(result),
                         [&path](auto current) { return current != path; }
            );

            return sys::path::join(result);
        }
    }

    std::string Session::remove_from_path(const std::string& path, const std::string& paths)
    {
        std::list<fs::path> result = { };

        auto existing = sys::path::split(paths);
        std::copy_if(existing.begin(), existing.end(),
                     std::back_inserter(result),
                     [&path](auto current) { return current != path; }
        );

        return sys::path::join(result);
    }

    rust::Result<int> Session::run(const ic::Execution &execution, const SessionLocator &session_locator) {
        session_locator_ = std::make_unique<SessionLocator>(session_locator);
        return supervise(execution)
                .spawn()
                .and_then<sys::ExitStatus>([](auto child) {
                    sys::SignalForwarder guard(child);
                    return child.wait();
                })
                .map<int>([](auto status) {
                    return status.code().value_or(EXIT_FAILURE);
                })
                .on_error([](auto error) {
                    spdlog::warn("Command execution failed: {}", error.what());
                })
                .on_success([](auto status) {
                    spdlog::debug("Running command. [Exited with {0}]", status);
                });
    }
}
