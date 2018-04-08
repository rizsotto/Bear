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

#include "config.h"

#include <cstddef>
#include <algorithm>

#include "libear_a/String.h"
#include "libear_a/Input.h"

#if defined HAVE_NSGETENVIRON
# include <crt_externs.h>
#else
extern "C" char **environ;
#endif

namespace ear {

    class Catcher {
    public:
        static const char **current() noexcept;

        static Catcher* create(const char**, void *) noexcept;

        ~Catcher() noexcept = default;

        const char *reporter() const noexcept;

        const char *target() const noexcept;

        const char *library() const noexcept;

    public:
        Catcher() noexcept = delete;

        Catcher(const Catcher &) = delete;

        Catcher(Catcher &&) noexcept = delete;

        Catcher &operator=(const Catcher &) = delete;

        Catcher &operator=(Catcher &&) noexcept = delete;

    protected:
        Catcher(const char *target,
                    const char *library,
                    const char *reporter) noexcept;

        static const char *get_env(const char **, const char *) noexcept;

    private:
        ::ear::String<4096> target_;
        ::ear::String<8192> library_;
        ::ear::String<8192> reporter_;
    };


    inline
    const char **Catcher::current() noexcept {
#ifdef HAVE_NSGETENVIRON
        return const_cast<const char **>(*_NSGetEnviron());
#else
        return const_cast<const char **>(environ);
#endif
    }

    inline
    Catcher* Catcher::create(const char** current, void* place) noexcept {

        if (current == nullptr)
            return nullptr;

        auto target_env = Catcher::get_env(current, ::ear::target_env_key);
        auto libray_env = Catcher::get_env(current, ::ear::library_env_key);
        auto reporter_env = Catcher::get_env(current, ::ear::reporter_env_key);
        if (target_env == nullptr || libray_env == nullptr || reporter_env == nullptr)
            return nullptr;

        return new(place) ::ear::Catcher(target_env, libray_env, reporter_env);
    }

    inline
    Catcher::Catcher(const char *target,
                const char *library,
                const char *reporter) noexcept
            : target_(target)
            , library_(library)
            , reporter_(reporter) {
    }

    inline
    const char *Catcher::reporter() const noexcept {
        return reporter_.begin();
    }

    inline
    const char *Catcher::target() const noexcept {
        return target_.begin();
    }

    inline
    const char *Catcher::library() const noexcept {
        return library_.begin();
    }

    inline
    const char *Catcher::get_env(const char **envp, const char *key) noexcept {
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