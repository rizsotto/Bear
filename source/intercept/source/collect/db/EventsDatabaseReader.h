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
#include "libresult/Result.h"
#include "intercept.pb.h"

#include <google/protobuf/io/zero_copy_stream_impl.h>

#include <filesystem>
#include <memory>
#include <vector>

namespace fs = std::filesystem;

namespace ic::collect::db {

    class EventsDatabaseReader;
    using EventPtr = std::shared_ptr<rpc::Event>;

    class EventsIterator {
    public:
        using difference_type = std::ptrdiff_t;
        using iterator_category = std::input_iterator_tag;
        using value_type = rust::Result<EventPtr>;
        using pointer = value_type const *;
        using reference = value_type const &;

    public:
        EventsIterator() noexcept;
        EventsIterator(EventsDatabaseReader *source, rust::Result<EventPtr> value) noexcept;

        reference operator*() const;

        EventsIterator operator++(int);
        EventsIterator &operator++();

        bool operator==(const EventsIterator &other) const;
        bool operator!=(const EventsIterator &other) const;

    private:
        EventsDatabaseReader *source_;
        rust::Result<EventPtr> value_;
    };

    class EventsDatabaseReader {
    public:
        using Ptr = std::shared_ptr<EventsDatabaseReader>;
        using StreamPtr = std::unique_ptr<google::protobuf::io::FileInputStream>;

        [[nodiscard]] static rust::Result<EventsDatabaseReader::Ptr> from(const fs::path &file);

        [[nodiscard]] EventsIterator events_begin();
        [[nodiscard]] EventsIterator events_end();

    private:
        friend class EventsIterator;

        [[nodiscard]] EventsIterator next() noexcept;
        [[nodiscard]] std::runtime_error error() noexcept;

    public:
        explicit EventsDatabaseReader(fs::path file, StreamPtr stream) noexcept;
        ~EventsDatabaseReader() noexcept;

        NON_DEFAULT_CONSTRUCTABLE(EventsDatabaseReader);
        NON_COPYABLE_NOR_MOVABLE(EventsDatabaseReader);

    private:
        fs::path file_;
        StreamPtr stream_;
    };
}
