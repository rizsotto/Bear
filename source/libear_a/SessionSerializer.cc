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

#include "libear_a/SessionSerializer.h"

namespace ear {

    SessionSerializer::SessionSerializer(Session const &session)
            : session_(session)
    { }

    std::size_t SessionSerializer::estimate() const noexcept {
        return (session_.verbose) ? 5 : 4;
    }

    char const **SessionSerializer::copy(char const **begin, char const **end) const noexcept {
        char const **it = begin;
        *it++ = session_.reporter;
        *it++ = "--report";
        *it++ = destination_flag;
        *it++ = session_.destination;

        if (!session_.verbose)
            return it;

        *it++ = verbose_flag;
        return it;
    }

    LibrarySessionSerializer::LibrarySessionSerializer(LibrarySession const &session)
            : session_(session)
    { }

    std::size_t LibrarySessionSerializer::estimate() const noexcept {
        return SessionSerializer(session_.session).estimate() + 2;
    }

    char const **LibrarySessionSerializer::copy(char const **begin, char const **end) const noexcept {
        char const **it = SessionSerializer(session_.session).copy(begin, end);
        *it++ = library_flag;
        *it++ = session_.library;
        return it;
    }

}