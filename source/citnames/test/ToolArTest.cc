/*  Copyright (C) 2012-2023 by László Nagy
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
#include "semantic/ToolAr.h"

using namespace cs::semantic;

namespace {

    TEST(ToolAr, is_ar_call) {
        struct Expose : public ToolAr {
            using ToolAr::is_linker_call;
        };
        Expose sut;

        EXPECT_TRUE(sut.is_linker_call("ar"));
        EXPECT_TRUE(sut.is_linker_call("/usr/bin/ar"));
        EXPECT_FALSE(sut.is_linker_call("gcc"));
        EXPECT_FALSE(sut.is_linker_call("/usr/bin/gcc"));
    }

    TEST(ToolAr, target_compiler) {
        Execution input = {
                "/usr/bin/ar",
                {"ar", "qc", "libmy.a"},
                "/home/user/project",
                {},
        };

        ToolAr sut;

        EXPECT_TRUE(Tool::not_recognized(sut.recognize(input, BuildTarget::COMPILER)));
    }

    TEST(ToolAr, fails_on_empty) {
        Execution input = {};

        ToolAr sut;

        EXPECT_TRUE(Tool::not_recognized(sut.recognize(input, BuildTarget::LINKER)));
    }

    TEST(ToolAr, pass_on_help) {
        Execution input = {
                "/usr/bin/ar",
                {"ar", "--version"},
                "/home/user/project",
                {},
        };
        SemanticPtr expected = SemanticPtr(new QueryCompiler());

        ToolAr sut({});

        auto result = sut.recognize(input, BuildTarget::LINKER);
        EXPECT_TRUE(result.is_ok());
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolAr, simple_without_files) {
        Execution input = {
                "/usr/bin/ar",
                {"ar", "qc", "libmy.a"},
                "/home/user/project",
                {},
        };

        SemanticPtr expected = SemanticPtr(
                new Link(
                        input.working_dir,
                        input.executable,
                        {"qc", "libmy.a"},
                        {},
                        {fs::path("libmy.a")}
                )
        );

        ToolAr sut({});

        auto result = sut.recognize(input, BuildTarget::LINKER);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolAr, simple_with_files) {
        Execution input = {
                "/usr/bin/ar",
                {"ar", "qc", "libmy.a", "x.o", "lmy.a", "x.cpp"},
                "/home/user/project",
                {},
        };

        SemanticPtr expected = SemanticPtr(
                new Link(
                        input.working_dir,
                        input.executable,
                        {"qc", "libmy.a", "x.o", "lmy.a", "x.cpp"},
                        {"x.o", "lmy.a", "x.cpp"},
                        {fs::path("libmy.a")}
                )
        );

        ToolAr sut({});

        auto result = sut.recognize(input, BuildTarget::LINKER);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolAr, with_flags) {
        Execution input = {
                "/usr/bin/ar",
                {"ar", "qc", "--plugin", "l.a", "--output=/usr/my/", "libmy.a", "x.o"},
                "/home/user/project",
                {},
        };

        SemanticPtr expected = SemanticPtr(
                new Link(
                        input.working_dir,
                        input.executable,
                        {"qc", "--plugin", "l.a", "--output=/usr/my/", "libmy.a", "x.o"},
                        {"x.o"},
                        {fs::path("libmy.a")}
                )
        );

        ToolAr sut({});

        auto result = sut.recognize(input, BuildTarget::LINKER);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }
}
