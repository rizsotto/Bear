/*  Copyright (C) 2012-2018 by László Nagy
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

#include "libexec_a/Session.h"

#include "libexec_a/Environment.h"
#include "libexec_a/Storage.h"


namespace {

    constexpr char KEY_LIBRARY[]     = "INTERCEPT_SESSION_LIBRARY";
    constexpr char KEY_REPORTER[]    = "INTERCEPT_REPORT_COMMAND";
    constexpr char KEY_DESTINATION[] = "INTERCEPT_REPORT_DESTINATION";
    constexpr char KEY_VERBOSE[]     = "INTERCEPT_VERBOSE";
}

namespace ear {

    Session::Session(
            char const *library,
            char const *reporter,
            char const *destination,
            bool verbose) noexcept
            : library(library)
            , reporter(reporter)
            , destination(destination)
            , verbose(verbose)
    { }

    Session Session::from(const char **environment) noexcept {
        if (nullptr == environment)
            return {};
        else
            return {
                    environment::get_env_value(environment, KEY_LIBRARY),
                    environment::get_env_value(environment, KEY_REPORTER),
                    environment::get_env_value(environment, KEY_DESTINATION),
                    environment::get_env_value(environment, KEY_VERBOSE) != nullptr
            };
    }

    const char *Session::get_library() const {
        return library;
    }

    const char *Session::get_reporter() const {
        return reporter;
    }

    const char *Session::get_destination() const {
        return destination;
    }

    bool Session::is_verbose() const {
        return verbose;
    }

    bool Session::is_not_valid() const noexcept {
        return (library == nullptr || reporter == nullptr || destination == nullptr);
    }

    void Session::persist(Storage &storage) noexcept {
        if (is_not_valid())
            return;

        library = storage.store(library);
        reporter = storage.store(reporter);
        destination = storage.store(destination);
    }
}
