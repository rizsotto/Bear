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

#include "libsys/Environment.h"

namespace {

    TEST(environment, nullptr_to_empty_map)
    {
        auto result = sys::env::from(nullptr);

        EXPECT_TRUE(result.empty());
    }

    TEST(environment, non_nullptr_to_non_empty_map)
    {
        const char* envp[] = {
            "sky=blue",
            nullptr
        };
        auto result = sys::env::from(envp);

        EXPECT_FALSE(result.empty());
        EXPECT_EQ("blue", result["sky"]);
    }

    TEST(environment, missing_value_does_not_crash)
    {
        const char* envp[] = {
            "only_key",
            nullptr
        };
        auto result = sys::env::from(envp);

        EXPECT_FALSE(result.empty());
        EXPECT_EQ("", result["only_key"]);
    }

    TEST(environment, missing_value_with_assign_does_not_crash)
    {
        const char* envp[] = {
            "only_key=",
            nullptr
        };
        auto result = sys::env::from(envp);

        EXPECT_FALSE(result.empty());
        EXPECT_EQ("", result["only_key"]);
    }

    TEST(environment, empty_value_does_not_crash)
    {
        const char* envp[] = {
            "",
            nullptr
        };
        auto result = sys::env::from(envp);

        EXPECT_FALSE(result.empty());
        EXPECT_EQ("", result[""]);
    }

    TEST(environment, empty_value_with_assign_does_not_crash)
    {
        const char* envp[] = {
            "=",
            nullptr
        };
        auto result = sys::env::from(envp);

        EXPECT_FALSE(result.empty());
        EXPECT_EQ("", result[""]);
    }

    TEST(environment, empty_map_creates_empty_array)
    {
        const std::map<std::string, std::string> input = {};
        const sys::env::Guard sut(input);

        EXPECT_TRUE(sut.data() != nullptr);
        EXPECT_TRUE(sut.data()[0] == nullptr);
    }

    TEST(environment, non_empty_map_creates_array)
    {
        const std::map<std::string, std::string> input = {
            { "grass", "green" },
            { "sky", "blue" } };
        const sys::env::Guard sut(input);

        EXPECT_TRUE(sut.data() != nullptr);
        EXPECT_STREQ(sut.data()[0], "grass=green");
        EXPECT_STREQ(sut.data()[1], "sky=blue");
        EXPECT_TRUE(sut.data()[2] == nullptr);
    }
}