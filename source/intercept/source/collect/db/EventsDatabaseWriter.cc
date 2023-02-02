/*  Copyright (C) 2012-2023 by László Nagy
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

#include "EventsDatabaseWriter.h"
#include "libsys/Errors.h"

#include <google/protobuf/util/json_util.h>
#include <fmt/format.h>

#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#include <cerrno>

using google::protobuf::util::JsonPrintOptions;

namespace {

    JsonPrintOptions create_print_options() {
        JsonPrintOptions print_options;
        print_options.add_whitespace = false;
        print_options.always_print_primitive_fields = true;
        print_options.preserve_proto_field_names = true;
        print_options.always_print_enums_as_ints = false;
        return print_options;
    }

    const JsonPrintOptions print_options = create_print_options();
}

namespace ic::collect::db {

    rust::Result<EventsDatabaseWriter::Ptr> EventsDatabaseWriter::create(const fs::path &file) {
        int fd = open(file.c_str(), O_WRONLY | O_CREAT, 00644);
        if (fd == -1) {
            auto message = fmt::format("Events db open failed (file {}): {}", file.string(), sys::error_string(errno));
            return rust::Err(std::runtime_error(message));
        }
        std::shared_ptr<EventsDatabaseWriter> result =
                std::make_shared<EventsDatabaseWriter>(file, fd);
        return rust::Ok(result);
    }

    EventsDatabaseWriter::EventsDatabaseWriter(fs::path path, int file) noexcept
            : path_(std::move(path))
            , file_(file)
    { }

    EventsDatabaseWriter::~EventsDatabaseWriter() noexcept {
        close(file_);
    }

    rust::Result<int> EventsDatabaseWriter::insert_event(const rpc::Event &event) {
        return to_json(event)
                .and_then<int>([this](const auto &json) {
                    return write_to_file(json);
                })
                .and_then<int>([this](const auto &) {
                    return write_to_file("\n");
                });
    }

    rust::Result<std::string> EventsDatabaseWriter::to_json(const rpc::Event &event) noexcept {
        std::string json;
        if (const auto status = google::protobuf::util::MessageToJsonString(event, &json, print_options); !status.ok()) {
            auto message = fmt::format(
                    "Events db write failed (to file {}): JSON formatting error",
                    path_.string()
            );
            return rust::Err(std::runtime_error(message));
        }
        return rust::Ok(std::move(json));
    }

    rust::Result<int> EventsDatabaseWriter::write_to_file(const std::string &content) noexcept {
        if (-1 == write(file_, content.c_str(), content.size())) {
            auto message = fmt::format(
                    "Events db write failed (to file {}): {}",
                    path_.string(),
                    sys::error_string(errno)
            );
            errno = 0;
            return rust::Result<int>(rust::Err(std::runtime_error(message)));
        }
        return rust::Result<int>(rust::Ok(1));
    }
}
