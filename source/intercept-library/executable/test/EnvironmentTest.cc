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

#include "Environment.h"

namespace {

    TEST(environment, empty_gets_empty_list)
    {
        ::pear::Environment::Builder builder(nullptr);
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_EQ(nullptr, result[0]);
    }

    TEST(environment, not_empty_says_the_same)
    {
        const char* envp[] = {
            "THIS=that",
            nullptr
        };
        ::pear::Environment::Builder builder(envp);
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("THIS=that", result[0]);
    }

    TEST(environment, reporter_inserted)
    {
        ::pear::Environment::Builder builder(nullptr);
        builder.add_reporter("/usr/libexec/intercept");
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("INTERCEPT_REPORT_COMMAND=/usr/libexec/intercept", result[0]);
    }

    TEST(environment, destination_inserted)
    {
        ::pear::Environment::Builder builder(nullptr);
        builder.add_destination("/tmp/intercept");
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("INTERCEPT_REPORT_DESTINATION=/tmp/intercept", result[0]);
    }

    TEST(environment, verbose_enabled)
    {
        ::pear::Environment::Builder builder(nullptr);
        builder.add_verbose(true);
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("INTERCEPT_VERBOSE=1", result[0]);
    }

    TEST(environment, verbose_disabled)
    {
        ::pear::Environment::Builder builder(nullptr);
        builder.add_verbose(false);
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_EQ(nullptr, result[0]);
    }

#ifdef APPLE
#else
    TEST(environment, empty_library)
    {
        ::pear::Environment::Builder builder(nullptr);
        builder.add_library("/usr/libexec/libexec.so");
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("INTERCEPT_LIBRARY=/usr/libexec/libexec.so", result[0]);
        EXPECT_STREQ("LD_PRELOAD=/usr/libexec/libexec.so", result[1]);
    }

    TEST(environment, library_already_there)
    {
        const char* envp[] = {
            "LD_PRELOAD=/usr/libexec/libexec.so",
            nullptr
        };
        ::pear::Environment::Builder builder(envp);
        builder.add_library("/usr/libexec/libexec.so");
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("INTERCEPT_LIBRARY=/usr/libexec/libexec.so", result[0]);
        EXPECT_STREQ("LD_PRELOAD=/usr/libexec/libexec.so", result[1]);
    }

    TEST(environment, library_already_with_another)
    {
        const char* envp[] = {
            "LD_PRELOAD=/usr/libexec/libexec.so:/usr/libexec/libio.so",
            nullptr
        };
        ::pear::Environment::Builder builder(envp);
        builder.add_library("/usr/libexec/libexec.so");
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("INTERCEPT_LIBRARY=/usr/libexec/libexec.so", result[0]);
        EXPECT_STREQ("LD_PRELOAD=/usr/libexec/libexec.so:/usr/libexec/libio.so", result[1]);
    }

    TEST(environment, another_libray_is_there)
    {
        const char* envp[] = {
            "LD_PRELOAD=/usr/libexec/libio.so",
            nullptr
        };
        ::pear::Environment::Builder builder(envp);
        builder.add_library("/usr/libexec/libexec.so");
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("INTERCEPT_LIBRARY=/usr/libexec/libexec.so", result[0]);
        EXPECT_STREQ("LD_PRELOAD=/usr/libexec/libexec.so:/usr/libexec/libio.so", result[1]);
    }
#endif
}