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

#include "config.h"

#include "libexec/Environment.h"

#include <cstring>
#include <functional>
#include <list>
#include <numeric>
#include <string>

#include "Environment.h"

namespace {

    constexpr char OSX_PRELOAD_KEY[] = "DYLD_INSERT_LIBRARIES";
    constexpr char OSX_NAMESPACE_KEY[] = "DYLD_FORCE_FLAT_NAMESPACE";
    constexpr char GLIBC_PRELOAD_KEY[] = "LD_PRELOAD";

    using env_t = std::map<std::string, std::string>;
    using mapper_t = std::function<std::string(const std::string&, const std::string&)>;

    char**
    to_c_array(const env_t& input)
    {
        const size_t result_size = input.size() + 1;
        const auto result = new char*[result_size];
        auto result_it = result;
        for (const auto& it : input) {
            const size_t entry_size = it.first.size() + it.second.size() + 2;
            auto entry = new char[entry_size];

            auto key = std::copy(it.first.begin(), it.first.end(), entry);
            *key++ = '=';
            auto value = std::copy(it.second.begin(), it.second.end(), key);
            *value = '\0';

            *result_it++ = entry;
        }
        *result_it = nullptr;
        return result;
    }

    env_t to_map(const char** const input) noexcept
    {
        env_t result;
        if (input == nullptr)
            return result;

        for (const char** it = input; *it != nullptr; ++it) {
            const auto end = *it + std::strlen(*it);
            const auto sep = std::find(*it, end, '=');
            const std::string key = (sep != end) ? std::string(*it, sep) : std::string(*it, end);
            const std::string value = (sep != end) ? std::string(sep + 1, end) : std::string();
            result.emplace(key, value);
        }
        return result;
    }

    std::list<std::string>
    split(const std::string& input, const char sep) noexcept
    {
        std::list<std::string> result;

        std::string::size_type previous = 0;
        do {
            const std::string::size_type current = input.find(sep, previous);
            result.emplace_back(input.substr(previous, current - previous));
            previous = (current != std::string::npos) ? current + 1 : current;
        } while (previous != std::string::npos);

        return result;
    }

    std::string
    merge_into_paths(const std::string& current, const std::string& value) noexcept
    {
        auto paths = split(current, ':');
        if (std::find(paths.begin(), paths.end(), value) == paths.end()) {
            paths.emplace_front(value);
            return std::accumulate(paths.begin(),
                paths.end(),
                std::string(),
                [](std::string acc, std::string item) {
                    return (acc.empty()) ? item : acc + ':' + item;
                });
        } else {
            return current;
        }
    }

    void insert_or_assign(env_t& target, const char* key, const char* value) noexcept
    {
        if (auto it = target.find(key); it != target.end()) {
            it->second = std::string(value);
        } else {
            target.emplace(key, std::string(value));
        }
    }

    void insert_or_merge(env_t& target,
        const char* key,
        const char* value,
        const mapper_t& merger) noexcept
    {
        if (auto it = target.find(key); it != target.end()) {
            it->second = merger(it->second, std::string(value));
        } else {
            target.emplace(key, std::string(value));
        }
    }

}

namespace er {

    Environment::Environment(
        const std::map<std::string, std::string>& environ) noexcept
            : data_(to_c_array(environ))
    {
    }

    Environment::~Environment() noexcept
    {
        for (char** it = data_; *it != nullptr; ++it) {
            delete[] * it;
        }
        delete[] data_;
    }

    const char**
    Environment::data() const noexcept
    {
        return const_cast<const char**>(data_);
    }

    Environment::Builder::Builder(const char** environment) noexcept
            : environ_(to_map(environment))
    {
    }

    Environment::Builder&
    Environment::Builder::add_reporter(const char* reporter) noexcept
    {
        insert_or_assign(environ_, el::env::KEY_REPORTER, reporter);
        return *this;
    }

    Environment::Builder&
    Environment::Builder::add_destination(const char* destination) noexcept
    {
        insert_or_assign(environ_, el::env::KEY_DESTINATION, destination);
        return *this;
    }

    Environment::Builder&
    Environment::Builder::add_verbose(bool verbose) noexcept
    {
        if (verbose) {
            insert_or_assign(environ_, el::env::KEY_VERBOSE, "1");
        }
        return *this;
    }

    Environment::Builder&
    Environment::Builder::add_library(const char* library) noexcept
    {
        insert_or_assign(environ_, el::env::KEY_LIBRARY, library);
#ifdef APPLE
        insert_or_assign(environ_, OSX_NAMESPACE_KEY, "1");
        const char* key = OSX_PRELOAD_KEY;
#else
        const char* key = GLIBC_PRELOAD_KEY;
#endif
        insert_or_merge(environ_, key, library, merge_into_paths);
        return *this;
    }

    EnvironmentPtr
    Environment::Builder::build() const noexcept
    {
        return std::unique_ptr<Environment>(new Environment(environ_));
    }

}
