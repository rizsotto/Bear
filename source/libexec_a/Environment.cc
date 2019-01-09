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

#include "config.h"

#include "libexec_a/Environment.h"
#include "libexec_a/Array.h"

#if defined HAVE_NSGETENVIRON
# include <crt_externs.h>
#else
extern "C" char **environ;
#endif

namespace {

    constexpr char KEY_LIBRARY[]     = "INTERCEPT_SESSION_LIBRARY";
    constexpr char KEY_REPORTER[]    = "INTERCEPT_REPORT_COMMAND";
    constexpr char KEY_DESTINATION[] = "INTERCEPT_REPORT_DESTINATION";
    constexpr char KEY_VERBOSE[]     = "INTERCEPT_VERBOSE";

    const char *get_env(const char **envp, const char *key) noexcept {
        const size_t key_size = ::ear::array::length(key);

        for (const char **it = envp; *it != nullptr; ++it) {
            const char *const current = *it;
            // Is the key a prefix of the pointed string?
            if (not ::ear::array::equal_n(key, current, key_size))
                continue;
            // Is the next character is the equal sign in the pointed string?
            if (current[key_size] != '=')
                continue;
            // It must be the one! Calculate the address of the value string.
            return current + key_size + 1;
        }
        return nullptr;
    }

}


namespace ear {
    namespace environment {

        const char **current() noexcept {
#ifdef HAVE_NSGETENVIRON
            return const_cast<const char **>(*_NSGetEnviron());
#else
            return const_cast<const char **>(environ);
#endif
        }

        Session capture_session(const char **environment) noexcept {
            if (nullptr == environment)
                return Session {nullptr, nullptr, nullptr, false };
            else
                return Session {
                        get_env(environment, KEY_LIBRARY),
                        get_env(environment, KEY_REPORTER),
                        get_env(environment, KEY_DESTINATION),
                        get_env(environment, KEY_VERBOSE) != nullptr
                };
        }

    }
}
