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

#include "Configuration.h"

namespace cs::cfg {

    Value default_value(const std::map<std::string, std::string>& environment) {
        Value value {
                output::Format {
                    // command as array
                    true,
                    // drop output field
                    false
                },
                output::Content {
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
                        {}
//                            // mpi
//                            { R"(^mpi(cc|cxx|CC|c\+\+|fort|f77|f90)$)" },
//                            // cuda
//                            { "nvcc" },
//                            // distcc
//                            { "distcc" },
//                            // ccache
//                            { "ccache" },
//                            // cc
//                            {
//                                // gcc
//                                R"(^([^-]*-)*[mg]cc(-?\d+(\.\d+){0,2})?$)",
//                                // clang
//                                R"(^([^-]*-)*clang(-\d+(\.\d+){0,2})?$)",
//                                // intel compiler
//                                R"(^(|i)cc$)",
//                                // ibm compiler
//                                R"(^(g|)xlc$)"
//                            },
//                            // cxx
//                            {
//                                // generic
//                                R"(^(c\+\+|cxx|CC)$)",
//                                // gcc
//                                R"(^([^-]*-)*[mg]\+\+(-?\d+(\.\d+){0,2})?$)",
//                                // clang
//                                R"(^([^-]*-)*clang\+\+(-\d+(\.\d+){0,2})?$)",
//                                // intel compiler
//                                R"(^icpc$)",
//                                // ibm compiler
//                                R"(^(g|)xl(C|c\+\+)$)"
//                            },
//                            // fortran
//                            {
//                                R"(^([^-]*-)*(gfortran)(-?\d+)$)",
//                                R"(^(ifort)$)",
//                                R"(^(pg|)(f77|f90|f95|fortran)$)"
//                            }
                }
        };

        if (auto it = environment.find("CC"); it != environment.end()) {
            value.compilation.compilers.push_back(it->second);
        }
        if (auto it = environment.find("CXX"); it != environment.end()) {
            value.compilation.compilers.push_back(it->second);
        }
        if (auto it = environment.find("FC"); it != environment.end()) {
            value.compilation.compilers.push_back(it->second);
        }

        return value;
    }
}