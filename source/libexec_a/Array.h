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

namespace ear {
    namespace array {

        /**
         * Return a pointer to the last element of a nullptr terminated array.
         *
         * @param it the input array to count,
         * @return the pointer which points the nullptr.
         */
        template<typename T>
        constexpr T *end(T *it) noexcept {
            if (it == nullptr)
                return nullptr;

            while (*it != 0)
                ++it;
            return it;
        }

        /**
         * Return the size of a nullptr terminated array.
         *
         * @param begin the input array to count,
         * @return the size of the array.
         */
        template<typename T>
        constexpr size_t length(T *const begin) noexcept {
            return end(begin) - begin;
        }

        /**
         * Re-implementation of std::copy to avoid `memmove` symbol.
         *
         * @tparam I input type
         * @tparam O output type
         * @param src_begin
         * @param src_end
         * @param dst_begin
         * @param dst_end
         * @return output iterator to the last copied element.
         */
        template<typename I, typename O>
        constexpr O *copy(I *const src_begin,
                          I *const src_end,
                          O *const dst_begin,
                          O *const dst_end) noexcept {
            auto src_it = src_begin;
            auto dst_it = dst_begin;
            for (; src_it != src_end && dst_it != dst_end;)
                *dst_it++ = *src_it++;

            return (dst_it != dst_end) ? dst_it : nullptr;
        }

    }
}
