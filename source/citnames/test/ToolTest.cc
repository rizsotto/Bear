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

#include "semantic/Tool.h"
#include "semantic/ToolGcc.h"

namespace {

    TEST(ToolGcc, fails_on_empty) {
        report::Command input = {};

        cs::ToolGcc sut({});

        EXPECT_FALSE(sut.compilations(input).is_ok());
    }

    TEST(ToolGcc, simple) {
        report::Command input = {
                "/usr/bin/cc",
                {"cc", "-c", "-o", "source.o", "source.c"},
                "/home/user/project",
                {},
        };
        cs::output::Entries expected = {
                {
                        "/home/user/project/source.c",
                        "/home/user/project",
                        { "/home/user/project/source.o" },
                        {"/usr/bin/cc", "-c", "-o", "source.o", "source.c"}},
        };

        cs::ToolGcc sut({});

        auto result = sut.compilations(input);
        EXPECT_TRUE(result.is_ok());
        EXPECT_EQ(expected, result.unwrap_or({}));
    }

    TEST(ToolGcc, linker_flag_filtered) {
        report::Command input = {
                "/usr/bin/cc",
                {"cc", "-L.", "-lthing", "-o", "exe", "source.c"},
                "/home/user/project",
                {},
        };
        cs::output::Entries expected = {
                {
                        "/home/user/project/source.c",
                        "/home/user/project",
                        { "/home/user/project/exe" },
                        {"/usr/bin/cc", "-c", "-o", "exe", "source.c"}},
        };

        cs::ToolGcc sut({});

        auto result = sut.compilations(input);
        EXPECT_TRUE(result.is_ok());
        EXPECT_EQ(expected, result.unwrap_or({}));
    }

    TEST(ToolGcc, pass_on_help) {
        report::Command input = {
                "/usr/bin/gcc",
                {"gcc", "--version"},
                "/home/user/project",
                {},
        };
        cs::output::Entries expected = {};

        cs::ToolGcc sut({});

        auto result = sut.compilations(input);
        EXPECT_TRUE(result.is_ok());
        EXPECT_EQ(expected, result.unwrap_or({}));
    }

    TEST(ToolGcc, simple_with_C_PATH) {
        report::Command input = {
                "/usr/bin/cc",
                {"cc", "-c", "source.c"},
                "/home/user/project",
                {{"CPATH", "/usr/include/path1:/usr/include/path2"},
                 {"C_INCLUDE_PATH", ":/usr/include/path3"}},
        };
        cs::output::Entries expected = {
                {
                        "/home/user/project/source.c",
                        "/home/user/project",
                        std::nullopt,
                        {
                                "/usr/bin/cc",
                                "-c", "source.c",
                                "-I", "/usr/include/path1",
                                "-I", "/usr/include/path2",
                                "-I", ".",
                                "-I", "/usr/include/path3",
                        }
                },
        };

        cs::ToolGcc sut({});

        auto result = sut.compilations(input);
        EXPECT_TRUE(result.is_ok());
        EXPECT_EQ(expected, result.unwrap_or({}));
    }

    TEST(ToolGcc, simple_where_compiler_from_env) {
        report::Command input = {
                "/usr/bin/wrapper",
                {"wrapper", "-c", "source.c"},
                "/home/user/project",
                {},
        };
        cs::output::Entries expected = {
                {
                        "/home/user/project/source.c",
                        "/home/user/project",
                        std::nullopt,
                        {"/usr/bin/wrapper", "-c", "source.c"}
                },
        };

        cs::ToolGcc sut({"/usr/bin/wrapper"});

        auto result = sut.compilations(input);
        EXPECT_TRUE(result.is_ok());
        EXPECT_EQ(expected, result.unwrap_or({}));
    }
}
