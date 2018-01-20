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

    namespace string {

        inline
        const char *end(const char *it) noexcept {
            if (it == nullptr)
                return nullptr;

            while (*it != 0)
                ++it;
            return it;
        }

        inline
        size_t length(const char *const begin) noexcept {
            return end(begin) - begin;
        }

        inline
        bool equal(const char *const lhs, const char *const rhs, size_t length) {
            for (int idx = 0; idx < length; ++idx) {
                if (lhs[idx] != rhs[idx])
                    return false;
            }
            return true;
        }
    }

    template <unsigned int Size>
    class String {
    public:
        explicit String(const char *) noexcept;

        ~String() noexcept = default;

        const char *begin() const noexcept;

        const char *end() const noexcept;

    public:
        String() noexcept = delete;

        String(const String &) = delete;

        String(String &&) noexcept = delete;

        String &operator=(const String &) = delete;

        String &operator=(String &&) noexcept = delete;

    private:
        char buffer_[Size];
    };


    template <unsigned int Size>
    String<Size>::String(const char *const input) noexcept
            : buffer_() {
        // Copy over the content from input.
        const char *in_it = input;
        char *out_it = buffer_;
        char *const out_end = buffer_ + Size;
        while (*in_it != 0 && out_it != out_end)
            *out_it++ = *in_it++;
        // Close the string with the zero value (or mark the whole zero).
        if (out_it != out_end)
            *out_it = 0;
        else
            *buffer_ = 0;
    }

    template <unsigned int Size>
    const char *String<Size>::begin() const noexcept {
        return buffer_;
    }

    template <unsigned int Size>
    const char *String<Size>::end() const noexcept {
        return ::ear::string::end(buffer_);
    }

}