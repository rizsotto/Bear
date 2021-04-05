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

#include "EventsDatabaseWriter.h"
#include "libsys/Errors.h"

#include <google/protobuf/util/delimited_message_util.h>
#include <fmt/format.h>

#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>

namespace ic::collect::db {

    rust::Result<EventsDatabaseWriter::Ptr> EventsDatabaseWriter::create(const fs::path &file) {
        int fd = open(file.c_str(), O_WRONLY | O_CREAT, 00644);
        if (fd == -1) {
            auto message = fmt::format("Events db open failed (file {}): {}", file.string(), sys::error_string(errno));
            return rust::Err(std::runtime_error(message));
        }
        std::unique_ptr<google::protobuf::io::FileOutputStream> stream =
                std::make_unique<google::protobuf::io::FileOutputStream>(fd, -1);
        std::shared_ptr<EventsDatabaseWriter> result =
                std::make_shared<EventsDatabaseWriter>(file, std::move(stream));
        return rust::Ok(result);
    }

    EventsDatabaseWriter::EventsDatabaseWriter(fs::path file, StreamPtr stream) noexcept
            : file_(std::move(file))
            , stream_(std::move(stream))
    { }

    EventsDatabaseWriter::~EventsDatabaseWriter() noexcept {
        stream_->Flush();
        stream_->Close();
    }

    rust::Result<int> EventsDatabaseWriter::insert_event(const rpc::Event &event) {
        return google::protobuf::util::SerializeDelimitedToZeroCopyStream(event, stream_.get())
               ? rust::Result<int>(rust::Ok(1))
               : rust::Result<int>(rust::Err(error()));
    }

    std::runtime_error EventsDatabaseWriter::error() noexcept {
        int error_num = stream_->GetErrno();
        auto message = fmt::format("Events db write failed (to file {}): {}", file_.string(), sys::error_string(error_num));
        return std::runtime_error(message);
    }
}
