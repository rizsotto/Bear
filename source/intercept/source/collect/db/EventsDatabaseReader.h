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

#include "config.h"
#include "libresult/Result.h"
#include "intercept.pb.h"

#include <iosfwd>
#include <filesystem>
#include <memory>
#include <optional>

namespace fs = std::filesystem;

namespace ic::collect::db {

    using EventPtr = std::shared_ptr<rpc::Event>;

    class EventsDatabaseReader {
    public:
        using Ptr = std::shared_ptr<EventsDatabaseReader>;
        using StreamPtr = std::unique_ptr<std::istream>;

        [[nodiscard]] static rust::Result<EventsDatabaseReader::Ptr> from(const fs::path &path);

        [[nodiscard]] std::optional<rust::Result<EventPtr>> next() noexcept;

    private:
        [[nodiscard]] std::optional<rust::Result<std::string>> next_line() noexcept;
        [[nodiscard]] rust::Result<EventPtr> from_json(const std::string &) noexcept;

    public:
        explicit EventsDatabaseReader(fs::path path, StreamPtr file) noexcept;

        NON_DEFAULT_CONSTRUCTABLE(EventsDatabaseReader)
        NON_COPYABLE_NOR_MOVABLE(EventsDatabaseReader)

    private:
        fs::path path_;
        StreamPtr file_;
    };
}
