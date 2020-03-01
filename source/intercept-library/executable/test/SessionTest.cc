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
#include "flags.h"

namespace {

    TEST(session, parse_empty_fails)
    {
        const char* argv[] = {
            "program",
            nullptr
        };
        const int argc = sizeof(argv) / sizeof(char*) - 1;

        ::er::Result<::er::SessionPtr> const result = ::er::parse(argc, const_cast<char**>(argv));
        ::er::SessionPtr const expected = ::er::SessionPtr(nullptr);

        ASSERT_EQ(expected, result.get_or_else(expected));
    }

    TEST(session, parse_help_fails)
    {
        const char* argv[] = {
            "program",
            ::er::flags::HELP,
            nullptr
        };
        const int argc = sizeof(argv) / sizeof(char*) - 1;

        ::er::Result<::er::SessionPtr> const result = ::er::parse(argc, const_cast<char**>(argv));
        ::er::SessionPtr const expected = ::er::SessionPtr(nullptr);

        ASSERT_EQ(expected, result.get_or_else(expected));
    }

    TEST(session, parse_library_success)
    {
        const char* argv[] = {
            "program",
            er::flags::LIBRARY, "/install/path/libexec.so",
            er::flags::DESTINATION, "/tmp/destination",
            er::flags::VERBOSE,
            er::flags::EXECUTE, "/bin/ls",
            er::flags::COMMAND, "ls", "-l", "-a",
            nullptr
        };
        const int argc = sizeof(argv) / sizeof(char*) - 1;

        ::er::Result<::er::SessionPtr> const result = ::er::parse(argc, const_cast<char**>(argv));
        ::er::SessionPtr const dummy = ::er::SessionPtr(nullptr);
        ASSERT_NE(dummy, result.get_or_else(dummy));
        auto session_result = (::er::LibrarySession const*)result.get_or_else(dummy).get();

        ASSERT_STREQ(argv[0], session_result->context_.reporter);
        ASSERT_STREQ(argv[4], session_result->context_.destination);
        ASSERT_EQ(true, session_result->context_.verbose);

        ASSERT_EQ(argv + 9, session_result->execution_.command);
        ASSERT_EQ(argv[7], session_result->execution_.path);

        ASSERT_EQ(argv[2], session_result->library);
    }
}