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

#include "semantic/ToolCrayFtnfe.h"

using namespace cs::semantic;

namespace {

    TEST(ToolCrayFtnfe, is_compiler_call)
    {
        struct Expose : public ToolCrayFtnfe {
            using ToolCrayFtnfe::is_compiler_call;
        };
        Expose sut;
        EXPECT_TRUE(sut.is_compiler_call("ftnfe"));
        EXPECT_TRUE(sut.is_compiler_call("/usr/bin/ftnfe"));
        EXPECT_TRUE(sut.is_compiler_call("/opt/cray/pe/cce/18.0.0/cce/x86_64/bin/ftnfe"));
        EXPECT_FALSE(sut.is_compiler_call("gfortran"));
        EXPECT_FALSE(sut.is_compiler_call("gcc"));
        // `crayftn` and `ftn` are not the real Cray Fortran compiler. `crayftn`
        // and `ftn` are generic drivers that may call other compilers depending
        // on the configuration of the system. The real Cray Fortran compiler is
        // `ftnfe`!
        EXPECT_FALSE(sut.is_compiler_call("/opt/cray/pe/cce/18.0.0/bin/crayftn"));
        EXPECT_FALSE(sut.is_compiler_call("/opt/cray/pe/craype/2.7.32/bin/ftn"));
        EXPECT_FALSE(sut.is_compiler_call("crayftn"));
        EXPECT_FALSE(sut.is_compiler_call("ftn"));
    }

    TEST(ToolCrayFtnfe, fails_on_empty)
    {
        ToolCrayFtnfe sut;
        EXPECT_TRUE(Tool::not_recognized(sut.recognize(Execution {})));
    }

    TEST(ToolCrayFtnfe, simple)
    {
        Execution input = {
            "/opt/cray/pe/cce/18.0.0/cce/x86_64/bin/ftnfe",
            { "ftnfe", "-b", "source_out.o", "-r", "file.listing", "source.c" },
            "/home/user/project",
            {},
        };
        SemanticPtr expected = SemanticPtr(
            new Compile(
                input.working_dir,
                input.executable,
                { "-c", "-r", "file.listing" },
                { fs::path("source.c") },
                { fs::path("source_out.o") }));
        ToolCrayFtnfe sut;
        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }

    TEST(ToolCrayFtnfe, linker_flag_filtered)
    {
        Execution input = {
            "/opt/cray/pe/cce/18.0.0/cce/x86_64/bin/ftnfe",
            { "ftnfe", "-L.", "-lthing", "-o", "exe", "source.c" },
            "/home/user/project",
            {},
        };
        SemanticPtr expected = SemanticPtr(
            new Compile(
                input.working_dir,
                input.executable,
                { "-c" },
                { fs::path("source.c") },
                { fs::path("exe") }));
        ToolCrayFtnfe sut;
        auto result = sut.recognize(input);
        EXPECT_TRUE(Tool::recognized_ok(result));
        EXPECT_PRED2([](auto lhs, auto rhs) { return lhs->operator==(*rhs); }, expected, result.unwrap());
    }
}
