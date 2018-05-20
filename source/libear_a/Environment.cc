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

#include "config.h"

#include "libear_a/Environment.h"
#include "libear_a/String.h"
#include "libear_a/Interface.h"

#if defined HAVE_NSGETENVIRON
# include <crt_externs.h>
#else
extern "C" char **environ;
#endif

namespace {

    const char *get_env(const char **envp, const char *key) noexcept {
        const size_t key_size = ::ear::string::length(key);

        for (const char **it = envp; *it != nullptr; ++it) {
            const char *const current = *it;
            // Is the key a prefix of the pointed string?
            if (not ::ear::string::equal(key, current, key_size))
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

        const LibrarySession *
        capture(LibrarySession &session, Storage &storage, const char **environment) noexcept {
            if (nullptr == environment)
                return nullptr;

            session.destination = storage.store(get_env(environment, ::ear::destination_env_key));
            session.library     = storage.store(get_env(environment, ::ear::library_env_key));
            session.reporter    = storage.store(get_env(environment, ::ear::reporter_env_key));
            session.verbose     = get_env(environment, ::ear::verbose_env_key) != nullptr;

            return (session.destination == nullptr ||
                    session.library == nullptr ||
                    session.reporter == nullptr)
                ? nullptr : &session;
        }

    }

}
