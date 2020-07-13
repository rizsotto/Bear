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

#include "libreport/Report.h"

#include <iostream>
#include <sstream>

namespace {

    TEST(report, simple_value_serialized_and_read_back)
    {
        report::Report expected = report::Report {
            report::Context { "session", { { "key", "value" } } },
            {
                report::Execution {
                    report::Execution::Command {
                        "/usr/bin/ls",
                        { "ls" },
                        "/home/user",
                        { { "HOME", "/home/user" }, { "PATH", "/usr/bin:/usr/local/bin" } } },
                    report::Execution::Run {
                        42 ,
                        { 12 },
                        {
                            report::Execution::Event {"started", "2020-04-04T07:13:47.027Z", std::nullopt, std::nullopt },
                            report::Execution::Event {"signaled", "2020-04-04T07:13:47.045Z", std::nullopt, { 15 } },
                            report::Execution::Event {"terminated", "2020-04-04T07:13:47.074Z", { 0 }, std::nullopt }
                        }
                    }
                },
                report::Execution {
                    report::Execution::Command {
                        "/usr/bin/ls",
                        { "ls", "-l" },
                        "/home/user",
                        { { "HOME", "/home/user" }, { "PATH", "/usr/bin:/usr/local/bin" } } },
                    report::Execution::Run {
                        43 ,
                        { },
                        {
                            report::Execution::Event {"started", "2020-04-04T07:13:47.027Z", std::nullopt, std::nullopt },
                            report::Execution::Event {"signaled", "2020-04-04T07:13:47.045Z", std::nullopt, { 17 } },
                            report::Execution::Event {"terminated", "2020-04-04T07:13:47.074Z", { 8 }, std::nullopt }
                        }
                    }
                }
            }
        };

        std::stringstream buffer;

        nlohmann::json out = expected;
        buffer << std::setw(4) << out << std::endl;

        nlohmann::json in;
        buffer >> in;

        report::Report result;
        report::from_json(in, result);

        EXPECT_EQ(expected, result);
    }
}