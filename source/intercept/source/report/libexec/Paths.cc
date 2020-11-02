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

#include "report/libexec/Paths.h"

namespace {

    constexpr char PATH_SEPARATOR = ':';

    const char *next_path_separator(const char *const current, const char *const end) {
        auto it = current;
        while ((it != end) && (*it != PATH_SEPARATOR)) {
            ++it;
        }
        return it;
    }

    std::string_view first(std::string_view const& paths) {
        const char *const begin = paths.begin();
        const char *const end = next_path_separator(begin, paths.end());

        return std::string_view(begin, (end - begin));
    }

    std::string_view last(std::string_view const& paths) {
        return std::string_view(paths.end(), 0);
    }
}

namespace el {

    Paths::Paths(std::string_view path)
            : path_(path)
    { }

    Paths::iterator Paths::begin() const {
        return el::Paths::iterator(path_, true);
    }

    Paths::iterator Paths::end() const {
        return el::Paths::iterator(path_, false);
    }

    PathsIterator::PathsIterator(std::string_view paths, bool start)
            : paths_(paths)
            , current_(start ? first(paths) : last(paths))
    { }

    PathsIterator::reference PathsIterator::operator*() const {
        return current_;
    }

    PathsIterator PathsIterator::operator++(int) {
        PathsIterator result(*this);
        this->operator++();
        return result;
    }

    PathsIterator &PathsIterator::operator++() {
        if (current_.end() != paths_.end()) {
            const char *const begin = current_.end() + 1;
            const char *const end = next_path_separator(begin, paths_.end());

            current_ = std::string_view(begin, (end - begin));
        } else {
            current_ = last(paths_);
        }
        return *this;
    }

    bool PathsIterator::operator==(const PathsIterator &other) const {
        if (this == &other) {
            return true;
        }
        // simple equal would make two empty to be the same.
        return (paths_.begin() == other.paths_.begin()) &&
                (paths_.size() == other.paths_.size()) &&
                (current_.begin() == other.current_.begin()) &&
                (current_.size() == other.current_.size());
    }

    bool PathsIterator::operator!=(const PathsIterator &other) const {
        return !(this->operator==(other));
    }
}
