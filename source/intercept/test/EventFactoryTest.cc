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

#include "report/wrapper/EventFactory.h"

namespace {

    const wr::ProcessId TEST_PID = 11;
    const wr::ProcessId TEST_PPID = 10;
    const wr::Execution TEST_EXECUTION = wr::Execution {
            fs::path("/usr/bin/ls"),
            {"ls", "-la"},
            fs::path("/home/user"),
            { {"PATH", "/usr/bin:/usr/sbin"} }
    };

    TEST(event_factory, same_factory_creates_events_with_same_id)
    {
        wr::EventFactory sut;
        auto start = sut.start(TEST_PID, TEST_PPID, TEST_EXECUTION);
        auto signal = sut.signal(11);
        auto stop = sut.terminate(5);

        EXPECT_EQ(start.rid(), signal.rid());
        EXPECT_EQ(start.rid(), stop.rid());
    }

    TEST(event_factory, different_factory_creates_event_with_different_id)
    {
        wr::EventFactory sut1;
        auto start1 = sut1.start(TEST_PID, TEST_PPID, TEST_EXECUTION);

        wr::EventFactory sut2;
        auto start2 = sut2.start(TEST_PID, TEST_PPID, TEST_EXECUTION);

        EXPECT_NE(start1.rid(), start2.rid());
    }
}
