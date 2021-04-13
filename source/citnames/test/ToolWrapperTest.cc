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
#include "gmock/gmock.h"

#include "semantic/Tool.h"
#include "semantic/ToolWrapper.h"
#include "report/libexec/Resolver.h"

using namespace cs::semantic;
using ::testing::_;
using ::testing::Eq;
using ::testing::Return;

namespace {

    class ResolverMock : public el::Resolver {
    public:
        MOCK_METHOD(
        (rust::Result<const char*, int>),
                from_current_directory,
        (std::string_view const &),
        (override)
        );

        MOCK_METHOD(
        (rust::Result<const char*, int>),
                from_path,
        (std::string_view const &, const char **),
        (override)
        );

        MOCK_METHOD(
        (rust::Result<const char*, int>),
                from_search_path,
        (std::string_view const &, const char *),
        (override)
        );
    };

    TEST(ToolWrapper, is_ccache_call) {
        EXPECT_FALSE(ToolWrapper::is_ccache_call("cc"));
        EXPECT_FALSE(ToolWrapper::is_ccache_call("/usr/bin/cc"));
        EXPECT_FALSE(ToolWrapper::is_ccache_call("gcc"));
        EXPECT_FALSE(ToolWrapper::is_ccache_call("/usr/bin/gcc"));
        EXPECT_FALSE(ToolWrapper::is_ccache_call("c++"));
        EXPECT_FALSE(ToolWrapper::is_ccache_call("/usr/bin/c++"));
        EXPECT_FALSE(ToolWrapper::is_ccache_call("g++"));
        EXPECT_FALSE(ToolWrapper::is_ccache_call("/usr/bin/g++"));

        EXPECT_TRUE(ToolWrapper::is_ccache_call("ccache"));
    }

    TEST(ToolWrapper, is_ccache_query) {
        EXPECT_TRUE(ToolWrapper::is_ccache_query({"ccache"}));
        EXPECT_TRUE(ToolWrapper::is_ccache_query({"ccache", "-c"}));
        EXPECT_TRUE(ToolWrapper::is_ccache_query({"ccache", "--cleanup"}));

        EXPECT_FALSE(ToolWrapper::is_ccache_query({"ccache", "cc", "-c"}));
    }

    TEST(ToolWrapper, is_distcc_call) {
        EXPECT_FALSE(ToolWrapper::is_distcc_call("cc"));
        EXPECT_FALSE(ToolWrapper::is_distcc_call("/usr/bin/cc"));
        EXPECT_FALSE(ToolWrapper::is_distcc_call("gcc"));
        EXPECT_FALSE(ToolWrapper::is_distcc_call("/usr/bin/gcc"));
        EXPECT_FALSE(ToolWrapper::is_distcc_call("c++"));
        EXPECT_FALSE(ToolWrapper::is_distcc_call("/usr/bin/c++"));
        EXPECT_FALSE(ToolWrapper::is_distcc_call("g++"));
        EXPECT_FALSE(ToolWrapper::is_distcc_call("/usr/bin/g++"));

        EXPECT_TRUE(ToolWrapper::is_distcc_call("distcc"));
    }

    TEST(ToolWrapper, is_distcc_query) {
        EXPECT_TRUE(ToolWrapper::is_ccache_query({"distcc"}));
        EXPECT_TRUE(ToolWrapper::is_ccache_query({"distcc", "--help"}));
        EXPECT_TRUE(ToolWrapper::is_ccache_query({"distcc", "--show-hosts"}));
        EXPECT_TRUE(ToolWrapper::is_ccache_query({"distcc", "-j"}));

        EXPECT_FALSE(ToolWrapper::is_ccache_query({"distcc", "cc", "--help"}));
        EXPECT_FALSE(ToolWrapper::is_ccache_query({"distcc", "cc", "-c"}));
    }

    TEST(ToolWrapper, remove_wrapper) {
        const Execution input = {
                "/usr/bin/ccache",
                {"ccache", "cc", "-c", "-o", "source.o", "source.c"},
                "/home/user/project",
                {{"PATH", "/usr/bin:/usr/sbin"}},
        };
        const Execution expected = {
                "/usr/bin/cc",
                {"cc", "-c", "-o", "source.o", "source.c"},
                "/home/user/project",
                {{"PATH", "/usr/bin:/usr/sbin"}},
        };

        ResolverMock resolver;
        EXPECT_CALL(resolver, from_search_path(Eq(std::string_view("cc")), _))
                .Times(1)
                .WillOnce(Return(rust::Result<const char*, int>(rust::Ok("/usr/bin/cc"))));

        auto result = ToolWrapper::remove_wrapper(resolver, input);
        EXPECT_EQ(expected, result);
    }

    TEST(ToolWrapper, remove_wrapper_fails_to_resolve) {
        const Execution input = {
                "/usr/bin/ccache",
                {"ccache", "cc", "-c", "-o", "source.o", "source.c"},
                "/home/user/project",
                {{"PATH", "/usr/bin:/usr/sbin"}},
        };
        const Execution expected = {
                "cc",
                {"cc", "-c", "-o", "source.o", "source.c"},
                "/home/user/project",
                {{"PATH", "/usr/bin:/usr/sbin"}},
        };

        ResolverMock resolver;
        EXPECT_CALL(resolver, from_search_path(Eq(std::string_view("cc")), _))
                .Times(1)
                .WillOnce(Return(rust::Result<const char*, int>(rust::Err(12))));

        auto result = ToolWrapper::remove_wrapper(resolver, input);
        EXPECT_EQ(expected, result);
    }
}
