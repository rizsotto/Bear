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

#include "Configuration.h"
#include "Output.h"
#include "semantic/Tool.h"

namespace {

//    TEST(Tools, parses_empty_command_list)
//    {
//        cs::Configuration cfg;
//
//        auto sut = cs::semantic::Tools::from(cfg.compilation);
//        EXPECT_TRUE(sut.is_ok());
//
//        auto input = report::Report {
//                report::Context { "session", {} },
//                {}
//        };
//        auto result = sut.map<cs::Entries>([&input](auto semantic) {
//            return semantic.transform(input);
//        });
//        EXPECT_TRUE(result.is_ok());
//    }
//
//    TEST(Tools, parses_command_list)
//    {
//        cs::Configuration cfg;
//
//        auto sut = cs::semantic::Tools::from(cfg.compilation);
//        EXPECT_TRUE(sut.is_ok());
//
//        auto input = report::Report {
//                report::Context { "session", {} },
//                {
//                        report::Execution {
//                                report::Execution {
//                                        "/usr/bin/cc",
//                                        { "cc", "--version" },
//                                        "/home/user/project",
//                                        {}
//                                },
//                                report::Run { 1, 0, {} }
//                        },
//                        report::Execution {
//                                report::Execution {
//                                        "/usr/bin/ls",
//                                        { "ls", "-la" },
//                                        "/home/user/project",
//                                        {}
//                                },
//                                report::Run { 2, 0, {} }
//                        },
//                        report::Execution {
//                                report::Execution {
//                                        "/usr/bin/cc",
//                                        { "cc", "-c", "-Wall", "source.1.c" },
//                                        "/home/user/project",
//                                        {}
//                                },
//                                report::Run { 3, 0, {} }
//                        },
//                        report::Execution {
//                                report::Execution {
//                                        "/usr/bin/c++",
//                                        { "c++", "-c", "-Wall", "source.2.cc" },
//                                        "/home/user/project",
//                                        {}
//                                },
//                                report::Run { 4, 0, {} }
//                        },
//                }
//        };
//        auto result = sut.map<cs::Entries>([&input](auto semantic) {
//            return semantic.transform(input);
//        });
//        EXPECT_TRUE(result.is_ok());
//
//        cs::Entries expected = {
//                cs::Entry{
//                        "/home/user/project/source.1.c",
//                        "/home/user/project",
//                        {"/home/user/project/source.1.o"},
//                        {"/usr/bin/cc", "-c", "-Wall", "source.1.c"}
//                },
//                cs::Entry{
//                        "/home/user/project/source.2.cc",
//                        "/home/user/project",
//                        {"/home/user/project/source.2.o"},
//                        {"/usr/bin/c++", "-c", "-Wall", "source.2.cc"}
//                },
//        };
//        auto compilations = result.unwrap_or({});
//        EXPECT_EQ(expected, compilations);
//    }
//
//    TEST(Tools, child_commands_are_ignored)
//    {
//        cs::Configuration cfg;
//
//        auto sut = cs::semantic::Tools::from(cfg.compilation);
//        EXPECT_TRUE(sut.is_ok());
//
//        auto input = report::Report {
//                report::Context { "session", {} },
//                {
//                        report::Execution {
//                                report::Execution {
//                                        "/usr/bin/nvcc",
//                                        { "cc", "-c", "source.cu" },
//                                        "/home/user/project",
//                                        {}
//                                },
//                                report::Run { 1, 0, {} }
//                        },
//                        report::Execution {
//                                report::Execution {
//                                        "/usr/bin/gcc",
//                                        { "cc", "-E", "source.cu" },
//                                        "/home/user/project",
//                                        {}
//                                },
//                                report::Run { 2, 1, {} }
//                        },
//                        report::Execution {
//                                report::Execution {
//                                        "/usr/bin/gcc",
//                                        { "cc", "-c", "-Dthing", "source.cu" },
//                                        "/home/user/project",
//                                        {}
//                                },
//                                report::Run { 3, 1, {} }
//                        },
//                }
//        };
//        auto result = sut.map<cs::Entries>([&input](auto semantic) {
//            return semantic.transform(input);
//        });
//        EXPECT_TRUE(result.is_ok());
//
//        cs::Entries expected = {
//                cs::Entry{
//                        "/home/user/project/source.cu",
//                        "/home/user/project",
//                        {"/home/user/project/source.o"},
//                        {"/usr/bin/nvcc", "-c", "source.cu"}
//                },
//        };
//        auto compilations = result.unwrap_or({});
//        EXPECT_EQ(expected, compilations);
//    }
}
