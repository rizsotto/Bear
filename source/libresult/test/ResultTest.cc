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
    using namespace rust;

    TEST(result, unwrap_or_on_success)
    {
        EXPECT_EQ(2,
            (Result<int, Error>(Ok(2)).unwrap_or(8)));
        EXPECT_EQ('c',
            (Result<char, Error>(Ok('c')).unwrap_or('+')));
    }

    TEST(result, unwrap_or_on_failure)
    {
        EXPECT_EQ(8,
            (Result<int, Error>(Err("problem")).unwrap_or(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Err("problem")).unwrap_or('+')));
    }

    TEST(result, unwrap_or_else_on_success)
    {
        EXPECT_EQ(2,
            (Result<int, Error>(Ok(2)).unwrap_or_else([](auto error) { return 8; })));
        EXPECT_EQ('c',
            (Result<char, Error>(Ok('c')).unwrap_or_else([](auto error) { return '+'; })));
    }

    TEST(result, unwrap_or_else_on_failure)
    {
        EXPECT_EQ(8,
            (Result<int, Error>(Err("problem")).unwrap_or_else([](auto error) { return 8; })));
        EXPECT_EQ('+',
            (Result<char, Error>(Err("problem")).unwrap_or_else([](auto error) { return '+'; })));
    }

    TEST(result, map_on_success)
    {
        EXPECT_EQ(4,
            (Result<int, Error>(Ok(2))
                    .map<int>([](auto& in) { return in * 2; })
                    .unwrap_or(8)));
        EXPECT_EQ(2.5f,
            (Result<int, Error>(Ok(2))
                    .map<float>([](auto& in) { return in + 0.5f; })
                    .unwrap_or(8.0f)));
        EXPECT_EQ('d',
            (Result<char, Error>(Ok('c'))
                    .map<int>([](auto& in) { return in + 1; })
                    .unwrap_or(42)));
    }

    TEST(result, map_on_failure)
    {
        EXPECT_EQ(8,
            (Result<int, Error>(Err("problem"))
                    .map<int>([](auto& in) { return in * 2; })
                    .unwrap_or(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Err("problem"))
                    .map<char>([](const char& in) { return char(in + 1); })
                    .unwrap_or('+')));
    }

    TEST(result, map_or_on_success)
    {
        EXPECT_EQ(4,
            (Result<int, Error>(Ok(2))
                    .map_or<int>(7, [](auto& in) { return in * 2; })
                    .unwrap_or(8)));
        EXPECT_EQ(2.5f,
            (Result<int, Error>(Ok(2))
                    .map_or<float>(7.8, [](auto& in) { return in + 0.5f; })
                    .unwrap_or(8.0f)));
        EXPECT_EQ('d',
            (Result<char, Error>(Ok('c'))
                    .map_or<int>(13, [](auto& in) { return in + 1; })
                    .unwrap_or(42)));
    }

    TEST(result, map_or_on_failure)
    {
        EXPECT_EQ(9,
            (Result<int, Error>(Err("problem"))
                    .map_or<int>(9, [](auto& in) { return in * 2; })
                    .unwrap_or(8)));
        EXPECT_EQ('#',
            (Result<char, Error>(Err("problem"))
                    .map_or<char>('#', [](const char& in) { return char(in + 1); })
                    .unwrap_or('+')));
    }

    TEST(result, map_or_else_on_success)
    {
        EXPECT_EQ(4,
            (Result<int, Error>(Ok(2))
                    .map_or_else<int>([](auto error) { return 9; }, [](auto& in) { return in * 2; })
                    .unwrap_or(8)));
        EXPECT_EQ(2.5f,
            (Result<int, Error>(Ok(2))
                    .map_or_else<float>([](auto error) { return 7.8; }, [](auto& in) { return in + 0.5f; })
                    .unwrap_or(8.0f)));
        EXPECT_EQ('d',
            (Result<char, Error>(Ok('c'))
                    .map_or_else<int>([](auto error) { return 13; }, [](auto& in) { return in + 1; })
                    .unwrap_or(42)));
    }

    TEST(result, map_or_else_on_failure)
    {
        EXPECT_EQ(9,
            (Result<int, Error>(Err("problem"))
                    .map_or_else<int>([](auto error) { return 9; }, [](auto& in) { return in * 2; })
                    .unwrap_or(8)));
        EXPECT_EQ('#',
            (Result<char, Error>(Err("problem"))
                    .map_or_else<char>([](auto error) { return '#'; }, [](const char& in) { return char(in + 1); })
                    .unwrap_or('+')));
    }

    TEST(result, map_err_on_success)
    {
        EXPECT_EQ(2,
            (Result<int, Error>(Ok(2))
                    .map_err<int>([](auto error) { return 9; })
                    .unwrap_or(8)));
        EXPECT_EQ(2.5f,
            (Result<float, Error>(Ok(2.5f))
                    .map_err<char>([](auto error) { return '+'; })
                    .unwrap_or(8.0f)));
    }

    TEST(result, map_err_on_failure)
    {
        EXPECT_EQ(8,
            (Result<int, Error>(Err("problem"))
                    .map_err<int>([](auto error) { return 9; })
                    .unwrap_or(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Err("problem"))
                    .map_err<char>([](auto error) { return '#'; })
                    .unwrap_or('+')));
    }

    TEST(result, and_)
    {
        {
            auto x = Result<int, Error>(Ok(2));
            auto y = Result<int, Error>(Err("late error"));

            x.and_(y).map_err<int>([](auto error) {
                EXPECT_STREQ("late error", error);
                return 0;
            });
        }
        {
            auto x = Result<int, Error>(Err("early error"));
            auto y = Result<int, Error>(Ok(2));

            x.and_(y).map_err<int>([](auto error) {
                EXPECT_STREQ("early error", error);
                return 0;
            });
        }
        {
            auto x = Result<int, Error>(Err("early error"));
            auto y = Result<int, Error>(Err("late error"));

            x.and_(y).map_err<int>([](auto error) {
                EXPECT_STREQ("early error", error);
                return 0;
            });
        }
        {
            auto x = Result<int, Error>(Ok(2));
            auto y = Result<char, Error>(Ok('x'));

            x.and_(y).map<char>([](auto value) {
                EXPECT_EQ('x', value);
                return 0;
            });
        }
    }

    TEST(result, and_then_on_success)
    {
        EXPECT_EQ(2,
            (Result<int, Error>(Ok(1))
                    .and_then<int>([](auto& in) { return Ok(in * 2); })
                    .unwrap_or(8)));
        EXPECT_EQ('d',
            (Result<char, Error>(Ok('c'))
                    .and_then<char>([](auto& in) { return Ok(char(in + 1)); })
                    .unwrap_or('+')));
        EXPECT_EQ(8,
            (Result<int, Error>(Ok(1))
                    .and_then<int>([](auto& in) { return Err("problem"); })
                    .unwrap_or(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Ok('c'))
                    .and_then<char>([](auto& in) { return Err("problem"); })
                    .unwrap_or('+')));
    }

    TEST(result, and_then_on_failure)
    {
        EXPECT_EQ(8,
            (Result<int, Error>(Err("problem"))
                    .and_then<int>([](auto& in) { return Ok(in * 2); })
                    .unwrap_or(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Err("problem"))
                    .and_then<char>([](auto& in) { return Ok(char(in + 1)); })
                    .unwrap_or('+')));
        EXPECT_EQ(8,
            (Result<int, Error>(Err("problem"))
                    .and_then<int>([](auto& in) { return Err("another problem"); })
                    .unwrap_or(8)));
        EXPECT_EQ('+',
            (Result<char, Error>(Err("problem"))
                    .and_then<char>([](auto& in) { return Err("another problem"); })
                    .unwrap_or('+')));
    }

    TEST(result, or_)
    {
        {
            auto x = Result<int, Error>(Ok(2));
            auto y = Result<int, Error>(Err("late error"));

            x.or_(y).map<int>([](auto value) {
                EXPECT_EQ(2, value);
                return 0;
            });
        }
        {
            auto x = Result<int, Error>(Err("early error"));
            auto y = Result<int, Error>(Ok(2));

            x.or_(y).map<int>([](auto value) {
                EXPECT_EQ(2, value);
                return 0;
            });
        }
        {
            auto x = Result<int, Error>(Err("early error"));
            auto y = Result<int, Error>(Err("late error"));

            x.or_(y).map_err<int>([](auto error) {
                EXPECT_STREQ("late error", error);
                return 0;
            });
        }
        {
            auto x = Result<int, Error>(Ok(2));
            auto y = Result<int, Error>(Ok(100));

            x.or_(y).map<int>([](auto value) {
                EXPECT_EQ(2, value);
                return 0;
            });
        }
    }

    TEST(result, or_else_on_success)
    {
        EXPECT_EQ(1,
                  (Result<int, Error>(Ok(1))
                      .or_else([](auto& error) { return Ok(2); })
                      .unwrap_or(8)));
        EXPECT_EQ('c',
                  (Result<char, Error>(Ok('c'))
                      .or_else([](auto& error) { return Ok('x'); })
                      .unwrap_or('+')));
        EXPECT_EQ(1,
                  (Result<int, Error>(Ok(1))
                      .or_else([](auto& error) { return Err("problem"); })
                      .unwrap_or(8)));
        EXPECT_EQ('c',
                  (Result<char, Error>(Ok('c'))
                      .or_else([](auto& error) { return Err("problem"); })
                      .unwrap_or('+')));
    }

    TEST(result, or_else_on_failure)
    {
        EXPECT_EQ(2,
                  (Result<int, Error>(Err("problem"))
                      .or_else([](auto& error) { return Ok(2); })
                      .unwrap_or(8)));
        EXPECT_EQ('x',
                  (Result<char, Error>(Err("problem"))
                      .or_else([](auto& error) { return Ok('x'); })
                      .unwrap_or('+')));
        EXPECT_EQ(8,
                  (Result<int, Error>(Err("problem"))
                      .or_else([](auto& error) { return Err("another problem"); })
                      .unwrap_or(8)));
        EXPECT_EQ('+',
                  (Result<char, Error>(Err("problem"))
                      .or_else([](auto& error) { return Err("another problem"); })
                      .unwrap_or('+')));
    }
}
