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

#pragma once

namespace el {

    class Buffer;

    /**
     * Represents an intercept session parameter set.
     *
     * It does not own the memory (of the pointed areas).
     */
    struct Session {
        char const* reporter;
        char const* destination;
        bool verbose;
    };

    namespace session {

        // Util method to create instance.
        inline constexpr Session init() noexcept
        {
            return { nullptr, nullptr, true };
        }

        // Util method to initialize instance.
        void from(Session& session, const char** environment) noexcept;

        // Util method to store the values.
        void persist(Session& session, char* begin, char* end) noexcept;

        // Util method to check if session is initialized.
        bool is_valid(Session const& session) noexcept;
    }
}
