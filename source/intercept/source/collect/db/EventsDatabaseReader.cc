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

#include "EventsDatabaseReader.h"
#include "libsys/Errors.h"

#include <google/protobuf/util/delimited_message_util.h>
#include <fmt/format.h>

#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>

#include <utility>

namespace ic::collect::db {

    rust::Result<EventsDatabaseReader::Ptr> EventsDatabaseReader::from(const fs::path &file) {
        int fd = open(file.c_str(), O_RDONLY);
        if (fd == -1) {
            auto message = fmt::format("Events db open failed (file {}): {}", file.string(), sys::error_string(errno));
            return rust::Err(std::runtime_error(message));
        }
        std::unique_ptr<google::protobuf::io::FileInputStream> stream =
                std::make_unique<google::protobuf::io::FileInputStream>(fd, -1);
        std::shared_ptr<EventsDatabaseReader> result =
                std::make_shared<EventsDatabaseReader>(file, std::move(stream));
        return rust::Ok(result);
    }

    EventsDatabaseReader::EventsDatabaseReader(fs::path file, StreamPtr stream) noexcept
            : file_(std::move(file))
            , stream_(std::move(stream))
    { }

    EventsDatabaseReader::~EventsDatabaseReader() noexcept {
        stream_->Close();
    }

    EventsIterator EventsDatabaseReader::events_begin() {
        return next();
    }

    EventsIterator EventsDatabaseReader::events_end() {
        return EventsIterator();
    }

    EventsIterator EventsDatabaseReader::next() noexcept {
        std::shared_ptr<rpc::Event> event = std::make_shared<rpc::Event>();
        bool clean_eof;
        const bool success =
                google::protobuf::util::ParseDelimitedFromZeroCopyStream(event.get(), stream_.get(), &clean_eof);
        if (success && !clean_eof) {
            return EventsIterator(this, rust::Ok(event));
        } else if (clean_eof) {
            return EventsIterator();
        } else {
            return EventsIterator(this, rust::Err(error()));
        }
    }

    std::runtime_error EventsDatabaseReader::error() noexcept {
        int error_num = stream_->GetErrno();
        auto message = fmt::format("Events db read failed (from file {}): {}", file_.string(), sys::error_string(error_num));
        return std::runtime_error(message);
    }

    EventsIterator::EventsIterator() noexcept
            : source_(nullptr)
            , value_(rust::Err(std::runtime_error("end")))
    { }

    EventsIterator::EventsIterator(EventsDatabaseReader *source, rust::Result<EventPtr> value) noexcept
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
