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

#include "intercept/EventsDatabase.h"

#include <google/protobuf/util/json_util.h>
#include <sqlite3.h>
#include <spdlog/spdlog.h>

#include <memory>
#include <utility>

namespace {

    rust::Result<std::string> to_string(const rpc::Event &event) {
        std::string value;
        auto rc = google::protobuf::util::MessageToJsonString(event, &value);
        if (rc.ok()) {
            return rust::Ok(std::move(value));
        } else {
            return rust::Err(std::runtime_error(rc.ToString()));
        }
    }

    rust::Result<std::shared_ptr<rpc::Event>> from_string(const std::string &value) {
        auto event = std::make_shared<rpc::Event>();
        auto input = google::protobuf::StringPiece(value);
        auto rc = google::protobuf::util::JsonStringToMessage(input, &(*event));
        if (rc.ok()) {
            return rust::Ok(std::move(event));
        } else {
            return rust::Err(std::runtime_error(rc.ToString()));
        }
    }

    std::runtime_error create_error(const char *message, sqlite3 *handle) {
        return std::runtime_error(fmt::format("{}: {}", message, sqlite3_errmsg(handle)));
    }

    rust::Result<sqlite3 *> open(const fs::path &file) {
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

    rust::Result<sqlite3 *> execute_statement(sqlite3 *handle, const char *sql) {
        char *error_message = nullptr;
        if (auto rc = sqlite3_exec(handle, sql, nullptr, nullptr, &error_message); rc != SQLITE_OK) {
            auto error = std::runtime_error(fmt::format("Execute statement failed: {}", error_message));
            sqlite3_free(error_message);
            return rust::Err(error);
        }
        return rust::Ok(handle);
    }

    rust::Result<sqlite3_stmt *> create_prepared_statement(sqlite3 *handle, const char *sql) {
        sqlite3_stmt *stmt;
        if (auto rc = sqlite3_prepare_v2(handle, sql, -1, &stmt, nullptr); rc != SQLITE_OK) {
            return rust::Err(create_error("Creating prepared statement failed", handle));
        }
        return rust::Ok(stmt);
    }

    rust::Result<std::tuple<std::uint64_t, bool>> select_events_ids(sqlite3 *handle, sqlite3_stmt *stmt) {
        auto rc = sqlite3_step(stmt);
        if (rc == SQLITE_ROW) {
            std::uint64_t id = sqlite3_column_int64(stmt, 0);
            return rust::Ok(std::make_tuple(id, false));
        }
        if (rc == SQLITE_DONE) {
            constexpr std::uint64_t id = 0;
            return rust::Ok(std::make_tuple(id, true));
        }
        return rust::Err(create_error("Prepared statement execution failed", handle));
    }

    rust::Result<ic::EventPtrs> select_events(sqlite3 *handle, sqlite3_stmt *stmt, std::uint64_t id) {
        ic::EventPtrs results;

        if (auto rc = sqlite3_bind_int64(stmt, 1, id); rc != SQLITE_OK) {
            return rust::Err(create_error("Prepared statement binding (1) failed", handle));
        }
        {
            auto rc = sqlite3_step(stmt);
            for (; rc == SQLITE_ROW; rc = sqlite3_step(stmt)) {
                auto value = (const char *) sqlite3_column_text(stmt, 0);
                auto event = from_string(std::string(value));
                if (event.is_err()) {
                    return rust::Err(event.unwrap_err());
                }
                results.push_back(event.unwrap());
            }
            if (rc != SQLITE_DONE) {
                return rust::Err(create_error("Prepared statement step failed", handle));
            }
        }
        if (auto rc = sqlite3_clear_bindings(stmt); rc != SQLITE_OK) {
            return rust::Err(create_error("Prepared statement clear bindings failed", handle));
        }
        if (auto rc = sqlite3_reset(stmt); rc != SQLITE_OK) {
            return rust::Err(create_error("Prepared statement reset failed", handle));
        }
        return rust::Ok(results);
    }
}

namespace ic {

    EventsDatabase::EventsDatabase(sqlite3 *handle,
                                   sqlite3_stmt *insert,
                                   sqlite3_stmt *select_events,
                                   sqlite3_stmt *select_events_per_run) noexcept
            : handle_(handle)
            , insert_event_(insert)
            , select_events_(select_events)
            , select_events_per_run_(select_events_per_run)
    { }

    EventsDatabase::~EventsDatabase() noexcept {
        if (auto rc = sqlite3_finalize(select_events_per_run_); rc != SQLITE_OK) {
            auto error = create_error("Finalize prepared statement failed", handle_);
            spdlog::warn(error.what());
        }
        if (auto rc = sqlite3_finalize(select_events_); rc != SQLITE_OK) {
            auto error = create_error("Finalize prepared statement failed", handle_);
            spdlog::warn(error.what());
        }
        if (auto rc = sqlite3_finalize(insert_event_); rc != SQLITE_OK) {
            auto error = create_error("Finalize prepared statement failed", handle_);
            spdlog::warn(error.what());
        }
        if (auto rc = sqlite3_close(handle_); rc != SQLITE_OK) {
            auto error = create_error("Closing database failed", handle_);
            spdlog::warn(error.what());
        }
    }

    rust::Result<EventsDatabase::Ptr> EventsDatabase::open(const fs::path &file) {
        return EventsDatabase::open(file, false);
    }

    rust::Result<EventsDatabase::Ptr> EventsDatabase::create(const fs::path &file) {
        return EventsDatabase::open(file, true);
    }

    rust::Result<EventsDatabase::Ptr> EventsDatabase::open(const fs::path &file, bool create) {
        auto handle = create
                      ? ::open(file)
                              .and_then<sqlite3 *>([](auto handle) {
                                  constexpr const char *sql =
                                          "DROP TABLE IF EXISTS events;"
                                          "CREATE TABLE events ("
                                          "  event_id INTEGER PRIMARY KEY,"
                                          "  reporter_id INTEGER NOT NULL,"
                                          "  timestamp TEXT NOT NULL,"
                                          "  value TEXT NOT NULL"
                                          ");";

                                  return execute_statement(handle, sql);
                              })
                      : ::open(file);

        auto insert_event = handle
                .and_then<sqlite3_stmt *>([](auto handle) {
                    constexpr const char *sql =
                            "INSERT INTO events (reporter_id, timestamp, value) VALUES(?, ?, ?);";
                    return create_prepared_statement(handle, sql);
                });

        auto select_events = handle
                .and_then<sqlite3_stmt *>([](auto handle) {
                    constexpr const char *sql =
                            "SELECT DISTINCT reporter_id FROM events;";
                    return create_prepared_statement(handle, sql);
                });

        auto select_events_per_run = handle
                .and_then<sqlite3_stmt *>([](auto handle) {
                    constexpr const char *sql =
                            "SELECT value FROM events WHERE reporter_id = ? ORDER BY timestamp;";
                    return create_prepared_statement(handle, sql);
                });

        return rust::merge(handle, insert_event, select_events, select_events_per_run)
                .map<EventsDatabase::Ptr>([](auto tuple) {
                    const auto& [handle, stmt1, stmt2, stmt3] = tuple;
                    return std::make_shared<EventsDatabase>(handle, stmt1, stmt2, stmt3);
                });
    }

    rust::Result<int> EventsDatabase::insert_event(const rpc::Event &event) {
        return to_string(event)
            .and_then<int>([this, &event](auto value) -> rust::Result<int> {
                if (auto rc = sqlite3_bind_int64(insert_event_, 1, event.rid()); rc != SQLITE_OK) {
                    return rust::Err(create_error("Prepared statement binding (1) failed", handle_));
                }
                if (auto rc = sqlite3_bind_text(insert_event_, 2, event.timestamp().c_str(), -1, nullptr); rc != SQLITE_OK) {
                    return rust::Err(create_error("Prepared statement binding (2) failed", handle_));
                }
                if (auto rc = sqlite3_bind_text(insert_event_, 3, value.c_str(), -1, nullptr); rc != SQLITE_OK) {
                    return rust::Err(create_error("Prepared statement binding (3) failed", handle_));
                }
                if (auto rc = sqlite3_step(insert_event_); rc != SQLITE_DONE) {
                    return rust::Err(create_error("Prepared statement execution failed", handle_));
                }
                if (auto rc = sqlite3_clear_bindings(insert_event_); rc != SQLITE_OK) {
                    return rust::Err(create_error("Prepared statement clear bindings failed", handle_));
                }
                if (auto rc = sqlite3_reset(insert_event_); rc != SQLITE_OK) {
                    return rust::Err(create_error("Prepared statement reset failed", handle_));
                }
                return rust::Ok(0);
            });
    }

    EventsIterator EventsDatabase::events_by_process_begin() {
        if (auto rc = sqlite3_clear_bindings(select_events_per_run_); rc != SQLITE_OK) {
            auto error = create_error("Prepared statement clear bindings failed", handle_);
            spdlog::warn(error.what());
        }
        if (auto rc = sqlite3_reset(select_events_per_run_); rc != SQLITE_OK) {
            auto error = create_error("Prepared statement reset failed", handle_);
            spdlog::warn(error.what());
        }
        if (auto rc = sqlite3_reset(select_events_); rc != SQLITE_OK) {
            auto error = create_error("Prepared statement reset failed", handle_);
            spdlog::warn(error.what());
        }
        return next();
    }

    EventsIterator EventsDatabase::events_by_process_end() {
        return EventsIterator();
    }

    EventsIterator EventsDatabase::next() noexcept {
        auto tuple = select_events_ids(handle_, select_events_);
        if (tuple.is_err()) {
            return EventsIterator(this, rust::Err(tuple.unwrap_err()));
        } else {
            const auto&[id, end] = tuple.unwrap();
            if (end) {
                return EventsIterator();
            } else {
                auto result = select_events(handle_, select_events_per_run_, id);
                return EventsIterator(this, result);
            }
        }
    }

    EventsIterator::EventsIterator() noexcept
            : source_(nullptr)
            , value_(rust::Err(std::runtime_error("end")))
    { }

    EventsIterator::EventsIterator(EventsDatabase *source, rust::Result<EventPtrs> value) noexcept
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
        return (this == &other) || ((source_ == other.source_) && (value_ == other.value_));
    }

    bool EventsIterator::operator!=(const EventsIterator &other) const {
        return !(this->operator==(other));
    }
}
