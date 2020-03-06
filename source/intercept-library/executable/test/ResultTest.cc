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
                    .unwrap_or(8)));
        EXPECT_EQ('c',
            (Result<char, Error>(Ok('c'))
                    .unwrap_or('+')));
    }

    TEST(result, get_or_else_on_failure)
    {
        EXPECT_EQ(8,
            (Result<int, Error>(Err("problem"))
                    .unwrap_or(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Err("problem"))
                    .unwrap_or('+')));
    }

    TEST(result, map_on_success)
    {
        EXPECT_EQ(4,
            (Result<int, Error>(Ok(2))
                    .map<int>([](auto& in) {
                        return in * 2;
                    })
                    .unwrap_or(8)));
        EXPECT_EQ(2.5f,
            (Result<int, Error>(Ok(2))
                    .map<float>([](auto& in) {
                        return in + 0.5f;
                    })
                    .unwrap_or(8.0f)));
        EXPECT_EQ('d',
            (Result<char, Error>(Ok('c'))
                    .map<int>([](auto& in) {
                        return in + 1;
                    })
                    .unwrap_or(42)));
    }

    TEST(result, map_on_failure)
    {
        EXPECT_EQ(8,
            (Result<int, Error>(Err("problem"))
                    .map<int>([](auto& in) {
                        return in * 2;
                    })
                    .unwrap_or(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Err("problem"))
                    .map<char>([](const char& in) {
                        return char(in + 1);
                    })
                    .unwrap_or('+')));
    }

    TEST(result, bind_on_success)
    {
        EXPECT_EQ(2,
            (Result<int, Error>(Ok(1))
                    .and_then<int>([](auto& in) {
                        return Ok(in * 2);
                    })
                    .unwrap_or(8)));
        EXPECT_EQ('d',
            (Result<char, Error>(Ok('c'))
                    .and_then<char>([](auto& in) {
                        return Ok(char(in + 1));
                    })
                    .unwrap_or('+')));
        EXPECT_EQ(8,
            (Result<int, Error>(Ok(1))
                    .and_then<int>([](auto& in) {
                        return Err("problem");
                    })
                    .unwrap_or(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Ok('c'))
                    .and_then<char>([](auto& in) {
                        return Err("problem");
                    })
                    .unwrap_or('+')));
    }

    TEST(result, bind_on_failure)
    {
        EXPECT_EQ(8,
            (Result<int, Error>(Err("problem"))
                    .and_then<int>([](auto& in) {
                        return Ok(in * 2);
                    })
                    .unwrap_or(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Err("problem"))
                    .and_then<char>([](auto& in) {
                        return Ok(char(in + 1));
                    })
                    .unwrap_or('+')));
        EXPECT_EQ(8,
            (Result<int, Error>(Err("problem"))
                    .and_then<int>([](auto& in) {
                        return Err("another problem");
                    })
                    .unwrap_or(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Err("problem"))
                    .and_then<char>([](auto& in) {
                        return Err("another problem");
                    })
                    .unwrap_or('+')));
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