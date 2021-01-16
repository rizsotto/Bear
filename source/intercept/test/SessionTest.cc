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

#include "gmock/gmock.h"
#include "gtest/gtest.h"

#include "collect/Session.h"

namespace {

    struct SessionFixture : ic::Session {
    public:
        MOCK_METHOD(
            rust::Result<std::string>,
            resolve,
            (const std::string& name),
            (const, override));

        MOCK_METHOD(
            (rust::Result<std::map<std::string, std::string>>),
            update,
            ((const std::map<std::string, std::string>&)env),
            (const, override));

        MOCK_METHOD(
            (rust::Result<sys::Process::Builder>),
            supervise,
            (const std::vector<std::string_view>& command),
            (const, override));

        using Session::keep_front_in_path;
        using Session::remove_from_path;
    };

    TEST(session, remove_from_path)
    {
        EXPECT_EQ("",
                  SessionFixture::remove_from_path("/opt", ""));

        EXPECT_EQ("",
                  SessionFixture::remove_from_path("/opt", "/opt"));
        EXPECT_EQ("",
                  SessionFixture::remove_from_path("/opt", "/opt:/opt"));

        EXPECT_EQ("/usr/bin:/usr/local/bin",
                  SessionFixture::remove_from_path("/opt", "/usr/bin:/usr/local/bin"));
        EXPECT_EQ("/usr/bin:/usr/local/bin",
                  SessionFixture::remove_from_path("/opt", "/opt:/usr/bin:/usr/local/bin"));
        EXPECT_EQ("/usr/bin:/usr/local/bin",
                  SessionFixture::remove_from_path("/opt", "/usr/bin:/opt:/usr/local/bin"));
        EXPECT_EQ("/usr/bin:/usr/local/bin",
                  SessionFixture::remove_from_path("/opt", "/usr/bin:/usr/local/bin:/opt"));
    }

    TEST(session, keep_front_in_path)
    {
        EXPECT_EQ("/opt",
                  SessionFixture::keep_front_in_path("/opt", ""));

        EXPECT_EQ("/opt",
                  SessionFixture::keep_front_in_path("/opt", "/opt"));
        EXPECT_EQ("/opt",
                  SessionFixture::keep_front_in_path("/opt", "/opt:/opt"));

        EXPECT_EQ("/opt:/usr/bin:/usr/local/bin",
                  SessionFixture::keep_front_in_path("/opt", "/usr/bin:/usr/local/bin"));
        EXPECT_EQ("/opt:/usr/bin:/usr/local/bin",
                  SessionFixture::keep_front_in_path("/opt", "/opt:/usr/bin:/usr/local/bin"));
        EXPECT_EQ("/opt:/usr/bin:/usr/local/bin",
                  SessionFixture::keep_front_in_path("/opt", "/usr/bin:/opt:/usr/local/bin"));
        EXPECT_EQ("/opt:/usr/bin:/usr/local/bin",
                  SessionFixture::keep_front_in_path("/opt", "/usr/bin:/usr/local/bin:/opt"));
    }
}