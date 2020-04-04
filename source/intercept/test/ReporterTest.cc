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

#include "Reporter.h"

namespace ic {

    bool operator==(const Execution::Command& lhs, const Execution::Command& rhs)
    {
        return (lhs.program == rhs.program)
            && (lhs.arguments == rhs.arguments)
            && (lhs.working_dir == rhs.working_dir)
            && (lhs.environment == rhs.environment);
    }

    bool operator==(const Execution::Event& lhs, const Execution::Event& rhs)
    {
        return (lhs.at == rhs.at)
            && (lhs.type == rhs.type)
            && (lhs.status == rhs.status)
            && (lhs.signal == rhs.signal);
    }

    bool operator==(const Execution::Run& lhs, const Execution::Run& rhs)
    {
        return (lhs.pid == rhs.pid)
            && (lhs.ppid == rhs.ppid)
            && (lhs.events == rhs.events);
    }

    bool operator==(const Execution& lhs, const Execution& rhs)
    {
        return (lhs.command == rhs.command)
            && (lhs.run == rhs.run);
    }
}

namespace {

    supervise::Event start_event()
    {
        auto inner = new supervise::Event_Started();
        inner->set_pid(42);
        inner->set_ppid(12);
        inner->set_executable("/usr/bin/ls");
        inner->add_arguments("ls");
        inner->add_arguments("-l");
        inner->set_working_dir("/home/user");
        inner->mutable_environment()->insert({ "HOME", "/home/user" });
        inner->mutable_environment()->insert({ "PATH", "/usr/bin:/usr/local/bin" });

        auto result = supervise::Event();
        result.set_timestamp("2020-04-04T07:13:47.027Z");
        result.set_allocated_started(inner);

        return result;
    }

    supervise::Event signal_event()
    {
        auto inner = new supervise::Event_Signalled();
        inner->set_number(15);

        auto result = supervise::Event();
        result.set_timestamp("2020-04-04T07:13:47.045Z");
        result.set_allocated_signalled(inner);

        return result;
    }

    supervise::Event stop_event()
    {
        auto inner = new supervise::Event_Stopped();
        inner->set_status(0);

        auto result = supervise::Event();
        result.set_timestamp("2020-04-04T07:13:47.074Z");
        result.set_allocated_stopped(inner);

        return result;
    }

    TEST(reporter, builder_makes_empty_execution_object)
    {
        ic::Execution::Builder sut;
        ic::Execution::UniquePtr result = sut.build();

        EXPECT_EQ(result.get(), nullptr);
    }

    TEST(reporter, builder_makes_empty_object_without_start_event)
    {
        ic::Execution::Builder sut;
        sut.add(signal_event());
        sut.add(stop_event());
        ic::Execution::UniquePtr result = sut.build();

        EXPECT_EQ(result.get(), nullptr);
    }

    TEST(reporter, builder_makes_execution_object_from_events)
    {
        ic::Execution::Builder sut;
        sut.add(start_event());
        sut.add(signal_event());
        sut.add(stop_event());
        ic::Execution::UniquePtr result = sut.build();

        ic::Execution expected = ic::Execution {
            ic::Execution::Command {
                "/usr/bin/ls",
                { "ls", "-l" },
                "/home/user",
                { {"HOME", "/home/user"}, {"PATH", "/usr/bin:/usr/local/bin"} }
            },
            ic::Execution::Run {
                { 42 },
                { 12 },
                {
                    ic::Execution::Event {
                        "start", "2020-04-04T07:13:47.027Z", std::nullopt, std::nullopt
                    },
                    ic::Execution::Event {
                        "signal", "2020-04-04T07:13:47.045Z", std::nullopt, { 15 }
                    },
                    ic::Execution::Event {
                        "stop", "2020-04-04T07:13:47.074Z", { 0 }, std::nullopt
                    }
                }
            }
        };
        EXPECT_EQ(*result, expected);
    }
}
