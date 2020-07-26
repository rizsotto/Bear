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

#include "Tool.h"

namespace {

    TEST(GnuCompilerCollection, fails_on_empty) {
        report::Execution::Command input = {};

        cs::GnuCompilerCollection sut(std::nullopt);

        EXPECT_FALSE(sut.recognize(input).is_ok());
    }

    TEST(GnuCompilerCollection, simple) {
        report::Execution::Command input = {
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

        cs::GnuCompilerCollection sut(std::nullopt);

        auto result = sut.recognize(input);
        EXPECT_TRUE(result.is_ok());
        EXPECT_EQ(expected, result.unwrap_or({}));
    }

    TEST(GnuCompilerCollection, pass_on_help) {
        report::Execution::Command input = {
                "/usr/bin/gcc",
                {"gcc", "--version"},
                "/home/user/project",
                {},
        };
        cs::output::Entries expected = {};

        cs::GnuCompilerCollection sut(std::nullopt);

        auto result = sut.recognize(input);
        EXPECT_TRUE(result.is_ok());
        EXPECT_EQ(expected, result.unwrap_or({}));
    }

    TEST(GnuCompilerCollection, simple_with_C_PATH) {
        report::Execution::Command input = {
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

        cs::GnuCompilerCollection sut(std::nullopt);

        auto result = sut.recognize(input);
        EXPECT_TRUE(result.is_ok());
        EXPECT_EQ(expected, result.unwrap_or({}));
    }

    TEST(GnuCompilerCollection, simple_where_compiler_from_env) {
        report::Execution::Command input = {
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
                        {"/usr/bin/wrapper", "-c", "source.c"}},
        };

        cs::GnuCompilerCollection sut(std::optional("/usr/bin/wrapper"));

        auto result = sut.recognize(input);
        EXPECT_TRUE(result.is_ok());
        EXPECT_EQ(expected, result.unwrap_or({}));
    }
}
