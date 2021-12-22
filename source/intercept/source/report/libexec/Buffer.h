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

#pragma once

#include "config.h"

namespace el {

    /**
     * Represents a character buffer.
     *
     * Define helper methods to persist character sequences. The covered
     * functionality is not more than a `memcpy` to a static char array.
     */
    class Buffer {
    public:
        /**
         * Takes the memory addresses of the buffer.
         *
         * @param begin of the buffer.
         * @param end of the buffer.
         */
        Buffer(char* begin, char* end) noexcept;

        ~Buffer() noexcept = default;

        /**
         * Copy the input to the buffer.
         *
         * @param input to persist.
         * @return the address of the persisted input.
         */
        char const* store(char const* input) noexcept;

        NON_DEFAULT_CONSTRUCTABLE(Buffer)
        NON_COPYABLE_NOR_MOVABLE(Buffer)

    private:
        char* top_;
        char* const end_;
    };

    inline
    Buffer::Buffer(char* const begin, char* const end) noexcept
            : top_(begin)
            , end_(end)
    {
    }
}
