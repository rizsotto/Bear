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

#include "Flags.h"

using namespace flags;

namespace {

    constexpr char HELP[] = "--help";
    constexpr char FLAG[] = "--flag";
    constexpr char OPTION[] = "--option";
    constexpr char OPTIONS[] = "--options";
    constexpr char SEPARATOR[] = "--";

    TEST(flags, parse_successful)
    {
        const char* argv[] = { "executable", FLAG, OPTION, "0", OPTIONS, "1", "2", "3", SEPARATOR, "4", "5" };
        const int argc = sizeof(argv) / sizeof(const char*);

        const Parser parser("test",
            { { HELP, { 0, false, "this message", std::nullopt } },
                { FLAG, { 0, false, "a single flag", std::nullopt } },
                { OPTION, { 1, false, "a flag with a value", std::nullopt } },
                { OPTIONS, { 3, false, "a flag with 3 values", std::nullopt } },
                { SEPARATOR, { -1, false, "rest of the arguments", std::nullopt } } });
        parser.parse(argc, const_cast<const char**>(argv))
            .map<int>([](auto params) {
                EXPECT_STREQ(params.program().data(), "executable");

                EXPECT_TRUE(params.as_bool(HELP).is_ok());
                EXPECT_FALSE(params.as_bool(HELP).unwrap_or(true));

                EXPECT_TRUE(params.as_bool(FLAG).is_ok());
                EXPECT_TRUE(params.as_bool(FLAG).unwrap_or(true));

                auto option = params.as_string(OPTION);
                EXPECT_TRUE(option.is_ok());
                EXPECT_STREQ(option.unwrap_or("").data(), "0");

                auto option_int = params.as_int(OPTION);
                EXPECT_TRUE(option_int.is_ok());
                EXPECT_EQ(option_int.unwrap_or(2), 0);

                std::vector<std::string_view> expected_options = { "1", "2", "3" };
                auto options = params.as_string_list(OPTIONS);
                EXPECT_TRUE(options.is_ok());
                EXPECT_EQ(expected_options, options.unwrap_or({}));

                std::vector<std::string_view> expected_separator = { "4", "5" };
                auto separator = params.as_string_list(SEPARATOR);
                EXPECT_TRUE(separator.is_ok());
                EXPECT_EQ(expected_separator, separator.unwrap_or({}));
                return 0;
            })
            .map_err<int>([](auto error) {
                EXPECT_FALSE(true);
                return 0;
            });
    }

    TEST(flags, parse_fails_for_unkown_flags)
    {
        const char* argv[] = { "executable", FLAG, OPTION, "0" };
        const int argc = sizeof(argv) / sizeof(const char*);

        const Parser parser("test",
            { { HELP, { 0, false, "this message", std::nullopt } },
                { FLAG, { 0, false, "a single flag", std::nullopt } } });
        parser.parse(argc, const_cast<const char**>(argv))
            .map<int>([](auto params) {
                EXPECT_FALSE(true);
                return 0;
            })
            .map_err<int>([](auto error) {
                EXPECT_TRUE(true);
                EXPECT_STREQ(error.what(), "Unrecognized parameter: --option");
                return 0;
            });
    }

    TEST(flags, parse_fails_for_not_enough_params)
    {
        const char* argv[] = { "executable", FLAG, OPTIONS, "1" };
        const int argc = sizeof(argv) / sizeof(const char*);

        const Parser parser("test",
            { { HELP, { 0, false, "this message", std::nullopt } },
                { FLAG, { 0, false, "a single flag", std::nullopt } },
                { OPTIONS, { 3, false, "a flag with 3 values", std::nullopt } } });
        parser.parse(argc, const_cast<const char**>(argv))
            .map<int>([](auto params) {
                EXPECT_FALSE(true);
                return 0;
            })
            .map_err<int>([](auto error) {
                EXPECT_TRUE(true);
                EXPECT_STREQ(error.what(), "Not enough parameters for flag: --options");
                return 0;
            });
    }

    TEST(flags, parse_help)
    {
//        const std::string expected = "Usage: thing [OPTION]\n"
//                                     "\n"
//                                     "  --flag                 a single flag\n"
//                                     "  --help                 this message\n";
//        const Parser parser({ { HELP, { 0, "this message" } },
//            { FLAG, { 0, "a single flag" } } });
//        const std::string help = parser.help("thing");
//
//        EXPECT_EQ(help, expected);
    }
}
