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

#include "libsys/Path.h"

namespace {

    TEST(path, split_produces_empty_list_for_empty_string)
    {
        const auto result = sys::path::split("");

        EXPECT_TRUE(result.empty());
    }

    TEST(path, split_produces_list_for_single_entry)
    {
        const auto result = sys::path::split("/path/to");

        const std::list<std::string> expected = { "/path/to" };
        EXPECT_EQ(expected, result);
    }

    TEST(path, split_produces_list_for_multiple_entries)
    {
        const auto result = sys::path::split("/path/to:/path/to/another");

        const std::list<std::string> expected = { "/path/to", "/path/to/another" };
        EXPECT_EQ(expected, result);
    }

    TEST(path, join_empty_list)
    {
        const std::list<std::string> input = {};

        const auto result = sys::path::join(input);

        EXPECT_TRUE(result.empty());
    }

    TEST(path, join_single_entry)
    {
        const std::list<std::string> input = { "/path/to" };

        const auto result = sys::path::join(input);

        const std::string expected = "/path/to";
        EXPECT_EQ(expected, result);
    }

    TEST(path, join_multiple_entries)
    {
        const std::list<std::string> input = { "/path/to", "/path/to/another" };

        const auto result = sys::path::join(input);

        const std::string expected = "/path/to:/path/to/another";
        EXPECT_EQ(expected, result);
    }

    TEST(path, basename)
    {
        EXPECT_EQ("cc", sys::path::basename("cc"));
        EXPECT_EQ("cc", sys::path::basename("./cc"));
        EXPECT_EQ("cc", sys::path::basename("/usr/bin/cc"));
    }

    TEST(path, concat)
    {
        EXPECT_EQ("/usr/bin/cc", sys::path::concat("/usr/bin", "cc"));
    }
}