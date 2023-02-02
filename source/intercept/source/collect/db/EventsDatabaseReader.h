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

#pragma once

#include "config.h"
#include "libresult/Result.h"
#include "intercept.pb.h"

#include <iosfwd>
#include <filesystem>
#include <memory>
#include <optional>

namespace fs = std::filesystem;

namespace ic::collect::db {

    using EventPtr = std::shared_ptr<rpc::Event>;

    class EventsDatabaseReader {
    public:
        class Iterator;
        friend class Iterator;

        using Ptr = std::shared_ptr<EventsDatabaseReader>;
        using StreamPtr = std::unique_ptr<std::istream>;

        [[nodiscard]] static rust::Result<EventsDatabaseReader::Ptr> from(const fs::path &path);

        [[nodiscard]] Iterator begin() noexcept;
        [[nodiscard]] Iterator end() noexcept;

    private:
        [[nodiscard]] std::optional<rust::Result<EventPtr>> next() noexcept;
        [[nodiscard]] std::optional<rust::Result<std::string>> next_line() noexcept;
        [[nodiscard]] rust::Result<EventPtr> from_json(const std::string &) noexcept;

    public:
        explicit EventsDatabaseReader(fs::path path, StreamPtr file) noexcept;

        NON_DEFAULT_CONSTRUCTABLE(EventsDatabaseReader)
        NON_COPYABLE_NOR_MOVABLE(EventsDatabaseReader)

    private:
        fs::path path_;
        StreamPtr file_;
    };

    class EventsDatabaseReader::Iterator {
    public:
        using value_type = rpc::Event;
        using difference_type = std::ptrdiff_t;
        using reference = const value_type &;
        using pointer = const value_type *;
        using iterator_category = std::forward_iterator_tag;

        explicit Iterator(EventsDatabaseReader &reader, bool end) noexcept;

        reference operator*() const;
        pointer operator->() const;

        Iterator &operator++();
        Iterator operator++(int);

        NON_DEFAULT_CONSTRUCTABLE(Iterator)

        friend bool operator==(const Iterator &lhs, const Iterator &rhs);
        friend bool operator!=(const Iterator &lhs, const Iterator &rhs);

    private:
        EventsDatabaseReader &reader_;
        EventPtr current;
    };
}
