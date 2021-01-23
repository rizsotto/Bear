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

#include "semantic/Tool.h"
#include "semantic/ToolGcc.h"

namespace {

    TEST(ToolGcc, recognize) {
        cs::semantic::ToolGcc sut({});

        EXPECT_TRUE(sut.recognize("cc"));
        EXPECT_TRUE(sut.recognize("/usr/bin/cc"));
        EXPECT_TRUE(sut.recognize("gcc"));
        EXPECT_TRUE(sut.recognize("/usr/bin/gcc"));
        EXPECT_TRUE(sut.recognize("c++"));
        EXPECT_TRUE(sut.recognize("/usr/bin/c++"));
        EXPECT_TRUE(sut.recognize("g++"));
        EXPECT_TRUE(sut.recognize("/usr/bin/g++"));
        EXPECT_TRUE(sut.recognize("arm-none-eabi-g++"));
        EXPECT_TRUE(sut.recognize("/usr/bin/arm-none-eabi-g++"));
        EXPECT_TRUE(sut.recognize("gcc-6"));
        EXPECT_TRUE(sut.recognize("/usr/bin/gcc-6"));
    }

    TEST(ToolGcc, fails_on_empty) {
        cs::semantic::Execution input = {};

        cs::semantic::ToolGcc sut({});

        EXPECT_FALSE(sut.compilations(input).is_ok());
    }

    TEST(ToolGcc, simple) {
        cs::semantic::Execution input = {
                "/usr/bin/cc",
                {"cc", "-c", "-o", "source.o", "source.c"},
                "/home/user/project",
                {},
        };
        cs::semantic::SemanticPtrs expected = {
                cs::semantic::SemanticPtr(
                        new cs::semantic::Compile(
                                input,
                                fs::path("/home/user/project/source.c"),
                                fs::path("/home/user/project/source.o"),
                                {"/usr/bin/cc", "-c", "-o", "source.o", "source.c"}
                        )
                )
        };

        cs::semantic::ToolGcc sut({});

        auto result = sut.compilations(input);
        EXPECT_TRUE(result.is_ok());
        EXPECT_EQ(expected, result.unwrap_or({}));
    }

    TEST(ToolGcc, linker_flag_filtered) {
        cs::semantic::Execution input = {
                "/usr/bin/cc",
                {"cc", "-L.", "-lthing", "-o", "exe", "source.c"},
                "/home/user/project",
                {},
        };
        cs::semantic::SemanticPtrs expected = {
                cs::semantic::SemanticPtr(
                        new cs::semantic::Compile(
                                input,
                                fs::path("/home/user/project/source.c"),
                                fs::path("/home/user/project/source.o"),
                                {"/usr/bin/cc", "-c", "-o", "exe", "source.c"}
                        )
                )
        };

        cs::semantic::ToolGcc sut({});

        auto result = sut.compilations(input);
        EXPECT_TRUE(result.is_ok());
        EXPECT_EQ(expected, result.unwrap_or({}));
    }

    TEST(ToolGcc, pass_on_help) {
        cs::semantic::Execution input = {
                "/usr/bin/gcc",
                {"gcc", "--version"},
                "/home/user/project",
                {},
        };
        cs::semantic::SemanticPtrs expected = {
                cs::semantic::SemanticPtr(
                        new cs::semantic::QueryCompiler(input)
                )
        };

        cs::semantic::ToolGcc sut({});

        auto result = sut.compilations(input);
        EXPECT_TRUE(result.is_ok());
        EXPECT_EQ(expected, result.unwrap_or({}));
    }

    TEST(ToolGcc, simple_with_C_PATH) {
        cs::semantic::Execution input = {
                "/usr/bin/cc",
                {"cc", "-c", "source.c"},
                "/home/user/project",
                {{"CPATH", "/usr/include/path1:/usr/include/path2"},
                 {"C_INCLUDE_PATH", ":/usr/include/path3"}},
        };
        cs::semantic::SemanticPtrs expected = {
                cs::semantic::SemanticPtr(
                        new cs::semantic::Compile(
                                input,
                                fs::path("/home/user/project/source.c"),
                                fs::path("/home/user/project/source.o"),
                                {
                                    "/usr/bin/cc",
                                    "-c",
                                    "source.c",
                                    "-I", "/usr/include/path1",
                                    "-I", "/usr/include/path2",
                                    "-I", ".",
                                    "-I", "/usr/include/path3",
                                }
                        )
                )
        };

        cs::semantic::ToolGcc sut({});

        auto result = sut.compilations(input);
        EXPECT_TRUE(result.is_ok());
        EXPECT_EQ(expected, result.unwrap_or({}));
    }
}
