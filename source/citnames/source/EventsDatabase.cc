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

#include "EventsDatabase.h"

#include <google/protobuf/util/json_util.h>
#include <sqlite3.h>
#include <spdlog/spdlog.h>

#include <memory>
#include <utility>

namespace {

    rust::Result<cs::EventPtr> from_string(const char *value) {
        cs::EventPtr event = std::make_shared<rpc::Event>();
        auto rc = google::protobuf::util::JsonStringToMessage(value, &(*event));
        if (rc.ok()) {
            return rust::Ok(std::move(event));
        } else {
            return rust::Err(std::runtime_error(rc.ToString()));
        }
    }

    std::runtime_error create_error(const char *message, sqlite3 *handle) {
        return std::runtime_error(fmt::format("{}: {}", message, sqlite3_errmsg(handle)));
    }

    rust::Result<sqlite3 *> open_sqlite(const fs::path &file) {
        sqlite3 *handle;
        if (auto rc = sqlite3_open(file.c_str(), &handle); rc == SQLITE_OK) {
            return rust::Ok(handle);
        }
        return rust::Err(
                std::runtime_error(
                        fmt::format("Opening database {}, failed: {}",
                                    file.string(),
                                    sqlite3_errmsg(handle))));
    }

    rust::Result<sqlite3_stmt *> create_prepared_statement(sqlite3 *handle, const char *sql) {
        sqlite3_stmt *stmt;
        if (auto rc = sqlite3_prepare_v2(handle, sql, -1, &stmt, nullptr); rc != SQLITE_OK) {
            return rust::Err(create_error("Creating prepared statement failed", handle));
        }
        return rust::Ok(stmt);
    }
}

namespace cs {

    EventsDatabase::EventsDatabase(sqlite3 *handle, sqlite3_stmt *select_events) noexcept
            : handle_(handle)
            , select_events_(select_events)
    { }

    EventsDatabase::~EventsDatabase() noexcept {
        if (auto rc = sqlite3_finalize(select_events_); rc != SQLITE_OK) {
            auto error = create_error("Finalize prepared statement failed", handle_);
            spdlog::warn(error.what());
        }
        if (auto rc = sqlite3_close(handle_); rc != SQLITE_OK) {
            auto error = create_error("Closing database failed", handle_);
            spdlog::warn(error.what());
        }
    }

    rust::Result<EventsDatabase::Ptr> EventsDatabase::open(const fs::path &file) {
        auto handle = open_sqlite(file);

        auto select_events = handle
                .and_then<sqlite3_stmt *>([](auto handle) {
                    constexpr const char *sql =
                            "SELECT value FROM events ORDER BY timestamp;";
                    return create_prepared_statement(handle, sql);
                });

        return rust::merge(handle, select_events)
                .map<EventsDatabase::Ptr>([](auto tuple) {
                    const auto& [handle, stmt] = tuple;
                    return std::make_shared<EventsDatabase>(handle, stmt);
                });
    }

    EventsIterator EventsDatabase::events_begin() {
        if (auto rc = sqlite3_reset(select_events_); rc != SQLITE_OK) {
            auto error = create_error("Prepared statement reset failed", handle_);
            spdlog::warn(error.what());
        }
        return next();
    }

    EventsIterator EventsDatabase::events_end() {
        return EventsIterator();
    }

    EventsIterator EventsDatabase::next() noexcept {
        auto rc = sqlite3_step(select_events_);
        if (rc == SQLITE_ROW) {
            auto value = (const char *) sqlite3_column_text(select_events_, 0);
            return EventsIterator(this, from_string(value));
        }
        if (rc != SQLITE_DONE) {
            rust::Result<EventPtr> event =
                    rust::Err(create_error("Prepared statement step failed", handle_));
            return EventsIterator(this, event);
        }
        return EventsIterator();
    }

    EventsIterator::EventsIterator() noexcept
            : source_(nullptr)
            , value_(rust::Err(std::runtime_error("end")))
    { }

    EventsIterator::EventsIterator(EventsDatabase *source, rust::Result<EventPtr> value) noexcept
            : source_(source)
            , value_(std::move(value))
    { }

    const EventsIterator::value_type &EventsIterator::operator*() const {
        return value_;
    }

    EventsIterator EventsIterator::operator++(int) {
        return (source_ != nullptr) ? source_->next() : *this;
    }

    EventsIterator &EventsIterator::operator++() {
        if (source_ != nullptr) {
            *this = source_->next();
        }
        return *this;
    }

    bool EventsIterator::operator==(const EventsIterator &other) const {
        return (this == &other) || (source_ == other.source_);
    }

    bool EventsIterator::operator!=(const EventsIterator &other) const {
        return !(this->operator==(other));
    }
}
