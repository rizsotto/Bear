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
#include "Environment.h"

namespace {

    TEST(Environment, dont_crash_on_nullptr) {
        const auto result = ear::Session::from(nullptr);
        ASSERT_TRUE(result.is_not_valid());
    }

    TEST(Environment, capture_on_empty) {
        const char *envp[] = { "this=is", "these=are", nullptr };

        const auto result = ear::Session::from(envp);
        ASSERT_TRUE(result.is_not_valid());
    }

    TEST(Environment, capture_silent) {
        const char *envp[] = {
                "INTERCEPT_LIBRARY=/usr/libexec/libexec.so",
                "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
                "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
                nullptr
        };

        const auto result = ear::Session::from(envp);
        ASSERT_FALSE(result.is_not_valid());

        EXPECT_STREQ("/tmp/intercept.random", result.get_destination());
        EXPECT_STREQ("/usr/libexec/libexec.so", result.get_library());
        EXPECT_STREQ("/usr/bin/intercept", result.get_reporter());
        EXPECT_EQ(false, result.is_verbose());
    }

    TEST(Environment, capture_verbose) {
        const char *envp[] = {
                "INTERCEPT_LIBRARY=/usr/libexec/libexec.so",
                "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
                "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
                "INTERCEPT_VERBOSE=true",
                nullptr
        };

        const auto result = ear::Session::from(envp);
        ASSERT_FALSE(result.is_not_valid());

        EXPECT_STREQ("/tmp/intercept.random", result.get_destination());
        EXPECT_STREQ("/usr/libexec/libexec.so", result.get_library());
        EXPECT_STREQ("/usr/bin/intercept", result.get_reporter());
        EXPECT_EQ(true, result.is_verbose());
    }

}