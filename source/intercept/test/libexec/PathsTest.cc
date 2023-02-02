/*  Copyright (C) 2012-2023 by László Nagy
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

#include "report/libexec/Paths.h"

namespace {

    TEST(PathIterator, works_on_empty)
    {
        el::Paths paths("");
        for (auto path : paths) {
            EXPECT_EQ(std::string_view("shall not match"), path);
        }
    }

    TEST(PathIterator, works_on_single)
    {
        el::Paths paths("/bin");
        for (auto path : paths) {
            EXPECT_EQ(path, std::string_view("/bin"));
        }
    }

    TEST(PathIterator, works_on_multiple)
    {
        size_t count = 0;

        el::Paths paths("/bin:/sbin:/usr/bin:/usr/sbin");
        for (auto path : paths) {
            EXPECT_FALSE(path.empty());
            ++count;
        }
        EXPECT_EQ(4, count);

        el::Paths::Iterator it = paths.begin();
        el::Paths::Iterator end = paths.end();
        EXPECT_NE(it, end);
        EXPECT_EQ(std::string_view("/bin"), *(it++));
        EXPECT_NE(it, end);
        EXPECT_EQ(std::string_view("/sbin"), *it); it++;
        EXPECT_NE(it, end);
        EXPECT_EQ(std::string_view("/usr/bin"), *it); ++it;
        EXPECT_NE(it, end);
        EXPECT_EQ(std::string_view("/usr/sbin"), *it); ++it;
        EXPECT_EQ(it, end);
    }

    TEST(PathIterator, works_with_empty_values)
    {
        size_t count = 0;
        size_t empty = 0;

        el::Paths paths("/bin::/sbin::");
        for (auto path : paths) {
            ++count;
            empty += path.empty() ? 1 : 0;
        }
        EXPECT_EQ(4, count);
        EXPECT_EQ(2, empty);
    }
}