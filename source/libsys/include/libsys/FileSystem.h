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

#pragma once

#include <memory>
#include <list>
#include <string>

#include "libresult/Result.h"

namespace sys {

    struct FileSystem {
        static constexpr char OS_SEPARATOR = '/';
        static constexpr char OS_PATH_SEPARATOR = ':';

        static std::list<std::string> split_path(const std::string& input);
        static std::string join_path(const std::list<std::string>& input);

    public:
        virtual ~FileSystem() = default;

        virtual rust::Result<std::string> get_cwd() const;

        virtual rust::Result<std::string> find_in_path(const std::string& name, const std::string& paths) const;

    protected:
        virtual int is_executable(const std::string& path) const;
        virtual rust::Result<std::string> real_path(const std::string& path) const;
    };
}
