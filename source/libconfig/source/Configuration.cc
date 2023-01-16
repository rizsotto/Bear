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

namespace cs {

    void from_json(const nlohmann::json &j, Format &rhs) {
        j.at("command_as_array").get_to(rhs.command_as_array);
        j.at("drop_output_field").get_to(rhs.drop_output_field);
    }

    void to_json(nlohmann::json &j, const Format &rhs) {
        j = nlohmann::json{
                {"command_as_array",  rhs.command_as_array},
                {"drop_output_field", rhs.drop_output_field},
        };
    }

    void from_json(const nlohmann::json &j, Content &rhs) {
        j.at("include_only_existing_source").get_to(rhs.include_only_existing_source);

        if (j.contains("paths_to_include")) {
            j.at("paths_to_include").get_to(rhs.paths_to_include);
        }
        if (j.contains("paths_to_exclude")) {
            j.at("paths_to_exclude").get_to(rhs.paths_to_exclude);
        }
        if (j.contains("duplicate_filter_fields")) {
            j.at("duplicate_filter_fields").get_to(rhs.duplicate_filter_fields);
        }
    }

    void to_json(nlohmann::json &j, const Content &rhs) {
        j = nlohmann::json{
                {"include_only_existing_source", rhs.include_only_existing_source},
                {"duplicate_filter_fields", rhs.duplicate_filter_fields},
        };
        if (!rhs.paths_to_include.empty()) {
            j["paths_to_include"] = rhs.paths_to_include;
        }
        if (!rhs.paths_to_exclude.empty()) {
            j["paths_to_exclude"] = rhs.paths_to_exclude;
        }
    }

    void from_json(const nlohmann::json &j, Output &rhs) {
        j.at("format").get_to(rhs.format);
        j.at("content").get_to(rhs.content);
    }

    void to_json(nlohmann::json &j, const Output &rhs) {
        j = nlohmann::json{
                {"format",  rhs.format},
                {"content", rhs.content},
        };
    }

    void from_json(const nlohmann::json &j, CompilerWrapper &rhs) {
        j.at("executable").get_to(rhs.executable);

        if (j.contains("flags_to_add")) {
            j.at("flags_to_add").get_to(rhs.flags_to_add);
        }
        if (j.contains("flags_to_remove")) {
            j.at("flags_to_remove").get_to(rhs.flags_to_remove);
        }
    }

    void to_json(nlohmann::json &j, const CompilerWrapper &rhs) {
        j = nlohmann::json{
                {"executable",  rhs.executable},
        };
        if (!rhs.flags_to_add.empty()) {
            j["flags_to_add"] = rhs.flags_to_add;
        }
        if (!rhs.flags_to_remove.empty()) {
            j["flags_to_remove"] = rhs.flags_to_remove;
        }
    }

    void from_json(const nlohmann::json &j, Compilation &rhs) {
        if (j.contains("compilers_to_recognize")) {
            j.at("compilers_to_recognize").get_to(rhs.compilers_to_recognize);
        }
        if (j.contains("compilers_to_exclude")) {
            j.at("compilers_to_exclude").get_to(rhs.compilers_to_exclude);
        }
    }

    void to_json(nlohmann::json &j, const Compilation &rhs) {
        if (!rhs.compilers_to_recognize.empty()) {
            j["compilers_to_recognize"] = rhs.compilers_to_recognize;
        }
        if (!rhs.compilers_to_exclude.empty()) {
            j["compilers_to_exclude"] = rhs.compilers_to_exclude;
        }
    }

    void from_json(const nlohmann::json &j, Configuration &rhs) {
        if (j.contains("output")) {
            j.at("output").get_to(rhs.output);
        }
        if (j.contains("compilation")) {
            j.at("compilation").get_to(rhs.compilation);
        }
    }

    void to_json(nlohmann::json &j, const Configuration &rhs) {
        j = nlohmann::json{
                {"output",  rhs.output},
                {"compilation", rhs.compilation},
        };
    }

    std::ostream &operator<<(std::ostream &os, const Format &value)
    {
        nlohmann::json payload;
        to_json(payload, value);
        os << payload;

        return os;
    }

    std::ostream &operator<<(std::ostream &os, const Content &value)
    {
        nlohmann::json payload;
        to_json(payload, value);
        os << payload;

        return os;
    }

    std::ostream &operator<<(std::ostream &os, const Output &value)
    {
        nlohmann::json payload;
        to_json(payload, value);
        os << payload;

        return os;
    }

    std::ostream &operator<<(std::ostream &os, const CompilerWrapper &value)
    {
        nlohmann::json payload;
        to_json(payload, value);
        os << payload;

        return os;
    }

    std::ostream &operator<<(std::ostream &os, const Compilation &value)
    {
        nlohmann::json payload;
        to_json(payload, value);
        os << payload;

        return os;
    }

    std::ostream &operator<<(std::ostream &os, const Configuration &value)
    {
        nlohmann::json payload;
        to_json(payload, value);
        os << payload;

        return os;
    }

    rust::Result<size_t> ConfigurationSerializer::to_json(const fs::path &file, const Configuration &rhs) const
    {
        try {
            std::ofstream target(file);
            return to_json(target, rhs)
                    .map_err<std::runtime_error>([&file](auto error) {
                        return std::runtime_error(
                                fmt::format("Failed to write file: {}, cause: {}",
                                            file.string(),
                                            error.what()));
                    });
        } catch (const std::exception &error) {
            return rust::Err(std::runtime_error(
                    fmt::format("Failed to write file: {}, cause: {}",
                                file.string(),
                                error.what())));
        }
    }

    rust::Result<size_t> ConfigurationSerializer::to_json(std::ostream &os, const Configuration &rhs) const
    {
        try {
            nlohmann::json out = rhs;
            os << std::setw(4) << out << std::endl;

            return rust::Ok(size_t(1));
        } catch (const std::exception &error) {
            return rust::Err(std::runtime_error(error.what()));
        }
    }

    rust::Result<Configuration> ConfigurationSerializer::from_json(const fs::path &file) const
    {
        try {
            std::ifstream source(file);
            return from_json(source)
                    .map_err<std::runtime_error>([&file](auto error) {
                        return std::runtime_error(
                                fmt::format("Failed to read file: {}, cause: {}",
                                            file.string(),
                                            error.what()));
                    });
        } catch (const std::exception &error) {
            return rust::Err(std::runtime_error(
                    fmt::format("Failed to read file: {}, cause: {}",
                                file.string(),
                                error.what())));
        }
    }

    rust::Result<Configuration> ConfigurationSerializer::from_json(std::istream &is) const
    {
        try {
            nlohmann::json in;
            is >> in;

            Configuration result;
            ::cs::from_json(in, result);

            return rust::Ok(std::move(result));
        } catch (const std::exception &error) {
            return rust::Err(std::runtime_error(error.what()));
        }
    }
}
