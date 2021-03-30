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

#include "config.h"
#include "libmain/ApplicationFromArgs.h"
#include "libmain/main.h"
#include "libflags/Flags.h"
#include "collect/db/EventsDatabaseReader.h"

#include <google/protobuf/util/json_util.h>

#include <filesystem>
#include <iostream>
#include <fstream>

namespace fs = std::filesystem;

namespace {

    constexpr char APPEND[] = "append";
    constexpr char DUMP[] = "dump";

    struct AppendCommand : ps::Command {
        explicit AppendCommand(std::string_view input, std::string_view path)
                : ps::Command()
                , input(input)
                , path(path)
        { }

        [[nodiscard]] rust::Result<int> execute() const override {
            return rust::Err(std::runtime_error("Not implemented"));
        }

    private:
        std::string input;
        std::string path;
    };

    struct DumpCommand : ps::Command {
        explicit DumpCommand(std::string_view output, std::string_view path)
                : ps::Command()
                , output(output)
                , path(path)
        { }

        [[nodiscard]] rust::Result<int> execute() const override {
            std::ofstream os(output);
            return ic::collect::db::EventsDatabaseReader::open(path)
                    .and_then<int>([&os](auto db) {
                        unsigned int count = 0;
                        os << "[" << std::endl;
                        for (auto it = db->events_begin(); it != db->events_end(); ++it, ++count) {
                            auto result = (*it).template map<std::string>([](const auto &event) {
                                std::string json;
                                google::protobuf::util::MessageToJsonString(*event, &json);
                                return json;
                            });
                            if (result.is_ok()) {
                                if (count > 0) {
                                    os << "," << std::endl;
                                }
                                os << result.unwrap() << std::endl;
                            } else {
                                return rust::Result<int>(rust::Err(std::runtime_error("")));
                            }
                        }
                        os << "]";
                        return rust::Result<int>(rust::Ok(0));
                    });
        }

    private:
        fs::path output;
        fs::path path;
    };

    struct Application : ps::ApplicationFromArgs {

        Application() noexcept
                : ps::ApplicationFromArgs(ps::ApplicationLogConfig("events_db", "db"))
        { }

        rust::Result<flags::Arguments> parse(int argc, const char **argv) const override {
            const flags::Parser append(APPEND, {
                    {"--input", {1, false, "path of the input file", {"-"},        std::nullopt}},
                    {"--path",  {1, true,  "path of the db file",    std::nullopt, std::nullopt}},
            });
            const flags::Parser dump(DUMP, {
                    {"--output", {1, true, "path of the output file", std::nullopt, std::nullopt}},
                    {"--path",   {1, true, "path of the db file",     std::nullopt, std::nullopt}},
            });
            const flags::Parser parser("intercept", VERSION, {append, dump});

            return parser.parse_or_exit(argc, const_cast<const char **>(argv));
        }

        rust::Result<ps::CommandPtr> command(const flags::Arguments &args, const char **) const override {
            return args.as_string(flags::COMMAND)
                    .and_then<ps::CommandPtr>([&args](auto command) {
                        if (command == APPEND) {
                            return command_append(args);
                        }
                        if (command == DUMP) {
                            return command_dump(args);
                        }
                        return rust::Result<ps::CommandPtr>(rust::Err(std::runtime_error("")));
                    });
        }

        [[nodiscard]] static rust::Result<ps::CommandPtr> command_append(const flags::Arguments &args) {
            auto input = args.as_string("--input");
            auto path = args.as_string("--path");
            return rust::merge(input, path)
                    .map<ps::CommandPtr>([](auto tuple) {
                        const auto& [input, path] = tuple;
                        return std::make_unique<AppendCommand>(input, path);
                    });
        }

        [[nodiscard]] static rust::Result<ps::CommandPtr> command_dump(const flags::Arguments &args) {
            auto input = args.as_string("--output");
            auto path = args.as_string("--path");
            return rust::merge(input, path)
                    .map<ps::CommandPtr>([](auto tuple) {
                        const auto& [output, path] = tuple;
                        return std::make_unique<DumpCommand>(output, path);
                    });
        }
    };
}

int main(int argc, char* argv[], char* envp[])
{
    return ps::main<Application>(argc, argv, envp);
}
