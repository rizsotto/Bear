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

#include "Output.h"
#include "libshell/Command.h"

#include <algorithm>
#include <iomanip>
#include <fstream>
#include <memory>
#include <unordered_set>
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
                return (end == directory.end()) || (*end == "");
            });
        }

    private:
        const cs::Content config;
    };

    // Pure version of the boost::hash_combine function.
    static size_t hash_combine(size_t hash, size_t to_combine) {
        return hash ^ (to_combine + 0x9e3779b9 + (hash << 6) + (hash >> 2));
    }

    using DuplicateFilterPtr = std::unique_ptr<struct DuplicateFilter>;

    struct DuplicateFilter : public Filter {
        static DuplicateFilterPtr from_content(const cs::Content&);

        bool apply(const cs::Entry &entry) override {
            const auto h2 = hash(entry);
            auto [_, new_entry] = hashes.emplace(h2);
            return new_entry;
        }

    private:
        virtual size_t hash(const cs::Entry&) const = 0;

        std::unordered_set<size_t> hashes;
    };


    struct FileDuplicateFilter : public DuplicateFilter {
        private:
            size_t hash(const cs::Entry &entry) const override {
                auto string_hasher = std::hash<std::string>{};

                return string_hasher(entry.file);
            }
    };

    struct FileOutputDuplicateFilter : public DuplicateFilter {
        private:
            size_t hash(const cs::Entry &entry) const override {
                auto string_hasher = std::hash<std::string>{};

                auto hash = string_hasher(entry.file);

                if (entry.output) {
                    hash = hash_combine(hash, string_hasher(*entry.output));
                }

                return hash;
            }
    };

    struct StrictDuplicateFilter : public DuplicateFilter {
        private:
            size_t hash(const cs::Entry &entry) const override {
                auto string_hasher = std::hash<std::string>{};

                auto hash = string_hasher(entry.file);

                if (entry.output) {
                    hash = hash_combine(hash, string_hasher(*entry.output));
                }

                for (const auto& arg : entry.arguments) {
                    hash = hash_combine(hash, string_hasher(arg));
                }

                return hash;
            }
    };

    DuplicateFilterPtr DuplicateFilter::from_content(const cs::Content& content) {
        auto fields = content.duplicate_filter_fields;
        if (fields == cs::DUPLICATE_ALL) {
            return std::make_unique<StrictDuplicateFilter>();
        }
        if (fields == cs::DUPLICATE_FILE_OUTPUT) {
            return std::make_unique<FileOutputDuplicateFilter>();
        }
        if (fields == cs::DUPLICATE_FILE) {
            return std::make_unique<FileDuplicateFilter>();
        }

        // If the parameter is invalid use the default filter
        return std::make_unique<FileOutputDuplicateFilter>();
    }

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
            DuplicateFilterPtr duplicate_filter = DuplicateFilter::from_content(content);

            size_t count = 0;
            nlohmann::json json = nlohmann::json::array();
            for (const auto &entry : entries) {
                if (content_filter.apply(entry) && duplicate_filter->apply(entry)) {
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

    void to_json(nlohmann::json &j, const Entry &entry, const Format &format) {
        j = nlohmann::json{
                {"file",      entry.file},
                {"directory", entry.directory},
        };
        if (!format.drop_output_field && entry.output) {
            j["output"] = entry.output.value();
        }
        if (format.command_as_array) {
            j["arguments"] = entry.arguments;
        } else {
            j["command"] = sh::join(entry.arguments);
        }
    }

    nlohmann::json to_json(const LinkEntry &rhs, const Format &format) {
        nlohmann::json json;
        json["directory"] = rhs.directory;
        json["input_files"] = rhs.input_files;
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

    void to_json(nlohmann::json &j, const LinkEntry &entry, const Format &format) {
        j = nlohmann::json{
                {"directory", entry.directory},
                {"input_files", entry.input_files},
        };
        if (!format.drop_output_field && entry.output.has_value()) {
            j["output"] = entry.output.value();
        }
        if (format.command_as_array) {
            j["arguments"] = entry.arguments;
        } else {
            j["command"] = sh::join(entry.arguments);
        }
    }

    bool operator==(const LinkEntry &lhs, const LinkEntry &rhs) {
        return (lhs.directory == rhs.directory)
               && (lhs.output == rhs.output)
               && (lhs.arguments == rhs.arguments)
               && (lhs.input_files == rhs.input_files);
    }

    std::ostream &operator<<(std::ostream &os, const LinkEntry &entry) {
        const Format format;
        nlohmann::json j;
        to_json(j, entry, format);
        os << j;
        return os;
    }

    rust::Result<size_t> CompilationDatabase::to_link_json(const fs::path &file, const LinkEntries &rhs) const {
        try {
            std::ofstream target(file);
            return to_link_json(target, rhs)
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

    rust::Result<size_t> CompilationDatabase::to_link_json(std::ostream &ostream, const LinkEntries &entries) const {
        try {
            size_t count = 0;
            nlohmann::json json = nlohmann::json::array();
            for (const auto &entry : entries) {
                nlohmann::json j;
                cs::to_json(j, entry, format);
                json.emplace_back(std::move(j));
                ++count;
            }

            ostream << std::setw(2) << json << std::endl;

            return rust::Ok(count);
        } catch (const std::exception &error) {
            return rust::Err(std::runtime_error(error.what()));
        }
    }

    rust::Result<size_t> CompilationDatabase::from_link_json(const fs::path &file, LinkEntries &entries) const {
        try {
            std::ifstream source(file);
            return from_link_json(source, entries)
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

    rust::Result<size_t> CompilationDatabase::from_link_json(std::istream &istream, LinkEntries &entries) const {
        try {
            nlohmann::json in;
            istream >> in;

            for (const auto &e : in) {
                LinkEntry entry;
                cs::from_json(e, entry);
                entries.emplace_back(std::move(entry));
            }

            return rust::Ok(in.size());
        } catch (const std::exception &error) {
            return rust::Err(std::runtime_error(error.what()));
        }
    }

    void validate(const LinkEntry &entry) {
        if (entry.directory.empty()) {
            throw std::runtime_error("Field 'directory' is empty");
        }
        if (entry.arguments.empty()) {
            throw std::runtime_error("Field 'arguments' is empty");
        }
        if (entry.input_files.empty()) {
            throw std::runtime_error("Field 'input_files' is empty");
        }
        if (entry.output.has_value() && entry.output.value().empty()) {
            throw std::runtime_error("Field 'output' is empty string.");
        }
    }

    void from_json(const nlohmann::json &j, LinkEntry &entry) {
        j.at("directory").get_to(entry.directory);
        j.at("input_files").get_to(entry.input_files);
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

    void validate(const ArEntry &entry) {
        if (entry.directory.empty()) {
            throw std::runtime_error("Field 'directory' is empty");
        }
        if (entry.arguments.empty()) {
            throw std::runtime_error("Field 'arguments' is empty");
        }
        if (entry.input_files.empty()) {
            throw std::runtime_error("Field 'input_files' is empty");
        }
        if (entry.output.has_value() && entry.output.value().empty()) {
            throw std::runtime_error("Field 'output' is empty string.");
        }
        if (entry.operation.empty()) {
            throw std::runtime_error("Field 'operation' is empty");
        }
    }

    void from_json(const nlohmann::json &j, ArEntry &entry) {
        j.at("directory").get_to(entry.directory);
        j.at("input_files").get_to(entry.input_files);
        j.at("operation").get_to(entry.operation);
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

    bool operator==(const ArEntry &lhs, const ArEntry &rhs) {
        return (lhs.directory == rhs.directory)
               && (lhs.output == rhs.output)
               && (lhs.arguments == rhs.arguments)
               && (lhs.input_files == rhs.input_files)
               && (lhs.operation == rhs.operation);
    }

    std::ostream &operator<<(std::ostream &os, const ArEntry &entry) {
        const Format format;
        nlohmann::json j;
        to_json(j, entry, format);
        os << j;
        return os;
    }

    void to_json(nlohmann::json &j, const ArEntry &entry, const Format &format) {
        j = nlohmann::json{
                {"directory", entry.directory},
                {"input_files", entry.input_files},
                {"operation", entry.operation}
        };
        if (!format.drop_output_field && entry.output.has_value()) {
            j["output"] = entry.output.value();
        }
        if (format.command_as_array) {
            j["arguments"] = entry.arguments;
        } else {
            j["command"] = sh::join(entry.arguments);
        }
    }

    nlohmann::json to_json(const ArEntry &rhs, const Format &format) {
        nlohmann::json json;
        to_json(json, rhs, format);
        return json;
    }

    rust::Result<size_t> CompilationDatabase::to_ar_json(const fs::path &file, const ArEntries &entries) const {
        try {
            std::ofstream target(file);
            return to_ar_json(target, entries)
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

    rust::Result<size_t> CompilationDatabase::to_ar_json(std::ostream &ostream, const ArEntries &entries) const {
        try {
            size_t count = 0;
            nlohmann::json json = nlohmann::json::array();
            for (const auto &entry : entries) {
                nlohmann::json j;
                cs::to_json(j, entry, format);
                json.emplace_back(std::move(j));
                ++count;
            }

            ostream << std::setw(2) << json << std::endl;

            return rust::Ok(count);
        } catch (const std::exception &error) {
            return rust::Err(std::runtime_error(error.what()));
        }
    }

    rust::Result<size_t> CompilationDatabase::from_ar_json(const fs::path &file, ArEntries &entries) const {
        try {
            std::ifstream source(file);
            return from_ar_json(source, entries)
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

    rust::Result<size_t> CompilationDatabase::from_ar_json(std::istream &istream, ArEntries &entries) const {
        try {
            nlohmann::json in;
            istream >> in;

            for (const auto &e : in) {
                ArEntry entry;
                cs::from_json(e, entry);
                entries.emplace_back(std::move(entry));
            }

            return rust::Ok(in.size());
        } catch (const std::exception &error) {
            return rust::Err(std::runtime_error(error.what()));
        }
    }
}
