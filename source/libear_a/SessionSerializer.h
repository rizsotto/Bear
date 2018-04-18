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

#include "libear_a/Session.h"

namespace ear {

    class Serializable {
    public:
        virtual ~Serializable() noexcept = default;

        virtual std::size_t estimate() const noexcept = 0;
        virtual char const **copy(char const **begin, char const **end) const noexcept = 0;
    };

    class SessionSerializer : public Serializable {
    public:
        explicit SessionSerializer(Session const &session);

        std::size_t estimate() const noexcept override;

        char const **copy(char const **begin, char const **end) const noexcept override;

    private:
        Session const &session_;
    };

    class LibrarySessionSerializer : public Serializable {
    public:
        explicit LibrarySessionSerializer(LibrarySession const &session);

        std::size_t estimate() const noexcept override;

        char const **copy(char const **begin, char const **end) const noexcept override;

    private:
        LibrarySession const &session_;
    };

}
