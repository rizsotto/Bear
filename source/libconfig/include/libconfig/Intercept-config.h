/*  Copyright (C) 2012-2023 by Samu698
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

#include <libresult/Result.h>
#include <libflags/Flags.h>

#include <filesystem>
#include <iosfwd>
#include <list>
#include <map>
#include <string>
#include <optional>
#include <utility>

namespace fs = std::filesystem;

namespace config {

    struct Intercept {
        fs::path output_file = cmd::intercept::DEFAULT_OUTPUT;
        fs::path library = cmd::library::DEFAULT_PATH;
        fs::path wrapper = cmd::wrapper::DEFAULT_PATH;
        fs::path wrapper_dir = cmd::wrapper::DEFAULT_DIR_PATH;
        std::list<std::string> command;
        bool use_preload = true;
        bool use_wrapper = true;
        bool verbose = false;

        std::optional<std::runtime_error> update(const flags::Arguments& args);
    };

    // Convenient methods for these types.
    std::ostream& operator<<(std::ostream&, const Intercept&);
}
