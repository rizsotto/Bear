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

#include <iterator>
#include <string_view>

namespace el {

    class PathsIterator;

    class Paths {
    public:
        using iterator = PathsIterator;

    public:
        explicit Paths(std::string_view path);

        [[nodiscard]] iterator begin() const;
        [[nodiscard]] iterator end() const;

    private:
        std::string_view path_;
    };

    class PathsIterator {
    public:
        using difference_type = std::ptrdiff_t;
        using iterator_category = std::input_iterator_tag;
        using value_type = std::string_view;
        using pointer = value_type const*;
        using reference = value_type const&;

    public:
        PathsIterator(std::string_view paths, bool start);

        reference operator*() const;

        PathsIterator operator++(int);
        PathsIterator &operator++();

        bool operator==(const PathsIterator &other) const;
        bool operator!=(const PathsIterator &other) const;

    private:
        std::string_view paths_;
        std::string_view current_;
    };
}