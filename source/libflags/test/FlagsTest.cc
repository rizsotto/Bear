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

        const Parser parser({ { HELP, { 0, "this message" } },
            { FLAG, { 0, "a single flag" } },
            { OPTION, { 1, "a flag with a value" } },
            { OPTIONS, { 3, "a flag with 3 values" } },
            { SEPARATOR, { -1, "rest of the arguments" } } });
        parser.parse(argc, const_cast<const char**>(argv))
            .map<int>([](auto params) {
                auto program_it = params.find(PROGRAM_KEY);
                EXPECT_NE(program_it, params.end());
                EXPECT_EQ(std::get<0>(program_it->second) + 1, std::get<1>(program_it->second));
                EXPECT_STREQ(*(std::get<0>(program_it->second)), "executable");

                EXPECT_EQ(params.find(HELP), params.end());

                auto flag_it = params.find(FLAG);
                EXPECT_NE(flag_it, params.end());
                EXPECT_EQ(std::get<0>(flag_it->second), std::get<1>(flag_it->second));

                auto option_it = params.find(OPTION);
                EXPECT_NE(option_it, params.end());
                EXPECT_EQ(std::get<0>(option_it->second) + 1, std::get<1>(option_it->second));
                EXPECT_STREQ(*(std::get<0>(option_it->second) + 0), "0");

                auto options_it = params.find(OPTIONS);
                EXPECT_NE(options_it, params.end());
                EXPECT_EQ(std::get<0>(options_it->second) + 3, std::get<1>(options_it->second));
                EXPECT_STREQ(*(std::get<0>(options_it->second) + 0), "1");
                EXPECT_STREQ(*(std::get<0>(options_it->second) + 1), "2");
                EXPECT_STREQ(*(std::get<0>(options_it->second) + 2), "3");

                auto separator_it = params.find(SEPARATOR);
                EXPECT_NE(separator_it, params.end());
                EXPECT_EQ(std::get<0>(separator_it->second) + 2, std::get<1>(separator_it->second));
                EXPECT_STREQ(*(std::get<0>(separator_it->second) + 0), "4");
                EXPECT_STREQ(*(std::get<0>(separator_it->second) + 1), "5");
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

        const Parser parser({ { HELP, { 0, "this message" } },
            { FLAG, { 0, "a single flag" } } });
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

        const Parser parser({ { HELP, { 0, "this message" } },
            { FLAG, { 0, "a single flag" } },
            { OPTIONS, { 3, "a flag with 3 values" } } });
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
        const std::string expected = "Usage: thing [OPTION]\n"
                                     "\n"
                                     "  --flag                 a single flag\n"
                                     "  --help                 this message\n";
        const Parser parser({ { HELP, { 0, "this message" } },
            { FLAG, { 0, "a single flag" } } });
        const std::string help = parser.help("thing");

        EXPECT_EQ(help, expected);
    }
}
