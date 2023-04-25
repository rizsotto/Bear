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
#include "semantic/ToolGcc.h"

using namespace cs::semantic;

namespace {

    TEST(ToolGcc, is_compiler_call) {
        struct Expose : public ToolGcc {
            using ToolGcc::is_compiler_call;
        };
        Expose sut;

        EXPECT_TRUE(sut.is_compiler_call("cc"));
        EXPECT_TRUE(sut.is_compiler_call("/usr/bin/cc"));
        EXPECT_TRUE(sut.is_compiler_call("gcc"));
        EXPECT_TRUE(sut.is_compiler_call("/usr/bin/gcc"));
        EXPECT_TRUE(sut.is_compiler_call("c++"));
        EXPECT_TRUE(sut.is_compiler_call("/usr/bin/c++"));
        EXPECT_TRUE(sut.is_compiler_call("g++"));
        EXPECT_TRUE(sut.is_compiler_call("/usr/bin/g++"));
        EXPECT_TRUE(sut.is_compiler_call("arm-none-eabi-g++"));
        EXPECT_TRUE(sut.is_compiler_call("/usr/bin/arm-none-eabi-g++"));
        EXPECT_TRUE(sut.is_compiler_call("gcc-6"));
        EXPECT_TRUE(sut.is_compiler_call("/usr/bin/gcc-6"));
        EXPECT_TRUE(sut.is_compiler_call("gfortran"));
        EXPECT_TRUE(sut.is_compiler_call("fortran"));
    }

    TEST(ToolGcc, compilation_fails_on_empty) {
        Execution input = {};

        ToolGcc sut;

        EXPECT_TRUE(Tool::not_recognized(sut.recognize(input, BuildTarget::COMPILER)));
    }

    TEST(ToolGcc, compilation_check_compilation_without_compilation) {
        Execution input = {
                "/usr/bin/cc",
                {"cc", "-L.", "source_1.o", "lib.a", "source_2.o", "-la"},
                "/home/user/project",
                {},
        };

        ToolGcc sut({});
        EXPECT_TRUE(Tool::recognized_with_error(sut.recognize(input, BuildTarget::COMPILER)));
    }

    TEST(ToolGcc, compilation_simple) {
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
                        {},
                        {fs::path("source.o")},
                        false
                )
        );

        ToolGcc sut({});

        auto result = sut.recognize(input, BuildTarget::COMPILER);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolGcc, compilation_output_filtered) {
        Execution input = {
                "/usr/bin/cc",
                {"cc", "source.c", "-L.", "-lthing", "-o", "exe"},
                "/home/user/project",
                {},
        };
        SemanticPtr expected = SemanticPtr(
                new Compile(
                        input.working_dir,
                        input.executable,
                        {"-c", "-L.", "-lthing"},
                        {fs::path("source.c")},
                        {},
                        {fs::path("exe")},
                        true
                )
        );

        ToolGcc sut({});

        auto result = sut.recognize(input, BuildTarget::COMPILER);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolGcc, compilation_pass_on_help) {
        Execution input = {
                "/usr/bin/gcc",
                {"gcc", "--version"},
                "/home/user/project",
                {},
        };
        SemanticPtr expected = SemanticPtr(new QueryCompiler());

        ToolGcc sut({});

        auto result = sut.recognize(input, BuildTarget::COMPILER);
        EXPECT_TRUE(result.is_ok());
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolGcc, compilation_simple_with_C_PATH) {
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
                        {},
                        std::nullopt,
                        false
                )
        );

        ToolGcc sut({});

        auto result = sut.recognize(input, BuildTarget::COMPILER);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolGcc, compilation_with_linking_one_file) {
        Execution input = {
                "/usr/bin/cc",
                {"cc", "-o", "source", "source.c"},
                "/home/user/project",
                {},
        };
        SemanticPtr expected = SemanticPtr(
                new Compile(
                        input.working_dir,
                        input.executable,
                        {"-c"},
                        {fs::path("source.c")},
                        {},
                        {fs::path("source")},
                        true
                )
        );

        ToolGcc sut({});

        auto result = sut.recognize(input, BuildTarget::COMPILER);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolGcc, compilation_with_linking_with_obj) {
        Execution input = {
                "/usr/bin/cc",
                {"cc", "source_1.c", "-o", "source", "source_2.c", "obj.o"},
                "/home/user/project",
                {},
        };
        SemanticPtr expected = SemanticPtr(
                new Compile(
                        input.working_dir,
                        input.executable,
                        {"-c", "obj.o"},
                        {"source_1.c", "source_2.c"},
                        {"obj.o"},
                        {fs::path("source")},
                        true
                )
        );

        ToolGcc sut({});

        auto result = sut.recognize(input, BuildTarget::COMPILER);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolGcc, compilation_with_obj_and_libs) {
        Execution input = {
                "/usr/bin/cc",
                {"cc", "-c", "lib.library", "source_1.c", "lib.so.2", "-o", "source", "source_2.c", "obj.o", "lib.dll"},
                "/home/user/project",
                {},
        };
        SemanticPtr expected = SemanticPtr(
                new Compile(
                        input.working_dir,
                        input.executable,
                        {"-c", "lib.library", "lib.so.2", "obj.o", "lib.dll"},
                        {"source_1.c", "source_2.c"},
                        {"lib.library", "lib.so.2", "obj.o", "lib.dll"},
                        {fs::path("source")},
                        false
                )
        );

        ToolGcc sut({});

        auto result = sut.recognize(input, BuildTarget::COMPILER);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolGcc, compilation_with_unknown_files) {
        Execution input = {
                "/usr/bin/cc",
                {"cc", "-c", "lib.library", "lib", "aaaaa", "source_1.c", "lib.so", "-o", "source", "source_2.c", "obj.o", "lib.dll"},
                "/home/user/project",
                {},
        };
        SemanticPtr expected = SemanticPtr(
                new Compile(
                        input.working_dir,
                        input.executable,
                        {"-c", "lib.library", "lib", "aaaaa", "lib.so", "obj.o", "lib.dll"},
                        {"source_1.c", "source_2.c"},
                        {"lib.library", "lib.so", "obj.o", "lib.dll"},
                        {fs::path("source")},
                        false
                )
        );

        ToolGcc sut({});

        auto result = sut.recognize(input, BuildTarget::COMPILER);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

//---------------------------------------------------------------------------------------------------------

    TEST(ToolGcc, is_linker_call) {
        struct Expose : public ToolGcc {
            using ToolGcc::is_linker_call;
        };
        Expose sut;

        EXPECT_TRUE(sut.is_linker_call("cc"));
        EXPECT_TRUE(sut.is_linker_call("/usr/bin/cc"));
        EXPECT_TRUE(sut.is_linker_call("gcc"));
        EXPECT_TRUE(sut.is_linker_call("/usr/bin/gcc"));
        EXPECT_TRUE(sut.is_linker_call("c++"));
        EXPECT_TRUE(sut.is_linker_call("/usr/bin/c++"));
        EXPECT_TRUE(sut.is_linker_call("g++"));
        EXPECT_TRUE(sut.is_linker_call("/usr/bin/g++"));
        EXPECT_TRUE(sut.is_linker_call("arm-none-eabi-g++"));
        EXPECT_TRUE(sut.is_linker_call("/usr/bin/arm-none-eabi-g++"));
        EXPECT_TRUE(sut.is_linker_call("gcc-6"));
        EXPECT_TRUE(sut.is_linker_call("/usr/bin/gcc-6"));
        EXPECT_TRUE(sut.is_linker_call("gfortran"));
        EXPECT_TRUE(sut.is_linker_call("fortran"));
        EXPECT_TRUE(sut.is_linker_call("ld"));
        EXPECT_TRUE(sut.is_linker_call("lld"));
        EXPECT_TRUE(sut.is_linker_call("ar"));
    }

    TEST(ToolGcc, linking_fails_on_empty) {
        Execution input = {};

        ToolGcc sut;

        EXPECT_TRUE(Tool::not_recognized(sut.recognize(input, BuildTarget::LINKER)));
    }

    TEST(ToolGcc, without_linking_simple) {
        Execution input = {
                "/usr/bin/cc",
                {"cc", "-c", "-o", "source.o", "source.c"},
                "/home/user/project",
                {},
        };

        ToolGcc sut;
        EXPECT_TRUE(Tool::recognized_with_error(sut.recognize(input, BuildTarget::LINKER)));
    }

    TEST(ToolGcc, without_linking_with_object_files) {
        Execution input = {
                "/usr/bin/cc",
                {"cc", "-c", "x.o", "x2.o", "-o", "source.o", "source.c"},
                "/home/user/project",
                {},
        };

        ToolGcc sut;
        EXPECT_TRUE(Tool::recognized_with_error(sut.recognize(input, BuildTarget::LINKER)));
    }

    TEST(ToolGcc, linking_with_compilation_one_file) {
        Execution input = {
                "/usr/bin/cc",
                {"cc", "-o", "source", "source.c"},
                "/home/user/project",
                {},
        };
        SemanticPtr expected = SemanticPtr(
                new Link(
                        input.working_dir,
                        input.executable,
                        {"source.c.o"},
                        {fs::path("source.c.o")},
                        {fs::path("source")}
                )
        );

        ToolGcc sut({});

        auto result = sut.recognize(input, BuildTarget::LINKER);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolGcc, linking_with_compilation) {
        Execution input = {
                "/usr/bin/cc",
                {"cc", "source_1.c", "-o", "source", "lib.o",  "-la", "source_2.c"},
                "/home/user/project",
                {},
        };
        SemanticPtr expected = SemanticPtr(
                new Link(
                        input.working_dir,
                        input.executable,
                        {"source_1.c.o", "lib.o",  "-la", "source_2.c.o"},
                        {"source_1.c.o", "lib.o", "source_2.c.o"},
                        {fs::path("source")}
                )
        );

        ToolGcc sut({});

        auto result = sut.recognize(input, BuildTarget::LINKER);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolGcc, linking) {
        Execution input = {
                "/usr/bin/cc",
                {"cc", "-L.", "source_1.o", "uncorrect_lib", "lib.DLL", "source_2.o", "-la"},
                "/home/user/project",
                {},
        };
        SemanticPtr expected = SemanticPtr(
                new Link(
                        input.working_dir,
                        input.executable,
                        {"-L.", "source_1.o", "uncorrect_lib", "lib.DLL", "source_2.o", "-la"},
                        {"source_1.o", "lib.DLL", "source_2.o"},
                        std::nullopt
                )
        );

        ToolGcc sut({});

        auto result = sut.recognize(input, BuildTarget::LINKER);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }
}
