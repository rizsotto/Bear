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

#include "Output.h"
#include "libshell/Command.h"

#include <algorithm>
#include <iomanip>
#include <fstream>

#include <nlohmann/json.hpp>

namespace {

    using Filter = std::function<bool(const cs::Entry &)>;

    bool is_exists(const fs::path &path) {
        std::error_code error_code;
        return fs::exists(path, error_code);
    }

    bool contains(const std::list<fs::path> &root, const fs::path &file) {
        return std::any_of(root.begin(), root.end(), [&file](auto directory) {
            // check if the path elements (list of directory names) are the same.
            auto[end, nothing] = std::mismatch(directory.begin(), directory.end(), file.begin());
            // the file is contained in the directory if all path elements are
            // in the file paths too.
            return (end == directory.end());
        });
    }

    Filter make_filter(const cs::Content &config) {
        return [config](const auto &entry) -> bool {
            if (config.include_only_existing_source) {
                const auto exists = is_exists(entry.file);

                const auto &include = config.paths_to_include;
                const bool to_include = include.empty() || contains(include, entry.file);
                const auto &exclude = config.paths_to_exclude;
                const bool to_exclude = !exclude.empty() && contains(exclude, entry.file);

                return exists && to_include && !to_exclude;
            }
            // if no check required, accept every entry.
            return true;
        };
    }

    using Comparator = std::function<bool(const cs::Entry &lhs, const cs::Entry &rhs)>;

    bool compare_by_output(const cs::Entry &lhs, const cs::Entry &rhs) {
        // compare entries by the source and the output attribute only.
        return (lhs.file == rhs.file) && (lhs.output == rhs.output);
    }

    bool compare_by_all(const cs::Entry &lhs, const cs::Entry &rhs) {
        // compare entries by all possible attributes except the output field.
        return (lhs.file == rhs.file)
               && (lhs.directory == rhs.directory)
               && (std::equal(
                std::next(lhs.arguments.begin()),
                lhs.arguments.end(),
                std::next(rhs.arguments.begin())));
    }

    Comparator select_comparator(const cs::Entries &lhs, const cs::Entries &rhs) {
        // Select comparator based on the input values.
        const bool lhs_outputs = std::all_of(lhs.begin(), lhs.end(), [](auto entry) { return entry.output; });
        const bool rhs_outputs = std::all_of(rhs.begin(), rhs.end(), [](auto entry) { return entry.output; });
        // if all entries have the output field, it can compare by the output field.
        return (lhs_outputs && rhs_outputs) ? compare_by_output : compare_by_all;
    }
}

namespace cs {

    CompilationDatabase::CompilationDatabase(const Format &_format, const Content &_content)
            : format(_format), content(_content) {}

    nlohmann::json to_json(const Entry &rhs, const Format &format) {
        nlohmann::json json;

        json["file"] = rhs.file;
        json["directory"] = rhs.directory;
        if (!format.drop_output_field && rhs.output.has_value()) {
            json["output"] = rhs.output.value();
        }
        if (format.command_as_array) {
            json["arguments"] = rhs.arguments;
        } else {
            json["command"] = sh::join(rhs.arguments);
        }

        return json;
    }

    rust::Result<size_t> CompilationDatabase::to_json(const fs::path &file, const Entries &entries) const {
        try {
            std::ofstream target(file);
            return to_json(target, entries);
        } catch (const std::exception &error) {
            return rust::Err(std::runtime_error(error.what()));
        }
    }

    rust::Result<size_t> CompilationDatabase::to_json(std::ostream &ostream, const Entries &entries) const {
        try {
            size_t count = 0;
            auto filter = make_filter(content);
            nlohmann::json json = nlohmann::json::array();
            for (const auto &entry : entries) {
                if (std::invoke(filter, entry)) {
                    auto json_entry = cs::to_json(entry, format);
                    json.emplace_back(std::move(json_entry));
                    ++count;
                }
            }

            ostream << std::setw(2) << json << std::endl;

            return rust::Ok(count);
        } catch (const std::exception &error) {
            return rust::Err(std::runtime_error(error.what()));
        }
    }

    void validate(const Entry &entry) {
        if (entry.file.empty()) {
            throw std::runtime_error("Field 'file' is empty string.");
        }
        if (entry.directory.empty()) {
            throw std::runtime_error("Field 'directory' is empty string.");
        }
        if (entry.output.has_value() && entry.output.value().empty()) {
            throw std::runtime_error("Field 'output' is empty string.");
        }
        if (entry.arguments.empty()) {
            throw std::runtime_error("Field 'arguments' is empty list.");
        }
    }

    void from_json(const nlohmann::json &j, Entry &entry) {
        j.at("file").get_to(entry.file);
        j.at("directory").get_to(entry.directory);
        if (j.contains("output")) {
            std::string output;
            j.at("output").get_to(output);
            entry.output.emplace(output);
        }
        if (j.contains("arguments")) {
            std::list<std::string> arguments;
            j.at("arguments").get_to(arguments);
            entry.arguments.swap(arguments);
        } else if (j.contains("command")) {
            std::string command;
            j.at("command").get_to(command);

            sh::split(command)
                    .on_success([&entry](auto arguments) {
                        entry.arguments = arguments;
                    })
                    .on_error([](auto error) {
                        throw error;
                    });
        } else {
            throw nlohmann::json::out_of_range::create(403, "key 'command' or 'arguments' not found");
        }

        validate(entry);
    }

    void from_json(const nlohmann::json &array, Entries &entries) {
        for (const auto &e : array) {
            Entry entry;
            from_json(e, entry);
            entries.emplace_back(entry);
        }
    }

    rust::Result<Entries> CompilationDatabase::from_json(const fs::path &file) const {
        try {
            std::ifstream source(file);
            return from_json(source);
        } catch (const std::exception &error) {
            return rust::Err(std::runtime_error(error.what()));
        }
    }

    rust::Result<Entries> CompilationDatabase::from_json(std::istream &istream) const {
        try {
            nlohmann::json in;
            istream >> in;

            Entries result;
            cs::from_json(in, result);

            return rust::Ok(result);
        } catch (const std::exception &error) {
            return rust::Err(std::runtime_error(error.what()));
        }
    }

    Entries merge(const Entries &lhs, const Entries &rhs) {
        Entries result;
        // create a predicate which decides if the entry is already in the result.
        auto comparator = select_comparator(lhs, rhs);
        auto not_in_result = [&result, &comparator](const auto &it) {
            return std::none_of(result.begin(), result.end(),
                                [&comparator, &it](const auto &already_in) { return comparator(already_in, it); });
        };
        // apply the predicate.
        std::copy_if(lhs.begin(), lhs.end(), std::back_inserter(result), not_in_result);
        std::copy_if(rhs.begin(), rhs.end(), std::back_inserter(result), not_in_result);
        return result;
    }

    bool operator==(const Entry &lhs, const Entry &rhs) {
        return (lhs.file == rhs.file)
               && (lhs.directory == rhs.directory)
               && (lhs.output == rhs.output)
               && (lhs.arguments == rhs.arguments);
    }

    std::ostream &operator<<(std::ostream &os, const Entry &entry) {
        Format format;
        nlohmann::json json = to_json(entry, format);
        os << json;

        return os;
    }

    std::ostream &operator<<(std::ostream &os, const Entries &entries) {
        for (auto it = entries.begin(); it != entries.end(); ++it) {
            if (it != entries.begin()) {
                os << ", ";
            }
            os << *it;
        }
        return os;
    }
}
