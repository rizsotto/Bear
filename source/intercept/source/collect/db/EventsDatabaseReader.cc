/*  Copyright (C) 2012-2022 by László Nagy
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

#include "EventsDatabaseReader.h"
#include "libsys/Errors.h"

#include <google/protobuf/util/json_util.h>
#include <fmt/format.h>

#include <iostream>
#include <fstream>
#include <utility>

using google::protobuf::util::JsonParseOptions;

namespace {
    const JsonParseOptions parse_options;
}

namespace ic::collect::db {

    rust::Result<EventsDatabaseReader::Ptr> EventsDatabaseReader::from(const fs::path &path) {
        std::error_code error_code;
        if (!fs::exists(path, error_code)) {
            return rust::Err(std::runtime_error(fmt::format("Cannot find input file: {}", path.c_str())));
        }
        std::unique_ptr<std::istream> file =
                std::make_unique<std::fstream>(path, std::ios::in);
        std::shared_ptr<EventsDatabaseReader> result =
                std::make_shared<EventsDatabaseReader>(path, std::move(file));
        return rust::Ok(result);
    }

    EventsDatabaseReader::EventsDatabaseReader(fs::path path, StreamPtr file) noexcept
            : path_(std::move(path))
            , file_(std::move(file))
    { }

    EventsDatabaseReader::Iterator EventsDatabaseReader::begin() noexcept {
        return EventsDatabaseReader::Iterator(*this, false);
    }

    EventsDatabaseReader::Iterator EventsDatabaseReader::end() noexcept {
        return EventsDatabaseReader::Iterator(*this, true);
    }

    std::optional<rust::Result<EventPtr>> EventsDatabaseReader::next() noexcept {
        const auto line = next_line();
        if (line.has_value()) {
            return line.value()
                    .and_then<EventPtr>([this](const auto &line) {
                        return from_json(line);
                    });
        }
        return {};
    }

    std::optional<rust::Result<std::string>> EventsDatabaseReader::next_line() noexcept {
        std::string line;
        if (std::getline(*file_, line)) {
            return line.empty()
                    ? std::optional<rust::Result<std::string>>()
                    : std::make_optional(rust::Ok(std::move(line)));
        } else {
            const std::runtime_error error(
                    fmt::format(
                            "Events db read failed (from file {}): io error",
                            path_.string()));
            return file_->eof()
                   ? std::optional<rust::Result<std::string>>()
                   : std::make_optional(rust::Err(error));
        }
    }

    rust::Result<EventPtr> EventsDatabaseReader::from_json(const std::string &line) noexcept {
        std::shared_ptr<rpc::Event> event = std::make_shared<rpc::Event>();
        if (const auto status = google::protobuf::util::JsonStringToMessage(line, event.get(), parse_options); !status.ok()) {
            auto message = fmt::format(
                    "Events db read failed (from file {}): JSON parsing error",
                    path_.string()
            );
            return rust::Err(std::runtime_error(message));
        }
        return rust::Ok(event);
    }

    EventsDatabaseReader::Iterator::Iterator(EventsDatabaseReader &reader, bool end) noexcept
            : reader_(reader)
    {
        if (!end) {
            auto candidate = reader_.next();
            while (candidate) {
                if (candidate.value().is_ok()) {
                    current = candidate.value().unwrap();
                    break;
                }
                candidate = reader_.next();
            }
        }
    }

    const EventsDatabaseReader::Iterator::value_type &EventsDatabaseReader::Iterator::operator*() const {
        return current.operator*();
    }

    EventsDatabaseReader::Iterator::pointer EventsDatabaseReader::Iterator::operator->() const {
        return current.operator->();
    }

    EventsDatabaseReader::Iterator &EventsDatabaseReader::Iterator::operator++() {
        if (current) {
            current = nullptr;
            auto candidate = reader_.next();
            while (candidate) {
                if (candidate.value().is_ok()) {
                    current = candidate.value().unwrap();
                    break;
                }
                candidate = reader_.next();
            }
        }
        return *this;
    }

    EventsDatabaseReader::Iterator EventsDatabaseReader::Iterator::operator++(int) {
        auto result(*this);
        this->operator++();
        return result;
    }

    bool operator==(const EventsDatabaseReader::Iterator &lhs, const EventsDatabaseReader::Iterator &rhs) {
        return (&lhs.reader_ == &rhs.reader_) && (lhs.current == rhs.current);
    }

    bool operator!=(const EventsDatabaseReader::Iterator &lhs, const EventsDatabaseReader::Iterator &rhs) {
        return !(lhs == rhs);
    }
}
