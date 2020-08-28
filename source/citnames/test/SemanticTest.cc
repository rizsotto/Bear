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

#include "gtest/gtest.h"

#include "Configuration.h"
#include "Output.h"
#include "semantic/Tool.h"

namespace {

    TEST(semantic, default_config_parses)
    {
        std::map<std::string, std::string> env = {
                { "FC", "/path/to/your-fc" },
                { "CC", "/path/to/your-cc" },
                { "CXX", "/path/to/your-cxx" },
        };
        auto cfg = cs::cfg::default_value(env);

        auto sut = cs::Tools::from(cfg.compilation);
        EXPECT_TRUE(sut.is_ok());
    }

    TEST(semantic, parses_empty_command_list)
    {
        auto cfg = cs::cfg::default_value({});

        auto sut = cs::Tools::from(cfg.compilation);
        EXPECT_TRUE(sut.is_ok());

        auto input = report::Report {
                report::Context { "session", {} },
                {}
        };
        auto result = sut.map<cs::output::Entries>([&input](auto semantic) {
            return semantic.transform(input);
        });
        EXPECT_TRUE(result.is_ok());
    }

    TEST(semantic, parses_command_list)
    {
        auto cfg = cs::cfg::default_value({});

        auto sut = cs::Tools::from(cfg.compilation);
        EXPECT_TRUE(sut.is_ok());

        auto input = report::Report {
                report::Context { "session", {} },
                {
                        report::Execution {
                                report::Command {
                                        "/usr/bin/cc",
                                        { "cc", "--version" },
                                        "/home/user/project",
                                        {}
                                },
                                report::Run { 1, std::nullopt, {} }
                        },
                        report::Execution {
                                report::Command {
                                        "/usr/bin/ls",
                                        { "ls", "-la" },
                                        "/home/user/project",
                                        {}
                                },
                                report::Run { 1, std::nullopt, {} }
                        },
                        report::Execution {
                                report::Command {
                                        "/usr/bin/cc",
                                        { "cc", "-c", "-Wall", "source.c" },
                                        "/home/user/project",
                                        {}
                                },
                                report::Run { 1, std::nullopt, {} }
                        },
                        report::Execution {
                                report::Command {
                                        "/usr/bin/c++",
                                        { "c++", "-c", "-Wall", "source.cc" },
                                        "/home/user/project",
                                        {}
                                },
                                report::Run { 1, std::nullopt, {} }
                        },
                }
        };
        auto result = sut.map<cs::output::Entries>([&input](auto semantic) {
            return semantic.transform(input);
        });
        EXPECT_TRUE(result.is_ok());

        cs::output::Entries expected = {
                cs::output::Entry{
                        "/home/user/project/source.c",
                        "/home/user/project",
                        {},
                        {"/usr/bin/cc", "-c", "-Wall", "source.c"}
                },
                cs::output::Entry{
                        "/home/user/project/source.cc",
                        "/home/user/project",
                        {},
                        {"/usr/bin/c++", "-c", "-Wall", "source.cc"}
                }
        };
        auto compilations = result.unwrap_or({});
        EXPECT_EQ(expected, compilations);
    }
}