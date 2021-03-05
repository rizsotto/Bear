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

    cs::Content NO_FILTER {
        false, {}, {}
    };

    void simple_value_serialized_and_read_back(
            const cs::Entries& expected,
            const cs::Format& format)
    {
        cs::CompilationDatabase sut(format, NO_FILTER);
        std::stringstream buffer;

        auto serialized = sut.to_json(buffer, expected);
        EXPECT_TRUE(serialized.is_ok());

        auto deserialized = sut.from_json(buffer);
        EXPECT_TRUE(deserialized.is_ok());
        deserialized.on_success([&expected](auto result) {
            EXPECT_EQ(expected, result);
        });
    }

    TEST(compilation_database, empty_value_serialized_and_read_back)
    {
        cs::Entries expected = {};

        simple_value_serialized_and_read_back(expected, AS_ARGUMENTS);
        simple_value_serialized_and_read_back(expected, AS_COMMAND);
    }

    TEST(compilation_database, simple_value_serialized_and_read_back)
    {
        cs::Entries expected = {
                { "entry_one.c", "/path/to", { }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { }, { "cc", "-c", "entry_two.c" } },
                { "entries.c", "/path/to", { "entries.o" }, { "cc", "-c", "-o", "entries.o", "entries.c" } },
        };

        simple_value_serialized_and_read_back(expected, AS_ARGUMENTS);
        simple_value_serialized_and_read_back(expected, AS_COMMAND);
    }

    void value_serialized_and_read_back_without_output(
            const cs::Entries& input,
            const cs::Entries& expected,
            cs::Format format)
    {
        format.drop_output_field = true;

        cs::CompilationDatabase sut(format, NO_FILTER);
        std::stringstream buffer;

        auto serialized = sut.to_json(buffer, input);
        EXPECT_TRUE(serialized.is_ok());

        auto deserialized = sut.from_json(buffer);
        EXPECT_TRUE(deserialized.is_ok());
        deserialized.on_success([&expected](auto result) {
            EXPECT_EQ(expected, result);
        });
    }

    TEST(compilation_database, value_serialized_and_read_back_without_output)
    {
        cs::Entries input = {
                { "entry_one.c", "/path/to", { }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { }, { "cc", "-c", "entry_two.c" } },
                { "entries.c", "/path/to", { "entries.o" }, { "cc", "-c", "-o", "entries.o", "entries.c" } },
        };
        cs::Entries expected = {
                { "entry_one.c", "/path/to", { }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { }, { "cc", "-c", "entry_two.c" } },
                { "entries.c", "/path/to", { }, { "cc", "-c", "-o", "entries.o", "entries.c" } },
        };

        value_serialized_and_read_back_without_output(input, expected, AS_ARGUMENTS);
        value_serialized_and_read_back_without_output(input, expected, AS_COMMAND);
    }

    TEST(compilation_database, deserialize_fails_with_empty_stream)
    {
        cs::CompilationDatabase sut(AS_COMMAND, NO_FILTER);
        std::stringstream buffer;

        auto deserialized = sut.from_json(buffer);
        EXPECT_FALSE(deserialized.is_ok());
    }

    TEST(compilation_database, deserialize_fails_with_missing_fields)
    {
        cs::CompilationDatabase sut(AS_COMMAND, NO_FILTER);
        std::stringstream buffer;

        buffer << "[ { } ]";

        auto deserialized = sut.from_json(buffer);
        EXPECT_FALSE(deserialized.is_ok());
    }

    TEST(compilation_database, deserialize_fails_with_empty_fields)
    {
        cs::CompilationDatabase sut(AS_COMMAND, NO_FILTER);
        std::stringstream buffer;

        buffer << R"#([ { "file": "file.c", "directory": "", "command": "cc -c file.c" } ])#";

        auto deserialized = sut.from_json(buffer);
        EXPECT_FALSE(deserialized.is_ok());
    }

    TEST(compilation_database, merge)
    {
        cs::Entries input_one = {
                { "entry_one.c", "/path/to", std::nullopt, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", std::nullopt, { "cc", "-c", "entry_two.c" } },
        };
        cs::Entries input_one_exec = {
                { "entry_one.c", "/path/to", std::nullopt, { "cc1", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", std::nullopt, { "cc1", "-c", "entry_two.c" } },
        };
        cs::Entries input_two = {
                { "entries.c", "/path/to", { "entries.o" }, { "cc", "-c", "-o", "entries.o", "entries.c" } },
        };
        cs::Entries input_three = {
                *std::next(input_one.begin(), 0),
                *std::next(input_two.begin(), 0),
        };
        cs::Entries expected = {
                *std::next(input_one.begin(), 0),
                *std::next(input_one.begin(), 1),
                *std::next(input_two.begin(), 0),
        };

        EXPECT_EQ(input_one, cs::merge(input_one, input_one));
        EXPECT_EQ(input_one, cs::merge(input_one, input_one_exec));
        EXPECT_EQ(input_two, cs::merge(input_two, input_two));
        EXPECT_EQ(expected, cs::merge(input_one, input_two));
        EXPECT_EQ(expected, cs::merge(input_one, input_three));
    }

    TEST(compilation_database, merge_with_output)
    {
        cs::Entries input_one = {
                { "entry_one.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { "entry_two.o" }, { "cc", "-c", "entry_two.c" } },
        };
        cs::Entries input_two = {
                { "entry_one.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c", "-flag" } },
                { "entry_two.c", "/path/to", { "entry_two.o" }, { "cc", "-c", "entry_two.c", "-flag" } },
        };

        EXPECT_EQ(input_one, cs::merge(input_one, input_one));
        EXPECT_EQ(input_one, cs::merge(input_one, input_two));
        EXPECT_EQ(input_two, cs::merge(input_two, input_two));
        EXPECT_EQ(input_two, cs::merge(input_two, input_one));
    }
}
