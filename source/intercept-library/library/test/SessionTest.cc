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

#include "Session.h"

namespace {

    TEST(Session, dont_crash_on_nullptr)
    {
        ear::Session sut {};
        ear::session::from(sut, nullptr);
        ASSERT_FALSE(ear::session::is_valid(sut));
    }

    TEST(Session, capture_on_empty)
    {
        const char* envp[] = { "this=is", "these=are", nullptr };

        ear::Session sut {};
        ear::session::from(sut, envp);
        ASSERT_FALSE(ear::session::is_valid(sut));
    }

    TEST(Session, capture_silent)
    {
        const char* envp[] = {
            "INTERCEPT_LIBRARY=/usr/libexec/libexec.so",
            "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
            "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
            nullptr
        };

        ear::Session sut {};
        ear::session::from(sut, envp);
        ASSERT_TRUE(ear::session::is_valid(sut));

        EXPECT_STREQ("/tmp/intercept.random", sut.destination);
        EXPECT_STREQ("/usr/libexec/libexec.so", sut.library);
        EXPECT_STREQ("/usr/bin/intercept", sut.reporter);
        EXPECT_EQ(false, sut.verbose);
    }

    TEST(Session, capture_verbose)
    {
        const char* envp[] = {
            "INTERCEPT_LIBRARY=/usr/libexec/libexec.so",
            "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
            "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
            "INTERCEPT_VERBOSE=true",
            nullptr
        };

        ear::Session sut {};
        ear::session::from(sut, envp);
        ASSERT_TRUE(ear::session::is_valid(sut));

        EXPECT_STREQ("/tmp/intercept.random", sut.destination);
        EXPECT_STREQ("/usr/libexec/libexec.so", sut.library);
        EXPECT_STREQ("/usr/bin/intercept", sut.reporter);
        EXPECT_EQ(true, sut.verbose);
    }
}