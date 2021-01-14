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

#include "libresult/Result.h"
#include "intercept.pb.h"

#include <filesystem>
#include <memory>
#include <vector>

namespace fs = std::filesystem;

struct sqlite3;
struct sqlite3_stmt;

namespace ic {

    class EventsIterator;

    class EventsDatabase {
    public:
        using Ptr = std::shared_ptr<EventsDatabase>;

        [[nodiscard]] static rust::Result<EventsDatabase::Ptr> open(const fs::path &file);
        [[nodiscard]] static rust::Result<EventsDatabase::Ptr> create(const fs::path &file);

        [[nodiscard]] rust::Result<int> insert_event(const rpc::Event &event);

        [[nodiscard]] EventsIterator events_by_process_begin();
        [[nodiscard]] EventsIterator events_by_process_end();

    private:
        friend class EventsIterator;

        [[nodiscard]] static rust::Result<EventsDatabase::Ptr> open(const fs::path &file, bool create);

        [[nodiscard]] EventsIterator next() noexcept;

    public:
        EventsDatabase(sqlite3 *handle,
                       sqlite3_stmt *insert_event,
                       sqlite3_stmt *select_events,
                       sqlite3_stmt *select_events_per_run) noexcept;
        ~EventsDatabase() noexcept;

        EventsDatabase(const EventsDatabase &) = delete;
        EventsDatabase(EventsDatabase &&) noexcept = delete;

        EventsDatabase &operator=(const EventsDatabase &) = delete;
        EventsDatabase &operator=(EventsDatabase &&) noexcept = delete;

    private:
        sqlite3 *handle_;
        sqlite3_stmt *insert_event_;
        sqlite3_stmt *select_events_;
        sqlite3_stmt *select_events_per_run_;
    };

    using EventPtr = std::shared_ptr<rpc::Event>;
    using EventPtrs = std::vector<EventPtr>;

    class EventsIterator {
    public:
        using difference_type = std::ptrdiff_t;
        using iterator_category = std::input_iterator_tag;
        using value_type = rust::Result<std::vector<std::shared_ptr<rpc::Event>>>;
        using pointer = value_type const *;
        using reference = value_type const &;

    public:
        EventsIterator() noexcept;
        EventsIterator(EventsDatabase *source, rust::Result<EventPtrs> value) noexcept;

        reference operator*() const;

        EventsIterator operator++(int);
        EventsIterator &operator++();

        bool operator==(const EventsIterator &other) const;
        bool operator!=(const EventsIterator &other) const;

    private:
        EventsDatabase *source_;
        rust::Result<EventPtrs> value_;
    };
}
