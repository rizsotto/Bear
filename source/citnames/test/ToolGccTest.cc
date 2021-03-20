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

using namespace cs::semantic;

namespace {

    TEST(ToolGcc, recognize) {
        struct Expose : public ToolGcc {
            [[nodiscard]] bool recognize(const fs::path& program) const override {
                return ToolGcc::recognize(program);
            }
        };
        Expose sut;

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
        EXPECT_TRUE(sut.recognize("gfortran"));
        EXPECT_TRUE(sut.recognize("fortran"));
    }

    TEST(ToolGcc, fails_on_empty) {
        Execution input = {};

        ToolGcc sut;

        EXPECT_TRUE(Tool::not_recognized(sut.recognize(input)));
    }

    TEST(ToolGcc, simple) {
        Execution input = {
                "/usr/bin/cc",
                {"cc", "-c", "-o", "source.o", "source.c"},
                "/home/user/project",
                {},
        };
        SemanticPtr expected = SemanticPtr(
                new Compile(
                        input.working_dir,
                        input.executable,
                        {"-c"},
                        {fs::path("source.c")},
                        {fs::path("source.o")})
        );

        ToolGcc sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolGcc, linker_flag_filtered) {
        Execution input = {
                "/usr/bin/cc",
                {"cc", "-L.", "-lthing", "-o", "exe", "source.c"},
                "/home/user/project",
                {},
        };
        SemanticPtr expected = SemanticPtr(
                new Compile(
                        input.working_dir,
                        input.executable,
                        {"-c"},
                        {fs::path("source.c")},
                        {fs::path("exe")}
                )
        );

        ToolGcc sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolGcc, pass_on_help) {
        Execution input = {
                "/usr/bin/gcc",
                {"gcc", "--version"},
                "/home/user/project",
                {},
        };
        SemanticPtr expected = SemanticPtr(new QueryCompiler());

        ToolGcc sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(result.is_ok());
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolGcc, simple_with_C_PATH) {
        Execution input = {
                "/usr/bin/cc",
                {"cc", "-c", "source.c"},
                "/home/user/project",
                {{"CPATH", "/usr/include/path1:/usr/include/path2"},
                 {"C_INCLUDE_PATH", ":/usr/include/path3"}},
        };
        SemanticPtr expected = SemanticPtr(
                new Compile(
                        input.working_dir,
                        input.executable,
                        {
                                "-c",
                                "-I", "/usr/include/path1",
                                "-I", "/usr/include/path2",
                                "-I", ".",
                                "-I", "/usr/include/path3",
                        },
                        {fs::path("source.c")},
                        std::nullopt
                )
        );

        ToolGcc sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }
}
