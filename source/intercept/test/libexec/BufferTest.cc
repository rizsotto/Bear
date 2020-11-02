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

#include "report/libexec/Buffer.h"

namespace {

    TEST(Buffer, dont_crash_on_nullptr)
    {
        char buffer[64];
        el::Buffer sut(buffer, buffer + 64);

        EXPECT_EQ(nullptr, sut.store(nullptr));
    }

    TEST(Buffer, stores)
    {
        char buffer[64];
        el::Buffer sut(buffer, buffer + 64);

        const char* literal = "Hi there people";
        EXPECT_STREQ(literal, sut.store(literal));
    }

    TEST(Buffer, not_same_ptr)
    {
        char buffer[64];
        el::Buffer sut(buffer, buffer + 64);

        const char* literal = "Hi there people";
        EXPECT_NE(literal, sut.store(literal));
    }

    TEST(Buffer, works_multiple_times)
    {
        char buffer[64];
        el::Buffer sut(buffer, buffer + 64);

        const char* literal0 = "Hi there people";
        const char* literal1 = "Hallo Leute";

        const char* result0 = sut.store(literal0);
        const char* result1 = sut.store(literal1);

        EXPECT_STREQ(literal0, result0);
        EXPECT_STREQ(literal1, result1);
    }

    TEST(Buffer, handles_size_issue)
    {
        char buffer[8];
        el::Buffer sut(buffer, buffer + 8);

        const char* literal = "Hi there people";

        EXPECT_EQ(nullptr, sut.store(literal));
    }

}
