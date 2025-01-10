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

#include "semantic/Tool.h"
#include "semantic/ToolIntelFortran.h"

using namespace cs::semantic;

namespace {

    TEST(ToolIntelFortran, is_compiler_call) {
        struct Expose : public ToolIntelFortran {
            using ToolIntelFortran::is_compiler_call;
        };
        Expose sut;

        EXPECT_TRUE(sut.is_compiler_call("ifx"));
        EXPECT_TRUE(sut.is_compiler_call("/usr/bin/ifx"));
        EXPECT_TRUE(sut.is_compiler_call("ifort"));
        EXPECT_TRUE(sut.is_compiler_call("/usr/bin/ifort"));
        EXPECT_TRUE(sut.is_compiler_call("/opt/intel/oneapi/compiler/2025.0/bin/ifx"));
        EXPECT_TRUE(sut.is_compiler_call("ifx2023"));
        EXPECT_TRUE(sut.is_compiler_call("ifx2025.0"));
        EXPECT_TRUE(sut.is_compiler_call("ifx-avx2"));

        EXPECT_FALSE(sut.is_compiler_call("gfortran"));
        EXPECT_FALSE(sut.is_compiler_call("gcc"));
    }

    TEST(ToolIntelFortran, fails_on_empty) {
        Execution input = {};

        ToolIntelFortran sut;

        EXPECT_TRUE(Tool::not_recognized(sut.recognize(input)));
    }

    TEST(ToolIntelFortran, simple) {
        Execution input = {
            "/opt/intel/oneapi/compiler/2025.0/bin/ifx",
            {"ifx", "-c", "-o", "source.o", "source.c"},
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

        ToolIntelFortran sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolIntelFortran, linker_flag_filtered) {
        Execution input = {
            "/opt/intel/oneapi/compiler/2025.0/bin/ifx",
            {"ifx", "-L.", "-lthing", "-o", "exe", "source.c"},
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

        ToolIntelFortran sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolIntelFortran, pass_on_help) {
        Execution input = {
            "/opt/intel/oneapi/compiler/2025.0/bin/ifx",
            {"ifx", "--version"},
            "/home/user/project",
            {},
            };
        SemanticPtr expected = SemanticPtr(new QueryCompiler());

        ToolIntelFortran sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(result.is_ok());
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }
}
