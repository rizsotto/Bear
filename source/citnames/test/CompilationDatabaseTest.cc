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

#include "gtest/gtest.h"

#include "CompilationDatabase.h"

#include <iterator>

namespace {

    void simple_value_serialized_and_read_back(
            const cs::output::CompilationDatabase& expected,
            const cs::cfg::Format& format)
    {
        std::stringstream buffer;

        auto serialized = cs::output::to_json(buffer, expected, format);
        EXPECT_TRUE(serialized.is_ok());

        auto deserialized = cs::output::from_json(buffer);
        EXPECT_TRUE(deserialized.is_ok());
        deserialized.on_success([&expected](auto result) {
            EXPECT_EQ(expected, result);
        });
    }

    TEST(compilation_database, empty_value_serialized_and_read_back)
    {
        cs::output::CompilationDatabase expected = {};

        simple_value_serialized_and_read_back(expected, (cs::cfg::Format) {true, false});
        simple_value_serialized_and_read_back(expected, (cs::cfg::Format) {false, false});
    }

    TEST(compilation_database, simple_value_serialized_and_read_back)
    {
        cs::output::CompilationDatabase expected = {
                { "entry_one.c", "/path/to", { }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { }, { "cc", "-c", "entry_two.c" } },
                { "entries.c", "/path/to", { "entries.o" }, { "cc", "-c", "-o", "entries.o", "entries.c" } },
        };

        simple_value_serialized_and_read_back(expected, (cs::cfg::Format) {true, false});
        simple_value_serialized_and_read_back(expected, (cs::cfg::Format) {false, false});
    }

    void value_serialized_and_read_back_without_output(
            const cs::output::CompilationDatabase& input,
            const cs::output::CompilationDatabase& expected,
            cs::cfg::Format format)
    {
        format.drop_output_field = true;

        std::stringstream buffer;

        auto serialized = cs::output::to_json(buffer, input, format);
        EXPECT_TRUE(serialized.is_ok());

        auto deserialized = cs::output::from_json(buffer);
        EXPECT_TRUE(deserialized.is_ok());
        deserialized.on_success([&expected](auto result) {
            EXPECT_EQ(expected, result);
        });
    }

    TEST(compilation_database, value_serialized_and_read_back_without_output)
    {
        cs::output::CompilationDatabase input = {
                { "entry_one.c", "/path/to", { }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { }, { "cc", "-c", "entry_two.c" } },
                { "entries.c", "/path/to", { "entries.o" }, { "cc", "-c", "-o", "entries.o", "entries.c" } },
        };
        cs::output::CompilationDatabase expected = {
                { "entry_one.c", "/path/to", { }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { }, { "cc", "-c", "entry_two.c" } },
                { "entries.c", "/path/to", { }, { "cc", "-c", "-o", "entries.o", "entries.c" } },
        };

        value_serialized_and_read_back_without_output(input, expected, (cs::cfg::Format) {true, false});
        value_serialized_and_read_back_without_output(input, expected, (cs::cfg::Format) {false, false});
    }

    TEST(compilation_database, deserialize_fails_with_empty_stream)
    {
        std::stringstream buffer;

        auto deserialized = cs::output::from_json(buffer);
        EXPECT_FALSE(deserialized.is_ok());
    }

    TEST(compilation_database, deserialize_fails_with_missing_fields)
    {
        std::stringstream buffer;

        buffer << "[ { } ]";

        auto deserialized = cs::output::from_json(buffer);
        EXPECT_FALSE(deserialized.is_ok());
    }

    TEST(compilation_database, deserialize_fails_with_empty_fields)
    {
        std::stringstream buffer;

        buffer << R"#([ { "file": "file.c", "directory": "", "command": "cc -c file.c" } ])#";

        auto deserialized = cs::output::from_json(buffer);
        EXPECT_FALSE(deserialized.is_ok());
    }

    TEST(compilation_database, merge)
    {
        cs::output::CompilationDatabase input_one = {
                { "entry_one.c", "/path/to", { }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { }, { "cc", "-c", "entry_two.c" } },
        };
        cs::output::CompilationDatabase input_two = {
                { "entries.c", "/path/to", { "entries.o" }, { "cc", "-c", "-o", "entries.o", "entries.c" } },
        };
        cs::output::CompilationDatabase input_three = {
                *std::next(input_one.begin(), 0),
                *std::next(input_two.begin(), 0),
        };
        cs::output::CompilationDatabase expected = {
                *std::next(input_one.begin(), 0),
                *std::next(input_one.begin(), 1),
                *std::next(input_two.begin(), 0),
        };

        EXPECT_EQ(input_one, cs::output::merge(input_one, input_one));
        EXPECT_EQ(input_two, cs::output::merge(input_two, input_two));
        EXPECT_EQ(expected, cs::output::merge(input_one, input_two));
        EXPECT_EQ(expected, cs::output::merge(input_one, input_three));
    }
}
