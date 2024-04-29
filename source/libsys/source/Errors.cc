/*  Copyright (C) 2012-2024 by László Nagy
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

#include "libsys/Errors.h"

#include "config.h"

#ifdef HAVE_STRERROR_R
#include <cstring>
#else
#include <fmt/format.h>
#endif

namespace sys {

    std::string error_string(const int error) noexcept
    {
#ifdef HAVE_STRERROR_R
#if defined(__GLIBC__) && defined(_GNU_SOURCE)
        char buffer[256];
        char *const result = ::strerror_r(error, buffer, 255);
        return {result};
#else
        char buffer[256];
        ::strerror_r(error, buffer, 255);
        return {buffer};
#endif
#else
        return fmt::format("{0}", error);
#endif
    }
}
