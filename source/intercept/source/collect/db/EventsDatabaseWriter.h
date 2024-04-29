/*  Copyright (C) 2012-2024 by László Nagy
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

#include "config.h"
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

        [[nodiscard]] static rust::Result<EventsDatabaseWriter::Ptr> create(const fs::path &file);
        [[nodiscard]] rust::Result<int> insert_event(const rpc::Event &event);

    public:
        explicit EventsDatabaseWriter(fs::path path, int file) noexcept;
        ~EventsDatabaseWriter() noexcept;

        NON_DEFAULT_CONSTRUCTABLE(EventsDatabaseWriter)
        NON_COPYABLE_NOR_MOVABLE(EventsDatabaseWriter)

    private:
        rust::Result<std::string> to_json(const rpc::Event &event) noexcept;
        rust::Result<int> write_to_file(const std::string &content) noexcept;

    private:
        fs::path path_;
        int file_;
    };
}
