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

#include <unistd.h>
#include <algorithm>

#include "libpear_a/Environment.h"
#include "libear_a/Input.h"

namespace {

#ifdef APPLE
    constexpr char osx_preload_key[] = "DYLD_INSERT_LIBRARIES";
    constexpr char osx_namespace_key[] = "DYLD_FORCE_FLAT_NAMESPACE";
#else
    constexpr char glibc_preload_key[] = "LD_PRELOAD";
#endif

    std::vector<const char *> render(std::vector<std::string> const &input) noexcept {
        std::vector<const char *> result;
        result.reserve(input.size() + 1);
        std::transform(input.begin(), input.end(), std::back_inserter(result),
                       [](auto &str) { return str.c_str(); });
        result.push_back(nullptr);
        return result;
    }

    std::vector<std::string> copy(const char **const input) noexcept {
        std::vector<std::string> result;
        for (const char **it = input; *it != nullptr; ++it) {
            result.emplace_back(std::string(*it));
        }
        return result;
    }

    std::string env_key_value(const char *const key, std::string const &value) noexcept {
        return std::string(key) + '=' + value;
    }

    std::tuple<std::string_view, std::string_view> env_key_value(const std::string &input) noexcept {
        auto equal_pos = input.find_first_of('=', 0);
        return (equal_pos == std::string::npos)
               ? std::tuple<std::string_view, std::string_view>(
                        std::string_view(input.c_str(), input.size()),
                        std::string_view())
               : std::tuple<std::string_view, std::string_view>(
                        std::string_view(input.c_str(), equal_pos),
                        std::string_view(input.c_str() + equal_pos, input.size() - (equal_pos + 1)));
    };

    constexpr bool loader_related(const std::string_view &input) noexcept {
#ifdef APPLE
        return input == osx_preload_key
            || input == osx_namespace_key;
#else
        return input == glibc_preload_key;
#endif
    }

    std::vector<std::string> update_loader_related(std::vector<std::string> const &input,
                                                   std::string const &library) noexcept {
        // TODO: don't overwrite, but extend the list
        std::vector<std::string> result;
        if (!library.empty()) {
#ifdef APPLE
            result.emplace_back(env_key_value(osx_preload_key, library));
            result.emplace_back(env_key_value(osx_namespace_key, std::string("1")));
#else
            result.emplace_back(env_key_value(glibc_preload_key, library));
#endif
        }
        return result;
    }
}

namespace pear {
    Environment::Environment(std::vector<std::string> &&environ) noexcept
            : environ_(environ)
            , rendered_(render(environ_))
    { }

    const char **Environment::as_array() const noexcept {
        return const_cast<const char **>(rendered_.data());
    }


    Environment::Builder::Builder() noexcept
            : Environment::Builder(const_cast<const char **>(environ))
    { }

    Environment::Builder::Builder(const char **environment) noexcept
            : environ_(copy(environment))
            , reporter_()
            , target_()
            , library_()
    { }

    Environment::Builder &Environment::Builder::add_reporter(const char *reporter) noexcept {
        reporter_ = (reporter != nullptr) ? std::string(reporter) : std::string();
        return *this;
    }

    Environment::Builder &Environment::Builder::add_target(const char *target) noexcept {
        target_ = (target != nullptr) ? std::string(target) : std::string();
        return *this;
    }

    Environment::Builder &Environment::Builder::add_library(const char *library) noexcept {
        library_= (library != nullptr) ? std::string(library) : std::string();
        return *this;
    }

    EnvironmentPtr Environment::Builder::build() const noexcept {
        std::vector<std::string> result;
        std::vector<std::string> affected;
        // copy those which are not relevant to pear
        std::partition_copy(environ_.begin(), environ_.end(),
                            std::back_inserter(result), std::back_inserter(affected),
                            [](auto &str) {
                                auto key_value = env_key_value(str);
                                auto key = std::get<0>(key_value);
                                return key != ::ear::destination_env_key
                                    && key != ::ear::library_env_key
                                    && key != ::ear::reporter_env_key
                                    && !loader_related(key);
                            });
        // overwrite the pear ones
        if (!reporter_.empty()) {
            result.emplace_back(env_key_value(::ear::reporter_env_key, reporter_));
        }
        if (!target_.empty()) {
            result.emplace_back(env_key_value(::ear::destination_env_key, target_));
        }
        if (!library_.empty()) {
            result.emplace_back(env_key_value(::ear::library_env_key, library_));
        }
        // add the loader ones
        auto loader_related = update_loader_related(affected, library_);
        std::copy(loader_related.begin(), loader_related.end(), std::back_inserter(result));

        return std::unique_ptr<Environment>(new Environment(std::move(result)));
    }

}
