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

#include "report/libexec/Array.h"

namespace {

    TEST(array_end, dont_crash_on_nullptr)
    {
        const char** input = nullptr;

        EXPECT_EQ(nullptr, el::array::end(input));
    }

    TEST(array_end, dont_crash_on_empty)
    {
        const char* input[] = { nullptr };

        EXPECT_EQ(&input[0], el::array::end(input));
    }

    TEST(array_end, works_on_strings)
    {
        const char* input = "hello";

        EXPECT_EQ(input + 5, el::array::end(input));
    }

    TEST(array_end, finds_the_last_one)
    {
        const char* input0 = "this";
        const char* input1 = "that";
        const char* input[] = { input0, input1, 0 };

        EXPECT_EQ(&input[2], el::array::end(input));
    }

    TEST(array_length, dont_crash_on_nullptr)
    {
        const char** input = nullptr;

        EXPECT_EQ(0, el::array::length(input));
    }

    TEST(array_length, dont_crash_on_empty)
    {
        const char* input[] = { nullptr };

        EXPECT_EQ(0, el::array::length(input));
    }

    TEST(array_length, finds_the_last_one)
    {
        const char* input0 = "this";
        const char* input1 = "that";
        const char* input[] = { input0, input1, 0 };

        EXPECT_EQ(2, el::array::length(input));
    }

    TEST(array_length, works_on_strings)
    {
        const char* input = "hello";

        EXPECT_EQ(5, el::array::length(input));
    }

    TEST(array_copy, works_with_zero_length_input)
    {
        const char src[5] = "";
        char dst[8] = {};

        auto result = el::array::copy(src, src, dst, dst + 8);
        EXPECT_EQ(dst, result);
    }

    TEST(array_copy, does_copy_elements_over)
    {
        const char src[5] = "this";
        char dst[8] = {};

        auto result = el::array::copy(src, src + 5, dst, dst + 8);
        EXPECT_NE(result, nullptr);
        EXPECT_EQ((dst + 5), result);
        EXPECT_STREQ(src, dst);
    }

    TEST(array_copy, does_copy_elements_into_same_size)
    {
        const char src[5] = "this";
        char dst[5] = {};

        auto result = el::array::copy(src, src + 5, dst, dst + 5);
        EXPECT_NE(result, nullptr);
        EXPECT_EQ((dst + 5), result);
        EXPECT_STREQ(src, dst);
    }

    TEST(array_copy, stops_when_short)
    {
        const char src[5] = "this";
        char dst[8] = {};

        auto result = el::array::copy(src, src + 5, dst, dst + 3);
        EXPECT_EQ(nullptr, result);
    }
}
