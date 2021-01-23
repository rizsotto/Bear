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

        [[nodiscard]] static rust::Result<EventsDatabase::Ptr> create(const fs::path &file);

        [[nodiscard]] rust::Result<int> insert_event(const rpc::Event &event);

    public:
        EventsDatabase(sqlite3 *handle, sqlite3_stmt *insert_event) noexcept;
        ~EventsDatabase() noexcept;

        EventsDatabase(const EventsDatabase &) = delete;
        EventsDatabase(EventsDatabase &&) noexcept = delete;

        EventsDatabase &operator=(const EventsDatabase &) = delete;
        EventsDatabase &operator=(EventsDatabase &&) noexcept = delete;

    private:
        sqlite3 *handle_;
        sqlite3_stmt *insert_event_;
    };
}
