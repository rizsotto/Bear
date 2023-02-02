/*  Copyright (C) 2012-2023 by László Nagy
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

#include "report/libexec/Paths.h"
#include "report/libexec/Array.h"

namespace {

    const char *next_path_separator(const char *const current, const char *const end) {
        auto it = current;
        while ((it != end) && (*it != OS_PATH_SEPARATOR)) {
            ++it;
        }
        return it;
    }
}

namespace el {

    Paths::Paths(const char *const path) noexcept
            : begin_(path)
            , end_(array::end(path))
    { }

    Paths::Iterator Paths::begin() const {
        if (begin_ == end_)
            return {*this, nullptr, nullptr};

        const auto candidate = next_path_separator(begin_, end_);
        return {*this, begin_, candidate};
    }

    Paths::Iterator Paths::end() const {
        return {*this, nullptr, nullptr};
    }

    std::pair<const char *, const char *> Paths::next(const char *const current) const {
        if (current == end_)
            return std::make_pair(nullptr, nullptr);

        const auto begin = std::next(current);
        if (begin == end_)
            return std::make_pair(nullptr, nullptr);

        const auto candidate = next_path_separator(begin, end_);
        return std::make_pair(begin, candidate);
    }

    Paths::Iterator::Iterator(const Paths &paths, const char *begin, const char *end) noexcept
            : paths_(paths)
            , begin_(begin)
            , end_(end)
    { }

    Paths::Iterator::value_type Paths::Iterator::operator*() const {
        return std::string_view(begin_, (end_ - begin_));
    }

    Paths::Iterator Paths::Iterator::operator++(int) {
        Paths::Iterator result(*this);
        this->operator++();
        return result;
    }

    Paths::Iterator &Paths::Iterator::operator++() {
        const auto&[begin, end] = paths_.next(end_);
        begin_ = begin;
        end_ = end;
        return *this;
    }

    bool Paths::Iterator::operator==(const Paths::Iterator &other) const {
        return &paths_ == &other.paths_ && begin_ == other.begin_ && end_ == other.end_;
    }

    bool Paths::Iterator::operator!=(const Paths::Iterator &other) const {
        return !(this->operator==(other));
    }
}
