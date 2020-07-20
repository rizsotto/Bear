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

#include "Config.h"

namespace cs::cfg {

    Configuration default_value() {
        return Configuration{
                cfg::Format {
                    // command as array
                    true,
                    // drop output field
                    false
                },
                cfg::Content {
                    // relative to
                    {},
                    // include only existing source
                    true,
                    // paths to include
                    {},
                    // paths to exclude
                    {}
                },
                cfg::Compilation{
                        cfg::ExpandWrappers{
                            // mpi
                            true,
                            // cuda
                            false,
                            // ccache
                            true,
                            // distcc
                            true
                        },
                        cfg::Compilers{
                            // mpi
                            { R"(^mpi(cc|cxx|CC|c\+\+|fort|f77|f90)$)" },
                            // cuda
                            { "nvcc" },
                            // distcc
                            { "distcc" },
                            // ccache
                            { "ccache" },
                            // cc
                            {
                                // gcc
                                R"(^([^-]*-)*[mg]cc(-?\d+(\.\d+){0,2})?$)",
                                // clang
                                R"(^([^-]*-)*clang(-\d+(\.\d+){0,2})?$)",
                                // intel compiler
                                R"(^(|i)cc$)",
                                // ibm compiler
                                R"(^(g|)xlc$)"
                            },
                            // cxx
                            {
                                // generic
                                R"(^(c\+\+|cxx|CC)$)",
                                // gcc
                                R"(^([^-]*-)*[mg]\+\+(-?\d+(\.\d+){0,2})?$)",
                                // clang
                                R"(^([^-]*-)*clang\+\+(-\d+(\.\d+){0,2})?$)",
                                // intel compiler
                                R"(^icpc$)",
                                // ibm compiler
                                R"(^(g|)xl(C|c\+\+)$)"
                            },
                            // fortran
                            {
                                R"(^([^-]*-)*(gfortran)(-?\d+)$)",
                                R"(^(ifort)$)",
                                R"(^(pg|)(f77|f90|f95|fortran)$)"
                            }
                        },
                        cfg::Sources{
                                {
                                        // object
                                        ".o", ".obj"
                                },
                                {
                                        // C
                                        ".c", ".C",
                                        // C++
                                        ".cc", ".CC", ".c++", ".C++", ".cxx", ".cpp", ".cp",
                                        // ObjectiveC
                                        ".m", ".mi", ".mm", ".mii",
                                        // Assembly
                                        ".s", ".S", ".sx", ".asm",
                                        // Fortran
                                        ".f95", ".F95", ".f90", ".F90", ".f", ".F", ".FOR", ".f77", ".fc", ".for", ".ftn", ".fpp"
                                }
                        },
                        {
                            // preprocessor macros, ignored because would cause duplicate entries in
                            // the output (the only difference would be these flags). this is actual
                            // finding from users, who suffered longer execution time caused by the
                            // duplicates.
                            { "-MD", "", "", false, 0 },
                            { "-MMD", "", "", false, 0 },
                            { "-MG", "", "", false, 0 },
                            { "-MP", "", "", false, 0 },
                            { "-MF", "", "", false, 1 },
                            { "-MT", "", "", false, 1 },
                            { "-MQ", "", "", false, 1 },
                            // linker options, ignored because for compilation database will contain
                            // compilation commands only. so, the compiler would ignore these flags
                            // anyway. the benefit to get rid of them is to make the output more
                            // readable.
                            { "-static", "", "", false, 0 },
                            { "-shared", "", "", false, 0 },
                            { "-s", "", "", false, 0 },
                            { "-rdynamic", "", "", false, 0 },
                            { "-static", "", "", false, 1 },
                            { "", "^-(l|L|Wl,).+", "", true, 1 },
                            { "-u", "", "", false, 1 },
                            { "-z", "", "", false, 1 },
                            { "-T", "", "", false, 1 },
                            { "-Xlinker", "", "", false, 1 },
                            // clang-cl / msvc cl specific flags
                            // consider moving visual studio specific warning flags also in.
                            { "-nologo", "", "", false, 0 },
                            { "-EHsc", "", "", false, 0 },
                            { "-EHa", "", "", false, 0 },
                        }
                }
        };
    }
}