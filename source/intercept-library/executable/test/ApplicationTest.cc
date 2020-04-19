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

#include "Application.h"
#include "ContextMock.h"
#include "er/Flags.h"
#include "libflags/Flags.h"

using ::testing::_;
using ::testing::Args;
using ::testing::ElementsAre;
using ::testing::ElementsAreArray;
using ::testing::Matcher;
using ::testing::NotNull;
using ::testing::Return;

namespace {

    class MockArguments : public ::flags::Arguments {
    public:
        MOCK_METHOD(std::string_view, program, (), (override, const));
        MOCK_METHOD(rust::Result<bool>, as_bool, (const std::string_view& key), (override, const));
        MOCK_METHOD(rust::Result<std::string_view>, as_string, (const std::string_view& key), (override, const));
        MOCK_METHOD(rust::Result<std::vector<std::string_view>>, as_string_list, (const std::string_view& key), (override, const));
    };

    TEST(application, create_fails_if_no_command)
    {
        MockArguments arguments;
        EXPECT_CALL(arguments, as_string(std::string_view(::er::flags::DESTINATION)))
            .WillOnce(Return(rust::Result<std::string_view>(rust::Ok(std::string_view("")))));
        EXPECT_CALL(arguments, as_string(std::string_view(::er::flags::EXECUTE)))
            .WillOnce(Return(rust::Result<std::string_view>(rust::Ok(std::string_view("")))));
        EXPECT_CALL(arguments, as_string_list(std::string_view(::er::flags::COMMAND)))
            .WillOnce(Return(rust::Result<std::vector<std::string_view>>(rust::Err(std::runtime_error("")))));

        ContextMock ctx;
        EXPECT_CALL(ctx, get_environment)
            .WillOnce(Return(std::map<std::string, std::string>()));
        EXPECT_CALL(ctx, get_cwd)
            .WillOnce(Return(rust::Result<std::string>(rust::Ok(std::string("/path")))));

        auto result = er::Application::create(arguments, ctx);

        ASSERT_FALSE(result.is_ok());
    }

    TEST(application, create_success)
    {
        const std::vector<std::string_view> command = { "ls", "-l", "-a" };
        MockArguments arguments;
        EXPECT_CALL(arguments, as_string(std::string_view(::er::flags::DESTINATION)))
            .WillOnce(Return(rust::Result<std::string_view>(rust::Ok(std::string_view("/destdir")))));
        EXPECT_CALL(arguments, as_string(std::string_view(::er::flags::EXECUTE)))
            .WillOnce(Return(rust::Result<std::string_view>(rust::Ok(std::string_view("/bin/ls")))));
        EXPECT_CALL(arguments, as_string_list(std::string_view(::er::flags::COMMAND)))
            .WillOnce(Return(rust::Result<std::vector<std::string_view>>(rust::Ok(command))));

        ContextMock ctx;
        EXPECT_CALL(ctx, get_environment)
            .WillOnce(Return(std::map<std::string, std::string>()));
        EXPECT_CALL(ctx, get_cwd)
            .WillOnce(Return(rust::Result<std::string>(rust::Ok(std::string("/path")))));

        auto result = er::Application::create(arguments, ctx);

        ASSERT_TRUE(result.is_ok());
    }
}