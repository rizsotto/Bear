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

#include "libconfig/Configuration.h"

#include <iomanip>
#include <fstream>

#include <fmt/format.h>
#include <nlohmann/json.hpp>

namespace config {

    // Forward declarations
    void from_json(const nlohmann::json &j, Citnames &rhs);
    void to_json(nlohmann::json &j, const Citnames &rhs);

    void from_json(const nlohmann::json &j, Configuration &rhs) {
        if (j.contains("citnames")) {
            j.at("citnames").get_to(rhs.citnames);
        }
    }

    void to_json(nlohmann::json &j, const Configuration &rhs) {
        j = nlohmann::json{
                {"citnames",  rhs.citnames},
        };
    }


    std::ostream &operator<<(std::ostream &os, const Configuration &value)
    {
        nlohmann::json payload;
        to_json(payload, value);
        os << payload;

        return os;
    }

    rust::Result<fs::path> get_config_path(rust::Result<std::string_view> config_path_arg) {
        std::error_code error_code;

        if (config_path_arg.is_ok()) {
            if (fs::exists(config_path_arg.unwrap(), error_code)) {
                return rust::Ok<fs::path>(config_path_arg.unwrap());
            }
            return rust::Err(std::runtime_error("Cannot open config file"));
        }

        auto cwd = fs::current_path(error_code);

        fs::path default_config = cwd / "bear-config.json";
        if (fs::exists(default_config, error_code)) {
            return rust::Ok(default_config);
        }

        fs::path hidden_config = cwd / ".bear-config.json";
        if (fs::exists(hidden_config, error_code)) {
            return rust::Ok(hidden_config);
        }

        // Return empty path if no config file was found
        return rust::Ok(fs::path());
    }

    rust::Result<Configuration> read_configuration(fs::path config_file) {
        try {
            std::ifstream is(config_file);
            nlohmann::json in;
            is >> in;
            Configuration result;
            ::config::from_json(in, result);
            return rust::Ok(result);
        } catch (const std::exception &error) {
            return rust::Err(std::runtime_error(error.what()));
        }
    }

    rust::Result<Configuration> Configuration::load_config(const flags::Arguments &args) {
        auto config_file = get_config_path(args.as_string(cmd::citnames::FLAG_CONFIG));

        return config_file
             .and_then<Configuration>([](const auto& config_file_path) -> rust::Result<Configuration> {
                if (config_file_path.empty()) {
                    return rust::Ok(Configuration{});
                }
                return read_configuration(config_file_path);
            });
    }
}
