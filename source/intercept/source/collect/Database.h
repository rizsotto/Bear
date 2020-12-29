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

    class DatabaseWriter {
    public:
        using Ptr = std::shared_ptr<DatabaseWriter>;

        [[nodiscard]] static rust::Result<DatabaseWriter::Ptr> create(const fs::path &file);

        [[nodiscard]] rust::Result<int> insert_event(const rpc::Event &event);

    private:
        [[nodiscard]] std::runtime_error create_error(const char*);

    public:
        DatabaseWriter(sqlite3 *handle, sqlite3_stmt *insert) noexcept;
        ~DatabaseWriter() noexcept;

        DatabaseWriter(const DatabaseWriter &) = delete;
        DatabaseWriter(DatabaseWriter &&) noexcept = delete;

        DatabaseWriter &operator=(const DatabaseWriter &) = delete;
        DatabaseWriter &operator=(DatabaseWriter &&) noexcept = delete;

    private:
        sqlite3 *handle_;
        sqlite3_stmt *insert_;
    };

//    class EventsIterator;
//
//    class DatabaseReader {
//    public:
//        using Ptr = std::shared_ptr<DatabaseReader>;
//
//        [[nodiscard]] static rust::Result<DatabaseReader::Ptr> open(const fs::path &file);
//
//        [[nodiscard]] EventsIterator events_by_process_begin();
//        [[nodiscard]] EventsIterator events_by_process_end();
//
//    private:
//        [[nodiscard]] std::runtime_error create_error(const char*);
//
//    public:
//        explicit DatabaseReader(sqlite3 *handle) noexcept;
//        ~DatabaseReader() noexcept;
//
//        DatabaseReader(const DatabaseReader &) = delete;
//        DatabaseReader(DatabaseReader &&) noexcept = delete;
//
//        DatabaseReader &operator=(const DatabaseReader &) = delete;
//        DatabaseReader &operator=(DatabaseReader &&) noexcept = delete;
//
//    private:
//        sqlite3 *handle_;
//    };
//
//    class EventsIterator {
//    public:
//        using difference_type = std::ptrdiff_t;
//        using iterator_category = std::input_iterator_tag;
//        using value_type = rust::Result<std::vector<rpc::Event>>;
//        using pointer = value_type const *;
//        using reference = value_type const &;
//
//    public:
//        EventsIterator();
//
//        reference operator*() const;
//
//        EventsIterator operator++(int);
//        EventsIterator &operator++();
//
//        bool operator==(const EventsIterator &other) const;
//        bool operator!=(const EventsIterator &other) const;
//
//    private:
//    };
}
