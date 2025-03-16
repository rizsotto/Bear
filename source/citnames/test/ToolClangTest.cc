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
#include "semantic/ToolClang.h"

using namespace cs::semantic;

namespace {

    TEST(ToolClang, is_compiler_call) {
        struct Expose : public ToolClang {
            using ToolClang::is_compiler_call;
        };
        Expose sut;

        EXPECT_TRUE(sut.is_compiler_call("clang"));
        EXPECT_TRUE(sut.is_compiler_call("/usr/bin/clang"));
        EXPECT_TRUE(sut.is_compiler_call("clang++"));
        EXPECT_TRUE(sut.is_compiler_call("/usr/bin/clang++"));
        EXPECT_TRUE(sut.is_compiler_call("clang-6"));
        EXPECT_TRUE(sut.is_compiler_call("clang6"));
        EXPECT_TRUE(sut.is_compiler_call("clang-8.1"));
        EXPECT_TRUE(sut.is_compiler_call("clang8.1"));
        EXPECT_TRUE(sut.is_compiler_call("clang81"));
        EXPECT_TRUE(sut.is_compiler_call("flang"));
        EXPECT_TRUE(sut.is_compiler_call("flang-20"));
        EXPECT_TRUE(sut.is_compiler_call("flang-new"));
        EXPECT_TRUE(sut.is_compiler_call("flang-new-18"));
    }

    TEST(ToolClang, simple) {
        const Execution input = {
                "/usr/bin/clang",
                {"clang", "-c", "-o", "source.o", "source.c"},
                "/home/user/project",
                {},
        };
        const Compile expected(
                input.working_dir,
                input.executable,
                {"-c"},
                {fs::path("source.c")},
                {fs::path("source.o")}
        );

        ToolClang sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_EQ(expected, *(result.unwrap().get()));
    }

    TEST(ToolClang, linker_flag_filtered) {
        const Execution input = {
                "/usr/bin/clang",
                {"clang", "-L.", "-lthing", "-o", "exe", "source.c"},
                "/home/user/project",
                {},
        };
        const Compile expected(
                input.working_dir,
                input.executable,
                {"-c"},
                {fs::path("source.c")},
                {fs::path("exe")}
        );

        ToolClang sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_EQ(expected, *(result.unwrap().get()));
    }

    TEST(ToolClang, pass_on_version) {
        const Execution input = {
                "/usr/bin/clang",
                {"clang", "--version"},
                "/home/user/project",
                {},
        };
        const QueryCompiler expected;

        ToolClang sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_EQ(expected, *(result.unwrap().get()));
    }

    TEST(ToolClang, pass_on_Xclang) {
        const Execution input = {
                "/usr/bin/clang",
                {
                        "clang",
                        "-c",
                        "-o",
                        "source.o",
                        "source.c",
                        "-Xclang",
                        "-load",
                        "-Xclang",
                        "/path/to/LLVMHello.so"
                },
                "/home/user/project",
                {},
        };
        const Compile expected(
                input.working_dir,
                input.executable,
                {"-c", "-Xclang", "-load", "-Xclang", "/path/to/LLVMHello.so"},
                {fs::path("source.c")},
                {fs::path("source.o")}
        );

        ToolClang sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_EQ(expected, *(result.unwrap().get()));
    }

    TEST(ToolClang, pass_on_Xarch) {
        const Execution input = {
                "/usr/bin/clang",
                {
                        "clang",
                        "-c",
                        "-o",
                        "source.o",
                        "source.c",
                        "-Xarch_arg1",
                        "arg2",
                        "-Xarch_device",
                        "device1",
                        "-Xarch_host",
                        "host1"
                },
                "/home/user/project",
                {},
        };
        const Compile expected(
                input.working_dir,
                input.executable,
                {"-c", "-Xarch_arg1", "arg2", "-Xarch_device", "device1", "-Xarch_host", "host1"},
                {fs::path("source.c")},
                {fs::path("source.o")}
        );

        ToolClang sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_EQ(expected, *(result.unwrap().get()));
    }

    TEST(ToolClang, pass_on_Xcuda) {
        const Execution input = {
                "/usr/bin/clang",
                {
                        "clang",
                        "-c",
                        "-o",
                        "source.o",
                        "source.c",
                        "-Xcuda-fatbinary",
                        "arg1",
                        "-Xcuda-ptxas",
                        "arg2"
                },
                "/home/user/project",
                {},
        };
        const Compile expected(
                input.working_dir,
                input.executable,
                {"-c", "-Xcuda-fatbinary", "arg1", "-Xcuda-ptxas", "arg2"},
                {fs::path("source.c")},
                {fs::path("source.o")}
        );

        ToolClang sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_EQ(expected, *(result.unwrap().get()));
    }

    TEST(ToolClang, pass_on_Xopenmp) {
        const Execution input = {
                "/usr/bin/clang",
                {
                        "clang",
                        "-c",
                        "-o",
                        "source.o",
                        "source.c",
                        "-Xopenmp-target",
                        "arg1",
                        "-Xopenmp-target=arg1",
                        "arg2"
                },
                "/home/user/project",
                {},
        };
        const Compile expected(
                input.working_dir,
                input.executable,
                {"-c", "-Xopenmp-target", "arg1", "-Xopenmp-target=arg1", "arg2"},
                {fs::path("source.c")},
                {fs::path("source.o")}
        );

        ToolClang sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_EQ(expected, *(result.unwrap().get()));
    }

    TEST(ToolClang, pass_on_analyze) {
        const Execution input = {
                "/usr/bin/clang",
                {
                        "clang",
                        "-c",
                        "-o",
                        "source.o",
                        "source.c",
                        "-Z",
                        "arg1",
                        "-aargs",
                        "--analyze"
                },
                "/home/user/project",
                {},
        };
        const Compile expected(
                input.working_dir,
                input.executable,
                {"-c", "-Z", "arg1", "-aargs", "--analyze"},
                {fs::path("source.c")},
                {fs::path("source.o")}
        );

        ToolClang sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_EQ(expected, *(result.unwrap().get()));
    }

    TEST(ToolClang, pass_on_fintrinsic_modules_path) {
        const Execution input = {
                "/usr/bin/flang",
                {
                        "flang",
                        "-c",
                        "-o",
                        "source.o",
                        "source.f90",
                        "-fintrinsic-modules-path",
                        "arg1",
                },
                "/home/user/project",
                {},
        };
        const Compile expected(
                input.working_dir,
                input.executable,
                {"-c", "-fintrinsic-modules-path", "arg1"},
                {fs::path("source.f90")},
                {fs::path("source.o")}
        );

        ToolClang sut({});

        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_EQ(expected, *(result.unwrap().get()));
    }
}
