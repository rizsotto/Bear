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

#include <google/protobuf/io/zero_copy_stream_impl.h>

#include <filesystem>
#include <memory>

namespace fs = std::filesystem;

namespace ic::collect::db {

    class EventsDatabaseWriter {
    public:
        using Ptr = std::shared_ptr<EventsDatabaseWriter>;
        using StreamPtr = std::unique_ptr<google::protobuf::io::FileOutputStream>;

        [[nodiscard]] static rust::Result<EventsDatabaseWriter::Ptr> create(const fs::path &file);

        [[nodiscard]] rust::Result<int> insert_event(const rpc::Event &event);

    private:
        [[nodiscard]] std::runtime_error error() noexcept;

    public:
        explicit EventsDatabaseWriter(fs::path file, StreamPtr stream) noexcept;
        ~EventsDatabaseWriter() noexcept;

        EventsDatabaseWriter(const EventsDatabaseWriter &) = delete;
        EventsDatabaseWriter(EventsDatabaseWriter &&) noexcept = delete;

        EventsDatabaseWriter &operator=(const EventsDatabaseWriter &) = delete;
        EventsDatabaseWriter &operator=(EventsDatabaseWriter &&) noexcept = delete;

    private:
        fs::path file_;
        StreamPtr stream_;
    };
}
