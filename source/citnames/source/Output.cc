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

#include "Output.h"
#include "libshell/Command.h"

#include <algorithm>
#include <iomanip>
#include <fstream>
#include <set>
#include <utility>

#include <fmt/format.h>
#include <nlohmann/json.hpp>

namespace {

    struct Filter {
        virtual ~Filter() noexcept = default;
        virtual bool apply(const cs::Entry &) = 0;
    };

    struct ContentFilter : public Filter {
        explicit ContentFilter(cs::Content config)
                : config(std::move(config))
        { }

        bool apply(const cs::Entry &entry) override {
            const auto &file = entry.file;
            return exists(file) && to_include(file) && !to_exclude(file);
        }

    private:
        [[nodiscard]] inline bool exists(const fs::path &file) const {
            const auto &to_check = config.include_only_existing_source;
            return (!to_check) || (to_check && does_exist(file));
        }

        [[nodiscard]] inline bool to_include(const fs::path &file) const {
            const auto &include = config.paths_to_include;
            return include.empty() || does_contain(include, file);
        }

        [[nodiscard]] inline bool to_exclude(const fs::path &file) const {
            const auto &exclude = config.paths_to_exclude;
            return !exclude.empty() && does_contain(exclude, file);
        }

        [[nodiscard]] static bool does_exist(const fs::path &path) {
            std::error_code error_code;
            return fs::exists(path, error_code);
        }

        [[nodiscard]] static bool does_contain(const std::list<fs::path> &directories, const fs::path &file) {
            return std::any_of(directories.begin(), directories.end(), [&file](auto directory) {
                // check if the path elements (list of directory names) are the same.
                const auto [end, nothing] = std::mismatch(directory.begin(), directory.end(), file.begin());
                // the file is contained in the directory if all path elements are
                // in the file paths too.
                return (end == directory.end());
            });
        }

    private:
        const cs::Content config;
    };

    // Duplicate detection filter.
    //
    // Duplicate detection can be done two ways: first one is based on the `output` attribute,
    // second is based on all attributes. The benefit to use the `output` attribute only is,
    // that it faster and handles compiler flag changes better. The problem with it is, that
    // it not present in every entries.
    //
    // Current implementation is starts with the output one, but computes both hashes (and
    // maintains the hash sets). If an entry has no output attribute, then it switch to use
    // all attributes hashes.
    struct DuplicateFilter : public Filter {
        DuplicateFilter()
                : hashes()
        { }

        bool apply(const cs::Entry &entry) override {
            if (const auto h2 = hash(entry); hashes.find(h2) == hashes.end()) {
                hashes.insert(h2);
                return true;
            }
            return false;
        }

    private:
        // The hash function based on all attributes.
        //
        // - It shall ignore the compiler name, but count all compiler flags in.
        // - Same compiler call semantic is detected by filter out the irrelevant flags.
        static std::string hash(const cs::Entry &entry) {
            auto file = entry.file.string();
            std::reverse(file.begin(), file.end());

            const auto args = fmt::format(
                    "{}",
                    fmt::join(entry.arguments.rbegin(), std::prev(entry.arguments.rend()), ","));
            size_t args_hash = std::hash<std::string>{}(args);

            return fmt::format("{}:{}", args_hash, file);
        }

    private:
        std::set<std::string> hashes;
    };
}

namespace cs {

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
            throw std::runtime_error("Field 'command' or 'arguments' not found");
        }

        validate(entry);
    }

    bool operator==(const Entry &lhs, const Entry &rhs) {
        return (lhs.file == rhs.file)
               && (lhs.directory == rhs.directory)
               && (lhs.output == rhs.output)
               && (lhs.arguments == rhs.arguments);
    }

    std::ostream &operator<<(std::ostream &os, const Entry &entry) {
        const Format format;
        const nlohmann::json &json = to_json(entry, format);
        os << json;

        return os;
    }

    CompilationDatabase::CompilationDatabase(Format _format, Content _content)
            : format(_format)
            , content(std::move(_content))
    { }

    rust::Result<size_t> CompilationDatabase::to_json(const fs::path &file, const Entries &rhs) const {
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

    rust::Result<size_t> CompilationDatabase::to_json(std::ostream &ostream, const Entries &entries) const {
        try {
            ContentFilter content_filter(content);
            DuplicateFilter duplicate_filter;

            size_t count = 0;
            nlohmann::json json = nlohmann::json::array();
            for (const auto &entry : entries) {
                if (content_filter.apply(entry) && duplicate_filter.apply(entry)) {
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

    rust::Result<size_t> CompilationDatabase::from_json(const fs::path &file, std::list<cs::Entry> &entries) const {
        try {
            std::ifstream source(file);
            return from_json(source, entries)
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

    rust::Result<size_t> CompilationDatabase::from_json(std::istream &istream, std::list<cs::Entry> &entries) const {
        try {
            nlohmann::json in;
            istream >> in;

            for (const auto &e : in) {
                Entry entry;
                cs::from_json(e, entry);
                entries.emplace_back(entry);
            }

            return rust::Ok(in.size());
        } catch (const std::exception &error) {
            return rust::Err(std::runtime_error(error.what()));
        }
    }
}
