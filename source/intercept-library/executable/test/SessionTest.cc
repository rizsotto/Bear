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
#include "intercept.h"

namespace {

    TEST(session, parse_empty_fails)
    {
        const char* argv[] = {
            "program",
            nullptr
        };
        const int argc = sizeof(argv) / sizeof(char*) - 1;

        ::pear::Result<::pear::SessionPtr> const result = ::pear::parse(argc, const_cast<char**>(argv));
        ::pear::SessionPtr const expected = ::pear::SessionPtr(nullptr);

        ASSERT_EQ(expected, result.get_or_else(expected));
    }

    TEST(session, parse_help_fails)
    {
        const char* argv[] = {
            "program",
            ::pear::flag::HELP,
            nullptr
        };
        const int argc = sizeof(argv) / sizeof(char*) - 1;

        ::pear::Result<::pear::SessionPtr> const result = ::pear::parse(argc, const_cast<char**>(argv));
        ::pear::SessionPtr const expected = ::pear::SessionPtr(nullptr);

        ASSERT_EQ(expected, result.get_or_else(expected));
    }

    TEST(session, parse_library_success)
    {
        const char* argv[] = {
            "program",
            pear::flag::LIBRARY, "/install/path/libexec.so",
            pear::flag::DESTINATION, "/tmp/destination",
            pear::flag::VERBOSE,
            pear::flag::PATH, "/bin/ls",
            pear::flag::COMMAND, "ls", "-l", "-a",
            nullptr
        };
        const int argc = sizeof(argv) / sizeof(char*) - 1;

        ::pear::Result<::pear::SessionPtr> const result = ::pear::parse(argc, const_cast<char**>(argv));
        ::pear::SessionPtr const dummy = ::pear::SessionPtr(nullptr);
        ASSERT_NE(dummy, result.get_or_else(dummy));
        auto session_result = (::pear::LibrarySession const*)result.get_or_else(dummy).get();

        ASSERT_STREQ(argv[0], session_result->context_.reporter);
        ASSERT_STREQ(argv[4], session_result->context_.destination);
        ASSERT_EQ(true, session_result->context_.verbose);

        ASSERT_EQ(argv + 9, session_result->execution_.command);
        ASSERT_EQ(argv[7], session_result->execution_.path);

        ASSERT_EQ(argv[2], session_result->library);
    }
}