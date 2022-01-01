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

#pragma once

#include "config.h"

#include <iterator>
#include <string_view>

namespace el {

    class Paths {
    public:
        class Iterator;
        friend class Iterator;

    public:
        explicit Paths(const char *path) noexcept;

        NON_DEFAULT_CONSTRUCTABLE(Paths)
        NON_COPYABLE_NOR_MOVABLE(Paths)

        [[nodiscard]] Iterator begin() const;
        [[nodiscard]] Iterator end() const;

    private:
        [[nodiscard]] std::pair<const char *, const char *> next(const char *current) const;

    private:
        const char *const begin_;
        const char *const end_;
    };

    class Paths::Iterator {
    public:
        using difference_type = std::ptrdiff_t;
        using iterator_category = std::input_iterator_tag;
        using value_type = std::string_view;
        using pointer = value_type const*;
        using reference = value_type const&;

    public:
        Iterator(const Paths &paths, const char *begin, const char *end) noexcept;

        value_type operator*() const;

        Iterator operator++(int);
        Iterator &operator++();

        bool operator==(const Iterator &other) const;
        bool operator!=(const Iterator &other) const;

    private:
        const Paths &paths_;
        const char *begin_;
        const char *end_;
    };
}