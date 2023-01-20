/*  Copyright (C) 2012-2022 by László Nagy
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

#include "Guard.h"
#include "libsys/Environment.h"

#include <cstring>
#include <functional>
#include <unistd.h>

namespace {

    const char** to_c_array(const std::map<std::string, std::string>& entries)
    {
        // allocate the array for the pointer array
        const auto results = new const char*[entries.size() + 1];
        // copy the elements
        auto results_it = results;
        for (const auto& entry : entries) {
            const auto& [key, value] = entry;
            // allocate the entry
            const size_t entry_size = key.size() + value.size() + 2;
            auto result = new char[entry_size];
            // assemble the content
            {
                auto it = std::copy(key.begin(), key.end(), result);
                *it++ = '=';
                it = std::copy(value.begin(), value.end(), it);
                *it = '\0';
            }
            // put into the pointer array
            *results_it++ = result;
        }
        // set the terminator null pointer
        *results_it = nullptr;
        return results;
    }
}

namespace sys::env {

    Guard::Guard(const std::map<std::string, std::string> &environment)
            : data_(to_c_array(environment))
    {
    }

    Guard::~Guard() noexcept
    {
        for (const char** it = data_; *it != nullptr; ++it) {
            delete[] * it;
        }
        delete[] data_;
    }

    const char** Guard::data() const
    {
        return data_;
    }

    Vars from(const char** value)
    {
        Vars result;

        if (!value)
            return result;

        for (const char** it = value; *it != nullptr; ++it) {
            const auto end = *it + std::strlen(*it);
            const auto sep = std::find(*it, end, '=');
            const std::string key = (sep != end) ? std::string(*it, sep) : std::string(*it, end);
            const std::string value = (sep != end) ? std::string(sep + 1, end) : std::string();
            result.emplace(key, value);
        }

        return result;
    }

    const Vars& get()
    {
        static Vars result;
        static bool initialized = false;

        if (!initialized) {
            result = from(const_cast<const char **>(environ));
            initialized = true;
        }

        return result;
    }
}
