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

#pragma once

#include <cstddef>
#include <algorithm>

#include "String.h"

#if defined HAVE_NSGETENVIRON
# include <crt_externs.h>
#else
extern "C" char **environ;
#endif

namespace ear {

    constexpr char target_env_key[] = "BEAR_TARGET";
    constexpr char library_env_key[] = "BEAR_LIBRARY";
    constexpr char wrapper_env_key[] = "BEAR_WRAPPER";

    class Environment {
    public:
        static const char **current() noexcept;

        static Environment* create(const char**, void *) noexcept;

        ~Environment() noexcept = default;

        const char *wrapper() const noexcept;

        const char *target() const noexcept;

        const char *library() const noexcept;

    public:
        Environment() noexcept = delete;

        Environment(const Environment &) = delete;

        Environment(Environment &&) noexcept = delete;

        Environment &operator=(const Environment &) = delete;

        Environment &operator=(Environment &&) noexcept = delete;

    protected:
        Environment(const char *target,
                    const char *library,
                    const char *wrapper) noexcept;

        static const char *get_env(const char **, const char *) noexcept;

    private:
        ::ear::String<1024> target_;
        ::ear::String<8192> library_;
        ::ear::String<8192> wrapper_;
    };


    inline
    const char **Environment::current() noexcept {
#ifdef HAVE_NSGETENVIRON
        return const_cast<const char **>(*_NSGetEnviron());
#else
        return const_cast<const char **>(environ);
#endif
    }

    inline
    Environment* Environment::create(const char** current, void* place) noexcept {

        if (current == nullptr)
            return nullptr;

        auto target_env = Environment::get_env(current, ::ear::target_env_key);
        auto libray_env = Environment::get_env(current, ::ear::library_env_key);
        auto wrapper_env = Environment::get_env(current, ::ear::wrapper_env_key);
        if (target_env == nullptr || libray_env == nullptr || wrapper_env == nullptr)
            return nullptr;

        return new(place) ::ear::Environment(target_env, libray_env, wrapper_env);
    }

    inline
    Environment::Environment(const char *target,
                const char *library,
                const char *wrapper) noexcept
            : target_(target)
            , library_(library)
            , wrapper_(wrapper) {
    }

    inline
    const char *Environment::wrapper() const noexcept {
        return wrapper_.begin();
    }

    inline
    const char *Environment::target() const noexcept {
        return target_.begin();
    }

    inline
    const char *Environment::library() const noexcept {
        return library_.begin();
    }

    inline
    const char *Environment::get_env(const char **envp, const char *key) noexcept {
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