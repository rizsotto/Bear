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

#include "State.h"

#if defined HAVE_NSGETENVIRON
# include <crt_externs.h>
#else
extern "C" char **environ;
#endif


namespace ear {

    const char **State::current() noexcept {
#ifdef HAVE_NSGETENVIRON
        return const_cast<const char **>(*_NSGetEnviron());
#else
        return const_cast<const char **>(environ);
#endif
    }

    State* State::create(const char** current, void* place) noexcept {

        if (current == nullptr)
            return nullptr;

        auto target_env = State::get_env(current, ::ear::destination_env_key);
        auto libray_env = State::get_env(current, ::ear::library_env_key);
        auto reporter_env = State::get_env(current, ::ear::reporter_env_key);
        auto verbose_env = State::get_env(current, ::ear::verbose_env_key);
        if (target_env == nullptr || libray_env == nullptr || reporter_env == nullptr)
            return nullptr;

        return new(place) ::ear::State(target_env, libray_env, reporter_env,
                                       verbose_env != nullptr);
    }

    State *State::capture(void *place) noexcept {
        auto current = State::current();
        return State::create(current, place);
    }

    State::State(const char *target,
                 const char *library,
                 const char *reporter,
                 bool verbose) noexcept
            : target_(target)
            , library_(library)
            , reporter_(reporter)
            , verbose_(verbose) {
    }

    LibrarySession State::get_input() const noexcept {
        return LibrarySession {
                Session {
                        reporter_.begin(),
                        target_.begin(),
                        verbose_
                },
                library_.begin(),
        };
    }

    const char *State::get_env(const char **envp, const char *key) noexcept {
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
