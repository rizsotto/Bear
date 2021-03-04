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

    rust::Result<std::string> to_string(const rpc::Event &event) {
        std::string value;
        auto rc = google::protobuf::util::MessageToJsonString(event, &value);
        if (rc.ok()) {
            return rust::Ok(std::move(value));
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
}

namespace ic {

    EventsDatabase::EventsDatabase(sqlite3 *handle, sqlite3_stmt *insert) noexcept
            : handle_(handle)
            , insert_event_(insert)
    { }

    EventsDatabase::~EventsDatabase() noexcept {
        if (auto rc = sqlite3_finalize(insert_event_); rc != SQLITE_OK) {
            auto error = create_error("Finalize prepared statement failed", handle_);
            spdlog::warn(error.what());
        }
        if (auto rc = sqlite3_close(handle_); rc != SQLITE_OK) {
            auto error = create_error("Closing database failed", handle_);
            spdlog::warn(error.what());
        }
    }

    rust::Result<EventsDatabase::Ptr> EventsDatabase::create(const fs::path &file) {
        auto handle = open_sqlite(file)
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
                });

        auto insert_event = handle
                .and_then<sqlite3_stmt *>([](auto handle) {
                    constexpr const char *sql =
                            "INSERT INTO events (reporter_id, timestamp, value) VALUES(?, ?, ?);";
                    return create_prepared_statement(handle, sql);
                });

        return rust::merge(handle, insert_event)
                .map<EventsDatabase::Ptr>([](auto tuple) {
                    const auto& [handle, stmt1] = tuple;
                    return std::make_shared<EventsDatabase>(handle, stmt1);
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
}
