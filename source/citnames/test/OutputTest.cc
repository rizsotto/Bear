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

#include "gtest/gtest.h"

#include "Output.h"

#include <iterator>

namespace {

    constexpr cs::Format AS_ARGUMENTS { true, false };
    constexpr cs::Format AS_COMMAND { false, false };
    constexpr cs::Format AS_ARGUMENTS_NO_OUTPUT { true, true };
    constexpr cs::Format AS_COMMAND_NO_OUTPUT { false, true };

    cs::Content NO_FILTER {
        false, {}, {}
    };

    void value_serialized_and_read_back(
            const cs::Entries& input,
            const cs::Entries& expected,
            cs::Format format)
    {
        cs::CompilationDatabase sut(format, NO_FILTER);
        std::stringstream buffer;

        auto serialized = sut.to_json(buffer, input);
        EXPECT_TRUE(serialized.is_ok());

        cs::Entries deserialized;
        auto count = sut.from_json(buffer, deserialized);
        EXPECT_TRUE(count.is_ok());
        count.on_success([&expected, &deserialized](auto result) {
            EXPECT_EQ(expected.size(), result);
            EXPECT_EQ(expected, deserialized);
        });
    }

    TEST(compilation_database, empty_value_serialized_and_read_back)
    {
        cs::Entries expected = {};

        value_serialized_and_read_back(expected, expected, AS_ARGUMENTS);
        value_serialized_and_read_back(expected, expected, AS_COMMAND);
    }

    TEST(compilation_database, same_entries_read_back)
    {
        cs::Entries expected = {
                { "entry_one.c", "/path/to", std::nullopt, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", std::nullopt, { "cc", "-c", "entry_two.c" } },
                { "entries.c", "/path/to", { "entries.o" }, { "cc", "-c", "-o", "entries.o", "entries.c" } },
        };

        value_serialized_and_read_back(expected, expected, AS_ARGUMENTS);
        value_serialized_and_read_back(expected, expected, AS_COMMAND);
    }

    TEST(compilation_database, entries_without_output_read_back)
    {
        cs::Entries input = {
                { "entry_one.c", "/path/to", std::nullopt, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", std::nullopt, { "cc", "-c", "entry_two.c" } },
                { "entries.c", "/path/to", { "entries.o" }, { "cc", "-c", "-o", "entries.o", "entries.c" } },
        };
        cs::Entries expected = {
                { "entry_one.c", "/path/to", std::nullopt, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", std::nullopt, { "cc", "-c", "entry_two.c" } },
                { "entries.c", "/path/to", std::nullopt, { "cc", "-c", "-o", "entries.o", "entries.c" } },
        };

        value_serialized_and_read_back(input, expected, AS_ARGUMENTS_NO_OUTPUT);
        value_serialized_and_read_back(input, expected, AS_COMMAND_NO_OUTPUT);
    }

    TEST(compilation_database, merged_entries_read_back)
    {
        cs::Entries input = {
                { "entry_one.c", "/path/to", std::nullopt, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", std::nullopt, { "cc", "-c", "entry_two.c" } },
                { "entry_one.c", "/path/to", std::nullopt, { "cc1", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", std::nullopt, { "cc1", "-c", "entry_two.c" } },
        };
        cs::Entries expected = {
                { "entry_one.c", "/path/to", std::nullopt, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", std::nullopt, { "cc", "-c", "entry_two.c" } },
        };

        value_serialized_and_read_back(input, expected, AS_ARGUMENTS);
        value_serialized_and_read_back(input, expected, AS_COMMAND);
        value_serialized_and_read_back(input, expected, AS_ARGUMENTS_NO_OUTPUT);
        value_serialized_and_read_back(input, expected, AS_COMMAND_NO_OUTPUT);
    }

    TEST(compilation_database, merged_with_output_read_back)
    {
        cs::Entries input = {
                { "entry_one.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { "entry_two.o" }, { "cc", "-c", "entry_two.c" } },
                { "entry_one.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c", "-flag" } },
                { "entry_two.c", "/path/to", { "entry_two.o" }, { "cc", "-c", "entry_two.c", "-flag" } },
        };
        cs::Entries expected = {
                { "entry_one.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { "entry_two.o" }, { "cc", "-c", "entry_two.c" } },
        };

        value_serialized_and_read_back(input, expected, AS_ARGUMENTS);
        value_serialized_and_read_back(input, expected, AS_COMMAND);
    }

    TEST(compilation_database, deserialize_fails_with_empty_stream)
    {
        cs::CompilationDatabase sut(AS_COMMAND, NO_FILTER);
        std::stringstream buffer;

        cs::Entries deserialized;
        auto count = sut.from_json(buffer, deserialized);
        EXPECT_FALSE(count.is_ok());
    }

    TEST(compilation_database, deserialize_fails_with_missing_fields)
    {
        cs::CompilationDatabase sut(AS_COMMAND, NO_FILTER);
        std::stringstream buffer;

        buffer << "[ { } ]";

        cs::Entries deserialized;
        auto count = sut.from_json(buffer, deserialized);
        EXPECT_FALSE(count.is_ok());
    }

    TEST(compilation_database, deserialize_fails_with_empty_fields)
    {
        cs::CompilationDatabase sut(AS_COMMAND, NO_FILTER);
        std::stringstream buffer;

        buffer << R"#([ { "file": "file.c", "directory": "", "command": "cc -c file.c" } ])#";

        cs::Entries deserialized;
        auto count = sut.from_json(buffer, deserialized);
        EXPECT_FALSE(count.is_ok());
    }
}
