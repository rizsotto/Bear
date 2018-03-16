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

#include "Environment.h"

#include <unistd.h>
#include <algorithm>

namespace {
    constexpr char target_env_key[] = "BEAR_TARGET";
    constexpr char library_env_key[] = "BEAR_LIBRARY";
    constexpr char wrapper_env_key[] = "BEAR_WRAPPER";

    constexpr char glibc_preload_key[] = "LD_PRELOAD";

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
        // TODO: make it work on OSX too.
        return input == glibc_preload_key;
    }

    std::vector<std::string> update_loader_related(std::vector<std::string> const &input,
                                                   std::string const &library) noexcept {
        // TODO: don't overwrite, but extend the list
        // TODO: make it work on OSX too
        std::vector<std::string> result;
        result.emplace_back(env_key_value(glibc_preload_key, library));
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
            , wrapper_()
            , target_()
            , library_()
    { }

    Environment::Builder &Environment::Builder::add_wrapper(const char *wrapper) noexcept {
        wrapper_ = (wrapper != nullptr) ? std::string(wrapper) : std::string();
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
                                return key != target_env_key
                                    && key != library_env_key
                                    && key != wrapper_env_key
                                    && !loader_related(key);
                            });
        // overwrite the pear ones
        if (!wrapper_.empty()) {
            result.emplace_back(env_key_value(wrapper_env_key, wrapper_));
        }
        if (!target_.empty()) {
            result.emplace_back(env_key_value(target_env_key, target_));
        }
        if (!library_.empty()) {
            result.emplace_back(env_key_value(library_env_key, library_));
        }
        // add the loader ones
        auto loader_related = update_loader_related(affected, library_);
        std::copy(loader_related.begin(), loader_related.end(), std::back_inserter(result));

        return std::unique_ptr<Environment>(new Environment(std::move(result)));
    }

}
