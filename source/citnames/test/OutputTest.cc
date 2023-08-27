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

#include "gtest/gtest.h"

#include "Output.h"

#include <iterator>

namespace {

    constexpr cs::Format AS_ARGUMENTS { true, false };
    constexpr cs::Format AS_COMMAND { false, false };
    constexpr cs::Format AS_ARGUMENTS_NO_OUTPUT { true, true };
    constexpr cs::Format AS_COMMAND_NO_OUTPUT { false, true };

    cs::Content DEFAULT_CONTENT{};

    void value_serialized_and_read_back(
            const std::list<cs::Entry>& input,
            const std::list<cs::Entry>& expected,
            cs::Format format,
            cs::Content content = DEFAULT_CONTENT
            )
    {
        cs::CompilationDatabase sut(format, content);
        std::stringstream buffer;

        auto serialized = sut.to_json(buffer, input);
        EXPECT_TRUE(serialized.is_ok());

        std::list<cs::Entry> deserialized;
        auto count = sut.from_json(buffer, deserialized);
        EXPECT_TRUE(count.is_ok());
        count.on_success([&expected, &deserialized](auto result) {
            EXPECT_EQ(expected.size(), result);
            EXPECT_EQ(expected, deserialized);
        });
    }

    TEST(compilation_database, empty_value_serialized_and_read_back)
    {
        std::list<cs::Entry> expected = {};

        value_serialized_and_read_back(expected, expected, AS_ARGUMENTS);
        value_serialized_and_read_back(expected, expected, AS_COMMAND);
    }

    TEST(compilation_database, same_entries_read_back)
    {
        std::list<cs::Entry> expected = {
                { "entry_one.c", "/path/to", std::nullopt, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", std::nullopt, { "cc", "-c", "entry_two.c" } },
                { "entries.c", "/path/to", { "entries.o" }, { "cc", "-c", "-o", "entries.o", "entries.c" } },
        };

        value_serialized_and_read_back(expected, expected, AS_ARGUMENTS);
        value_serialized_and_read_back(expected, expected, AS_COMMAND);
    }

    TEST(compilation_database, entries_without_output_read_back)
    {
        std::list<cs::Entry> input = {
                { "entry_one.c", "/path/to", std::nullopt, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", std::nullopt, { "cc", "-c", "entry_two.c" } },
                { "entries.c", "/path/to", { "entries.o" }, { "cc", "-c", "-o", "entries.o", "entries.c" } },
        };
        std::list<cs::Entry> expected = {
                { "entry_one.c", "/path/to", std::nullopt, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", std::nullopt, { "cc", "-c", "entry_two.c" } },
                { "entries.c", "/path/to", std::nullopt, { "cc", "-c", "-o", "entries.o", "entries.c" } },
        };

        value_serialized_and_read_back(input, expected, AS_ARGUMENTS_NO_OUTPUT);
        value_serialized_and_read_back(input, expected, AS_COMMAND_NO_OUTPUT);
    }

    TEST(compilation_database, merged_entries_read_back)
    {
        std::list<cs::Entry> input = {
                { "entry_one.c", "/path/to", std::nullopt, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", std::nullopt, { "cc", "-c", "entry_two.c" } },
                { "entry_one.c", "/path/to", std::nullopt, { "cc1", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", std::nullopt, { "cc1", "-c", "entry_two.c" } },
        };
        std::list<cs::Entry> expected = {
                { "entry_one.c", "/path/to", std::nullopt, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", std::nullopt, { "cc", "-c", "entry_two.c" } },
        };

        value_serialized_and_read_back(input, expected, AS_ARGUMENTS);
        value_serialized_and_read_back(input, expected, AS_COMMAND);
        value_serialized_and_read_back(input, expected, AS_ARGUMENTS_NO_OUTPUT);
        value_serialized_and_read_back(input, expected, AS_COMMAND_NO_OUTPUT);
    }

    TEST(compilation_database, duplicate_entries_file_read_back)
    {
        std::list<cs::Entry> input = {
                { "entry_one.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { "entry_two.o" }, { "cc", "-c", "entry_two.c" } },
                { "entry_one.c", "/path/to/changed", { "entry_one2.o" }, { "cc1", "-c", "-o", "entry_one2.o", "entry_one.c" } },
                { "entry_two.c", "/path/to/changed", { "entry_two2.o" }, { "cc1", "-c", "-o", "entry_two2.o", "entry_two.c" } },
        };
        std::list<cs::Entry> expected = {
                { "entry_one.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { "entry_two.o" }, { "cc", "-c", "entry_two.c" } },
        };

        cs::Content content;
        content.duplicate_filter_fields = cs::DUPLICATE_FILE;
        value_serialized_and_read_back(input, expected, AS_ARGUMENTS, content);
    }

    TEST(compilation_database, duplicate_entries_file_output_read_back)
    {
        std::list<cs::Entry> input = {
                { "entry_one.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { "entry_two.o" }, { "cc", "-c", "entry_two.c" } },
                { "entry_one.c", "/path/to/changed", { "entry_one2.o" }, { "cc1", "-c", "-o", "entry_one2.o", "entry_one.c" } },
                { "entry_two.c", "/path/to/changed", { "entry_two2.o" }, { "cc1", "-c", "-o", "entry_two2.o", "entry_two.c" } },
                { "entry_one.c", "/path/to/changed", { "entry_one.o" }, { "cc1", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to/changed", { "entry_two.o" }, { "cc1", "-c", "entry_two.c" } },
        };
        std::list<cs::Entry> expected = {
                { "entry_one.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { "entry_two.o" }, { "cc", "-c", "entry_two.c" } },
                { "entry_one.c", "/path/to/changed", { "entry_one2.o" }, { "cc1", "-c", "-o", "entry_one2.o", "entry_one.c" } },
                { "entry_two.c", "/path/to/changed", { "entry_two2.o" }, { "cc1", "-c", "-o", "entry_two2.o", "entry_two.c" } },
        };

        cs::Content content;
        content.duplicate_filter_fields = cs::DUPLICATE_FILE_OUTPUT;
        value_serialized_and_read_back(input, expected, AS_ARGUMENTS, content);
    }

    TEST(compilation_database, duplicate_entries_all_read_back)
    {
        std::list<cs::Entry> input = {
                { "entry_one.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { "entry_two.o" }, { "cc", "-c", "entry_two.c" } },
                { "entry_three.c", "/path/to", { "entry_three.o" }, { "cc", "-c", "entry_three.c" } },

                // Filename changed
                { "entry_one.changed.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c" } },

                // Output changed
                { "entry_two.c", "/path/to", { "entry_two_changed.o" }, { "cc", "-c", "entry_two.c" } },

                // Flags changed
                { "entry_three.c", "/path/to", { "entry_three.o" }, { "cc", "-DCHANGED", "-c", "entry_three.c" } },

                { "entry_one.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { "entry_two.o" }, { "cc", "-c", "entry_two.c" } },
                { "entry_three.c", "/path/to", { "entry_three.o" }, { "cc", "-c", "entry_three.c" } },
        };
        std::list<cs::Entry> expected = {
                { "entry_one.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { "entry_two.o" }, { "cc", "-c", "entry_two.c" } },
                { "entry_three.c", "/path/to", { "entry_three.o" }, { "cc", "-c", "entry_three.c" } },
                { "entry_one.changed.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c" } },
                { "entry_two.c", "/path/to", { "entry_two_changed.o" }, { "cc", "-c", "entry_two.c" } },
                { "entry_three.c", "/path/to", { "entry_three.o" }, { "cc", "-DCHANGED", "-c", "entry_three.c" } },

        };

        cs::Content content;
        content.duplicate_filter_fields = cs::DUPLICATE_ALL;
        value_serialized_and_read_back(input, expected, AS_ARGUMENTS, content);
    }

    TEST(compilation_database, deserialize_fails_with_empty_stream)
    {
        cs::CompilationDatabase sut(AS_COMMAND, DEFAULT_CONTENT);
        std::stringstream buffer;

        std::list<cs::Entry> deserialized;
        auto count = sut.from_json(buffer, deserialized);
        EXPECT_FALSE(count.is_ok());
    }

    TEST(compilation_database, deserialize_fails_with_missing_fields)
    {
        cs::CompilationDatabase sut(AS_COMMAND, DEFAULT_CONTENT);
        std::stringstream buffer;

        buffer << "[ { } ]";

        std::list<cs::Entry> deserialized;
        auto count = sut.from_json(buffer, deserialized);
        EXPECT_FALSE(count.is_ok());
    }

    TEST(compilation_database, deserialize_fails_with_empty_fields)
    {
        cs::CompilationDatabase sut(AS_COMMAND, DEFAULT_CONTENT);
        std::stringstream buffer;

        buffer << R"#([ { "file": "file.c", "directory": "", "command": "cc -c file.c" } ])#";

        std::list<cs::Entry> deserialized;
        auto count = sut.from_json(buffer, deserialized);
        EXPECT_FALSE(count.is_ok());
    }

    TEST(compilation_database, include_filter_works_with_trailing_slash)
    {
        std::list<cs::Entry> input = {
            { "/home/user/project/build/source/entry_one.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c" } },
            { "/home/user/project/build/source/entry_two.c", "/path/to", { "entry_two.o" }, { "cc", "-c", "entry_two.c" } },
            { "/home/user/project/build/test/entry_one_test.c", "/path/to", { "entry_one_test.o" }, { "cc", "-c", "entry_one.c" } },
            { "/home/user/project/build/test/entry_two_test.c", "/path/to", { "entry_two_test.o" }, { "cc", "-c", "entry_two.c" } },
        };
        std::list<cs::Entry> expected = {
            { "/home/user/project/build/source/entry_one.c", "/path/to", { "entry_one.o" }, { "cc", "-c", "entry_one.c" } },
            { "/home/user/project/build/source/entry_two.c", "/path/to", { "entry_two.o" }, { "cc", "-c", "entry_two.c" } },
        };

        cs::Content content;
        content.paths_to_include = { fs::path("/home/user/project/build/source") };
        content.paths_to_exclude = { fs::path("/home/user/project/build/test") };
        value_serialized_and_read_back(input, expected, AS_ARGUMENTS, content);

        content.paths_to_include = { fs::path("/home/user/project/build/source/") };
        content.paths_to_exclude = { fs::path("/home/user/project/build/test/") };
        value_serialized_and_read_back(input, expected, AS_ARGUMENTS, content);
    }
}
