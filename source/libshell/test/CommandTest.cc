/*  Copyright (C) 2012-2024 by László Nagy
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

#include "libshell/Command.h"

namespace {

    TEST(command, empty) {
        std::list<std::string> expected = {};

        EXPECT_EQ(expected,sh::split("").unwrap_or({"fake"}));
    }

    TEST(command, whitespace) {
        std::list<std::string> expected = {};

        EXPECT_EQ(expected,sh::split("  ").unwrap_or({"fake"}));
    }

    TEST(command, single_word) {
        std::list<std::string> expected = {"abcd"};

        EXPECT_EQ(expected,sh::split("abcd").unwrap_or({}));
    }

    TEST(command, nothing_special) {
        std::list<std::string> expected = {"a", "b", "c", "d"};

        EXPECT_EQ(expected,sh::split("a b c d").unwrap_or({}));
    }

    TEST(command, quoted_strings) {
        std::list<std::string> expected = {"a", "b b", "a"};

        EXPECT_EQ(expected,sh::split("a \"b b\" a").unwrap_or({}));
    }

    TEST(command, escaped_double_quotes)
    {
        std::list<std::string> expected = {"a", "\"b\" c", "d"};

        EXPECT_EQ(expected,sh::split("a \"\\\"b\\\" c\" d").unwrap_or({}));
    }

    TEST(command, escaped_single_quoutes)
    {
        std::list<std::string> expected = {"a", "'b' c", "d"};

        EXPECT_EQ(expected,sh::split("a \"'b' c\" d").unwrap_or({}));
    }

    TEST(command, escaped_spaces)
    {
        std::list<std::string> expected = {"a", "b c", "d"};

        EXPECT_EQ(expected,sh::split("a b\\ c d").unwrap_or({}));
    }

    TEST(command, bad_double_quotes)
    {
        EXPECT_FALSE(sh::split("a \"b c d e").is_ok());
    }

    TEST(command, bad_single_quotes)
    {
        EXPECT_FALSE(sh::split("a 'b c d e").is_ok());
    }

    TEST(command, bad_quotes)
    {
        EXPECT_FALSE(sh::split("one '\"\"\"").is_ok());
    }

    TEST(command, trailing_whitespace)
    {
        std::list<std::string> expected = {"a", "b", "c", "d"};

        EXPECT_EQ(expected,sh::split("a b c d ").unwrap_or({}));
    }

    TEST(command, percent_signs)
    {
        std::list<std::string> expected = {"abc", "%foo bar%"};

        EXPECT_EQ(expected,sh::split("abc '%foo bar%'").unwrap_or({}));
    }

    TEST(command, empty_escape)
    {
        EXPECT_EQ("''",sh::escape(""));
    }

    TEST(command, full_escape)
    {
        EXPECT_EQ("foo\\ \\'\\\"\\'\\ bar",sh::escape("foo '\"' bar"));
    }

    TEST(command, escape_and_join_whitespace)
    {
        std::string empty;
        std::string space(" ");
        std::string newline("\n");
        std::string tab("\t");

        std::list<std::string> tokens = {
            empty,
            space,
            space + space,
            newline,
            newline + newline,
            tab,
            tab + tab,
            empty,
            space + newline + tab,
            empty
        };

        for (const auto& token : tokens) {
            const std::list<std::string> expected = { token };
            EXPECT_EQ(expected, sh::split(sh::escape(token)).unwrap_or({}));
        }

        EXPECT_EQ(tokens, sh::split(sh::join(tokens)).unwrap_or({}));
    }
}