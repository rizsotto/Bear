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

#include <list>
#include <algorithm>
#include <numeric>
#include <cstring>

#include "intercept_a/Environment.h"
#include "intercept_a/Interface.h"

namespace {

    constexpr char osx_preload_key[] = "DYLD_INSERT_LIBRARIES";
    constexpr char osx_namespace_key[] = "DYLD_FORCE_FLAT_NAMESPACE";
    constexpr char glibc_preload_key[] = "LD_PRELOAD";
    constexpr char cc_key[] = "CC";
    constexpr char cxx_key[] = "CXX";


    char **to_c_array(const std::map<std::string, std::string> &input) {
        const size_t result_size = input.size() + 1;
        auto const result = new char *[result_size];
        auto result_it = result;
        for (auto &it : input) {
            const size_t entry_size = it.first.size() + it.second.size() + 2;
            auto entry = new char [entry_size];

            auto key = std::copy(it.first.begin(), it.first.end(), entry);
            *key++ = '=';
            auto value = std::copy(it.second.begin(), it.second.end(), key);
            *value = '\0';

            *result_it++ = entry;
        }
        *result_it = nullptr;
        return result;
    }

    std::map<std::string, std::string> to_map(const char **const input) noexcept {
        std::map<std::string, std::string> result;
        if (input == nullptr)
            return result;

        for (const char **it = input; *it != nullptr; ++it) {
            auto end = *it + std::strlen(*it);
            auto sep = std::find(*it, end, '=');
            const std::string key = (sep != end) ? std::string(*it, sep) : std::string(*it, end);
            const std::string value = (sep != end) ? std::string(sep + 1, end) : std::string();
            result.emplace(key, value);
        }
        return result;
    }

    std::list<std::string> split(const std::string &input, const char sep) noexcept {
        std::list<std::string> result;

        std::string::size_type previous = 0;
        do {
            const std::string::size_type current = input.find(sep, previous);
            result.emplace_back(input.substr(previous, current - previous));
            previous = (current != std::string::npos) ? current + 1 : current;
        } while (previous != std::string::npos);

        return result;
    }

}

namespace pear {

    Environment::Environment(const std::map<std::string, std::string> &environ) noexcept
            : data_(to_c_array(environ))
    { }

    Environment::~Environment() noexcept {
        for (char **it = data_; *it != nullptr; ++it) {
            delete [] *it;
        }
        delete [] data_;
    }

    const char **Environment::data() const noexcept {
        return const_cast<const char **>(data_);
    }


    Environment::Builder::Builder(const char **environment) noexcept
            : environ_(to_map(environment))
    { }

    Environment::Builder &
    Environment::Builder::add_reporter(const char *reporter) noexcept {
        environ_.insert_or_assign(::pear::env::reporter_key, reporter);
        return *this;
    }

    Environment::Builder &
    Environment::Builder::add_destination(const char *destination) noexcept {
        environ_.insert_or_assign(::pear::env::destination_key, destination);
        return *this;
    }

    Environment::Builder &
    Environment::Builder::add_verbose(bool verbose) noexcept {
        if (verbose) {
            environ_.insert_or_assign(::pear::env::verbose_key, "1");
        }
        return *this;
    }

    Environment::Builder &
    Environment::Builder::add_library(const char *library) noexcept {
#ifdef APPLE
        const std::string key = osx_preload_key;
#else
        const std::string key = glibc_preload_key;
#endif
        const std::string value = library;
        if (auto preloads = environ_.find(key); preloads != environ_.end()) {
            auto paths = split(preloads->second, ':');
            if (std::find(paths.begin(), paths.end(), value) == paths.end()) {
                paths.emplace_front(value);
                const std::string updated =
                        std::accumulate(paths.begin(), paths.end(),
                                        std::string(),
                                        [](std::string acc, std::string item) {
                                            return (acc.empty()) ? item : acc + ':' + item;
                                        });
                preloads->second = updated;
            }
        } else {
            environ_.emplace(key, value);
        }
#ifdef APPLE
        environ_.insert_or_assign(osx_namespace_key, "1");
#endif
        return *this;
    }

    Environment::Builder &
    Environment::Builder::add_cc_compiler(const char *compiler, const char *wrapper) noexcept {
        environ_.insert_or_assign(cc_key, wrapper);
        environ_.insert_or_assign(::pear::env::cc_key, compiler);
        return *this;
    }

    Environment::Builder &
    Environment::Builder::add_cxx_compiler(const char *compiler, const char *wrapper) noexcept {
        environ_.insert_or_assign(cxx_key, wrapper);
        environ_.insert_or_assign(::pear::env::cxx_key, compiler);
        return *this;
    }

    EnvironmentPtr Environment::Builder::build() const noexcept {
        return std::unique_ptr<Environment>(new Environment(environ_));
    }

}
