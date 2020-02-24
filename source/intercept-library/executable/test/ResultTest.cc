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

#include "Result.h"

namespace {

    using Error = const char*;
    using namespace er;

    TEST(result, get_or_else_on_success)
    {
        EXPECT_EQ(2,
            (Result<int, Error>(Ok(2))
                    .get_or_else(8)));
        EXPECT_EQ('c',
            (Result<char, Error>(Ok('c'))
                    .get_or_else('+')));
    }

    TEST(result, get_or_else_on_failure)
    {
        EXPECT_EQ(8,
            (Result<int, Error>(Err("problem"))
                    .get_or_else(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Err("problem"))
                    .get_or_else('+')));
    }

    TEST(result, map_on_success)
    {
        EXPECT_EQ(4,
            (Result<int, Error>(Ok(2))
                    .map<int>([](auto& in) {
                        return in * 2;
                    })
                    .get_or_else(8)));
        EXPECT_EQ(2.5f,
            (Result<int, Error>(Ok(2))
                    .map<float>([](auto& in) {
                        return in + 0.5f;
                    })
                    .get_or_else(8.0f)));
        EXPECT_EQ('d',
            (Result<char, Error>(Ok('c'))
                    .map<int>([](auto& in) {
                        return in + 1;
                    })
                    .get_or_else(42)));
    }

    TEST(result, map_on_failure)
    {
        EXPECT_EQ(8,
            (Result<int, Error>(Err("problem"))
                    .map<int>([](auto& in) {
                        return in * 2;
                    })
                    .get_or_else(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Err("problem"))
                    .map<char>([](const char& in) {
                        return char(in + 1);
                    })
                    .get_or_else('+')));
    }

    TEST(result, bind_on_success)
    {
        EXPECT_EQ(2,
            (Result<int, Error>(Ok(1))
                    .bind<int>([](auto& in) {
                        return Ok(in * 2);
                    })
                    .get_or_else(8)));
        EXPECT_EQ('d',
            (Result<char, Error>(Ok('c'))
                    .bind<char>([](auto& in) {
                        return Ok(char(in + 1));
                    })
                    .get_or_else('+')));
        EXPECT_EQ(8,
            (Result<int, Error>(Ok(1))
                    .bind<int>([](auto& in) {
                        return Err("problem");
                    })
                    .get_or_else(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Ok('c'))
                    .bind<char>([](auto& in) {
                        return Err("problem");
                    })
                    .get_or_else('+')));
    }

    TEST(result, bind_on_failure)
    {
        EXPECT_EQ(8,
            (Result<int, Error>(Err("problem"))
                    .bind<int>([](auto& in) {
                        return Ok(in * 2);
                    })
                    .get_or_else(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Err("problem"))
                    .bind<char>([](auto& in) {
                        return Ok(char(in + 1));
                    })
                    .get_or_else('+')));
        EXPECT_EQ(8,
            (Result<int, Error>(Err("problem"))
                    .bind<int>([](auto& in) {
                        return Err("another problem");
                    })
                    .get_or_else(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Err("problem"))
                    .bind<char>([](auto& in) {
                        return Err("another problem");
                    })
                    .get_or_else('+')));
    }

    TEST(result, handle_with_on_success)
    {
        char const* result = "expected";

        Result<int, Error>(Ok(1))
            .handle_with([&result](char const* in) {
                result = in;
            });
        EXPECT_STREQ("expected", result);
    }

    TEST(result, handle_with_on_failure)
    {
        char const* result = "expected";

        Result<int, Error>(Err("problem"))
            .handle_with([&result](char const* in) {
                result = in;
            });
        EXPECT_STREQ("problem", result);
    }

}