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

#include "Database.h"

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

    rust::Result<std::unique_ptr<rpc::Event>> from_string(const std::string &value) {
        auto event = std::make_unique<rpc::Event>();
        auto input = google::protobuf::StringPiece(value);
        auto rc = google::protobuf::util::JsonStringToMessage(input, &(*event));
        if (rc.ok()) {
            return rust::Ok(std::move(event));
        } else {
            return rust::Err(std::runtime_error(rc.ToString()));
        }
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

    rust::Result<sqlite3 *> create_tables(sqlite3 *handle) {
        constexpr const char *sql =
                "CREATE TABLE events ("
                "  event_id INTEGER PRIMARY KEY,"
                "  reporter_id INTEGER NOT NULL,"
                "  timestamp TEXT NOT NULL,"
                "  value TEXT NOT NULL"
                ");";

        char *error_message = nullptr;
        if (auto rc = sqlite3_exec(handle, sql, nullptr, nullptr, &error_message); rc != SQLITE_OK) {
            auto error = std::runtime_error(fmt::format("Create table failed: {}", error_message));
            sqlite3_free(error_message);
            return rust::Err(error);
        }
        return rust::Ok(handle);
    }

    rust::Result<sqlite3_stmt *> create_insert_statement(sqlite3 *handle) {
        constexpr const char *sql = "INSERT INTO events (reporter_id, timestamp, value) VALUES(?, ?, ?);";

        sqlite3_stmt *stmt;
        if (auto rc = sqlite3_prepare_v2(handle, sql, -1, &stmt, nullptr); rc != SQLITE_OK) {
            return rust::Err(
                    std::runtime_error(
                            fmt::format("Creating prepared statement failed: {}",
                                        sqlite3_errmsg(handle))));
        }
        return rust::Ok(stmt);
    }
}

namespace ic {

    DatabaseWriter::DatabaseWriter(sqlite3 *handle, sqlite3_stmt *insert) noexcept
            : handle_(handle)
            , insert_(insert)
    { }

    DatabaseWriter::~DatabaseWriter() noexcept {
        if (auto rc = sqlite3_finalize(insert_); rc != SQLITE_OK) {
            auto error = create_error("Finalize prepared statement failed");
            spdlog::warn(error.what());
        }
        if (auto rc = sqlite3_close(handle_); rc != SQLITE_OK) {
            auto error = create_error("Closing database failed");
            spdlog::warn(error.what());
        }
    }

    rust::Result<DatabaseWriter::Ptr> DatabaseWriter::create(const fs::path &file) {
        auto handle = open(file)
                .and_then<sqlite3 *>([](auto handle) {
                    return create_tables(handle);
                });
        auto stmt = handle
                .and_then<sqlite3_stmt *>([](auto handle) {
                    return create_insert_statement(handle);
                });

        return rust::merge(handle, stmt)
                .map<DatabaseWriter::Ptr>([](auto tuple) {
                    const auto& [handle, stmt] = tuple;
                    return std::make_shared<DatabaseWriter>(handle, stmt);
                });
    }

    rust::Result<int> DatabaseWriter::insert_event(const rpc::Event &event) {
        return to_string(event)
            .and_then<int>([this, &event](auto value) -> rust::Result<int> {
                if (auto rc = sqlite3_bind_int64(insert_, 1, event.rid()); rc != SQLITE_OK) {
                    return rust::Err(create_error("Prepared statement binding (1) failed"));
                }
                if (auto rc = sqlite3_bind_text(insert_, 2, event.timestamp().c_str(), -1, nullptr); rc != SQLITE_OK) {
                    return rust::Err(create_error("Prepared statement binding (2) failed"));
                }
                if (auto rc = sqlite3_bind_text(insert_, 3, value.c_str(), -1, nullptr); rc != SQLITE_OK) {
                    return rust::Err(create_error("Prepared statement binding (3) failed"));
                }
                if (auto rc = sqlite3_step(insert_); rc != SQLITE_DONE) {
                    return rust::Err(create_error("Prepared statement execution failed"));
                }
                if (auto rc = sqlite3_clear_bindings(insert_); rc != SQLITE_OK) {
                    return rust::Err(create_error("Prepared statement clear bindings failed"));
                }
                if (auto rc = sqlite3_reset(insert_); rc != SQLITE_OK) {
                    return rust::Err(create_error("Prepared statement reset failed"));
                }
                return rust::Ok(0);
            });
    }

    std::runtime_error DatabaseWriter::create_error(const char *message) {
        return std::runtime_error(
                fmt::format("{}: {}", message, sqlite3_errmsg(handle_)));
    }
}
