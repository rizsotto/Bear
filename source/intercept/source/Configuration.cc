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

#include "Configuration.h"

#include <fmt/format.h>
#include <nlohmann/json.hpp>

#ifdef FMT_NEEDS_OSTREAM_FORMATTER
#include <fmt/ostream.h>
template <> struct fmt::formatter<ic::Configuration> : ostream_formatter {};
#endif

namespace ic {
    void from_json(const nlohmann::json &j, Configuration &rhs) {
        if (j.contains("output_file")) {
            j.at("output_file").get_to(rhs.output_file);
        }
        if (j.contains("library")) {
            j.at("library").get_to(rhs.library);
        }
        if (j.contains("wrapper")) {
            j.at("wrapper").get_to(rhs.wrapper);
        }
        if (j.contains("wrapper_dir")) {
            j.at("wrapper_dir").get_to(rhs.wrapper_dir);
        }
        if (j.contains("command")) {
            j.at("command").get_to(rhs.command);
        }
        if (j.contains("use_preload")) {
            j.at("use_preload").get_to(rhs.use_preload);
        }
        if (j.contains("use_wrapper")) {
            j.at("use_wrapper").get_to(rhs.use_wrapper);
        }
        if (j.contains("verbose")) {
            j.at("verbose").get_to(rhs.verbose);
        }
    }

    void to_json(nlohmann::json &j, const Configuration &rhs) {
        j = nlohmann::json{
                {"output_file",  rhs.output_file},
                {"use_preload", rhs.use_preload},
                {"use_wrapper", rhs.use_wrapper},
                {"verbose", rhs.verbose},
        };
        if (!rhs.command.empty()) {
            j["command"] = rhs.command;
        }
    }

    std::ostream &operator<<(std::ostream &os, const Configuration &value) {
        nlohmann::json payload;
        to_json(payload, value);
        os << payload;

        return os;
    }

}
