/*  Copyright (C) 2012-2021 by László Nagy
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

#include "libsys/Path.h"

#include <algorithm>
#include <numeric>

namespace {

    std::list<fs::path> split_by(const std::string &input, const char sep)
    {
        std::list<fs::path> result;
        // make an early return if there is no chance to have something.
        if (input.empty()) {
            return result;
        }
        // otherwise start to collect the elements into result.
        std::string::size_type previous = 0;
        do {
            const std::string::size_type current = input.find(sep, previous);
            result.emplace_back(input.substr(previous, current - previous));
            previous = (current != std::string::npos) ? current + 1 : current;
        } while (previous != std::string::npos);

        return result;
    }

    std::string join_with(const std::list<fs::path> &input, const char sep)
    {
        std::string result;
        for (auto it = input.begin(); it != input.end(); ++it) {
            if (it != input.begin()) {
                result.push_back(sep);
            }
            result.append(it->string());
        }
        return result;
    }
}

namespace sys::path {

    std::list<fs::path> split(const std::string &input)
    {
        return split_by(input, OS_PATH_SEPARATOR);
    }

    std::string join(const std::list<fs::path> &input)
    {
        return join_with(input, OS_PATH_SEPARATOR);
    }

    rust::Result<fs::path> get_cwd()
    {
        std::error_code error_code;
        auto result = fs::current_path(error_code);
        return (error_code)
               ? rust::Result<fs::path>(rust::Err(std::runtime_error(error_code.message())))
               : rust::Result<fs::path>(rust::Ok(result));
    }
}
