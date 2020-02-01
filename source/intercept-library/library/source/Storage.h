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

namespace ear {

    /**
     * Represents a character storage.
     *
     * Define helper methods to persist character sequences. The covered
     * functionality is not more than a `memcpy` to a static char array.
     */
    class Storage {
    public:
        /**
         * Takes the memory addresses of the buffer.
         *
         * @param begin of the buffer.
         * @param end of the buffer.
         */
        Storage(char *begin, char *end) noexcept;

        ~Storage() noexcept = default;

        /**
         * Copy the input to the buffer.
         *
         * @param input to persist.
         * @return the address of the persisted input.
         */
        char const *store(char const *input) noexcept;

    public:
        Storage(Storage const &) = delete;

        Storage(Storage &&) noexcept = delete;

        Storage &operator=(Storage const &) = delete;

        Storage &operator=(Storage &&) noexcept = delete;

    private:
        char *const begin_;
        char *const end_;
        char *top_;
    };

    inline
    Storage::Storage(char *const begin, char *const end) noexcept
            : begin_(begin)
            , end_(end)
            , top_(begin)
    { }
}
